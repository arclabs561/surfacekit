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
        })
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
