//! Virtualized collections: lists, trees and tables that draw only the visible rows, so they
//! stay fast with hundreds of thousands of items.
//!
//! All three share one model: the caller owns the data and the selection
//! (`&mut Option<usize>`), the widget draws the visible rows, moves the selection with the
//! keyboard and reports what happened in a [`CollectionResponse`].

use super::{ListRow, common};
use crate::icons::{self, Glyph, Icon};
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{
    Align, CornerRadius, CursorIcon, Id, Key, Rect, Response, Sense, Stroke, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// What happened in a [`VirtualList`], [`Tree`] or [`Table`] this frame.
#[derive(Debug)]
pub struct CollectionResponse {
    /// The scroll area's response.
    pub response: Response,
    /// The selection changed (click or keyboard).
    pub selection_changed: bool,
    /// A row was opened: double-click or <kbd>Enter</kbd>.
    pub activated: Option<usize>,
    /// A tree row asked to expand or collapse: chevron, double-click on a parent,
    /// <kbd>→</kbd>/<kbd>←</kbd>.
    pub toggled: Option<usize>,
    /// A table header was clicked to sort by that column.
    pub sort_clicked: Option<usize>,
}

/// One drawn row: its response and whether its chevron asked to toggle.
struct DrawnRow {
    response: Response,
    toggled: bool,
}

/// Scroll position and viewport height from the last frame.
#[derive(Clone, Copy)]
struct Viewport {
    offset: f32,
    height: f32,
}

/// What a collection is: identity, size and accessible role.
struct Spec<'a> {
    id: Id,
    len: usize,
    height: f32,
    role: egui::accesskit::Role,
    name: &'a str,
}

/// The shared engine: keyboard navigation, scroll-into-view and virtualized rows.
/// Returns the response and the horizontal arrow pressed while focused (for trees).
fn collection(
    ui: &mut Ui,
    spec: Spec<'_>,
    selected: &mut Option<usize>,
    mut row: impl FnMut(&mut Ui, usize, bool, bool) -> DrawnRow,
) -> (CollectionResponse, Option<Key>) {
    let Spec {
        id,
        len,
        height,
        role,
        name,
    } = spec;
    let row_height = size::ROW_HEIGHT;
    let focus_id = id.with("focus");
    let focused = ui.memory(|m| m.has_focus(focus_id));
    let viewport: Option<Viewport> = ui.ctx().data(|d| d.get_temp(id));
    let mut selection_changed = false;
    let mut activated = None;
    let mut horizontal = None;
    let mut scroll_to = None;

    if *selected >= Some(len) {
        *selected = len.checked_sub(1);
        selection_changed = true;
    }
    if focused && len > 0 {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                focus_id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });
        let page = viewport.map_or(10, |v| ((v.height / row_height) as usize).max(1));
        let current = *selected;
        let next = crate::parts::nav::pressed(ui, current, len, page);
        if let Some(next) = next {
            selection_changed |= current != Some(next);
            *selected = Some(next);
            scroll_to = Some(next);
        }
        ui.input(|i| {
            if i.key_pressed(Key::Enter) {
                activated = *selected;
            }
            if i.key_pressed(Key::ArrowLeft) {
                horizontal = Some(Key::ArrowLeft);
            } else if i.key_pressed(Key::ArrowRight) {
                horizontal = Some(Key::ArrowRight);
            }
        });
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
    if let (Some(index), Some(view)) = (scroll_to, viewport) {
        let top = index as f32 * row_height;
        if top < view.offset {
            area = area.vertical_scroll_offset(top);
        } else if top + row_height > view.offset + view.height {
            area = area.vertical_scroll_offset(top + row_height - view.height);
        }
    }

    let mut clicked = None;
    let mut toggled = None;
    // show_rows strides by the row height plus the item spacing of the ui it starts in.
    let rows = ui.scope_builder(egui::UiBuilder::new().id_salt(id.with("rows")), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        area.show_rows(ui, row_height, len, |ui, range| {
            for index in range {
                let drawn = row(ui, index, *selected == Some(index), focused);
                if drawn.toggled {
                    toggled = Some(index);
                } else if drawn.response.double_clicked() {
                    activated = Some(index);
                }
                if drawn.response.clicked() || drawn.response.double_clicked() {
                    clicked = Some(index);
                }
            }
        })
    });
    let output = rows.inner;
    // The rows' parent node gets the viewport bounds, so tools see which rows are cut off.
    let viewport_rect = output.inner_rect;
    ui.ctx().accesskit_node_builder(rows.response.id, |node| {
        node.set_role(egui::accesskit::Role::Group);
        node.set_bounds(egui::accesskit::Rect {
            x0: viewport_rect.min.x.into(),
            y0: viewport_rect.min.y.into(),
            x1: viewport_rect.max.x.into(),
            y1: viewport_rect.max.y.into(),
        });
    });
    if let Some(index) = clicked {
        selection_changed |= *selected != Some(index);
        *selected = Some(index);
    }
    let view = Viewport {
        offset: output.state.offset.y,
        height: output.inner_rect.height(),
    };
    ui.ctx().data_mut(|d| d.insert_temp(id, view));
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
    if clicked.is_some() {
        ui.memory_mut(|m| m.request_focus(focus_id));
    }
    if response.has_focus() && selected.is_none() {
        let colors = Theme::colors(ui.ctx());
        common::focus_ring(ui.painter(), output.inner_rect.shrink(1.0), 4, &colors);
    }
    (
        CollectionResponse {
            response,
            selection_changed,
            activated,
            toggled,
            sort_clicked: None,
        },
        horizontal,
    )
}

/// A scrolling list of [`ListRow`]s that draws only the visible ones.
///
/// <kbd>↑</kbd>/<kbd>↓</kbd>, <kbd>Home</kbd>/<kbd>End</kbd> and
/// <kbd>PgUp</kbd>/<kbd>PgDn</kbd> move the selection; <kbd>Enter</kbd> or a double-click
/// activates a row.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{ListRow, VirtualList};
/// use archipel_kit::Glyph;
///
/// let requests: Vec<String> = (0..10_000).map(|i| format!("GET /users/{i}")).collect();
/// let mut selected = None;
/// let out = VirtualList::new("history", requests.len(), &mut selected)
///     .height(240.0)
///     .show(ui, |i| ListRow::new(&requests[i]).icon(Glyph::Send));
/// if let Some(i) = out.activated { /* open requests[i] */ }
/// # });
/// ```
#[must_use = "show it with `list.show(ui, row)`"]
pub struct VirtualList<'a> {
    id: Id,
    name: String,
    len: usize,
    selected: &'a mut Option<usize>,
    height: f32,
}

impl<'a> VirtualList<'a> {
    /// A list of `len` rows with a unique `id_salt`, editing `selected`.
    pub fn new(id_salt: impl std::hash::Hash, len: usize, selected: &'a mut Option<usize>) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "List".into(),
            len,
            selected,
            height: 240.0,
        }
    }

    /// Visible height in points (default 240).
    /// The accessible name, e.g. "Request history" (default: "List", "Tree" or "Table").
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Visible height in points.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Draws the visible rows; `row(index)` describes row `index` (the list sets its
    /// selection).
    pub fn show(self, ui: &mut Ui, mut row: impl FnMut(usize) -> ListRow) -> CollectionResponse {
        let (response, _) = collection(
            ui,
            Spec {
                id: self.id,
                len: self.len,
                height: self.height,
                role: egui::accesskit::Role::List,
                name: &self.name,
            },
            self.selected,
            |ui, index, selected, focused| {
                let shown = row(index)
                    .selected(selected)
                    .inactive(!focused)
                    .focusable(false)
                    .show(ui);
                DrawnRow {
                    response: shown.response,
                    toggled: shown.toggled,
                }
            },
        );
        response
    }
}

/// A visible row of a [`Tree`]: the caller flattens its tree into the rows that are
/// visible (children of expanded nodes) and keeps the expanded state.
#[derive(Debug, Clone)]
pub struct TreeRow {
    label: String,
    secondary: Option<String>,
    icon: Option<Icon>,
    depth: usize,
    children: bool,
    expanded: bool,
}

impl TreeRow {
    /// A row reading `label` at `depth` (0 is the root level).
    pub fn new(label: impl Into<String>, depth: usize) -> Self {
        Self {
            label: label.into(),
            secondary: None,
            icon: None,
            depth,
            children: false,
            expanded: false,
        }
    }

    /// A node with children, shown expanded or collapsed.
    pub fn parent(mut self, expanded: bool) -> Self {
        self.children = true;
        self.expanded = expanded;
        self
    }

    /// Icon before the label.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Dimmed text after the label.
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Depth (0 is the root level).
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Whether this row has children.
    pub fn has_children(&self) -> bool {
        self.children
    }

    /// Whether this row is expanded.
    pub fn is_expanded(&self) -> bool {
        self.children && self.expanded
    }
}

/// A virtualized tree: project views, structure views, JSON documents.
///
/// Keys as in [`VirtualList`], plus <kbd>→</kbd> expands a collapsed node or moves to its
/// first child, and <kbd>←</kbd> collapses an expanded node or moves to its parent.
/// Expanding and collapsing is reported in [`CollectionResponse::toggled`]: flip your
/// expanded state and rebuild the rows.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Tree, TreeRow};
/// use archipel_kit::Glyph;
///
/// let mut src_open = true;
/// let mut selected = Some(1);
/// let mut rows = vec![TreeRow::new("src", 0).icon(Glyph::Folder).parent(src_open)];
/// if src_open {
///     rows.push(TreeRow::new("main.rs", 1));
/// }
/// let out = Tree::new("project", &rows, &mut selected).show(ui);
/// if out.toggled == Some(0) {
///     src_open = !src_open;
/// }
/// # });
/// ```
#[must_use = "show it with `tree.show(ui)`"]
pub struct Tree<'a> {
    id: Id,
    name: String,
    rows: &'a [TreeRow],
    selected: &'a mut Option<usize>,
    height: f32,
}

impl<'a> Tree<'a> {
    /// A tree of the visible `rows` with a unique `id_salt`, editing `selected`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        rows: &'a [TreeRow],
        selected: &'a mut Option<usize>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "Tree".into(),
            rows,
            selected,
            height: 240.0,
        }
    }

    /// Visible height in points (default 240).
    /// The accessible name, e.g. "Request history" (default: "List", "Tree" or "Table").
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Visible height in points.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Draws the visible rows and reports clicks, keys and toggles.
    pub fn show(self, ui: &mut Ui) -> CollectionResponse {
        let rows = self.rows;
        let (mut response, horizontal) = collection(
            ui,
            Spec {
                id: self.id,
                len: rows.len(),
                height: self.height,
                role: egui::accesskit::Role::Tree,
                name: &self.name,
            },
            self.selected,
            |ui, index, selected, focused| {
                let tree_row = &rows[index];
                let mut row = ListRow::new(tree_row.label.as_str())
                    .depth(tree_row.depth)
                    .expanded(tree_row.children.then_some(tree_row.expanded))
                    .selected(selected)
                    .inactive(!focused)
                    .focusable(false);
                if let Some(icon) = tree_row.icon {
                    row = row.icon(icon);
                }
                if let Some(secondary) = &tree_row.secondary {
                    row = row.secondary(secondary.as_str());
                }
                let shown = row.show(ui);
                DrawnRow {
                    response: shown.response,
                    toggled: shown.toggled,
                }
            },
        );
        if let (Some(key), Some(index)) = (horizontal, *self.selected) {
            let row = &rows[index];
            let mut select = |target: usize| {
                if *self.selected != Some(target) {
                    *self.selected = Some(target);
                    response.selection_changed = true;
                }
            };
            match key {
                Key::ArrowRight if row.children && !row.expanded => response.toggled = Some(index),
                Key::ArrowRight if row.is_expanded() => {
                    if rows
                        .get(index + 1)
                        .is_some_and(|next| next.depth > row.depth)
                    {
                        select(index + 1);
                    }
                }
                Key::ArrowLeft if row.is_expanded() => response.toggled = Some(index),
                Key::ArrowLeft => {
                    if let Some(parent) = rows[..index].iter().rposition(|r| r.depth < row.depth) {
                        select(parent);
                    }
                }
                _ => {}
            }
        }
        response
    }
}

/// Sort direction of a [`Table`] column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// A to Z, small to large.
    Ascending,
    /// Z to A, large to small.
    Descending,
}

impl SortOrder {
    /// The other direction.
    pub fn reversed(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

/// A [`Table`] column. The caller owns the columns; dragging a header edge changes
/// `width`.
#[derive(Debug, Clone)]
pub struct Column {
    title: String,
    /// Width in points.
    pub width: f32,
    align: Align,
    mono: bool,
}

impl Column {
    /// A column titled `title`, `width` points wide.
    pub fn new(title: impl Into<String>, width: f32) -> Self {
        Self {
            title: title.into(),
            width,
            align: Align::Min,
            mono: false,
        }
    }

    /// Right-aligned (numbers, sizes, durations).
    pub fn right(mut self) -> Self {
        self.align = Align::Max;
        self
    }

    /// Monospace cells (ids, hashes, JSON values).
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// The title.
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// The content of a table cell.
#[derive(Debug, Clone, Default)]
pub struct Cell {
    text: String,
    icon: Option<Icon>,
    tone: Option<Tone>,
}

impl Cell {
    /// A cell reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// An icon before the text.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Meaning of the text's color: [`Tone::Error`] for a 500, [`Tone::Muted`] for nulls
    /// and timestamps.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Secondary text (null values, timestamps): [`Tone::Secondary`].
    pub fn muted(self) -> Self {
        self.tone(Tone::Secondary)
    }
}

impl From<&str> for Cell {
    fn from(text: &str) -> Self {
        Cell::new(text)
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Cell::new(text)
    }
}

/// Horizontal padding inside a cell.
const CELL_PADDING: f32 = 8.0;
/// Narrowest a column can be dragged.
const MIN_COLUMN: f32 = 40.0;

/// A virtualized table: a header with sort indicators and resizable columns, and rows
/// that are drawn only when visible. Query results, request history, headers.
///
/// Keys as in [`VirtualList`]. Clicking a header reports
/// [`CollectionResponse::sort_clicked`]; sort your rows and pass the new order with
/// [`Table::sort`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Cell, Column, SortOrder, Table};
///
/// let mut columns = vec![Column::new("_id", 220.0).mono(), Column::new("name", 140.0)];
/// let users = [("64f1…a1", "Ada"), ("64f1…a2", "Linus")];
/// let mut selected = None;
/// let mut sort = Some((1, SortOrder::Ascending));
/// let out = Table::new("users", &mut columns, users.len(), &mut selected)
///     .sort(sort)
///     .show(ui, |row, col| match col {
///         0 => Cell::new(users[row].0),
///         _ => Cell::new(users[row].1),
///     });
/// if let Some(col) = out.sort_clicked {
///     sort = Some(match sort {
///         Some((c, order)) if c == col => (col, order.reversed()),
///         _ => (col, SortOrder::Ascending),
///     });
/// }
/// # });
/// ```
#[must_use = "show it with `table.show(ui, cell)`"]
pub struct Table<'a> {
    id: Id,
    name: String,
    columns: &'a mut [Column],
    len: usize,
    selected: &'a mut Option<usize>,
    sort: Option<(usize, SortOrder)>,
    height: f32,
}

impl<'a> Table<'a> {
    /// A table of `len` rows in `columns` with a unique `id_salt`, editing `selected`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        columns: &'a mut [Column],
        len: usize,
        selected: &'a mut Option<usize>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "Table".into(),
            columns,
            len,
            selected,
            sort: None,
            height: 240.0,
        }
    }

    /// The column and direction the rows are sorted by (shown in the header).
    pub fn sort(mut self, sort: Option<(usize, SortOrder)>) -> Self {
        self.sort = sort;
        self
    }

    /// Height of the rows area in points, without the header (default 240).
    /// The accessible name, e.g. "Request history" (default: "List", "Tree" or "Table").
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Visible height in points.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Draws the header and the visible rows; `cell(row, column)` gives each cell.
    pub fn show(
        self,
        ui: &mut Ui,
        mut cell: impl FnMut(usize, usize) -> Cell,
    ) -> CollectionResponse {
        let colors = Theme::colors(ui.ctx());
        let sort_clicked = header(ui, self.id, self.columns, self.sort);
        let columns: &[Column] = self.columns;
        let (mut response, _) = collection(
            ui,
            Spec {
                id: self.id,
                len: self.len,
                height: self.height,
                role: egui::accesskit::Role::Table,
                name: &self.name,
            },
            self.selected,
            |ui, index, selected, focused| {
                let (rect, response) = ui.allocate_exact_size(
                    vec2(ui.available_width(), size::ROW_HEIGHT),
                    Sense::CLICK,
                );
                crate::cursor::on_hover(&response, CursorIcon::Default);
                let cells: Vec<Cell> = (0..columns.len()).map(|col| cell(index, col)).collect();
                response.widget_info(|| {
                    let label: Vec<&str> = cells.iter().map(|c| c.text.as_str()).collect();
                    WidgetInfo::selected(
                        WidgetType::SelectableLabel,
                        true,
                        selected,
                        label.join(", "),
                    )
                });
                if ui.is_rect_visible(rect) {
                    let state = crate::parts::State::of(&response)
                        .selected(selected)
                        .inactive(!focused);
                    let fill = state.row_fill(&colors);
                    common::row_pill(ui.painter(), rect, fill);
                    let mut x = rect.left() + size::ROW_PILL_INSET;
                    for (column, cell) in columns.iter().zip(cells) {
                        let cell_rect = Rect::from_min_max(
                            pos2(x, rect.top()),
                            pos2(x + column.width, rect.bottom()),
                        );
                        paint_cell(ui, cell_rect.intersect(rect), column, cell, selected);
                        x += column.width;
                    }
                }
                DrawnRow {
                    response,
                    toggled: false,
                }
            },
        );
        response.sort_clicked = sort_clicked;
        response
    }
}

/// Draws the header row; returns the column whose title was clicked.
fn header(
    ui: &mut Ui,
    id: Id,
    columns: &mut [Column],
    sort: Option<(usize, SortOrder)>,
) -> Option<usize> {
    let colors = Theme::colors(ui.ctx());
    let height = size::ROW_HEIGHT + 4.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let font = typography::semibold(ui.ctx(), typography::BODY);
    let mut clicked = None;
    let mut x = rect.left() + size::ROW_PILL_INSET;
    for (index, column) in columns.iter_mut().enumerate() {
        let cell = Rect::from_min_max(pos2(x, rect.top()), pos2(x + column.width, rect.bottom()))
            .intersect(rect);
        let title = ui.interact(cell, id.with(("header", index)), Sense::click());
        crate::cursor::on_hover(&title, CursorIcon::Default);
        title.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &column.title));
        if title.clicked() {
            clicked = Some(index);
        }
        if title.hovered() {
            ui.painter().rect_filled(
                cell.shrink2(vec2(1.0, 3.0)),
                CornerRadius::same(radius::ROW),
                colors.row_hover,
            );
        }
        let galley = common::galley(ui, column.title.as_str(), font.clone());
        let sorted = sort
            .filter(|(col, _)| *col == index)
            .map(|(_, order)| order);
        let arrow = if sorted.is_some() {
            size::ICON + 2.0
        } else {
            0.0
        };
        let text_x = match column.align {
            Align::Max => cell.right() - CELL_PADDING - galley.size().x - arrow,
            _ => cell.left() + CELL_PADDING,
        };
        let text_width = galley.size().x;
        let painter = ui.painter().with_clip_rect(cell);
        common::text_left(&painter, cell, text_x, galley, colors.text_secondary);
        if let Some(order) = sorted {
            let glyph = match order {
                SortOrder::Ascending => Glyph::ArrowUp,
                SortOrder::Descending => Glyph::ArrowDown,
            };
            let icon = Rect::from_min_size(
                pos2(
                    text_x + text_width + 2.0,
                    cell.center().y - size::ICON / 2.0,
                ),
                vec2(size::ICON, size::ICON),
            );
            icons::paint(&painter, icon, glyph, colors.text_secondary);
        }
        // Resize handle on the right edge.
        let edge = Rect::from_center_size(pos2(cell.right(), cell.center().y), vec2(7.0, height));
        let handle = ui.interact(edge, id.with(("resize", index)), Sense::drag());
        crate::cursor::on_hover(&handle, CursorIcon::ResizeColumn);
        ui.ctx().accesskit_node_builder(handle.id, |node| {
            node.set_role(egui::accesskit::Role::Splitter);
            node.set_label(format!("Resize {}", column.title));
        });
        if handle.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
            column.width = (column.width + handle.drag_delta().x).max(MIN_COLUMN);
        }
        let line = if handle.hovered() || handle.dragged() {
            colors.accent
        } else {
            colors.separator
        };
        ui.painter().vline(
            cell.right(),
            (cell.top() + 7.0)..=(cell.bottom() - 7.0),
            Stroke::new(1.0_f32, line),
        );
        x += column.width;
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, colors.separator),
    );
    clicked
}

fn paint_cell(ui: &Ui, rect: Rect, column: &Column, cell: Cell, selected: bool) {
    let colors = Theme::colors(ui.ctx());
    let painter = ui
        .painter()
        .with_clip_rect(rect.shrink2(vec2(CELL_PADDING / 2.0, 0.0)));
    let font = if column.mono {
        egui::FontId::monospace(typography::CODE)
    } else {
        typography::body(typography::BODY)
    };
    let color = match (cell.tone, selected) {
        (Some(tone), _) => tone.color(&colors),
        (None, true) => colors.text_strong,
        (None, false) => colors.text,
    };
    let galley = common::galley(ui, cell.text.as_str(), font);
    let icon_width = if cell.icon.is_some() {
        size::ICON + size::ICON_TEXT_GAP
    } else {
        0.0
    };
    let content = icon_width + galley.size().x;
    let mut x = match column.align {
        Align::Max => rect.right() - CELL_PADDING - content,
        _ => rect.left() + CELL_PADDING,
    };
    if let Some(icon) = cell.icon {
        let icon_rect = Rect::from_min_size(
            pos2(x, rect.center().y - size::ICON / 2.0),
            vec2(size::ICON, size::ICON),
        );
        icons::paint(&painter, icon_rect, icon, colors.icon);
        x += icon_width;
    }
    common::text_left(&painter, rect, x, galley, color);
}
