//! Tool window stripes: the icon bars beside the islands that open and close tool windows.

use crate::common;
use crate::icons::{self, Icon};
use crate::ide::HelpTooltip;
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, stripe};
use egui::{
    Id, Order, Pos2, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

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
    /// index of the button clicked this frame (top group first, then bottom group). A close
    /// click (middle button, or Shift and the left button) is not a click: see
    /// [`show_with_events`](Self::show_with_events).
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        self.show_with_events(ui).clicked
    }

    /// Like [`show`](Self::show), and reports everything the user did with the stripe's
    /// buttons: the click that toggles, the close click that hides, the popup trigger and a
    /// drag that was dropped (see [`StripeResponse`]).
    pub fn show_with_events(self, ui: &mut Ui) -> StripeResponse {
        let colors = Theme::colors(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(
            vec2(stripe::WIDTH, ui.available_height().max(stripe::PITCH)),
            Sense::hover(),
        );
        let x = rect.center().x;
        let mut out = StripeResponse {
            rect,
            clicked: None,
            close_clicked: None,
            popup: None,
            dropped: None,
        };
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
        let modifiers = crate::wm::action_rows::pointer_event_modifiers(ui);
        let dragging_any: bool = ui.ctx().data(|d| d.get_temp(drag_flag())).unwrap_or(false);
        for (index, y, button) in placed {
            let place = Rect::from_center_size(
                pos2(x, y + stripe::BUTTON / 2.0),
                vec2(stripe::BUTTON, stripe::BUTTON),
            );
            let icon = button.icon;
            let response = ui.put(place, button.id_salt(self.id.with(index)));
            let pointer = ui.input(|i| i.pointer.latest_pos());
            // Only the left button toggles; the close click (middle, or Shift and left)
            // hides; the popup trigger asks for the button's menu.
            if response.clicked() {
                if modifiers.shift {
                    out.close_clicked = Some(index);
                } else {
                    out.clicked = Some(index);
                }
            }
            if response.middle_clicked() {
                out.close_clicked = Some(index);
            }
            if response.secondary_clicked() {
                out.popup = Some((index, pointer.unwrap_or(place.center())));
            }
            // A drag starts 7 px from the press; a ghost of the button follows the pointer;
            // Esc cancels; a release drops it where the pointer is.
            let cancelled_id = self.id.with(("drag-cancelled", index));
            if response.dragged() {
                let origin = ui.input(|i| i.pointer.press_origin());
                let beyond = matches!((origin, pointer), (Some(a), Some(b)) if a.distance(b) > stripe::DRAG_DEADZONE);
                let cancelled: bool = ui.data(|d| d.get_temp(cancelled_id)).unwrap_or(false);
                if beyond && !cancelled {
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        ui.data_mut(|d| d.insert_temp(cancelled_id, true));
                        ui.ctx().data_mut(|d| d.insert_temp(drag_flag(), false));
                    } else {
                        ui.ctx().data_mut(|d| d.insert_temp(drag_flag(), true));
                        ui.data_mut(|d| d.insert_temp(self.id.with(("dragging", index)), true));
                        if let Some(pointer) = pointer {
                            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                                Order::Tooltip,
                                Id::new("archipel-stripe-ghost"),
                            ));
                            let ghost = Rect::from_center_size(
                                pointer,
                                vec2(stripe::BUTTON, stripe::BUTTON),
                            );
                            painter.rect_filled(
                                ghost,
                                radius::ICON_BUTTON,
                                colors.accent.gamma_multiply(0.5),
                            );
                            icons::paint(
                                &painter,
                                Rect::from_center_size(pointer, vec2(stripe::ICON, stripe::ICON)),
                                icon,
                                colors.text_on_accent,
                            );
                        }
                    }
                }
            }
            if response.drag_stopped() {
                let was_dragging: bool = ui
                    .data(|d| d.get_temp(self.id.with(("dragging", index))))
                    .unwrap_or(false);
                // An Esc in the frame of the release cancels too.
                let cancelled = ui.data(|d| d.get_temp(cancelled_id)).unwrap_or(false)
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
                if was_dragging && !cancelled {
                    out.dropped = Some((index, pointer.unwrap_or(place.center())));
                }
                ui.data_mut(|d| {
                    d.remove::<bool>(self.id.with(("dragging", index)));
                    d.remove::<bool>(cancelled_id);
                });
                ui.ctx().data_mut(|d| d.insert_temp(drag_flag(), false));
            }
        }
        // While a button is dragged, the stripe under the pointer is the drop target.
        if dragging_any && ui.rect_contains_pointer(rect) {
            ui.painter().rect_filled(
                rect.shrink2(vec2(2.0, 0.0)),
                radius::ICON_BUTTON,
                colors.accent.gamma_multiply(0.18),
            );
        }
        out
    }
}

/// The id of the context-wide flag that says a stripe button is being dragged.
fn drag_flag() -> Id {
    Id::new("archipel-stripe-drag")
}

/// What the user did with a [`ToolWindowToolbar`]'s buttons this frame. Indices count the
/// top group first, then the bottom group.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StripeResponse {
    /// The stripe's area.
    pub rect: Rect,
    /// A button clicked with the left button: toggles its tool window.
    pub clicked: Option<usize>,
    /// A close click (middle button, or Shift and the left button): hides its tool window.
    pub close_clicked: Option<usize>,
    /// A popup trigger (right click) on a button, and where.
    pub popup: Option<(usize, Pos2)>,
    /// A button dragged more than 7 px and dropped, and where the pointer was.
    pub dropped: Option<(usize, Pos2)>,
}

/// A stripe button on its own, [`stripe::BUTTON`] square, for a stripe laid out by the
/// caller. [`ToolWindowToolbar`] places them for you.
impl Widget for SquareStripeButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        // Click, and drag to another stripe group (reported by the `ToolWindowToolbar`).
        let (rect, response) = match self.id_salt {
            Some(id) => {
                let rect = ui.available_rect_before_wrap();
                let rect = Rect::from_min_size(rect.min, vec2(stripe::BUTTON, stripe::BUTTON));
                (rect, ui.interact(rect, id, Sense::click_and_drag()))
            }
            None => ui.allocate_exact_size(
                vec2(stripe::BUTTON, stripe::BUTTON),
                Sense::click_and_drag(),
            ),
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
