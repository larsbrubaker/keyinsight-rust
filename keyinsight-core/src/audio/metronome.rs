//! Metronome with a host-clock beat grid. The authoritative clock is host
//! time — the same clock NoteEvents carry — so matcher targets and audible
//! clicks cannot drift apart. If audio output is unavailable, the clock
//! still runs and the metronome is simply silent.
//!
//! Ports `Audio/Metronome.swift`. The AVAudioPlayerNode scheduling maps to
//! [`AudioOut::play_click`]; instead of a 0.4 s Timer, the session engine
//! pumps [`Metronome::schedule_ahead`] from its per-frame sweep.

use std::rc::Rc;

use crate::audio::AudioOut;

pub struct Metronome {
    audio: Rc<dyn AudioOut>,
    is_running: bool,
    /// Host-time seconds of beat 0.
    start_host_seconds: f64,
    beat_interval_seconds: f64,
    beats_per_measure: i32,
    scheduled_beats: i32,
}

impl Metronome {
    pub fn new(audio: Rc<dyn AudioOut>) -> Self {
        Self {
            audio,
            is_running: false,
            start_host_seconds: 0.0,
            beat_interval_seconds: 1.0,
            beats_per_measure: 4,
        scheduled_beats: 0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn beats_per_measure(&self) -> i32 {
        self.beats_per_measure
    }

    /// Starts the clock (and clicks, through the audio seam) with beat 0 at
    /// `start_at` host seconds.
    pub fn start(&mut self, bpm: f64, beats_per_measure: i32, start_at: f64, now: f64) {
        self.stop();
        self.beats_per_measure = beats_per_measure;
        self.beat_interval_seconds = 60.0 / bpm;
        self.start_host_seconds = start_at;
        self.scheduled_beats = 0;
        self.is_running = true;
        self.schedule_ahead(now);
    }

    pub fn stop(&mut self) {
        if !self.is_running {
            return;
        }
        self.is_running = false;
    }

    /// Milliseconds since beat 0 on the shared host clock.
    pub fn milliseconds_since_start(&self, host_seconds: f64) -> f64 {
        (host_seconds - self.start_host_seconds) * 1000.0
    }

    /// Current beat index since start (negative before beat 0).
    pub fn beat_index(&self, host_seconds: f64) -> i32 {
        ((host_seconds - self.start_host_seconds) / self.beat_interval_seconds).floor() as i32
    }

    /// Schedule clicks up to 1.5 s ahead. Call regularly while running
    /// (the engine's sweep does).
    pub fn schedule_ahead(&mut self, now: f64) {
        if !self.is_running {
            return;
        }
        let horizon = now + 1.5;
        loop {
            let beat_time =
                self.start_host_seconds + self.scheduled_beats as f64 * self.beat_interval_seconds;
            if beat_time > horizon {
                break;
            }
            if beat_time >= now - 0.05 {
                let accent = self.scheduled_beats % self.beats_per_measure == 0;
                self.audio.play_click(beat_time, accent);
            }
            self.scheduled_beats += 1;
        }
    }
}
