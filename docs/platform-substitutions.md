# Platform substitutions (Swift/Apple → cross-platform Rust)

The Swift app leans on Apple frameworks. Each has a designated replacement;
the substitution is always made **behind a trait in `keyinsight-core`**,
never inline. These are the only sanctioned divergences from the Swift
source (see `docs/porting.md`).

| Swift / Apple | Rust port |
|---|---|
| SwiftUI views | agg-gui widgets (see `docs/architecture.md`) |
| Verovio SVG notation via WKWebView | agg-gui-drawn notation: a custom SMuFL-font staff renderer painting through `DrawCtx` (the "custom renderer for the constrained subset" path upstream's architecture doc itself names as the alternative). Per-note recolor, cursor, and ghost notes become ordinary widget painting. |
| SwiftMIDI / CoreMIDI | `midir` on native; Web MIDI API via `wasm-bindgen` on WASM — both behind the core's `MidiPort` trait |
| AVAudioEngine sampler + metronome | `cpal` output on native; WebAudio on WASM — behind the core's `AudioOut` trait. The SF2 soundfont path may start as a simple synthesized piano tone; fidelity is Phase 2. |
| SoundpipeAudioKit PitchTap | Port the Swift `YinPitchDetector` (pure DSP, ports directly); mic capture behind the platform trait |
| GRDB / SQLite | Storage trait in core (load/save serialized state); native = file-backed, WASM = localStorage/IndexedDB. Port the `AppDatabase` schema semantics (skill stats, session history, settings, library) even though the storage engine differs. |
| MusicXML via Verovio | Port `MusicXMLImporter`/`MusicXMLEncoder` directly (plain XML processing); use `quick-xml` |

Notes:

- Music glyphs (noteheads, clefs, accidentals) come from a bundled SMuFL
  font (Bravura, OFL-licensed) rendered through agg-gui's text stack.
- The shims (`keyinsight-native`, `keyinsight-wasm`) implement these traits;
  the core never `cfg`-gates on platform.
