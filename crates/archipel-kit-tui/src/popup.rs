//! Popup lists: [`ListPopup`] over the kit's [`PopupStep`]s and [`ActionPopupMenu`] over an
//! [`ActionGroup`], both driven by one engine, [`PopupChain`].
//!
//! Behavior (IntelliJ `ListPopupImpl`, spec section 5):
//!
//! - <kbd>Up</kbd>/<kbd>Down</kbd> skip separators and disabled rows and wrap, <kbd>Home</kbd>,
//!   <kbd>End</kbd>, <kbd>PgUp</kbd>, <kbd>PgDn</kbd> jump, <kbd>Enter</kbd> chooses,
//!   <kbd>Right</kbd> opens a sub-step, <kbd>Left</kbd> and <kbd>Esc</kbd> go back one
//!   level, <kbd>Shift+Esc</kbd> closes all, a digit or letter chooses the row it numbers
//!   when mnemonics are on, and typing starts the speed search when it is on.
//! - Hovering selects the row under the pointer (not on the first move after opening);
//!   resting on a row with a sub-step opens it after 250 ms; moving toward an open submenu
//!   does not retarget the selection; a click chooses on release, a press on the chevron
//!   opens the sub-step without choosing; a press outside cancels.
//! - A popup with auto-selection opens the selected sub-step after 500 ms without input.
//! - A popup refuses to reopen within 200 ms of closing, so the click that closes it by
//!   pressing its owner button does not reopen it.
//!
//! Two looks: [`PopupStyle::Menu`], like Neovim's completion menu (`pmenu`: a raised surface,
//! the selected row filled with the accent, kind icons in color, shortcuts grey at the
//! right, submenus as adjacent floats), and [`PopupStyle::Telescope`], a borderless centered
//! float with a prompt on the cursor line color, a colored title block and the results on
//! `darker_black`.

use crate::Look;
use crate::components::{RowIcon, fill, fill_rect, put, text_width};
use crate::input::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton};
use crate::pointer::{Pointer, PointerKind};
use crate::runtime::Runtime;
use archipel_kit::action_system::{
    ActionGroup, ActionPlace, ActionRow, DataContext, InputKind, Keymap, flatten_with_shortcuts,
    perform_with_modifiers,
};
use archipel_kit::icons::Icon;
use archipel_kit::tokens::motion;
use archipel_kit::ui::SpeedSearchComparator;
use archipel_kit::ui::popup::{PopupStep, PopupStepResult};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::rc::Rc;

/// One row of a popup level.
#[derive(Debug, Clone, Default)]
pub struct RowSpec {
    /// The text.
    pub text: String,
    /// The icon.
    pub icon: Option<RowIcon>,
    /// The shortcut text, grey at the right.
    pub shortcut: Option<String>,
    /// `Some` for toggles: a check mark column.
    pub checked: Option<bool>,
    /// Whether the row can be selected and chosen (disabled rows are grey).
    pub selectable: bool,
    /// A separator line above the row, with an optional caption.
    pub separator: Option<Option<String>>,
    /// Whether the row opens a sub-step (a chevron).
    pub sub: bool,
    /// The mnemonic character that chooses the row.
    pub mnemonic: Option<char>,
}

/// A level of a popup: rows and options.
#[derive(Debug, Clone, Default)]
pub struct LevelSpec {
    /// The rows.
    pub rows: Vec<RowSpec>,
    /// The title.
    pub title: Option<String>,
    /// Whether typing starts a speed search that filters the rows.
    pub speed_search: bool,
    /// Whether the selected sub-step opens after 500 ms.
    pub auto_select: bool,
    /// Whether mnemonic characters choose rows.
    pub mnemonics: bool,
    /// The row selected when the popup opens by keyboard.
    pub default_index: Option<usize>,
}

/// What choosing a row did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    /// The popup is done: the value was chosen.
    Final,
    /// A sub-step opened.
    Opened,
    /// Nothing more happens (the popup stays open).
    Stay,
}

/// What a popup shows and how it reacts to a choice: the model behind a [`PopupChain`].
pub trait PopupSource {
    /// The rows of the level that `path` (the open rows of the levels above) leads to, or
    /// `None` if there is no such level.
    fn level(&mut self, path: &[usize]) -> Option<LevelSpec>;

    /// Chooses `row` of the level at `path`. `final_choice` is false when a sub-step is
    /// being opened.
    fn choose(&mut self, path: &[usize], row: usize, final_choice: bool) -> Chosen;

    /// The levels above `depth` closed: forget their state.
    fn closed(&mut self, depth: usize) {
        let _ = depth;
    }

    /// The popup was cancelled.
    fn cancelled(&mut self) {}
}

/// How a popup looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PopupStyle {
    /// A completion menu: a raised surface, the selection filled with the accent.
    #[default]
    Menu,
    /// A Telescope float: title block, prompt, results.
    Telescope,
}

/// Where level 0 opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Below `anchor` (above when there is no room).
    Below(Rect),
    /// At a cell.
    At(u16, u16),
    /// Centered on the screen.
    Center,
}

/// Options of a popup.
#[derive(Debug, Clone, Copy)]
pub struct PopupOptions {
    /// Open a sub-step 250 ms after the pointer rests on its row (action popups do).
    pub show_submenu_on_hover: bool,
    /// Cancel on a press outside.
    pub cancel_on_click_outside: bool,
    /// The look.
    pub style: PopupStyle,
}

impl Default for PopupOptions {
    fn default() -> Self {
        Self {
            show_submenu_on_hover: false,
            cancel_on_click_outside: true,
            style: PopupStyle::Menu,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Level {
    selected: Option<usize>,
    open_row: Option<usize>,
    query: String,
    pending: Option<(usize, f64)>,
    idle_since: f64,
    auto_fired: bool,
    rect: Rect,
    row_rects: Vec<(usize, Rect)>,
    scroll: usize,
    scroll_to: bool,
    select_first: bool,
    initialised: bool,
    page: usize,
    chevron_press: bool,
}

/// What an event did to a popup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PopupOutput {
    /// The path of rows that was chosen, when the popup finished with a choice.
    pub chosen: Option<Vec<usize>>,
    /// The popup was cancelled (Esc, a press outside).
    pub cancelled: bool,
}

/// The state of a popup: its open levels, selection, queries and timers.
#[derive(Debug, Clone, Default)]
pub struct PopupChain {
    levels: Vec<Level>,
    open: bool,
    closed_at: Option<f64>,
    last_pointer: Option<(u16, u16)>,
    moves: u32,
    away: i32,
    started: bool,
    preselect: bool,
}

/// How far (cells) the pointer may move away from an open submenu before the "moving
/// toward the submenu" guard ends.
const AWAY_CELLS: i32 = 3;

impl PopupChain {
    /// Whether the popup is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the popup at time `now`; `preselect` selects the first row (keyboard opened).
    /// Returns false when it closed less than 200 ms ago.
    pub fn open(&mut self, now: f64, preselect: bool) -> bool {
        if self.open {
            return true;
        }
        if self
            .closed_at
            .is_some_and(|closed| now - closed < f64::from(motion::POPUP_REOPEN_GUARD))
        {
            return false;
        }
        *self = Self {
            open: true,
            closed_at: self.closed_at,
            preselect,
            ..Self::default()
        };
        true
    }

    /// Forgets that the popup closed, so it may reopen at once.
    pub fn forget_close(&mut self) {
        self.closed_at = None;
    }

    /// Closes the popup at time `now`.
    pub fn close(&mut self, now: f64) {
        if self.open {
            self.closed_at = Some(now);
        }
        self.open = false;
        self.levels.clear();
        self.started = false;
    }

    /// How many levels are open.
    pub fn depth(&self) -> usize {
        self.levels.len()
    }

    /// The selected row of level `depth`.
    pub fn selected(&self, depth: usize) -> Option<usize> {
        self.levels.get(depth).and_then(|l| l.selected)
    }

    /// The row of level `depth` whose sub-step is open.
    pub fn open_row(&self, depth: usize) -> Option<usize> {
        self.levels.get(depth).and_then(|l| l.open_row)
    }

    /// The speed search text of level `depth`.
    pub fn query(&self, depth: usize) -> &str {
        self.levels.get(depth).map_or("", |l| l.query.as_str())
    }

    /// The rectangle of level `depth` as of the last draw.
    pub fn rect(&self, depth: usize) -> Option<Rect> {
        self.levels.get(depth).map(|l| l.rect)
    }

    fn path(&self, depth: usize) -> Vec<usize> {
        self.levels[..depth.min(self.levels.len())]
            .iter()
            .filter_map(|l| l.open_row)
            .collect()
    }

    fn truncate(&mut self, depth: usize, src: &mut dyn PopupSource) {
        self.levels.truncate(depth);
        if let Some(last) = self.levels.last_mut() {
            last.open_row = None;
            last.pending = None;
        }
        src.closed(depth);
    }
}

fn visible_rows(spec: &LevelSpec, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..spec.rows.len()).collect();
    }
    let comparator = SpeedSearchComparator::new(query);
    spec.rows
        .iter()
        .enumerate()
        .filter(|(_, r)| comparator.matches(&r.text))
        .map(|(i, _)| i)
        .collect()
}

fn first_selectable(spec: &LevelSpec, visible: &[usize]) -> Option<usize> {
    visible.iter().copied().find(|r| spec.rows[*r].selectable)
}

/// Steps `by` through the selectable rows of `visible`.
fn step(
    spec: &LevelSpec,
    visible: &[usize],
    current: Option<usize>,
    by: isize,
    wrap: bool,
) -> Option<usize> {
    let selectable: Vec<usize> = visible
        .iter()
        .copied()
        .filter(|r| spec.rows[*r].selectable)
        .collect();
    if selectable.is_empty() {
        return None;
    }
    let pos = current.and_then(|row| selectable.iter().position(|r| *r == row));
    let len = selectable.len() as isize;
    let next = match pos {
        None if by > 0 => 0,
        None => len - 1,
        Some(p) => {
            let target = p as isize + by;
            if wrap {
                target.rem_euclid(len)
            } else {
                target.clamp(0, len - 1)
            }
        }
    };
    Some(selectable[next as usize])
}

/// Whether `p` is inside the triangle (`a`, `b`, `c`).
fn in_triangle(p: (i32, i32), a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> bool {
    let sign = |p1: (i32, i32), p2: (i32, i32), p3: (i32, i32)| {
        (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
    };
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let neg = d1 < 0 || d2 < 0 || d3 < 0;
    let pos = d1 > 0 || d2 > 0 || d3 > 0;
    !(neg && pos)
}

impl PopupChain {
    fn ensure_started(&mut self, now: f64) {
        if !self.started && self.open {
            self.started = true;
            self.levels.push(Level {
                select_first: self.preselect,
                idle_since: now,
                ..Level::default()
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn choose_row(
        &mut self,
        rt: &Runtime,
        src: &mut dyn PopupSource,
        depth: usize,
        row: usize,
        final_choice: bool,
        by_keyboard: bool,
        out: &mut PopupOutput,
    ) {
        let path = self.path(depth);
        match src.choose(&path, row, final_choice) {
            Chosen::Final => {
                let mut chosen = path;
                chosen.push(row);
                out.chosen = Some(chosen);
                self.close(rt.now);
            }
            Chosen::Opened => {
                self.levels.truncate(depth + 1);
                if let Some(level) = self.levels.get_mut(depth) {
                    level.open_row = Some(row);
                    level.pending = None;
                }
                self.levels.push(Level {
                    select_first: by_keyboard,
                    idle_since: rt.now,
                    ..Level::default()
                });
            }
            Chosen::Stay => {}
        }
    }

    /// Applies `event` to the popup. The popup is modal: every key and mouse event is for it.
    pub fn event(
        &mut self,
        rt: &mut Runtime,
        src: &mut dyn PopupSource,
        event: &Event,
        options: &PopupOptions,
    ) -> PopupOutput {
        let mut out = PopupOutput::default();
        if !self.open {
            return out;
        }
        self.ensure_started(rt.now);
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.key(rt, src, key, &mut out);
            }
            Event::Mouse(_) => {
                if let Some(p) = rt.pointer().copied() {
                    self.mouse(rt, src, &p, options, &mut out);
                }
            }
            _ => {}
        }
        if out.cancelled {
            src.cancelled();
        }
        out
    }

    fn key(
        &mut self,
        rt: &Runtime,
        src: &mut dyn PopupSource,
        key: &KeyEvent,
        out: &mut PopupOutput,
    ) {
        let depth = self.levels.len().saturating_sub(1);
        let path = self.path(depth);
        let Some(spec) = src.level(&path) else {
            self.truncate(depth, src);
            return;
        };
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let visible = visible_rows(&spec, &self.levels[depth].query);
        let page = self.levels[depth].page.max(2) as isize - 1;
        let current = self.levels[depth].selected;
        let select = |this: &mut Self, row: Option<usize>| {
            let level = &mut this.levels[depth];
            level.selected = row;
            level.scroll_to = true;
            level.pending = None;
            level.idle_since = rt.now;
            level.auto_fired = false;
        };
        match key.code {
            KeyCode::Esc => {
                if shift {
                    self.close_all(rt, src, out);
                } else if !self.levels[depth].query.is_empty() {
                    self.levels[depth].query.clear();
                } else if depth > 0 {
                    self.truncate(depth, src);
                } else {
                    self.close_all(rt, src, out);
                }
            }
            KeyCode::Left if !ctrl && !alt => {
                if depth > 0 {
                    self.truncate(depth, src);
                }
            }
            KeyCode::Right if !ctrl && !alt => {
                if let Some(row) = current.filter(|r| spec.rows[*r].sub) {
                    self.choose_row(rt, src, depth, row, false, true, out);
                }
            }
            KeyCode::Enter => {
                if let Some(row) = current {
                    self.choose_row(rt, src, depth, row, true, true, out);
                }
            }
            KeyCode::Up => select(self, step(&spec, &visible, current, -1, true)),
            KeyCode::Down => select(self, step(&spec, &visible, current, 1, true)),
            KeyCode::PageUp => select(self, step(&spec, &visible, current, -page, false)),
            KeyCode::PageDown => select(self, step(&spec, &visible, current, page, false)),
            KeyCode::Home => select(self, first_selectable(&spec, &visible)),
            KeyCode::End => select(
                self,
                visible
                    .iter()
                    .rev()
                    .copied()
                    .find(|r| spec.rows[*r].selectable),
            ),
            KeyCode::Tab => select(self, step(&spec, &visible, current, 1, true)),
            KeyCode::BackTab => select(self, step(&spec, &visible, current, -1, true)),
            KeyCode::Backspace if spec.speed_search => {
                let query = &mut self.levels[depth].query;
                if ctrl {
                    query.clear();
                } else {
                    query.pop();
                }
                self.refresh_selection(depth, &spec);
            }
            KeyCode::Char(c)
                if ctrl && matches!(c, 'n' | 'N' | 'p' | 'P' | 'j' | 'J' | 'k' | 'K') =>
            {
                let by = if matches!(c, 'n' | 'N' | 'j' | 'J') {
                    1
                } else {
                    -1
                };
                select(self, step(&spec, &visible, current, by, true));
            }
            KeyCode::Char(c) if !ctrl && !alt && !c.is_control() => {
                let mnemonic = spec.mnemonics && self.levels[depth].query.is_empty();
                let hit = mnemonic
                    .then(|| {
                        visible.iter().copied().find(|r| {
                            spec.rows[*r].selectable
                                && spec.rows[*r]
                                    .mnemonic
                                    .is_some_and(|m| m.eq_ignore_ascii_case(&c))
                        })
                    })
                    .flatten();
                if let Some(row) = hit {
                    self.choose_row(rt, src, depth, row, true, true, out);
                } else if spec.speed_search {
                    self.levels[depth].query.push(c);
                    self.refresh_selection(depth, &spec);
                }
            }
            _ => {}
        }
        if let Some(level) = self.levels.get_mut(depth) {
            level.idle_since = rt.now;
        }
    }

    /// After the query changed: keep the selection if still visible, else the first match.
    fn refresh_selection(&mut self, depth: usize, spec: &LevelSpec) {
        let level = &mut self.levels[depth];
        let visible = visible_rows(spec, &level.query);
        if level.selected.is_none_or(|r| !visible.contains(&r)) {
            level.selected = first_selectable(spec, &visible);
        }
        level.scroll = 0;
        level.scroll_to = true;
    }

    fn close_all(&mut self, rt: &Runtime, src: &mut dyn PopupSource, out: &mut PopupOutput) {
        self.truncate(0, src);
        self.close(rt.now);
        out.cancelled = true;
    }

    fn level_at(&self, x: u16, y: u16) -> Option<usize> {
        let pos = ratatui::layout::Position::new(x, y);
        (0..self.levels.len())
            .rev()
            .find(|d| self.levels[*d].rect.contains(pos))
    }

    fn mouse(
        &mut self,
        rt: &Runtime,
        src: &mut dyn PopupSource,
        p: &Pointer,
        options: &PopupOptions,
        out: &mut PopupOutput,
    ) {
        let inside = self.level_at(p.x, p.y);
        // Presses outside every level cancel the chain.
        if matches!(p.kind, PointerKind::Down(_))
            && inside.is_none()
            && options.cancel_on_click_outside
            && self.levels.iter().any(|l| !l.rect.is_empty())
        {
            self.close_all(rt, src, out);
            return;
        }
        let Some(depth) = inside else {
            return;
        };
        let path = self.path(depth);
        let Some(spec) = src.level(&path) else {
            return;
        };
        let hit = self.levels[depth]
            .row_rects
            .iter()
            .find(|(_, r)| r.contains((p.x, p.y).into()))
            .map(|(row, _)| *row);
        match p.kind {
            PointerKind::Move | PointerKind::Drag(_) => {
                self.moves += 1;
                let real_move = self.moves >= 2 && p.moved;
                let prev = self.last_pointer.replace((p.x, p.y));
                if !real_move {
                    return;
                }
                self.hover(rt, depth, &spec, hit, prev, p, options);
            }
            PointerKind::Down(MouseButton::Left) => {
                if let Some(row) = hit.filter(|r| spec.rows[*r].selectable) {
                    let rect = self.levels[depth].rect;
                    let chevron = spec.rows[row].sub && p.x + 3 >= rect.right();
                    self.levels[depth].selected = Some(row);
                    self.levels[depth].chevron_press = chevron;
                    if chevron && self.levels[depth].open_row != Some(row) {
                        self.choose_row(rt, src, depth, row, false, false, out);
                    }
                }
            }
            PointerKind::Up(MouseButton::Left) => {
                let chevron = std::mem::take(&mut self.levels[depth].chevron_press);
                // A click: pressed and released on the same row.
                let origin_row = p.origin_pos.and_then(|(ox, oy)| {
                    self.levels[depth]
                        .row_rects
                        .iter()
                        .find(|(_, rect)| rect.contains((ox, oy).into()))
                        .map(|(r, _)| *r)
                });
                if let Some(row) = hit.filter(|r| spec.rows[*r].selectable)
                    && !chevron
                    && origin_row == Some(row)
                {
                    self.choose_row(rt, src, depth, row, !spec.rows[row].sub, false, out);
                }
            }
            PointerKind::ScrollUp | PointerKind::ScrollDown => {
                let level = &mut self.levels[depth];
                let total = visible_rows(&spec, &level.query).len();
                let max = total.saturating_sub(level.page.max(1));
                level.scroll = (level.scroll as i32 + p.wheel() * 3).clamp(0, max as i32) as usize;
                level.scroll_to = false;
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn hover(
        &mut self,
        rt: &Runtime,
        depth: usize,
        spec: &LevelSpec,
        hit: Option<usize>,
        prev: Option<(u16, u16)>,
        p: &Pointer,
        options: &PopupOptions,
    ) {
        // Moving toward an open submenu does not retarget the selection.
        let child = self.levels.get(depth + 1).map(|l| l.rect);
        let own = self.levels[depth].rect;
        let moving_to_child = child.is_some_and(|child| {
            let near = if child.x >= own.right().saturating_sub(1) {
                i32::from(child.x)
            } else {
                i32::from(child.right())
            };
            let from = prev.map_or((i32::from(p.x), i32::from(p.y)), |(x, y)| {
                (i32::from(x), i32::from(y))
            });
            let dx = i32::from(p.x) - from.0;
            let toward = (near - from.0).signum() == dx.signum() || dx == 0;
            if toward {
                self.away = 0;
            } else {
                self.away += dx.abs();
            }
            in_triangle(
                (i32::from(p.x), i32::from(p.y)),
                from,
                (near, i32::from(child.y)),
                (near, i32::from(child.bottom())),
            ) && (toward || self.away <= AWAY_CELLS)
        });
        let level = &mut self.levels[depth];
        level.idle_since = rt.now;
        level.auto_fired = false;
        if moving_to_child {
            return;
        }
        let target = hit.filter(|r| spec.rows[*r].selectable);
        if level.selected != target {
            level.selected = target;
            level.pending = None;
        }
        if level.open_row.is_some() && level.open_row != target {
            level.open_row = None;
            self.levels.truncate(depth + 1);
        }
        let level = &mut self.levels[depth];
        if let Some(row) = target
            && spec.rows[row].sub
            && options.show_submenu_on_hover
            && level.open_row != Some(row)
            && level.pending.map(|(r, _)| r) != Some(row)
        {
            level.pending = Some((row, rt.now));
        }
    }

    /// Runs the timers: the 250 ms submenu delay and the 500 ms auto-selection. Call it every
    /// tick; [`PopupChain::deadline`] says when the next one is due.
    pub fn tick(&mut self, rt: &Runtime, src: &mut dyn PopupSource) {
        if !self.open {
            return;
        }
        self.ensure_started(rt.now);
        let mut depth = 0;
        while depth < self.levels.len() {
            let path = self.path(depth);
            let Some(spec) = src.level(&path) else {
                self.truncate(depth, src);
                return;
            };
            let delay = f64::from(motion::POPUP_SUBMENU_DELAY);
            let mut open_child = None;
            let level = &mut self.levels[depth];
            if let Some((row, since)) = level.pending {
                if level.selected != Some(row) {
                    level.pending = None;
                } else if rt.now - since >= delay {
                    level.pending = None;
                    open_child = Some(row);
                }
            }
            if spec.auto_select
                && let Some(row) = level.selected
                && !level.auto_fired
                && level.open_row.is_none()
                && spec.rows[row].sub
                && rt.now - level.idle_since >= f64::from(motion::POPUP_AUTO_SELECTION_DELAY)
            {
                level.auto_fired = true;
                open_child = Some(row);
            }
            if let Some(row) = open_child {
                let mut out = PopupOutput::default();
                self.choose_row(rt, src, depth, row, false, false, &mut out);
                return;
            }
            depth += 1;
        }
    }

    /// The next time [`tick`](Self::tick) has something to do.
    pub fn deadline(&self) -> Option<f64> {
        let mut next: Option<f64> = None;
        let mut consider = |t: f64| next = Some(next.map_or(t, |n: f64| n.min(t)));
        for level in &self.levels {
            if let Some((_, since)) = level.pending {
                consider(since + f64::from(motion::POPUP_SUBMENU_DELAY));
            }
            if level.selected.is_some() && !level.auto_fired && level.open_row.is_none() {
                consider(level.idle_since + f64::from(motion::POPUP_AUTO_SELECTION_DELAY));
            }
        }
        next
    }
}

// ----- drawing -----

#[derive(Debug, Clone, Copy)]
enum Line {
    Separator(usize),
    Row(usize),
}

fn icon_of(icon: &Icon) -> Option<RowIcon> {
    Some(match icon {
        Icon::Glyph(g) => RowIcon::Glyph(*g),
        Icon::Language(l) => RowIcon::Language(*l),
        Icon::Symbol(s) => RowIcon::Symbol(*s),
        Icon::FolderKind(_) => RowIcon::Folder(false),
        Icon::FileImage => RowIcon::Glyph(crate::Glyph::Image),
        Icon::FileArchive => RowIcon::Glyph(crate::Glyph::Archive),
    })
}

struct Surface {
    bg: Color,
    fg: Color,
    muted: Color,
    selected_bg: Color,
    selected_fg: Color,
    disabled: Color,
}

fn surface(look: &Look, style: PopupStyle) -> Surface {
    let p = &look.palette;
    match style {
        PopupStyle::Menu => Surface {
            bg: p.one_bg,
            fg: p.white,
            muted: p.light_grey,
            selected_bg: p.pmenu_bg,
            selected_fg: p.on(p.pmenu_bg),
            disabled: p.grey_fg,
        },
        PopupStyle::Telescope => Surface {
            bg: p.darker_black,
            fg: p.white,
            muted: p.light_grey,
            selected_bg: p.one_bg3,
            selected_fg: p.white,
            disabled: p.grey_fg,
        },
    }
}

impl PopupChain {
    /// Draws every open level into `buf` and registers them (and a block over the screen)
    /// with the hit map. Call from the draw of the overlay that owns the popup.
    pub fn draw(
        &mut self,
        rt: &mut Runtime,
        src: &mut dyn PopupSource,
        placement: Placement,
        options: &PopupOptions,
        buf: &mut Buffer,
    ) {
        if !self.open {
            return;
        }
        self.ensure_started(rt.now);
        let screen = rt.screen.intersection(buf.area);
        rt.hits.block(screen);
        let mut depth = 0;
        let mut origin: Option<Rect> = None;
        while depth < self.levels.len() {
            let path = self.path(depth);
            let Some(spec) = src.level(&path) else {
                self.truncate(depth, src);
                break;
            };
            self.init_level(depth, &spec);
            let rect = self.draw_level(rt, depth, &spec, placement, origin, options, screen, buf);
            rt.hits
                .add(rect, crate::Id::new("popup").with(depth as u64));
            origin = self.levels[depth]
                .open_row
                .and_then(|row| {
                    self.levels[depth]
                        .row_rects
                        .iter()
                        .find(|(r, _)| *r == row)
                        .map(|(_, r)| *r)
                })
                .or(Some(rect));
            depth += 1;
        }
    }

    fn init_level(&mut self, depth: usize, spec: &LevelSpec) {
        let level = &mut self.levels[depth];
        if !level.initialised {
            level.initialised = true;
            if level.select_first {
                let visible = visible_rows(spec, &level.query);
                level.selected = spec
                    .default_index
                    .filter(|r| *r < spec.rows.len() && spec.rows[*r].selectable)
                    .or_else(|| first_selectable(spec, &visible));
                level.scroll_to = true;
            }
        }
        let visible = visible_rows(spec, &level.query);
        if level.selected.is_some_and(|r| {
            r >= spec.rows.len() || !visible.contains(&r) || !spec.rows[r].selectable
        }) {
            level.selected = None;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_level(
        &mut self,
        rt: &mut Runtime,
        depth: usize,
        spec: &LevelSpec,
        placement: Placement,
        origin: Option<Rect>,
        options: &PopupOptions,
        screen: Rect,
        buf: &mut Buffer,
    ) -> Rect {
        let style = options.style;
        let colors = surface(&rt.look, style);
        let p = rt.look.palette;
        let level = &mut self.levels[depth];
        let visible = visible_rows(spec, &level.query);
        let telescope = style == PopupStyle::Telescope;
        let show_prompt = spec.speed_search && (telescope || !level.query.is_empty());
        let show_title = !telescope && spec.title.is_some()
            || (telescope && !show_prompt && spec.title.is_some());
        let header = u16::from(show_prompt) + u16::from(show_title);
        // Lines: separators and rows.
        let mut lines = Vec::new();
        for &row in &visible {
            if spec.rows[row].separator.is_some() && !lines.is_empty() {
                lines.push(Line::Separator(row));
            }
            lines.push(Line::Row(row));
        }
        // Width.
        let rows_iter = || visible.iter().map(|r| &spec.rows[*r]);
        let text_w = rows_iter().map(|r| text_width(&r.text)).max().unwrap_or(0);
        let check_w = if rows_iter().any(|r| r.checked.is_some()) {
            2
        } else {
            0
        };
        let icon_w = if rows_iter().any(|r| r.icon.is_some()) {
            2
        } else {
            0
        };
        let mnem_w = if spec.mnemonics { 3 } else { 0 };
        let sc_w = rows_iter()
            .filter_map(|r| r.shortcut.as_deref())
            .map(text_width)
            .max()
            .unwrap_or(0);
        let chevron_w = if rows_iter().any(|r| r.sub) { 2 } else { 0 };
        let title_w = spec.title.as_deref().map_or(0, |t| text_width(t) + 4);
        let content = 2
            + check_w
            + icon_w
            + mnem_w
            + text_w
            + if sc_w > 0 { 3 + sc_w } else { 0 }
            + chevron_w;
        let min_w = if telescope { 44 } else { 18 };
        let query_w = text_width(&level.query) + 8;
        let anchor_w = match (depth, placement) {
            (0, Placement::Below(anchor)) => anchor.width,
            _ => 0,
        };
        let width = content
            .max(min_w)
            .max(anchor_w)
            .max(
                title_w
                    + if show_prompt {
                        text_width(&level.query) + 6
                    } else {
                        0
                    },
            )
            .max(query_w)
            .min(screen.width.max(1));
        // Height.
        let max_body = screen.height.saturating_sub(header + 1).max(1);
        let want_body = lines.len().max(1) as u16;
        let body = if telescope {
            want_body.min(max_body.min(14))
        } else {
            want_body.min(max_body)
        };
        let height = (header + body).min(screen.height.max(1));
        // Position.
        let (mut x, mut y) = match (depth, origin) {
            (0, _) => match placement {
                Placement::Below(anchor) => {
                    let below = anchor.bottom();
                    if below + height <= screen.bottom() || anchor.y < height {
                        (anchor.x, below)
                    } else {
                        (anchor.x, anchor.y - height)
                    }
                }
                Placement::At(px, py) => (px, py),
                Placement::Center => (
                    screen.x + screen.width.saturating_sub(width) / 2,
                    screen.y + (screen.height.saturating_sub(height) / 2).saturating_sub(1),
                ),
            },
            (_, Some(row)) => {
                let right = row.right();
                if right + width <= screen.right() {
                    (right, row.y.saturating_sub(header))
                } else {
                    (row.x.saturating_sub(width), row.y.saturating_sub(header))
                }
            }
            (_, None) => (screen.x, screen.y),
        };
        x = x.min(screen.right().saturating_sub(width)).max(screen.x);
        y = y.min(screen.bottom().saturating_sub(height)).max(screen.y);
        let rect = Rect::new(x, y, width, height);
        level.rect = rect;
        level.page = body as usize;
        // Scroll.
        let selected_line = level.selected.and_then(|s| {
            lines
                .iter()
                .position(|l| matches!(l, Line::Row(r) if *r == s))
        });
        let max_scroll = lines.len().saturating_sub(body as usize);
        level.scroll = level.scroll.min(max_scroll);
        if level.scroll_to {
            level.scroll_to = false;
            if let Some(line) = selected_line {
                if line < level.scroll {
                    level.scroll = line;
                } else if line >= level.scroll + body as usize {
                    level.scroll = line + 1 - body as usize;
                }
            }
        }
        // Paint.
        fill_rect(buf, rect, Style::default().bg(colors.bg).fg(colors.fg));
        let mut y = rect.y;
        if show_prompt {
            let row = Rect::new(rect.x, y, rect.width, 1);
            fill(buf, row, row.x, y, row.width, Style::default().bg(p.black2));
            let mut cx = rect.x;
            if let Some(title) = &spec.title {
                cx = crate::components::title_block(buf, row, cx, y, title, p.on(p.red), p.red) + 1;
            }
            cx = put(
                buf,
                row,
                cx,
                y,
                &rt.look.glyph(crate::Glyph::Search),
                Style::default().fg(p.nord_blue).bg(p.black2),
            ) + 1;
            let end = put(
                buf,
                row,
                cx,
                y,
                &level.query,
                Style::default().fg(p.white).bg(p.black2),
            );
            put(
                buf,
                row,
                end,
                y,
                " ",
                Style::default()
                    .bg(p.white)
                    .fg(p.black2)
                    .add_modifier(Modifier::empty()),
            );
            y += 1;
        } else if show_title && let Some(title) = &spec.title {
            let row = Rect::new(rect.x, y, rect.width, 1);
            put(
                buf,
                row,
                rect.x + 1,
                y,
                title,
                Style::default()
                    .fg(colors.muted)
                    .bg(colors.bg)
                    .add_modifier(Modifier::BOLD),
            );
            y += 1;
        }
        level.row_rects.clear();
        let comparator = SpeedSearchComparator::new(&level.query);
        for (cy, line) in (y..).zip(lines.iter().skip(level.scroll).take(body as usize)) {
            let row_rect = Rect::new(rect.x, cy, rect.width, 1);
            match *line {
                Line::Separator(row) => {
                    let caption = spec.rows[row].separator.clone().flatten();
                    let line_style = Style::default().fg(p.line).bg(colors.bg);
                    for sx in rect.x + 1..rect.right().saturating_sub(1) {
                        put(buf, row_rect, sx, cy, "─", line_style);
                    }
                    if let Some(caption) = caption {
                        put(
                            buf,
                            row_rect,
                            rect.x + 2,
                            cy,
                            &format!(" {caption} "),
                            Style::default().fg(colors.muted).bg(colors.bg),
                        );
                    }
                }
                Line::Row(row) => {
                    let r = &spec.rows[row];
                    let selected = level.selected == Some(row);
                    let open = level.open_row == Some(row);
                    let (bg, fg) = if selected || open {
                        (colors.selected_bg, colors.selected_fg)
                    } else if r.selectable {
                        (colors.bg, colors.fg)
                    } else {
                        (colors.bg, colors.disabled)
                    };
                    let base = Style::default().bg(bg).fg(fg);
                    fill(buf, row_rect, row_rect.x, cy, row_rect.width, base);
                    let mut x = rect.x + 1;
                    if check_w > 0 {
                        if r.checked == Some(true) {
                            put(
                                buf,
                                row_rect,
                                x,
                                cy,
                                &rt.look.glyph(crate::Glyph::Check),
                                base.fg(if selected { fg } else { p.green }),
                            );
                        }
                        x += 2;
                    }
                    if spec.mnemonics {
                        if let Some(m) = r.mnemonic {
                            put(
                                buf,
                                row_rect,
                                x,
                                cy,
                                &format!("{m}"),
                                base.fg(if selected { fg } else { colors.muted })
                                    .add_modifier(Modifier::UNDERLINED),
                            );
                        }
                        x += 3;
                    }
                    if let Some(icon) = r.icon {
                        let (ch, color) = icon.char_and_color(&rt.look);
                        let color = if selected || open || !r.selectable {
                            fg
                        } else {
                            color
                        };
                        put(buf, row_rect, x, cy, &ch.to_string(), base.fg(color));
                    }
                    x += icon_w;
                    let fragments = if comparator.is_empty() {
                        Vec::new()
                    } else {
                        comparator.matching_fragments(&r.text).unwrap_or_default()
                    };
                    let mut cx = x;
                    for (i, ch) in r.text.chars().enumerate() {
                        let hit = fragments.iter().any(|f| f.contains(&i));
                        let st = if hit {
                            base.fg(if selected { fg } else { p.orange })
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                        } else if selected && telescope {
                            base.add_modifier(Modifier::BOLD)
                        } else {
                            base
                        };
                        cx = put(buf, row_rect, cx, cy, ch.encode_utf8(&mut [0; 4]), st);
                    }
                    let mut right = rect.right().saturating_sub(1);
                    if chevron_w > 0 {
                        right = right.saturating_sub(2);
                        if r.sub {
                            put(
                                buf,
                                row_rect,
                                right + 1,
                                cy,
                                &rt.look.glyph(crate::Glyph::ChevronRight),
                                base,
                            );
                        }
                    }
                    if let Some(sc) = &r.shortcut {
                        let w = text_width(sc);
                        let sx = right.saturating_sub(w);
                        if sx > cx {
                            put(
                                buf,
                                row_rect,
                                sx,
                                cy,
                                sc,
                                base.fg(if selected { fg } else { colors.muted }),
                            );
                        }
                    }
                    level.row_rects.push((row, row_rect));
                }
            }
        }
        if lines.is_empty() {
            put(
                buf,
                rect,
                rect.x + 1,
                y,
                "No matches",
                Style::default().fg(colors.muted).bg(colors.bg),
            );
        }
        // Scroll thumb.
        if lines.len() > body as usize && body > 0 && rect.width > 2 {
            let h = body as usize;
            let thumb = (h * h / lines.len()).max(1);
            let top = level.scroll * (h - thumb) / max_scroll.max(1);
            for i in 0..thumb {
                put(
                    buf,
                    rect,
                    rect.right() - 1,
                    y + (top + i) as u16,
                    "▐",
                    Style::default().fg(p.grey).bg(colors.bg),
                );
            }
        }
        rect
    }
}

// ----- sources -----

/// A [`PopupSource`] over the kit's [`PopupStep`]s: the same trait the GUI's `ListPopup` uses.
pub struct StepSource<T: Clone + 'static> {
    steps: Vec<Box<dyn PopupStep<T>>>,
    chosen: Option<T>,
}

impl<T: Clone + 'static> StepSource<T> {
    /// A source starting at `step`.
    pub fn new(step: impl PopupStep<T> + 'static) -> Self {
        Self {
            steps: vec![Box::new(step)],
            chosen: None,
        }
    }

    /// The chosen value, once the popup finished with a choice.
    pub fn take_chosen(&mut self) -> Option<T> {
        self.chosen.take()
    }
}

impl<T: Clone + 'static> PopupSource for StepSource<T> {
    fn level(&mut self, path: &[usize]) -> Option<LevelSpec> {
        let step = self.steps.get(path.len())?;
        let mut rows = Vec::new();
        for value in step.values() {
            rows.push(RowSpec {
                text: step.text_for(value),
                icon: step.icon_for(value).as_ref().and_then(icon_of),
                shortcut: step.shortcut_for(value),
                checked: None,
                selectable: step.is_selectable(value),
                separator: step
                    .separator_for(value)
                    .map(|s| s.title().map(str::to_owned)),
                sub: step.has_substep(value),
                mnemonic: step.mnemonic_for(value),
            });
        }
        Some(LevelSpec {
            rows,
            title: step.title().map(str::to_owned),
            speed_search: step.is_speed_search_enabled(),
            auto_select: step.is_auto_selection_enabled(),
            mnemonics: step.is_mnemonics_navigation_enabled(),
            default_index: Some(step.default_option_index()),
        })
    }

    fn choose(&mut self, path: &[usize], row: usize, final_choice: bool) -> Chosen {
        let depth = path.len();
        let Some(step) = self.steps.get_mut(depth) else {
            return Chosen::Stay;
        };
        let Some(value) = step.values().get(row).cloned() else {
            return Chosen::Stay;
        };
        match step.on_chosen(&value, final_choice) {
            PopupStepResult::FinalChoice => {
                self.chosen = Some(value);
                Chosen::Final
            }
            PopupStepResult::Next(next) => {
                self.steps.truncate(depth + 1);
                self.steps.push(next);
                Chosen::Opened
            }
            _ => Chosen::Stay,
        }
    }

    fn closed(&mut self, depth: usize) {
        self.steps.truncate(depth + 1);
    }

    fn cancelled(&mut self) {
        for step in &mut self.steps {
            step.canceled();
        }
    }
}

/// A [`PopupSource`] over an action group (IntelliJ `ActionPopupMenu`).
pub struct ActionMenuSource {
    group: Rc<dyn ActionGroup>,
    data: Rc<DataContext>,
    place: ActionPlace,
    keymap: Keymap,
    caps: crate::input::Caps,
    speed_search: bool,
    numbering: bool,
    performed: Option<String>,
}

impl ActionMenuSource {
    fn rows_at(&self, path: &[usize]) -> Option<Vec<ActionRow>> {
        let mac = self.caps.os.is_mac();
        let mut rows = flatten_with_shortcuts(
            self.group.as_ref(),
            &self.data,
            &self.place,
            &self.keymap,
            mac,
        );
        for &index in path {
            // Path entries count rows without separators, like the level specs.
            let group = match rows.iter().filter(|r| !r.is_separator()).nth(index)? {
                ActionRow::Group { group, .. } => group.clone(),
                _ => return None,
            };
            rows =
                flatten_with_shortcuts(group.as_ref(), &self.data, &self.place, &self.keymap, mac);
        }
        Some(rows)
    }

    /// The id of the action that ran, once the popup finished with a choice.
    pub fn take_performed(&mut self) -> Option<String> {
        self.performed.take()
    }
}

impl PopupSource for ActionMenuSource {
    fn level(&mut self, path: &[usize]) -> Option<LevelSpec> {
        let rows = self.rows_at(path)?;
        let mut specs: Vec<RowSpec> = Vec::new();
        let mut separator: Option<Option<String>> = None;
        let mut number = 0usize;
        for row in &rows {
            match row {
                ActionRow::Separator(s) => separator = Some(s.text.clone()),
                ActionRow::Action {
                    action,
                    presentation,
                } => {
                    number += 1;
                    let shortcut =
                        crate::input::display_shortcut(&self.keymap, action.id(), &self.caps)
                            .or_else(|| presentation.shortcut_text.clone());
                    specs.push(RowSpec {
                        text: presentation.text.clone(),
                        icon: presentation.icon.as_ref().and_then(icon_of),
                        shortcut,
                        checked: presentation.selected,
                        selectable: presentation.enabled,
                        separator: separator.take(),
                        sub: false,
                        mnemonic: mnemonic_for(self.numbering, number),
                    });
                }
                ActionRow::Group { presentation, .. } => {
                    number += 1;
                    specs.push(RowSpec {
                        text: presentation.text.clone(),
                        icon: presentation.icon.as_ref().and_then(icon_of),
                        shortcut: None,
                        checked: None,
                        selectable: presentation.enabled,
                        separator: separator.take(),
                        sub: true,
                        mnemonic: mnemonic_for(self.numbering, number),
                    });
                }
                _ => {}
            }
        }
        // Separators map to the rows after them; remember which data row each spec is.
        Some(LevelSpec {
            rows: specs,
            title: None,
            speed_search: self.speed_search,
            auto_select: false,
            mnemonics: self.numbering,
            default_index: None,
        })
    }

    fn choose(&mut self, path: &[usize], row: usize, _final_choice: bool) -> Chosen {
        let Some(rows) = self.rows_at(path) else {
            return Chosen::Stay;
        };
        // `row` indexes the specs, which skip separators.
        let Some(item) = rows.iter().filter(|r| !r.is_separator()).nth(row) else {
            return Chosen::Stay;
        };
        match item {
            ActionRow::Group { .. } => Chosen::Opened,
            ActionRow::Action {
                action,
                presentation,
            } => {
                let ran = perform_with_modifiers(
                    action.as_ref(),
                    &self.data,
                    &self.place,
                    InputKind::Keyboard,
                    archipel_kit::egui::Modifiers::NONE,
                );
                if ran {
                    self.performed = Some(action.id().to_owned());
                }
                if presentation.keep_popup_on_perform {
                    Chosen::Stay
                } else {
                    Chosen::Final
                }
            }
            _ => Chosen::Stay,
        }
    }
}

fn mnemonic_for(numbering: bool, number: usize) -> Option<char> {
    if !numbering {
        return None;
    }
    match number {
        1..=9 => char::from_digit(number as u32, 10),
        10 => Some('0'),
        11..=36 => char::from_u32('A' as u32 + (number as u32 - 11)),
        _ => None,
    }
}

// ----- hosts -----

/// How a popup ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupState {
    /// Still open (or closed and untouched).
    Open,
    /// Finished with a choice.
    Chosen,
    /// Cancelled.
    Cancelled,
}

/// A popup: the chain, its source and where it opens.
pub struct PopupHost<S: PopupSource> {
    /// The model.
    pub source: S,
    /// The levels, selection and timers.
    pub chain: PopupChain,
    /// Where level 0 opens.
    pub placement: Placement,
    /// Behavior and look.
    pub options: PopupOptions,
    result: PopupState,
}

impl<S: PopupSource> PopupHost<S> {
    fn new(source: S, placement: Placement, options: PopupOptions) -> Self {
        Self {
            source,
            chain: PopupChain::default(),
            placement,
            options,
            result: PopupState::Open,
        }
    }

    /// Opens the popup (keyboard opened: the first row is selected).
    pub fn open(&mut self, rt: &Runtime, by_keyboard: bool) -> bool {
        self.result = PopupState::Open;
        self.chain.open(rt.now, by_keyboard)
    }

    /// Whether the popup is open.
    pub fn is_open(&self) -> bool {
        self.chain.is_open()
    }

    /// How the popup ended (`Open` while it is open).
    pub fn state(&self) -> PopupState {
        self.result
    }

    /// Applies `event`.
    pub fn event(&mut self, rt: &mut Runtime, event: &Event) -> PopupState {
        let out = self.chain.event(rt, &mut self.source, event, &self.options);
        if out.chosen.is_some() {
            self.result = PopupState::Chosen;
        } else if out.cancelled {
            self.result = PopupState::Cancelled;
        }
        self.result
    }

    /// Runs the timers.
    pub fn tick(&mut self, rt: &Runtime) {
        self.chain.tick(rt, &mut self.source);
    }

    /// The next time [`tick`](Self::tick) has something to do.
    pub fn deadline(&self) -> Option<f64> {
        self.chain.deadline()
    }

    /// Draws the popup.
    pub fn draw(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        self.chain
            .draw(rt, &mut self.source, self.placement, &self.options, buf);
    }
}

/// A list of values that chooses one: the TUI counterpart of the kit's `ListPopup`. It takes
/// the same [`PopupStep`]s.
pub type ListPopup<T> = PopupHost<StepSource<T>>;

impl<T: Clone + 'static> PopupHost<StepSource<T>> {
    /// A Telescope-style popup (a centered float) for `step`.
    pub fn list(step: impl PopupStep<T> + 'static) -> Self {
        Self::new(
            StepSource::new(step),
            Placement::Center,
            PopupOptions {
                style: PopupStyle::Telescope,
                ..PopupOptions::default()
            },
        )
    }

    /// A completion-menu popup below `anchor` for `step` (a combo box list).
    pub fn menu_below(step: impl PopupStep<T> + 'static, anchor: Rect) -> Self {
        Self::new(
            StepSource::new(step),
            Placement::Below(anchor),
            PopupOptions::default(),
        )
    }

    /// Sets the placement.
    #[must_use]
    pub fn at(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the look.
    #[must_use]
    pub fn style(mut self, style: PopupStyle) -> Self {
        self.options.style = style;
        self
    }

    /// Opens sub-steps on hover (action popups do).
    #[must_use]
    pub fn show_submenu_on_hover(mut self, on: bool) -> Self {
        self.options.show_submenu_on_hover = on;
        self
    }

    /// The chosen value.
    pub fn take_chosen(&mut self) -> Option<T> {
        self.source.take_chosen()
    }
}

/// An action group as a popup menu (IntelliJ `ActionPopupMenu`): context menus and
/// drop-downs. Choosing a row performs its action through the action system.
pub type ActionPopupMenu = PopupHost<ActionMenuSource>;

impl PopupHost<ActionMenuSource> {
    /// A menu of `group` for `place`, with the data of `data`, anchored at `placement`.
    /// Shortcuts come from `keymap`; submenus open on hover.
    pub fn actions(
        group: Rc<dyn ActionGroup>,
        data: Rc<DataContext>,
        place: ActionPlace,
        keymap: Keymap,
        caps: crate::input::Caps,
        placement: Placement,
    ) -> Self {
        Self::new(
            ActionMenuSource {
                group,
                data,
                place,
                keymap,
                caps,
                speed_search: false,
                numbering: false,
                performed: None,
            },
            placement,
            PopupOptions {
                show_submenu_on_hover: true,
                ..PopupOptions::default()
            },
        )
    }

    /// Turns the speed search on (typing filters the menu).
    #[must_use]
    pub fn speed_search(mut self, on: bool) -> Self {
        self.source.speed_search = on;
        self
    }

    /// Numbers the rows 1..9, 0, A..Z; pressing the number chooses the row.
    #[must_use]
    pub fn numbering(mut self, on: bool) -> Self {
        self.source.numbering = on;
        self
    }

    /// The id of the action that ran.
    pub fn take_performed(&mut self) -> Option<String> {
        self.source.take_performed()
    }
}
