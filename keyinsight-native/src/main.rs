//! # Native Shell for KeyInSight
//!
//! Thinnest possible desktop shim: everything platform-generic (winit
//! window and event loop, wgpu surface, input forwarding, frame painting)
//! lives in `demo_wgpu::native_shell`. This file contributes only what is
//! genuinely specific to KeyInSight on desktop: the [`KeyInSightPlatform`]
//! implementation (file-backed storage under the OS app-data directory;
//! MIDI via midir and audio out via cpal land here next — see
//! `docs/platform-substitutions.md`) and the per-frame engine tick.

mod audio;

use std::path::PathBuf;
use std::rc::Rc;

use keyinsight_core::audio::AudioOut;
use keyinsight_core::persistence::Storage;
use keyinsight_core::{build_keyinsight_app, KeyInSightPlatform, UiFonts};

/// File-backed storage in the platform app-data directory (the port of
/// `AppDatabase.onDisk()`'s Application Support path).
struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    fn in_app_data() -> Option<Self> {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| {
                    let mut p = PathBuf::from(home);
                    p.push(".local");
                    p.push("share");
                    p
                })
            })?;
        let dir = base.join("KeyInSight");
        std::fs::create_dir_all(&dir).ok()?;
        Some(Self {
            path: dir.join("keyinsight.json"),
        })
    }
}

impl Storage for FileStorage {
    fn load(&self) -> Option<String> {
        std::fs::read_to_string(&self.path).ok()
    }

    fn save(&self, contents: &str) {
        // Persistence failures never take down the training loop (the
        // Swift app logged and continued the same way).
        if let Err(err) = std::fs::write(&self.path, contents) {
            eprintln!("KeyInSight: persistence unavailable ({err}) — continuing without it");
        }
    }
}

/// Desktop implementation of the platform capability surface.
struct NativePlatform;

impl KeyInSightPlatform for NativePlatform {
    fn storage(&self) -> Option<Box<dyn Storage>> {
        FileStorage::in_app_data().map(|s| Box::new(s) as Box<dyn Storage>)
    }

    /// Metronome clicks + Hear It playback through the default output
    /// device (silent fallback when none exists).
    fn audio(&self) -> Rc<dyn AudioOut> {
        Rc::new(audio::CpalAudioOut::new())
    }

    fn supports_musicxml_import(&self) -> bool {
        true
    }

    /// Native file picker for the Library sheet's Import (the
    /// `NSOpenPanel` in `LibrarySheet.swift`). `rfd` blocks the event
    /// loop while open, same as the Swift `runModal()`.
    fn open_musicxml(&self, on_file: Box<dyn FnOnce(Vec<u8>, String)>) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MusicXML", &["musicxml", "xml"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Ok(data) => {
                let name = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Imported".to_string());
                on_file(data, name);
            }
            Err(err) => eprintln!("KeyInSight: couldn't read {}: {err}", path.display()),
        }
    }
}

fn main() {
    // Headless audio diagnostic: play a C-major arpeggio + two clicks
    // through the real output path and exit (`keyinsight-native --audio-smoke`).
    if std::env::args().any(|arg| arg == "--audio-smoke") {
        audio_smoke();
        return;
    }

    let (app, handles) = build_keyinsight_app(UiFonts::bundled(), NativePlatform);

    demo_wgpu::native_shell::run(
        demo_wgpu::NativeShellConfig {
            title: "KeyInSight",
            logical_size: (1180.0, 640.0),
        },
        app,
        // Advance the engine every painted frame (input queue, deferred
        // actions, metronome sweep).
        move || handles.tick(),
    );
}

fn audio_smoke() {
    use keyinsight_core::audio::MidiFileEncoder;
    use keyinsight_core::score::{Exercise, NoteDuration, ScoreNote};

    let out = audio::CpalAudioOut::new();
    let exercise = Exercise::new(
        vec![
            ScoreNote::note(60, NoteDuration::Quarter),
            ScoreNote::note(64, NoteDuration::Quarter),
            ScoreNote::note(67, NoteDuration::Quarter),
            ScoreNote::note(72, NoteDuration::Half),
        ],
        4,
    );
    let smf = MidiFileEncoder::encode(&exercise, 120.0, 0);
    let playing = out.play_smf(&smf);
    let now = keyinsight_core::host_now();
    out.play_click(now + 0.5, true);
    out.play_click(now + 1.0, false);
    println!("audio-smoke: play_smf accepted = {playing}");
    std::thread::sleep(std::time::Duration::from_millis(3500));
    println!("audio-smoke: done");
}
