//! Tool window stripes: the icon bars beside the islands that open and close tool windows.

use crate::common;
use crate::icons::{self, Icon};
use crate::ide::HelpTooltip;
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::stripe;
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
        let metrics = crate::tokens::metrics(ui.ctx());
        let (pitch, tile) = (metrics.stripe_pitch, metrics.stripe_button);
        let (rect, _) = ui.allocate_exact_size(
            vec2(metrics.stripe_width, ui.available_height().max(pitch)),
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
                let y = rect.top() + stripe::PADDING + index as f32 * pitch;
                (index, y, button)
            })
            .chain(self.bottom.into_iter().enumerate().map(|(index, button)| {
                // Bottom-aligned: the last button sits as far from the bottom as the first
                // from the top.
                let from_bottom =
                    (bottom - index) as f32 * pitch - (pitch - tile) + stripe::PADDING;
                (count_top + index, rect.bottom() - from_bottom, button)
            }));
        let modifiers = crate::wm::action_rows::pointer_event_modifiers(ui);
        let dragging_any: bool = ui.ctx().data(|d| d.get_temp(drag_flag())).unwrap_or(false);
        for (index, y, button) in placed {
            let place = Rect::from_center_size(pos2(x, y + tile / 2.0), vec2(tile, tile));
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
                            let ghost = Rect::from_center_size(pointer, vec2(tile, tile));
                            painter.rect_filled(
                                ghost,
                                metrics.icon_button_radius,
                                colors.accent.gamma_multiply(0.5),
                            );
                            icons::paint(
                                &painter,
                                Rect::from_center_size(
                                    pointer,
                                    vec2(metrics.stripe_icon, metrics.stripe_icon),
                                ),
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
                metrics.icon_button_radius,
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
        let metrics = crate::tokens::metrics(ui.ctx());
        // Classic has no icon stripes: the button keeps the New UI tile size there.
        let tile = if metrics.stripe_button > 0.0 {
            metrics.stripe_button
        } else {
            stripe::BUTTON
        };
        let corner = metrics.icon_button_radius;
        // Click, and drag to another stripe group (reported by the `ToolWindowToolbar`).
        let (rect, response) = match self.id_salt {
            Some(id) => {
                let rect = ui.available_rect_before_wrap();
                let rect = Rect::from_min_size(rect.min, vec2(tile, tile));
                (rect, ui.interact(rect, id, Sense::click_and_drag()))
            }
            None => ui.allocate_exact_size(vec2(tile, tile), Sense::click_and_drag()),
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
            painter.rect_filled(rect, corner, fill);
            if response.has_focus() {
                common::focus_ring(painter, rect, corner, &colors);
            }
            let edge = if metrics.has_icon_stripes() {
                metrics.stripe_icon
            } else {
                stripe::ICON
            };
            let icon = Rect::from_center_size(rect.center(), vec2(edge, edge));
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

/// Which frame edge a classic [`Stripe`] runs along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StripeSide {
    /// The left edge: buttons read bottom to top.
    Left,
    /// The right edge: buttons read top to bottom.
    Right,
    /// The bottom edge, above the status bar: buttons read left to right.
    Bottom,
}

impl StripeSide {
    fn vertical(self) -> bool {
        self != StripeSide::Bottom
    }
}

/// A tool window button of the Classic look's [`Stripe`]: the tool window's name (with its
/// icon), rotated on the left and right stripes, filled while the tool window is open.
///
/// IntelliJ counterpart: the classic `StripeButton` (looks spec §2.3: label font −2, min 11;
/// border (5,5,0,5); hover and selected translucent fills; mnemonic number prefix).
#[derive(Debug, Clone)]
#[must_use = "give it to a `Stripe`"]
pub struct StripeButton {
    side: StripeSide,
    icon: Option<Icon>,
    label: String,
    number: Option<u8>,
    shortcut: Option<String>,
    open: bool,
    active: bool,
}

impl StripeButton {
    /// The button of the tool window `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            side: StripeSide::Left,
            icon: None,
            label: label.into(),
            number: None,
            shortcut: None,
            open: false,
            active: false,
        }
    }

    /// The stripe the button is on (default left): it reads bottom to top on the left,
    /// top to bottom on the right and left to right at the bottom. A [`Stripe`] sets it.
    pub fn side(mut self, side: StripeSide) -> Self {
        self.side = side;
        self
    }

    /// The tool window's icon before its name.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Shows the tool window's number before its name (`showToolWindowsNumbers`, "1: Project").
    pub fn number(mut self, number: u8) -> Self {
        self.number = Some(number);
        self
    }

    /// The shortcut shown in the tooltip.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Whether the tool window is open: the button is filled.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Whether the tool window is the active one.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    fn text(&self) -> String {
        match self.number {
            Some(number) => format!("{number}: {}", self.label),
            None => self.label.clone(),
        }
    }

    /// The button's length along the stripe.
    fn length(&self, ui: &Ui) -> f32 {
        let galley = common::galley(
            ui,
            self.text(),
            crate::tokens::typography::body(STRIPE_FONT),
        );
        let icon = if self.icon.is_some() {
            crate::tokens::metrics(ui.ctx()).stripe_icon + CLASSIC_ICON_GAP
        } else {
            0.0
        };
        2.0 * CLASSIC_BUTTON_PADDING + icon + galley.size().x
    }
}

/// Classic stripe button text: the label font minus 2 (looks spec §2.3).
const STRIPE_FONT: f32 = crate::tokens::typography::SMALL;
/// Space at both ends of a classic stripe button (its border (5,5,0,5)).
const CLASSIC_BUTTON_PADDING: f32 = 5.0;
/// Space between a classic stripe button's icon and name.
const CLASSIC_ICON_GAP: f32 = 4.0;
/// Space between classic stripe buttons.
const CLASSIC_BUTTON_GAP: f32 = 1.0;

/// The Classic look's tool window stripe: a bar along a frame edge with a [`StripeButton`]
/// per tool window and a 1 px line on the side of the tool windows. Clicking a button
/// reports its index, like [`ToolWindowToolbar`] (which the New UI and Islands looks use).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, wm::{Stripe, StripeButton, StripeSide}};
///
/// let clicked = Stripe::new("left", StripeSide::Left)
///     .button(StripeButton::new("Project").icon(Glyph::Folder).number(1).open(true))
///     .show(ui);
/// # let _ = clicked;
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct Stripe {
    id: Id,
    side: StripeSide,
    buttons: Vec<StripeButton>,
}

impl Stripe {
    /// An empty stripe along `side`.
    pub fn new(id_salt: impl std::hash::Hash, side: StripeSide) -> Self {
        Self {
            id: Id::new(id_salt),
            side,
            buttons: Vec::new(),
        }
    }

    /// Adds a button.
    pub fn button(mut self, button: StripeButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// Draws the stripe over the whole rect of `ui` and reports what the user did (the
    /// index of a clicked button, a close click, a popup trigger).
    pub fn show(self, ui: &mut Ui) -> StripeResponse {
        let colors = Theme::colors(ui.ctx());
        let rect = ui.max_rect();
        ui.allocate_rect(rect, Sense::hover());
        let mut out = StripeResponse {
            rect,
            clicked: None,
            close_clicked: None,
            popup: None,
            dropped: None,
        };
        ui.painter().rect_filled(rect, 0.0, colors.frame);
        // The 1 px line on the tool windows' side.
        let line = egui::Stroke::new(1.0_f32, colors.divider);
        let painter = ui.painter();
        match self.side {
            StripeSide::Left => painter.vline(rect.right() - 0.5, rect.y_range(), line),
            StripeSide::Right => painter.vline(rect.left() + 0.5, rect.y_range(), line),
            StripeSide::Bottom => painter.hline(rect.x_range(), rect.top() + 0.5, line),
        };
        let vertical = self.side.vertical();
        let thickness = if vertical {
            rect.width() - 1.0
        } else {
            rect.height() - 1.0
        };
        let modifiers = crate::wm::action_rows::pointer_event_modifiers(ui);
        let mut along = if vertical { rect.top() } else { rect.left() } + CLASSIC_BUTTON_GAP;
        for (index, button) in self.buttons.into_iter().enumerate() {
            let length = button.length(ui);
            let place = match self.side {
                StripeSide::Left => {
                    Rect::from_min_size(pos2(rect.left(), along), vec2(thickness, length))
                }
                StripeSide::Right => {
                    Rect::from_min_size(pos2(rect.left() + 1.0, along), vec2(thickness, length))
                }
                StripeSide::Bottom => {
                    Rect::from_min_size(pos2(along, rect.top() + 1.0), vec2(length, thickness))
                }
            };
            along += length + CLASSIC_BUTTON_GAP;
            let response = ui
                .push_id(self.id.with(index), |ui| {
                    ui.put(place, button.side(self.side))
                })
                .inner;
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
                let at = ui
                    .input(|i| i.pointer.latest_pos())
                    .unwrap_or(place.center());
                out.popup = Some((index, at));
            }
        }
        out
    }
}

/// A classic stripe button on its own, as long as its name and as thick as the stripe; a
/// [`Stripe`] places them for you.
impl Widget for StripeButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let metrics = crate::tokens::metrics(ui.ctx());
        let length = self.length(ui);
        let thickness = metrics.stripe_width - 1.0;
        let size = if self.side.vertical() {
            vec2(thickness, length)
        } else {
            vec2(length, thickness)
        };
        let (place, response) = ui.allocate_exact_size(size, Sense::click());
        crate::cursor::on_hover_action(&response);
        let enabled = response.enabled();
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                enabled,
                self.open || self.active,
                &self.label,
            )
        });
        if ui.is_rect_visible(place) {
            paint_text_button(ui, place, &self, &response, &metrics);
        }
        let mut tooltip = HelpTooltip::new(self.label.clone())
            .align(match self.side {
                StripeSide::Left => egui::RectAlign::RIGHT,
                StripeSide::Right => egui::RectAlign::LEFT,
                StripeSide::Bottom => egui::RectAlign::TOP,
            })
            .immediate(true);
        if let Some(shortcut) = &self.shortcut {
            tooltip = tooltip.shortcut(shortcut.clone());
        }
        tooltip.show_for(&response);
        response
    }
}

/// One classic stripe button: its fill, icon and (rotated) name.
fn paint_text_button(
    ui: &Ui,
    place: Rect,
    button: &StripeButton,
    response: &Response,
    metrics: &crate::look::Metrics,
) {
    let side = button.side;
    let colors = Theme::colors(ui.ctx());
    let painter = ui.painter();
    let fill = if button.open || button.active || response.is_pointer_button_down_on() {
        colors.pressed
    } else if response.hovered() {
        colors.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(place, 0.0, fill);
    if response.has_focus() {
        common::focus_ring(painter, place.shrink(1.0), 0, &colors);
    }
    let text_color = if !response.enabled() {
        colors.text_disabled
    } else if button.open || button.active {
        colors.text_strong
    } else {
        colors.text
    };
    let galley = common::galley(
        ui,
        button.text(),
        crate::tokens::typography::body(STRIPE_FONT),
    );
    let icon = metrics.stripe_icon;
    let has_icon = button.icon.is_some();
    let icon_room = if has_icon {
        icon + CLASSIC_ICON_GAP
    } else {
        0.0
    };
    let h = galley.size().y;
    let center = place.center();
    match side {
        StripeSide::Bottom => {
            let mut x = place.left() + CLASSIC_BUTTON_PADDING;
            if let Some(glyph) = button.icon {
                let r = Rect::from_min_size(pos2(x, center.y - icon / 2.0), vec2(icon, icon));
                icons::paint(painter, r, glyph, Tone::Icon.color(&colors));
                x += icon_room;
            }
            painter.galley(pos2(x, center.y - h / 2.0), galley, text_color);
        }
        StripeSide::Left => {
            // Reads bottom to top: the icon comes first, at the bottom.
            let mut y = place.bottom() - CLASSIC_BUTTON_PADDING;
            if let Some(glyph) = button.icon {
                let r =
                    Rect::from_min_size(pos2(center.x - icon / 2.0, y - icon), vec2(icon, icon));
                icons::paint(painter, r, glyph, Tone::Icon.color(&colors));
                y -= icon_room;
            }
            let text =
                egui::epaint::TextShape::new(pos2(center.x - h / 2.0, y), galley, text_color)
                    .with_angle(-std::f32::consts::FRAC_PI_2);
            painter.add(text);
        }
        StripeSide::Right => {
            let mut y = place.top() + CLASSIC_BUTTON_PADDING;
            if let Some(glyph) = button.icon {
                let r = Rect::from_min_size(pos2(center.x - icon / 2.0, y), vec2(icon, icon));
                icons::paint(painter, r, glyph, Tone::Icon.color(&colors));
                y += icon_room;
            }
            let text =
                egui::epaint::TextShape::new(pos2(center.x + h / 2.0, y), galley, text_color)
                    .with_angle(std::f32::consts::FRAC_PI_2);
            painter.add(text);
        }
    }
}
