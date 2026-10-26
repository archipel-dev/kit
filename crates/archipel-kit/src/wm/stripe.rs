//! Tool window stripes: the icon bars beside the islands that open and close tool windows.

use crate::common;
use crate::icons::{self, Icon};
use crate::ide::HelpTooltip;
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, stripe};
use egui::{Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

/// One button of a [`ToolWindowToolbar`]: a tool window's icon, name and shortcut.
#[derive(Debug, Clone)]
#[must_use = "add it with `ui.add(button)` or give it to a `ToolWindowToolbar`"]
pub struct SquareStripeButton {
    icon: Icon,
    label: String,
    shortcut: Option<String>,
    open: bool,
    active: bool,
    id_salt: Option<egui::Id>,
}

impl SquareStripeButton {
    /// The button of the tool window `label`, shown as `icon`.
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            shortcut: None,
            open: false,
            active: false,
            id_salt: None,
        }
    }

    /// Whether the tool window is the active one (it has the keyboard focus): its button is
    /// filled with the accent color, as in Android Studio. Implies open.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    fn id_salt(mut self, id: egui::Id) -> Self {
        self.id_salt = Some(id);
        self
    }

    /// The shortcut that toggles the tool window, shown in the tooltip (e.g. `"Alt+1"`).
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Whether the tool window is open: its button is filled.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// A vertical bar of tool window buttons on the window frame, as on the left and right edge
/// of Android Studio: a group at the top and one at the bottom. A button is filled while its
/// tool window is open; clicking it reports its index, so the caller can open or close it.
/// Tooltips (name and shortcut) appear beside the buttons.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, wm::{SquareStripeButton, ToolWindowToolbar}};
///
/// let mut project = true;
/// let mut terminal = false;
/// let clicked = ToolWindowToolbar::new("left")
///     .top(SquareStripeButton::new(Glyph::Folder, "Project").shortcut("Alt+1").open(project))
///     .bottom(SquareStripeButton::new(Glyph::Terminal, "Terminal").shortcut("Alt+F12").open(terminal))
///     .show(ui);
/// match clicked {
///     Some(0) => project = !project,
///     Some(1) => terminal = !terminal,
///     _ => {}
/// }
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct ToolWindowToolbar {
    id: egui::Id,
    top: Vec<SquareStripeButton>,
    bottom: Vec<SquareStripeButton>,
}

impl ToolWindowToolbar {
    /// An empty stripe; `id_salt` tells stripes apart.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: egui::Id::new(id_salt),
            top: Vec::new(),
            bottom: Vec::new(),
        }
    }

    /// Adds a button to the top group. Buttons are numbered top group first, then bottom.
    pub fn top(mut self, button: SquareStripeButton) -> Self {
        self.top.push(button);
        self
    }

    /// Adds a button to the bottom group.
    pub fn bottom(mut self, button: SquareStripeButton) -> Self {
        self.bottom.push(button);
        self
    }

    /// Draws the stripe in the full height of `ui`, [`stripe::WIDTH`] wide. Returns the
    /// index of the button clicked this frame (top group first, then bottom group).
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let (rect, _) = ui.allocate_exact_size(
            vec2(stripe::WIDTH, ui.available_height().max(stripe::PITCH)),
            Sense::hover(),
        );
        let x = rect.center().x;
        let mut clicked = None;
        let count_top = self.top.len();
        let bottom = self.bottom.len();
        let placed = self
            .top
            .into_iter()
            .enumerate()
            .map(|(index, button)| {
                let y = rect.top() + stripe::PADDING + index as f32 * stripe::PITCH;
                (index, y, button)
            })
            .chain(self.bottom.into_iter().enumerate().map(|(index, button)| {
                // Bottom-aligned: the last button sits as far from the bottom as the first
                // from the top.
                let from_bottom = (bottom - index) as f32 * stripe::PITCH
                    - (stripe::PITCH - stripe::BUTTON)
                    + stripe::PADDING;
                (count_top + index, rect.bottom() - from_bottom, button)
            }));
        for (index, y, button) in placed {
            let place = Rect::from_center_size(
                pos2(x, y + stripe::BUTTON / 2.0),
                vec2(stripe::BUTTON, stripe::BUTTON),
            );
            if ui.put(place, button.id_salt(self.id.with(index))).clicked() {
                clicked = Some(index);
            }
        }
        clicked
    }
}

/// A stripe button on its own, [`stripe::BUTTON`] square, for a stripe laid out by the
/// caller. [`ToolWindowToolbar`] places them for you.
impl Widget for SquareStripeButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (rect, response) = match self.id_salt {
            Some(id) => {
                let rect = ui.available_rect_before_wrap();
                let rect = Rect::from_min_size(rect.min, vec2(stripe::BUTTON, stripe::BUTTON));
                (rect, ui.interact(rect, id, Sense::click()))
            }
            None => ui.allocate_exact_size(vec2(stripe::BUTTON, stripe::BUTTON), Sense::click()),
        };
        let enabled = response.enabled();
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                enabled,
                self.open || self.active,
                &self.label,
            )
        });
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            // Open: filled like a pressed button; otherwise the usual hover and press.
            let fill = if self.active && enabled {
                colors.accent
            } else if self.open || self.active {
                colors.pressed
            } else if enabled {
                common::flat_fill(&response, &colors)
            } else {
                egui::Color32::TRANSPARENT
            };
            painter.rect_filled(rect, radius::ICON_BUTTON, fill);
            if response.has_focus() {
                common::focus_ring(painter, rect, radius::ICON_BUTTON, &colors);
            }
            let icon = Rect::from_center_size(rect.center(), vec2(stripe::ICON, stripe::ICON));
            let color = if self.active && enabled {
                colors.text_on_accent
            } else if enabled {
                Tone::Icon.color(&colors)
            } else {
                colors.text_disabled
            };
            icons::paint(painter, icon, self.icon, color);
        }
        // Stripe buttons name themselves at once, beside the button, as in Android Studio.
        let mut tooltip = HelpTooltip::new(self.label.clone())
            .align(egui::RectAlign::RIGHT)
            .immediate(true);
        if let Some(shortcut) = &self.shortcut {
            tooltip = tooltip.shortcut(shortcut.clone());
        }
        tooltip.show_for(&response);
        response
    }
}
