//! Themed single-line and multi-line text inputs.

use super::completion::Completer;
use super::popup::{Options, Placement, PopupChain, RowSpec, drive};
use super::text_extensions::{Extension, SearchHistory, extension_button};
use crate::common;
use crate::editor::{Highlighter, Span, TemplateAt, TokenKind, template_at};
use crate::editor::{styled_job, token_format};
use crate::icons::{self, Glyph, Icon};
use crate::openapi::ui::ValidationInfo;
use crate::theme::Theme;
use crate::tokens::{motion, radius, size, typography};
use egui::text::{CCursor, CCursorRange};
use egui::{Color32, CornerRadius, Id, Rect, Response, Stroke, Ui, Widget, pos2, vec2};
use std::cell::RefCell;
use std::sync::Arc;

/// A text input with an optional label, placeholder, leading icon, clear
/// button and validation message.
/// Use [`multiline`](Self::multiline) for wrapped text with an internal scroll area.
///
/// The returned [`Response`] is the text edit's: `changed()` after every edit,
/// `lost_focus()` when the user leaves the field.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::JBTextField;
///
/// let mut name = String::new();
/// ui.add(JBTextField::new(&mut name).label("Project name").placeholder("my-app"));
///
/// let mut query = String::new();
/// ui.add(JBTextField::search(&mut query));
///
/// let mut port = "80a".to_owned();
/// let error = port.parse::<u16>().is_err().then_some("Enter a number from 0 to 65535");
/// ui.add(JBTextField::new(&mut port).label("Port").error(error));
/// # });
/// ```
///
/// # Highlighting and `{{variables}}`
///
/// [`highlighter`](JBTextField::highlighter) colors the text with any
/// [`Highlighter`], keeping everything else of the field: height, focus ring, placeholder,
/// clear button. With a [`TemplateHighlighter`](crate::editor::TemplateHighlighter) it
/// is the URL bar and header value input of an HTTP client; [`show`](JBTextField::show)
/// then also reports the caret and the `{{variable}}` at it, to anchor an autocomplete. See
/// the [code page](https://github.com/archipel-dev/kit/blob/main/docs/components/code.md).
#[must_use = "add it with `ui.add(field)`"]
pub struct JBTextField<'t> {
    text: &'t mut String,
    label: Option<String>,
    name: Option<String>,
    rows: usize,
    placeholder: Option<String>,
    error: Option<String>,
    warning: bool,
    leading: Option<Icon>,
    clearable: bool,
    clear_on_escape: bool,
    spinner: bool,
    extensions: Vec<Extension>,
    history: Option<&'t mut SearchHistory>,
    password: bool,
    enabled: bool,
    monospace: bool,
    width: Option<f32>,
    id: Option<Id>,
    highlighter: Option<&'t dyn Highlighter>,
    completer: Option<&'t dyn Completer>,
    suggestions: Option<&'t [String]>,
}

/// What [`JBTextField::show`] reports besides the usual [`Response`].
#[non_exhaustive]
pub struct JBTextFieldOutput {
    /// The text edit's response: `changed()` after every edit, `lost_focus()` on leaving.
    pub response: Response,
    /// The field lost focus because <kbd>Enter</kbd> was pressed. Enter never inserts a
    /// newline in single-line mode. Always false in multi-line mode.
    pub submitted: bool,
    /// The caret as a screen rectangle (a thin bar the height of the line): anchor a
    /// popup at its left-bottom. `None` before the field was ever focused.
    pub caret_rect: Option<Rect>,
    /// The `{{variable}}` the caret is in or just after (see [`template_at`]). `None`
    /// unless the field has focus or the pointer is pressed or released this frame, so a
    /// click on a suggestion (which moves the focus first) can still complete the token.
    /// Open the list only while `response.has_focus()`.
    pub template: Option<TemplateAt>,
    /// The [extension](JBTextField::extension) (index in the order added) that was activated
    /// this frame by a click, or by <kbd>Space</kbd>/<kbd>Enter</kbd> on it.
    pub extension_clicked: Option<usize>,
    /// The field's frame, including the room of its icons and buttons.
    pub frame: Rect,
    /// Steps the spinner arrows asked for this frame (positive: up). Zero unless the field is
    /// a spinner.
    pub(crate) spin: i32,
}

impl JBTextFieldOutput {
    /// Replaces the [`template`](Self::template) token in `text` with `{{name}}`, puts the
    /// caret after it and puts the focus back in the field. Returns whether there was a token.
    ///
    /// It belongs to the frame of [`show`](JBTextField::show) that returned this output: the
    /// token's byte range is only valid for the text as it was then, so call it in that frame
    /// and not with an output kept from an earlier one.
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

impl<'t> JBTextField<'t> {
    /// An input editing `text`.
    pub fn new(text: &'t mut String) -> Self {
        Self {
            text,
            label: None,
            name: None,
            rows: 1,
            placeholder: None,
            error: None,
            warning: false,
            leading: None,
            clearable: false,
            clear_on_escape: false,
            spinner: false,
            extensions: Vec::new(),
            history: None,
            password: false,
            enabled: true,
            monospace: false,
            width: None,
            id: None,
            highlighter: None,
            completer: None,
            suggestions: None,
        }
    }

    /// Offers completions while typing: a list under the caret, filled by `completer`.
    /// <kbd>Up</kbd>/<kbd>Down</kbd> move in it, <kbd>Enter</kbd> or <kbd>Tab</kbd> insert the
    /// suggestion, <kbd>Esc</kbd> closes it; a click inserts one too. The list opens on
    /// every edit that has suggestions and closes when the field loses focus.
    pub fn completer(mut self, completer: &'t dyn Completer) -> Self {
        self.completer = Some(completer);
        self
    }

    /// Makes the field an editable combo box: free text, plus a chevron that lists
    /// `suggestions` below the field (a field name, a recent value). Typing filters the list
    /// (ignoring case); picking one replaces the text. The chevron,
    /// <kbd>Alt</kbd>+<kbd>Down</kbd> or <kbd>Ctrl</kbd>+<kbd>Space</kbd> open the full list.
    /// Use a [`ComboBox`](crate::openapi::ui::ComboBox) when only the listed values are valid. Replaces
    /// a [`completer`](Self::completer) and the clear button.
    pub fn suggestions(mut self, suggestions: &'t [String]) -> Self {
        self.suggestions = Some(suggestions);
        self
    }

    /// A search input: magnifier icon, "Search" placeholder and a clear button.
    pub fn search(text: &'t mut String) -> Self {
        Self::new(text)
            .leading_icon(Glyph::Search)
            .placeholder("Search")
            .clearable(true)
            .clear_on_escape(true)
    }

    /// A visible label above the input; it is also the input's accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// An accessible name without adding a visible label. Defaults to the visible label
    /// or placeholder. Use it for compact search fields and request-body editors.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// A multi-line input with `rows` visible lines (at least two). Enter inserts a
    /// newline; Tab leaves the field. Extra lines scroll inside the field.
    pub fn multiline(mut self, rows: usize) -> Self {
        self.rows = rows.max(2);
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

    /// Like [`error`](Self::error) for a [`ValidationInfo`]: an error gets the red outline, a
    /// [warning](ValidationInfo::warning) the yellow one (IntelliJ `ComponentValidator`).
    /// The message is shown under the field and as its tooltip. `None` clears it.
    pub fn validation(mut self, info: Option<&ValidationInfo>) -> Self {
        self.warning = info.is_some_and(|info| info.warning);
        self.error = info.map(|info| info.message.clone());
        self
    }

    /// Adds an icon button inside the field (IntelliJ `ExtendableTextField.Extension`).
    /// See [`Extension`]. Extensions are numbered in the order they are added.
    pub fn extension(mut self, extension: Extension) -> Self {
        self.extensions.push(extension);
        self
    }

    /// <kbd>Esc</kbd> clears the text while the field has any, instead of leaving it
    /// (IntelliJ `toClearTextOnEscape`). Default off; search fields turn it on.
    pub fn clear_on_escape(mut self, clear: bool) -> Self {
        self.clear_on_escape = clear;
        self
    }

    /// Gives the field a search history: the leading icon opens it (click, or
    /// <kbd>Alt</kbd>+<kbd>Down</kbd>), <kbd>Alt</kbd>+<kbd>Up</kbd> cycles to the previous
    /// entry, and a submitted text (<kbd>Enter</kbd>) is remembered. Used by
    /// [`SearchTextField`](crate::ui::SearchTextField).
    pub fn history(mut self, history: &'t mut SearchHistory) -> Self {
        self.history = Some(history);
        self
    }

    /// Draws stacked up and down arrows at the right edge: the field of a
    /// [`JBIntSpinner`](crate::ui::JBIntSpinner).
    pub(crate) fn spinner(mut self, spinner: bool) -> Self {
        self.spinner = spinner;
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

    /// The id of the text edit, like [`egui::TextEdit::id`]: the response's `id` is exactly
    /// this one, so a caller can move focus to the field with
    /// `ctx.memory_mut(|m| m.request_focus(id))` or `ui.memory_mut(...)`.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Colors the text with `highlighter`: a URL, a header value, `{{variables}}` (see
    /// [`TemplateHighlighter`](crate::editor::TemplateHighlighter)). Ignored for
    /// [`password`](Self::password) fields. In single-line mode <kbd>Enter</kbd> submits;
    /// multi-line fields wrap the highlighted text and insert newlines.
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

impl Widget for JBTextField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

impl JBTextField<'_> {
    /// Adds the field and reports the caret, the `{{variable}}` at it and whether
    /// <kbd>Enter</kbd> submitted it, besides the response.
    pub fn show(mut self, ui: &mut Ui) -> JBTextFieldOutput {
        let colors = Theme::colors(ui.ctx());
        // A layout that has run out of room must not make egui allocate a negative size.
        // Never narrower than `TextField.minimumSize`, nor than its icons need.
        let icons_width = self.extensions.len() as f32 * (size::ICON + size::EXTENSION_GAP)
            + if self.leading.is_some() {
                size::ICON + size::ICON_TEXT_GAP
            } else {
                0.0
            };
        let min_width =
            size::TEXT_FIELD_MIN_WIDTH.max(2.0 * size::CODE_PADDING + icons_width + size::ICON);
        let width = self
            .width
            .unwrap_or_else(|| ui.available_width())
            .max(min_width);
        // Exactly `width` wide, wherever the layout puts it (a right-to-left row too).
        let column = vec2(width, ui.available_height().max(0.0));
        let layout = egui::Layout::top_down(egui::Align::Min);
        ui.allocate_ui_with_layout(column, layout, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let label_id = self.label.as_ref().map(|label| {
                ui.add(egui::Label::new(egui::RichText::new(label).color(
                    common::enabled_text(self.enabled, colors.text, &colors),
                )))
                .id
            });

            let multiline = self.rows > 1;
            let height = if multiline {
                self.rows as f32 * size::CODE_LINE_HEIGHT + size::CODE_PADDING
            } else {
                size::CONTROL_HEIGHT
            };
            let (rect, frame_response) =
                ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
            // Painted after the content, once focus is known, but drawn beneath it.
            let background = ui.painter().add(egui::Shape::Noop);
            // An open completion list owns the navigation keys, before the edit sees them.
            let suggestions = self
                .suggestions
                .filter(|_| !multiline)
                .map(super::completion::Suggestions);
            let completer: Option<&dyn Completer> = match &suggestions {
                Some(suggestions) => Some(suggestions),
                None => self.completer,
            };
            let field_focused = completer.is_some()
                && self.id.is_some_and(|id| ui.memory(|m| m.has_focus(id)))
                    | ui.data(|d| d.get_temp::<bool>(frame_response.id.with("archipel-focused")))
                        .unwrap_or(false);
            let (mut list, keys) =
                super::completion::List::begin(ui, frame_response.id, field_focused);
            if suggestions.is_some() {
                list.below(rect);
                if field_focused
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown))
                {
                    list.open();
                }
            }
            // The edit's id is known up front, so keys meant for the field can be taken before
            // the edit sees them.
            let edit_id = self
                .id
                .unwrap_or_else(|| frame_response.id.with("archipel-edit"));
            let focused_before = self.enabled && ui.memory(|m| m.has_focus(edit_id));
            let history_id = frame_response.id.with("archipel-history");
            let mut history_chain: PopupChain =
                ui.data(|d| d.get_temp(history_id)).unwrap_or_default();
            let entries: Vec<String> = self
                .history
                .as_deref()
                .map(|history| history.entries().to_vec())
                .unwrap_or_default();
            let now = ui.input(|i| i.time);
            let mut replaced = false;
            if self.clear_on_escape
                && focused_before
                && !history_chain.is_open()
                && !list.is_open()
                && !self.text.is_empty()
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                self.text.clear();
                replaced = true;
            }
            if focused_before && !entries.is_empty() {
                let (down, up) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown),
                        i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp),
                    )
                });
                if down && !history_chain.is_open() {
                    history_chain.open(now, true);
                    ui.ctx().request_repaint();
                } else if down || up {
                    // Alt+Down / Alt+Up step through the entries, wrapping around.
                    let at = entries.iter().position(|e| e == self.text.as_str());
                    let len = entries.len();
                    let next = match (at, down) {
                        (None, true) => 0,
                        (None, false) => len - 1,
                        (Some(i), true) => (i + 1) % len,
                        (Some(i), false) => (i + len - 1) % len,
                    };
                    *self.text = entries[next].clone();
                    replaced = true;
                }
            }
            if history_chain.is_open() {
                let mut source = super::popup_rows::ListRows {
                    rows: entries
                        .iter()
                        .map(|entry| RowSpec {
                            text: entry.clone(),
                            selectable: true,
                            ..RowSpec::default()
                        })
                        .collect(),
                    default: entries
                        .iter()
                        .position(|e| e == self.text.as_str())
                        .unwrap_or(0),
                    speed_search: false,
                    chosen: None,
                };
                let output = drive(
                    ui.ctx(),
                    history_id,
                    Placement::Combo {
                        anchor: rect,
                        max_rows: size::COMBO_MAX_ROWS,
                    },
                    Options {
                        show_submenu_on_hover: false,
                        cancel_on_click_outside: true,
                    },
                    &mut history_chain,
                    &mut source,
                );
                if let Some(entry) = source
                    .chosen
                    .filter(|_| output.chosen.is_some())
                    .and_then(|row| entries.get(row))
                {
                    *self.text = entry.clone();
                    replaced = true;
                    if let Some(history) = self.history.as_deref_mut() {
                        history.add(entry.clone());
                    }
                    ui.memory_mut(|m| m.request_focus(edit_id));
                }
                if output.cancelled {
                    ui.memory_mut(|m| m.request_focus(edit_id));
                }
            }
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink2(vec2(
                        size::CODE_PADDING,
                        if multiline { size::POPUP_PADDING } else { 0.0 },
                    )))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            content.spacing_mut().item_spacing.x = size::ICON_TEXT_GAP;
            let mut leading_rect = None;
            if let Some(icon) = self.leading {
                let (icon_rect, _) =
                    content.allocate_exact_size(vec2(size::ICON, size::ICON), egui::Sense::hover());
                icons::paint(content.painter(), icon_rect, icon, colors.text_placeholder);
                leading_rect = Some(icon_rect);
            }
            // Extensions before the text sit after the leading icon.
            let mut before_rects = Vec::new();
            for (index, extension) in self.extensions.iter().enumerate() {
                if extension.before_text {
                    let (icon_rect, _) = content
                        .allocate_exact_size(vec2(size::ICON, size::ICON), egui::Sense::hover());
                    before_rects.push((index, icon_rect));
                }
            }
            let after_count = self.extensions.iter().filter(|e| !e.before_text).count();
            let show_clear =
                self.clearable && !self.text.is_empty() && self.enabled && suggestions.is_none();
            // The clear button or chevron sits at a fixed distance from the field's right
            // edge; the text stops a gap before it.
            let clear_width = if show_clear {
                size::CLEAR_BUTTON_INSET + size::CLEAR_BUTTON / 2.0 - size::CODE_PADDING
                    + size::ICON_TEXT_GAP
            } else if suggestions.is_some() {
                size::COMBO_CHEVRON_INSET + size::ICON / 2.0 - size::CODE_PADDING
                    + size::ICON_TEXT_GAP
            } else {
                0.0
            } + after_count as f32 * (size::ICON + size::EXTENSION_GAP)
                + if self.spinner {
                    size::SPINNER_ARROW_AREA - size::CODE_PADDING + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
            let mut edit = if multiline {
                egui::TextEdit::multiline(self.text).desired_rows(self.rows)
            } else {
                egui::TextEdit::singleline(self.text)
            }
            .frame(false)
            .password(self.password)
            .interactive(self.enabled)
            .text_color(common::enabled_text(self.enabled, colors.text, &colors))
            .desired_width(content.available_width() - clear_width)
            .margin(vec2(0.0, 0.0));
            edit = edit.id(edit_id);
            let font = if self.monospace {
                edit = edit.font(egui::TextStyle::Monospace);
                egui::TextStyle::Monospace.resolve(content.style())
            } else {
                egui::TextStyle::Body.resolve(content.style())
            };
            let highlighter = self.highlighter.filter(|_| !self.password);
            let enabled = self.enabled;
            let theme = Theme::current(content.ctx());
            // The spans of the last layout, which the underline reuses within the frame.
            let spans = RefCell::new(Vec::new());
            let mut layouter =
                |ui: &Ui, buf: &dyn egui::TextBuffer, wrap: f32| -> Arc<egui::Galley> {
                    let text = buf.as_str();
                    let mut spans = spans.borrow_mut();
                    spans.clear();
                    if let Some(highlighter) = highlighter {
                        highlighter.highlight_text(text, &mut spans);
                    }
                    let job = styled_job(
                        text,
                        0,
                        &spans,
                        if multiline { wrap } else { f32::INFINITY },
                        |kind| {
                            let mut format =
                                token_format(kind, &font, &theme.editor, None, enabled);
                            if kind == TokenKind::Plain {
                                format.color = common::enabled_text(enabled, colors.text, &colors);
                            }
                            format
                        },
                    );
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
                .add_enabled_ui(self.enabled, |ui| {
                    if multiline {
                        crate::parts::scroll_vertical()
                            .id_salt(self.id.unwrap_or(frame_response.id).with("lines"))
                            .max_height(rect.height() - size::CODE_PADDING)
                            .auto_shrink([false, false])
                            .show(ui, |ui| edit.show(ui))
                            .inner
                    } else {
                        edit.show(ui)
                    }
                })
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
            if self.clear_on_escape && edited.has_focus() && !self.text.is_empty() {
                // egui drops the focus on Escape; here it clears the text instead. The edit
                // sets its own filter while it shows, so this one goes after it.
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        edited.id,
                        egui::EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                            ..Default::default()
                        },
                    )
                });
            }
            if let Some(id) = label_id {
                edited = edited.labelled_by(id);
            }
            if let Some(name) = self
                .name
                .as_ref()
                .or(self.label.as_ref())
                .or(self.placeholder.as_ref())
            {
                ui.ctx()
                    .accesskit_node_builder(edited.id, |node| node.set_label(name.as_str()));
            }
            let mut over_button = false;
            let mut extension_clicked = None;
            if replaced {
                edited.mark_changed();
            }
            if show_clear {
                let clear = clear_button(&content, rect, edited.id);
                over_button |= clear.hovered();
                if clear.clicked() {
                    self.text.clear();
                    edited.mark_changed();
                    edited.request_focus();
                }
            }
            // Extensions after the text, right to left from the clear button or the edge.
            let mut right = if show_clear {
                rect.right()
                    - size::CLEAR_BUTTON_INSET
                    - size::CLEAR_BUTTON / 2.0
                    - size::EXTENSION_GAP
            } else if suggestions.is_some() {
                rect.right() - size::COMBO_CHEVRON_INSET - size::ICON / 2.0 - size::EXTENSION_GAP
            } else {
                rect.right() - size::CODE_PADDING
            };
            let mut slots: Vec<(usize, Rect)> = before_rects;
            for (index, extension) in self.extensions.iter().enumerate().rev() {
                if !extension.before_text {
                    slots.push((
                        index,
                        Rect::from_center_size(
                            pos2(right - size::ICON / 2.0, rect.center().y),
                            vec2(size::ICON, size::ICON),
                        ),
                    ));
                    right -= size::ICON + size::EXTENSION_GAP;
                }
            }
            for (index, slot) in slots {
                let response = extension_button(
                    &content,
                    slot,
                    edited.id.with(("extension", index)),
                    &self.extensions[index],
                    self.enabled,
                );
                over_button |= response.hovered();
                if response.clicked() {
                    extension_clicked = Some(index);
                }
            }
            let mut spin = 0;
            if self.spinner {
                let area = Rect::from_min_max(
                    pos2(rect.right() - size::SPINNER_ARROW_AREA, rect.top()),
                    rect.right_bottom(),
                );
                let sense = if self.enabled {
                    egui::Sense::CLICK
                } else {
                    egui::Sense::hover()
                };
                let arrows = content.interact(area, edited.id.with("spin"), sense);
                arrows.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        self.enabled,
                        "Increment or decrement",
                    )
                });
                crate::cursor::on_hover_action(&arrows);
                over_button |= arrows.hovered();
                spin = spinner_steps(&content, &arrows, area);
                spinner_arrows(&content, area, &arrows, self.enabled);
            }
            // The leading icon of a field with a history opens it.
            if leading_rect.is_some() && self.history.is_some() {
                let area = Rect::from_min_max(
                    pos2(rect.left(), rect.top()),
                    pos2(rect.left() + size::SEARCH_ICON_AREA, rect.bottom()),
                );
                let sense = if self.enabled {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                };
                let button = content.interact(area, edited.id.with("history-button"), sense);
                button.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        self.enabled,
                        "Search history",
                    )
                });
                crate::cursor::on_hover_action(&button);
                over_button |= button.hovered();
                if button.clicked() && !entries.is_empty() && history_chain.open(now, true) {
                    ui.ctx().request_repaint();
                }
            }
            ui.data_mut(|d| d.insert_temp(history_id, history_chain));
            if suggestions.is_some() && chevron(&content, rect, edited.id, self.enabled).clicked() {
                if list.is_open() {
                    list.close();
                } else {
                    list.open();
                }
                edited.request_focus();
            }

            let focused = edited.has_focus();
            if highlighter.is_some() {
                // The layouter ran on this text unless egui reused its galley.
                let spans = spans.into_inner();
                underline_unresolved(
                    &content
                        .painter()
                        .with_clip_rect(edited.rect.intersect(rect).intersect(ui.clip_rect())),
                    &spans,
                    self.text,
                    &galley,
                    galley_pos,
                    theme.editor.invalid,
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
                    // Unfocused, a click (on a suggestion) may still be completing it.
                    let live = focused
                        || ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
                    (
                        Some(rect),
                        live.then(|| template_at(self.text, byte)).flatten(),
                    )
                }
                None => (None, None),
            };
            if let Some(completer) = completer {
                ui.data_mut(|d| d.insert_temp(frame_response.id.with("archipel-focused"), focused));
                // A combo box lists below the field whether or not it ever had a caret.
                let caret = match cursor.map(|range| range.primary.index).zip(caret_rect) {
                    None if suggestions.is_some() => Some((self.text.chars().count(), rect)),
                    caret => caret,
                };
                list.end(ui, completer, self.text, &mut edited, caret);
            }
            let submitted = !multiline
                && !keys.accepted()
                && edited.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if submitted && let Some(history) = self.history.as_deref_mut() {
                // A committed search is remembered (IntelliJ `addCurrentTextToHistory`).
                history.add(self.text.clone());
            }
            let problem = if self.warning {
                colors.warning
            } else {
                colors.error
            };
            let stroke = if self.error.is_some() {
                Stroke::new(if focused { size::FOCUS_RING } else { 1.0_f32 }, problem)
            } else if focused {
                Stroke::new(size::FOCUS_RING, colors.focus_ring)
            } else if self.enabled {
                Stroke::new(1.0_f32, colors.border)
            } else {
                Stroke::new(1.0_f32, colors.border_disabled)
            };
            // As in Android Studio, a 2 pt (focus or error) border reaches 1 pt outside.
            let border_rect = if stroke.width > 1.0 {
                rect.expand(1.0)
            } else {
                rect
            };
            let fill = if self.enabled {
                colors.field
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().set(
                background,
                egui::epaint::RectShape::new(
                    border_rect,
                    CornerRadius::same(radius::CONTROL),
                    fill,
                    stroke,
                    egui::StrokeKind::Inside,
                ),
            );
            if self.enabled && frame_response.hovered() && !over_button {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
            if let Some(error) = &self.error {
                // The message may run past a narrow field (a number), as in Android Studio.
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(error)
                            .color(problem)
                            .font(typography::body(typography::SMALL + 1.0)),
                    )
                    .extend(),
                );
                crate::ide::HelpTooltip::new(error.clone()).show_for(&edited);
            }
            JBTextFieldOutput {
                response: edited,
                submitted,
                caret_rect,
                template,
                extension_clicked,
                frame: rect,
                spin,
            }
        })
        .inner
    }
}

/// How many steps the spinner arrows in `area` ask for this frame: one when pressed, then
/// more while held, after [`motion::SPINNER_REPEAT_DELAY`] every
/// [`motion::SPINNER_REPEAT_INTERVAL`] (Swing's spinner timer, IntelliJ spec section 13.4).
/// The upper half of the area steps up, the lower half down; the sign says which.
fn spinner_steps(ui: &Ui, arrows: &Response, area: Rect) -> i32 {
    let key = arrows.id.with("hold");
    let now = ui.input(|i| i.time);
    let pressed = arrows.contains_pointer()
        && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    let direction = |ui: &Ui| {
        let y = ui.input(|i| i.pointer.interact_pos()).map_or(0.0, |p| p.y);
        if y < area.center().y { 1 } else { -1 }
    };
    if pressed {
        let dir = direction(ui);
        ui.data_mut(|d| d.insert_temp(key, (now, now, dir)));
        return dir;
    }
    if !arrows.is_pointer_button_down_on() {
        ui.data_mut(|d| d.remove_temp::<(f64, f64, i32)>(key));
        return 0;
    }
    let Some((start, last, dir)) = ui.data(|d| d.get_temp::<(f64, f64, i32)>(key)) else {
        return 0;
    };
    let delay = f64::from(motion::SPINNER_REPEAT_DELAY);
    let interval = f64::from(motion::SPINNER_REPEAT_INTERVAL);
    let due = if now - start >= delay {
        (last + interval).max(start + delay)
    } else {
        start + delay
    };
    if now >= due {
        ui.data_mut(|d| d.insert_temp(key, (start, now, dir)));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(interval));
        return dir;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(due - now));
    0
}

/// The stacked arrows of a spinner: the half under the pointer is pressed-dark while held.
fn spinner_arrows(ui: &Ui, area: Rect, arrows: &Response, enabled: bool) {
    let colors = Theme::colors(ui.ctx());
    let color = if enabled {
        colors.combo_arrow
    } else {
        colors.text_disabled
    };
    let half = area.height() / 2.0;
    for (glyph, top) in [
        (Glyph::ChevronUp, area.top()),
        (Glyph::ChevronDown, area.top() + half),
    ] {
        let band = Rect::from_min_size(pos2(area.left(), top), vec2(area.width(), half));
        if arrows.is_pointer_button_down_on()
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| band.contains(p))
        {
            ui.painter().circle_filled(
                band.center(),
                size::SPINNER_ARROW_GLYPH / 2.0 + 2.0,
                colors.clear_hover,
            );
        }
        icons::paint(
            ui.painter(),
            Rect::from_center_size(
                band.center(),
                vec2(size::SPINNER_ARROW_GLYPH, size::SPINNER_ARROW_GLYPH),
            ),
            glyph,
            color,
        );
    }
}

/// Draws a dotted line under the unresolved variables.
fn underline_unresolved(
    painter: &egui::Painter,
    spans: &[Span],
    text: &str,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    color: Color32,
) {
    for span in spans.iter().filter(|s| s.kind.dotted_underline()) {
        if span.end > text.len()
            || !text.is_char_boundary(span.start)
            || !text.is_char_boundary(span.end)
        {
            continue;
        }
        let start = text[..span.start].chars().count();
        let end = text[..span.end].chars().count();
        let mut row_start = 0;
        for row in &galley.rows {
            let row_end = row_start + row.char_count_excluding_newline();
            let from = start.max(row_start);
            let to = end.min(row_end);
            if from < to {
                let y = galley_pos.y + row.pos.y + row.height();
                let mut x = galley_pos.x + row.pos.x + row.x_offset(from - row_start);
                let right = galley_pos.x + row.pos.x + row.x_offset(to - row_start);
                while x < right {
                    let top = y - size::DOTTED_INSET - size::DOTTED_DOT;
                    let dot = Rect::from_min_max(
                        pos2(x, top),
                        pos2((x + size::DOTTED_DOT).min(right), top + size::DOTTED_DOT),
                    );
                    painter.rect_filled(dot, 0.0, color);
                    x += size::DOTTED_PITCH;
                }
            }
            row_start += row.char_count_including_newline();
        }
    }
}

/// The clear button of a field: a gray cross that gets a round background under the
/// pointer, at a fixed distance from the field's right edge (measured on Android Studio's
/// search fields).
/// The chevron of a combo box ([`JBTextField::suggestions`]), where a
/// [`ComboBox`](crate::openapi::ui::ComboBox) has its own.
fn chevron(ui: &Ui, field: Rect, field_id: Id, enabled: bool) -> Response {
    let rect = Rect::from_center_size(
        pos2(field.right() - size::COMBO_CHEVRON_INSET, field.center().y),
        vec2(size::ICON, field.height()),
    );
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let response = ui.interact(rect, field_id.with("chevron"), sense);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Show suggestions")
    });
    let colors = Theme::colors(ui.ctx());
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(size::ICON, size::ICON)),
        Glyph::ChevronDown,
        if enabled {
            colors.combo_arrow
        } else {
            colors.text_disabled
        },
    );
    response
}

fn clear_button(ui: &Ui, field: Rect, field_id: Id) -> Response {
    let rect = Rect::from_center_size(
        pos2(field.right() - size::CLEAR_BUTTON_INSET, field.center().y),
        vec2(size::CLEAR_BUTTON, size::CLEAR_BUTTON),
    );
    let response = ui.interact(rect, field_id.with("clear"), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Clear"));
    let colors = Theme::colors(ui.ctx());
    let hovered = response.hovered();
    if hovered {
        ui.painter()
            .circle_filled(rect.center(), size::CLEAR_BUTTON / 2.0, colors.clear_hover);
    }
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(size::CLEAR_GLYPH, size::CLEAR_GLYPH)),
        Glyph::Close,
        if hovered {
            colors.clear_icon_hover
        } else {
            colors.clear_icon
        },
    );
    response
}
