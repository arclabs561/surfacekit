//! A generic command PTY, separate from the piku-specific recorder.
//!
//! `PtyHandle` owns piku's launch contract. This type is the transport seam for
//! any command, so the actor loop can drive a fixture without importing a
//! product.

use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

use nix::fcntl::{fcntl, FcntlArg, OFlag};

use super::pty::set_pty_winsize;
use super::screen::{ScreenObserver, ScreenSnapshot};

/// Byte transport used by the actor loop.
pub trait SessionIo {
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// Non-blocking read. `Ok(0)` means "nothing available", not EOF; check
    /// [`SessionIo::is_dead`] for closure.
    fn read_available(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()>;
    fn is_dead(&self) -> bool;
}

pub struct CommandPty {
    _process: rexpect::process::PtyProcess,
    writer: File,
    reader: File,
    eof: bool,
    /// Exit code, once observed; `waitpid` reports it only once.
    exited: Option<i32>,
}

impl CommandPty {
    pub fn spawn(command: Command, rows: u16, cols: u16) -> io::Result<Self> {
        let mut process = rexpect::process::PtyProcess::new(command)
            .map_err(|error| io::Error::other(error.to_string()))?;
        process.set_kill_timeout(Some(5_000));

        let pty_fd = process
            .get_file_handle()
            .map_err(|error| io::Error::other(error.to_string()))?;
        set_pty_winsize(&pty_fd, rows, cols);

        let writer = process
            .get_file_handle()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let reader = process
            .get_file_handle()
            .map_err(|error| io::Error::other(error.to_string()))?;

        let flags = fcntl(&reader, FcntlArg::F_GETFL)
            .map_err(|error| io::Error::other(error.to_string()))?;
        fcntl(
            &reader,
            FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

        Ok(Self {
            _process: process,
            writer,
            reader,
            eof: false,
            exited: None,
        })
    }

    /// Drain everything the child has written into `observer` until `ready`
    /// holds or `timeout` passes. Returns whether it held and the last screen.
    pub fn wait_until(
        &mut self,
        observer: &mut ScreenObserver,
        timeout: Duration,
        mut ready: impl FnMut(&ScreenSnapshot) -> bool,
    ) -> (bool, ScreenSnapshot) {
        let deadline = Instant::now() + timeout;
        let mut buf = [0u8; 4096];
        loop {
            while let Ok(n) = self.read_available(&mut buf) {
                if n == 0 {
                    break;
                }
                observer.process(&buf[..n]);
            }
            let snapshot = observer.snapshot();
            if ready(&snapshot) {
                return (true, snapshot);
            }
            if Instant::now() >= deadline || self.eof {
                return (false, snapshot);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// The child's exit code once it has exited (128 + signal when killed by
    /// one), or `None` while it runs. Never blocks.
    pub fn exit_code(&mut self) -> Option<i32> {
        use rexpect::process::WaitStatus;
        if let Some(code) = self.exited {
            return Some(code);
        }
        let code = match self._process.status()? {
            WaitStatus::Exited(_, code) => code,
            WaitStatus::Signaled(_, signal, _) => 128 + signal as i32,
            _ => return None,
        };
        self.exited = Some(code);
        Some(code)
    }

    /// Wait up to `timeout` for the child to exit on its own, reading its
    /// output meanwhile so a full terminal buffer cannot stall the exit.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<i32> {
        let deadline = Instant::now() + timeout;
        loop {
            self.discard_output();
            if let Some(code) = self.exit_code() {
                return Some(code);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Send `signal` (a raw number such as `libc::SIGUSR1`) to the child, for
    /// tests of a program's own signal handling. A plain number keeps nix's
    /// version out of this crate's API.
    pub fn signal(&mut self, signal: i32) -> io::Result<()> {
        let signal = rexpect::process::Signal::try_from(signal)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        self._process
            .signal(signal)
            .map_err(|error| io::Error::other(error.to_string()))
    }

    /// Another handle to the PTY master, for terminal ioctls (window size,
    /// termios) a caller needs beyond [`SessionIo`].
    pub fn master(&self) -> io::Result<File> {
        self.writer.try_clone()
    }

    /// Read and discard pending output; true once the PTY is closed.
    fn discard_output(&mut self) -> bool {
        let mut buf = [0u8; 4096];
        while let Ok(n) = self.read_available(&mut buf) {
            if n == 0 {
                break;
            }
        }
        self.eof
    }
}

impl Drop for CommandPty {
    /// Terminate the child within a fixed bound. rexpect's own drop loops
    /// until the child is gone, and a child whose terminal output buffer is
    /// full cannot finish exiting (the kernel drains tty output on close), not
    /// even after SIGKILL, unless someone keeps reading the master. So keep
    /// reading while waiting, and escalate to SIGKILL after a short grace.
    fn drop(&mut self) {
        use rexpect::process::{Signal, WaitStatus};
        if self.exited.is_some() {
            return;
        }
        for (signal, bound) in [
            (Signal::SIGTERM, Duration::from_secs(1)),
            (Signal::SIGKILL, Duration::from_secs(5)),
        ] {
            let _ = self._process.signal(signal);
            let deadline = Instant::now() + bound;
            while Instant::now() < deadline {
                self.discard_output();
                match self._process.status() {
                    Some(WaitStatus::StillAlive) => std::thread::sleep(Duration::from_millis(10)),
                    Some(_) | None => return,
                }
            }
        }
    }
}

impl SessionIo for CommandPty {
    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        // The master fd is nonblocking for reads; a write can transiently
        // return EAGAIN while the slave catches up.
        let deadline = Instant::now() + Duration::from_millis(200);
        let mut written = 0;
        while written < bytes.len() {
            match self.writer.write(&bytes[written..]) {
                Ok(0) => break,
                Ok(n) => written += n,
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(error),
            }
        }
        self.writer.flush()
    }

    fn read_available(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.reader.read(buf) {
            // A zero-length read on a PTY master can mean "no data yet" before
            // the child opens the slave; closure is reported as EIO below.
            Ok(n) => Ok(n),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => {
                self.eof = true;
                Ok(0)
            }
            Err(error) => Err(error),
        }
    }

    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        set_pty_winsize(&self.writer, rows, cols);
        Ok(())
    }

    fn is_dead(&self) -> bool {
        self.eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A child that ignores SIGTERM and fills its terminal output buffer used
    /// to hang the owning test forever on drop. Dropping must now return
    /// within the SIGTERM grace plus the SIGKILL bound.
    #[test]
    fn drop_terminates_a_flooding_child_that_ignores_sigterm() {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "trap '' TERM; while :; do echo flooding-output-line; done",
        ]);
        let pty = CommandPty::spawn(command, 24, 80).expect("spawn");
        std::thread::sleep(Duration::from_millis(300));
        let started = Instant::now();
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            drop(pty);
            let _ = done.send(());
        });
        assert!(
            finished.recv_timeout(Duration::from_secs(10)).is_ok(),
            "dropping a flooding child did not return"
        );
        assert!(started.elapsed() < Duration::from_secs(7));
    }

    #[test]
    fn exit_code_reports_the_status_once_and_keeps_it() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf done; exit 3"]);
        let mut pty = CommandPty::spawn(command, 24, 80).expect("spawn");
        assert_eq!(pty.wait_for_exit(Duration::from_secs(5)), Some(3));
        assert_eq!(
            pty.exit_code(),
            Some(3),
            "a second query must not lose the status"
        );
    }

    #[test]
    fn signal_reaches_the_child_handler() {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "trap 'exit 7' USR1; printf ready; while :; do sleep 0.05; done",
        ]);
        let mut pty = CommandPty::spawn(command, 24, 80).expect("spawn");
        let mut observer = ScreenObserver::new(24, 80);
        let (ready, _) =
            pty.wait_until(&mut observer, Duration::from_secs(5), |s| s.shows("ready"));
        assert!(ready);
        pty.signal(libc::SIGUSR1).expect("signal");
        assert_eq!(pty.wait_for_exit(Duration::from_secs(5)), Some(7));
        assert!(
            pty.signal(-1).is_err(),
            "an invalid signal number is an error"
        );
    }

    #[test]
    fn wait_for_exit_gives_up_on_a_running_child() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        let mut pty = CommandPty::spawn(command, 24, 80).expect("spawn");
        assert_eq!(pty.wait_for_exit(Duration::from_millis(200)), None);
        assert!(pty.master().is_ok());
    }

    #[test]
    fn wait_until_returns_the_screen_once_the_predicate_holds() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'hello surface'; sleep 5"]);
        let mut pty = CommandPty::spawn(command, 24, 80).expect("spawn");
        let mut observer = ScreenObserver::new(24, 80);
        let (held, screen) = pty.wait_until(&mut observer, Duration::from_secs(5), |s| {
            s.shows("hello surface")
        });
        assert!(held, "{}", screen.summary(5));
        let (held, _) = pty.wait_until(&mut observer, Duration::from_millis(100), |s| {
            s.shows("never")
        });
        assert!(!held);
    }
}
