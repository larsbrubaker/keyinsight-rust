//! Plain-language names for clicked notation elements: unobtrusive
//! vocabulary, introduced on demand, never required.
//!
//! Ports `Notation/NotationVocabulary.swift`.

use crate::core::PitchSpelling;
use crate::score::{NoteDuration, ScoreNote};

pub struct NotationVocabulary;

impl NotationVocabulary {
    pub fn duration_name(duration: NoteDuration) -> &'static str {
        match duration {
            NoteDuration::Eighth => "eighth note",
            NoteDuration::Quarter => "quarter note",
            NoteDuration::DottedQuarter => "dotted quarter note",
            NoteDuration::Half => "half note",
            NoteDuration::DottedHalf => "dotted half note",
            NoteDuration::Whole => "whole note",
        }
    }

    /// Description for a non-note element kind (the engraver's element
    /// kinds, matching Verovio's SVG class names).
    pub fn describe(kind: &str, fifths: i32, beats_per_measure: i32) -> Option<String> {
        match kind {
            "clef" => Some("Treble clef — fixes G4 on the second staff line".to_string()),
            "keySig" => Some(match fifths {
                1 => "Key signature: G major — every F is played F♯".to_string(),
                2 => "Key signature: D major — every F and C is played sharp".to_string(),
                f if f < 0 => format!(
                    "Key signature: {} — these flats apply through the whole piece",
                    PitchSpelling::key_name(f)
                ),
                f if f >= 3 => format!(
                    "Key signature: {} — these sharps apply through the whole piece",
                    PitchSpelling::key_name(f)
                ),
                _ => "Key signature — applies through the whole piece".to_string(),
            }),
            "meterSig" => Some(format!(
                "Time signature — {beats_per_measure} beats per measure"
            )),
            "rest" => Some("Rest — silence, as long as the note value it matches".to_string()),
            "barLine" => Some("Barline — marks the end of a measure".to_string()),
            "accid" => Some("Accidental — alters just this note (♯ means one key up)".to_string()),
            "dots" => Some("Dot — makes the note half again as long".to_string()),
            _ => None,
        }
    }

    pub fn describe_note(note: &ScoreNote) -> String {
        match note.midi {
            None => Self::describe("rest", 0, 4).unwrap_or_else(|| "Rest".to_string()),
            Some(midi) => format!(
                "{} — {}",
                PitchSpelling::name(midi),
                Self::duration_name(note.duration)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::{NoteDuration, ScoreNote};

    #[test]
    fn names_notes_and_durations() {
        assert_eq!(
            NotationVocabulary::describe_note(&ScoreNote::note(66, NoteDuration::DottedHalf)),
            "F#4 — dotted half note"
        );
        assert!(NotationVocabulary::describe_note(&ScoreNote::rest(NoteDuration::Quarter))
            .starts_with("Rest"));
    }

    #[test]
    fn key_signature_descriptions_follow_fifths() {
        assert!(NotationVocabulary::describe("keySig", 1, 4)
            .unwrap()
            .contains("G major"));
        assert!(NotationVocabulary::describe("keySig", -2, 4)
            .unwrap()
            .contains("B♭ major"));
        assert!(NotationVocabulary::describe("meterSig", 0, 3)
            .unwrap()
            .contains("3 beats"));
        assert_eq!(NotationVocabulary::describe("unknown", 0, 4), None);
    }
}
