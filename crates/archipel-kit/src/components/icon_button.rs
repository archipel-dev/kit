//! Square icon-only buttons for toolbars and headers.

use super::common;
use crate::icons::{self, Icon};
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, size, toolbar};
use egui::{Color32, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, vec2};

/// A transparent, square button showing one icon; hover and press tint it.
///
/// The label is required: it is the tooltip and the accessible name, since there is no
/// visible text.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, components::IconButton};
///
/// if ui.add(IconButton::new(Glyph::Refresh, "Reload")).clicked() { /* ... */ }
/// let mut pinned = false;
/// if ui.add(IconButton::new(Glyph::Pin, "Pin tab").selected(pinned)).clicked() {
///     pinned = !pinned;
/// }
/// # });
/// ```
#[must_use = "add it with `ui.add(button)`"]
pub struct IconButton {
    icon: Icon,
    label: String,
    selected: Option<bool>,
    enabled: bool,
    tone: Option<Tone>,
    small: bool,
    large: bool,
}

impl IconButton {
    /// A button showing `icon`, described by `label` (tooltip and accessible name).
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            selected: None,
            enabled: true,
            tone: None,
            small: false,
            large: false,
        }
    }

    /// Makes it a toggle: selected toggles are filled with the accent color.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Disabled buttons are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A 20-point button for actions inside rows and tabs, which are 24 points high.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// A 30-point button with a 20-point icon, for the main window toolbar (as in Android
    /// Studio).
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Meaning of the icon's color (default: the icon gray), e.g. [`Tone::Run`] or
    /// [`Tone::Stop`] for run and stop buttons.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }
}

impl Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (edge, icon_edge) = if self.large {
            (toolbar::BUTTON, toolbar::ICON)
        } else if self.small {
            (size::ICON_BUTTON_SMALL, size::ICON)
        } else {
            (size::ICON_BUTTON, size::ICON)
        };
        let (rect, response) = ui.allocate_exact_size(vec2(edge, edge), sense);
        let selected = self.selected.unwrap_or(false);
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| match self.selected {
            Some(selected) => {
                WidgetInfo::selected(WidgetType::Button, self.enabled, selected, &self.label)
            }
            None => WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label),
        });
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let fill = if selected {
                colors.accent
            } else if self.enabled {
                common::flat_fill(&response, &colors)
            } else {
                Color32::TRANSPARENT
            };
            // Toolbar buttons are rounder than the 22 pt ones inside islands.
            let corner = if self.large {
                radius::ICON_BUTTON
            } else {
                radius::ICON_BUTTON_SMALL
            };
            painter.rect_filled(rect, corner, fill);
            if response.has_focus() {
                common::focus_ring(painter, rect, corner, &colors);
            }
            let color = if !self.enabled {
                colors.text_disabled
            } else if selected {
                colors.text_on_accent
            } else {
                self.tone.unwrap_or(Tone::Icon).color(&colors)
            };
            let icon_rect = Rect::from_center_size(rect.center(), vec2(icon_edge, icon_edge));
            icons::paint(painter, icon_rect, self.icon, color);
        }
        super::Tooltip::new(self.label).show_for(&response);
        response
    }
}
