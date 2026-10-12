//! Tooltips, dialogs and menu separators: the things that float above the workbench.

use super::{Button, common};
use crate::icons::{self, Glyph};
use crate::parts::Surface;
use crate::theme::Theme;
use crate::tokens::{popup, size, typography};
use egui::{Align, Key, Layout, Response, Sense, Stroke, Ui, Widget, vec2};

/// A tooltip: a label, an optional shortcut after it, and an optional hint below, on the
/// tooltip color. Attach it with [`TooltipExt::tooltip`] or [`TooltipExt::tooltip_with`];
/// it shows after the hover delay, also over disabled widgets (to say why they are
/// disabled).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{IconButton, Tooltip, TooltipExt};
/// use archipel_kit::Glyph;
///
/// ui.add(IconButton::new(Glyph::Run, "Run"))
///     .tooltip_with(Tooltip::new("Run 'gallery'").shortcut("Shift+F10"));
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct Tooltip {
    label: String,
    shortcut: Option<String>,
    hint: Option<String>,
    align: Option<egui::RectAlign>,
    immediate: bool,
}

impl Tooltip {
    /// A tooltip reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            hint: None,
            align: None,
            immediate: false,
        }
    }

    /// Shows the tooltip as soon as the widget is hovered, without the usual delay (about
    /// half a second, as in Android Studio). Android Studio's tool window stripe buttons
    /// name themselves at once.
    pub fn immediate(mut self, immediate: bool) -> Self {
        self.immediate = immediate;
        self
    }

    /// Where the tooltip goes relative to its widget (default: below it). Tool window
    /// stripes put it beside the button, `egui::RectAlign::RIGHT`, as Android Studio does.
    pub fn align(mut self, align: egui::RectAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// A shortcut shown after the label, e.g. `"Ctrl+Shift+F"`.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// A longer explanation below the label, wrapped.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The tooltip frame: the tooltip color, popup border and radius.
    pub fn frame(ctx: &egui::Context) -> egui::Frame {
        Surface::Tooltip.frame(ctx)
    }

    /// Draws the content (without the frame) into `ui`. Use it to show a tooltip outside
    /// of hover, e.g. in a snapshot or an onboarding hint.
    pub fn content(&self, ui: &mut Ui) {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        ui.spacing_mut().item_spacing = vec2(size::TOOLTIP_SHORTCUT_GAP, 3.0);
        // One text line high, not a row of controls.
        ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
        ui.horizontal(|ui| {
            let label = common::galley(ui, self.label.as_str(), font.clone());
            let (rect, response) = ui.allocate_exact_size(label.size(), Sense::hover());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.label)
            });
            common::text(ui.painter(), rect.min, label, colors.text);
            if let Some(shortcut) = &self.shortcut {
                let shortcut = common::galley(ui, shortcut.as_str(), font.clone());
                let (rect, _) = ui.allocate_exact_size(shortcut.size(), Sense::hover());
                common::text(ui.painter(), rect.min, shortcut, colors.text_secondary);
            }
        });
        if let Some(hint) = &self.hint {
            ui.set_max_width(280.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(hint)
                        .font(typography::body(typography::BODY))
                        .color(colors.text_secondary),
                )
                .wrap(),
            );
        }
    }

    /// Shows the tooltip for `response` when it is hovered long enough.
    pub fn show_for(&self, response: &Response) {
        let mut tooltip = if self.immediate {
            let mut tooltip = egui::Tooltip::for_widget(response);
            let dragging = response.ctx.dragged_id().is_some();
            tooltip.popup = tooltip.popup.open(response.hovered() && !dragging);
            tooltip
        } else if response.enabled() {
            egui::Tooltip::for_enabled(response)
        } else {
            egui::Tooltip::for_disabled(response)
        };
        tooltip.popup = tooltip.popup.frame(Self::frame(&response.ctx));
        if let Some(align) = self.align {
            tooltip.popup = tooltip.popup.align(align).gap(size::POPUP_PADDING);
        }
        tooltip.show(|ui| self.content(ui));
    }
}

/// Kit tooltips on any [`Response`].
pub trait TooltipExt {
    /// A tooltip reading `label`.
    fn tooltip(self, label: impl Into<String>) -> Self;
    /// A tooltip with a shortcut or hint.
    fn tooltip_with(self, tooltip: Tooltip) -> Self;
}

impl TooltipExt for Response {
    fn tooltip(self, label: impl Into<String>) -> Self {
        self.tooltip_with(Tooltip::new(label))
    }

    fn tooltip_with(self, tooltip: Tooltip) -> Self {
        tooltip.show_for(&self);
        self
    }
}

/// A thin line between groups of menu items.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{MenuItem, MenuSeparator};
///
/// ui.menu_button("File", |ui| {
///     ui.add(MenuItem::new("Open…"));
///     ui.add(MenuSeparator);
///     ui.add(MenuItem::new("Exit"));
/// });
/// # });
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct MenuSeparator;

impl Widget for MenuSeparator {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let width = ui.available_width().max(1.0);
        super::misc::menu_rows(ui);
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, popup::SEPARATOR_HEIGHT), Sense::hover());
        // As in Android Studio: inset like the item pills, in the popup's border color.
        ui.painter().hline(
            (rect.left() + popup::ITEM_INSET)..=(rect.right() - popup::ITEM_INSET),
            rect.top() + (popup::SEPARATOR_HEIGHT / 2.0).floor() + 0.5,
            Stroke::new(1.0_f32, colors.popup_border),
        );
        response
    }
}

/// A section title inside a menu or popup, like "Open Projects" in Android Studio's project
/// picker: dimmed semibold text aligned with the items' icons. It is not focusable.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{MenuHeader, MenuItem, MenuSeparator};
///
/// ui.add(MenuItem::new("Open…"));
/// ui.add(MenuSeparator);
/// ui.add(MenuHeader::new("Open Projects"));
/// ui.add(MenuItem::new("archipel").description("~/code/archipel"));
/// # });
/// ```
#[must_use = "add it with `ui.add(header)`"]
pub struct MenuHeader {
    title: String,
}

impl MenuHeader {
    /// A header reading `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl Widget for MenuHeader {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(
            ui,
            self.title.as_str(),
            typography::semibold(ui.ctx(), typography::BODY),
        );
        let x = popup::ITEM_INSET + popup::ITEM_PADDING;
        let width = ui.available_width().max(galley.size().x + 2.0 * x);
        super::misc::menu_rows(ui);
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, popup::HEADER_HEIGHT), Sense::hover());
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.title));
        common::text_left(
            ui.painter(),
            rect,
            rect.left() + x,
            galley,
            colors.text_placeholder,
        );
        response
    }
}

/// Custom dialog content.
type Content<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// What the user did with a [`Dialog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DialogAction {
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
/// [`Dialog::show`] reports an action. <kbd>Enter</kbd> picks the primary button
/// (inside a multiline field it is a new line: <kbd>Ctrl</kbd>+<kbd>Enter</kbd>, or
/// <kbd>Cmd</kbd>+<kbd>Enter</kbd> on macOS, picks it), <kbd>Escape</kbd> dismisses. The
/// dialog is as tall as its content, up to [`Dialog::max_height`], then scrolls.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Dialog, DialogAction};
///
/// let mut open = true;
/// if open {
///     let action = Dialog::new("Delete “main.rs”?")
///         .message("The file will be moved to the trash.")
///         .primary("Delete")
///         .danger(true)
///         .secondary("Cancel")
///         .show(ui.ctx());
///     if action == Some(DialogAction::Primary) { /* delete */ }
///     if action.is_some() { open = false; }
/// }
/// # });
/// ```
#[must_use = "show it with `dialog.show(ctx)`"]
pub struct Dialog<'a> {
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

impl<'a> Dialog<'a> {
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
    /// [`Dialog::show`] reports [`DialogAction::Tertiary`] and the dialog stays yours to
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
    pub fn show(self, ctx: &egui::Context) -> Option<DialogAction> {
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
                        action = Some(DialogAction::Tertiary);
                    }
                    ui.add_space((ui.available_width() - group).max(0.0));
                    if let Some(secondary) = secondary
                        && ui.add(secondary).clicked()
                    {
                        action = Some(DialogAction::Secondary);
                    }
                    if let Some(primary) = primary
                        && ui.add(primary).clicked()
                    {
                        action = Some(DialogAction::Primary);
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
                action = Some(DialogAction::Primary);
            } else if modal.should_close() {
                action = Some(DialogAction::Dismissed);
            }
        }
        action
    }
}
