//! Tool window stripes: the icon bars beside the islands that open and close tool windows.

use super::{Tooltip, common};
use crate::icons::{self, Icon};
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, stripe};
use egui::{Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// One button of a [`ToolStripe`]: a tool window's icon, name and shortcut.
#[derive(Debug, Clone)]
pub struct StripeButton {
    icon: Icon,
    label: String,
    shortcut: Option<String>,
    open: bool,
}

impl StripeButton {
    /// The button of the tool window `label`, shown as `icon`.
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            shortcut: None,
            open: false,
        }
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
/// use archipel_kit::{Glyph, components::{StripeButton, ToolStripe}};
///
/// let mut project = true;
/// let mut terminal = false;
/// let clicked = ToolStripe::new("left")
///     .top(StripeButton::new(Glyph::Folder, "Project").shortcut("Alt+1").open(project))
///     .bottom(StripeButton::new(Glyph::Terminal, "Terminal").shortcut("Alt+F12").open(terminal))
///     .show(ui);
/// match clicked {
///     Some(0) => project = !project,
///     Some(1) => terminal = !terminal,
///     _ => {}
/// }
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct ToolStripe {
    id: egui::Id,
    top: Vec<StripeButton>,
    bottom: Vec<StripeButton>,
}

impl ToolStripe {
    /// An empty stripe; `id_salt` tells stripes apart.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: egui::Id::new(id_salt),
            top: Vec::new(),
            bottom: Vec::new(),
        }
    }

    /// Adds a button to the top group. Buttons are numbered top group first, then bottom.
    pub fn top(mut self, button: StripeButton) -> Self {
        self.top.push(button);
        self
    }

    /// Adds a button to the bottom group.
    pub fn bottom(mut self, button: StripeButton) -> Self {
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
        for (index, button) in self.top.iter().enumerate() {
            let y = rect.top() + stripe::PADDING + index as f32 * stripe::PITCH;
            if self
                .button(ui, button, pos2(x, y + stripe::BUTTON / 2.0))
                .clicked()
            {
                clicked = Some(index);
            }
        }
        let bottom = self.bottom.len();
        for (index, button) in self.bottom.iter().enumerate() {
            // Bottom-aligned: the last button sits as far from the bottom as the first from
            // the top.
            let from_bottom = (bottom - index) as f32 * stripe::PITCH
                - (stripe::PITCH - stripe::BUTTON)
                + stripe::PADDING;
            let y = rect.bottom() - from_bottom;
            if self
                .button(ui, button, pos2(x, y + stripe::BUTTON / 2.0))
                .clicked()
            {
                clicked = Some(count_top + index);
            }
        }
        clicked
    }

    fn button(&self, ui: &mut Ui, button: &StripeButton, center: egui::Pos2) -> Response {
        let colors = Theme::colors(ui.ctx());
        let rect = Rect::from_center_size(center, vec2(stripe::BUTTON, stripe::BUTTON));
        let response = ui.interact(rect, self.id.with(&button.label), Sense::click());
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Button, true, button.open, &button.label)
        });
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            // Open: filled like a pressed button; otherwise the usual hover and press.
            let fill = if button.open {
                colors.pressed
            } else {
                common::flat_fill(&response, &colors)
            };
            painter.rect_filled(rect, radius::ICON_BUTTON, fill);
            if response.has_focus() {
                common::focus_ring(painter, rect, radius::ICON_BUTTON, &colors);
            }
            let icon = Rect::from_center_size(center, vec2(stripe::ICON, stripe::ICON));
            icons::paint(painter, icon, button.icon, Tone::Icon.color(&colors));
        }
        // Stripe buttons name themselves at once, beside the button, as in Android Studio.
        let mut tooltip = Tooltip::new(button.label.clone())
            .align(egui::RectAlign::RIGHT)
            .immediate(true);
        if let Some(shortcut) = &button.shortcut {
            tooltip = tooltip.shortcut(shortcut.clone());
        }
        tooltip.show_for(&response);
        response
    }
}
