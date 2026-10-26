//! The engine shared by lists, trees, tables and tree tables: selection (keyboard and
//! mouse), speed search, scrolling, empty text and virtualized rows.
//!
//! The rules come from the IntelliJ spec section 3: the keys of Swing's `ListActions` and
//! `TreeActions`, the click rules of `JBTable`/`Tree`, the speed search of `SpeedSearchBase`.

use super::CollectionResponse;
use crate::common;
use crate::parts::ScrollBar;
use crate::theme::Theme;
use crate::tokens::{empty_text, size};
use crate::ui::speed_search::{self, Action, Jump};
use crate::ui::{
    ListSelectionModel, SelectionMode, SpeedSearch, SpeedSearchComparator, SpeedSearchState,
    StatusText,
};
use egui::{Event, Id, Key, Modifiers, Response, Sense, Ui};
use std::sync::Arc;

/// Where a collection keeps its selection: one optional row, or a [`ListSelectionModel`].
///
/// `&mut Option<usize>` and `&mut ListSelectionModel` convert into it, so
/// `JBList::new(id, len, &mut selected)` takes either.
#[derive(Debug)]
pub enum SelectionTarget<'a> {
    /// At most one selected row.
    Single(&'a mut Option<usize>),
    /// Any number of selected rows, with anchor and lead.
    Model(&'a mut ListSelectionModel),
}

impl<'a> From<&'a mut Option<usize>> for SelectionTarget<'a> {
    fn from(selected: &'a mut Option<usize>) -> Self {
        Self::Single(selected)
    }
}

impl<'a> From<&'a mut ListSelectionModel> for SelectionTarget<'a> {
    fn from(model: &'a mut ListSelectionModel) -> Self {
        Self::Model(model)
    }
}

/// One drawn row: its response and whether its chevron asked to toggle.
pub(super) struct DrawnRow {
    pub response: Response,
    pub toggled: bool,
    /// The row consumed its double-click itself (it started an edit): don't activate.
    pub handled: bool,
}

/// What the engine tells a row about itself.
pub(super) struct RowCtx<'s> {
    /// The row's index in the caller's data.
    pub index: usize,
    pub selected: bool,
    /// The collection has the keyboard focus (and the window is active): the selection is
    /// drawn in the active color.
    pub focused: bool,
    /// The row above / below is selected too: their pills join.
    pub joined_above: bool,
    pub joined_below: bool,
    /// The keyboard's lead row while it is not selected: outlined.
    pub lead: bool,
    /// The typed speed search, for highlighting what matches.
    pub search: Option<&'s SpeedSearchComparator>,
}

/// What the keyboard did that the widgets react to.
#[derive(Default)]
pub(super) struct Keys {
    /// <kbd>Enter</kbd> was pressed (it also reports `activated`, or `toggled` on a tree
    /// parent).
    pub enter: bool,
    /// <kbd>F2</kbd> was pressed.
    pub f2: bool,
}

/// What a tree does with <kbd>←</kbd>/<kbd>→</kbd> on its lead row.
pub(super) enum TreeMove {
    /// Expand or collapse the row.
    Toggle,
    /// Move the selection to this row (the first child, or the parent).
    Select(usize),
}

/// The rows of a collection, as the engine sees them.
pub(super) trait Rows {
    /// The texts of row `index` a speed search looks at, one per cell.
    fn texts(&mut self, index: usize) -> Vec<String>;
    /// Draws row `index`.
    fn row(&mut self, ui: &mut Ui, ctx: RowCtx<'_>) -> DrawnRow;
    /// What <kbd>←</kbd> (`left`) or <kbd>→</kbd> does on the lead row `index`. `None` for
    /// plain rows.
    fn horizontal(&mut self, index: usize, left: bool) -> Option<TreeMove> {
        let _ = (index, left);
        None
    }
    /// Whether <kbd>Enter</kbd> on row `index` expands or collapses it instead of activating.
    fn enter_toggles(&mut self, index: usize) -> bool {
        let _ = index;
        false
    }
}

/// Scroll position and viewport height from the last frame.
#[derive(Clone, Copy)]
struct Viewport {
    offset: f32,
    height: f32,
}

/// The rows speed search matched last, so a large collection is searched once per typed
/// text and length, not every frame.
#[derive(Clone)]
struct MatchCache {
    text: String,
    len: usize,
    matches: Arc<Vec<usize>>,
}

/// What a collection is: identity, size and accessible role.
pub(super) struct Spec<'a> {
    pub id: Id,
    pub len: usize,
    pub height: f32,
    pub role: egui::accesskit::Role,
    pub name: &'a str,
    pub search: Option<SpeedSearch<'a>>,
    pub empty: Option<StatusText>,
}

/// The rows that are shown, in order: all of them, or only the matches of a filtering speed
/// search. A position is the place in this order; an index is the row in the caller's data.
#[derive(Clone, Copy)]
struct Order<'a> {
    map: Option<&'a [usize]>,
    len: usize,
}

impl Order<'_> {
    fn count(&self) -> usize {
        self.map.map_or(self.len, <[usize]>::len)
    }

    fn at(&self, position: usize) -> Option<usize> {
        match self.map {
            Some(map) => map.get(position).copied(),
            None => (position < self.len).then_some(position),
        }
    }

    fn position(&self, index: usize) -> Option<usize> {
        match self.map {
            Some(map) => map.binary_search(&index).ok(),
            None => (index < self.len).then_some(index),
        }
    }
}

/// Which rows are in view, in positions, for the page keys.
#[derive(Clone, Copy)]
struct Page {
    first: usize,
    last: usize,
    rows: usize,
}

impl Page {
    fn of(viewport: Option<Viewport>) -> Self {
        match viewport {
            Some(view) => {
                let rows = ((view.height / size::ROW_HEIGHT) as usize).max(1);
                let first = (view.offset / size::ROW_HEIGHT).ceil() as usize;
                let last = (((view.offset + view.height) / size::ROW_HEIGHT).floor() as usize)
                    .saturating_sub(1)
                    .max(first);
                Self { first, last, rows }
            }
            None => Self {
                first: 0,
                last: 9,
                rows: 10,
            },
        }
    }
}

fn matches_row(rows: &mut dyn Rows, index: usize, comparator: &SpeedSearchComparator) -> bool {
    rows.texts(index)
        .iter()
        .any(|text| comparator.matches(text))
}

/// The rows matching `comparator`, from the cache when the text and length are the same.
fn cached_matches(
    ctx: &egui::Context,
    id: Id,
    text: &str,
    len: usize,
    comparator: &SpeedSearchComparator,
    rows: &mut dyn Rows,
) -> Arc<Vec<usize>> {
    let key = id.with("speed-search-matches");
    if let Some(cache) = ctx.data(|d| d.get_temp::<MatchCache>(key))
        && cache.text == text
        && cache.len == len
    {
        return cache.matches;
    }
    let matches: Arc<Vec<usize>> = Arc::new(
        (0..len)
            .filter(|&index| matches_row(rows, index, comparator))
            .collect(),
    );
    ctx.data_mut(|d| {
        d.insert_temp(
            key,
            MatchCache {
                text: text.to_owned(),
                len,
                matches: matches.clone(),
            },
        );
    });
    matches
}

/// Selects the rows between two positions (and nothing else unless `additive`).
fn select_between(
    model: &mut ListSelectionModel,
    order: &Order<'_>,
    anchor: usize,
    lead: usize,
    additive: bool,
) -> bool {
    let (Some(anchor_index), Some(lead_index)) = (order.at(anchor), order.at(lead)) else {
        return false;
    };
    if order.map.is_none() {
        return if additive {
            model.add_selection_interval(anchor_index, lead_index)
        } else {
            model.set_selection_interval(anchor_index, lead_index)
        };
    }
    // A filtered view: only the rows that are shown between them.
    if model.mode() != SelectionMode::MultipleInterval {
        return model.select(lead_index);
    }
    let mut changed = !additive && model.clear();
    for position in anchor.min(lead)..=anchor.max(lead) {
        if let Some(index) = order.at(position) {
            changed |= model.add_selection_interval(index, index);
        }
    }
    model.set_anchor_and_lead(anchor_index, lead_index);
    changed
}

/// A click on row `index` with `mods`: select it, toggle it (<kbd>Ctrl</kbd>/<kbd>Cmd</kbd>)
/// or extend from the anchor (<kbd>Shift</kbd>).
fn apply_click(
    model: &mut ListSelectionModel,
    order: &Order<'_>,
    index: usize,
    mods: Modifiers,
) -> bool {
    if mods.shift && model.mode() != SelectionMode::Single {
        let anchor = model
            .anchor()
            .and_then(|anchor| order.position(anchor))
            .or_else(|| order.position(index));
        if let (Some(anchor), Some(lead)) = (anchor, order.position(index)) {
            return select_between(model, order, anchor, lead, mods.command);
        }
    }
    if mods.command {
        model.toggle(index)
    } else {
        model.select(index)
    }
}

/// What a navigation key did.
struct Moved {
    changed: bool,
    /// The position to scroll into view.
    position: usize,
}

/// A navigation key on the lead row: the keys of Swing's list and tree actions.
fn navigate(
    model: &mut ListSelectionModel,
    order: &Order<'_>,
    key: Key,
    mods: Modifiers,
    page: Page,
) -> Option<Moved> {
    let count = order.count();
    let last = count.checked_sub(1)?;
    let lead = model.lead().and_then(|lead| order.position(lead));
    let target = match key {
        Key::ArrowDown => lead.map_or(0, |p| (p + 1).min(last)),
        Key::ArrowUp => lead.map_or(0, |p| p.saturating_sub(1)),
        Key::Home => 0,
        Key::End => last,
        // The first press goes to the edge of the view, the next one a page further.
        Key::PageDown => lead.map_or(0, |p| {
            if p < page.last {
                page.last
            } else {
                p + page.rows
            }
            .min(last)
        }),
        Key::PageUp => lead.map_or(0, |p| {
            if p > page.first {
                page.first
            } else {
                p.saturating_sub(page.rows)
            }
        }),
        _ => return None,
    };
    let index = order.at(target.min(last))?;
    let changed = if mods.command {
        // Ctrl/Cmd moves the lead without selecting.
        model.set_lead(index);
        false
    } else if mods.shift && model.mode() != SelectionMode::Single {
        let anchor = model
            .anchor()
            .and_then(|anchor| order.position(anchor))
            .or(lead)
            .unwrap_or(target);
        select_between(model, order, anchor, target, false)
    } else {
        model.select(index)
    };
    Some(Moved {
        changed,
        position: target.min(last),
    })
}

/// The shared engine: selection, speed search, scroll-into-view and virtualized rows.
pub(super) fn collection(
    ui: &mut Ui,
    spec: Spec<'_>,
    target: SelectionTarget<'_>,
    rows: &mut dyn Rows,
) -> (CollectionResponse, Keys) {
    let Spec {
        id,
        len,
        height,
        role,
        name,
        search,
        empty,
    } = spec;
    let row_height = size::ROW_HEIGHT;
    let focus_id = id.with("focus");
    let widget_focused = ui.memory(|m| m.has_focus(focus_id));
    // A selection is drawn in the active color only while the collection has the focus and
    // the window is the active one.
    let window_active = ui.ctx().input(|i| i.viewport().focused.unwrap_or(true));
    let focused = widget_focused && window_active;
    let viewport: Option<Viewport> = ui.ctx().data(|d| d.get_temp(id));

    let (single, borrowed) = match target {
        SelectionTarget::Single(selected) => (Some(selected), None),
        SelectionTarget::Model(model) => (None, Some(model)),
    };
    let mut own = ListSelectionModel::single();
    if let Some(index) = single.as_deref().copied().flatten() {
        own.select(index);
    }
    let model: &mut ListSelectionModel = match borrowed {
        Some(model) => model,
        None => &mut own,
    };

    let mut selection_changed = false;
    let mut activated = None;
    let mut toggled = None;
    let mut keys = Keys::default();
    let mut scroll_to: Option<usize> = None;
    if model.clamp(len) {
        selection_changed = true;
        if single.is_some() && model.is_empty() && len > 0 {
            model.select(len - 1);
        }
    }

    let (mut state, filter): (Option<&mut SpeedSearchState>, bool) = match search {
        Some(search) => (Some(search.state), search.filter),
        None => (None, false),
    };
    if !widget_focused && let Some(state) = state.as_deref_mut() {
        // Focus loss closes the popup.
        if state.is_active() {
            state.reset();
        }
    }

    // The keyboard: speed search first (it takes typed text and some keys), then the keys
    // that are left.
    let mut actions: Vec<Action> = Vec::new();
    if widget_focused && len > 0 {
        let escape = state.as_deref().is_some_and(|s| s.is_active());
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                focus_id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape,
                    ..Default::default()
                },
            );
        });
        actions = match state.as_deref_mut() {
            Some(state) => ui.input(|input| speed_search::actions(input, state)),
            None => ui.input(|input| {
                input
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } => Some(Action::Pass(*key, *modifiers)),
                        _ => None,
                    })
                    .collect()
            }),
        };
    }

    let comparator: Option<SpeedSearchComparator> = state
        .as_deref()
        .filter(|state| !state.text().is_empty())
        .map(|state| SpeedSearchComparator::new(state.text()))
        .filter(|comparator| !comparator.is_empty());
    let matches: Option<Arc<Vec<usize>>> = match (&comparator, state.as_deref()) {
        (Some(comparator), Some(state)) => Some(cached_matches(
            ui.ctx(),
            id,
            state.text(),
            len,
            comparator,
            rows,
        )),
        _ => None,
    };
    let order = Order {
        map: matches.as_deref().filter(|_| filter).map(Vec::as_slice),
        len,
    };
    let page = Page::of(viewport);
    let multiple = model.mode() != SelectionMode::Single;

    for action in actions {
        match action {
            Action::Changed => {
                if let Some(matches) = &matches {
                    // The first match from the lead on, else the first of all.
                    let from = model.lead().unwrap_or(0);
                    let pick = matches
                        .iter()
                        .copied()
                        .find(|&index| index >= from)
                        .or_else(|| matches.first().copied());
                    if let Some(index) = pick {
                        selection_changed |= model.select(index);
                        scroll_to = order.position(index);
                    }
                }
            }
            Action::Jump(jump) => {
                if let Some(matches) = &matches {
                    let lead = model.lead();
                    let pick = match jump {
                        Jump::First => matches.first().copied(),
                        Jump::Last => matches.last().copied(),
                        Jump::Next => matches
                            .iter()
                            .copied()
                            .find(|&index| lead.is_none_or(|lead| index > lead)),
                        Jump::Previous => matches
                            .iter()
                            .copied()
                            .rev()
                            .find(|&index| lead.is_some_and(|lead| index < lead)),
                    };
                    if let Some(index) = pick {
                        selection_changed |= model.select(index);
                        scroll_to = order.position(index);
                    }
                }
            }
            Action::Pass(key, mods) => {
                if len == 0 {
                    continue;
                }
                let current = model
                    .lead()
                    .filter(|lead| model.is_selected(*lead))
                    .or_else(|| model.first());
                match key {
                    Key::Enter => {
                        keys.enter = true;
                        if let Some(index) = current {
                            if rows.enter_toggles(index) {
                                toggled = Some(index);
                            } else {
                                activated = Some(index);
                            }
                        }
                    }
                    Key::F2 => keys.f2 = true,
                    Key::ArrowLeft | Key::ArrowRight if !mods.command && !mods.alt => {
                        if let Some(lead) = model.lead().filter(|lead| *lead < len) {
                            match rows.horizontal(lead, key == Key::ArrowLeft) {
                                Some(TreeMove::Toggle) => toggled = Some(lead),
                                Some(TreeMove::Select(index)) => {
                                    selection_changed |= model.select(index);
                                    scroll_to = order.position(index);
                                }
                                None => {}
                            }
                        }
                    }
                    Key::A if mods.command && !mods.shift && multiple => {
                        selection_changed |= model.select_all(len);
                    }
                    Key::Space if mods.command => {
                        if let Some(lead) = model.lead() {
                            selection_changed |= if mods.shift {
                                model.add_selection_interval(lead, lead)
                            } else {
                                model.toggle(lead)
                            };
                        }
                    }
                    _ => {
                        if let Some(moved) = navigate(model, &order, key, mods, page) {
                            selection_changed |= moved.changed;
                            scroll_to = Some(moved.position);
                        }
                    }
                }
            }
        }
    }

    let mut area = crate::parts::scroll_vertical()
        .id_salt(id)
        .max_height(height)
        .min_scrolled_height(height)
        .scroll_source(
            egui::scroll_area::ScrollSource::MOUSE_WHEEL
                | egui::scroll_area::ScrollSource::SCROLL_BAR,
        )
        .auto_shrink([false, false]);
    if let (Some(position), Some(view)) = (scroll_to, viewport) {
        let top = position as f32 * row_height;
        if top < view.offset {
            area = area.vertical_scroll_offset(top);
        } else if top + row_height > view.offset + view.height {
            area = area.vertical_scroll_offset(top + row_height - view.height);
        }
    }
    let bar = ScrollBar::new(id);
    let area = bar.prepare(ui, area);

    let mut clicked = None;
    let mut secondary = None;
    let count = order.count();
    let lead = model.lead().filter(|_| widget_focused);
    // show_rows strides by the row height plus the item spacing of the ui it starts in.
    let shown = ui.scope_builder(egui::UiBuilder::new().id_salt(id.with("rows")), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        area.show_rows(ui, row_height, count, |ui, range| {
            for position in range {
                let Some(index) = order.at(position) else {
                    continue;
                };
                let selected = model.is_selected(index);
                let neighbor = |position: Option<usize>| {
                    position
                        .and_then(|p| order.at(p))
                        .is_some_and(|i| model.is_selected(i))
                };
                let drawn = rows.row(
                    ui,
                    RowCtx {
                        index,
                        selected,
                        focused,
                        joined_above: selected && neighbor(position.checked_sub(1)),
                        joined_below: selected && neighbor(Some(position + 1)),
                        lead: lead == Some(index) && !selected,
                        search: comparator.as_ref(),
                    },
                );
                if drawn.toggled {
                    toggled = Some(index);
                } else if drawn.response.double_clicked() && !drawn.handled {
                    activated = Some(index);
                }
                if drawn.response.clicked() || drawn.response.double_clicked() {
                    clicked = Some(index);
                }
                if drawn.response.secondary_clicked() {
                    secondary = Some(index);
                }
            }
        })
    });
    let output = shown.inner;
    // The rows' parent node gets the viewport bounds, so tools see which rows are cut off.
    let viewport_rect = output.inner_rect;
    ui.ctx().accesskit_node_builder(shown.response.id, |node| {
        node.set_role(egui::accesskit::Role::Group);
        node.set_bounds(egui::accesskit::Rect {
            x0: viewport_rect.min.x.into(),
            y0: viewport_rect.min.y.into(),
            x1: viewport_rect.max.x.into(),
            y1: viewport_rect.max.y.into(),
        });
    });
    bar.show(ui, &output);

    let mods = ui.input(|input| {
        // The modifiers the click happened with; else the ones held now.
        input
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::PointerButton { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or(input.modifiers)
    });
    if let Some(index) = clicked {
        selection_changed |= apply_click(model, &order, index, mods);
    }
    // A right click keeps the selection when it lands inside it, else selects the row.
    if let Some(index) = secondary
        && !model.is_selected(index)
    {
        selection_changed |= model.select(index);
    }
    let view = Viewport {
        offset: output.state.offset.y,
        height: output.inner_rect.height(),
    };
    ui.ctx().data_mut(|d| d.insert_temp(id, view));

    // Nothing to show: the empty text, centered in the viewport.
    let mut empty_link = None;
    if count == 0 {
        let text = if comparator.is_some() && filter {
            StatusText::text("No matches")
        } else {
            empty.unwrap_or_else(|| StatusText::text(empty_text::DEFAULT))
        };
        empty_link = text.show_in(ui, output.inner_rect, id.with("empty-text"));
    }

    // The list itself holds keyboard focus (Tab reaches it once, not every row).
    let response = ui.interact(
        output.inner_rect,
        focus_id,
        Sense::focusable_noninteractive(),
    );
    ui.ctx().accesskit_node_builder(focus_id, |node| {
        node.set_role(role);
        node.set_label(name);
    });
    // After `interact`: a click on a row counts as "elsewhere" for the focus widget, which
    // would otherwise give up the focus requested here.
    if clicked.is_some() || secondary.is_some() {
        ui.memory_mut(|m| m.request_focus(focus_id));
    }
    if response.has_focus() && model.is_empty() {
        let colors = Theme::colors(ui.ctx());
        common::focus_ring(ui.painter(), output.inner_rect.shrink(1.0), 4, &colors);
    }
    if let Some(state) = state.as_deref()
        && state.is_active()
    {
        let no_hits = comparator.is_some() && matches.as_ref().is_some_and(|m| m.is_empty());
        speed_search::paint_popup(ui, output.inner_rect, state, no_hits);
    }
    let current = model
        .lead()
        .filter(|lead| model.is_selected(*lead))
        .or_else(|| model.first());
    if let Some(selected) = single {
        *selected = model.first();
    }
    (
        CollectionResponse {
            response,
            selection_changed,
            activated,
            toggled,
            sort_clicked: None,
            edited: None,
            column_order: None,
            secondary_clicked: secondary,
            empty_text_link: empty_link,
            current,
        },
        keys,
    )
}
