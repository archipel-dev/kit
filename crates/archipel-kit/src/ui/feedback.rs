//! Inline banners and their status.

use crate::action_system::ActionButton;
use crate::icons::{self, Glyph};
use crate::theme::{Colors, Theme, Tint};
use crate::tokens::{radius, size};
use egui::{CornerRadius, Stroke, Ui};

/// Status of a [`InlineBanner`] or [`Notification`]: picks its tint and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Neutral information.
    Info,
    /// Something finished successfully.
    Success,
    /// Something needs attention but nothing failed.
    Warning,
    /// Something failed.
    Error,
}

impl Status {
    pub(crate) fn tint(self, colors: &Colors) -> Tint {
        match self {
            Status::Info => colors.banner_info,
            Status::Success => colors.banner_success,
            Status::Warning => colors.banner_warning,
            Status::Error => colors.banner_error,
        }
    }

    pub(crate) fn icon(self) -> Glyph {
        match self {
            Status::Info => Glyph::Info,
            Status::Success => Glyph::Success,
            Status::Warning => Glyph::Warning,
            Status::Error => Glyph::Error,
        }
    }

    pub(crate) fn icon_color(self, colors: &Colors) -> egui::Color32 {
        match self {
            Status::Info => colors.link,
            Status::Success => colors.success,
            Status::Warning => colors.warning,
            Status::Error => colors.error,
        }
    }
}

/// What the user did with a [`InlineBanner`] or [`Notification`] this frame.
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
/// use archipel_kit::ui::{InlineBanner, Status};
///
/// let out = InlineBanner::new(Status::Warning, "Project JDK is not defined")
///     .action("Set up JDK")
///     .dismissible(true)
///     .show(ui);
/// if out.action == Some(0) { /* open the SDK dialog */ }
/// # });
/// ```
pub struct InlineBanner {
    severity: Status,
    text: String,
    actions: Vec<String>,
    dismissible: bool,
}

impl InlineBanner {
    /// A banner of `severity` saying `text`.
    pub fn new(severity: Status, text: impl Into<String>) -> Self {
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
        let line = ui.text_style_height(&egui::TextStyle::Body).max(size::ICON);
        let vertical = ((size::BANNER_HEIGHT - 2.0 - line) / 2.0).max(0.0) as i8;
        egui::Frame::new()
            .fill(tint.background)
            .stroke(Stroke::new(1.0_f32, tint.border))
            .corner_radius(CornerRadius::same(radius::BANNER))
            // Measured on Android Studio's editor notifications: 40 pt high, 16 pt before
            // the icon, 10 pt to the text, links 22 pt apart, 16 pt at the end.
            .inner_margin(egui::Margin {
                left: size::BANNER_PADDING as i8,
                right: size::BANNER_PADDING_END as i8,
                top: vertical,
                bottom: vertical,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // One text line high: links and the icon do not stretch the row.
                ui.spacing_mut().interact_size.y = line;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = size::BANNER_ICON_GAP;
                    icons::show(
                        ui,
                        self.severity.icon(),
                        size::ICON,
                        self.severity.icon_color(&colors),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = size::BANNER_ACTION_GAP;
                        if self.dismissible
                            && ui
                                .add(ActionButton::new(Glyph::Close, "Dismiss").small(true))
                                .clicked()
                        {
                            out.dismissed = true;
                        }
                        // Actions are links, as in Android Studio's editor notifications.
                        for (index, label) in self.actions.iter().enumerate().rev() {
                            if ui.add(crate::ui::ActionLink::new(label.as_str())).clicked() {
                                out.action = Some(index);
                            }
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = size::BANNER_ICON_GAP;
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
