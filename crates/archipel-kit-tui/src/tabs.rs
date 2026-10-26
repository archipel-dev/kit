//! [`TabsControl`]: the interactive tabufline (IntelliJ `JBTabs`, spec section 2).
//!
//! - A left press on a tab selects it at once (the press, not the release).
//! - A middle click, or Shift+click, closes it without selecting it. The close icon also
//!   closes it, but only 150 ms after the pointer entered the tab, so a pointer that lands on
//!   it does not close it by accident.
//! - A right press asks for the tab's context menu.
//! - Dragging a tab moves it: once the pointer left the dead zone, the tab swaps places with
//!   its neighbor whenever the pointer crosses the neighbor's center.
//! - The wheel scrolls a strip that is wider than its area.
//! - With keyboard focus, <kbd>Left</kbd> and <kbd>Right</kbd> select the previous or next
//!   tab; the `NextTab` and `PreviousTab` actions (Alt+Right, Alt+Left) do it from anywhere
//!   ([`TabsState::select_relative`]).

use crate::components::{JBTabs, TabInfo};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use archipel_kit::tokens::motion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// Cells the pointer must move before a pressed tab starts to drag (IntelliJ: 7 px).
pub const DRAG_DEAD_ZONE: u16 = 2;

/// What the tabs did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TabsEvent {
    /// Nothing for the caller.
    None,
    /// A tab was selected.
    Selected(usize),
    /// A tab asks to be closed (the caller removes it, after asking about unsaved changes).
    CloseRequested(usize),
    /// A tab was dragged from index `from` to `to`; the tabs vector is already reordered.
    Reordered {
        /// The old index.
        from: usize,
        /// The new index.
        to: usize,
    },
    /// A context menu was requested for the tab.
    ContextMenu {
        /// The tab.
        index: usize,
        /// The column.
        x: u16,
        /// The row.
        y: u16,
    },
}

/// The selection, scroll and drag state of a tab strip.
#[derive(Debug, Clone, Default)]
pub struct TabsState {
    /// The selected tab.
    pub selected: usize,
    offset: usize,
    hover: Option<usize>,
    hover_since: f64,
    drag: Option<(usize, u16, bool)>,
    manual_scroll: bool,
}

impl TabsState {
    /// A strip with tab `selected`.
    pub fn new(selected: usize) -> Self {
        Self {
            selected,
            ..Self::default()
        }
    }

    /// Selects the tab `delta` away (wrapping), as the `NextTab` and `PreviousTab` actions
    /// do; returns the new index.
    pub fn select_relative(&mut self, len: usize, delta: isize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let next = (self.selected as isize + delta).rem_euclid(len as isize) as usize;
        self.selected = next;
        self.manual_scroll = false;
        Some(next)
    }

    /// Keeps the selection valid after the tab at `index` was removed.
    pub fn removed(&mut self, index: usize, len_after: usize) {
        if len_after == 0 {
            self.selected = 0;
        } else if self.selected > index || self.selected >= len_after {
            self.selected = self.selected.saturating_sub(1).min(len_after - 1);
        }
        self.manual_scroll = false;
    }

    /// The index of the first visible tab.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

/// The interactive tabufline.
#[derive(Debug, Clone)]
pub struct TabsControl {
    /// The identity; tabs use child ids.
    pub id: Id,
    /// Whether tabs can be reordered by dragging.
    pub reorderable: bool,
    area: Rect,
    layout: crate::components::TabsLayout,
}

impl TabsControl {
    /// A strip.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            reorderable: true,
            area: Rect::default(),
            layout: crate::components::TabsLayout::default(),
        }
    }

    fn tab_id(&self, index: usize) -> Id {
        self.id.with(index as u64)
    }

    fn close_id(&self, index: usize) -> Id {
        self.id.with(0x1000 + index as u64)
    }

    /// Draws the strip in `area` (one row) on `fill` and registers it.
    pub fn show(
        &mut self,
        rt: &mut Runtime,
        state: &mut TabsState,
        tabs: &[TabInfo],
        area: Rect,
        fill: ratatui::style::Color,
        buf: &mut Buffer,
    ) {
        self.area = area;
        if !state.manual_scroll {
            state.offset =
                JBTabs::scroll_to_selected(tabs, state.selected, state.offset, area.width);
        }
        state.offset = state.offset.min(tabs.len().saturating_sub(1));
        let hovered = rt.hovered();
        let hover_tab = (0..tabs.len())
            .find(|i| hovered == Some(self.tab_id(*i)) || hovered == Some(self.close_id(*i)));
        if hover_tab != state.hover {
            state.hover = hover_tab;
            state.hover_since = rt.now;
        }
        let strip = JBTabs::new(&rt.look, tabs, state.selected)
            .offset(state.offset)
            .hovered(hover_tab)
            .fill(fill)
            .focused(rt.focused(self.id));
        self.layout = strip.layout(area);
        strip.render(area, buf);
        rt.hits.add(area, self.id);
        rt.focus.register(self.id);
        for (i, rect) in &self.layout.tabs {
            rt.hits.add(*rect, self.tab_id(*i));
        }
        for (i, rect) in &self.layout.closes {
            rt.hits.add(*rect, self.close_id(*i));
        }
    }

    /// Applies `event`. `tabs` is reordered in place when a drag swaps tabs.
    pub fn handle(
        &mut self,
        rt: &mut Runtime,
        state: &mut TabsState,
        tabs: &mut [TabInfo],
        event: &Event,
    ) -> TabsEvent {
        match event {
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && rt.focused(self.id)
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let by = match key.code {
                    KeyCode::Left => -1,
                    KeyCode::Right => 1,
                    _ => return TabsEvent::None,
                };
                let next = (state.selected as isize + by).clamp(0, tabs.len() as isize - 1);
                if tabs.is_empty() || next as usize == state.selected {
                    return TabsEvent::None;
                }
                state.selected = next as usize;
                state.manual_scroll = false;
                TabsEvent::Selected(state.selected)
            }
            Event::Mouse(_) => self.pointer(rt, state, tabs),
            _ => TabsEvent::None,
        }
    }

    fn tab_at(&self, rt: &Runtime, p: &crate::pointer::Pointer) -> Option<(usize, bool)> {
        for (i, _) in &self.layout.tabs {
            if p.target == Some(self.close_id(*i)) {
                return Some((*i, true));
            }
            if p.target == Some(self.tab_id(*i)) {
                return Some((*i, false));
            }
        }
        let _ = rt;
        None
    }

    fn pointer(
        &mut self,
        rt: &mut Runtime,
        state: &mut TabsState,
        tabs: &mut [TabInfo],
    ) -> TabsEvent {
        let Some(p) = rt.pointer().copied() else {
            return TabsEvent::None;
        };
        match p.kind {
            PointerKind::Down(MouseButton::Left) => {
                let Some((index, on_close)) = self.tab_at(rt, &p) else {
                    return TabsEvent::None;
                };
                if p.shift() {
                    // The close click: Shift+click closes without selecting.
                    return if tabs[index].can_close() {
                        TabsEvent::CloseRequested(index)
                    } else {
                        TabsEvent::None
                    };
                }
                if on_close {
                    // The close icon acts on release; the tab is not selected.
                    return TabsEvent::None;
                }
                rt.focus.focus(self.id);
                state.drag = Some((index, p.x, false));
                if state.selected != index {
                    state.selected = index;
                    state.manual_scroll = false;
                    return TabsEvent::Selected(index);
                }
                TabsEvent::None
            }
            PointerKind::Down(MouseButton::Middle) => match self.tab_at(rt, &p) {
                Some((index, _)) if tabs[index].can_close() => TabsEvent::CloseRequested(index),
                _ => TabsEvent::None,
            },
            PointerKind::Down(MouseButton::Right) => match self.tab_at(rt, &p) {
                Some((index, _)) => TabsEvent::ContextMenu {
                    index,
                    x: p.x,
                    y: p.y,
                },
                None => TabsEvent::None,
            },
            PointerKind::Up(MouseButton::Left) => {
                state.drag = None;
                let deadzone = f64::from(motion::TAB_CLOSE_DEADZONE);
                for (i, _) in &self.layout.closes {
                    if p.clicked(self.close_id(*i)) && tabs[*i].can_close() {
                        // Ignore a close click right after the pointer landed on the tab.
                        if state.hover == Some(*i) && rt.now - state.hover_since < deadzone {
                            return TabsEvent::None;
                        }
                        return TabsEvent::CloseRequested(*i);
                    }
                }
                TabsEvent::None
            }
            PointerKind::Drag(MouseButton::Left) if self.reorderable => {
                let Some((index, press_x, active)) = state.drag else {
                    return TabsEvent::None;
                };
                if !active && p.x.abs_diff(press_x) < DRAG_DEAD_ZONE {
                    return TabsEvent::None;
                }
                state.drag = Some((index, press_x, true));
                // Swap with a neighbor when the pointer crosses its center.
                let rects = &self.layout.tabs;
                let target = rects.iter().find(|(i, r)| {
                    let center = r.x + r.width / 2;
                    *i != index
                        && r.x <= p.x
                        && p.x < r.right()
                        && ((*i > index && p.x >= center) || (*i < index && p.x <= center))
                });
                let Some(&(to, _)) = target else {
                    return TabsEvent::None;
                };
                if tabs[to].pinned != tabs[index].pinned {
                    return TabsEvent::None;
                }
                tabs.swap(index, to);
                if state.selected == index {
                    state.selected = to;
                } else if state.selected == to {
                    state.selected = index;
                }
                state.drag = Some((to, press_x, true));
                TabsEvent::Reordered { from: index, to }
            }
            PointerKind::ScrollUp | PointerKind::ScrollDown
                if p.over(self.id) || self.tab_at(rt, &p).is_some() =>
            {
                let max = tabs.len().saturating_sub(1);
                state.offset = (state.offset as i32 + p.wheel()).clamp(0, max as i32) as usize;
                state.manual_scroll = true;
                TabsEvent::None
            }
            _ => TabsEvent::None,
        }
    }
}
