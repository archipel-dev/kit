//! Single-line text inputs.

use super::{IconButton, common};
use crate::icons::{self, Glyph, Icon};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{Color32, CornerRadius, Response, Stroke, Ui, Widget, vec2};

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

    /// Total width in points (default: the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Widget for TextField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
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
            if self.monospace {
                edit = edit.font(egui::TextStyle::Monospace);
            }
            if let Some(placeholder) = &self.placeholder {
                edit =
                    edit.hint_text(egui::RichText::new(placeholder).color(colors.text_placeholder));
            }
            // Disabled through the ui, so the field reports disabled to screen readers and
            // tests, not just ignores input.
            let mut edited = content.add_enabled(self.enabled, edit);
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
            edited
        })
        .inner
    }
}
