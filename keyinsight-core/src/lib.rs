//! # KeyInSight Core
//!
//! Target-agnostic core for the KeyInSight port: the training engine, score
//! model, skill model, and every visible widget. Per `docs/architecture.md`,
//! all UI paints through agg-gui's `DrawCtx` — the native and WASM shells in
//! sibling crates own only the OS window/canvas, event loop, and platform
//! capability implementations.
//!
//! The crate is `wasm32`-clean: no `tokio`, no `winit`, no `wgpu`, no
//! `midir`, no `cpal`. Platform shells inject capabilities through the
//! [`KeyInSightPlatform`] trait.

pub mod audio;
pub mod core;
pub mod engine;
pub mod input;
pub mod notation;
pub mod persistence;
pub mod score;
pub mod skill;
pub mod ui;

pub use ui::{build_keyinsight_app, KeyInSightHandles, KeyInSightPlatform, UiFonts};

/// Version stamp reported by the demo site.
pub const PORT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopPlatform;
    impl KeyInSightPlatform for NoopPlatform {}

    /// The full app must build and survive a layout pass on both targets —
    /// the end-to-end smoke test CI runs (an exercise is generated,
    /// engraved, and laid out into the widget tree).
    #[test]
    fn full_app_builds_and_lays_out() {
        let (mut app, handles) = build_keyinsight_app(UiFonts::bundled(), NoopPlatform);
        app.layout(agg_gui::geometry::Size::new(1180.0, 620.0));
        handles.tick();
        assert_eq!(
            *handles.engine.borrow().phase(),
            crate::engine::Phase::Playing
        );
    }

    fn dump(widget: &dyn agg_gui::widget::Widget, depth: usize) {
        let b = widget.bounds();
        println!(
            "{:indent$}{} at ({:.0},{:.0}) {:.0}x{:.0}",
            "",
            widget.type_name(),
            b.x,
            b.y,
            b.width,
            b.height,
            indent = depth * 2
        );
        for child in widget.children() {
            dump(child.as_ref(), depth + 1);
        }
    }

    /// Diagnostic: print the laid-out widget tree (`cargo test dump_tree
    /// -- --nocapture --ignored`).
    #[test]
    #[ignore]
    fn dump_tree() {
        let (mut app, _handles) = build_keyinsight_app(UiFonts::bundled(), NoopPlatform);
        app.layout(agg_gui::geometry::Size::new(1180.0, 620.0));
        dump(app.root(), 0);
    }
}
