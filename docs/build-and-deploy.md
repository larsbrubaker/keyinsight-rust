# Build, test, and deploy

## Commands

```powershell
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Hot-reload native shell (requires `cargo install cargo-watch` once)
cargo dev

# WASM build into the demo site
wasm-pack build keyinsight-wasm --target web --out-dir ../demo/public/pkg --no-typescript
```

`default-members` excludes `keyinsight-wasm` so plain `cargo build` doesn't
drag wasm-only deps into a native build.

## Running tests

```powershell
cargo test --workspace                    # everything
cargo test -p keyinsight-core matcher     # one module's tests
cargo test test_name -- --exact           # one test
cargo test -- --nocapture                 # with output
```

## Demo site (GitHub Pages)

The wasm demo site (`demo/`) is the deployed app: the full trainer running
in the browser (simulated-keyboard input works everywhere; Web MIDI where
the browser supports it). Until the training loop lands, the site is a
status/landing page that loads the wasm build and reports the port version —
keeping the whole pipeline green from day one.

```powershell
cd demo
bun install
bun run dev      # local dev server (builds nothing Rust — run wasm-pack first)
bun run build    # production bundle into demo/dist
```

## CI / deployment

- `.github/workflows/ci.yml` — native build + `cargo test --workspace`, plus
  a wasm-pack build, on every push/PR. Clones `larsbrubaker/agg-gui` as a
  sibling so the `[patch.crates-io]` path dep resolves.
- `.github/workflows/deploy-demo.yml` — on push to main: wasm-pack + vite
  build, deploy `demo/dist` to GitHub Pages.
