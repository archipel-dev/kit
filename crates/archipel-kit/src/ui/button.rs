//! Push buttons.

use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::size;
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
/// use archipel_kit::ui::Button;
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
    mnemonic: Option<char>,
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
            min_width: size::BUTTON_MIN_WIDTH,
            mnemonic: None,
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

    /// The mnemonic: <kbd>Alt</kbd>+`letter` clicks the button while it is enabled, and the
    /// first matching letter of the label is underlined (IntelliJ spec section 13.2).
    pub fn mnemonic(mut self, letter: char) -> Self {
        self.mnemonic = Some(letter);
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
        let metrics = crate::tokens::metrics(ui.ctx());
        let height = if self.small {
            metrics.control_height_small
        } else {
            metrics.control_height
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
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), sense);
        // Space activates on release, Enter on press; Alt+letter is the mnemonic.
        super::activation::key_activation(ui, &mut response, self.enabled, true);
        if self.enabled
            && let Some(key) = self
                .mnemonic
                .and_then(|letter| egui::Key::from_name(&letter.to_uppercase().to_string()))
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, key))
        {
            response
                .flags
                .set(egui::response::Flags::FAKE_PRIMARY_CLICKED, true);
        }
        crate::cursor::on_hover_action(&response);
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, galley.text()));
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let pressed = response.is_pointer_button_down_on();
        let hovered = response.hovered() && self.enabled;
        let (fill, stroke, text) = match (self.variant, self.enabled) {
            // Buttons do not react to hover (measured on Android Studio, IntelliJ spec section
            // 13.2); only focus shows. A press darkens them, as in Swing's button model.
            (ButtonVariant::Primary, true) => (
                darken(colors.accent, pressed),
                Stroke::NONE,
                colors.text_on_accent,
            ),
            (ButtonVariant::Danger, true) => (
                darken(colors.stop, pressed),
                Stroke::NONE,
                colors.text_on_accent,
            ),
            (ButtonVariant::Primary | ButtonVariant::Danger, false) => (
                colors.accent_disabled,
                Stroke::NONE,
                colors.text_on_accent_disabled,
            ),
            (ButtonVariant::Secondary, enabled) => (
                if pressed && enabled {
                    colors.pressed
                } else {
                    Color32::TRANSPARENT
                },
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
        let control_radius = metrics.control_radius;
        let corner = CornerRadius::same(control_radius);
        let painter = ui.painter();
        let focused = response.has_focus();
        let filled = matches!(self.variant, ButtonVariant::Primary | ButtonVariant::Danger);
        // Classic fills buttons with a top-to-bottom gradient (`startBackground` to
        // `endBackground`); New UI and Islands are flat.
        let gradient = match (self.variant, self.enabled) {
            _ if !metrics.button_gradient => None,
            (ButtonVariant::Primary, true) => Some((
                darken(colors.accent, pressed),
                darken(colors.accent_end, pressed),
            )),
            (ButtonVariant::Secondary, true) if !pressed => {
                Some((colors.button_start, colors.button_end))
            }
            _ => None,
        };
        if let Some((top, bottom)) = gradient {
            let inner = if focused && filled {
                rect.shrink(1.0)
            } else {
                rect
            };
            crate::parts::fill_gradient(painter, inner, corner, top, bottom);
        }
        let fill = if gradient.is_some() {
            Color32::TRANSPARENT
        } else {
            fill
        };
        if focused && filled {
            // A filled button keeps a 1 pt gap inside its focus ring, as in Android Studio.
            painter.rect(rect.shrink(1.0), corner, fill, stroke, StrokeKind::Inside);
            painter.rect_stroke(
                rect.expand(2.0),
                CornerRadius::same(control_radius + 2),
                Stroke::new(size::FOCUS_RING, colors.focus_ring),
                StrokeKind::Inside,
            );
        } else {
            painter.rect(rect, corner, fill, stroke, StrokeKind::Inside);
            if focused {
                crate::parts::focus_ring(painter, rect, control_radius, &colors);
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
        let top = rect.center().y - galley.size().y / 2.0;
        if let Some(letter) = self.mnemonic
            && let Some(index) = galley
                .text()
                .chars()
                .position(|c| c.to_lowercase().eq(letter.to_lowercase()))
        {
            let from = galley.pos_from_cursor(egui::text::CCursor::new(index));
            let to = galley.pos_from_cursor(egui::text::CCursor::new(index + 1));
            let y = top + galley.size().y - 1.0;
            painter.hline(x + from.min.x..=x + to.min.x, y, Stroke::new(1.0_f32, text));
        }
        painter.galley_with_override_text_color(egui::pos2(x, top), galley, text);
        response
    }
}

/// `color` darkened for a pressed button (Swing's button model darkens on press).
fn darken(color: Color32, pressed: bool) -> Color32 {
    if pressed {
        color.lerp_to_gamma(Color32::BLACK, size::BUTTON_PRESSED_DARKEN)
    } else {
        color
    }
}
