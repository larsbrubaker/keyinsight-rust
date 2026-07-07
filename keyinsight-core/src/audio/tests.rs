//! Ports `Tests/KeyInSightTests/MicTests.swift` (YinPitchDetectorTests +
//! NoteGateTests) and `MIDIFileEncoderTests.swift`.

use crate::audio::{GateAction, MidiFileEncoder, NoteGate, YinPitchDetector};
use crate::core::{Rng64, SplitMix64};
use crate::score::{Exercise, NoteDuration, ScoreNote};

const SAMPLE_RATE: f64 = 44100.0;

fn sine(frequency: f64, count: usize, amplitude: f32) -> Vec<f32> {
    (0..count)
        .map(|i| {
            amplitude
                * (2.0 * std::f64::consts::PI * frequency * i as f64 / SAMPLE_RATE).sin() as f32
        })
        .collect()
}

/// Piano-ish tone: fundamental plus decaying harmonics.
fn tone(frequency: f64, count: usize) -> Vec<f32> {
    (0..count)
        .map(|i| {
            let t = i as f64 / SAMPLE_RATE;
            let value = (2.0 * std::f64::consts::PI * frequency * t).sin()
                + 0.5 * (2.0 * std::f64::consts::PI * 2.0 * frequency * t).sin()
                + 0.25 * (2.0 * std::f64::consts::PI * 3.0 * frequency * t).sin();
            (0.3 * value) as f32
        })
        .collect()
}

// --- YinPitchDetectorTests ---

#[test]
fn detects_a440() {
    let detector = YinPitchDetector::new(SAMPLE_RATE);
    let detection = detector.detect(&sine(440.0, 4096, 0.4)).unwrap();
    assert!((detection.frequency - 440.0).abs() < 2.0);
    assert_eq!(detection.midi(), Some(69));
    assert!(detection.confidence > 0.9);
}

#[test]
fn detects_middle_c_with_harmonics() {
    let detector = YinPitchDetector::new(SAMPLE_RATE);
    let detection = detector.detect(&tone(261.63, 4096));
    assert_eq!(detection.and_then(|d| d.midi()), Some(60));
}

#[test]
fn detects_low_and_high_piano_range() {
    let detector = YinPitchDetector::new(SAMPLE_RATE);
    assert_eq!(
        detector.detect(&tone(98.0, 4096)).and_then(|d| d.midi()),
        Some(43)
    ); // G2
    assert_eq!(
        detector
            .detect(&sine(880.0, 4096, 0.4))
            .and_then(|d| d.midi()),
        Some(81)
    ); // A5
}

#[test]
fn rejects_noise_and_silence() {
    let detector = YinPitchDetector::new(SAMPLE_RATE);
    let mut rng = SplitMix64::new(9);
    let noise: Vec<f32> = (0..4096)
        .map(|_| (rng.next_f64_below(0.8) - 0.4) as f32)
        .collect();
    if let Some(detection) = detector.detect(&noise) {
        assert!(detection.confidence < 0.6);
    }
    let silence = vec![0.0f32; 4096];
    assert_eq!(detector.detect(&silence), None);
}

// --- NoteGateTests ---

#[test]
fn two_consistent_frames_open_a_note() {
    let mut gate = NoteGate::default();
    assert_eq!(gate.process(Some(60), 0.9), GateAction::None);
    assert_eq!(
        gate.process(Some(60), 0.9),
        GateAction::On {
            midi: 60,
            confidence: 0.9
        }
    );
    // Sustained: no re-trigger.
    assert_eq!(gate.process(Some(60), 0.9), GateAction::None);
}

#[test]
fn flicker_does_not_trigger() {
    let mut gate = NoteGate::default();
    assert_eq!(gate.process(Some(60), 0.9), GateAction::None);
    assert_eq!(gate.process(Some(62), 0.9), GateAction::None);
    assert_eq!(gate.process(Some(64), 0.9), GateAction::None);
    assert_eq!(gate.active_midi(), None);
}

#[test]
fn two_quiet_frames_close_the_note() {
    let mut gate = NoteGate::default();
    let _ = gate.process(Some(60), 0.9);
    let _ = gate.process(Some(60), 0.9);
    assert_eq!(gate.process(None, 0.0), GateAction::None);
    assert_eq!(gate.process(None, 0.0), GateAction::Off { midi: 60 });
    assert_eq!(gate.active_midi(), None);
}

#[test]
fn low_confidence_counts_as_silence() {
    let mut gate = NoteGate::default();
    let _ = gate.process(Some(60), 0.9);
    let _ = gate.process(Some(60), 0.9);
    let _ = gate.process(Some(62), 0.3);
    assert_eq!(gate.process(Some(62), 0.3), GateAction::Off { midi: 60 });
}

#[test]
fn pitch_change_replaces_the_note() {
    let mut gate = NoteGate::default();
    let _ = gate.process(Some(60), 0.9);
    let _ = gate.process(Some(60), 0.9);
    assert_eq!(gate.process(Some(64), 0.8), GateAction::None);
    assert_eq!(
        gate.process(Some(64), 0.8),
        GateAction::Replace {
            off: 60,
            on: 64,
            confidence: 0.8
        }
    );
    assert_eq!(gate.active_midi(), Some(64));
}

// --- MIDIFileEncoderTests ---

#[test]
fn variable_length_quantities() {
    assert_eq!(MidiFileEncoder::variable_length(0), [0x00]);
    assert_eq!(MidiFileEncoder::variable_length(0x7F), [0x7F]);
    assert_eq!(MidiFileEncoder::variable_length(0x80), [0x81, 0x00]);
    assert_eq!(MidiFileEncoder::variable_length(240), [0x81, 0x70]);
    assert_eq!(MidiFileEncoder::variable_length(1766), [0x8D, 0x66]);
    assert_eq!(MidiFileEncoder::variable_length(0x4000), [0x81, 0x80, 0x00]);
}

#[test]
fn header_and_tempo_and_end_of_track() {
    let exercise = Exercise::new(vec![ScoreNote::note(60, NoteDuration::Whole)], 4);
    let bytes = MidiFileEncoder::encode(&exercise, 120.0, 0);

    // MThd, format 0, one track, 480 ticks per quarter.
    assert_eq!(
        &bytes[0..14],
        [0x4D, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0]
    );
    assert_eq!(&bytes[14..18], [0x4D, 0x54, 0x72, 0x6B]); // MTrk
    // Tempo meta first: 120 BPM = 500000 µs/quarter = 0x07A120.
    assert_eq!(&bytes[22..29], [0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
    // Ends with end-of-track.
    assert_eq!(&bytes[bytes.len() - 3..], [0xFF, 0x2F, 0x00]);
    // Declared track length matches actual.
    let declared = ((bytes[18] as usize) << 24)
        | ((bytes[19] as usize) << 16)
        | ((bytes[20] as usize) << 8)
        | bytes[21] as usize;
    assert_eq!(declared, bytes.len() - 22);
}

#[test]
fn note_events_and_rest_gaps() {
    // C4 quarter, quarter rest, D4 quarter → D4's note-on lands at tick 960
    // (240 ticks/unit × 4 units in).
    let exercise = Exercise::new(
        vec![
            ScoreNote::note(60, NoteDuration::Quarter),
            ScoreNote::rest(NoteDuration::Quarter),
            ScoreNote::note(62, NoteDuration::Quarter),
        ],
        4,
    );
    let bytes = MidiFileEncoder::encode(&exercise, 120.0, 0);

    // Walk the track: collect (absolute_tick, status, note) triples.
    let mut i = 29 + 3; // after tempo meta + program change
    let mut tick = 0usize;
    let mut note_events: Vec<(usize, u8, u8)> = Vec::new();
    while i < bytes.len() - 3 {
        let mut delta = 0usize;
        while bytes[i] & 0x80 != 0 {
            delta = (delta << 7) | (bytes[i] & 0x7F) as usize;
            i += 1;
        }
        delta = (delta << 7) | bytes[i] as usize;
        i += 1;
        tick += delta;
        let status = bytes[i];
        if status == 0x90 || status == 0x80 {
            note_events.push((tick, status, bytes[i + 1]));
            i += 3;
        } else {
            break;
        }
    }
    assert_eq!(note_events.len(), 4); // 2 notes × on+off
    assert_eq!(note_events[0], (0, 0x90, 60));
    assert_eq!(note_events[1].1, 0x80);
    assert_eq!(
        note_events[1].0,
        (480.0 * MidiFileEncoder::GATE_RATIO) as usize
    );
    assert_eq!(note_events[2], (960, 0x90, 62));
}

#[test]
fn durations_and_start_times() {
    let exercise = Exercise::new(
        vec![
            ScoreNote::note(60, NoteDuration::Quarter),
            ScoreNote::rest(NoteDuration::Quarter),
            ScoreNote::note(62, NoteDuration::Half),
        ],
        4,
    );
    // 8 units total at 60 BPM: 0.5 s per unit → 4 s.
    assert!((MidiFileEncoder::duration(&exercise, 60.0) - 4.0).abs() < 1e-9);
    let starts = MidiFileEncoder::sounded_note_start_seconds(&exercise, 60.0);
    assert_eq!(starts.len(), 2);
    assert!((starts[0] - 0.0).abs() < 1e-9);
    assert!((starts[1] - 2.0).abs() < 1e-9); // after quarter + rest
}
