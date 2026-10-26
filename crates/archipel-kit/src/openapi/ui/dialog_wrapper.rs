//! The modal dialog.

use crate::common;
use crate::icons::{self, Glyph};
use crate::parts::Surface;
use crate::theme::Theme;
use crate::tokens::{size, typography};
use crate::ui::Button;
use egui::{Align, Key, Layout, Sense, Ui, vec2};

/// Custom dialog content.
type Content<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// What the user did with a [`DialogWrapper`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExitCode {
    /// The primary button, or <kbd>Enter</kbd>.
    Primary,
    /// The secondary button.
    Secondary,
    /// The tertiary button (left-aligned, e.g. "Reset").
    Tertiary,
    /// <kbd>Escape</kbd>, the close button or a click outside.
    Dismissed,
}

/// A modal dialog: a title, a message or custom content, and buttons at the bottom right.
/// The rest of the window is dimmed and blocked while it is shown.
///
/// Stateless: show it while your `open` flag is set and clear the flag when
/// [`DialogWrapper::show`] reports an action. <kbd>Enter</kbd> picks the primary button
/// (inside a multiline field it is a new line: <kbd>Ctrl</kbd>+<kbd>Enter</kbd>, or
/// <kbd>Cmd</kbd>+<kbd>Enter</kbd> on macOS, picks it), <kbd>Escape</kbd> dismisses. The
/// dialog is as tall as its content, up to [`DialogWrapper::max_height`], then scrolls.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::{DialogWrapper, ExitCode};
///
/// let mut open = true;
/// if open {
///     let action = DialogWrapper::new("Delete “main.rs”?")
///         .message("The file will be moved to the trash.")
///         .primary("Delete")
///         .danger(true)
///         .secondary("Cancel")
///         .show(ui.ctx());
///     if action == Some(ExitCode::Primary) { /* delete */ }
///     if action.is_some() { open = false; }
/// }
/// # });
/// ```
#[must_use = "show it with `dialog.show(ctx)`"]
pub struct DialogWrapper<'a> {
    title: String,
    message: Option<String>,
    icon: Option<Glyph>,
    primary: Option<String>,
    secondary: Option<String>,
    tertiary: Option<String>,
    danger: bool,
    primary_enabled: bool,
    width: f32,
    max_height: Option<f32>,
    content: Option<Content<'a>>,
}

impl<'a> DialogWrapper<'a> {
    /// A dialog titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: None,
            icon: None,
            primary: None,
            secondary: None,
            tertiary: None,
            danger: false,
            primary_enabled: true,
            width: 400.0,
            max_height: None,
            content: None,
        }
    }

    /// A message below the title, wrapped.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// An icon before the title (e.g. [`Glyph::Warning`]).
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Custom content below the message (fields, lists).
    pub fn content(mut self, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// The primary button (rightmost; <kbd>Enter</kbd>).
    pub fn primary(mut self, label: impl Into<String>) -> Self {
        self.primary = Some(label.into());
        self
    }

    /// The secondary button (left of the primary).
    pub fn secondary(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }

    /// A tertiary action, a secondary-style button at the left edge (e.g. "Reset").
    /// [`DialogWrapper::show`] reports [`ExitCode::Tertiary`] and the dialog stays yours to
    /// close or keep open.
    pub fn tertiary(mut self, label: impl Into<String>) -> Self {
        self.tertiary = Some(label.into());
        self
    }

    /// Makes the primary button destructive (red).
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// Disables the primary button (and <kbd>Enter</kbd>), e.g. until a form is valid.
    pub fn primary_enabled(mut self, enabled: bool) -> Self {
        self.primary_enabled = enabled;
        self
    }

    /// Width in points (default 400).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Tallest the dialog may grow, in points (default 80% of the window). The dialog is
    /// as tall as its content; beyond this the message and content scroll, the title and
    /// the buttons stay.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Shows the dialog this frame and reports what the user did, if anything.
    pub fn show(self, ctx: &egui::Context) -> Option<ExitCode> {
        let colors = Theme::colors(ctx);
        let id = egui::Id::new(("archipel-dialog", &self.title));
        let frame = Surface::Dialog.frame(ctx);
        let frame_height = frame.total_margin().sum().y;
        let mut action = None;
        let primary_enabled = self.primary_enabled;
        let max_height = self
            .max_height
            .unwrap_or_else(|| ctx.content_rect().height() * 0.8);
        // Focus before the dialog runs: a field that still has it afterwards kept Enter.
        let focus_before = ctx.memory(|m| m.focused());
        let modal = egui::Modal::new(id)
            .frame(frame)
            .backdrop_color(
                egui::Color32::BLACK.gamma_multiply(if Theme::current(ctx).dark {
                    crate::tokens::opacity::SCRIM_DARK
                } else {
                    crate::tokens::opacity::SCRIM_LIGHT
                }),
            )
            .show(ctx, |ui| {
                ui.set_width(self.width);
                ui.spacing_mut().item_spacing.y = size::SECTION_SPACING;
                ui.horizontal(|ui| {
                    if let Some(icon) = self.icon {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(size::ICON, size::ICON), Sense::hover());
                        icons::paint(ui.painter(), rect, icon, colors.icon);
                    }
                    let title = common::galley(
                        ui,
                        self.title.as_str(),
                        typography::semibold(ui.ctx(), typography::TITLE + 1.0),
                    );
                    let (rect, response) = ui.allocate_exact_size(title.size(), Sense::hover());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.title)
                    });
                    common::text(ui.painter(), rect.min, title, colors.text_strong);
                });
                // Title and buttons stay; what lies between scrolls past the max height.
                // What doesn't scroll: the frame, the title as drawn (measured), the button
                // row, and the four gaps between the title, body, space and buttons.
                let title_height = ui.min_rect().height();
                // A dialog without buttons (its body has its own) has no button row.
                let has_buttons =
                    self.primary.is_some() || self.secondary.is_some() || self.tertiary.is_some();
                let button_row = if has_buttons {
                    size::CONTROL_HEIGHT + 2.0 * size::SECTION_SPACING
                } else {
                    0.0
                };
                let chrome = frame_height + title_height + button_row + 2.0 * size::SECTION_SPACING;
                let body_max = (max_height - chrome).max(size::ROW_HEIGHT);
                // A solid scroll bar: it shows that there is more without hovering.
                ui.scope(|ui| {
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    ui.visuals_mut().extreme_bg_color = colors.border;
                    ui.visuals_mut().widgets.inactive.bg_fill = colors.text_placeholder;
                    ui.visuals_mut().widgets.hovered.bg_fill = colors.text_secondary;
                    ui.visuals_mut().widgets.active.bg_fill = colors.text_secondary;
                    let body = crate::parts::scroll_vertical()
                        .id_salt("archipel-dialog-body")
                        .max_height(body_max)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = size::SECTION_SPACING;
                            if let Some(message) = &self.message {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(message)
                                            .font(typography::body(typography::BODY))
                                            .color(colors.text),
                                    )
                                    .wrap(),
                                );
                            }
                            if let Some(content) = self.content {
                                content(ui);
                            }
                        });
                    // The modal sizes itself from the last frame: when the body grew or
                    // shrank (a page switched, a banner appeared), draw once more so the
                    // dialog fits without waiting for the next input.
                    let key = id.with("body-height");
                    let height = body.content_size.y;
                    let before = ui.data(|d| d.get_temp::<f32>(key));
                    if before.is_some_and(|h| (h - height).abs() > 0.5) {
                        ui.ctx().request_repaint();
                    }
                    ui.data_mut(|d| d.insert_temp(key, height));
                });
                if !has_buttons {
                    return;
                }
                ui.add_space(size::SECTION_SPACING);
                // A fixed-height row: `with_layout` would take all the height left over.
                // Drawn left to right so Tab follows reading order: tertiary, secondary,
                // primary. The secondary and primary buttons sit at the right edge: their
                // widths are measured first, so the first frame is already right.
                let secondary = self.secondary.as_deref().map(Button::secondary);
                let primary = self.primary.as_deref().map(|label| {
                    let button = if self.danger {
                        Button::danger(label)
                    } else {
                        Button::primary(label)
                    };
                    button.enabled(primary_enabled)
                });
                let gap = ui.spacing().item_spacing.x;
                let mut group = 0.0;
                for button in [&secondary, &primary].into_iter().flatten() {
                    group += button.width(ui);
                }
                if secondary.is_some() && primary.is_some() {
                    group += gap;
                }
                let row = vec2(ui.available_width(), size::CONTROL_HEIGHT);
                ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                    if let Some(tertiary) = &self.tertiary
                        && ui.add(Button::secondary(tertiary.as_str())).clicked()
                    {
                        action = Some(ExitCode::Tertiary);
                    }
                    ui.add_space((ui.available_width() - group).max(0.0));
                    if let Some(secondary) = secondary
                        && ui.add(secondary).clicked()
                    {
                        action = Some(ExitCode::Secondary);
                    }
                    if let Some(primary) = primary
                        && ui.add(primary).clicked()
                    {
                        action = Some(ExitCode::Primary);
                    }
                });
            });
        if action.is_none() {
            // A multiline field keeps the focus when it takes Enter (a single-line field
            // gives it up), and Enter there is a new line: Ctrl/Cmd+Enter confirms.
            // egui has no "this field is multiline" query, so this relies on two of its
            // behaviours: a `TextEdit` reports IME output while it has focus, and a
            // single-line one surrenders focus on Enter. Anything else that keeps the focus
            // (no IME output) is not a text field: Enter still confirms. Tests pin this.
            let in_multiline = focus_before.is_some()
                && ctx.memory(|m| m.focused()) == focus_before
                && ctx.output(|o| o.ime.is_some());
            let enter = ctx.input(|i| {
                i.events.iter().any(|event| {
                    matches!(event, egui::Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        modifiers,
                        ..
                    } if !in_multiline || modifiers.command || modifiers.ctrl)
                })
            });
            if enter && primary_enabled && self.primary.is_some() {
                action = Some(ExitCode::Primary);
            } else if modal.should_close() {
                action = Some(ExitCode::Dismissed);
            }
        }
        action
    }
}
