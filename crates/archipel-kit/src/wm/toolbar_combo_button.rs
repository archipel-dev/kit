//! Main toolbar buttons: text widgets such as Android Studio's project and VCS widgets.

use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::{Slot, State, Tone};
use crate::theme::Theme;
use crate::tokens::{size, toolbar, typography};
use egui::{Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// A flat button with text for the main window toolbar, like Android Studio's project and
/// version control widgets: an optional leading icon (or a slot, e.g. an [`crate::ui::Avatar`]),
/// the text, and a chevron when it opens a menu. Flat at rest; hover and press fill it.
/// Open the menu with a [`crate::ui::popup::JBPopup`] anchored to the response.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{ui::Avatar, wm::ToolbarComboButton};
/// use archipel_kit::parts::Slot;
///
/// let project = ToolbarComboButton::new("archipel")
///     .leading(Slot::ui_width(20.0, |ui| { ui.add(Avatar::new("archipel").size(20.0)); }))
///     .dropdown(true)
///     .show(ui);
/// let vcs = ToolbarComboButton::new("main").icon(archipel_kit::Glyph::Branch).dropdown(true).show(ui);
/// # let _ = (project, vcs);
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct ToolbarComboButton<'a> {
    label: String,
    icon: Option<Icon>,
    leading: Option<Slot<'a>>,
    dropdown: bool,
    open: bool,
    enabled: bool,
}

impl<'a> ToolbarComboButton<'a> {
    /// A button reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            leading: None,
            dropdown: false,
            open: false,
            enabled: true,
        }
    }

    /// A 20-point icon before the text.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Caller content before the text (20 points high), e.g. a project's avatar.
    pub fn leading(mut self, slot: Slot<'a>) -> Self {
        self.leading = Some(slot);
        self
    }

    /// Shows a chevron after the text: the button opens a menu.
    pub fn dropdown(mut self, dropdown: bool) -> Self {
        self.dropdown = dropdown;
        self
    }

    /// Whether its menu is open: the button stays filled.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Disabled buttons are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the button.
    pub fn show(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        let natural = common::galley(ui, self.label.as_str(), font.clone());
        let leading_width = match (&self.leading, self.icon) {
            (Some(slot), _) => Some(slot.width(ui)),
            (None, Some(_)) => Some(toolbar::ICON),
            (None, None) => None,
        };
        let start = match leading_width {
            Some(width) => toolbar::ICON_PADDING + width + toolbar::ICON_GAP,
            None => toolbar::PADDING,
        };
        let end = if self.dropdown {
            toolbar::CHEVRON_GAP + size::ICON + toolbar::ICON_PADDING
        } else {
            toolbar::PADDING
        };
        // Drop-down buttons grow to at most 350 pt (`MainToolbar.Dropdown.maxWidth`); longer
        // text is elided.
        let room = (toolbar::DROPDOWN_MAX_WIDTH - start - end).max(0.0);
        let galley = if natural.size().x > room {
            common::galley_elided(ui, self.label.as_str(), font, room)
        } else {
            natural
        };
        let width = start + galley.size().x + end;
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let metrics = crate::tokens::metrics(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(vec2(width, metrics.toolbar_button), sense);
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| {
            let kind = if self.dropdown {
                WidgetType::ComboBox
            } else {
                WidgetType::Button
            };
            WidgetInfo::labeled(kind, self.enabled, &self.label)
        });
        let state = State::of(&response);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let fill = if self.open && self.enabled {
                colors.pressed
            } else if self.enabled {
                common::flat_fill(&response, &colors)
            } else {
                egui::Color32::TRANSPARENT
            };
            painter.rect_filled(rect, metrics.icon_button_radius, fill);
            if response.has_focus() {
                common::focus_ring(painter, rect, metrics.icon_button_radius, &colors);
            }
            if let Some(width) = leading_width {
                let lead = Rect::from_min_size(
                    pos2(rect.left() + toolbar::ICON_PADDING, rect.top()),
                    vec2(width, rect.height()),
                );
                if let Some(icon) = self.icon.filter(|_| self.leading.is_none()) {
                    let icon_rect =
                        Rect::from_center_size(lead.center(), vec2(toolbar::ICON, toolbar::ICON));
                    icons::paint(painter, icon_rect, icon, state.icon(Tone::Icon, &colors));
                }
            }
            let text_color = if self.enabled {
                colors.text
            } else {
                colors.text_disabled
            };
            common::text_left(painter, rect, rect.left() + start, galley, text_color);
            if self.dropdown {
                let chevron = Rect::from_min_size(
                    pos2(
                        rect.right() - toolbar::ICON_PADDING - size::ICON,
                        rect.center().y - size::ICON / 2.0,
                    ),
                    vec2(size::ICON, size::ICON),
                );
                icons::paint(
                    painter,
                    chevron,
                    Glyph::ChevronDown,
                    state.icon(Tone::Icon, &colors),
                );
            }
        }
        if let (Some(slot), Some(width)) = (self.leading, leading_width) {
            let lead = Rect::from_min_size(
                pos2(rect.left() + toolbar::ICON_PADDING, rect.top()),
                vec2(width, rect.height()),
            );
            slot.show(ui, lead, state);
        }
        response
    }
}
