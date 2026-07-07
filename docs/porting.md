# Porting KeyInSight from Swift to Rust

Phase 1 reproduces the Swift app's behavior module by module: same
algorithms, same state machines, same scoring, same test expectations.
Phase 2 (taking over development in the agg-gui environment) never begins
while a Phase 1 module is half-ported.

## The Swift reference

The exact source being ported is the git submodule at
`keyinsight-swift-reference/` (pinned). **Always read the pinned submodule,
not the upstream website** — upstream moves. Layout:

- `Sources/KeyInSight/Core/` — `NoteEvent`, `PitchSpelling`, `SplitMix64`
- `Sources/KeyInSight/Engine/` — `SessionEngine`, `Matcher`, `TempoMatcher`,
  `OctaveAnchor`, `DemoDriver` (the training loop — the heart of the app)
- `Sources/KeyInSight/Score/` — score model, `ExerciseGenerator`,
  MusicXML import/export, repertoire library, difficulty descriptors
- `Sources/KeyInSight/Skill/` — `SkillModel` (adaptive difficulty + spaced
  repetition)
- `Sources/KeyInSight/Input/` — MIDI / mic / simulated-keyboard / unplugged
  backends, all emitting the normalized `NoteEvent` stream
- `Sources/KeyInSight/Audio/` — metronome, SMF encoder, playback engine,
  YIN pitch detector
- `Sources/KeyInSight/Notation/` — Verovio-backed notation controller
- `Sources/KeyInSight/Persistence/` — `AppDatabase` (SQLite/GRDB)
- `Sources/KeyInSight/UI/` — SwiftUI views (become agg-gui widgets)
- `Sources/KeyInSight/Resources/Pieces/` — bundled public-domain MusicXML
- `Tests/KeyInSightTests/` — the Swift test suite (port alongside each module)
- `docs/` — upstream's planning docs; `docs/03-architecture.md` explains
  every design decision and module boundary. Read it before porting a module.

## Behavioral matching

- Match Swift behavior: same algorithms, state machines, edge cases,
  thresholds, and timing windows.
- **Determinism matters.** `SplitMix64` is the seeded RNG behind exercise
  generation — port it bit-for-bit so a given seed produces the identical
  exercise on macOS-Swift, native-Rust, and WASM. Never substitute `rand`.
- **The `NoteEvent` stream is the load-bearing seam.** All input backends
  emit `NoteEvent { pitch, kind: on|off, velocity, timestamp, confidence }`
  and everything above (matcher, scoring, UI) is input-agnostic. Preserve
  this boundary exactly — it is what lets backends vary per platform.
- Timing math stays in the numeric types the Swift source uses (`Double` →
  `f64`). Don't "simplify" tolerance-window or latency-compensation
  arithmetic.
- Swift `assert`/`precondition` → `debug_assert!`. Swift optionals →
  `Option` — never `unwrap()` where the Swift code handles `nil`.
- Platform APIs (MIDI, audio, mic, storage, notation) are the only
  sanctioned divergences — see `docs/platform-substitutions.md`. Any other
  divergence is a bug unless a comment explains *why* the platforms force it.

## Porting the tests

Port `Tests/KeyInSightTests/` module by module alongside the code it tests.
Every `XCTAssert` becomes an `assert!`/`assert_eq!`; keep test names
mappable to the originals. Tests call real production code — never copies.

## Dependency-ordered implementation

Before implementing any function: read the Swift source to identify
everything it calls, and implement incomplete dependencies first.

Port phase-by-phase in complete, testable modules (this worked for
clipper2-rust, box2d-rust, and box3d-rust; function-by-function tracking
did not). Expected order:

1. `Core` (NoteEvent, PitchSpelling, SplitMix64) — pure, no platform code
2. `Score` (score model → ExerciseGenerator → MusicXML import/export →
   repertoire library) + bundled pieces
3. `Engine` (Matcher → TempoMatcher → OctaveAnchor → SessionEngine →
   DemoDriver)
4. `Skill` (SkillModel) + `Persistence` (storage trait + schema port)
5. `Audio` (YinPitchDetector, MIDIFileEncoder, metronome/playback behind
   the AudioOut trait)
6. `Input` backends (simulated keyboard first — no hardware needed and it
   unblocks everything; then MIDI native, Web MIDI, mic, unplugged)
7. Notation renderer (SMuFL staff painting through DrawCtx)
8. `UI` (agg-gui widget tree mirroring TrainingView / SidePanel /
   BottomBar / PianoKeyboardView / ProgressPanel / sheets)

One green module per commit. Note: `SessionEngine.swift` is ~1400 lines —
its Rust port must be split into focused modules from the start (800-line
limit).

## Forbidden patterns

- `todo!()` / `unimplemented!()` / `panic!()` for missing functionality
- Stub functions or placeholder implementations
- Implementing without dependencies ready
- Marking functions complete prematurely
- "Close enough" or "good enough for now" implementations
- Guessing at divergences — when Rust and Swift disagree, instrument both
  and diff traces (the Swift app builds with `swift build` on a Mac; without
  a Mac, derive expectations from the Swift test suite, which encodes the
  intended behavior)
