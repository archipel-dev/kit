//! Tooltips, dialogs and menu separators: the things that float above the workbench.

use super::{Button, common};
use crate::icons::{self, Glyph};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{Align, CornerRadius, Key, Layout, Response, Sense, Stroke, Ui, Widget, vec2};

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
}

impl Tooltip {
    /// A tooltip reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            hint: None,
        }
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
        let colors = Theme::colors(ctx);
        egui::Frame::popup(&ctx.style())
            .fill(colors.tooltip)
            .stroke(Stroke::new(1.0_f32, colors.popup_border))
            .corner_radius(CornerRadius::same(radius::CONTROL))
            .inner_margin(egui::Margin::symmetric(8, 5))
    }

    /// Draws the content (without the frame) into `ui`. Use it to show a tooltip outside
    /// of hover, e.g. in a snapshot or an onboarding hint.
    pub fn content(&self, ui: &mut Ui) {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        ui.spacing_mut().item_spacing = vec2(size::SECONDARY_GAP, 3.0);
        ui.horizontal(|ui| {
            let label = common::galley(ui, self.label.as_str(), font.clone());
            let (rect, response) = ui.allocate_exact_size(label.size(), Sense::hover());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.label)
            });
            common::text(ui.painter(), rect.min, label, colors.text_strong);
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
        let mut tooltip = if response.enabled() {
            egui::Tooltip::for_enabled(response)
        } else {
            egui::Tooltip::for_disabled(response)
        };
        tooltip.popup = tooltip.popup.frame(Self::frame(&response.ctx));
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
        let (rect, response) = ui.allocate_exact_size(vec2(width, 9.0), Sense::hover());
        let inset = size::ROW_PILL_INSET + 4.0;
        ui.painter().hline(
            (rect.left() + inset)..=(rect.right() - inset),
            rect.center().y,
            Stroke::new(1.0_f32, colors.separator),
        );
        response
    }
}

/// Custom dialog content.
type Content<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// What the user did with a [`Dialog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    /// The primary button, or <kbd>Enter</kbd>.
    Primary,
    /// The secondary button.
    Secondary,
    /// <kbd>Escape</kbd>, the close button or a click outside.
    Dismissed,
}

/// A modal dialog: a title, a message or custom content, and buttons at the bottom right.
/// The rest of the window is dimmed and blocked while it is shown.
///
/// Stateless: show it while your `open` flag is set and clear the flag when
/// [`Dialog::show`] reports an action. <kbd>Enter</kbd> picks the primary button,
/// <kbd>Escape</kbd> dismisses.
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
    danger: bool,
    primary_enabled: bool,
    width: f32,
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
            danger: false,
            primary_enabled: true,
            width: 400.0,
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

    /// Shows the dialog this frame and reports what the user did, if anything.
    pub fn show(self, ctx: &egui::Context) -> Option<DialogAction> {
        let colors = Theme::colors(ctx);
        let id = egui::Id::new(("archipel-dialog", &self.title));
        let frame = egui::Frame::popup(&ctx.style())
            .fill(colors.popup)
            .stroke(Stroke::new(1.0_f32, colors.popup_border))
            .corner_radius(CornerRadius::same(radius::ISLAND))
            .inner_margin(egui::Margin::same(size::DIALOG_PADDING as i8 + 4));
        let mut action = None;
        let primary_enabled = self.primary_enabled;
        let modal = egui::Modal::new(id)
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(
                if Theme::current(ctx).dark { 120 } else { 60 },
            ))
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
                ui.add_space(size::SECTION_SPACING);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(primary) = &self.primary {
                        let button = if self.danger {
                            Button::danger(primary.as_str())
                        } else {
                            Button::primary(primary.as_str())
                        };
                        if ui.add(button.enabled(primary_enabled)).clicked() {
                            action = Some(DialogAction::Primary);
                        }
                    }
                    if let Some(secondary) = &self.secondary
                        && ui.add(Button::secondary(secondary.as_str())).clicked()
                    {
                        action = Some(DialogAction::Secondary);
                    }
                });
            });
        if action.is_none() {
            let enter = ctx.input(|i| i.key_pressed(Key::Enter));
            if enter && primary_enabled && self.primary.is_some() {
                action = Some(DialogAction::Primary);
            } else if modal.should_close() {
                action = Some(DialogAction::Dismissed);
            }
        }
        action
    }
}
