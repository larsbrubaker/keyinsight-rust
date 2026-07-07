//! Monophonic pitch detection: YIN (difference function + cumulative-mean
//! normalization + parabolic interpolation), plus the [`NoteGate`] that
//! turns per-buffer detections into note on/off decisions.
//!
//! Ports `Audio/YinPitchDetector.swift` — pure DSP, ports directly.

/// One per-buffer detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detection {
    pub frequency: f64,
    /// 1 − CMND minimum: ~1 for clean periodic input, ~0 for noise.
    pub confidence: f64,
}

impl Detection {
    pub fn midi(&self) -> Option<u8> {
        let value = 69.0 + 12.0 * (self.frequency / 440.0).log2();
        if !(0.0..=127.0).contains(&value) {
            return None;
        }
        Some(value.round() as u8)
    }
}

pub struct YinPitchDetector {
    pub sample_rate: f64,
    /// Piano-relevant band; narrower = cheaper and fewer octave errors.
    pub min_frequency: f64,
    pub max_frequency: f64,
    /// CMND absolute threshold (canonical YIN uses 0.1–0.15).
    pub threshold: f64,
    /// Below this RMS the buffer is silence — YIN would otherwise report
    /// silence as perfectly periodic.
    pub energy_floor: f32,
}

impl YinPitchDetector {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            min_frequency: 60.0,
            max_frequency: 1200.0,
            threshold: 0.15,
            energy_floor: 0.005,
        }
    }

    pub fn detect(&self, samples: &[f32]) -> Option<Detection> {
        let mut sum_squares: f32 = 0.0;
        for sample in samples {
            sum_squares += sample * sample;
        }
        if (sum_squares / samples.len().max(1) as f32).sqrt() < self.energy_floor {
            return None;
        }

        let n = samples.len();
        let tau_max = ((self.sample_rate / self.min_frequency) as usize).min(n / 2);
        let tau_min = ((self.sample_rate / self.max_frequency) as usize).max(2);
        if tau_max <= tau_min + 2 {
            return None;
        }
        let window = n - tau_max;

        // Difference function over a fixed window.
        let mut difference = vec![0.0f64; tau_max];
        for (tau, diff) in difference.iter_mut().enumerate().take(tau_max).skip(tau_min) {
            let mut sum = 0.0;
            for j in 0..window {
                let d = samples[j] as f64 - samples[j + tau] as f64;
                sum += d * d;
            }
            *diff = sum;
        }

        // Cumulative mean normalized difference.
        let mut cmnd = vec![1.0f64; tau_max];
        let mut running_sum = 0.0;
        for tau in tau_min..tau_max {
            running_sum += difference[tau];
            cmnd[tau] = difference[tau] * (tau - tau_min + 1) as f64 / running_sum.max(f64::EPSILON);
        }

        // First dip under the threshold (walk to its local minimum), else
        // the global minimum if it is at least moderately periodic.
        let mut tau_estimate: isize = -1;
        let mut tau = tau_min;
        while tau < tau_max {
            if cmnd[tau] < self.threshold {
                while tau + 1 < tau_max && cmnd[tau + 1] < cmnd[tau] {
                    tau += 1;
                }
                tau_estimate = tau as isize;
                break;
            }
            tau += 1;
        }
        if tau_estimate < 0 {
            let mut min_tau = tau_min;
            for t in tau_min..tau_max {
                if cmnd[t] < cmnd[min_tau] {
                    min_tau = t;
                }
            }
            if cmnd[min_tau] >= 0.35 {
                return None;
            }
            tau_estimate = min_tau as isize;
        }
        let tau_estimate = tau_estimate as usize;

        // Parabolic interpolation around the minimum for sub-sample accuracy.
        let mut better_tau = tau_estimate as f64;
        if tau_estimate > tau_min && tau_estimate < tau_max - 1 {
            let a = cmnd[tau_estimate - 1];
            let b = cmnd[tau_estimate];
            let c = cmnd[tau_estimate + 1];
            let denominator = 2.0 * (a - 2.0 * b + c);
            if denominator.abs() > f64::EPSILON {
                better_tau += (a - c) / denominator;
            }
        }

        Some(Detection {
            frequency: self.sample_rate / better_tau,
            confidence: (1.0 - cmnd[tau_estimate]).clamp(0.0, 1.0),
        })
    }
}

/// Turns a stream of per-buffer detections into note on/off decisions: two
/// consistent frames open a note (debounce), two silent/uncertain frames
/// close it, a pitch change closes and reopens. Note-offs from a decaying
/// piano are unreliable by nature — the matcher only consumes note-ons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GateAction {
    None,
    On { midi: u8, confidence: f64 },
    Off { midi: u8 },
    Replace { off: u8, on: u8, confidence: f64 },
}

pub struct NoteGate {
    pub confidence_threshold: f64,
    active_midi: Option<u8>,
    candidate_midi: Option<u8>,
    candidate_confidence: f64,
    candidate_frames: i32,
    silent_frames: i32,
}

impl Default for NoteGate {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.5,
            active_midi: None,
            candidate_midi: None,
            candidate_confidence: 0.0,
            candidate_frames: 0,
            silent_frames: 0,
        }
    }
}

impl NoteGate {
    pub fn active_midi(&self) -> Option<u8> {
        self.active_midi
    }

    pub fn process(&mut self, midi: Option<u8>, confidence: f64) -> GateAction {
        let Some(midi) = midi.filter(|_| confidence >= self.confidence_threshold) else {
            self.candidate_midi = None;
            self.candidate_frames = 0;
            self.silent_frames += 1;
            if self.silent_frames >= 2 {
                if let Some(active) = self.active_midi.take() {
                    return GateAction::Off { midi: active };
                }
            }
            return GateAction::None;
        };
        self.silent_frames = 0;

        if Some(midi) == self.active_midi {
            self.candidate_midi = None;
            self.candidate_frames = 0;
            return GateAction::None;
        }
        if Some(midi) == self.candidate_midi {
            self.candidate_frames += 1;
            self.candidate_confidence = self.candidate_confidence.max(confidence);
        } else {
            self.candidate_midi = Some(midi);
            self.candidate_frames = 1;
            self.candidate_confidence = confidence;
        }
        if self.candidate_frames < 2 {
            return GateAction::None;
        }

        let previous = self.active_midi;
        self.active_midi = Some(midi);
        self.candidate_midi = None;
        self.candidate_frames = 0;
        match previous {
            Some(previous) => GateAction::Replace {
                off: previous,
                on: midi,
                confidence: self.candidate_confidence,
            },
            None => GateAction::On {
                midi,
                confidence: self.candidate_confidence,
            },
        }
    }
}
