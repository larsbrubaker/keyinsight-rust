//! # WebAssembly Shell for KeyInSight
//!
//! Thinnest possible browser shim: everything platform-generic (canvas
//! sizing, wgpu/WebGL2 surface, the rAF loop, DOM pointer / wheel /
//! keyboard / clipboard listeners) lives in `demo_wgpu::web_shell`. This
//! crate contributes only what is genuinely specific to KeyInSight in a
//! browser: the [`KeyInSightPlatform`] implementation (Web MIDI, WebAudio,
//! getUserMedia, localStorage — added as those modules are ported; see
//! `docs/platform-substitutions.md`).

#![cfg(target_arch = "wasm32")]

use demo_wgpu::web_shell;
use keyinsight_core::{build_keyinsight_app, load_default_font, KeyInSightPlatform};
use wasm_bindgen::prelude::*;

/// Browser implementation of the platform capability surface.
struct WasmPlatform;

impl KeyInSightPlatform for WasmPlatform {}

#[wasm_bindgen(start)]
pub fn start() {
    web_shell::start(
        "keyinsight-canvas",
        || build_keyinsight_app(load_default_font(), WasmPlatform),
        // Per-frame tick. The session engine's clock feeds from here once
        // the Engine module is ported.
        || {},
    );
}

/// Report the port version to the hosting page (used by the status site).
#[wasm_bindgen]
pub fn port_version() -> String {
    keyinsight_core::PORT_VERSION.to_string()
}
