# KeyInSight Port — Status & TODO

*Updated 2026-07-07. This file is the hand-off point for resuming work on
another machine.*

## Where things stand

**Phase 1 (the true port of the Swift app) is complete for the core
training loop and shipped.** Every module of the Swift reference
(`keyinsight-swift-reference/`, pinned submodule) is ported to Rust with
its test suite: 168 tests green across the workspace, clippy clean
(`-D warnings`), CI and GitHub Pages deploys passing.

Live app: <https://larsbrubaker.github.io/keyinsight-rust/>
(training loop works in the browser: press A S D F G H J K = C4–C5,
W E T Y U = sharps, Z/X octave shift; progress persists in localStorage).

Native app: `cargo run -p keyinsight-native` (persists to
`%APPDATA%/KeyInSight/keyinsight.json`).

## Repository layout (three repos, all pushed)

| Repo | License | Role |
|---|---|---|
| [keyinsight-rust](https://github.com/larsbrubaker/keyinsight-rust) | MIT | The app. Submodule of rust-apps. Contains `keyinsight-swift-reference/` (pinned Swift source). |
| [verovio-rust](https://github.com/larsbrubaker/verovio-rust) | **LGPL-3.0** | Music engraving port (Verovio → Rust, renders via agg-gui). Separate repo purely for license isolation — never inline its code into the app. Contains `verovio-cpp-reference/` pinned at `8d42439` (6.2.1, same revision the Swift app pinned). Submodule of rust-apps. |
| [agg-gui](https://github.com/larsbrubaker/agg-gui) | MIT | UI framework. Path-patched sibling (`[patch.crates-io]`). |

Local layout must be siblings (the rust-apps superproject provides this):
`../agg-gui`, `../verovio-rust` next to this repo. CI clones the same way.

## Setup on a fresh machine

```powershell
git clone --recurse-submodules https://github.com/larsbrubaker/rust-apps.git
cd rust-apps/keyinsight-rust
cargo test --workspace              # everything should be green
cargo run -p keyinsight-native      # desktop app
```

Rules live in `CLAUDE.md` + `docs/` (porting rules, platform
substitutions, architecture, build/deploy). verovio-rust has its own
`CLAUDE.md` + `docs/`.

## What is ported (all with the Swift test suites)

- **Core**: `NoteEvent` seam, `PitchSpelling`, bit-exact `SplitMix64`.
- **Score**: model (chords/ties/two voices), `DifficultyDescriptors`,
  adaptive `ExerciseGenerator`, `FreePlayScore`, MusicXML encoder +
  importer (round-trips), all 18 bundled pieces verified note-for-note.
- **Engine**: `SelfPacedMatcher`, `TempoMatcher` + tempo/rhythm policies,
  `OctaveAnchor`, and the full `SessionEngine`
  (`src/engine/session/{mod,lifecycle,input,modes,progress}.rs`):
  adaptive exercises, drills, free play, repertoire, unplugged
  self-grading, tempo runs with count-in + miss sweep, auto-advance,
  per-user sessions, event logging. Swift timers/dispatch map to the
  deadline queue processed in `SessionEngine::tick()` (called every
  painted frame by the shells).
- **Skill**: EWMA mastery, unlock ladder, interval items, key unlocks.
- **Persistence**: full `AppDatabase` semantics (users, sessions,
  exercises, note-event log, item stats, settings, piece plays — all
  per-user) as one serde document behind the `Storage` trait
  (native = file, wasm = localStorage, tests = memory).
- **Audio**: YIN pitch detector + `NoteGate` (pure DSP, ready for the mic
  backend), SMF encoder, metronome clock over the `AudioOut` trait
  (currently `NullAudioOut` everywhere — silent, clock still runs).
- **Input**: simulated keyboard backend (focus-routed through the
  `TrainingRoot` widget), unplugged backend.
- **Notation**: `NotationRenderer` (wraps verovio-rust), feedback
  `NotationController` (state colors, ghost note, timing ticks,
  playback-follow schedule), `NotationWidget` (paints the score, always
  light page, hover-to-name vocabulary).
- **UI**: training root + side panel + bottom bar + piano strip +
  Library/Progress/Calibration sheets + add/rename player dialogs, light
  theme forced app-wide. Visual parity pass done (2026-07-07): Inter
  regular/bold + Cascadia mono + Font Awesome faces (`ui/fonts.rs`),
  macOS system palette (`ui/palette.rs`), dividers/fixed geometry
  matching the SwiftUI views, colored status rows with icons + painted
  beat dots (`InfoRows`), mic `LevelMeter`, boxed instructions,
  segmented pickers with disabled states, toggle switches, ComboBox
  player picker, centered modal sheets over a scrim (`agg-gui
  ModalSheet`), heat-map staff + stat tables in Progress, MusicXML
  import via `rfd` on native (`KeyInSightPlatform::open_musicxml`).

## TODO — Phase 2 (rough priority order)

1. **Native MIDI input** (`midir`): implement `InputBackend` in
   `keyinsight-native`, feed the engine's `event_queue`, wire the
   `BackendFactory` (see `default_backend_factory` — it currently
   substitutes the simulated backend for MIDI/mic).
2. **Native audio out** (`cpal`): implement `AudioOut` (metronome clicks =
   synthesized sine bursts per `Metronome.swift`; SMF playback needs a
   small sampler/synth — start with a simple synthesized piano tone, the
   Swift architecture doc sanctions that).
3. **Web MIDI + WebAudio** in `keyinsight-wasm` (same two traits;
   permission requests belong in the shim/`main.ts`, never visible UI).
4. **Mic backend**: platform mic capture → `YinPitchDetector` + `NoteGate`
   (both ported and tested) → `NoteEvent`s; level meter UI.
5. ~~**CalibrationSheet**~~ — done (2026-07-07): `ui/sheets/calibration.rs`,
   tap-along flow with warm-ups, median input-latency compensation,
   piano keys pass through the modal (`ModalSheet::with_key_passthrough`).
6. **DemoDriver** (`Engine/DemoDriver.swift`): the scripted `--demo`
   playthrough. The engine surface it needs (`current_expected_midi`,
   tempo debug) exists.
7. ~~**Text-input overlays**~~ — done (2026-07-07):
   `ui/sheets/player_dialogs.rs`, add/rename dialogs with auto-focused
   TextField (modal subtree focus routing landed in agg-gui).
8. **Engraving polish in verovio-rust**: ledger-line coverage check,
   accidental spacing, beam slants, non-linear spacing, glyph metrics
   from `bravura_metadata.json`-style font metadata instead of the fixed
   width table, multi-system line breaking for long repertoire
   (currently one long system; the notation widget scales to fit).
9. **Notation widget scroll** for long pieces (Swift used a scrollable
   page + auto-follow; the widget currently scales down).
10. ~~**Progress sheet heat staff**~~ — done (2026-07-07): the Progress
    sheet renders the heat-map staff through a dedicated
    `NotationController` plus the full stat sections and legend.
11. **PWA polish for the demo site** (icons/manifest like the other apps).

## Known rough edges

- The visual-parity pass depends on unpublished agg-gui additions
  (`ModalSheet`, `Rebuilder`, `Stack::with_hit_children_only`, modal
  subtree event routing): agg-gui must be pushed/published before
  keyinsight CI can build this revision.
- `Toolkit::layout()`/`render()` panic if called before `load_music_xml`
  (mirrors the C++ toolkit contract; the app never does).
- The session RNG seeds from wall time at launch (Swift used
  `SystemRandomNumberGenerator`); pass a fixed seed to
  `SessionEngine::new` for reproducible runs.
- rust-apps superproject: only the keyinsight-rust/verovio-rust pointers
  were committed by this work; other submodules have unrelated local
  changes from earlier sessions.

## Conventions to keep (short version — CLAUDE.md is authoritative)

- Port the Swift tests with every module; never weaken a test.
- 800-line file cap (enforced by `keyinsight-core/tests/file_line_count.rs`).
- `keyinsight-core` stays wasm-clean; platform APIs behind traits.
- All UI through agg-gui; notation only through verovio-rust (LGPL wall).
- Music always renders light; the app runs agg-gui's light visuals.
- One green module per commit.
