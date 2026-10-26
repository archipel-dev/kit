//! The engine behind [`ListPopup`](super::ListPopup) and
//! [`ActionPopupMenu`](crate::action_system::ActionPopupMenu): a chain of list levels with one
//! shared set of keyboard, pointer and timing rules (IntelliJ spec §5).
//!
//! The engine is immediate mode. A caller-owned [`PopupChain`] holds what has to survive
//! between frames (selection per level, which row opened a child, timers); the rows come from
//! a [`PopupSource`] asked every frame, so action presentations stay current.

use super::ListSeparator;
use crate::action_system::ActionMenuItem;
use crate::icons::Icon;
use crate::parts::{Surface, scroll_vertical};
use crate::theme::Theme;
use crate::tokens::{motion, popup, size, typography};
use egui::{Context, Event, Id, Key, Modifiers, Order, Pos2, Rect, Sense, Ui, pos2, vec2};
use std::time::Duration;

/// One row of a level.
#[derive(Debug, Clone, Default)]
pub(crate) struct RowSpec {
    pub text: String,
    pub icon: Option<Icon>,
    pub shortcut: Option<String>,
    pub checked: Option<bool>,
    pub description: Option<String>,
    /// Enabled and selectable (disabled rows and group titles are skipped by the keys).
    pub selectable: bool,
    /// A separator drawn before this row: `Some(None)` a line, `Some(Some(title))` a titled one.
    pub separator: Option<Option<String>>,
    /// The row has a sub-step (chevron; Right/Enter open it).
    pub sub: bool,
    pub mnemonic: Option<char>,
    /// Choosing it leaves the popup open.
    pub keep_open: bool,
    /// Colors the row's text by meaning (combo boxes of HTTP methods).
    pub tone: Option<crate::parts::Tone>,
}

/// What a source says about a level.
#[derive(Debug, Clone, Default)]
pub(crate) struct LevelSpec {
    pub rows: Vec<RowSpec>,
    pub title: Option<String>,
    pub speed_search: bool,
    pub auto_select: bool,
    pub mnemonics: bool,
    /// Row selected when the level opens with a selection.
    pub default_index: Option<usize>,
}

/// The result of choosing a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Chosen {
    /// The popup is done.
    Final,
    /// A sub-step exists now: open a child level.
    Opened,
    /// Nothing more happens; the popup stays.
    Stay,
}

/// Where the rows come from.
pub(crate) trait PopupSource {
    /// The rows of level `depth`, reached through the rows `path` chose; `None` when the
    /// level no longer exists.
    fn level(&mut self, depth: usize, path: &[usize]) -> Option<LevelSpec>;
    /// Chooses `row` of level `depth`. `final_choice` is false for a row with a sub-step
    /// (and for auto-selection), which opens it.
    fn choose(&mut self, depth: usize, path: &[usize], row: usize, final_choice: bool) -> Chosen;
}

#[derive(Debug, Clone)]
struct Level {
    /// Selected row (index into the level's rows).
    selected: Option<usize>,
    /// The row whose child level is open.
    open_row: Option<usize>,
    query: String,
    /// A row that opens its child when the hover delay passes: (row, since).
    pending: Option<(usize, f64)>,
    /// The last time the pointer moved over this level or the selection changed.
    idle_since: f64,
    auto_fired: bool,
    rect: Rect,
    row_rects: Vec<(usize, Rect)>,
    scroll_to: bool,
    /// Select the first row once the rows are known.
    select_first: bool,
    initialised: bool,
    last_height: f32,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            selected: None,
            open_row: None,
            query: String::new(),
            pending: None,
            idle_since: 0.0,
            auto_fired: false,
            rect: Rect::ZERO,
            row_rects: Vec::new(),
            scroll_to: false,
            select_first: false,
            initialised: false,
            last_height: 0.0,
        }
    }
}

/// The state of an open popup chain, owned by the caller.
#[derive(Debug, Clone, Default)]
pub struct PopupChain {
    levels: Vec<Level>,
    open: bool,
    closed_at: Option<f64>,
    last_pointer: Option<Pos2>,
    prev_pointer: Option<Pos2>,
    moves: u32,
    away: f32,
    started: bool,
    preselect: bool,
}

impl PopupChain {
    /// Whether the popup is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the popup at `now` (the context's input time). Refused for
    /// [`POPUP_REOPEN_GUARD`](motion::POPUP_REOPEN_GUARD) after it closed, so the click that
    /// closed it by pressing its owner button does not reopen it on release. Returns whether
    /// it opened. `preselect` selects the first row at once (list popups); menus select
    /// nothing until a key or the pointer moves.
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

    /// Forgets when the popup last closed, so the reopen guard no longer applies: for owners
    /// that keep the guard per button themselves.
    pub fn forget_close(&mut self) {
        self.closed_at = None;
    }

    /// Closes the popup at `now`.
    pub fn close(&mut self, now: f64) {
        if self.open {
            self.closed_at = Some(now);
        }
        self.open = false;
        self.levels.clear();
    }

    /// How many levels are open (1 for a plain list).
    pub fn depth(&self) -> usize {
        self.levels.len()
    }

    /// The selected row of level `depth` (an index into the level's rows).
    pub fn selected(&self, depth: usize) -> Option<usize> {
        self.levels.get(depth).and_then(|level| level.selected)
    }

    /// The row whose child level is open at `depth`.
    pub fn open_row(&self, depth: usize) -> Option<usize> {
        self.levels.get(depth).and_then(|level| level.open_row)
    }

    /// The speed search text of level `depth`.
    pub fn query(&self, depth: usize) -> &str {
        self.levels
            .get(depth)
            .map_or("", |level| level.query.as_str())
    }

    /// The screen rectangle of level `depth` in the last frame.
    pub fn rect(&self, depth: usize) -> Option<Rect> {
        self.levels.get(depth).map(|level| level.rect)
    }

    /// Cuts the chain back to `depth` levels (a step closed itself).
    pub(crate) fn truncate(&mut self, depth: usize) {
        self.levels.truncate(depth);
        if let Some(last) = self.levels.last_mut() {
            last.open_row = None;
        }
    }
}

/// Where the first level goes.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Placement {
    /// Below (or above, when there is no room) a widget.
    Below(Rect),
    /// With its top-left corner at a point (context menus).
    At(Pos2),
    /// A combo box's list: below (or above) the combo, at least as wide as it, showing at
    /// most `max_rows` rows before it scrolls (IntelliJ spec section 4).
    Combo { anchor: Rect, max_rows: usize },
}

/// How the chain behaves.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Options {
    /// Submenus open when the pointer rests on their row for 250 ms (action button popups).
    pub show_submenu_on_hover: bool,
    /// A press outside every level closes the chain.
    pub cancel_on_click_outside: bool,
}

/// What a frame of the chain produced.
#[derive(Debug, Clone, Default)]
pub(crate) struct Output {
    /// The final choice: the rows chosen from the first level down to the last.
    pub chosen: Option<Vec<usize>>,
    /// The chain closed without a final choice.
    pub cancelled: bool,
}

fn visible_rows(spec: &LevelSpec, query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    spec.rows
        .iter()
        .enumerate()
        .filter(|(_, row)| query.is_empty() || row.text.to_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect()
}

fn first_selectable(spec: &LevelSpec, visible: &[usize]) -> Option<usize> {
    visible
        .iter()
        .copied()
        .find(|&row| spec.rows[row].selectable)
}

fn path_of(chain: &PopupChain, depth: usize) -> Vec<usize> {
    chain.levels[..depth]
        .iter()
        .filter_map(|level| level.open_row)
        .collect()
}

/// Moves the selection `by` steps among selectable visible rows, wrapping around (`wrap`) or
/// stopping at the ends.
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
        .filter(|&row| spec.rows[row].selectable)
        .collect();
    if selectable.is_empty() {
        return None;
    }
    let position = current.and_then(|row| selectable.iter().position(|&r| r == row));
    let len = selectable.len() as isize;
    let next = match position {
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

fn in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let sign = |p1: Pos2, p2: Pos2, p3: Pos2| {
        (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
    };
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// Runs one frame of the chain: handles input, draws every level, and reports a final
/// choice or a cancel.
pub(crate) fn drive(
    ctx: &Context,
    id: Id,
    placement: Placement,
    options: Options,
    chain: &mut PopupChain,
    source: &mut dyn PopupSource,
) -> Output {
    let mut output = Output::default();
    if !chain.open {
        return output;
    }
    let now = ctx.input(|i| i.time);
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    if !chain.started {
        chain.started = true;
        chain.last_pointer = pointer;
        chain.prev_pointer = pointer;
        chain.levels.push(Level {
            select_first: chain.preselect,
            idle_since: now,
            ..Level::default()
        });
    }
    // Pointer movement: the first move after opening is ignored (a menu that opens under the
    // pointer highlights nothing until the pointer really moves); frames where the point
    // did not change are not moves.
    let moved = pointer.is_some() && pointer != chain.last_pointer;
    let prev_pointer = chain.last_pointer;
    let mut real_move = false;
    if moved {
        chain.moves += 1;
        real_move = chain.moves >= 2;
        chain.prev_pointer = chain.last_pointer;
        chain.last_pointer = pointer;
    }

    // Presses outside every level cancel the chain.
    let press = ctx.input(|i| {
        i.pointer
            .button_pressed(egui::PointerButton::Primary)
            .then(|| i.pointer.interact_pos())
            .flatten()
    });
    if options.cancel_on_click_outside
        && let Some(p) = press
        && chain.levels.iter().all(|level| !level.rect.contains(p))
        && chain.levels.iter().any(|level| level.rect.is_positive())
    {
        chain.close(now);
        output.cancelled = true;
        return output;
    }

    let keys_taken = handle_keys(ctx, chain, source, &mut output, now);
    if !chain.open {
        return output;
    }
    let _ = keys_taken;

    let screen = ctx.content_rect();
    let mut depth = 0;
    while depth < chain.levels.len() {
        let path = path_of(chain, depth);
        let Some(spec) = source.level(depth, &path) else {
            chain.truncate(depth);
            break;
        };
        let visible = visible_rows(&spec, &chain.levels[depth].query);
        // Initial selection.
        {
            let level = &mut chain.levels[depth];
            if !level.initialised {
                level.initialised = true;
                if level.select_first {
                    level.selected = spec
                        .default_index
                        .filter(|&row| row < spec.rows.len() && spec.rows[row].selectable)
                        .or_else(|| first_selectable(&spec, &visible));
                    level.scroll_to = true;
                }
            }
            if level.selected.is_some_and(|row| {
                row >= spec.rows.len() || !visible.contains(&row) || !spec.rows[row].selectable
            }) {
                level.selected = None;
            }
        }

        // Pointer: hover selects the selectable row under it, or clears the selection.
        if real_move
            && let Some(p) = pointer
            && chain.levels[depth].rect.contains(p)
        {
            let hit = chain.levels[depth]
                .row_rects
                .iter()
                .find(|(_, rect)| rect.contains(p))
                .map(|(row, _)| *row);
            let child_rect = chain.levels.get(depth + 1).map(|l| l.rect);
            let moving_to_child = child_rect.is_some_and(|child| {
                let near = if child.left() >= chain.levels[depth].rect.center().x {
                    child.left()
                } else {
                    child.right()
                };
                let from = chain.prev_pointer.unwrap_or(p);
                let dx = p.x - from.x;
                let toward = (near - from.x).signum() == dx.signum() || dx == 0.0;
                if toward {
                    chain.away = 0.0;
                } else {
                    chain.away += dx.abs();
                }
                in_triangle(p, from, pos2(near, child.top()), pos2(near, child.bottom()))
                    && (toward || chain.away <= popup::SUBMENU_AWAY)
            });
            let level = &mut chain.levels[depth];
            level.idle_since = now;
            level.auto_fired = false;
            if !moving_to_child {
                let target = hit.filter(|&row| spec.rows[row].selectable);
                if level.selected != target {
                    level.selected = target;
                    level.pending = None;
                }
                if level.open_row.is_some() && level.open_row != target {
                    level.open_row = None;
                    chain.levels.truncate(depth + 1);
                }
                let level = &mut chain.levels[depth];
                if let Some(row) = target
                    && spec.rows[row].sub
                    && options.show_submenu_on_hover
                    && level.open_row != Some(row)
                    && level.pending.map(|(r, _)| r) != Some(row)
                {
                    level.pending = Some((row, now));
                }
            }
        }
        let _ = prev_pointer;

        // Timers: the submenu delay and the auto-selection delay.
        let delay = f64::from(motion::POPUP_SUBMENU_DELAY);
        let mut open_child = None;
        if let Some((row, since)) = chain.levels[depth].pending {
            if chain.levels[depth].selected != Some(row) {
                chain.levels[depth].pending = None;
            } else if now - since >= delay {
                chain.levels[depth].pending = None;
                open_child = Some((row, false));
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(delay - (now - since)));
            }
        }
        if spec.auto_select {
            let wait = f64::from(motion::POPUP_AUTO_SELECTION_DELAY);
            let level = &chain.levels[depth];
            if let Some(row) = level.selected
                && !level.auto_fired
                && level.open_row.is_none()
                && spec.rows[row].sub
            {
                if now - level.idle_since >= wait {
                    open_child = Some((row, false));
                    chain.levels[depth].auto_fired = true;
                } else {
                    ctx.request_repaint_after(Duration::from_secs_f64(
                        wait - (now - level.idle_since),
                    ));
                }
            }
        }
        if let Some((row, by_key)) = open_child {
            open_level_child(chain, source, depth, row, by_key);
        }

        // Draw.
        let colors = Theme::colors(ctx);
        let level_id = id.with(("level", depth));
        let pos = level_position(ctx, level_id, depth, chain, placement, screen);
        let max_height = match placement {
            Placement::Combo { max_rows, .. } if depth == 0 => max_rows as f32 * size::ROW_HEIGHT,
            _ => (screen.bottom() - pos.y - popup::PADDING).max(160.0),
        };
        let min_width = match placement {
            Placement::Combo { anchor, .. } if depth == 0 => anchor.width(),
            _ => popup::MIN_WIDTH,
        };
        let selected = chain.levels[depth].selected;
        let scroll_to = std::mem::take(&mut chain.levels[depth].scroll_to);
        let reserve_icons = spec
            .rows
            .iter()
            .any(|row| row.icon.is_some() || row.checked.is_some());
        let mut clicked: Option<usize> = None;
        let mut chevron_pressed: Option<usize> = None;
        let mut row_rects = Vec::new();
        let area = egui::Area::new(level_id)
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                Surface::Popup.frame(ctx).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mnemonic_column =
                        if spec.mnemonics || spec.rows.iter().any(|r| r.mnemonic.is_some()) {
                            let font = typography::body(typography::BODY);
                            spec.rows
                                .iter()
                                .filter_map(|r| r.mnemonic)
                                .map(|c| {
                                    crate::common::galley(ui, c.to_string(), font.clone())
                                        .size()
                                        .x
                                })
                                .fold(0.0_f32, f32::max)
                                + popup::MNEMONIC_GAP
                        } else {
                            0.0
                        };
                    let items: Vec<(usize, ActionMenuItem<'_>)> = visible
                        .iter()
                        .map(|&row_index| {
                            let row = &spec.rows[row_index];
                            let mut item = ActionMenuItem::new(row.text.clone())
                                .enabled(row.selectable)
                                .managed(true)
                                .arrow(row.sub)
                                .reserve_icons(reserve_icons)
                                .mnemonic(row.mnemonic, mnemonic_column)
                                .highlighted(selected == Some(row_index));
                            if let Some(icon) = &row.icon {
                                item = item.icon(*icon);
                            }
                            if let Some(shortcut) = &row.shortcut {
                                item = item.shortcut(shortcut.clone());
                            }
                            if let Some(tone) = row.tone {
                                item = item.label_tone(tone);
                            }
                            if let Some(checked) = row.checked {
                                item = item.checked(checked);
                            }
                            if let Some(description) = &row.description {
                                item = item.description(description.clone());
                            }
                            (row_index, item)
                        })
                        .collect();
                    let width = items
                        .iter()
                        .map(|(_, item)| item.natural_width(ui))
                        .fold(min_width, f32::max);
                    ui.set_width(width);
                    if let Some(title) = &spec.title {
                        ui.add(ListSeparator::with_text(title.clone()));
                    }
                    if !chain.levels[depth].query.is_empty() {
                        let (rect, _) = ui
                            .allocate_exact_size(vec2(width, popup::SEARCH_HEIGHT), Sense::hover());
                        let galley = crate::common::galley(
                            ui,
                            chain.levels[depth].query.as_str(),
                            typography::body(typography::BODY),
                        );
                        crate::common::text_left(
                            ui.painter(),
                            rect,
                            rect.left() + popup::ITEM_INSET + popup::ITEM_PADDING,
                            galley,
                            colors.text,
                        );
                    }
                    let filtering = !chain.levels[depth].query.is_empty();
                    let body = |ui: &mut Ui| {
                        let mut first = true;
                        for (row_index, item) in items {
                            let row = &spec.rows[row_index];
                            if let Some(separator) = &row.separator
                                && !filtering
                            {
                                match separator {
                                    None if !first => {
                                        ui.add(ListSeparator::new());
                                    }
                                    Some(title) => {
                                        ui.add(ListSeparator::with_text(title.clone()));
                                    }
                                    None => {}
                                }
                            }
                            first = false;
                            let item = item.fixed_width(width);
                            let response = ui.add(item);
                            row_rects.push((row_index, response.rect));
                            if scroll_to && selected == Some(row_index) {
                                ui.scroll_to_rect(response.rect, None);
                            }
                            if row.selectable {
                                if response.clicked() {
                                    clicked = Some(row_index);
                                }
                                if row.sub
                                    && let Some(p) = press
                                    && response.rect.contains(p)
                                    && p.x
                                        >= response.rect.right()
                                            - popup::ITEM_INSET
                                            - popup::CHEVRON_HIT
                                {
                                    chevron_pressed = Some(row_index);
                                }
                            }
                        }
                    };
                    scroll_vertical()
                        .max_height(max_height)
                        .auto_shrink([false, true])
                        .show(ui, body);
                });
            });
        let rect = area.response.rect;
        // The popup's own node is a menu, not an anonymous interactive area.
        ctx.accesskit_node_builder(area.response.id, |node| {
            node.set_role(egui::accesskit::Role::Menu);
        });
        {
            let level = &mut chain.levels[depth];
            level.rect = rect;
            level.row_rects = row_rects;
            level.last_height = rect.height();
        }
        ctx.data_mut(|d| d.insert_temp(level_id, rect));

        // Clicks.
        if let Some(row) = chevron_pressed {
            chain.levels[depth].selected = Some(row);
            if chain.levels[depth].open_row != Some(row) {
                open_level_child(chain, source, depth, row, true);
            }
        } else if let Some(row) = clicked {
            chain.levels[depth].selected = Some(row);
            activate(chain, source, depth, row, &spec, &mut output, now);
            if !chain.open {
                return output;
            }
        }
        depth += 1;
    }
    output
}

fn level_position(
    ctx: &Context,
    level_id: Id,
    depth: usize,
    chain: &PopupChain,
    placement: Placement,
    screen: Rect,
) -> Pos2 {
    let height = chain.levels[depth].last_height;
    let width = ctx
        .data(|d| d.get_temp::<Rect>(level_id))
        .map_or(popup::MIN_WIDTH, |r| r.width());
    if depth == 0 {
        return match placement {
            Placement::Below(anchor) | Placement::Combo { anchor, .. } => {
                let below = anchor.left_bottom() + vec2(0.0, size::POPUP_PADDING);
                let x = below.x.min((screen.right() - width).max(screen.left()));
                if below.y + height > screen.bottom() && anchor.top() > height {
                    pos2(x, anchor.top() - height - size::POPUP_PADDING)
                } else {
                    pos2(x, below.y)
                }
            }
            Placement::At(p) => {
                let x = if p.x + width > screen.right() {
                    (p.x - width).max(screen.left())
                } else {
                    p.x
                };
                let y = if p.y + height > screen.bottom() {
                    (screen.bottom() - height).max(screen.top())
                } else {
                    p.y
                };
                pos2(x, y)
            }
        };
    }
    let parent = &chain.levels[depth - 1];
    let row = parent
        .open_row
        .and_then(|open| parent.row_rects.iter().find(|(r, _)| *r == open))
        .map(|(_, rect)| *rect);
    let anchor_top = row.map_or(parent.rect.top(), |r| r.top());
    let x = if parent.rect.right() + width > screen.right() && parent.rect.left() > width {
        parent.rect.left() - width
    } else {
        parent.rect.right()
    };
    // The popup's own padding: line its first row up with the parent row.
    let y = (anchor_top - popup::PADDING).min((screen.bottom() - height).max(screen.top()));
    pos2(x, y)
}

/// Opens the child level of `row`, if the source has one.
fn open_level_child(
    chain: &mut PopupChain,
    source: &mut dyn PopupSource,
    depth: usize,
    row: usize,
    select_first: bool,
) {
    let path = path_of(chain, depth);
    if source.choose(depth, &path, row, false) == Chosen::Opened {
        chain.levels.truncate(depth + 1);
        chain.levels[depth].open_row = Some(row);
        chain.levels.push(Level {
            select_first,
            idle_since: chain.levels[depth].idle_since,
            ..Level::default()
        });
    }
}

/// Chooses `row`: opens its sub-step, or finishes.
fn activate(
    chain: &mut PopupChain,
    source: &mut dyn PopupSource,
    depth: usize,
    row: usize,
    spec: &LevelSpec,
    output: &mut Output,
    now: f64,
) {
    let Some(spec_row) = spec.rows.get(row) else {
        return;
    };
    if !spec_row.selectable {
        return;
    }
    if spec_row.sub {
        if chain.levels[depth].open_row != Some(row) {
            open_level_child(chain, source, depth, row, true);
        }
        return;
    }
    let path = path_of(chain, depth);
    match source.choose(depth, &path, row, true) {
        Chosen::Final => {
            let mut full = path;
            full.push(row);
            if spec_row.keep_open {
                output.chosen = Some(full);
            } else {
                chain.close(now);
                output.chosen = Some(full);
            }
        }
        Chosen::Opened => {
            chain.levels.truncate(depth + 1);
            chain.levels[depth].open_row = Some(row);
            chain.levels.push(Level {
                select_first: true,
                idle_since: now,
                ..Level::default()
            });
        }
        Chosen::Stay => {}
    }
}

/// Keyboard handling for the deepest level. Returns whether a key was used.
fn handle_keys(
    ctx: &Context,
    chain: &mut PopupChain,
    source: &mut dyn PopupSource,
    output: &mut Output,
    now: f64,
) -> bool {
    let depth = chain.levels.len().saturating_sub(1);
    if chain.levels.is_empty() {
        return false;
    }
    // Shift+Esc closes the whole chain, Esc goes back one level.
    if ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Escape)) {
        chain.close(now);
        output.cancelled = true;
        return true;
    }
    let path = path_of(chain, depth);
    let Some(spec) = source.level(depth, &path) else {
        return false;
    };
    let visible = visible_rows(&spec, &chain.levels[depth].query);
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        if !chain.levels[depth].query.is_empty() {
            chain.levels[depth].query.clear();
        } else if depth == 0 {
            chain.close(now);
            output.cancelled = true;
        } else {
            chain.truncate(depth);
        }
        return true;
    }
    let mut used = false;
    let page = ((ctx.content_rect().height() / size::ROW_HEIGHT) as isize - 2).max(1);
    let moves: [(Key, isize, bool); 4] = [
        (Key::ArrowDown, 1, true),
        (Key::ArrowUp, -1, true),
        (Key::PageDown, page, false),
        (Key::PageUp, -page, false),
    ];
    for (key, by, wrap) in moves {
        while ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
            let level = &mut chain.levels[depth];
            level.selected = step(&spec, &visible, level.selected, by, wrap);
            level.scroll_to = true;
            level.idle_since = now;
            level.auto_fired = false;
            level.pending = None;
            used = true;
        }
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Home)) {
        let level = &mut chain.levels[depth];
        level.selected = first_selectable(&spec, &visible);
        level.scroll_to = true;
        used = true;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::End)) {
        let level = &mut chain.levels[depth];
        level.selected = visible
            .iter()
            .rev()
            .copied()
            .find(|&row| spec.rows[row].selectable);
        level.scroll_to = true;
        used = true;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
        && let Some(row) = chain.levels[depth].selected
    {
        activate(chain, source, depth, row, &spec, output, now);
        return true;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight)) {
        if let Some(row) = chain.levels[depth].selected
            && spec.rows[row].sub
        {
            activate(chain, source, depth, row, &spec, output, now);
        }
        return true;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowLeft)) {
        if depth > 0 {
            chain.truncate(depth);
        }
        return true;
    }
    // Typed text: mnemonics choose, otherwise speed search filters.
    if spec.mnemonics || spec.speed_search {
        let typed: Vec<String> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| match event {
                    Event::Text(text) if !text.chars().any(char::is_control) => Some(text.clone()),
                    _ => None,
                })
                .collect()
        });
        if !typed.is_empty() {
            ctx.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Text(_))));
            for text in typed {
                for c in text.chars() {
                    let mnemonic = spec.mnemonics.then(|| {
                        visible.iter().copied().find(|&row| {
                            spec.rows[row].selectable
                                && spec.rows[row]
                                    .mnemonic
                                    .is_some_and(|m| m.eq_ignore_ascii_case(&c))
                        })
                    });
                    if let Some(Some(row)) = mnemonic {
                        chain.levels[depth].selected = Some(row);
                        activate(chain, source, depth, row, &spec, output, now);
                        return true;
                    }
                    if spec.speed_search {
                        chain.levels[depth].query.push(c);
                        used = true;
                    }
                }
            }
        }
    }
    if spec.speed_search
        && !chain.levels[depth].query.is_empty()
        && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
    {
        chain.levels[depth].query.pop();
        used = true;
    }
    if used && spec.speed_search {
        let visible = visible_rows(&spec, &chain.levels[depth].query);
        let level = &mut chain.levels[depth];
        if !level.query.is_empty()
            && level
                .selected
                .is_none_or(|row| !visible.contains(&row) || !spec.rows[row].selectable)
        {
            level.selected = first_selectable(&spec, &visible);
            level.scroll_to = true;
        }
    }
    used
}
