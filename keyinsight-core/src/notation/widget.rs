//! The agg-gui widget that paints the engraved score with its feedback
//! overlays. Replaces the Swift `NotationView`/WKWebView: the score paints
//! on a light page (music is always light — docs/platform-substitutions.md),
//! per-note colors come from the controller's states, and the ghost /
//! ticks / follow cursor are ordinary painting.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use agg_gui::color::Color;
use agg_gui::draw_ctx::DrawCtx;
use agg_gui::event::{Event, EventResult};
use agg_gui::geometry::{Point, Rect, Size};
use agg_gui::text::Font;
use agg_gui::widget::Widget;

use crate::notation::{NotationController, NoteState};

pub struct NotationWidget {
    controller: Rc<RefCell<NotationController>>,
    /// Host clock for the follow schedule (injected so native and WASM
    /// share code; seconds, monotonic).
    now: Rc<dyn Fn() -> f64>,
    music_font: Arc<Font>,
    bounds: Rect,
    children: Vec<Box<dyn Widget>>,
    /// Content scale + offset of the last paint (hover hit-testing).
    scale: f64,
    offset_x: f64,
}

impl NotationWidget {
    pub fn new(controller: Rc<RefCell<NotationController>>, now: Rc<dyn Fn() -> f64>) -> Self {
        Self {
            controller,
            now,
            music_font: verovio_rust::leipzig_font(),
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            children: Vec::new(),
            scale: 1.0,
            offset_x: 0.0,
        }
    }

    fn layout_size(&self) -> Option<(f64, f64)> {
        let controller = self.controller.borrow();
        let renderer = controller.renderer.borrow();
        let layout = renderer.toolkit().current_layout()?;
        Some((layout.width, layout.height))
    }
}

impl Widget for NotationWidget {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn Widget>> {
        &mut self.children
    }

    fn layout(&mut self, available: Size) -> Size {
        // Reflow long scores into systems fitted to this viewport.
        if available.width > 0.0 && available.height > 0.0 {
            if let Ok(controller) = self.controller.try_borrow() {
                if let Ok(mut renderer) = controller.renderer.try_borrow_mut() {
                    renderer.fit_view(available.width, available.height);
                }
            }
        }
        available
    }

    fn paint(&mut self, ctx: &mut dyn DrawCtx) {
        let width = self.bounds.width;
        let height = self.bounds.height;

        // The page: music always renders light regardless of app theme.
        ctx.set_fill_color(Color::white());
        ctx.begin_path();
        ctx.rect(0.0, 0.0, width, height);
        ctx.fill();

        let Some((score_w, score_h)) = self.layout_size() else {
            return;
        };
        // Fit the engraving to the widget, centered horizontally, capped so
        // small exercises don't balloon.
        let scale = (width / score_w).min(height / score_h).min(1.6);
        self.scale = scale;
        self.offset_x = (width - score_w * scale) / 2.0;

        ctx.save();
        ctx.translate(self.offset_x, height - score_h * scale);
        ctx.scale(scale, scale);

        // Follow overlay: while playback-following, the scheduled group
        // paints as current on top of the stored states.
        let follow_ids = {
            let mut controller = self.controller.borrow_mut();
            let now = (self.now)();
            controller.follow_ids_at(now)
        };
        if self.controller.borrow().is_following() {
            agg_gui::animation::request_draw(); // keep the cursor moving
        }

        let controller = self.controller.borrow();
        let renderer = controller.renderer.borrow();
        let toolkit = renderer.toolkit();
        let Some(layout) = toolkit.current_layout() else {
            ctx.restore();
            return;
        };

        let mut options = verovio_rust::RenderOptions::default();
        for element in &layout.elements {
            let Some(id) = &element.id else { continue };
            if let Some(state) = controller.state_of(id) {
                options.overrides.insert(id.clone(), state.color());
            }
        }
        if let Some(ids) = &follow_ids {
            for id in ids {
                options
                    .overrides
                    .insert(id.clone(), NoteState::Current.color());
            }
        }

        // The toolkit draws y-up given the top edge of the score box.
        toolkit.render(ctx, &self.music_font, 0.0, score_h, &options);

        // Ghost note: gray notehead at the played staff position, aligned
        // with the expected note (ports the HTML overlay math — half a
        // staff space per diatonic step).
        if let Some(ghost) = controller.ghost() {
            if let Some((x, y_top, w, h)) = toolkit.element_bounds(&ghost.expected_id) {
                let staff_space = 10.0; // LayoutOptions::default().staff_space
                let cx = x + w / 2.0;
                let cy_down = y_top + h / 2.0 - ghost.offset_steps as f64 * staff_space / 2.0;
                let cy = score_h - cy_down;
                let gray = Color::from_rgba8(0x8A, 0x8A, 0x8A, 64);
                ctx.set_fill_color(gray);
                ctx.begin_path();
                ctx.circle(cx, cy, w * 0.5);
                ctx.fill();
                ctx.set_stroke_color(Color::from_rgb8(0x8A, 0x8A, 0x8A));
                ctx.set_line_width(2.0);
                ctx.begin_path();
                ctx.circle(cx, cy, w * 0.5);
                ctx.stroke();
            }
        }

        // Timing ticks: ◂ early / ▸ late above the note.
        for tick in controller.ticks() {
            if let Some((x, y_top, w, _h)) = toolkit.element_bounds(&tick.id) {
                let color = Color::from_rgb8(0xB8, 0x86, 0x0B);
                ctx.set_fill_color(color);
                ctx.set_font(Arc::clone(&self.music_font));
                let cx = x + w / 2.0;
                let cy = score_h - (y_top - 14.0);
                // Simple triangle glyphs drawn as paths (the UI font isn't
                // loaded here; a filled triangle reads identically).
                let s = 5.0;
                ctx.begin_path();
                if tick.early {
                    ctx.move_to(cx + s, cy + s);
                    ctx.line_to(cx - s, cy);
                    ctx.line_to(cx + s, cy - s);
                } else {
                    ctx.move_to(cx - s, cy + s);
                    ctx.line_to(cx + s, cy);
                    ctx.line_to(cx - s, cy - s);
                }
                ctx.close_path();
                ctx.fill();
            }
        }

        ctx.restore();
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        if let Event::MouseMove { pos } = event {
            self.route_hover(*pos);
        }
        EventResult::Ignored
    }

    fn type_name(&self) -> &'static str {
        "NotationWidget"
    }
}

impl NotationWidget {
    /// Hover-to-name: padded notehead hit boxes, nearest center wins (ports
    /// the Swift page's `noteHitAt`; the precise per-kind fallback arrives
    /// with non-note element hovers).
    fn route_hover(&self, pos: Point) {
        let controller = self.controller.borrow();
        let renderer = controller.renderer.borrow();
        let Some(layout) = renderer.toolkit().current_layout() else {
            return;
        };
        let Some((_, score_h)) = self.layout_size() else {
            return;
        };
        // Widget y-up → layout y-down.
        let lx = (pos.x - self.offset_x) / self.scale;
        let ly = score_h - (pos.y - (self.bounds.height - score_h * self.scale)) / self.scale;
        const HIT_PAD: f64 = 10.0;
        let mut best: Option<(String, f64)> = None;
        for (id, &(x, y_top, w, h)) in &layout.bounds_by_id {
            if lx < x - HIT_PAD
                || lx > x + w + HIT_PAD
                || ly < y_top - HIT_PAD
                || ly > y_top + h + HIT_PAD
            {
                continue;
            }
            let cx = x + w / 2.0;
            let cy = y_top + h / 2.0;
            let d = (lx - cx).powi(2) + (ly - cy).powi(2);
            if best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                best = Some((id.clone(), d));
            }
        }
        drop(renderer);
        match best {
            Some((id, _)) => {
                let kind = if id.starts_with("rest-") { "rest" } else { "note" };
                controller.send_hover(kind, &id)
            }
            None => controller.send_hover("", ""),
        }
    }
}
