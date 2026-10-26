//! The code editor: [`EditorComponent`] shows and edits a [`model::Editor`] with token
//! highlighting from a [`SyntaxHighlighter`]; with `viewer(true)` it is a read-only code view.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`EditorComponent`], [`EditorComponentResponse`] | The editor widget: editable code, or a read-only view of large text with folding and find. |
//! | [`GutterIcon`], [`ChangeMarker`], [`RangeHighlight`], [`GutterClick`], [`NavigationRequest`], [`HoverInfo`] | What the caller hands to it (gutter icons, markers, highlights) and what it reports back. |
//! | [`SyntaxHighlighter`], [`TextAttributesKey`], [`Span`] | The highlighter interface. |
//! | [`PlainText`], [`Json`], [`ExtendedJson`], [`JavaScript`], [`GraphQl`], [`Xml`] | Ready-made highlighters. |
//! | [`TemplateHighlighter`], [`TemplateToken`], [`TemplateAt`], [`template_at`] | `{{variable}}` templates over any highlighter. |
//! | [`model`] | The JetBrains-faithful, UI-free editing logic: document, carets, actions, undo, keymap. |

mod decorations;
mod editor_component;
mod find;
mod highlight;
mod lookup;
pub mod model;
mod selection_shape;
mod template;

pub use decorations::{
    ChangeKind, ChangeMarker, GutterArea, GutterClick, GutterIcon, HighlightSeverity, HoverInfo,
    NavigationKind, NavigationRequest, RangeHighlight,
};
pub use editor_component::{EditorComponent, EditorComponentResponse};
pub use highlight::{
    ExtendedJson, GraphQl, JavaScript, Json, PlainText, Span, SyntaxHighlighter, TextAttributesKey,
    Xml,
};
pub(crate) use highlight::{styled_job, token_format};
pub use template::{TemplateAt, TemplateHighlighter, TemplateToken, template_at};

use crate::theme::Theme;
use crate::tokens::radius;
use egui::{FontId, Rect, StrokeKind, TextStyle, Ui};

/// Texts up to this many bytes are exposed to screen readers as the node's value.
const A11Y_VALUE_LIMIT: usize = 64 * 1024;

/// The monospace font and the width of one of its cells.
fn code_font(ui: &Ui) -> (FontId, f32) {
    let font = TextStyle::Monospace.resolve(ui.style());
    let cell = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'M'));
    (font, cell)
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

/// The left margin that centers an editor's text in Distraction Free and presentation mode
/// (IntelliJ `editor.distraction.free.margin` = -1, the automatic rule): the text area of
/// `right_margin` columns of `cell` width, after `lead` (gutter and padding), is centered in
/// `width`. Never negative.
pub fn distraction_free_margin(width: f32, lead: f32, cell: f32, right_margin: usize) -> f32 {
    let content = lead + right_margin as f32 * cell;
    if !width.is_finite() || !content.is_finite() {
        return 0.0;
    }
    ((width - content) / 2.0).max(0.0)
}
