//! Notification cards.

use crate::action_system::ActionButton;
use crate::icons::{self, Glyph};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use crate::ui::{MessageResponse, Status};
use egui::{CornerRadius, Stroke, Ui, vec2};

/// A notification card: icon, title, optional body and actions. Draw it yourself inside
/// a panel, or let [`Notification::show_corner`] float it in the bottom-right corner.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{ui::Status, notification::Notification};
///
/// Notification::new(Status::Success, "Build finished")
///     .body("3 modules compiled in 4.2 s")
///     .action("Open log")
///     .show(ui);
/// # });
/// ```
pub struct Notification {
    severity: Status,
    title: String,
    body: Option<String>,
    actions: Vec<String>,
}

impl Notification {
    /// A toast of `severity` titled `title`.
    pub fn new(severity: Status, title: impl Into<String>) -> Self {
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
                                    if ui.add(crate::ui::ActionLink::new(label.as_str())).clicked()
                                    {
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
            .add(ActionButton::new(Glyph::Close, "Dismiss").small(true))
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
