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
    /// The widget viewport the engraving is fitted to (whole px).
    view: Option<(f64, f64)>,
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
            view: None,
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
        self.toolkit.layout(&self.layout_options);
        self.apply_fit();
        let layout = self.toolkit.current_layout()?;

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

    /// Wrap systems at `width` layout pixels: long scores flow onto
    /// multiple rows. Element ids are stable across relayouts, so
    /// feedback coloring and cursor state carry over.
    pub fn set_system_width(&mut self, width: f64) {
        // Whole pixels, and never so narrow that a single measure can't
        // fit — avoids relayout churn from sub-pixel resize noise.
        let width = Some(width.round().max(200.0));
        if self.layout_options.system_width == width {
            return;
        }
        self.layout_options.system_width = width;
        if self.toolkit.current_layout().is_some() {
            self.toolkit.layout(&self.layout_options);
        }
    }

    /// Fit the engraving to a widget viewport, like the Swift page
    /// reflowing on resize: try a few wrap widths and keep the one whose
    /// fitted (uniform) scale reads largest. Re-runs when the viewport
    /// changes and after each new score engraves.
    pub fn fit_view(&mut self, view_w: f64, view_h: f64) {
        let view = (view_w.round().max(1.0), view_h.round().max(1.0));
        if self.view == Some(view) {
            return;
        }
        self.view = Some(view);
        self.apply_fit();
    }

    fn apply_fit(&mut self) {
        let Some((view_w, view_h)) = self.view else {
            return;
        };
        if self.toolkit.current_layout().is_none() {
            return;
        }
        // Wider rows = fewer, shorter systems; narrower rows use the full
        // width. The best trade depends on the score, so measure it.
        let mut best: (f64, Option<f64>) = (f64::MIN, None);
        for factor in [1.0, 1.5, 2.0, 3.0, 4.0] {
            let candidate = Some((view_w * factor).round().max(200.0));
            self.layout_options.system_width = candidate;
            let layout = self.toolkit.layout(&self.layout_options);
            let scale = (view_w / layout.width)
                .min(view_h / layout.height)
                .min(1.6);
            if scale > best.0 {
                best = (scale, candidate);
            }
        }
        self.layout_options.system_width = best.1;
        self.toolkit.layout(&self.layout_options);
    }
}
