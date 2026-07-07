//! The notation layer: engraving via verovio-rust, the feedback-state
//! controller, and plain-language vocabulary.
//!
//! Ports `Sources/KeyInSight/Notation/`. The Swift WKWebView/SVG pipeline
//! maps to verovio-rust + an agg-gui widget (`docs/platform-substitutions.md`):
//! CSS class flips become per-id color overrides, the HTML ghost overlay
//! becomes widget painting, and the rAF follow loop becomes a schedule the
//! widget advances each painted frame.

mod controller;
mod renderer;
mod vocabulary;
mod widget;

pub use controller::{NotationController, NoteState};
pub use renderer::{NotationRenderer, Rendered};
pub use vocabulary::NotationVocabulary;
pub use widget::NotationWidget;
