//! Painting helpers shared by the components.

use crate::theme::Colors;
use crate::tokens::{radius, size};
use egui::{
    Color32, CornerRadius, FontId, Galley, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, WidgetText,
};
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

/// Keyboard focus ring around `rect`.
pub(crate) fn focus_ring(painter: &Painter, rect: Rect, corner: u8, colors: &Colors) {
    painter.rect_stroke(
        rect.expand(1.0),
        CornerRadius::same(corner.saturating_add(1)),
        Stroke::new(size::FOCUS_RING, colors.focus_ring),
        StrokeKind::Outside,
    );
}

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

/// The rounded selection/hover pill of a row spanning `rect`.
pub(crate) fn row_pill(painter: &Painter, rect: Rect, fill: Color32) {
    if fill != Color32::TRANSPARENT {
        let pill = rect.shrink2(egui::vec2(size::ROW_PILL_INSET, 0.0));
        painter.rect_filled(pill, CornerRadius::same(radius::ROW), fill);
    }
}

/// `text` if `enabled`, else the disabled text color.
pub(crate) fn enabled_text(enabled: bool, color: Color32, colors: &Colors) -> Color32 {
    if enabled { color } else { colors.text_disabled }
}
