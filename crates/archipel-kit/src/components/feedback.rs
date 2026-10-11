//! Messages: inline banners and toast notifications.

use super::{Button, IconButton};
use crate::icons::{self, Glyph};
use crate::theme::{Colors, Theme, Tint};
use crate::tokens::{radius, size, typography};
use egui::{CornerRadius, Stroke, Ui, vec2};

/// Severity of a [`Banner`] or [`Toast`]: picks its tint and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Neutral information.
    Info,
    /// Something finished successfully.
    Success,
    /// Something needs attention but nothing failed.
    Warning,
    /// Something failed.
    Error,
}

impl Severity {
    fn tint(self, colors: &Colors) -> Tint {
        match self {
            Severity::Info => colors.banner_info,
            Severity::Success => colors.banner_success,
            Severity::Warning => colors.banner_warning,
            Severity::Error => colors.banner_error,
        }
    }

    fn icon(self) -> Glyph {
        match self {
            Severity::Info => Glyph::Info,
            Severity::Success => Glyph::Success,
            Severity::Warning => Glyph::Warning,
            Severity::Error => Glyph::Error,
        }
    }

    fn icon_color(self, colors: &Colors) -> egui::Color32 {
        match self {
            Severity::Info => colors.link,
            Severity::Success => colors.success,
            Severity::Warning => colors.warning,
            Severity::Error => colors.error,
        }
    }
}

/// What the user did with a [`Banner`] or [`Toast`] this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageResponse {
    /// Index of the clicked action, in the order they were added.
    pub action: Option<usize>,
    /// The close button was clicked.
    pub dismissed: bool,
}

/// A full-width message above the content it is about: a missing SDK, an outdated index,
/// a failed sync. Actions are buttons on the right; the close button is optional.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Banner, Severity};
///
/// let out = Banner::new(Severity::Warning, "Project JDK is not defined")
///     .action("Set up JDK")
///     .dismissible(true)
///     .show(ui);
/// if out.action == Some(0) { /* open the SDK dialog */ }
/// # });
/// ```
pub struct Banner {
    severity: Severity,
    text: String,
    actions: Vec<String>,
    dismissible: bool,
}

impl Banner {
    /// A banner of `severity` saying `text`.
    pub fn new(severity: Severity, text: impl Into<String>) -> Self {
        Self {
            severity,
            text: text.into(),
            actions: Vec::new(),
            dismissible: false,
        }
    }

    /// Adds an action button; [`MessageResponse::action`] reports its index when clicked.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.actions.push(label.into());
        self
    }

    /// Shows a close button.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Draws the banner across the available width.
    pub fn show(self, ui: &mut Ui) -> MessageResponse {
        let colors = Theme::colors(ui.ctx());
        let tint = self.severity.tint(&colors);
        let mut out = MessageResponse::default();
        egui::Frame::new()
            .fill(tint.background)
            .stroke(Stroke::new(1.0_f32, tint.border))
            .corner_radius(CornerRadius::same(radius::BANNER))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    icons::show(
                        ui,
                        self.severity.icon(),
                        size::ICON,
                        self.severity.icon_color(&colors),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.dismissible
                            && ui.add(IconButton::new(Glyph::Close, "Dismiss")).clicked()
                        {
                            out.dismissed = true;
                        }
                        for (index, label) in self.actions.iter().enumerate().rev() {
                            if ui
                                .add(Button::secondary(label.as_str()).small(true).min_width(0.0))
                                .clicked()
                            {
                                out.action = Some(index);
                            }
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&self.text).color(colors.text),
                                )
                                .wrap(),
                            );
                        });
                    });
                });
            });
        out
    }
}

/// A notification card: icon, title, optional body and actions. Draw it yourself inside
/// a panel, or let [`Toast::show_corner`] float it in the bottom-right corner.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Severity, Toast};
///
/// Toast::new(Severity::Success, "Build finished")
///     .body("3 modules compiled in 4.2 s")
///     .action("Open log")
///     .show(ui);
/// # });
/// ```
pub struct Toast {
    severity: Severity,
    title: String,
    body: Option<String>,
    actions: Vec<String>,
}

impl Toast {
    /// A toast of `severity` titled `title`.
    pub fn new(severity: Severity, title: impl Into<String>) -> Self {
        Self {
            severity,
            title: title.into(),
            body: None,
            actions: Vec::new(),
        }
    }

    /// Detail text below the title.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Adds a link-style action; [`MessageResponse::action`] reports its index.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.actions.push(label.into());
        self
    }

    /// Draws the card at the current position (width 320).
    pub fn show(self, ui: &mut Ui) -> MessageResponse {
        let colors = Theme::colors(ui.ctx());
        let mut out = MessageResponse::default();
        let padding = size::NOTIFICATION_PADDING;
        let frame = egui::Frame::new()
            .fill(colors.notification)
            .stroke(Stroke::new(1.0_f32, colors.popup_border))
            .corner_radius(CornerRadius::same(radius::POPUP))
            .inner_margin(egui::Margin::same(padding as i8))
            .shadow(ui.visuals().popup_shadow)
            .show(ui, |ui| {
                ui.set_width(size::NOTIFICATION_WIDTH - 2.0 * padding);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    // The icon sits on the title line.
                    let line = ui.text_style_height(&egui::TextStyle::Body);
                    ui.allocate_ui_with_layout(
                        vec2(size::ICON, line),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            icons::show(
                                ui,
                                self.severity.icon(),
                                size::ICON,
                                self.severity.icon_color(&colors),
                            )
                        },
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        // Leave room for the dismiss button over the title's end.
                        ui.set_max_width(ui.available_width() - size::ICON_BUTTON_SMALL);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&self.title)
                                    .font(typography::semibold(ui.ctx(), typography::BODY))
                                    .color(colors.text_strong),
                            )
                            .wrap(),
                        );
                        if let Some(body) = &self.body {
                            ui.add(
                                egui::Label::new(egui::RichText::new(body).color(colors.text))
                                    .wrap(),
                            );
                        }
                        if !self.actions.is_empty() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;
                                for (index, label) in self.actions.iter().enumerate() {
                                    if ui.add(super::Link::new(label.as_str())).clicked() {
                                        out.action = Some(index);
                                    }
                                }
                            });
                        }
                    });
                });
            });
        // As in Android Studio's notifications, the dismiss button shows in the top-right
        // corner while the pointer is over the card (or the button has focus); it keeps its
        // name and Tab stop while hidden.
        let card = frame.response.rect;
        let corner = egui::Rect::from_min_size(
            egui::pos2(
                card.right() - padding / 2.0 - size::ICON_BUTTON_SMALL,
                card.top() + padding / 2.0,
            ),
            vec2(size::ICON_BUTTON_SMALL, size::ICON_BUTTON_SMALL),
        );
        let mut close = ui.new_child(egui::UiBuilder::new().max_rect(corner));
        let close_id = close.next_auto_id();
        let focused = ui.memory(|m| m.has_focus(close_id));
        if !(ui.rect_contains_pointer(card) || focused) {
            close.multiply_opacity(0.0);
        }
        if close
            .add(IconButton::new(Glyph::Close, "Dismiss").small(true))
            .clicked()
        {
            out.dismissed = true;
        }
        out
    }

    /// Floats the card in the bottom-right corner of the window, above other content.
    /// `id` must be unique among the visible toasts; `index` stacks several upwards.
    pub fn show_corner(
        self,
        ctx: &egui::Context,
        id: impl std::hash::Hash,
        index: usize,
    ) -> MessageResponse {
        let offset = -(16.0 + index as f32 * 96.0);
        egui::Area::new(egui::Id::new(("archipel_toast", id)))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_BOTTOM, vec2(-16.0, offset))
            .show(ctx, |ui| self.show(ui))
            .inner
    }
}
