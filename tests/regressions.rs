//! Regression tests for packet and timeline rules stated by the JSON schema
//! in `schema/surface-evaluation.schema.json` and by the module docs.

use surfacekit::contract::SurfacePacket;

fn schema() -> serde_json::Value {
    serde_json::from_str(include_str!("../schema/surface-evaluation.schema.json")).unwrap()
}

/// The schema's caller-owned scenario example, which carries notes.
fn example_with_notes() -> serde_json::Value {
    schema()["examples"]
        .as_array()
        .unwrap()
        .iter()
        .find(|example| example.get("notes").is_some())
        .cloned()
        .expect("an example with notes")
}

/// Schema: `scenario_id` is a string with `minLength: 1`.
#[test]
fn validate_shape_rejects_an_empty_scenario_id() {
    let mut packet = SurfacePacket::new(
        "x",
        surfacekit::Surface::Cli,
        [surfacekit::Claim::OutputVisible],
        surfacekit::Surface::Cli.required_evidence(),
    );
    assert!(packet.validate_shape().is_ok());
    packet.scenario_id.clear();
    assert_eq!(packet.validate_shape(), Err("scenario id is empty"));
}

/// Schema: note `text` has `maxLength: 400`, counted in characters, which is
/// `MAX_NOTE_CHARS`.
#[test]
fn validate_shape_rejects_note_text_over_the_schema_limit() {
    let limit = surfacekit::notes::MAX_NOTE_CHARS;
    let with_text = |text: String| {
        let mut value = example_with_notes();
        value["notes"][0]["text"] = serde_json::Value::String(text);
        let packet: SurfacePacket = serde_json::from_value(value).unwrap();
        packet.validate_shape()
    };
    assert!(with_text("x".repeat(limit)).is_ok());
    assert!(with_text("あ".repeat(limit)).is_ok());
    assert_eq!(
        with_text("x".repeat(limit + 1)),
        Err("a note text is longer than the schema allows")
    );
}

#[cfg(feature = "scenario")]
mod scenario {
    use surfacekit::scenario::{Event, Timeline};

    /// timeline.rs: "Append-only, monotonic event record". A JSONL record whose
    /// timestamps go backwards is not one.
    #[test]
    fn timeline_from_jsonl_rejects_a_decreasing_timestamp() {
        let backwards = "{\"code\":\"output\",\"at_ms\":100,\"bytes\":[104,105]}\n\
                         {\"code\":\"marker\",\"at_ms\":5,\"label\":\"late\"}\n";
        let err = Timeline::from_jsonl(backwards).unwrap_err();
        assert!(err.to_string().contains("line 2"), "{err}");
    }

    #[test]
    fn timeline_from_jsonl_accepts_equal_timestamps() {
        let same = "{\"code\":\"output\",\"at_ms\":5,\"bytes\":[104]}\n\
                    {\"code\":\"marker\",\"at_ms\":5,\"label\":\"same\"}\n";
        let timeline = Timeline::from_jsonl(same).unwrap();
        let ts: Vec<u64> = timeline.events().iter().map(Event::at_ms).collect();
        assert_eq!(ts, vec![5, 5]);
        assert_eq!(timeline.duration_ms(), 5);
    }
}
