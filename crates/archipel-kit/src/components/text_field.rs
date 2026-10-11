//! Single-line text inputs.

use super::code::{Highlighter, TemplateAt, TokenKind, template_at};
use super::{IconButton, common};
use crate::icons::{self, Glyph, Icon};
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, Rect, Response, Stroke, Ui, Widget, pos2, vec2};
use std::sync::Arc;

/// A single-line text input with an optional label, placeholder, leading icon, clear
/// button and validation message.
///
/// The returned [`Response`] is the text edit's: `changed()` after every edit,
/// `lost_focus()` when the user leaves the field.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::TextField;
///
/// let mut name = String::new();
/// ui.add(TextField::new(&mut name).label("Project name").placeholder("my-app"));
///
/// let mut query = String::new();
/// ui.add(TextField::search(&mut query));
///
/// let mut port = "80a".to_owned();
/// let error = port.parse::<u16>().is_err().then_some("Enter a number from 0 to 65535");
/// ui.add(TextField::new(&mut port).label("Port").error(error));
/// # });
/// ```
///
/// # Highlighting and `{{variables}}`
///
/// [`highlighter`](TextField::highlighter) colors the text with any
/// [`Highlighter`], keeping everything else of the field: height, focus ring, placeholder,
/// clear button. With a [`TemplateHighlighter`](super::code::TemplateHighlighter) it
/// is the URL bar and header value input of an HTTP client; [`show`](TextField::show)
/// then also reports the caret and the `{{variable}}` at it, to anchor an autocomplete. See
/// the [code page](https://github.com/archipel-dev/kit/blob/main/docs/components/code.md).
#[must_use = "add it with `ui.add(field)`"]
pub struct TextField<'t> {
    text: &'t mut String,
    label: Option<String>,
    placeholder: Option<String>,
    error: Option<String>,
    leading: Option<Icon>,
    clearable: bool,
    password: bool,
    enabled: bool,
    monospace: bool,
    width: Option<f32>,
    highlighter: Option<&'t dyn Highlighter>,
}

/// What [`TextField::show`] reports besides the usual [`Response`].
pub struct TextFieldOutput {
    /// The text edit's response: `changed()` after every edit, `lost_focus()` on leaving.
    pub response: Response,
    /// The field lost focus because <kbd>Enter</kbd> was pressed. Enter never inserts a
    /// newline.
    pub submitted: bool,
    /// The caret as a screen rectangle (a thin bar the height of the line): anchor a
    /// popup at its left-bottom. `None` before the field was ever focused.
    pub caret_rect: Option<Rect>,
    /// The `{{variable}}` the caret is in or just after (see [`template_at`]). It is kept
    /// while focus is on a suggestion list you clicked, so a click can still complete it:
    /// open the list only while `response.has_focus()`.
    pub template: Option<TemplateAt>,
}

impl TextFieldOutput {
    /// Replaces the [`template`](Self::template) token in `text` with `{{name}}`, puts the
    /// caret after it and puts the focus back in the field. Returns whether there was a token.
    pub fn insert_completion(&self, ctx: &egui::Context, text: &mut String, name: &str) -> bool {
        let Some(token) = &self.template else {
            return false;
        };
        let caret = token.insert_completion(text, name);
        let chars = text[..caret].chars().count();
        let id = self.response.id;
        let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(chars))));
        state.store(ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
        true
    }
}

impl<'t> TextField<'t> {
    /// An input editing `text`.
    pub fn new(text: &'t mut String) -> Self {
        Self {
            text,
            label: None,
            placeholder: None,
            error: None,
            leading: None,
            clearable: false,
            password: false,
            enabled: true,
            monospace: false,
            width: None,
            highlighter: None,
        }
    }

    /// A search input: magnifier icon, "Search" placeholder and a clear button.
    pub fn search(text: &'t mut String) -> Self {
        Self::new(text)
            .leading_icon(Glyph::Search)
            .placeholder("Search")
            .clearable(true)
    }

    /// A visible label above the input; it is also the input's accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Hint shown while the input is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Validation message: shown below the input, which gets an error outline. `None`
    /// clears it, so `.error(validate(&text))` works directly.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Icon drawn inside the input, before the text.
    pub fn leading_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.leading = Some(icon.into());
        self
    }

    /// Shows a clear button while the input has text.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Masks the text.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Disabled inputs are dimmed and read-only.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Uses the code font (for paths, URLs, identifiers).
    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    /// Colors the text with `highlighter`: a URL, a header value, `{{variables}}` (see
    /// [`TemplateHighlighter`](super::code::TemplateHighlighter)). Ignored for
    /// [`password`](Self::password) fields. The text stays one line: <kbd>Enter</kbd>
    /// submits.
    pub fn highlighter(mut self, highlighter: &'t dyn Highlighter) -> Self {
        self.highlighter = Some(highlighter);
        self
    }

    /// Total width in points (default: the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Widget for TextField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

impl TextField<'_> {
    /// Adds the field and reports the caret, the `{{variable}}` at it and whether
    /// <kbd>Enter</kbd> submitted it, besides the response.
    pub fn show(self, ui: &mut Ui) -> TextFieldOutput {
        let colors = Theme::colors(ui.ctx());
        let width = self.width.unwrap_or_else(|| ui.available_width());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let label_id = self.label.as_ref().map(|label| {
                ui.add(egui::Label::new(egui::RichText::new(label).color(
                    common::enabled_text(self.enabled, colors.text, &colors),
                )))
                .id
            });

            let (rect, frame_response) =
                ui.allocate_exact_size(vec2(width, size::CONTROL_HEIGHT), egui::Sense::hover());
            // Painted after the content, once focus is known, but drawn beneath it.
            let background = ui.painter().add(egui::Shape::Noop);
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink2(vec2(8.0, 0.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            content.spacing_mut().item_spacing.x = size::ICON_TEXT_GAP;
            if let Some(icon) = self.leading {
                icons::show(&mut content, icon, size::ICON, colors.text_placeholder);
            }
            let show_clear = self.clearable && !self.text.is_empty() && self.enabled;
            let clear_width = if show_clear {
                size::ICON_BUTTON + size::ICON_TEXT_GAP
            } else {
                0.0
            };
            let mut edit = egui::TextEdit::singleline(self.text)
                .frame(false)
                .password(self.password)
                .interactive(self.enabled)
                .text_color(common::enabled_text(self.enabled, colors.text, &colors))
                .desired_width(content.available_width() - clear_width)
                .margin(vec2(0.0, 0.0));
            let font = if self.monospace {
                edit = edit.font(egui::TextStyle::Monospace);
                egui::TextStyle::Monospace.resolve(content.style())
            } else {
                egui::TextStyle::Body.resolve(content.style())
            };
            let highlighter = self.highlighter.filter(|_| !self.password);
            let enabled = self.enabled;
            let theme = Theme::current(content.ctx());
            let mut layouter =
                |ui: &Ui, buf: &dyn egui::TextBuffer, _wrap: f32| -> Arc<egui::Galley> {
                    let job = highlight_job(buf.as_str(), highlighter, &font, &theme, enabled);
                    ui.fonts_mut(|fonts| fonts.layout_job(job))
                };
            if highlighter.is_some() {
                edit = edit.layouter(&mut layouter);
            }
            if let Some(placeholder) = &self.placeholder {
                edit =
                    edit.hint_text(egui::RichText::new(placeholder).color(colors.text_placeholder));
            }
            // Disabled through the ui, so the field reports disabled to screen readers and
            // tests, not just ignores input.
            let shown = content
                .add_enabled_ui(self.enabled, |ui| edit.show(ui))
                .inner;
            let galley = shown.galley.clone();
            let galley_pos = shown.galley_pos;
            // egui drops the cursor when focus leaves; the last caret is kept per field so a
            // click on a suggestion can still complete the token.
            let caret_key = shown.response.id.with("archipel-caret");
            let cursor = if shown.response.has_focus() {
                content.data_mut(|d| match shown.cursor_range {
                    Some(range) => d.insert_temp(caret_key, range),
                    None => {
                        d.remove_temp::<CCursorRange>(caret_key);
                    }
                });
                shown.cursor_range
            } else {
                content.data(|d| d.get_temp::<CCursorRange>(caret_key))
            };
            let mut edited = shown.response;
            if let Some(id) = label_id {
                edited = edited.labelled_by(id);
            }
            if show_clear
                && content
                    .add(IconButton::new(Glyph::Close, "Clear"))
                    .on_hover_cursor(egui::CursorIcon::Default)
                    .clicked()
            {
                self.text.clear();
                edited.mark_changed();
                edited.request_focus();
            }

            let focused = edited.has_focus();
            if let Some(highlighter) = highlighter {
                underline_unresolved(
                    &content.painter().with_clip_rect(edited.rect),
                    highlighter,
                    self.text,
                    &galley,
                    galley_pos,
                    colors.error,
                );
            }
            let (caret_rect, template) = match cursor {
                Some(range) => {
                    let r = galley.pos_from_cursor(range.primary);
                    let rect = Rect::from_min_max(
                        pos2(r.min.x, r.min.y) + galley_pos.to_vec2(),
                        pos2(r.min.x + 1.0, r.max.y) + galley_pos.to_vec2(),
                    );
                    let byte = self
                        .text
                        .char_indices()
                        .nth(range.primary.index)
                        .map_or(self.text.len(), |(b, _)| b);
                    (Some(rect), template_at(self.text, byte))
                }
                None => (None, None),
            };
            let submitted = edited.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let stroke = if self.error.is_some() {
                Stroke::new(
                    if focused { size::FOCUS_RING } else { 1.0_f32 },
                    colors.error,
                )
            } else if focused {
                Stroke::new(size::FOCUS_RING, colors.accent)
            } else {
                Stroke::new(1.0_f32, colors.border)
            };
            let fill = if self.enabled {
                colors.field
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().set(
                background,
                egui::epaint::RectShape::new(
                    rect,
                    CornerRadius::same(radius::CONTROL),
                    fill,
                    stroke,
                    egui::StrokeKind::Inside,
                ),
            );
            if self.enabled && frame_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
            if let Some(error) = &self.error {
                ui.add(egui::Label::new(
                    egui::RichText::new(error)
                        .color(colors.error)
                        .font(typography::body(typography::SMALL + 1.0)),
                ));
                edited = edited.on_hover_text(error);
            }
            TextFieldOutput {
                response: edited,
                submitted,
                caret_rect,
                template,
            }
        })
        .inner
    }
}

/// The layout of `text` colored by `highlighter`. Template kinds take the link, error and
/// warning tones; the others the editor's syntax colors.
fn highlight_job(
    text: &str,
    highlighter: Option<&dyn Highlighter>,
    font: &egui::FontId,
    theme: &Theme,
    enabled: bool,
) -> LayoutJob {
    let colors = &theme.colors;
    let dim = |c: Color32| if enabled { c } else { c.gamma_multiply(0.55) };
    let format = |kind: TokenKind| {
        let color = match kind {
            TokenKind::Plain => colors.text,
            TokenKind::Resolved => Tone::Link.color(colors),
            TokenKind::Unresolved => Tone::Error.color(colors),
            TokenKind::Secret => Tone::Warning.color(colors),
            other => other.color(&theme.editor),
        };
        TextFormat {
            font_id: font.clone(),
            color: dim(color),
            background: if kind == TokenKind::Secret {
                colors.warning.gamma_multiply(0.18)
            } else {
                Color32::TRANSPARENT
            },
            ..TextFormat::default()
        }
    };
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let mut spans = Vec::new();
    if let Some(highlighter) = highlighter {
        highlighter.highlight_text(text, &mut spans);
    }
    let mut at = 0;
    for span in &spans {
        let (a, b) = (span.start.max(at).min(text.len()), span.end.min(text.len()));
        if a >= b || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
            continue;
        }
        if a > at {
            job.append(&text[at..a], 0.0, format(TokenKind::Plain));
        }
        job.append(&text[a..b], 0.0, format(span.kind));
        at = b;
    }
    job.append(&text[at..], 0.0, format(TokenKind::Plain));
    job
}

/// Draws a dotted line under the unresolved variables.
fn underline_unresolved(
    painter: &egui::Painter,
    highlighter: &dyn Highlighter,
    text: &str,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    color: Color32,
) {
    let mut spans = Vec::new();
    highlighter.highlight_text(text, &mut spans);
    let x_of = |byte: usize| {
        let chars = text[..byte].chars().count();
        galley.pos_from_cursor(CCursor::new(chars))
    };
    for span in spans.iter().filter(|s| s.kind == TokenKind::Unresolved) {
        if span.end > text.len()
            || !text.is_char_boundary(span.start)
            || !text.is_char_boundary(span.end)
        {
            continue;
        }
        let (a, b) = (x_of(span.start), x_of(span.end));
        let y = galley_pos.y + a.max.y;
        let mut x = galley_pos.x + a.min.x;
        let end = galley_pos.x + b.min.x;
        while x < end {
            let dot = Rect::from_min_max(pos2(x, y - 1.5), pos2((x + 1.0).min(end), y - 0.5));
            painter.rect_filled(dot, 0.0, color);
            x += 3.0;
        }
    }
}
