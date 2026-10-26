//! Lists, trees and tables with the GUI kit's selection model and speed search.
//!
//! [`JBList`], [`JBTree`] and [`JBTable`] share one engine, [`CollectionState`]: it holds the
//! kit's [`ListSelectionModel`] and [`SpeedSearchState`] (the same types the GUI uses) and
//! implements the keyboard and mouse rules of IntelliJ lists:
//!
//! - Up/Down move (no wrap), Shift extends, Ctrl moves the lead without selecting, Ctrl+Space
//!   toggles the lead, Home/End and PageUp/PageDown jump, Ctrl+A selects all, Enter activates.
//! - A click selects, Ctrl+click toggles, Shift+click extends, dragging extends, a double click
//!   activates (expands or collapses a tree node), the wheel scrolls three rows, and a right
//!   click selects the row under the pointer unless it is already part of the selection, then
//!   asks for a context menu.
//! - In a tree, Right expands a collapsed node or selects its first child, Left collapses an
//!   expanded node or selects its parent.
//! - Typing starts the speed search (letters, digits and `*_-+"'/.#$>: ,;?!@%^&`); it matches
//!   by camel humps and word starts ([`SpeedSearchComparator`]), highlights matches, moves the
//!   selection to the first match, Up/Down/Home/End then move between matches, Backspace edits,
//!   Ctrl+Backspace clears, Esc closes, and Enter, Left, Right and PageUp/PageDown close it
//!   and act on the list.

use crate::components::{ColoredListCellRenderer, RowIcon, fill, put, text_width};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use archipel_kit::ui::{
    ListSelectionModel, SelectionMode, SortOrder, SpeedSearchComparator, SpeedSearchState,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::collections::BTreeSet;

/// The characters (besides letters and digits) that start a speed search.
pub const SPEED_SEARCH_START: &str = "*_-+\"'/.#$>:,;?!@%^&";

/// One row of a list or tree.
#[derive(Debug, Clone, Default)]
pub struct RowData {
    /// The label (what the speed search matches).
    pub label: String,
    /// Dimmed text after the label.
    pub secondary: Option<String>,
    /// The icon.
    pub icon: Option<RowIcon>,
    /// The tree depth.
    pub depth: u16,
    /// `Some` for tree parents: expanded or not.
    pub expanded: Option<bool>,
    /// A dimmed row.
    pub muted: bool,
}

impl RowData {
    /// A row with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    /// Sets the icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<RowIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the secondary text.
    #[must_use]
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Sets the tree depth.
    #[must_use]
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Marks it as a parent, expanded or not.
    #[must_use]
    pub fn parent(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// Dims the row.
    #[must_use]
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

/// Where a [`CollectionState`] reads rows from.
pub trait RowSource {
    /// The number of rows.
    fn len(&self) -> usize;
    /// Whether there are no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The text of row `i` for the speed search.
    fn text(&self, i: usize) -> String;
    /// The tree depth of row `i`.
    fn depth(&self, _i: usize) -> u16 {
        0
    }
    /// `Some(expanded)` when row `i` is a tree parent.
    fn expandable(&self, _i: usize) -> Option<bool> {
        None
    }
}

/// [`RowSource`] over a slice of [`RowData`].
pub struct Rows<'a>(pub &'a [RowData]);

impl RowSource for Rows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn text(&self, i: usize) -> String {
        self.0.get(i).map(|r| r.label.clone()).unwrap_or_default()
    }

    fn depth(&self, i: usize) -> u16 {
        self.0.get(i).map_or(0, |r| r.depth)
    }

    fn expandable(&self, i: usize) -> Option<bool> {
        self.0.get(i).and_then(|r| r.expanded)
    }
}

/// What a collection did with an event.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectionEvent {
    /// Nothing for the caller.
    None,
    /// The event was used (movement, scrolling, speed search typing) but the selection is as
    /// it was.
    Consumed,
    /// The selection changed.
    Selected,
    /// Enter or a double click on a row.
    Activated(usize),
    /// A tree parent should expand.
    Expand(usize),
    /// A tree parent should collapse.
    Collapse(usize),
    /// A context menu was requested at a cell; `row` is the row under it, if any.
    ContextMenu {
        /// The row.
        row: Option<usize>,
        /// The column of the pointer.
        x: u16,
        /// The row of the pointer.
        y: u16,
    },
}

/// The view state shared by lists, trees and tables.
#[derive(Debug, Clone)]
pub struct CollectionState {
    /// The selected rows (the kit's model: anchor, lead, intervals).
    pub selection: ListSelectionModel,
    /// The speed search.
    pub speed: SpeedSearchState,
    /// Whether the speed search hides the rows that do not match (a popup list) instead of
    /// only moving the selection.
    pub filter: bool,
    /// Whether typing starts a speed search.
    pub speed_search: bool,
    scroll: usize,
    reveal: bool,
    page: usize,
    hover: Option<usize>,
    area: Rect,
    drag_anchor: Option<usize>,
}

impl Default for CollectionState {
    fn default() -> Self {
        Self::new(ListSelectionModel::multiple())
    }
}

impl CollectionState {
    /// A view with `selection`.
    pub fn new(selection: ListSelectionModel) -> Self {
        Self {
            selection,
            speed: SpeedSearchState::new(),
            filter: false,
            speed_search: true,
            scroll: 0,
            reveal: false,
            page: 10,
            hover: None,
            area: Rect::default(),
            drag_anchor: None,
        }
    }

    /// A single-selection view.
    pub fn single() -> Self {
        Self::new(ListSelectionModel::single())
    }

    /// The first visible row index (in the visible rows, not data rows).
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// The row under the pointer.
    pub fn hovered(&self) -> Option<usize> {
        self.hover
    }

    /// The selected row, or the lead.
    pub fn current(&self) -> Option<usize> {
        self.selection
            .lead()
            .filter(|l| self.selection.is_selected(*l))
            .or(self.selection.first())
    }

    /// The rows shown: all, or only the matches while a filtering speed search is open.
    pub fn visible(&self, rows: &dyn RowSource) -> Vec<usize> {
        if self.filter && !self.speed.text().is_empty() {
            self.matches(rows)
        } else {
            (0..rows.len()).collect()
        }
    }

    /// The rows matching the speed search text (all rows when it is empty).
    pub fn matches(&self, rows: &dyn RowSource) -> Vec<usize> {
        let comparator = SpeedSearchComparator::new(self.speed.text());
        (0..rows.len())
            .filter(|i| comparator.matches(&rows.text(*i)))
            .collect()
    }

    fn set_lead(&mut self, to: usize, extend: bool) -> bool {
        let changed = if extend && self.selection.mode() != SelectionMode::Single {
            let anchor = self.selection.anchor().unwrap_or(to);
            self.selection.set_selection_interval(anchor, to)
        } else {
            self.selection.select(to)
        };
        self.reveal = true;
        changed
    }

    fn select_first_match(&mut self, rows: &dyn RowSource) -> bool {
        let comparator = SpeedSearchComparator::new(self.speed.text());
        let first = (0..rows.len()).find(|i| comparator.matches(&rows.text(*i)));
        match first {
            Some(i) => self.set_lead(i, false),
            None => false,
        }
    }

    /// Applies a key event for the collection `rows`. `tree` enables Left and Right as tree
    /// moves.
    pub fn key(&mut self, key: &KeyEvent, rows: &dyn RowSource, tree: bool) -> CollectionEvent {
        if key.kind == KeyEventKind::Release {
            return CollectionEvent::None;
        }
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let len = rows.len();
        let searching = self.speed.is_active();
        // Speed search first.
        if self.speed_search {
            match key.code {
                KeyCode::Esc if searching => {
                    self.speed.reset();
                    return CollectionEvent::Consumed;
                }
                KeyCode::Backspace if searching => {
                    if ctrl {
                        self.speed.clear();
                    } else {
                        let mut text = self.speed.text().to_owned();
                        text.pop();
                        self.speed.activate(text);
                    }
                    let changed = self.select_first_match(rows);
                    return if changed {
                        CollectionEvent::Selected
                    } else {
                        CollectionEvent::Consumed
                    };
                }
                KeyCode::Char(c)
                    if !ctrl
                        && !alt
                        && !key.modifiers.contains(KeyModifiers::SUPER)
                        && !c.is_control()
                        && (c.is_alphanumeric()
                            || SPEED_SEARCH_START.contains(c)
                            || (c == ' ' && searching)) =>
                {
                    let mut text = self.speed.text().to_owned();
                    text.push(c);
                    self.speed.activate(text);
                    let changed = self.select_first_match(rows);
                    return if changed {
                        CollectionEvent::Selected
                    } else {
                        CollectionEvent::Consumed
                    };
                }
                KeyCode::Enter
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown
                    if searching =>
                {
                    // These close the search and then act on the list.
                    self.speed.reset();
                }
                _ => {}
            }
        }
        if len == 0 {
            return CollectionEvent::None;
        }
        let searching = self.speed.is_active() && !self.speed.text().is_empty();
        let visible = if searching {
            self.matches(rows)
        } else {
            self.visible(rows)
        };
        if visible.is_empty() {
            return CollectionEvent::Consumed;
        }
        let lead = self.selection.lead().or(self.selection.first());
        let pos = lead.and_then(|l| visible.iter().position(|v| *v == l));
        let step = |by: isize| -> usize {
            match pos {
                Some(p) => visible[(p as isize + by).clamp(0, visible.len() as isize - 1) as usize],
                None if by >= 0 => visible[0],
                None => visible[visible.len() - 1],
            }
        };
        let to = match key.code {
            KeyCode::Up => Some(step(-1)),
            KeyCode::Down => Some(step(1)),
            KeyCode::Home => Some(visible[0]),
            KeyCode::End => Some(visible[visible.len() - 1]),
            KeyCode::PageUp => Some(step(-(self.page.max(2) as isize - 1))),
            KeyCode::PageDown => Some(step(self.page.max(2) as isize - 1)),
            _ => None,
        };
        if let Some(to) = to {
            if ctrl && matches!(key.code, KeyCode::Up | KeyCode::Down) {
                self.selection.set_lead(to);
                self.reveal = true;
                return CollectionEvent::Consumed;
            }
            return if self.set_lead(to, shift) {
                CollectionEvent::Selected
            } else {
                CollectionEvent::Consumed
            };
        }
        match key.code {
            KeyCode::Char(' ') if ctrl => {
                if let Some(l) = lead {
                    self.selection.toggle(l);
                    return CollectionEvent::Selected;
                }
                CollectionEvent::None
            }
            KeyCode::Char('a') | KeyCode::Char('A')
                if ctrl && self.selection.mode() == SelectionMode::MultipleInterval =>
            {
                self.selection.select_all(len);
                CollectionEvent::Selected
            }
            KeyCode::Enter => match self.current() {
                Some(row) if tree && rows.expandable(row).is_some() => {
                    if rows.expandable(row) == Some(true) {
                        CollectionEvent::Collapse(row)
                    } else {
                        CollectionEvent::Expand(row)
                    }
                }
                Some(row) => CollectionEvent::Activated(row),
                None => CollectionEvent::None,
            },
            KeyCode::Right if tree => self.tree_right(rows),
            KeyCode::Left if tree => self.tree_left(rows),
            _ => CollectionEvent::None,
        }
    }

    fn tree_right(&mut self, rows: &dyn RowSource) -> CollectionEvent {
        let Some(row) = self.current() else {
            return CollectionEvent::None;
        };
        match rows.expandable(row) {
            Some(false) => CollectionEvent::Expand(row),
            Some(true) => {
                let next = row + 1;
                if next < rows.len() && rows.depth(next) > rows.depth(row) {
                    self.set_lead(next, false);
                    CollectionEvent::Selected
                } else {
                    CollectionEvent::Consumed
                }
            }
            None => CollectionEvent::Consumed,
        }
    }

    fn tree_left(&mut self, rows: &dyn RowSource) -> CollectionEvent {
        let Some(row) = self.current() else {
            return CollectionEvent::None;
        };
        if rows.expandable(row) == Some(true) {
            return CollectionEvent::Collapse(row);
        }
        let depth = rows.depth(row);
        match (0..row).rev().find(|i| rows.depth(*i) < depth) {
            Some(parent) => {
                self.set_lead(parent, false);
                CollectionEvent::Selected
            }
            None => CollectionEvent::Consumed,
        }
    }

    /// The row index under cell row `y` of the last drawn area.
    fn row_at(&self, y: u16, visible: &[usize]) -> Option<usize> {
        if y < self.area.y || y >= self.area.bottom() {
            return None;
        }
        visible
            .get(self.scroll + (y - self.area.y) as usize)
            .copied()
    }

    /// Applies the current mouse event of `rt`, if it is for the collection `id`. `tree`
    /// makes a click on the chevron or a double click toggle a node.
    pub fn pointer(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        rows: &dyn RowSource,
        tree: bool,
    ) -> CollectionEvent {
        let Some(p) = rt.pointer().copied() else {
            return CollectionEvent::None;
        };
        let visible = self.visible(rows);
        let multi = self.selection.mode() != SelectionMode::Single;
        match p.kind {
            PointerKind::Move => {
                self.hover = if p.over(id) {
                    self.row_at(p.y, &visible)
                } else {
                    None
                };
                CollectionEvent::None
            }
            PointerKind::ScrollUp | PointerKind::ScrollDown if p.over(id) => {
                let max = visible.len().saturating_sub(self.area.height as usize);
                let by = p.wheel() * 3;
                self.scroll = (self.scroll as i32 + by).clamp(0, max as i32) as usize;
                CollectionEvent::Consumed
            }
            PointerKind::Down(MouseButton::Left) if p.over(id) => {
                rt.focus.focus(id);
                let Some(row) = self.row_at(p.y, &visible) else {
                    return CollectionEvent::Consumed;
                };
                self.hover = Some(row);
                // A click on a tree chevron toggles the node.
                if tree && let Some(expanded) = rows.expandable(row) {
                    let chevron = self.area.x + 1 + rows.depth(row) * 2;
                    if p.x >= chevron && p.x < chevron + 2 {
                        self.selection.select(row);
                        return if expanded {
                            CollectionEvent::Collapse(row)
                        } else {
                            CollectionEvent::Expand(row)
                        };
                    }
                }
                if p.clicks >= 2 {
                    self.selection.select(row);
                    return match (tree, rows.expandable(row)) {
                        (true, Some(true)) => CollectionEvent::Collapse(row),
                        (true, Some(false)) => CollectionEvent::Expand(row),
                        _ => CollectionEvent::Activated(row),
                    };
                }
                self.drag_anchor = Some(row);
                if p.command() && multi {
                    self.selection.toggle(row);
                } else if p.shift() && multi {
                    let anchor = self.selection.anchor().unwrap_or(row);
                    self.selection.set_selection_interval(anchor, row);
                } else {
                    self.selection.select(row);
                }
                CollectionEvent::Selected
            }
            PointerKind::Drag(MouseButton::Left) if p.origin == Some(id) => {
                let (top, bottom) = (self.area.y, self.area.bottom());
                // Dragging past an edge scrolls.
                if p.y < top && self.scroll > 0 {
                    self.scroll -= 1;
                } else if p.y >= bottom && self.scroll + (self.area.height as usize) < visible.len()
                {
                    self.scroll += 1;
                }
                let y = p.y.clamp(top, bottom.saturating_sub(1));
                if let (Some(anchor), Some(row)) = (self.drag_anchor, self.row_at(y, &visible))
                    && multi
                {
                    self.selection.set_selection_interval(anchor, row);
                    return CollectionEvent::Selected;
                }
                CollectionEvent::Consumed
            }
            PointerKind::Up(MouseButton::Left) => {
                self.drag_anchor = None;
                CollectionEvent::None
            }
            PointerKind::Down(MouseButton::Right) if p.over(id) => {
                rt.focus.focus(id);
                let row = self.row_at(p.y, &visible);
                if let Some(row) = row
                    && !self.selection.is_selected(row)
                {
                    self.selection.select(row);
                }
                CollectionEvent::ContextMenu {
                    row,
                    x: p.x,
                    y: p.y,
                }
            }
            _ => CollectionEvent::None,
        }
    }

    /// Keeps the lead row inside the viewport (called while drawing).
    fn follow(&mut self, visible: &[usize], height: usize) {
        let max = visible.len().saturating_sub(height);
        self.scroll = self.scroll.min(max);
        if self.reveal {
            self.reveal = false;
            if let Some(lead) = self.selection.lead()
                && let Some(pos) = visible.iter().position(|v| *v == lead)
            {
                if pos < self.scroll {
                    self.scroll = pos;
                } else if pos >= self.scroll + height {
                    self.scroll = pos + 1 - height;
                }
            }
        }
    }

    /// Draws the speed search box at the top-right of `area` (IntelliJ puts it at the
    /// top-left; in a terminal the labels start at the left, so the box stays out of the way).
    fn draw_speed_search(&self, rt: &Runtime, area: Rect, buf: &mut Buffer, matched: bool) {
        if !self.speed.is_active() || area.height == 0 {
            return;
        }
        let p = &rt.look.palette;
        let text = self.speed.text();
        let width = (text_width(text) + 6).max(14).min(area.width);
        let rect = Rect::new(area.right().saturating_sub(width), area.y, width, 1);
        let bg = p.one_bg2;
        fill(
            buf,
            rect,
            rect.x,
            rect.y,
            rect.width,
            Style::default().bg(bg),
        );
        let x = put(
            buf,
            rect,
            rect.x + 1,
            rect.y,
            &rt.look.glyph(crate::Glyph::Search),
            Style::default().fg(p.light_grey).bg(bg),
        );
        let color = if matched || text.is_empty() {
            p.white
        } else {
            p.red
        };
        let shown = if text.is_empty() { "Search" } else { text };
        let color = if text.is_empty() { p.grey_fg } else { color };
        put(
            buf,
            rect,
            x + 1,
            rect.y,
            shown,
            Style::default().fg(color).bg(bg),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rows(
    state: &mut CollectionState,
    rt: &mut Runtime,
    id: Id,
    area: Rect,
    rows: &[RowData],
    focused: bool,
    on: Color,
    buf: &mut Buffer,
    guides: bool,
) {
    state.area = area;
    state.page = area.height as usize;
    rt.widget(id, area);
    let p = rt.look.palette;
    let source = Rows(rows);
    let visible = state.visible(&source);
    state.follow(&visible, area.height as usize);
    let comparator = SpeedSearchComparator::new(state.speed.text());
    let show_matches = state.speed.is_active() && !comparator.is_empty();
    for y in 0..area.height {
        let rect = Rect::new(area.x, area.y + y, area.width, 1);
        let Some(&row) = visible.get(state.scroll + y as usize) else {
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(on),
            );
            continue;
        };
        let data = &rows[row];
        let fragments = if show_matches {
            comparator
                .matching_fragments(&data.label)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut cell = ColoredListCellRenderer::new(&rt.look, &data.label)
            .depth(data.depth)
            .expanded(data.expanded)
            .selected(state.selection.is_selected(row))
            .active(focused)
            .hovered(state.hover == Some(row))
            .on(on)
            .guides(guides)
            .muted(data.muted)
            .highlights(&fragments);
        if let Some(icon) = data.icon {
            cell = cell.icon(icon);
        }
        if let Some(secondary) = &data.secondary {
            cell = cell.secondary(secondary);
        }
        ratatui::widgets::Widget::render(cell, rect, buf);
    }
    // Scroll bar thumb on the last column.
    if visible.len() > area.height as usize && area.width > 4 && area.height > 0 {
        let h = area.height as usize;
        let thumb = (h * h / visible.len()).max(1);
        let top = state.scroll * (h - thumb) / (visible.len() - h).max(1);
        for i in 0..thumb {
            put(
                buf,
                area,
                area.right() - 1,
                area.y + (top + i) as u16,
                "▐",
                Style::default().fg(p.grey).bg(on),
            );
        }
    }
    if rows.is_empty() {
        let text = "Nothing to show";
        let x = area.x + area.width.saturating_sub(text_width(text)) / 2;
        put(
            buf,
            area,
            x,
            area.y + area.height / 2,
            text,
            Style::default().fg(p.grey_fg).bg(on),
        );
    }
    let matched = !show_matches || !state.matches(&source).is_empty();
    state.draw_speed_search(rt, area, buf, matched);
}

/// A list.
#[derive(Debug, Clone, Default)]
pub struct JBList {
    /// The view state: selection, speed search, scroll.
    pub state: CollectionState,
}

impl JBList {
    /// A list with a multiple-interval selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// A list with a single selection.
    pub fn single() -> Self {
        Self {
            state: CollectionState::single(),
        }
    }

    /// Draws the list in `area` on the background `on` and registers it under `id`.
    pub fn show(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        area: Rect,
        rows: &[RowData],
        on: Color,
        buf: &mut Buffer,
    ) {
        let focused = rt.focused(id);
        draw_rows(&mut self.state, rt, id, area, rows, focused, on, buf, false);
    }

    /// Applies `event`.
    pub fn handle(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        event: &Event,
        rows: &[RowData],
    ) -> CollectionEvent {
        match event {
            Event::Key(key) if rt.focused(id) => self.state.key(key, &Rows(rows), false),
            Event::Mouse(_) => self.state.pointer(rt, id, &Rows(rows), false),
            _ => CollectionEvent::None,
        }
    }
}

/// A node of a tree model.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// A key unique among siblings (the path of keys identifies the node).
    pub key: String,
    /// The label.
    pub label: String,
    /// The icon.
    pub icon: Option<RowIcon>,
    /// Dimmed text after the label.
    pub secondary: Option<String>,
    /// The children; a node with children is a parent.
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// A leaf.
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            key: label.clone(),
            label,
            icon: None,
            secondary: None,
            children: Vec::new(),
        }
    }

    /// Sets the key.
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    /// Sets the icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<RowIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the secondary text.
    #[must_use]
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Sets the children.
    #[must_use]
    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    /// Whether the node has children.
    pub fn is_parent(&self) -> bool {
        !self.children.is_empty()
    }
}

/// What a [`JBTree`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TreeEvent {
    /// Nothing for the caller.
    None,
    /// The selection changed.
    Selected,
    /// A leaf was activated (Enter or double click): its path of keys, joined with `/`.
    Activated(String),
    /// A node was expanded or collapsed.
    Toggled(String),
    /// A context menu was requested for the node at `path` (None on empty space).
    ContextMenu {
        /// The node.
        path: Option<String>,
        /// The column.
        x: u16,
        /// The row.
        y: u16,
    },
}

/// A tree: the nodes, which of them are expanded, and the view state.
#[derive(Debug, Clone, Default)]
pub struct JBTree {
    /// The view state.
    pub state: CollectionState,
    /// The paths of the expanded nodes.
    pub expanded: BTreeSet<String>,
    flat: Vec<(String, RowData)>,
}

impl JBTree {
    /// A tree with a multiple-interval selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// The rows of `nodes` with the expanded parents opened.
    pub fn rows(&self, nodes: &[TreeNode]) -> Vec<(String, RowData)> {
        fn walk(
            out: &mut Vec<(String, RowData)>,
            nodes: &[TreeNode],
            prefix: &str,
            depth: u16,
            expanded: &BTreeSet<String>,
        ) {
            for node in nodes {
                let path = if prefix.is_empty() {
                    node.key.clone()
                } else {
                    format!("{prefix}/{}", node.key)
                };
                let open = node.is_parent() && expanded.contains(&path);
                let mut row = RowData::new(&node.label).depth(depth);
                row.icon = node.icon;
                row.secondary = node.secondary.clone();
                if node.is_parent() {
                    row.expanded = Some(open);
                }
                out.push((path.clone(), row));
                if open {
                    walk(out, &node.children, &path, depth + 1, expanded);
                }
            }
        }
        let mut out = Vec::new();
        walk(&mut out, nodes, "", 0, &self.expanded);
        out
    }

    /// Expands the node at `path` and all its ancestors.
    pub fn expand_path(&mut self, path: &str) {
        let mut at = String::new();
        for part in path.split('/') {
            if !at.is_empty() {
                at.push('/');
            }
            at.push_str(part);
            self.expanded.insert(at.clone());
        }
    }

    /// The path of the selected node (the lead), as of the last frame.
    pub fn selected_path(&self) -> Option<&str> {
        let row = self.state.current()?;
        self.flat.get(row).map(|(p, _)| p.as_str())
    }

    /// The paths of all selected nodes, as of the last frame.
    pub fn selected_paths(&self) -> Vec<String> {
        self.state
            .selection
            .iter()
            .filter_map(|i| self.flat.get(i).map(|(p, _)| p.clone()))
            .collect()
    }

    /// Selects the node at `path`, expanding its ancestors; returns whether it was found.
    pub fn select_path(&mut self, nodes: &[TreeNode], path: &str) -> bool {
        if let Some((parent, _)) = path.rsplit_once('/') {
            self.expand_path(parent);
        }
        self.flat = self.rows(nodes);
        match self.flat.iter().position(|(p, _)| p == path) {
            Some(i) => {
                self.state.selection.select(i);
                self.state.reveal = true;
                true
            }
            None => false,
        }
    }

    /// Draws the tree of `nodes` in `area` on the background `on`.
    pub fn show(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        area: Rect,
        nodes: &[TreeNode],
        on: Color,
        buf: &mut Buffer,
    ) {
        self.flat = self.rows(nodes);
        self.state.selection.clamp(self.flat.len());
        let rows: Vec<RowData> = self.flat.iter().map(|(_, r)| r.clone()).collect();
        let focused = rt.focused(id);
        draw_rows(&mut self.state, rt, id, area, &rows, focused, on, buf, true);
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, id: Id, event: &Event) -> TreeEvent {
        let rows: Vec<RowData> = self.flat.iter().map(|(_, r)| r.clone()).collect();
        let result = match event {
            Event::Key(key) if rt.focused(id) => self.state.key(key, &Rows(&rows), true),
            Event::Mouse(_) => self.state.pointer(rt, id, &Rows(&rows), true),
            _ => CollectionEvent::None,
        };
        let path = |row: usize| self.flat.get(row).map(|(p, _)| p.clone());
        match result {
            CollectionEvent::Selected => TreeEvent::Selected,
            CollectionEvent::Activated(row) => {
                path(row).map_or(TreeEvent::None, TreeEvent::Activated)
            }
            CollectionEvent::Expand(row) => match path(row) {
                Some(p) => {
                    self.expanded.insert(p.clone());
                    TreeEvent::Toggled(p)
                }
                None => TreeEvent::None,
            },
            CollectionEvent::Collapse(row) => match path(row) {
                Some(p) => {
                    self.expanded.remove(&p);
                    TreeEvent::Toggled(p)
                }
                None => TreeEvent::None,
            },
            CollectionEvent::ContextMenu { row, x, y } => TreeEvent::ContextMenu {
                path: row.and_then(path),
                x,
                y,
            },
            CollectionEvent::None | CollectionEvent::Consumed => TreeEvent::None,
        }
    }
}

/// A table column.
#[derive(Debug, Clone)]
pub struct Column {
    /// The header text.
    pub title: String,
    /// The width in cells.
    pub width: u16,
    /// Right-aligned cells.
    pub right: bool,
}

impl Column {
    /// A left-aligned column.
    pub fn new(title: impl Into<String>, width: u16) -> Self {
        Self {
            title: title.into(),
            width,
            right: false,
        }
    }

    /// Right-aligns the cells.
    #[must_use]
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }
}

struct TableRows<'a>(&'a [Vec<String>]);

impl RowSource for TableRows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn text(&self, i: usize) -> String {
        self.0.get(i).map(|r| r.join(" ")).unwrap_or_default()
    }
}

/// What a [`JBTable`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TableEvent {
    /// Nothing for the caller.
    None,
    /// The selection changed.
    Selected,
    /// Enter or a double click on a row.
    Activated(usize),
    /// A header was clicked: sort by this column in this order (the caller sorts its rows).
    Sort(usize, SortOrder),
    /// A column was resized by dragging its divider.
    Resized(usize),
    /// A context menu was requested.
    ContextMenu {
        /// The row under the pointer.
        row: Option<usize>,
        /// The column.
        x: u16,
        /// The row.
        y: u16,
    },
}

/// A table: columns, a header with sort indicators, resizable columns and the shared
/// selection and speed search.
#[derive(Debug, Clone, Default)]
pub struct JBTable {
    /// The view state.
    pub state: CollectionState,
    /// The columns.
    pub columns: Vec<Column>,
    /// The sorted column and order (the caller sorts; the header shows it).
    pub sort: Option<(usize, SortOrder)>,
    resize: Option<(usize, u16, u16)>,
    header: Rect,
}

impl JBTable {
    /// A table with `columns`.
    pub fn new(columns: Vec<Column>) -> Self {
        Self {
            columns,
            ..Self::default()
        }
    }

    fn column_rects(&self, area: Rect) -> Vec<Rect> {
        let mut x = area.x;
        self.columns
            .iter()
            .map(|c| {
                let r = Rect::new(x, area.y, c.width, 1).intersection(area);
                x = x.saturating_add(c.width);
                r
            })
            .collect()
    }

    /// Draws the header and the rows of `cells` in `area`.
    pub fn show(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        area: Rect,
        cells: &[Vec<String>],
        on: Color,
        buf: &mut Buffer,
    ) {
        if area.height == 0 {
            return;
        }
        let p = rt.look.palette;
        self.header = Rect::new(area.x, area.y, area.width, 1);
        let header_bg = p.black2;
        fill(
            buf,
            self.header,
            area.x,
            area.y,
            area.width,
            Style::default().bg(header_bg),
        );
        let rects = self.column_rects(self.header);
        for (i, (column, rect)) in self.columns.iter().zip(&rects).enumerate() {
            let mut title = column.title.clone();
            if let Some((c, order)) = self.sort
                && c == i
            {
                title.push(' ');
                title.push(if order == SortOrder::Ascending {
                    '▲'
                } else {
                    '▼'
                });
            }
            put(
                buf,
                *rect,
                rect.x + 1,
                rect.y,
                &title,
                Style::default()
                    .fg(p.light_grey)
                    .bg(header_bg)
                    .add_modifier(Modifier::BOLD),
            );
            if rect.right() < area.right() {
                put(
                    buf,
                    area,
                    rect.right().saturating_sub(1),
                    rect.y,
                    "│",
                    Style::default().fg(p.line).bg(header_bg),
                );
            }
            rt.hits.add(*rect, id.with(0x7AB + i as u64));
        }
        let body = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
        let focused = rt.focused(id);
        let rows = TableRows(cells);
        self.state.area = body;
        self.state.page = body.height as usize;
        rt.widget(id, body);
        let visible = self.state.visible(&rows);
        self.state.follow(&visible, body.height as usize);
        let col_rects = self.column_rects(body);
        for y in 0..body.height {
            let rect = Rect::new(body.x, body.y + y, body.width, 1);
            let Some(&row) = visible.get(self.state.scroll + y as usize) else {
                fill(
                    buf,
                    rect,
                    rect.x,
                    rect.y,
                    rect.width,
                    Style::default().bg(on),
                );
                continue;
            };
            let selected = self.state.selection.is_selected(row);
            let (bg, fg) = match (selected, focused) {
                (true, true) => (p.one_bg3, p.white),
                (true, false) => (p.black2, p.white),
                (false, _) if self.state.hover == Some(row) => (p.black2, p.white),
                _ => (on, p.white),
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            for (c, crect) in col_rects.iter().enumerate() {
                let Some(text) = cells[row].get(c) else {
                    continue;
                };
                let crect = &Rect::new(crect.x, rect.y, crect.width, 1).intersection(rect);
                let w = crect.width.saturating_sub(2);
                let shown = crate::components::clip(text, w);
                let x = if self.columns[c].right {
                    crect.right().saturating_sub(1 + text_width(&shown))
                } else {
                    crect.x + 1
                };
                put(
                    buf,
                    *crect,
                    x,
                    crect.y,
                    &shown,
                    Style::default().fg(fg).bg(bg),
                );
            }
        }
        self.state.draw_speed_search(
            rt,
            body,
            buf,
            rows.len() == 0 || !self.state.matches(&rows).is_empty(),
        );
    }

    /// Applies `event`.
    pub fn handle(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        event: &Event,
        cells: &[Vec<String>],
    ) -> TableEvent {
        let rows = TableRows(cells);
        let result = match event {
            Event::Key(key) if rt.focused(id) => self.state.key(key, &rows, false),
            Event::Mouse(_) => {
                if let Some(e) = self.header_pointer(rt, id) {
                    return e;
                }
                self.state.pointer(rt, id, &rows, false)
            }
            _ => CollectionEvent::None,
        };
        match result {
            CollectionEvent::Selected => TableEvent::Selected,
            CollectionEvent::Activated(r) => TableEvent::Activated(r),
            CollectionEvent::ContextMenu { row, x, y } => TableEvent::ContextMenu { row, x, y },
            _ => TableEvent::None,
        }
    }

    fn header_pointer(&mut self, rt: &mut Runtime, id: Id) -> Option<TableEvent> {
        let p = rt.pointer().copied()?;
        let rects = self.column_rects(self.header);
        match p.kind {
            PointerKind::Down(MouseButton::Left) => {
                // The last cell of a column is its divider.
                for (i, rect) in rects.iter().enumerate() {
                    if p.y == rect.y
                        && !rect.is_empty()
                        && p.x + 1 == rect.right()
                        && p.over(id.with(0x7AB + i as u64))
                    {
                        self.resize = Some((i, p.x, self.columns[i].width));
                        return Some(TableEvent::None);
                    }
                }
                None
            }
            PointerKind::Drag(MouseButton::Left) => {
                let (col, start, width) = self.resize?;
                let new = (i32::from(width) + i32::from(p.x) - i32::from(start)).max(4) as u16;
                self.columns[col].width = new;
                Some(TableEvent::Resized(col))
            }
            PointerKind::Up(MouseButton::Left) => {
                if self.resize.take().is_some() {
                    return Some(TableEvent::None);
                }
                for (i, _) in rects.iter().enumerate() {
                    if p.clicked(id.with(0x7AB + i as u64)) {
                        let order = match self.sort {
                            Some((c, SortOrder::Ascending)) if c == i => SortOrder::Descending,
                            _ => SortOrder::Ascending,
                        };
                        self.sort = Some((i, order));
                        return Some(TableEvent::Sort(i, order));
                    }
                }
                None
            }
            _ => None,
        }
    }
}
