//! The workbench: tool windows around an editor, with stripes, headers, splitters and the
//! IntelliJ tool window keys.
//!
//! It keeps the GUI kit's model types: [`ToolWindowPaneState`] (the split ratios, which
//! anchors are hidden, the maximized one), [`ToolWindow`] and [`ContentManager`]. The
//! TUI draws them NvChad-style: a left side bar like nvim-tree, a bottom pane like a toggled
//! terminal split, a thin separator line between panes, and a narrow stripe of icon buttons
//! (left for left and bottom windows, right for right windows).
//!
//! Keys, through the keymap's action ids (call [`Workbench::perform`] for each id of
//! [`Runtime::actions`] when nothing else used the key):
//!
//! | Action | Default | Effect |
//! |---|---|---|
//! | `ActivateToolWindow1..9` | Alt+1..9 | show and focus; focus if shown but not focused; hide if focused |
//! | `HideActiveWindow` | Shift+Esc | hide the focused (or last) tool window |
//! | `HideAllWindows` | Ctrl+Shift+F12 | hide all; again, restore |
//! | `FocusEditor` | Esc | from a tool window, back to the editor |
//! | `JumpToLastWindow` | F12 | focus the last active tool window; from one, the editor |
//! | `MaximizeToolWindow` | Ctrl+Shift+' | toggle maximized |
//! | `ResizeToolWindowLeft/Right/Up/Down` | Ctrl+Alt+Shift+arrows | resize by two cells |
//!
//! Mouse: a stripe button toggles its window (middle click or Shift+click hides it, right
//! click asks for a menu); a click in a pane focuses it; a header click focuses, a double
//! click maximizes, a middle or Shift click hides, the minimize button hides; dragging a
//! separator resizes (a double click resets it); the header tabs select contents.

use crate::components::{RowIcon, fill, fill_rect, put, text_width};
use crate::hit::Id;
use crate::input::{Event, MouseButton};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use crate::tooltip::Tooltip;
use archipel_kit::wm::{ContentManager, ToolWindow, ToolWindowAnchor, ToolWindowPaneState};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// Cells of a stripe.
pub const STRIPE_WIDTH: u16 = 3;
/// The smallest width of a side tool window.
const MIN_SIDE: u16 = 12;
/// The smallest width of the editor column.
const MIN_EDITOR: u16 = 16;
/// The smallest height of the editor and of the bottom tool window.
const MIN_ROWS: u16 = 3;

/// Which part of the workbench has keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WorkbenchFocus {
    /// The editor.
    #[default]
    Editor,
    /// The tool window with this id.
    Tool(String),
}

/// What the workbench did.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WorkbenchEvent {
    /// A tool window was shown and focused.
    Activated(String),
    /// A tool window was hidden.
    Hidden(String),
    /// All tool windows were hidden.
    AllHidden,
    /// Focus went back to the editor.
    FocusEditor,
    /// A tool window was maximized or restored.
    MaximizeToggled(String, bool),
    /// A split ratio changed.
    Resized,
    /// A content tab of a tool window was selected.
    ContentSelected(String, usize),
    /// The close button of a content was pressed.
    ContentCloseRequested(String, usize),
    /// A context menu was requested on a tool window header.
    HeaderMenu(String, u16, u16),
    /// A context menu was requested on a stripe button.
    StripeMenu(String, u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Splitter {
    Left,
    Right,
    Bottom,
}

/// The rectangles of a workbench for one screen size.
#[derive(Debug, Clone, Default)]
pub struct WorkbenchLayout {
    /// The left stripe.
    pub left_stripe: Rect,
    /// The right stripe.
    pub right_stripe: Rect,
    /// The left tool window pane (header included).
    pub left: Option<Rect>,
    /// The right tool window pane.
    pub right: Option<Rect>,
    /// The bottom tool window pane.
    pub bottom: Option<Rect>,
    /// The editor.
    pub editor: Rect,
    /// A maximized pane takes everything but the stripes.
    pub maximized: Option<Rect>,
    /// Separator cells and the anchor they resize.
    splitters: Vec<(Splitter, Rect)>,
}

/// What the app draws content into after [`Workbench::draw`].
#[derive(Debug, Clone, Default)]
pub struct WorkbenchAreas {
    /// The editor.
    pub editor: Rect,
    /// The body of each visible tool window (below its header), by window id.
    pub tools: Vec<(String, Rect)>,
}

impl WorkbenchAreas {
    /// The body of the tool window `id`.
    pub fn tool(&self, id: &str) -> Option<Rect> {
        self.tools.iter().find(|(i, _)| i == id).map(|(_, r)| *r)
    }
}

/// The tool windows around an editor.
#[derive(Debug, Clone)]
pub struct Workbench {
    /// The split ratios and visibility (the GUI kit's state).
    pub state: ToolWindowPaneState,
    /// The tool windows.
    pub windows: Vec<ToolWindow>,
    /// Which part has keyboard focus.
    pub focus: WorkbenchFocus,
    last_tool: Option<String>,
    stash: Option<[bool; 3]>,
    drag: Option<Splitter>,
    layout: WorkbenchLayout,
    bodies: Vec<(String, Rect)>,
    /// The area of the last draw, for ratio math.
    area: Rect,
    id: Id,
}

fn anchor_index(anchor: ToolWindowAnchor) -> usize {
    match anchor {
        ToolWindowAnchor::Left => 0,
        ToolWindowAnchor::Right => 1,
        _ => 2,
    }
}

impl Workbench {
    /// A workbench of `windows` with the default layout.
    pub fn new(windows: Vec<ToolWindow>) -> Self {
        Self {
            state: ToolWindowPaneState::default(),
            windows,
            focus: WorkbenchFocus::Editor,
            last_tool: None,
            stash: None,
            drag: None,
            layout: WorkbenchLayout::default(),
            bodies: Vec::new(),
            area: Rect::default(),
            id: Id::new("workbench"),
        }
    }

    /// The windows anchored at `anchor`, in order.
    pub fn windows_at(&self, anchor: ToolWindowAnchor) -> Vec<&ToolWindow> {
        self.windows.iter().filter(|w| w.anchor == anchor).collect()
    }

    /// The window shown at `anchor`, if the anchor is visible.
    pub fn visible_window(&self, anchor: ToolWindowAnchor) -> Option<&ToolWindow> {
        if !self.state.is_visible(anchor) {
            return None;
        }
        let list = self.windows_at(anchor);
        list.get(
            self.state
                .tool_tab(anchor)
                .min(list.len().saturating_sub(1)),
        )
        .copied()
    }

    /// Whether the window `id` is shown.
    pub fn is_visible(&self, id: &str) -> bool {
        self.window(id)
            .is_some_and(|w| self.visible_window(w.anchor).is_some_and(|v| v.id == id))
    }

    /// The window with `id`.
    pub fn window(&self, id: &str) -> Option<&ToolWindow> {
        self.windows.iter().find(|w| w.id == id)
    }

    /// The window with `id`, mutably (to add contents).
    pub fn window_mut(&mut self, id: &str) -> Option<&mut ToolWindow> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    /// The id of the focused tool window.
    pub fn focused_tool(&self) -> Option<&str> {
        match &self.focus {
            WorkbenchFocus::Tool(id) => Some(id),
            WorkbenchFocus::Editor => None,
        }
    }

    /// Shows the window `id` (selecting its tab) without moving focus.
    pub fn show(&mut self, id: &str) {
        let Some(window) = self.window(id) else {
            return;
        };
        let anchor = window.anchor;
        let index = self
            .windows_at(anchor)
            .iter()
            .position(|w| w.id == id)
            .unwrap_or(0);
        self.state.tool_tabs[anchor_index(anchor)] = index;
        self.state.show(anchor);
    }

    /// Shows and focuses the window `id`.
    pub fn activate(&mut self, id: &str) -> Option<WorkbenchEvent> {
        self.window(id)?;
        self.show(id);
        self.focus = WorkbenchFocus::Tool(id.to_owned());
        self.last_tool = Some(id.to_owned());
        Some(WorkbenchEvent::Activated(id.to_owned()))
    }

    /// Hides the window `id`; focus returns to the editor if it had it.
    pub fn hide(&mut self, id: &str) -> Option<WorkbenchEvent> {
        let anchor = self.window(id)?.anchor;
        if !self.is_visible(id) {
            return None;
        }
        self.state.hide(anchor);
        if self.focused_tool() == Some(id) {
            self.focus = WorkbenchFocus::Editor;
        }
        Some(WorkbenchEvent::Hidden(id.to_owned()))
    }

    /// Toggles the window `id` as a stripe button or Alt+digit does: hidden windows show and
    /// take focus, a visible window without focus takes it, a focused one hides.
    pub fn toggle(&mut self, id: &str) -> Option<WorkbenchEvent> {
        if !self.is_visible(id) {
            return self.activate(id);
        }
        if self.focused_tool() == Some(id) {
            self.hide(id)
        } else {
            self.activate(id)
        }
    }

    /// Selects the content `delta` away in the focused tool window (NextTab, PreviousTab).
    pub fn next_content(&mut self, delta: isize) -> Option<WorkbenchEvent> {
        let id = self.focused_tool()?.to_owned();
        let window = self.window_mut(&id)?;
        let len = window.contents.len();
        if len < 2 {
            return None;
        }
        let at = window.contents.selected_index().unwrap_or(0) as isize;
        let next = (at + delta).rem_euclid(len as isize) as usize;
        window.contents.select_index(next);
        Some(WorkbenchEvent::ContentSelected(id, next))
    }

    fn set_ratio(&mut self, splitter: Splitter, cells: i32) {
        let area = self.area;
        let w = f32::from(area.width.max(1));
        let h = f32::from(area.height.max(1));
        match splitter {
            Splitter::Left => {
                let inner =
                    (w - f32::from(self.layout.left_stripe.width + self.layout.right_stripe.width))
                        .max(1.0);
                let min = f32::from(MIN_SIDE) / inner;
                let max = (1.0 - f32::from(MIN_EDITOR + 2) / inner).clamp(min, 0.7);
                self.state.left_ratio =
                    (self.state.left_ratio + cells as f32 / inner).clamp(min, max);
            }
            Splitter::Right => {
                // center_ratio is the editor's share of what is right of the left pane.
                let rest = w * (1.0 - self.state.left_ratio);
                let delta = cells as f32 / rest.max(1.0);
                self.state.center_ratio = (self.state.center_ratio + delta).clamp(0.3, 0.9);
            }
            Splitter::Bottom => {
                let min = f32::from(MIN_ROWS) / h;
                self.state.top_ratio =
                    (self.state.top_ratio + cells as f32 / h).clamp(min, 1.0 - min);
            }
        }
    }

    /// Applies the action `action_id`; returns the events if it was a workbench action.
    pub fn perform(&mut self, action_id: &str) -> Option<Vec<WorkbenchEvent>> {
        let mut events = Vec::new();
        if let Some(n) = action_id.strip_prefix("ActivateToolWindow")
            && let Ok(digit) = n.parse::<u8>()
        {
            let id = self
                .windows
                .iter()
                .find(|w| w.activate_number == Some(digit))
                .map(|w| w.id.clone())?;
            events.extend(self.toggle(&id));
            return Some(events);
        }
        match action_id {
            "HideActiveWindow" => {
                let id = self
                    .focused_tool()
                    .map(str::to_owned)
                    .or_else(|| self.last_tool.clone())?;
                events.extend(self.hide(&id));
            }
            "HideAllWindows" => {
                if let Some(stash) = self.stash.take() {
                    // Restore what was visible before.
                    self.state.left_hidden = stash[0];
                    self.state.right_hidden = stash[1];
                    self.state.bottom_hidden = stash[2];
                } else {
                    self.stash = Some([
                        self.state.left_hidden,
                        self.state.right_hidden,
                        self.state.bottom_hidden,
                    ]);
                    self.state.hide_all();
                    self.focus = WorkbenchFocus::Editor;
                    events.push(WorkbenchEvent::AllHidden);
                }
            }
            "FocusEditor" => {
                self.focused_tool()?;
                self.focus = WorkbenchFocus::Editor;
                events.push(WorkbenchEvent::FocusEditor);
            }
            "JumpToLastWindow" => {
                if self.focused_tool().is_some() {
                    self.focus = WorkbenchFocus::Editor;
                    events.push(WorkbenchEvent::FocusEditor);
                } else {
                    let id = self.last_tool.clone()?;
                    events.extend(self.activate(&id));
                }
            }
            "MaximizeToolWindow" => {
                let id = self.focused_tool()?.to_owned();
                let anchor = self.window(&id)?.anchor;
                let on = self.state.maximized != Some(anchor);
                self.state.maximized = on.then_some(anchor);
                events.push(WorkbenchEvent::MaximizeToggled(id, on));
            }
            "ResizeToolWindowLeft"
            | "ResizeToolWindowRight"
            | "ResizeToolWindowUp"
            | "ResizeToolWindowDown" => {
                let id = self.focused_tool()?.to_owned();
                let anchor = self.window(&id)?.anchor;
                let (splitter, grow) = match (anchor, action_id) {
                    (ToolWindowAnchor::Left, "ResizeToolWindowRight") => (Splitter::Left, 2),
                    (ToolWindowAnchor::Left, "ResizeToolWindowLeft") => (Splitter::Left, -2),
                    (ToolWindowAnchor::Right, "ResizeToolWindowLeft") => (Splitter::Right, -2),
                    (ToolWindowAnchor::Right, "ResizeToolWindowRight") => (Splitter::Right, 2),
                    (ToolWindowAnchor::Bottom, "ResizeToolWindowUp") => (Splitter::Bottom, -1),
                    (ToolWindowAnchor::Bottom, "ResizeToolWindowDown") => (Splitter::Bottom, 1),
                    _ => return Some(events),
                };
                self.set_ratio(splitter, grow);
                events.push(WorkbenchEvent::Resized);
            }
            "NextTab" | "PreviousTab" if self.focused_tool().is_some() => {
                events.extend(self.next_content(if action_id == "NextTab" { 1 } else { -1 }));
            }
            "CloseContent" if self.focused_tool().is_some() => {
                let id = self.focused_tool()?.to_owned();
                let selected = self.window(&id)?.contents.selected_index()?;
                events.push(WorkbenchEvent::ContentCloseRequested(id, selected));
            }
            _ => return None,
        }
        Some(events)
    }

    fn visible_anchors(&self) -> [bool; 3] {
        [
            self.visible_window(ToolWindowAnchor::Left).is_some(),
            self.visible_window(ToolWindowAnchor::Right).is_some(),
            self.visible_window(ToolWindowAnchor::Bottom).is_some(),
        ]
    }

    /// Computes the rectangles for `area`. Never panics: a screen too small for the panes
    /// drops the side windows and the bottom window.
    pub fn layout(&self, area: Rect) -> WorkbenchLayout {
        let mut layout = WorkbenchLayout::default();
        let has_left_stripe = self
            .windows
            .iter()
            .any(|w| matches!(w.anchor, ToolWindowAnchor::Left | ToolWindowAnchor::Bottom));
        let has_right_stripe = self
            .windows
            .iter()
            .any(|w| w.anchor == ToolWindowAnchor::Right);
        let roomy = area.width >= STRIPE_WIDTH * 2 + MIN_EDITOR + 2;
        let ls = if has_left_stripe && roomy {
            STRIPE_WIDTH
        } else {
            0
        };
        let rs = if has_right_stripe && roomy {
            STRIPE_WIDTH
        } else {
            0
        };
        layout.left_stripe = Rect::new(area.x, area.y, ls, area.height);
        layout.right_stripe = Rect::new(area.right().saturating_sub(rs), area.y, rs, area.height);
        let inner = Rect::new(
            area.x + ls,
            area.y,
            area.width.saturating_sub(ls + rs),
            area.height,
        );
        let [show_left, show_right, show_bottom] = self.visible_anchors();
        if let Some(anchor) = self.state.maximized
            && self.state.is_visible(anchor)
            && inner.width >= 4
            && inner.height >= 3
        {
            layout.maximized = Some(inner);
            layout.editor = Rect::new(inner.x, inner.y, 0, 0);
            return layout;
        }
        let mut x = inner.x;
        let mut width = inner.width;
        // Left pane and its separator.
        if show_left && width >= MIN_SIDE + 1 + MIN_EDITOR {
            let max = width - 1 - MIN_EDITOR;
            let w = ((f32::from(inner.width) * self.state.left_ratio).round() as u16)
                .clamp(MIN_SIDE, max.max(MIN_SIDE));
            let w = w.min(max);
            layout.left = Some(Rect::new(x, inner.y, w, inner.height));
            layout
                .splitters
                .push((Splitter::Left, Rect::new(x + w, inner.y, 1, inner.height)));
            x += w + 1;
            width -= w + 1;
        }
        // Right pane and its separator.
        if show_right && width >= MIN_SIDE + 1 + MIN_EDITOR {
            let max = width - 1 - MIN_EDITOR;
            let editor_share = (f32::from(width) * self.state.center_ratio).round() as u16;
            let w = width
                .saturating_sub(editor_share + 1)
                .clamp(MIN_SIDE, max.max(MIN_SIDE))
                .min(max);
            let rx = x + width - w;
            layout.right = Some(Rect::new(rx, inner.y, w, inner.height));
            layout
                .splitters
                .push((Splitter::Right, Rect::new(rx - 1, inner.y, 1, inner.height)));
            width -= w + 1;
        }
        // Bottom pane under the editor column.
        let mut editor_h = inner.height;
        if show_bottom && inner.height > MIN_ROWS * 2 {
            let max = inner.height - 1 - MIN_ROWS;
            let top = ((f32::from(inner.height) * self.state.top_ratio).round() as u16)
                .clamp(MIN_ROWS, max.max(MIN_ROWS))
                .min(max);
            editor_h = top;
            let by = inner.y + top + 1;
            layout.bottom = Some(Rect::new(x, by, width, inner.height - top - 1));
            layout
                .splitters
                .push((Splitter::Bottom, Rect::new(x, by - 1, width, 1)));
        }
        layout.editor = Rect::new(x, inner.y, width, editor_h);
        layout
    }

    fn stripe_buttons(&self, layout: &WorkbenchLayout) -> Vec<(String, Rect)> {
        let mut out = Vec::new();
        let mut row = layout.left_stripe.y + 1;
        for w in self
            .windows
            .iter()
            .filter(|w| w.anchor == ToolWindowAnchor::Left)
        {
            if layout.left_stripe.width > 0 && row < layout.left_stripe.bottom() {
                out.push((
                    w.id.clone(),
                    Rect::new(layout.left_stripe.x, row, STRIPE_WIDTH, 1),
                ));
                row += 2;
            }
        }
        let bottoms: Vec<&ToolWindow> = self
            .windows
            .iter()
            .filter(|w| w.anchor == ToolWindowAnchor::Bottom)
            .collect();
        let mut y = layout
            .left_stripe
            .bottom()
            .saturating_sub(1 + 2 * bottoms.len() as u16);
        y = y.max(row);
        for w in bottoms {
            if layout.left_stripe.width > 0 && y < layout.left_stripe.bottom() {
                out.push((
                    w.id.clone(),
                    Rect::new(layout.left_stripe.x, y, STRIPE_WIDTH, 1),
                ));
                y += 2;
            }
        }
        let mut row = layout.right_stripe.y + 1;
        for w in self
            .windows
            .iter()
            .filter(|w| w.anchor == ToolWindowAnchor::Right)
        {
            if layout.right_stripe.width > 0 && row < layout.right_stripe.bottom() {
                out.push((
                    w.id.clone(),
                    Rect::new(layout.right_stripe.x, row, STRIPE_WIDTH, 1),
                ));
                row += 2;
            }
        }
        out
    }

    fn button_id(&self, window: &str) -> Id {
        self.id.with(Id::new(window).0)
    }

    fn header_id(&self, window: &str) -> Id {
        self.id.with(Id::new(window).0 ^ 0xAA)
    }

    fn hide_button_id(&self, window: &str) -> Id {
        self.id.with(Id::new(window).0 ^ 0xCC)
    }

    fn tab_id(&self, window: &str, index: usize) -> Id {
        self.id.with(Id::new(window).0 ^ 0xDD ^ index as u64)
    }

    fn splitter_id(&self, s: Splitter) -> Id {
        self.id.with(0x5000 + anchor_index_of(s) as u64)
    }

    /// Draws the stripes, separators and tool window headers, and registers the mouse targets;
    /// returns where the editor and the tool window bodies are for the app to fill.
    pub fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) -> WorkbenchAreas {
        self.area = area;
        let layout = self.layout(area);
        let p = rt.look.palette;
        let area = area.intersection(buf.area);
        fill_rect(buf, area, Style::default().bg(p.black).fg(p.white));
        let mut areas = WorkbenchAreas::default();
        // Stripes.
        for stripe in [layout.left_stripe, layout.right_stripe] {
            fill_rect(buf, stripe, Style::default().bg(p.darker_black));
        }
        let buttons = self.stripe_buttons(&layout);
        for (id, rect) in &buttons {
            let Some(window) = self.window(id) else {
                continue;
            };
            let shown = self.is_visible(id);
            let active = shown && self.focused_tool() == Some(id.as_str());
            let hover = rt.hovered() == Some(self.button_id(id));
            let (bg, fg) = if active {
                (p.one_bg3, p.nord_blue)
            } else if shown {
                (p.one_bg, p.white)
            } else if hover {
                (p.black2, p.white)
            } else {
                (p.darker_black, p.light_grey)
            };
            fill(
                buf,
                *rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            if let Some(icon) = icon_of(&window.icon) {
                let (ch, color) = icon.char_and_color(&rt.look);
                let color = if shown || hover { fg } else { color };
                put(
                    buf,
                    *rect,
                    rect.x + 1,
                    rect.y,
                    &ch.to_string(),
                    Style::default().fg(color).bg(bg),
                );
            } else {
                put(
                    buf,
                    *rect,
                    rect.x + 1,
                    rect.y,
                    &window.title.chars().next().unwrap_or(' ').to_string(),
                    Style::default().fg(fg).bg(bg),
                );
            }
            rt.clickable(self.button_id(id), *rect);
            let mut tip = Tooltip::new(window.title.clone());
            if let Some(n) = window.activate_number
                && let Some(sc) = rt.shortcut_text(&crate::input::activate_tool_window_id(n))
            {
                tip = tip.shortcut(sc);
            }
            rt.tooltip.offer(self.button_id(id), *rect, tip);
        }
        // The maximized pane replaces everything.
        if let Some(rect) = layout.maximized
            && let Some(anchor) = self.state.maximized
            && let Some(window) = self.visible_window(anchor).cloned()
        {
            let body = self.draw_pane(rt, &window, rect, buf);
            areas.tools.push((window.id.clone(), body));
            areas.editor = Rect::new(rect.x, rect.y, 0, 0);
            self.layout = layout;
            self.bodies = areas.tools.clone();
            return areas;
        }
        // Panes.
        for (anchor, rect) in [
            (ToolWindowAnchor::Left, layout.left),
            (ToolWindowAnchor::Right, layout.right),
            (ToolWindowAnchor::Bottom, layout.bottom),
        ] {
            if let (Some(rect), Some(window)) = (rect, self.visible_window(anchor).cloned()) {
                let body = self.draw_pane(rt, &window, rect, buf);
                areas.tools.push((window.id.clone(), body));
            }
        }
        // Separators (and their hit zones).
        for (splitter, rect) in &layout.splitters {
            let horizontal = *splitter == Splitter::Bottom;
            let style = Style::default().fg(p.line).bg(p.black);
            for y in rect.top()..rect.bottom() {
                for x in rect.left()..rect.right() {
                    put(buf, *rect, x, y, if horizontal { "─" } else { "│" }, style);
                }
            }
            rt.hits.add(*rect, self.splitter_id(*splitter));
        }
        areas.editor = layout.editor;
        self.layout = layout;
        self.bodies = areas.tools.clone();
        areas
    }

    fn draw_pane(
        &self,
        rt: &mut Runtime,
        window: &ToolWindow,
        rect: Rect,
        buf: &mut Buffer,
    ) -> Rect {
        let p = rt.look.palette;
        let focused = self.focused_tool() == Some(window.id.as_str());
        fill_rect(buf, rect, Style::default().bg(p.darker_black).fg(p.white));
        let header = Rect::new(rect.x, rect.y, rect.width, 1.min(rect.height));
        let tabbed = window.contents.len() > 1;
        let (fg, bg) = if focused && !tabbed {
            (p.on(p.nord_blue), p.nord_blue)
        } else {
            (p.light_grey, p.one_bg)
        };
        rt.hits.add(header, self.header_id(&window.id));
        let mut x =
            crate::components::title_block(buf, header, rect.x + 1, rect.y, &window.title, fg, bg)
                + 1;
        if window.contents.len() > 1 {
            for (i, content) in window.contents.contents().iter().enumerate() {
                let selected = window.contents.selected_index() == Some(i);
                let label = format!(" {} ", content.display_name);
                let style = if selected {
                    Style::default()
                        .fg(p.white)
                        .bg(p.one_bg3)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.light_grey).bg(p.darker_black)
                };
                let w = text_width(&label);
                let r = Rect::new(x, rect.y, w, 1).intersection(header);
                put(buf, header, x, rect.y, &label, style);
                rt.clickable(self.tab_id(&window.id, i), r);
                x += w;
            }
        }
        let hide = Rect::new(
            rect.right().saturating_sub(3),
            rect.y,
            3.min(rect.width),
            1.min(rect.height),
        );
        if rect.width > 12 {
            put(
                buf,
                hide,
                hide.x + 1,
                hide.y,
                &rt.look.glyph(crate::Glyph::Minimize),
                Style::default().fg(p.light_grey).bg(p.darker_black),
            );
            rt.clickable(self.hide_button_id(&window.id), hide);
        }
        Rect::new(
            rect.x,
            rect.y + header.height,
            rect.width,
            rect.height.saturating_sub(header.height),
        )
    }

    /// Applies the mouse event of `rt`: stripe buttons, headers, panes, separators.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Vec<WorkbenchEvent> {
        let mut events = Vec::new();
        let Event::Mouse(_) = event else {
            return events;
        };
        let Some(p) = rt.pointer().copied() else {
            return events;
        };
        let ids: Vec<String> = self.windows.iter().map(|w| w.id.clone()).collect();
        // Separator drags.
        if let Some(s) = self.drag {
            match p.kind {
                PointerKind::Drag(MouseButton::Left) => {
                    self.drag_to(s, p.x, p.y);
                    events.push(WorkbenchEvent::Resized);
                    return events;
                }
                PointerKind::Up(MouseButton::Left) => {
                    self.drag = None;
                    return events;
                }
                _ => {}
            }
        }
        if p.kind == PointerKind::Down(MouseButton::Left) {
            for s in [Splitter::Left, Splitter::Right, Splitter::Bottom] {
                if p.over(self.splitter_id(s)) {
                    if p.clicks >= 2 {
                        let d = ToolWindowPaneState::default();
                        match s {
                            Splitter::Left => self.state.left_ratio = d.left_ratio,
                            Splitter::Right => self.state.center_ratio = d.center_ratio,
                            Splitter::Bottom => self.state.top_ratio = d.top_ratio,
                        }
                        events.push(WorkbenchEvent::Resized);
                    } else {
                        self.drag = Some(s);
                    }
                    return events;
                }
            }
        }
        for id in &ids {
            let close_click =
                p.middle_press() || (p.kind == PointerKind::Down(MouseButton::Left) && p.shift());
            // Stripe button.
            if p.over(self.button_id(id)) {
                if close_click {
                    events.extend(self.hide(id));
                } else if p.right_press() {
                    events.push(WorkbenchEvent::StripeMenu(id.clone(), p.x, p.y));
                } else if p.clicked(self.button_id(id)) && !p.shift() {
                    events.extend(self.toggle(id));
                }
                return events;
            }
            if p.over(self.hide_button_id(id)) {
                if p.clicked(self.hide_button_id(id)) {
                    events.extend(self.hide(id));
                }
                return events;
            }
            let content_count = self.window(id).map_or(0, |w| w.contents.len());
            for i in 0..content_count {
                if p.pressed_left(self.tab_id(id, i)) && !p.shift() {
                    if let Some(w) = self.window_mut(id) {
                        w.contents.select_index(i);
                    }
                    events.extend(self.activate(id));
                    events.push(WorkbenchEvent::ContentSelected(id.clone(), i));
                    return events;
                }
            }
            if p.over(self.header_id(id)) {
                if close_click {
                    events.extend(self.hide(id));
                } else if p.right_press() {
                    events.push(WorkbenchEvent::HeaderMenu(id.clone(), p.x, p.y));
                } else if p.kind == PointerKind::Down(MouseButton::Left) {
                    events.extend(self.activate(id));
                    if p.clicks >= 2 {
                        let anchor = self.window(id).map(|w| w.anchor);
                        if let Some(anchor) = anchor {
                            let on = self.state.maximized != Some(anchor);
                            self.state.maximized = on.then_some(anchor);
                            events.push(WorkbenchEvent::MaximizeToggled(id.clone(), on));
                        }
                    }
                }
                return events;
            }
        }
        // A press inside a pane focuses it (the content inside handles the event itself).
        if matches!(p.kind, PointerKind::Down(_)) {
            let pos = ratatui::layout::Position::new(p.x, p.y);
            if let Some((id, _)) = self.bodies.iter().find(|(_, r)| r.contains(pos)).cloned() {
                if self.focused_tool() != Some(id.as_str()) {
                    events.extend(self.activate(&id));
                }
            } else if self.layout.editor.contains(pos) && self.focus != WorkbenchFocus::Editor {
                self.focus = WorkbenchFocus::Editor;
                events.push(WorkbenchEvent::FocusEditor);
            }
        }
        events
    }

    fn drag_to(&mut self, s: Splitter, x: u16, y: u16) {
        let area = self.area;
        let w = f32::from(area.width.max(1));
        let h = f32::from(area.height.max(1));
        let ls = self.layout.left_stripe.width;
        match s {
            Splitter::Left => {
                let inner = (w - f32::from(ls + self.layout.right_stripe.width)).max(1.0);
                let min = f32::from(MIN_SIDE) / inner;
                let max = (1.0 - f32::from(MIN_EDITOR + 2) / inner).clamp(min, 0.7);
                self.state.left_ratio =
                    (f32::from(x.saturating_sub(area.x + ls)) / inner).clamp(min, max);
            }
            Splitter::Right => {
                let left_w = self.layout.left.map_or(0, |l| l.width + 1);
                let start = area.x + ls + left_w;
                let right_end = area.right().saturating_sub(self.layout.right_stripe.width);
                let span = f32::from(right_end.saturating_sub(start)).max(1.0);
                self.state.center_ratio =
                    (f32::from(x.saturating_sub(start)) / span).clamp(0.3, 0.9);
            }
            Splitter::Bottom => {
                let min = f32::from(MIN_ROWS) / h;
                self.state.top_ratio =
                    (f32::from(y.saturating_sub(area.y)) / h).clamp(min, 1.0 - min);
            }
        }
    }

    /// The ids of the contents of window `id` for a header (name, selected).
    pub fn contents(&self, id: &str) -> Option<&ContentManager> {
        self.window(id).map(|w| &w.contents)
    }
}

fn anchor_index_of(s: Splitter) -> usize {
    match s {
        Splitter::Left => 0,
        Splitter::Right => 1,
        Splitter::Bottom => 2,
    }
}

fn icon_of(icon: &archipel_kit::icons::Icon) -> Option<RowIcon> {
    use archipel_kit::icons::Icon;
    Some(match icon {
        Icon::Glyph(g) => RowIcon::Glyph(*g),
        Icon::Language(l) => RowIcon::Language(*l),
        Icon::Symbol(s) => RowIcon::Symbol(*s),
        Icon::FolderKind(_) => RowIcon::Folder(false),
        Icon::FileImage => RowIcon::Glyph(crate::Glyph::Image),
        Icon::FileArchive => RowIcon::Glyph(crate::Glyph::Archive),
    })
}
