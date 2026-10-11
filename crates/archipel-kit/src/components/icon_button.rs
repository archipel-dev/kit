//! Square icon-only buttons for toolbars and headers.

use super::common;
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{radius, size};
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
    color: Option<Color32>,
}

impl IconButton {
    /// A button showing `icon`, described by `label` (tooltip and accessible name).
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            selected: None,
            enabled: true,
            color: None,
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

    /// Icon color (default: the theme's icon color). Use it for semantic icons such as a
    /// green Run or red Stop.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = Some(color.into());
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
        let (rect, response) =
            ui.allocate_exact_size(vec2(size::ICON_BUTTON, size::ICON_BUTTON), sense);
        let selected = self.selected.unwrap_or(false);
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
            painter.rect_filled(rect, radius::ICON_BUTTON, fill);
            if response.has_focus() {
                common::focus_ring(painter, rect, radius::ICON_BUTTON, &colors);
            }
            let color = if !self.enabled {
                colors.text_disabled
            } else if selected {
                colors.text_on_accent
            } else {
                self.color.unwrap_or(colors.icon)
            };
            let icon_rect = Rect::from_center_size(rect.center(), vec2(size::ICON, size::ICON));
            icons::paint(painter, icon_rect, self.icon, color);
        }
        response.on_hover_text(self.label)
    }
}
