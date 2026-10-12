//! Push buttons.

use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{radius, size};
use egui::{
    Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetText,
    WidgetType, vec2,
};

/// Visual weight of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Filled with the accent color: the one default action of a dialog or form.
    Primary,
    /// Outlined: every other action.
    Secondary,
    /// No outline until hovered: actions inside toolbars, rows and banners.
    Ghost,
    /// Filled with the error color: destructive actions such as Delete.
    Danger,
}

/// A push button with a text label and an optional leading icon.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Button;
///
/// if ui.add(Button::primary("Create")).clicked() { /* ... */ }
/// ui.add(Button::secondary("Cancel"));
/// ui.add(Button::secondary("Delete").enabled(false));
/// # });
/// ```
#[must_use = "add it with `ui.add(button)`"]
pub struct Button {
    label: WidgetText,
    variant: ButtonVariant,
    icon: Option<Icon>,
    enabled: bool,
    small: bool,
    min_width: f32,
}

impl Button {
    /// A button of `variant`.
    pub fn new(label: impl Into<WidgetText>, variant: ButtonVariant) -> Self {
        Self {
            label: label.into(),
            variant,
            icon: None,
            enabled: true,
            small: false,
            min_width: 72.0,
        }
    }

    /// The default action: filled with the accent color.
    pub fn primary(label: impl Into<WidgetText>) -> Self {
        Self::new(label, ButtonVariant::Primary)
    }

    /// An outlined button.
    pub fn secondary(label: impl Into<WidgetText>) -> Self {
        Self::new(label, ButtonVariant::Secondary)
    }

    /// A borderless button.
    pub fn ghost(label: impl Into<WidgetText>) -> Self {
        Self::new(label, ButtonVariant::Ghost)
    }

    /// A destructive action.
    pub fn danger(label: impl Into<WidgetText>) -> Self {
        Self::new(label, ButtonVariant::Danger)
    }

    /// Draws `icon` before the label.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Disabled buttons are dimmed and ignore clicks. Default: enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Compact height for dense toolbars and banners.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// Minimum width in points (default 72, so short labels like "OK" do not look cramped).
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }
}

impl Button {
    /// The width the button will have in `ui`, for laying out neighbours before drawing.
    pub(crate) fn width(&self, ui: &Ui) -> f32 {
        let padding = if self.small {
            size::BUTTON_PADDING - 4.0
        } else {
            size::BUTTON_PADDING
        };
        let galley = self.label.clone().into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        let icon_width = if self.icon.is_some() {
            size::ICON + size::ICON_TEXT_GAP
        } else {
            0.0
        };
        (galley.size().x + icon_width + 2.0 * padding).max(self.min_width)
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let height = if self.small {
            size::CONTROL_HEIGHT_SMALL
        } else {
            size::CONTROL_HEIGHT
        };
        let galley = self.label.clone().into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        let icon_width = if self.icon.is_some() {
            size::ICON + size::ICON_TEXT_GAP
        } else {
            0.0
        };
        let width = self.width(ui);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
        crate::cursor::on_hover_action(&response);
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, galley.text()));
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let pressed = response.is_pointer_button_down_on();
        let hovered = response.hovered() && self.enabled;
        let (fill, stroke, text) = match (self.variant, self.enabled) {
            // Android Studio's buttons do not react to hover or press: only focus shows.
            (ButtonVariant::Primary, true) => (colors.accent, Stroke::NONE, colors.text_on_accent),
            (ButtonVariant::Danger, true) => (colors.stop, Stroke::NONE, colors.text_on_accent),
            (ButtonVariant::Primary | ButtonVariant::Danger, false) => (
                colors.accent_disabled,
                Stroke::NONE,
                colors.text_on_accent_disabled,
            ),
            (ButtonVariant::Secondary, enabled) => (
                Color32::TRANSPARENT,
                Stroke::new(
                    1.0_f32,
                    if enabled {
                        colors.border
                    } else {
                        colors.border_disabled
                    },
                ),
                if enabled {
                    colors.text
                } else {
                    colors.text_disabled
                },
            ),
            (ButtonVariant::Ghost, enabled) => (
                if pressed {
                    colors.pressed
                } else if hovered {
                    colors.hover
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::NONE,
                if enabled {
                    colors.text
                } else {
                    colors.text_disabled
                },
            ),
        };
        let corner = CornerRadius::same(radius::CONTROL);
        let painter = ui.painter();
        let focused = response.has_focus();
        let filled = matches!(self.variant, ButtonVariant::Primary | ButtonVariant::Danger);
        if focused && filled {
            // A filled button keeps a 1 pt gap inside its focus ring, as in Android Studio.
            painter.rect(rect.shrink(1.0), corner, fill, stroke, StrokeKind::Inside);
            painter.rect_stroke(
                rect.expand(2.0),
                CornerRadius::same(radius::CONTROL + 2),
                Stroke::new(size::FOCUS_RING, colors.focus_ring),
                StrokeKind::Inside,
            );
        } else {
            painter.rect(rect, corner, fill, stroke, StrokeKind::Inside);
            if focused {
                crate::parts::focus_ring(painter, rect, radius::CONTROL, &colors);
            }
        }
        let content = vec2(icon_width + galley.size().x, galley.size().y);
        let mut x = rect.center().x - content.x / 2.0;
        if let Some(icon) = self.icon {
            let icon_rect =
                egui::Rect::from_min_size(egui::pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0));
            icons::paint(painter, icon_rect, icon, text);
            x += icon_width;
        }
        painter.galley_with_override_text_color(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            text,
        );
        response
    }
}
