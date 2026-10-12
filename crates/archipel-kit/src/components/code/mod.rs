//! Code views and editors: [`CodeView`] shows large read-only text, [`CodeEditor`] edits
//! text in place, both with token highlighting from a [`Highlighter`].

mod editor;
mod find;
mod highlight;
mod template;
mod text;
mod view;

pub use editor::CodeEditor;
pub use highlight::{
    ExtendedJson, GraphQl, Highlighter, JavaScript, Json, PlainText, Span, TokenKind, Xml,
};
pub(crate) use highlight::{styled_job, token_format};
pub use template::{TemplateAt, TemplateHighlighter, TemplateToken, template_at};
pub use view::{CodeView, CodeViewResponse};

use crate::theme::Theme;
use crate::tokens::{radius, size};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, StrokeKind, TextStyle, Ui};

/// Texts up to this many bytes are exposed to screen readers as the node's value.
const A11Y_VALUE_LIMIT: usize = 64 * 1024;

/// The monospace font and the width of one of its cells.
fn code_font(ui: &Ui) -> (FontId, f32) {
    let font = TextStyle::Monospace.resolve(ui.style());
    let cell = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'M'));
    (font, cell)
}

/// Width of a gutter showing line numbers up to `lines`, with a fold column when `fold`.
fn gutter_width(lines: usize, cell: f32, numbers: bool, fold: bool) -> f32 {
    let mut width = 0.0;
    if numbers {
        width += lines.to_string().len().max(2) as f32 * cell + 2.0 * size::CODE_PADDING;
    }
    if fold {
        width += size::CHEVRON_COLUMN + if numbers { 0.0 } else { size::CODE_PADDING };
    }
    width
}

/// Paints one line number right-aligned in `gutter`, vertically centered on `y`.
fn paint_number(
    painter: &Painter,
    gutter: Rect,
    y: f32,
    number: usize,
    font: &FontId,
    color: Color32,
    fold: bool,
) {
    let right = gutter.right() - size::CODE_PADDING - if fold { size::CHEVRON_COLUMN } else { 0.0 };
    painter.text(
        Pos2::new(right, y),
        Align2::RIGHT_CENTER,
        number.to_string(),
        font.clone(),
        color,
    );
}

/// The editor box: its background, and the border (accent when focused) over the content.
fn paint_background(ui: &Ui, rect: Rect) -> egui::layers::ShapeIdx {
    let colors = Theme::current(ui.ctx());
    ui.painter().add(egui::epaint::RectShape::filled(
        rect,
        radius::CONTROL,
        colors.editor.background,
    ))
}

fn paint_border(ui: &Ui, rect: Rect, focused: bool) {
    let stroke = crate::parts::Surface::field_stroke(ui.ctx(), focused, false);
    ui.painter()
        .rect_stroke(rect, radius::CONTROL, stroke, StrokeKind::Inside);
}
