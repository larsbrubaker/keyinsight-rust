//! The third validation path: no input stream at all. The user plays their
//! real (unconnected) instrument, compares against reference playback, and
//! self-grades the pass — Nailed It / Try Again. Events never arrive; the
//! engine's `self_verify_grade` drives completion instead.
//!
//! Ports `Input/UnpluggedBackend.swift`.

use crate::core::{InputBackend, NoteEvent};

#[derive(Default)]
pub struct UnpluggedBackend {
    /// Held to satisfy the backend contract; never invoked (the Swift
    /// original's `onEvent` is equally silent).
    #[allow(dead_code)]
    on_event: Option<Box<dyn FnMut(NoteEvent)>>,
}

impl UnpluggedBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InputBackend for UnpluggedBackend {
    fn display_name(&self) -> &str {
        "Unplugged (self-verified)"
    }

    fn set_on_event(&mut self, on_event: Option<Box<dyn FnMut(NoteEvent)>>) {
        self.on_event = on_event;
    }

    fn start(&mut self) {}

    fn stop(&mut self) {}
}
