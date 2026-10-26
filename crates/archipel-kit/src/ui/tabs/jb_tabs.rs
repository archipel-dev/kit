//! Editor tabs.

use crate::action_system::{ActionGroup, ActionPlace, DataContext, flatten};
use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::theme::Theme;
use crate::tokens::{motion, popup, radius, size, tab, typography};
use egui::{
    Color32, CornerRadius, EventFilter, Id, Key, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    WidgetInfo, WidgetType, pos2, vec2,
};
use std::rc::Rc;

/// One tab of a [`JBTabs`].
///
/// IntelliJ counterpart: `com.intellij.ui.tabs.TabInfo` (text, icon, tab actions, pinned
/// state; the closable close button is the platform's tab action).
#[derive(Clone)]
pub struct TabInfo {
    label: String,
    icon: Option<Icon>,
    closable: bool,
    modified: bool,
    pinned: bool,
    enabled: bool,
    actions: Option<Rc<dyn ActionGroup>>,
}

impl std::fmt::Debug for TabInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabInfo")
            .field("label", &self.label)
            .field("icon", &self.icon)
            .field("closable", &self.closable)
            .field("modified", &self.modified)
            .field("pinned", &self.pinned)
            .field("enabled", &self.enabled)
            .field(
                "actions",
                &self.actions.as_ref().map(|group| group.name().to_owned()),
            )
            .finish()
    }
}

impl TabInfo {
    /// A tab titled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            closable: false,
            modified: false,
            pinned: false,
            enabled: true,
            actions: None,
        }
    }

    /// Icon before the title (e.g. [`Icon::for_file`]).
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Shows a close button (always on the selected tab, on hover on the others).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Marks unsaved changes with a dot after the title.
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// A pinned tab shows a pin where the close button would be (always, like IntelliJ's
    /// pinned editor tabs) and cannot be closed by a middle or Shift click; clicking the pin
    /// reports [`JBTabsResponse::pin_toggled`]. Keeping pinned tabs first is the caller's
    /// job.
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Whether the tab is pinned.
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }

    /// A disabled tab ignores the pointer and the arrow keys skip it (IntelliJ
    /// `TabInfo.setEnabled`).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Tab actions (IntelliJ `TabInfo.setTabLabelActions`, place
    /// [`ActionPlace::EDITOR_TAB`]): the visible actions of `group` are drawn as small
    /// [`ActionButton`](crate::action_system::ActionButton)s before the close button, shown
    /// like it (on the selected tab and on hover, with the same 150 ms dead zone). A click
    /// performs the action; it is reported in [`JBTabsResponse::performed`].
    pub fn actions(mut self, group: Rc<dyn ActionGroup>) -> Self {
        self.actions = Some(group);
        self
    }
}

/// What happened in a [`JBTabs`] this frame.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct JBTabsResponse {
    /// Index of the tab the user selected, if they selected another one (by clicking it or
    /// by picking it from the hidden tabs).
    pub selected: Option<usize>,
    /// Index of the tab whose close button was clicked (or that was middle-clicked).
    pub closed: Option<usize>,
    /// The "Show hidden tabs" button, while the tabs don't fit.
    pub hidden_tabs_button: Option<Response>,
    /// The "More actions" button, when the bar has a [`JBTabs::more_menu`].
    pub more_button: Option<Response>,
    /// A tab the user dragged to a new place, as `(from, to)`: the caller moves the tab
    /// (`tabs.insert(to, tabs.remove(from))`) and its selection. Reported on release.
    pub moved: Option<(usize, usize)>,
    /// The tab whose pin was clicked (pinned tabs only).
    pub pin_toggled: Option<usize>,
    /// The tab the user right-clicked (the popup group, if any, opens at the pointer).
    pub popup_requested: Option<usize>,
    /// A tab action or a popup action that ran, as the index of the tab (`None` for the
    /// popup group) and the action id. [`JBTabs`] performs the action itself, through
    /// [`perform_with_modifiers`](crate::action_system::perform_with_modifiers).
    pub performed: Option<(Option<usize>, String)>,
}

/// The content of the "More actions" menu.
type MoreMenu<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// Editor tabs as in Android Studio's Islands UI: the selected tab is a tinted pill with a
/// border; others show a hover pill and their close button on hover.
///
/// Tabs that don't fit scroll sideways, as in Android Studio: with the mouse wheel (a
/// vertical wheel scrolls the strip too), with a trackpad, or by dragging the thin scroll
/// bar at the strip's top edge, which shows while the pointer is over the strip. A
/// "Show hidden tabs" button lists the tabs that are out of view; the selected tab is
/// scrolled into view when it changes.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Icon, action_system::ActionMenuItem, ui::tabs::{TabInfo, JBTabs}};
///
/// let mut current = 0;
/// let tabs = [
///     TabInfo::new("main.rs").icon(Icon::for_file("main.rs")).closable(true),
///     TabInfo::new("Cargo.toml").icon(Icon::for_file("Cargo.toml")).closable(true).modified(true),
/// ];
/// let out = JBTabs::new(&tabs, current)
///     .more_menu(|ui| {
///         if ui.add(ActionMenuItem::new("Close All Tabs")).clicked() { /* ... */ }
///     })
///     .show(ui);
/// if let Some(index) = out.selected { current = index; }
/// # });
/// ```
#[must_use = "show it with `tab_bar.show(ui)`"]
pub struct JBTabs<'a> {
    tabs: &'a [TabInfo],
    selected: usize,
    separator: bool,
    id_salt: Option<Id>,
    more: Option<MoreMenu<'a>>,
    reorderable: bool,
    popup_group: Option<&'a dyn ActionGroup>,
    data_context: Option<&'a DataContext>,
}

/// What a [`JBTabs`] remembers between frames: where it is scrolled and which of its popups
/// is open. The caller owns the tabs and the selection.
#[derive(Debug, Clone, Copy, Default)]
struct Strip {
    /// How far the tabs are scrolled, in points.
    offset: f32,
    /// The selected tab and the number of tabs last frame, to notice a change.
    seen: Option<(usize, usize)>,
    /// The tab that had keyboard focus last frame.
    focused: Option<usize>,
    /// A tab to scroll into view next frame (picked from the hidden tabs).
    reveal: Option<usize>,
    /// When the pointer was last over the strip, or it last scrolled (the scroll bar shows
    /// until a delay after that).
    active: Option<f64>,
    hidden_open: bool,
    more_open: bool,
    /// A press on a tab that may become a drag: the tab and where the pointer went down.
    press: Option<(usize, egui::Pos2)>,
    /// A tab being dragged: the tab and how far the pointer is from its left edge.
    drag: Option<(usize, f32)>,
    /// The tab whose context menu is open, and where.
    popup: Option<(usize, egui::Pos2)>,
}

impl<'a> JBTabs<'a> {
    /// `tabs` with the one at `selected` active.
    pub fn new(tabs: &'a [TabInfo], selected: usize) -> Self {
        Self {
            tabs,
            selected,
            separator: true,
            id_salt: None,
            more: None,
            reorderable: true,
            popup_group: None,
            data_context: None,
        }
    }

    /// Whether tabs can be dragged to another place (default `true`, as in IntelliJ's
    /// editor tabs). The drag starts 7 px from the press; the tab follows the pointer, its
    /// neighbors move aside when its center crosses them, and the new place is reported in
    /// [`JBTabsResponse::moved`] on release.
    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
        self
    }

    /// The context menu of a tab (IntelliJ `JBTabs.setPopupGroup`, place
    /// [`ActionPlace::EDITOR_TAB_POPUP`]): opened by a right click on a tab, drawn from the
    /// visible actions of `group`; the action clicked is performed and reported in
    /// [`JBTabsResponse::performed`].
    pub fn popup_group(mut self, group: &'a dyn ActionGroup) -> Self {
        self.popup_group = Some(group);
        self
    }

    /// The data context the tab actions and the popup group update and run with (default:
    /// empty).
    pub fn data_context(mut self, data_context: &'a DataContext) -> Self {
        self.data_context = Some(data_context);
        self
    }

    /// The 1 pt line below the bar, edge to edge in the frame color, that separates an
    /// island's header from its content (on by default, like the tool window headers of a
    /// [`crate::wm::ToolWindowPane`]). Turn it off where the tabs are not an island's header.
    pub fn separator(mut self, enabled: bool) -> Self {
        self.separator = enabled;
        self
    }

    /// Tells two tab bars in the same `Ui` apart (each remembers its scroll position).
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// A "More actions" button (⋮) at the strip's right end, opening a menu drawn by `add`
    /// with [`ActionMenuItem`](crate::action_system::ActionMenuItem)s: the app's tab actions (close all, recent
    /// files…). A click on a `ActionMenuItem` closes the menu (call `ui.close()` for other
    /// widgets).
    pub fn more_menu(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.more = Some(Box::new(add));
        self
    }

    /// Draws the tabs in a strip as wide as the `Ui`, [`size::HEADER_HEIGHT`] high: put it at
    /// the top of an island, where Android Studio has its editor tabs. The first tab starts
    /// [`tab::STRIP_INSET`] from the left edge.
    pub fn show(self, ui: &mut Ui) -> JBTabsResponse {
        let colors = Theme::colors(ui.ctx());
        let mut out = JBTabsResponse::default();
        let id = ui.id().with(("archipel-tab-bar", self.id_salt));
        let mut state: Strip = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        let separator_range = ui.max_rect().x_range();
        let enabled = ui.is_enabled();
        let count = self.tabs.len();

        let empty_context = DataContext::empty();
        let data_context = self.data_context.unwrap_or(&empty_context);
        // The number of action buttons of each tab: their room is kept on every tab that
        // has them, so showing them does not move the tabs.
        let action_counts: Vec<usize> = self
            .tabs
            .iter()
            .map(|item| {
                item.actions.as_ref().map_or(0, |group| {
                    flatten(group.as_ref(), data_context, &ActionPlace::EDITOR_TAB)
                        .iter()
                        .filter(|row| !row.is_separator())
                        .count()
                })
            })
            .collect();

        // Each tab's width, and where it starts in the scrolled content (the strip's left
        // edge at offset 0). A title is cut off with an ellipsis at the widest a tab gets.
        let galleys: Vec<_> = self
            .tabs
            .iter()
            .map(|item| common::galley(ui, item.label.as_str(), typography::body(typography::BODY)))
            .collect();
        let widths: Vec<f32> = self
            .tabs
            .iter()
            .zip(&galleys)
            .zip(&action_counts)
            .map(|((item, galley), &actions)| tab_width(item, galley.size().x, actions))
            .collect();
        let layout_starts = |order: &[usize]| {
            let mut starts = vec![0.0; count];
            let mut x = tab::STRIP_INSET;
            for &index in order {
                starts[index] = x;
                x += widths[index] + tab::GAP;
            }
            (starts, x)
        };
        let natural: Vec<usize> = (0..count).collect();
        let (natural_starts, x) = layout_starts(&natural);
        let content = if count == 0 { x } else { x - tab::GAP };

        // The strip, its buttons at the right end, and the scrolling area before them.
        let button = vec2(size::ICON_BUTTON, size::ICON_BUTTON);
        let available = ui.available_width();
        let width = if available.is_finite() {
            available
        } else {
            content + tab::STRIP_BUTTONS_GAP + 2.0 * size::ICON_BUTTON + tab::BUTTON_GAP
        };
        let (strip, _) = ui.allocate_exact_size(vec2(width, size::HEADER_HEIGHT), Sense::hover());
        let mut right = strip.right() - tab::BUTTONS_INSET;
        let more_rect = self.more.is_some().then(|| {
            let rect = Rect::from_center_size(
                pos2(right - size::ICON_BUTTON / 2.0, strip.center().y),
                button,
            );
            right = rect.left() - tab::BUTTON_GAP;
            rect
        });
        let end = match more_rect {
            Some(rect) => rect.left() - tab::STRIP_BUTTONS_GAP,
            None => strip.right(),
        };
        let overflow = content > end - strip.left() + 0.5;
        let caret_rect = overflow.then(|| {
            Rect::from_center_size(
                pos2(right - size::ICON_BUTTON / 2.0, strip.center().y),
                button,
            )
        });
        let end = match caret_rect {
            Some(rect) => rect.left() - tab::STRIP_BUTTONS_GAP,
            None => end,
        };
        let viewport = Rect::from_x_y_ranges(strip.left()..=end.max(strip.left()), strip.y_range());
        let view = viewport.width();
        let max_offset = (content - view).max(0.0);
        let now = ui.input(|i| i.time);

        // The wheel scrolls the strip sideways: a vertical wheel too, as in Android Studio.
        if overflow && ui.rect_contains_pointer(viewport) {
            let delta = ui.input(|i| i.smooth_scroll_delta);
            let step = delta.x + delta.y;
            if step != 0.0 {
                state.offset -= step;
                ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
                state.active = Some(now);
            }
        }

        // The selected tab is scrolled into view when it changes (or tabs come and go), and
        // so is a tab that gets keyboard focus.
        let tab_id = |index: usize| id.with(("tab", index));
        let focused = ui
            .memory(|m| m.focused())
            .and_then(|focus| (0..count).find(|&index| tab_id(index) == focus));
        let mut reveal = state.reveal.take();
        if state.seen != Some((self.selected, count)) && self.selected < count {
            reveal = reveal.or(Some(self.selected));
        }
        if focused.is_some() && focused != state.focused {
            reveal = focused;
        }
        state.focused = focused;
        state.seen = Some((self.selected, count));
        if let Some(index) = reveal.filter(|&index| index < count) {
            // Clear of the fade at the edge it is scrolled to.
            let (left, right) = (natural_starts[index], natural_starts[index] + widths[index]);
            if left < state.offset {
                state.offset = if index == 0 {
                    0.0
                } else {
                    left - tab::EDGE_FADE
                };
            } else if right > state.offset + view {
                state.offset = right + tab::EDGE_FADE - view;
            }
        }
        state.offset = state.offset.clamp(0.0, max_offset);

        // The scroll bar: a thin thumb at the strip's top edge, across the whole island.
        let track = (
            strip.left() + tab::SCROLL_INSET,
            strip.right() - tab::SCROLL_INSET,
        );
        let track_width = (track.1 - track.0).max(0.0);
        let thumb_width = (track_width * view / content.max(1.0))
            .max(ui.spacing().scroll.handle_min_length)
            .min(track_width);
        let thumb_x = |offset: f32| {
            track.0
                + if max_offset > 0.0 {
                    (track_width - thumb_width) * offset / max_offset
                } else {
                    0.0
                }
        };
        let (mut thumb_hovered, mut dragging) = (false, false);
        if overflow {
            // `Sense::DRAG` alone: the thumb is a pointer target, not a Tab stop (the
            // keyboard reaches hidden tabs through the hidden tabs list).
            let hit = Rect::from_min_size(
                pos2(thumb_x(state.offset), strip.top()),
                vec2(
                    thumb_width,
                    2.0 * tab::SCROLL_MARGIN + tab::SCROLL_THUMB_HOVER,
                ),
            );
            let thumb = ui.interact(hit, id.with("thumb"), Sense::DRAG);
            if thumb.dragged() && track_width > thumb_width {
                state.offset = (state.offset
                    + thumb.drag_delta().x * max_offset / (track_width - thumb_width))
                    .clamp(0.0, max_offset);
            }
            thumb_hovered = thumb.hovered();
            dragging = thumb.dragged();
        }
        let hovered = ui.rect_contains_pointer(strip);
        if overflow && (hovered || dragging) {
            state.active = Some(now);
        }
        let thumb_alpha = match state.active {
            Some(active) if overflow => {
                let since = (now - active) as f32;
                if since <= motion::TAB_SCROLL_HIDE_DELAY {
                    1.0
                } else {
                    (1.0 - (since - motion::TAB_SCROLL_HIDE_DELAY) / motion::TAB_SCROLL_FADE_OUT)
                        .max(0.0)
                }
            }
            _ => 0.0,
        };
        if thumb_alpha > 0.0 && !hovered && !dragging {
            ui.ctx().request_repaint();
        } else if thumb_alpha == 0.0 {
            state.active = None;
        }

        // Dragging a tab: it starts 7 px from the press, follows the pointer, and the others
        // move aside when its center crosses them. The place is reported on release.
        let (primary_down, released, latest) = ui.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.pointer.latest_pos(),
            )
        });
        if self.reorderable && state.drag.is_none() {
            if let Some((pressed, origin)) = state.press {
                if !primary_down || pressed >= count {
                    state.press = None;
                } else if latest.is_some_and(|pos| pos.distance(origin) > tab::DRAG_DEADZONE) {
                    let grab = origin.x - (strip.left() + natural_starts[pressed] - state.offset);
                    state.drag = Some((pressed, grab));
                    state.press = None;
                }
            }
        } else if !self.reorderable {
            state.press = None;
        }
        let mut starts = natural_starts.clone();
        if let Some((dragged, grab)) = state.drag {
            if dragged >= count || (!primary_down && !released) {
                state.drag = None;
            } else if let Some(pos) = latest {
                let lowest = tab::STRIP_INSET;
                let want = (pos.x - strip.left() - grab + state.offset)
                    .clamp(lowest, (content - widths[dragged]).max(lowest));
                let others: Vec<usize> = (0..count).filter(|&index| index != dragged).collect();
                let (rest, _) = layout_starts(&others);
                let center = want + widths[dragged] / 2.0;
                let target = others
                    .iter()
                    .filter(|&&index| rest[index] + widths[index] / 2.0 < center)
                    .count();
                let mut order = others;
                order.insert(target, dragged);
                starts = layout_starts(&order).0;
                starts[dragged] = want;
                if released {
                    if target != dragged {
                        out.moved = Some((dragged, target));
                    }
                    state.drag = None;
                }
            }
        }
        let dragging_tab = state.drag.map(|(index, _)| index);

        // The tabs, clipped to the scrolling area.
        let mut tabs_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id.with("tabs"))
                .max_rect(viewport),
        );
        tabs_ui.set_clip_rect(viewport.intersect(ui.clip_rect()));
        // The tab being dragged is drawn last, over the others.
        let mut paint_order: Vec<usize> = (0..count).collect();
        if let Some(dragged) = dragging_tab {
            paint_order.retain(|&index| index != dragged);
            paint_order.push(dragged);
        }
        for index in paint_order {
            let (item, galley) = (&self.tabs[index], galleys[index].clone());
            let rect = Rect::from_min_size(
                pos2(strip.left() + starts[index] - state.offset, strip.top()),
                vec2(widths[index], size::HEADER_HEIGHT),
            );
            let selected = index == self.selected;
            let env = TabEnv {
                id: tab_id(index),
                colors: &colors,
                now,
                data_context,
                actions: action_counts[index],
                hover_allowed: dragging_tab.is_none(),
                dragged: dragging_tab == Some(index),
            };
            let tab_out = draw_tab(&mut tabs_ui, &env, rect, item, galley, selected);
            if tab_out.select && !selected {
                out.selected = Some(index);
            }
            if tab_out.close {
                out.closed = Some(index);
                out.selected = None;
            }
            if tab_out.pin {
                out.pin_toggled = Some(index);
            }
            if let Some(pos) = tab_out.press {
                state.press = Some((index, pos));
            }
            if let Some(pos) = tab_out.popup {
                out.popup_requested = Some(index);
                state.popup = self.popup_group.is_some().then_some((index, pos));
            }
            if let Some(action) = tab_out.performed {
                out.performed = Some((Some(index), action));
            }
        }

        // Left and Right on a focused tab select the previous or next enabled tab and move
        // the focus to it.
        if let Some(from) = focused.filter(|&index| index < count) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    tab_id(from),
                    EventFilter {
                        horizontal_arrows: true,
                        ..EventFilter::default()
                    },
                );
            });
            let press = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
            let step = if press(Key::ArrowRight) {
                Some(1isize)
            } else if press(Key::ArrowLeft) {
                Some(-1)
            } else {
                None
            };
            if let Some(step) = step {
                let mut next = from as isize + step;
                while (0..count as isize).contains(&next) && !self.tabs[next as usize].enabled {
                    next += step;
                }
                if (0..count as isize).contains(&next) {
                    let next = next as usize;
                    if next != self.selected {
                        out.selected = Some(next);
                    }
                    ui.memory_mut(|m| m.request_focus(tab_id(next)));
                    state.reveal = Some(next);
                    ui.ctx().request_repaint();
                }
            }
        }

        // The context menu of a tab, at the pointer.
        if let (Some((_, pos)), Some(group)) = (state.popup, self.popup_group) {
            let mut open = true;
            let place = ActionPlace::EDITOR_TAB_POPUP;
            let done = crate::ui::popup::JBPopup::new(
                id.with("tab-popup"),
                Rect::from_center_size(pos, vec2(8.0, 8.0)),
            )
            .width(size::SUBMENU_WIDTH)
            .show(ui.ctx(), &mut open, |ui| {
                let rows = flatten(group, data_context, &place);
                crate::wm::action_rows::show_menu_rows(ui, &rows, data_context, &place)
            })
            .flatten();
            if let Some(action) = done {
                out.performed = Some((None, action));
                open = false;
            }
            if !open {
                state.popup = None;
            }
        }
        // The tabs' parent node gets the scrolling area's bounds, so tools see which tabs
        // are cut off.
        ui.ctx()
            .accesskit_node_builder(tabs_ui.unique_id(), |node| {
                node.set_role(egui::accesskit::Role::TabList);
                node.set_bounds(egui::accesskit::Rect {
                    x0: viewport.min.x.into(),
                    y0: viewport.min.y.into(),
                    x1: viewport.max.x.into(),
                    y1: viewport.max.y.into(),
                });
            });

        // Tabs cut off at an edge fade out there.
        if overflow {
            let painter = tabs_ui.painter();
            if state.offset > 0.5 {
                fade(
                    painter,
                    viewport.left(),
                    viewport.left() + tab::EDGE_FADE,
                    strip,
                    colors.island,
                    true,
                );
            }
            if state.offset < max_offset - 0.5 {
                fade(
                    painter,
                    viewport.right() - tab::EDGE_FADE,
                    viewport.right(),
                    strip,
                    colors.island,
                    false,
                );
            }
        }

        // The buttons: "Show hidden tabs" while tabs don't fit, then "More actions".
        let mut buttons_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id.with("buttons"))
                .max_rect(strip),
        );
        let popup_anchor = |rect: Rect| {
            // The popup hangs right below the header, as in Android Studio.
            Rect::from_min_max(
                rect.left_top(),
                pos2(rect.right(), strip.bottom() - size::POPUP_PADDING),
            )
        };
        match caret_rect {
            Some(rect) => {
                let caret = buttons_ui.put(
                    rect,
                    crate::action_system::ActionButton::new(Glyph::ChevronDown, "Show hidden tabs")
                        .enabled(enabled),
                );
                if caret.clicked() {
                    state.hidden_open = !state.hidden_open;
                    state.more_open = false;
                }
                let just_opened = caret.clicked() && state.hidden_open;
                let hidden: Vec<usize> = (0..count)
                    .filter(|&index| {
                        natural_starts[index] < state.offset - 0.5
                            || natural_starts[index] + widths[index] > state.offset + view + 0.5
                    })
                    .collect();
                let label_width = hidden
                    .iter()
                    .map(|&index| galleys[index].size().x)
                    .fold(0.0, f32::max);
                let width = label_width
                    + size::ICON
                    + size::ICON_TEXT_GAP
                    + 2.0 * (popup::ITEM_INSET + popup::ITEM_PADDING);
                let was_open = state.hidden_open;
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                let mut picked = None;
                crate::ui::popup::JBPopup::new(id.with("hidden-tabs"), popup_anchor(rect))
                    .width(width)
                    .show(ui.ctx(), &mut state.hidden_open, |ui| {
                        for (row, &index) in hidden.iter().enumerate() {
                            let item = &self.tabs[index];
                            let mut entry =
                                crate::action_system::ActionMenuItem::new(item.label.as_str());
                            if let Some(icon) = item.icon {
                                entry = entry.icon(icon);
                            }
                            let response = ui.add(entry);
                            // The list opens on its first row, for the arrow keys.
                            if just_opened && row == 0 {
                                response.request_focus();
                            }
                            if response.clicked() {
                                picked = Some(index);
                            }
                        }
                    });
                if let Some(index) = picked {
                    state.hidden_open = false;
                    state.reveal = Some(index);
                    ui.ctx().request_repaint();
                    if index != self.selected {
                        out.selected = Some(index);
                    }
                    caret.request_focus();
                } else if was_open && !state.hidden_open && escape {
                    caret.request_focus();
                }
                out.hidden_tabs_button = Some(caret);
            }
            None => state.hidden_open = false,
        }
        if let (Some(add), Some(rect)) = (self.more, more_rect) {
            let more = buttons_ui.put(
                rect,
                crate::action_system::ActionButton::new(Glyph::MoreVertical, "More actions")
                    .enabled(enabled),
            );
            if more.clicked() {
                state.more_open = !state.more_open;
                state.hidden_open = false;
            }
            let was_open = state.more_open;
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let close = crate::ui::popup::JBPopup::new(id.with("more-actions"), popup_anchor(rect))
                .width(size::SUBMENU_WIDTH)
                .show(ui.ctx(), &mut state.more_open, |ui| {
                    // A clicked `ActionMenuItem` closes the menu it is in.
                    ui.scope_builder(egui::UiBuilder::new().closable(), |ui| {
                        add(ui);
                        ui.should_close()
                    })
                    .inner
                });
            if close == Some(true) {
                state.more_open = false;
            } else if was_open && !state.more_open && escape {
                more.request_focus();
            }
            out.more_button = Some(more);
        }

        // The scroll bar's thumb, over the tabs.
        if thumb_alpha > 0.0 {
            let (thickness, color) = if thumb_hovered || dragging {
                (tab::SCROLL_THUMB_HOVER, colors.tab_scroll_thumb_hover)
            } else {
                (tab::SCROLL_THUMB, colors.tab_scroll_thumb)
            };
            let thumb = Rect::from_min_size(
                pos2(thumb_x(state.offset), strip.top() + tab::SCROLL_MARGIN),
                vec2(thumb_width, thickness),
            );
            ui.painter()
                .rect_filled(thumb, thickness / 2.0, color.gamma_multiply(thumb_alpha));
        }
        if self.separator {
            ui.painter().hline(
                separator_range,
                strip.bottom() + 0.5,
                Stroke::new(1.0_f32, colors.frame),
            );
        }
        ui.data_mut(|d| d.insert_temp(id, state));
        out
    }
}

/// Room taken by the tab's buttons after its title: its actions, then the close button or
/// pin. Kept on every tab that has them, so showing them on hover moves nothing.
fn trailing_width(item: &TabInfo, actions: usize) -> f32 {
    let actions_width = if actions > 0 {
        actions as f32 * (size::ICON_BUTTON_SMALL + tab::ACTION_GAP) - tab::ACTION_GAP
    } else {
        0.0
    };
    let button = item.closable || item.pinned;
    match (actions > 0, button) {
        (false, false) => tab::PADDING_LEFT,
        (false, true) => tab::CLOSE_GAP + tab::CLOSE + tab::PADDING_RIGHT,
        (true, false) => tab::CLOSE_GAP + actions_width + tab::PADDING_RIGHT,
        (true, true) => {
            tab::CLOSE_GAP + actions_width + tab::CLOSE_GAP + tab::CLOSE + tab::PADDING_RIGHT
        }
    }
}

/// The width of `item`'s pill, whose title is `text` wide: at most [`tab::MAX_WIDTH`], where
/// the title is cut off.
fn tab_width(item: &TabInfo, text: f32, actions: usize) -> f32 {
    let icon = if item.icon.is_some() {
        size::ICON + size::ICON_TEXT_GAP
    } else {
        0.0
    };
    let dot = if item.modified { MODIFIED_DOT } else { 0.0 };
    let fixed = tab::PADDING_LEFT + icon + dot + trailing_width(item, actions);
    fixed + text.min((tab::MAX_WIDTH - fixed).max(0.0))
}

/// Room for the unsaved-changes dot after a title.
const MODIFIED_DOT: f32 = 10.0;

/// What a frame knows about the strip while drawing one tab.
struct TabEnv<'e> {
    id: Id,
    colors: &'e crate::theme::Colors,
    now: f64,
    data_context: &'e DataContext,
    /// How many action buttons the tab has.
    actions: usize,
    /// False while another tab is dragged: no hover, no buttons.
    hover_allowed: bool,
    /// Whether this tab is the one being dragged.
    dragged: bool,
}

/// What the user did to one tab.
#[derive(Default)]
struct TabOutcome {
    select: bool,
    close: bool,
    pin: bool,
    /// Where a primary press landed on the tab's body (a drag may start there).
    press: Option<egui::Pos2>,
    /// Where a right press landed.
    popup: Option<egui::Pos2>,
    performed: Option<String>,
}

/// Draws one tab in `rect` (the header's height) and reports what the user did to it.
///
/// The press selects (IntelliJ selects on press, not on release), a middle or Shift press
/// closes, a right press asks for the context menu; a press on the tab's own buttons does
/// none of these. A freshly shown close button of an unselected tab ignores clicks for
/// [`motion::TAB_CLOSE_DEADZONE`] after the pointer entered the tab.
fn draw_tab(
    ui: &mut Ui,
    env: &TabEnv<'_>,
    rect: Rect,
    item: &TabInfo,
    galley: std::sync::Arc<egui::Galley>,
    selected: bool,
) -> TabOutcome {
    let colors = env.colors;
    let mut out = TabOutcome::default();
    // Presses and drags are tracked by the strip itself (the scroll bar's thumb, drawn
    // before the tabs, must keep its drags).
    let sense = if item.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, env.id, sense);
    let pill = Rect::from_center_size(rect.center(), vec2(rect.width(), tab::HEIGHT));
    crate::cursor::on_hover(&response, egui::CursorIcon::Default);
    response.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::SelectableLabel,
            item.enabled,
            selected,
            &item.label,
        )
    });
    let hovered = env.hover_allowed && item.enabled && ui.rect_contains_pointer(rect);

    // How long the pointer has been on the tab, for the close button's dead zone.
    let hover_key = env.id.with("hover-since");
    let since: Option<f64> = ui.data(|d| d.get_temp(hover_key));
    let hover_since = match (hovered, since) {
        (true, None) => {
            ui.data_mut(|d| d.insert_temp(hover_key, env.now));
            env.now
        }
        (true, Some(since)) => since,
        (false, Some(_)) => {
            ui.data_mut(|d| d.remove::<f64>(hover_key));
            env.now
        }
        (false, None) => env.now,
    };
    let waited = (env.now - hover_since) as f32;
    let armed = selected || waited >= motion::TAB_CLOSE_DEADZONE;
    if hovered && !armed {
        ui.ctx()
            .request_repaint_after_secs(motion::TAB_CLOSE_DEADZONE - waited);
    }

    // The tab's buttons, from the right: close or pin, then its actions.
    let show_buttons = item.enabled && (selected || hovered || env.dragged);
    let has_button = item.closable || item.pinned;
    let button_center = pos2(
        pill.right() - tab::PADDING_RIGHT - tab::CLOSE / 2.0,
        pill.center().y,
    );
    let button_rect = Rect::from_center_size(
        button_center,
        vec2(2.0 * tab::CLOSE_HOVER_RADIUS, 2.0 * tab::CLOSE_HOVER_RADIUS),
    );
    let actions_right = if has_button {
        pill.right() - tab::PADDING_RIGHT - tab::CLOSE - tab::CLOSE_GAP
    } else {
        pill.right() - tab::PADDING_RIGHT
    };
    let actions_width = if env.actions > 0 {
        env.actions as f32 * (size::ICON_BUTTON_SMALL + tab::ACTION_GAP) - tab::ACTION_GAP
    } else {
        0.0
    };
    let actions_rect = Rect::from_min_max(
        pos2(
            actions_right - actions_width,
            pill.center().y - size::ICON_BUTTON_SMALL / 2.0,
        ),
        pos2(
            actions_right,
            pill.center().y + size::ICON_BUTTON_SMALL / 2.0,
        ),
    );

    // Input: presses on the tab's body (not on its buttons).
    // The modifiers are the ones the press event carries.
    let pressed = |button: egui::PointerButton| {
        ui.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::PointerButton {
                    button: pressed,
                    pressed: true,
                    modifiers,
                    ..
                } if *pressed == button => Some(*modifiers),
                _ => None,
            })
        })
    };
    let (primary, middle, secondary) = (
        pressed(egui::PointerButton::Primary),
        pressed(egui::PointerButton::Middle),
        pressed(egui::PointerButton::Secondary),
    );
    let pos = ui.input(|i| i.pointer.interact_pos());
    let on_buttons = show_buttons
        && pos.is_some_and(|pos| {
            (has_button && button_rect.contains(pos))
                || (env.actions > 0 && actions_rect.contains(pos))
        });
    if hovered && !on_buttons {
        let plain = |m: egui::Modifiers| !m.command && !m.ctrl && !m.mac_cmd && !m.alt;
        if middle.is_some() || primary.is_some_and(|m| m.shift && plain(m)) {
            // The close click: closes, never selects.
            out.close = item.closable && !item.pinned;
        } else if primary.is_some_and(|m| plain(m) && !m.shift) {
            out.select = true;
            out.press = pos;
            response.request_focus();
        }
        if secondary.is_some() {
            out.popup = pos;
        }
    }
    if response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space))
    {
        out.select = true;
    }
    if !ui.is_rect_visible(rect) {
        return out;
    }

    let painter = ui.painter().clone();
    let corner = CornerRadius::same(radius::TAB);
    let lifted = selected || env.dragged;
    if lifted {
        // The selected tab: a tinted pill with a 1 pt border.
        painter.rect_filled(pill, corner, colors.tab_selected);
        painter.rect_stroke(
            pill,
            corner,
            Stroke::new(1.0_f32, colors.tab_selected_border),
            StrokeKind::Inside,
        );
    } else if hovered {
        painter.rect_filled(pill, corner, colors.hover);
    }
    if response.has_focus() {
        crate::parts::focus_ring(&painter, pill, radius::TAB, colors);
    }
    let mut x = pill.left() + tab::PADDING_LEFT;
    if let Some(icon) = item.icon {
        let icon_rect = Rect::from_min_size(
            pos2(x, pill.center().y - size::ICON / 2.0),
            vec2(size::ICON, size::ICON),
        );
        icons::paint(&painter, icon_rect, icon, colors.icon);
        x += size::ICON + size::ICON_TEXT_GAP;
    }
    // Hover brings an unselected title up to full strength, as in Android Studio.
    let text_color = if !item.enabled {
        colors.text_disabled
    } else if selected || hovered {
        colors.text
    } else {
        colors.tab_text
    };
    // The title is cut off with an ellipsis where the tab is at its widest.
    let dot = if item.modified { MODIFIED_DOT } else { 0.0 };
    let reserved = trailing_width(item, env.actions);
    let room = (pill.right() - reserved - x - dot).max(0.0);
    let elided = galley.size().x > room + 0.5;
    let galley = if elided {
        common::galley_elided(ui, &item.label, typography::body(typography::BODY), room)
    } else {
        galley
    };
    let text_width = galley.size().x;
    common::text_left(&painter, pill, x, galley, text_color);
    x += text_width;
    if item.modified {
        painter.circle_filled(
            pos2(x + MODIFIED_DOT / 2.0 + 1.0, pill.center().y),
            3.0,
            colors.text_secondary,
        );
    }
    if elided {
        crate::ide::HelpTooltip::new(item.label.clone()).show_for(&response);
    }

    if show_buttons
        && env.actions > 0
        && let Some(group) = &item.actions
    {
        let mut buttons = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(env.id.with("actions"))
                .max_rect(actions_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        buttons.spacing_mut().item_spacing.x = tab::ACTION_GAP;
        out.performed = crate::wm::action_rows::show_action_buttons(
            &mut buttons,
            env.id.with("actions"),
            group.as_ref(),
            env.data_context,
            &ActionPlace::EDITOR_TAB,
            crate::wm::action_rows::ButtonSize::Small,
            crate::wm::action_rows::ButtonOptions {
                perform: armed,
                separators: false,
                ..crate::wm::action_rows::ButtonOptions::default()
            },
        );
    }
    // The pin of a pinned tab is always there; the close button shows on the selected tab
    // and on hover.
    if has_button && (item.pinned || show_buttons) {
        let close = ui.interact(button_rect, response.id.with("close"), Sense::click());
        crate::cursor::on_hover_action(&close);
        let name = if item.pinned {
            format!("Unpin {}", item.label)
        } else {
            format!("Close {}", item.label)
        };
        close.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name.clone()));
        // A round hover behind the cross, lighter than the tab's.
        if close.hovered() {
            painter.circle_filled(button_center, tab::CLOSE_HOVER_RADIUS, colors.hover);
        }
        let glyph = Rect::from_center_size(button_center, vec2(tab::CLOSE_GLYPH, tab::CLOSE_GLYPH));
        icons::paint(
            &painter,
            glyph,
            if item.pinned {
                Glyph::Pin
            } else {
                Glyph::Close
            },
            if close.hovered() {
                colors.text_secondary
            } else {
                colors.text_placeholder
            },
        );
        if close.clicked() && armed {
            if item.pinned {
                out.pin = true;
            } else {
                out.close = true;
            }
        }
    }
    out
}

/// A gradient from `color` to transparent over `x0..x1` of `strip`: opaque at the left end
/// when `opaque_left`, else at the right end.
fn fade(painter: &egui::Painter, x0: f32, x1: f32, strip: Rect, color: Color32, opaque_left: bool) {
    let rect = Rect::from_x_y_ranges(x0..=x1, strip.y_range());
    let (left, right) = if opaque_left {
        (color, Color32::TRANSPARENT)
    } else {
        (Color32::TRANSPARENT, color)
    };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}
