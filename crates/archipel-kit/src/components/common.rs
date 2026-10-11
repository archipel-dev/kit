//! Painting helpers shared by the components.

use crate::theme::Colors;
use egui::{Color32, FontId, Galley, Painter, Pos2, Rect, Ui, WidgetText};
use std::sync::Arc;

/// Lays out `text` on one line in `font`, without wrapping.
pub(crate) fn galley(ui: &Ui, text: impl Into<WidgetText>, font: FontId) -> Arc<Galley> {
    text.into().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::FontSelection::FontId(font),
    )
}

/// Lays out `text` on one line in `font`, cut at `max_width` with `…`.
pub(crate) fn galley_elided(ui: &Ui, text: &str, font: FontId, max_width: f32) -> Arc<Galley> {
    let mut job =
        egui::text::LayoutJob::simple_singleline(text.to_owned(), font, Color32::PLACEHOLDER);
    job.wrap.max_width = max_width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    ui.painter().layout_job(job)
}

/// Paints `galley` at `pos` in `color`, ignoring the style's global text color override.
pub(crate) fn text(painter: &Painter, pos: Pos2, galley: Arc<Galley>, color: Color32) {
    painter.galley_with_override_text_color(pos, galley, color);
}

/// Paints `galley` vertically centered in `rect`, starting at `x`.
pub(crate) fn text_left(
    painter: &Painter,
    rect: Rect,
    x: f32,
    galley: Arc<Galley>,
    color: Color32,
) {
    let y = rect.center().y - galley.size().y / 2.0;
    text(painter, egui::pos2(x, y), galley, color);
}

pub(crate) use crate::parts::focus_ring;

/// Fill of a flat control (toolbar button, row, menu item) for its interaction state.
pub(crate) fn flat_fill(response: &egui::Response, colors: &Colors) -> Color32 {
    if response.is_pointer_button_down_on() {
        colors.pressed
    } else if response.hovered() {
        colors.hover
    } else {
        Color32::TRANSPARENT
    }
}

pub(crate) use crate::parts::row_pill;

/// `text` if `enabled`, else the disabled text color.
pub(crate) fn enabled_text(enabled: bool, color: Color32, colors: &Colors) -> Color32 {
    if enabled { color } else { colors.text_disabled }
}
