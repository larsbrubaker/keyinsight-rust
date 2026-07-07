//! Wraps the verovio-rust toolkit: MusicXML in, an engraved layout +
//! time-ordered note ids out.
//!
//! Ports `Notation/NotationRenderer.swift`. Where the Swift renderer got
//! SVG + a JSON timemap from C++ Verovio, this one holds the toolkit's
//! layout directly; qstamps stay quarter-note based (units / 2) so the
//! session engine's onset binding matches the Swift math line for line.

use verovio_rust::{LayoutOptions, Toolkit};

/// One engraving result.
pub struct Rendered {
    /// Per-note element ids in playback order (from the timemap).
    pub note_ids: Vec<String>,
    /// Timemap onset groups: quarter-note stamp → ids sounding at that
    /// moment (document order — treble voice then bass).
    pub note_groups: Vec<(f64, Vec<String>)>,
}

pub struct NotationRenderer {
    toolkit: Toolkit,
    layout_options: LayoutOptions,
}

impl Default for NotationRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl NotationRenderer {
    pub fn new() -> Self {
        Self {
            toolkit: Toolkit::new(),
            layout_options: LayoutOptions::default(),
        }
    }

    /// MIDI pitch of a rendered note id, from the toolkit's own model —
    /// pins the id-order assumptions without duplicating pitch math.
    pub fn midi_pitch(&self, id: &str) -> Option<u8> {
        self.toolkit.note_midi(id)
    }

    /// Engrave; returns None when the input is outside the supported
    /// subset (the Swift renderer returned nil on toolkit failure).
    pub fn render(&mut self, music_xml: &str) -> Option<Rendered> {
        self.toolkit.load_music_xml(music_xml).ok()?;
        let layout = self.toolkit.layout(&self.layout_options);

        let mut note_ids: Vec<String> = Vec::new();
        let mut note_groups: Vec<(f64, Vec<String>)> = Vec::new();
        for moment in &layout.timemap {
            if moment.note_ids.is_empty() {
                continue;
            }
            note_ids.extend(moment.note_ids.iter().cloned());
            note_groups.push((moment.onset_units as f64 / 2.0, moment.note_ids.clone()));
        }
        Some(Rendered {
            note_ids,
            note_groups,
        })
    }

    /// The toolkit holding the current engraving (widget painting and
    /// bounds queries go through it).
    pub fn toolkit(&self) -> &Toolkit {
        &self.toolkit
    }
}
