//! Menu items.

use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::{SimpleColoredComponent, State, Tone};
use crate::theme::Theme;
use crate::tokens::{popup, radius, size, typography};
use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

/// A menu entry with an optional icon and right-aligned shortcut, for context menus and
/// menu bars (inside `ui.menu_button` or `response.context_menu`).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, action_system::ActionMenuItem};
///
/// ui.menu_button("Edit", |ui| {
///     if ui.add(ActionMenuItem::new("Copy").icon(Glyph::Copy).shortcut("Ctrl+C")).clicked() {
///         ui.close();
///     }
/// });
/// # });
/// ```
///
/// [`submenu`](Self::submenu) turns the item into a parent: a chevron at the right end, and
/// a menu to its right that opens on hover, <kbd>→</kbd> or <kbd>Enter</kbd> and closes on
/// <kbd>←</kbd>, <kbd>Escape</kbd> or when the pointer moves to another item. Clicking an
/// item of the submenu closes the whole menu.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::action_system::ActionMenuItem;
///
/// ui.add(ActionMenuItem::new("New").submenu(|ui| {
///     ui.add(ActionMenuItem::new("File…"));
///     ui.add(ActionMenuItem::new("Directory…"));
/// }));
/// # });
/// ```
#[must_use = "add it with `ui.add(item)`"]
pub struct ActionMenuItem<'a> {
    label: String,
    icon: Option<Icon>,
    shortcut: Option<String>,
    enabled: bool,
    checked: Option<bool>,
    highlighted: bool,
    description: Option<String>,
    submenu: Option<Submenu<'a>>,
    /// Drawn by a popup engine ([`ListPopup`](crate::ui::popup::ListPopup),
    /// [`ActionPopupMenu`](super::ActionPopupMenu)): the engine owns the highlight, the
    /// submenus and closing, so the item only paints and reports the pointer.
    managed: bool,
    /// Managed: a chevron for a row with a sub-step.
    arrow: bool,
    /// Managed: the icon column is reserved (some row of the list has an icon or check).
    reserve_icons: Option<bool>,
    /// Managed: width of the mnemonic column (0 when no row has a mnemonic).
    mnemonic_column: f32,
    mnemonic: Option<char>,
    /// Managed: the width the list gives every row.
    fixed_width: Option<f32>,
}

/// The content of a submenu.
type Submenu<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl<'a> ActionMenuItem<'a> {
    /// An item reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            enabled: true,
            checked: None,
            highlighted: false,
            description: None,
            submenu: None,
            managed: false,
            arrow: false,
            reserve_icons: None,
            mnemonic_column: 0.0,
            mnemonic: None,
            fixed_width: None,
        }
    }

    /// An item rendering `presentation`: text, icon, shortcut text, description, enabled and
    /// toggle state (a check mark), and a chevron for a popup group.
    pub fn from_presentation(presentation: &super::Presentation) -> Self {
        let mut item = Self::new(presentation.text.clone());
        item.icon = presentation.icon;
        item.shortcut = presentation.shortcut_text.clone();
        item.enabled = presentation.enabled;
        item.checked = presentation.selected;
        item.arrow = presentation.popup_group;
        if !presentation.description.is_empty() {
            item.description = Some(presentation.description.clone());
        }
        item
    }

    /// Engine use: the list owns highlight, submenus and closing.
    pub(crate) fn managed(mut self, managed: bool) -> Self {
        self.managed = managed;
        self
    }

    /// Engine use: draw the sub-step chevron.
    pub(crate) fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }

    /// Engine use: reserve (or not) the icon column.
    pub(crate) fn reserve_icons(mut self, reserve: bool) -> Self {
        self.reserve_icons = Some(reserve);
        self
    }

    /// Engine use: the mnemonic hint and the width of the mnemonic column.
    pub(crate) fn mnemonic(mut self, mnemonic: Option<char>, column: f32) -> Self {
        self.mnemonic = mnemonic;
        self.mnemonic_column = column;
        self
    }

    /// Engine use: the row's width.
    pub(crate) fn fixed_width(mut self, width: f32) -> Self {
        self.fixed_width = Some(width);
        self
    }

    /// The width this item needs, without the list's minimum (engine use: all rows of a list
    /// share the widest).
    pub(crate) fn natural_width(&self, ui: &Ui) -> f32 {
        let two_line = self.description.is_some();
        let reserve = self
            .reserve_icons
            .unwrap_or(self.icon.is_some() || self.checked.is_some());
        let content = SimpleColoredComponent {
            icon: if two_line { None } else { self.icon },
            reserve_icon: !two_line && reserve,
            label: &self.label,
            trailing: self
                .shortcut
                .as_deref()
                .filter(|_| self.submenu.is_none() && !self.arrow),
            ..SimpleColoredComponent::default()
        };
        let chrome = 2.0 * (popup::ITEM_INSET + popup::ITEM_PADDING);
        let large_icon = popup::LARGE_ICON + 2.0 * size::ICON_TEXT_GAP;
        content.min_width(ui)
            + chrome
            + self.mnemonic_column
            + if two_line { large_icon } else { 0.0 }
            + if self.submenu.is_some() || self.arrow {
                size::ICON
            } else {
                0.0
            }
    }

    /// A second, dimmed line under the label, like the path under a project in Android
    /// Studio's project picker. The item becomes two lines high with a larger icon.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Makes the item open a submenu drawn by `add` (with [`ActionMenuItem`]s). A chevron
    /// replaces the shortcut.
    pub fn submenu(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.submenu = Some(Box::new(add));
        self
    }

    /// Icon in the left column.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Shortcut shown on the right, e.g. `"Ctrl+Shift+F"`.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Disabled items are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the hover fill without the pointer: the row the keyboard is on in a
    /// suggestion list (see the template autocomplete in the code page).
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// A check mark in the icon column (for toggle items).
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
}

impl Widget for ActionMenuItem<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (icon, icon_tone) = match (self.checked, self.icon) {
            (Some(true), _) => (Some(Icon::Glyph(Glyph::Success)), Some(Tone::Default)),
            (Some(false), _) => (None, None),
            (None, icon) => (icon, None),
        };
        let two_line = self.description.is_some();
        let reserve_icons = match self.reserve_icons {
            Some(reserve) => reserve,
            None => menu_icon_column(ui, icon.is_some() || self.checked.is_some()),
        };
        let has_arrow = self.submenu.is_some() || self.arrow;
        let content = SimpleColoredComponent {
            // A two-line item paints its larger icon itself.
            icon: if two_line { None } else { icon },
            icon_tone,
            reserve_icon: !two_line && reserve_icons,
            label: &self.label,
            label_tone: Tone::Default,
            secondary: None,
            trailing: self.shortcut.as_deref().filter(|_| !has_arrow),
            ..SimpleColoredComponent::default()
        };
        // Geometry measured on Android Studio: the highlight pill is inset from the popup's
        // sides, and the icon column is inset inside the pill.
        let large_icon = popup::LARGE_ICON + 2.0 * size::ICON_TEXT_GAP;
        let width = match self.fixed_width {
            Some(width) => width,
            None => ui
                .available_width()
                .max(self.natural_width(ui))
                .max(popup::MIN_WIDTH),
        };
        let height = if two_line {
            size::ROW_HEIGHT + popup::DESCRIPTION_HEIGHT
        } else {
            size::ROW_HEIGHT
        };
        let sense = match (self.enabled, self.managed) {
            (false, _) => Sense::hover(),
            // The list owns the highlight, so rows are not Tab stops.
            (true, true) => Sense::CLICK,
            (true, false) => Sense::click(),
        };
        common::menu_rows(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
        crate::cursor::on_hover(&response, egui::CursorIcon::Default);
        response.widget_info(|| {
            let mut info = match self.checked {
                Some(checked) => {
                    WidgetInfo::selected(WidgetType::Checkbox, self.enabled, checked, &self.label)
                }
                None => WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label),
            };
            if let Some(description) = &self.description {
                info.label = Some(format!("{} {description}", self.label));
            }
            info
        });
        let current = if self.managed {
            false
        } else {
            menu_current(ui, &response, self.enabled)
        };
        if ui.is_rect_visible(rect) {
            let mut state = State::of(&response);
            state.hovered = self.enabled && (self.highlighted || current);
            let pill = rect.shrink2(vec2(popup::ITEM_INSET, 0.0));
            let fill = state.menu_fill(&colors);
            if fill != Color32::TRANSPARENT {
                ui.painter()
                    .rect_filled(pill, CornerRadius::same(radius::ROW), fill);
            }
            let mut x = pill.left() + popup::ITEM_PADDING;
            if self.mnemonic_column > 0.0 {
                if let Some(mnemonic) = self.mnemonic {
                    let galley = common::galley(
                        ui,
                        mnemonic.to_string(),
                        typography::body(typography::BODY),
                    );
                    common::text_left(ui.painter(), pill, x, galley, colors.popup_mnemonic);
                }
                x += self.mnemonic_column;
            }
            if let Some(description) = &self.description {
                let line = Rect::from_min_size(pill.min, vec2(pill.width(), size::ROW_HEIGHT));
                if let Some(icon) = icon {
                    let icon_rect = Rect::from_center_size(
                        pos2(x + popup::LARGE_ICON / 2.0, line.center().y),
                        vec2(popup::LARGE_ICON, popup::LARGE_ICON),
                    );
                    icons::paint(
                        ui.painter(),
                        icon_rect,
                        icon,
                        state.icon(icon_tone.unwrap_or(Tone::Icon), &colors),
                    );
                }
                let text_x = x + large_icon;
                content.paint(ui, line, text_x, state);
                let below = Rect::from_min_size(
                    pos2(line.left(), line.bottom()),
                    vec2(
                        line.width(),
                        popup::DESCRIPTION_HEIGHT - 2.0 * size::ICON_TEXT_GAP,
                    ),
                );
                let galley = common::galley_elided(
                    ui,
                    description,
                    typography::body(typography::BODY),
                    (pill.right() - popup::ITEM_PADDING - text_x).max(0.0),
                );
                let color = if self.enabled {
                    colors.text_placeholder
                } else {
                    colors.text_disabled
                };
                common::text_left(ui.painter(), below, text_x, galley, color);
            } else {
                content.paint(ui, pill, x, state);
            }
            if has_arrow {
                let chevron = Rect::from_min_size(
                    pos2(
                        pill.right() - size::ROW_PILL_INSET - size::ICON,
                        rect.top() + (size::ROW_HEIGHT - size::ICON) / 2.0,
                    ),
                    vec2(size::ICON, size::ICON),
                );
                icons::paint(
                    ui.painter(),
                    chevron,
                    Glyph::ChevronRight,
                    state.icon(Tone::Icon, &colors),
                );
            }
        }
        if self.managed {
            return response;
        }
        match self.submenu {
            Some(submenu) if self.enabled => show_submenu(ui, &response, submenu),
            Some(_) => {}
            // A click on an item closes the menu it is in.
            None if response.clicked() => ui.close(),
            None => {}
        }
        response
    }
}

/// The row a menu's pointer and keyboard share, per menu (the `Ui` its items are in).
#[derive(Clone, Copy, Default)]
struct MenuCursor {
    /// The current item: the last one hovered or focused. It stays when the pointer leaves.
    item: Option<egui::Id>,
    /// The frame the pointer first moved after the menu opened; that move is ignored.
    armed: Option<u64>,
    /// The last frame the menu was drawn, to notice it was closed and reopened.
    seen: u64,
    /// Where the pointer was, to tell a real move from a frame drawn under a still pointer.
    pointer: Option<egui::Pos2>,
    /// Whether the pointer moved in frame `seen`.
    moved: bool,
    /// Whether an item of this menu has an icon (this frame or the last): only then do the
    /// items reserve an icon column, as in Android Studio.
    icons: bool,
    /// The same, for the frame being drawn.
    icons_now: bool,
    /// Whether a frame was already rolled over (the first one keeps `icons` as found).
    seen_once: bool,
}

/// Whether the item of `response` is its menu's current row, as in Android Studio: hover and
/// arrow keys move one shared highlight; it stays on the last item when the pointer leaves;
/// and the first pointer move after the menu opens is ignored, so a menu that opens under
/// the pointer does not highlight whatever happens to be there.
fn menu_current(ui: &Ui, response: &Response, enabled: bool) -> bool {
    let ctx = ui.ctx();
    let frame = ctx.cumulative_frame_nr();
    let (key, mut cursor) = menu_cursor(ui);
    let moved = cursor.moved;
    if moved && cursor.armed.is_none() {
        cursor.armed = Some(frame);
    }
    let armed = cursor.armed.is_some_and(|f| f < frame);
    if enabled && response.hovered() && moved && armed {
        cursor.item = Some(response.id);
        // The keyboard continues from where the pointer is.
        if !response.has_focus() {
            response.request_focus();
        }
    }
    if enabled && response.has_focus() {
        cursor.item = Some(response.id);
    }
    ctx.data_mut(|d| d.insert_temp(key, cursor));
    enabled && cursor.item == Some(response.id)
}

/// Whether the items of the menu `ui` holds reserve an icon column: only when one of them
/// has an icon, as in Android Studio (a list of plain names starts its text at the pill's
/// padding). `has_icon` is this item's.
fn menu_icon_column(ui: &Ui, has_icon: bool) -> bool {
    let (key, mut cursor) = menu_cursor(ui);
    cursor.icons_now |= has_icon;
    ui.ctx().data_mut(|d| d.insert_temp(key, cursor));
    cursor.icons || cursor.icons_now
}

/// The shared state of the menu `ui` holds, rolled over to the current frame.
fn menu_cursor(ui: &Ui) -> (egui::Id, MenuCursor) {
    let ctx = ui.ctx();
    let key = ui.id().with("kit-menu-cursor");
    let frame = ctx.cumulative_frame_nr();
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let mut cursor = ctx
        .data(|d| d.get_temp::<MenuCursor>(key))
        .unwrap_or_default();
    if cursor.seen + 1 < frame {
        // Not drawn last frame: the menu (re)opened, under a pointer that has not moved yet.
        // What its items look like (icons) is kept.
        cursor = MenuCursor {
            pointer,
            icons: cursor.icons || cursor.icons_now,
            ..MenuCursor::default()
        };
    }
    if cursor.seen != frame {
        // First item this frame: did the pointer move since the last one?
        cursor.moved = pointer.is_some() && pointer != cursor.pointer;
        cursor.pointer = pointer;
        cursor.seen = frame;
        if cursor.seen_once {
            cursor.icons = cursor.icons_now;
        }
        cursor.icons_now = false;
        cursor.seen_once = true;
    }
    (key, cursor)
}

/// Open state of a submenu: whether it is open, and the frame the item was last drawn in
/// (a menu that was closed forgets which submenu was open).
#[derive(Clone, Copy, Default)]
struct SubmenuState {
    open: bool,
    seen: u64,
}

/// Opens, shows and closes the submenu of the item `response`.
fn show_submenu(ui: &mut Ui, response: &Response, add: Submenu<'_>) {
    let ctx = ui.ctx().clone();
    // The state outlives the menu, which stops drawing its items when it closes: nothing
    // tells the item it was closed. So the item records the frame it was last drawn in, and
    // an item that was not drawn in the previous frame (its menu was closed and has just
    // reopened) starts closed instead of showing a stale open submenu.
    let frame = ctx.cumulative_frame_nr();
    let mut state: SubmenuState = ctx.data(|d| d.get_temp(response.id)).unwrap_or_default();
    if frame > state.seen + 1 {
        state.open = false;
    }
    state.seen = frame;
    let popup_id = response.id.with("submenu");
    let submenu_rect = ctx.memory(|m| m.area_rect(egui::Id::new(popup_id)));

    // (Not inside `ctx.input`: the context's lock is not reentrant.)
    let focused = response.has_focus();
    let keyed = (focused || response.hovered())
        && ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                || (focused && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        });
    let was_open = state.open;
    // Hover opens when the pointer moves onto the item, so Left can close it under a still
    // pointer.
    let moving = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
    if (response.hovered() && moving) || response.clicked() || keyed {
        state.open = true;
    }
    if state.open {
        // Left closes; so does moving the pointer to something that is neither this item
        // nor its submenu. (Escape is handled by the popover.)
        let left = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
        let away = ctx.input(|i| {
            i.pointer.delta() != egui::Vec2::ZERO
                && i.pointer.hover_pos().is_some_and(|p| {
                    !response.rect.contains(p) && !submenu_rect.is_some_and(|r| r.contains(p))
                })
        });
        if left || away {
            state.open = false;
            if left {
                response.request_focus();
            }
        }
    }
    let mut open = state.open;
    let shown = crate::ui::popup::JBPopup::right_of(popup_id, response)
        .width(size::SUBMENU_WIDTH)
        .show(&ctx, &mut open, add);
    if state.open && !open {
        // Escape (or a click outside) closed it: keep the item focused for the next key.
        response.request_focus();
    }
    if shown.is_some() && keyed && !was_open {
        // Into the submenu: the first item is the next widget in the focus order.
        ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
    }
    state.open = open;
    ctx.data_mut(|d| d.insert_temp(response.id, state));
    ctx.accesskit_node_builder(response.id, |node| {
        node.set_expanded(state.open);
        node.set_has_popup(egui::accesskit::HasPopup::Menu);
    });
}
