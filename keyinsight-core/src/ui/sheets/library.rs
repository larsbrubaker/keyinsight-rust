//! The repertoire Library sheet — `UI/LibrarySheet.swift` as a 620×420
//! modal: bundled pieces with detail lines, per-piece play stats, and
//! MusicXML import on platforms that provide a file picker.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use agg_gui::geometry::Size;
use agg_gui::layout_props::Insets;
use agg_gui::widget::Widget;
use agg_gui::widgets::{
    Button, Conditional, FlexColumn, FlexRow, Label, ModalSheet, Padding, ScrollView, Separator,

};

use crate::score::{MusicXmlImporter, RepertoireLibrary, RepertoirePiece};
use crate::ui::app::SharedPlatform;
use crate::ui::fonts::{icon, size, UiFonts};
use crate::ui::palette;
use crate::ui::side_panel::SidePanelCells;
use crate::ui::{DynamicLabel, InfoRow, InfoRows};

use super::Engine;

/// The Swift `.frame(width: 620, height: 420)`.
const SHEET_SIZE: Size = Size {
    width: 620.0,
    height: 420.0,
};

pub fn build_library_sheet(
    engine: &Engine,
    fonts: &UiFonts,
    cells: &SidePanelCells,
    platform: &SharedPlatform,
) -> Box<dyn Widget> {
    let visible = Rc::clone(&cells.show_library);
    let import_error: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let mut column = FlexColumn::new().with_gap(0.0);

    // Header: title + Import + Done (padding 14).
    {
        let mut header = FlexRow::new().with_gap(8.0).with_padding(14.0);
        header = header.add(Box::new(
            Label::new("Library", Arc::clone(&fonts.bold)).with_font_size(size::TITLE2),
        ));
        header = header.add_flex(Box::new(crate::ui::hspacer()), 1.0);
        if platform.supports_musicxml_import() {
            let import_engine = Rc::clone(engine);
            let import_visible = Rc::clone(&visible);
            let import_platform = Rc::clone(platform);
            let error = Rc::clone(&import_error);
            header = header.add(Box::new(
                Button::new("Import MusicXML…", Arc::clone(&fonts.regular))
                    .with_subtle().with_active_fn(|| false)
                    .on_click(move || {
                        error.borrow_mut().clear();
                        let engine = Rc::clone(&import_engine);
                        let visible = Rc::clone(&import_visible);
                        let error = Rc::clone(&error);
                        import_platform.open_musicxml(Box::new(move |data, name| {
                            match MusicXmlImporter::parse(&data, &name) {
                                Ok(imported) => {
                                    engine.borrow_mut().start_piece(RepertoirePiece {
                                        slug: format!("import:{name}"),
                                        title: imported.title,
                                        exercise: imported.exercise,
                                    });
                                    visible.set(false);
                                }
                                Err(err) => {
                                    *error.borrow_mut() = err.to_string();
                                }
                            }
                            agg_gui::animation::request_draw();
                        }));
                    }),
            ));
        }
        {
            let close = Rc::clone(&visible);
            header = header.add(Box::new(
                Button::new("Done", Arc::clone(&fonts.regular)).with_subtle().with_active_fn(|| false).on_click(move || {
                    close.set(false);
                    agg_gui::animation::request_draw();
                }),
            ));
        }
        column = column.add(Box::new(header));
    }

    // Import error line (visible while non-empty; the specific reason
    // per OQ-10, never silent stripping).
    {
        let watch = Rc::clone(&import_error);
        let has_error = crate::ui::side_panel::watch_cell(move || !watch.borrow().is_empty());
        let text = Rc::clone(&import_error);
        let error_rows = InfoRows::new(fonts, move || {
            let error = text.borrow();
            if error.is_empty() {
                Vec::new()
            } else {
                vec![InfoRow::text(error.clone(), size::CALLOUT)
                    .with_icon(icon::WARNING)
                    .with_color(palette::RED)]
            }
        });
        column = column.add(Box::new(Conditional::new(
            has_error,
            Box::new(Padding::new(
                Insets {
                    left: 14.0,
                    right: 14.0,
                    top: 0.0,
                    bottom: 8.0,
                },
                Box::new(error_rows),
            )),
        )));
    }

    column = column.add(Box::new(Separator::horizontal().with_line_inset(0.0)));

    // Piece rows, easiest first (bundled order is already curated; match
    // the Swift list order).
    let mut list = FlexColumn::new().with_gap(4.0).with_padding(10.0);
    for piece in RepertoireLibrary::bundled() {
        list = list.add(piece_row(engine, fonts, &visible, piece));
    }
    column = column.add_flex(
        Box::new(ScrollView::new(Box::new(list))),
        1.0,
    );

    Box::new(ModalSheet::new(visible, Box::new(column)).with_panel_size(SHEET_SIZE))
}

/// `title + detail | stats | Play` — one List row.
fn piece_row(
    engine: &Engine,
    fonts: &UiFonts,
    visible: &Rc<Cell<bool>>,
    piece: RepertoirePiece,
) -> Box<dyn Widget> {
    let text = FlexColumn::new()
        .with_fit_width(true)
        .with_gap(2.0)
        .add(Box::new(
            Label::new(piece.title.clone(), Arc::clone(&fonts.bold)).with_font_size(size::BODY),
        ))
        .add(Box::new(
            Label::new(piece_detail(&piece), Arc::clone(&fonts.regular))
                .with_font_size(size::CALLOUT)
                .with_dim(true),
        ));

    let stats = {
        let engine = Rc::clone(engine);
        let slug = piece.slug.clone();
        DynamicLabel::new(
            move || match engine.borrow().piece_stats(&slug) {
                Some((plays, best)) => {
                    format!("{plays}× · best {}%", (best * 100.0).round() as i64)
                }
                None => String::new(),
            },
            Arc::clone(&fonts.regular),
        )
        .with_font_size(size::CALLOUT)
        .with_dim(true)
    };

    let play = {
        let engine = Rc::clone(engine);
        let close = Rc::clone(visible);
        Button::new("Play", Arc::clone(&fonts.regular))
            .with_subtle().with_active_fn(|| false)
            .with_compact()
            .on_click(move || {
                engine.borrow_mut().start_piece(piece.clone());
                close.set(false);
                agg_gui::animation::request_draw();
            })
    };

    Box::new(
        FlexRow::new()
            .with_gap(10.0)
            .with_padding(4.0)
            .add(Box::new(text))
            .add_flex(Box::new(crate::ui::hspacer()), 1.0)
            .add(Box::new(stats))
            .add(Box::new(play)),
    )
}

/// `"8 measures · 14 notes · C major · difficulty 1.2"`.
fn piece_detail(piece: &RepertoirePiece) -> String {
    let exercise = &piece.exercise;
    let key = ["C major", "G major", "D major"][exercise.fifths.clamp(0, 2) as usize];
    format!(
        "{} measures · {} notes · {} · difficulty {:.1}",
        exercise.measures().len(),
        exercise.sounded_notes().len(),
        key,
        piece.difficulty_index()
    )
}
