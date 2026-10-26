//! Editor tabs.

use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::theme::Theme;
use crate::tokens::{motion, popup, radius, size, tab, typography};
use egui::{
    Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// One tab of a [`JBTabs`].
#[derive(Debug, Clone)]
pub struct TabInfo {
    label: String,
    icon: Option<Icon>,
    closable: bool,
    modified: bool,
}

impl TabInfo {
    /// A tab titled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            closable: false,
            modified: false,
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
        }
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

        // Each tab's width, and where it starts in the scrolled content (the strip's left
        // edge at offset 0).
        let galleys: Vec<_> = self
            .tabs
            .iter()
            .map(|item| common::galley(ui, item.label.as_str(), typography::body(typography::BODY)))
            .collect();
        let widths: Vec<f32> = self
            .tabs
            .iter()
            .zip(&galleys)
            .map(|(item, galley)| tab_width(item, galley.size().x))
            .collect();
        let mut starts = Vec::with_capacity(count);
        let mut x = tab::STRIP_INSET;
        for width in &widths {
            starts.push(x);
            x += width + tab::GAP;
        }
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
            let (left, right) = (starts[index], starts[index] + widths[index]);
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

        // The tabs, clipped to the scrolling area.
        let mut tabs_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id.with("tabs"))
                .max_rect(viewport),
        );
        tabs_ui.set_clip_rect(viewport.intersect(ui.clip_rect()));
        for (index, (item, galley)) in self.tabs.iter().zip(galleys.iter().cloned()).enumerate() {
            let rect = Rect::from_min_size(
                pos2(strip.left() + starts[index] - state.offset, strip.top()),
                vec2(widths[index], size::HEADER_HEIGHT),
            );
            let selected = index == self.selected;
            if let Some(action) = draw_tab(
                &mut tabs_ui,
                tab_id(index),
                rect,
                item,
                galley,
                selected,
                &colors,
            ) {
                match action {
                    TabAction::Select => out.selected = Some(index),
                    TabAction::Close => {
                        out.closed = Some(index);
                        out.selected = None;
                    }
                }
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
                        starts[index] < state.offset - 0.5
                            || starts[index] + widths[index] > state.offset + view + 0.5
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

/// The width of `item`'s pill, whose title is `text` wide.
fn tab_width(item: &TabInfo, text: f32) -> f32 {
    let icon = if item.icon.is_some() {
        size::ICON + size::ICON_TEXT_GAP
    } else {
        0.0
    };
    let dot = if item.modified { MODIFIED_DOT } else { 0.0 };
    // The close button's room is kept on every closable tab, so showing it on hover does
    // not move the tabs (as in Android Studio).
    let end = if item.closable {
        tab::CLOSE_GAP + tab::CLOSE + tab::PADDING_RIGHT
    } else {
        tab::PADDING_LEFT
    };
    tab::PADDING_LEFT + icon + text + dot + end
}

/// Room for the unsaved-changes dot after a title.
const MODIFIED_DOT: f32 = 10.0;

/// What the user did to one tab.
enum TabAction {
    Select,
    Close,
}

/// Draws one tab in `rect` (the header's height) and reports a click on it or its close
/// button.
fn draw_tab(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    item: &TabInfo,
    galley: std::sync::Arc<egui::Galley>,
    selected: bool,
    colors: &crate::theme::Colors,
) -> Option<TabAction> {
    let mut action = None;
    let response = ui.interact(rect, id, Sense::click());
    let pill = Rect::from_center_size(rect.center(), vec2(rect.width(), tab::HEIGHT));
    crate::cursor::on_hover(&response, egui::CursorIcon::Default);
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &item.label)
    });
    if response.clicked() && !selected {
        action = Some(TabAction::Select);
    }
    if response.middle_clicked() && item.closable {
        action = Some(TabAction::Close);
    }
    if !ui.is_rect_visible(rect) {
        return action;
    }
    let painter = ui.painter().clone();
    let corner = CornerRadius::same(radius::TAB);
    if selected {
        // The selected tab: a tinted pill with a 1 pt border.
        painter.rect_filled(pill, corner, colors.tab_selected);
        painter.rect_stroke(
            pill,
            corner,
            Stroke::new(1.0_f32, colors.tab_selected_border),
            StrokeKind::Inside,
        );
    } else if response.hovered() {
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
    let text_color = if selected || response.hovered() {
        colors.text
    } else {
        colors.tab_text
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
    if item.closable && (selected || response.hovered()) {
        let center = pos2(
            pill.right() - tab::PADDING_RIGHT - tab::CLOSE / 2.0,
            pill.center().y,
        );
        let close_rect = Rect::from_center_size(
            center,
            vec2(2.0 * tab::CLOSE_HOVER_RADIUS, 2.0 * tab::CLOSE_HOVER_RADIUS),
        );
        let close = ui.interact(close_rect, response.id.with("close"), Sense::click());
        crate::cursor::on_hover_action(&close);
        close.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Button, true, format!("Close {}", item.label))
        });
        // A round hover behind the cross, lighter than the tab's.
        if close.hovered() {
            painter.circle_filled(center, tab::CLOSE_HOVER_RADIUS, colors.hover);
        }
        let glyph = Rect::from_center_size(center, vec2(tab::CLOSE_GLYPH, tab::CLOSE_GLYPH));
        icons::paint(
            &painter,
            glyph,
            Glyph::Close,
            if close.hovered() {
                colors.text_secondary
            } else {
                colors.text_placeholder
            },
        );
        if close.clicked() {
            action = Some(TabAction::Close);
        }
    }
    action
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
