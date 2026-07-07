# keyinsight-rust

A cross-platform port of [KeyInSight](https://github.com/kevinepope/KeyInSight)
— a piano sight-reading trainer that builds the *see-note → press-key* reflex
the way touch-typing trainers work: adaptive generated exercises, tight
feedback loops, no music-reading prerequisite.

The original is a macOS Swift/SwiftUI app. This port is Rust +
[agg-gui](https://github.com/larsbrubaker/agg-gui), producing:

- a **native desktop app** (Windows / macOS / Linux via winit + wgpu), and
- a **browser app** (WASM, deployed to GitHub Pages).

## Status

**Scaffold — the port has not started yet.** The workspace, CI, WASM/Pages
pipeline, and porting rules (see [CLAUDE.md](CLAUDE.md)) are in place; the
demo site is a status page until the training loop lands.

## Layout

| Path | Contents |
|---|---|
| `keyinsight-swift-reference/` | The pinned Swift source being ported (git submodule) |
| `docs/` | Porting rules, platform substitutions, architecture, build/deploy |
| `keyinsight-core/` | The entire app: engine, score model, skill model, widgets. `wasm32`-clean. |
| `keyinsight-native/` | Desktop shim (platform impl over `demo_wgpu::native_shell`) |
| `keyinsight-wasm/` | Browser shim (platform impl over `demo_wgpu::web_shell`) |
| `demo/` | Vite site that hosts the WASM build (GitHub Pages) |

## Building

```powershell
# Clone with the Swift reference submodule
git clone --recurse-submodules https://github.com/larsbrubaker/keyinsight-rust.git

# agg-gui is path-patched to a sibling checkout
git clone https://github.com/larsbrubaker/agg-gui.git ../agg-gui

cargo test --workspace     # build + tests
cargo run -p keyinsight-native   # desktop app

# Browser build
wasm-pack build keyinsight-wasm --target web --out-dir ../demo/public/pkg --no-typescript
cd demo; bun install; bun run dev
```

## Porting plan

Phase 1 is the truest port we can make — module by module in dependency
order, with the Swift test suite ported alongside as the acceptance gate.
Phase 2 takes over development in the agg-gui environment. The rules live in
[CLAUDE.md](CLAUDE.md) and the `docs/` directory:
[porting](docs/porting.md), [platform substitutions](docs/platform-substitutions.md)
(CoreMIDI → midir / Web MIDI, AVAudioEngine → cpal / WebAudio, Verovio →
SMuFL staff renderer, GRDB → storage trait),
[architecture](docs/architecture.md), and
[build & deploy](docs/build-and-deploy.md).

## License

MIT for the Rust code in this repository. The Swift reference submodule
remains under its upstream repository's terms.
