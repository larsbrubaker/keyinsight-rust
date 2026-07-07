//! Desktop [`AudioOut`] over cpal: a sample mixer on the default output
//! device playing the shared synth buffers (metronome clicks + rendered
//! SMF piano clips) at host-clock-mapped positions.
//!
//! The port of `Metronome.swift`'s AVAudioPlayerNode scheduling +
//! `PlaybackEngine.swift`'s sequencer, per
//! `docs/platform-substitutions.md`. If no output device exists the app
//! keeps training silently, exactly like the Swift engine-start failure
//! path.

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use keyinsight_core::audio::{self, AudioOut};
use keyinsight_core::host_now;

/// One scheduled buffer (a click or an SMF clip) on the mixer timeline.
struct Sound {
    samples: Arc<Vec<f32>>,
    /// Mixer frame at which sample 0 plays.
    start_frame: i64,
    /// True for SMF playback (removed by `stop_smf`; clicks aren't).
    is_clip: bool,
}

/// State shared with the audio callback.
struct Mixer {
    /// Frames rendered since the stream started.
    cursor: i64,
    /// `host_now()` at cursor 0 — maps host seconds to mixer frames.
    t0_host: f64,
    sounds: Vec<Sound>,
}

impl Mixer {
    fn frame_for(&self, host_seconds: f64, sample_rate: f64) -> i64 {
        ((host_seconds - self.t0_host) * sample_rate).round() as i64
    }

    /// Render one mono frame and advance the cursor.
    fn next_frame(&mut self) -> f32 {
        let cursor = self.cursor;
        self.cursor += 1;
        let mut value = 0.0f32;
        self.sounds.retain(|sound| {
            let offset = cursor - sound.start_frame;
            if offset >= sound.samples.len() as i64 {
                return false; // finished
            }
            if offset >= 0 {
                value += sound.samples[offset as usize];
            }
            true
        });
        value.clamp(-1.0, 1.0)
    }
}

struct StreamHandle {
    // Held for its Drop; the callback owns the mixing.
    _stream: cpal::Stream,
    mixer: Arc<Mutex<Mixer>>,
    sample_rate: f64,
    click: Arc<Vec<f32>>,
    accent: Arc<Vec<f32>>,
}

/// The platform audio output. Construct once at startup.
pub struct CpalAudioOut {
    handle: Option<StreamHandle>,
}

impl CpalAudioOut {
    pub fn new() -> Self {
        let handle = Self::open_stream()
            .map_err(|err| {
                eprintln!("KeyInSight: audio output unavailable ({err}) — continuing silently");
            })
            .ok();
        Self { handle }
    }

    fn open_stream() -> Result<StreamHandle, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no default output device")?;
        let config = device
            .default_output_config()
            .map_err(|err| err.to_string())?;
        let sample_rate = config.sample_rate().0 as f64;
        let channels = config.channels() as usize;

        let mixer = Arc::new(Mutex::new(Mixer {
            cursor: 0,
            t0_host: host_now(),
            sounds: Vec::new(),
        }));

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), channels, Arc::clone(&mixer))
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), channels, Arc::clone(&mixer))
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config.into(), channels, Arc::clone(&mixer))
            }
            other => return Err(format!("unsupported sample format {other:?}")),
        }?;
        stream.play().map_err(|err| err.to_string())?;

        Ok(StreamHandle {
            _stream: stream,
            mixer,
            sample_rate,
            click: Arc::new(audio::click_samples(sample_rate, false)),
            accent: Arc::new(audio::click_samples(sample_rate, true)),
        })
    }

    fn build_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        channels: usize,
        mixer: Arc<Mutex<Mixer>>,
    ) -> Result<cpal::Stream, String> {
        device
            .build_output_stream(
                config,
                move |data: &mut [T], _| {
                    let mut mixer = mixer.lock().expect("audio mixer lock");
                    for frame in data.chunks_mut(channels) {
                        let value = T::from_sample(mixer.next_frame());
                        for slot in frame {
                            *slot = value;
                        }
                    }
                },
                |err| eprintln!("KeyInSight: audio stream error ({err})"),
                None,
            )
            .map_err(|err| err.to_string())
    }
}

impl AudioOut for CpalAudioOut {
    fn play_click(&self, at_host_seconds: f64, accent: bool) {
        let Some(handle) = &self.handle else { return };
        let samples = if accent {
            Arc::clone(&handle.accent)
        } else {
            Arc::clone(&handle.click)
        };
        let mut mixer = handle.mixer.lock().expect("audio mixer lock");
        let start_frame = mixer.frame_for(at_host_seconds, handle.sample_rate);
        mixer.sounds.push(Sound {
            samples,
            start_frame,
            is_clip: false,
        });
    }

    fn play_smf(&self, smf: &[u8]) -> bool {
        let Some(handle) = &self.handle else {
            return false;
        };
        let Some(clip) = audio::render_smf(smf, handle.sample_rate) else {
            return false;
        };
        let mut mixer = handle.mixer.lock().expect("audio mixer lock");
        mixer.sounds.retain(|sound| !sound.is_clip);
        // A short lead keeps the first notes off the already-rendered
        // buffer edge.
        let start_frame = mixer.frame_for(host_now() + 0.05, handle.sample_rate);
        mixer.sounds.push(Sound {
            samples: Arc::new(clip.samples),
            start_frame,
            is_clip: true,
        });
        true
    }

    fn stop_smf(&self) {
        let Some(handle) = &self.handle else { return };
        let mut mixer = handle.mixer.lock().expect("audio mixer lock");
        mixer.sounds.retain(|sound| !sound.is_clip);
    }
}
