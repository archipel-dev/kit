//! The editor notification panel.

use super::{MessageResponse, Status};
use crate::action_system::ActionButton;
use crate::icons::{self, Glyph};
use crate::theme::Theme;
use crate::tokens::size;
use egui::{Stroke, Ui};

/// The strip above a file's editor: an optional status icon, a line of text, link actions
/// and a close button, across the full width with a line below it. It is IntelliJ's
/// `EditorNotificationPanel`; [`InlineBanner`](super::InlineBanner) is the rounded variant
/// for dialogs and tool windows.
///
/// Without a [`status`](EditorNotificationPanel::status) the panel is the editor's own
/// color and has no icon; with one it takes the tint and icon of that [`Status`]
/// (IntelliJ spec section 10.2).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{EditorNotificationPanel, Status};
///
/// let out = EditorNotificationPanel::new("File is read-only")
///     .status(Status::Warning)
///     .action("Make writable")
///     .dismissible(true)
///     .show(ui);
/// if out.action == Some(0) { /* make the file writable */ }
/// # });
/// ```
#[must_use = "show it with `panel.show(ui)`"]
pub struct EditorNotificationPanel {
    status: Option<Status>,
    text: String,
    actions: Vec<String>,
    dismissible: bool,
}

impl EditorNotificationPanel {
    /// A panel saying `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            status: None,
            text: text.into(),
            actions: Vec::new(),
            dismissible: false,
        }
    }

    /// Tints the panel and gives it the icon of `status`.
    pub fn status(mut self, status: Status) -> Self {
        self.status = Some(status);
        self
    }

    /// Adds a link action; [`MessageResponse::action`] reports its index.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.actions.push(label.into());
        self
    }

    /// Shows the close button ([`MessageResponse::dismissed`]).
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Draws the panel across the available width.
    pub fn show(self, ui: &mut Ui) -> MessageResponse {
        let colors = Theme::colors(ui.ctx());
        let (fill, line) = match self.status {
            Some(status) => {
                let tint = status.tint(&colors);
                (tint.background, tint.border)
            }
            None => (colors.island, colors.separator),
        };
        let mut out = MessageResponse::default();
        let text_line = ui.text_style_height(&egui::TextStyle::Body).max(size::ICON);
        let frame = egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin::symmetric(
                size::EDITOR_NOTIFICATION_PADDING_X as i8,
                size::EDITOR_NOTIFICATION_PADDING_Y as i8,
            ))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().interact_size.y = text_line;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = size::EDITOR_NOTIFICATION_ICON_GAP;
                    if let Some(status) = self.status {
                        icons::show(ui, status.icon(), size::ICON, status.icon_color(&colors));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = size::BANNER_ACTION_GAP;
                        if self.dismissible {
                            let close =
                                ui.add(ActionButton::new(Glyph::Close, "Dismiss").small(true));
                            out.dismissed |= close.clicked();
                            out.focused |= close.has_focus();
                        }
                        for (index, label) in self.actions.iter().enumerate().rev() {
                            let link = ui.add(crate::ui::ActionLink::new(label.as_str()));
                            if link.clicked() {
                                out.action = Some(index);
                            }
                            out.focused |= link.has_focus();
                        }
                        ui.spacing_mut().item_spacing.x = size::EDITOR_NOTIFICATION_TEXT_END;
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
        let rect = frame.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0_f32, line),
        );
        out
    }
}
