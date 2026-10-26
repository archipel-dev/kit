//! Virtualized collections: lists, trees and tables that draw only the visible rows, so they
//! stay fast with hundreds of thousands of items.
//!
//! All three share one model: the caller owns the data and the selection
//! (`&mut Option<usize>`), the widget draws the visible rows, moves the selection with the
//! keyboard and reports what happened in a [`CollectionResponse`].

mod edit;
mod tree_table;

pub use edit::CellEdit;
pub use tree_table::TreeTable;

use crate::behaviour::{Behaviour, DragSource, DropTarget};
use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::{Tone, ValueKind};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use crate::ui::ColoredListCellRenderer;
use egui::{
    Align, CornerRadius, CursorIcon, Id, Key, Rect, Response, Sense, Stroke, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// What happened in a [`JBList`], [`Tree`], [`JBTable`] or [`TreeTable`] this frame.
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
    /// An inline edit was committed with <kbd>Enter</kbd> (or by leaving the field): the
    /// row, column and new text. The caller stores the text; the widget never changes data.
    pub edited: Option<CellEdit>,
    /// A [`JBTable`] header was dragged to a new place (with [`JBTable::reorderable`]): the new
    /// column order as indices into the columns you passed (`order[new_position] =
    /// old_index`). Reorder your columns with it; the table never does.
    pub column_order: Option<Vec<usize>>,
}

/// One drawn row: its response and whether its chevron asked to toggle.
struct DrawnRow {
    response: Response,
    toggled: bool,
    /// The row consumed its double-click itself (it started an edit): don't activate.
    handled: bool,
}

/// Keys the engine saw while the collection had focus, for the widgets that react to them.
#[derive(Default)]
struct Keys {
    /// <kbd>←</kbd> or <kbd>→</kbd> (trees).
    horizontal: Option<Key>,
    /// <kbd>Enter</kbd> was pressed (it also reports `activated`).
    enter: bool,
    /// <kbd>F2</kbd> was pressed.
    f2: bool,
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
) -> (CollectionResponse, Keys) {
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
    let mut keys = Keys::default();
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
                keys.enter = true;
            }
            keys.f2 = i.key_pressed(Key::F2);
            if i.key_pressed(Key::ArrowLeft) {
                keys.horizontal = Some(Key::ArrowLeft);
            } else if i.key_pressed(Key::ArrowRight) {
                keys.horizontal = Some(Key::ArrowRight);
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
                } else if drawn.response.double_clicked() && !drawn.handled {
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
            edited: None,
            column_order: None,
        },
        keys,
    )
}

/// A scrolling list of [`ColoredListCellRenderer`]s that draws only the visible ones.
///
/// <kbd>↑</kbd>/<kbd>↓</kbd>, <kbd>Home</kbd>/<kbd>End</kbd> and
/// <kbd>PgUp</kbd>/<kbd>PgDn</kbd> move the selection; <kbd>Enter</kbd> or a double-click
/// activates a row. Rows with an [`expanded`](ColoredListCellRenderer::expanded) state (a tree drawn from
/// `ColoredListCellRenderer`s) report <kbd>→</kbd> on a collapsed and <kbd>←</kbd> on an expanded selected
/// row in [`CollectionResponse::toggled`], like [`Tree`] does.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{ColoredListCellRenderer, JBList};
/// use archipel_kit::Glyph;
///
/// let requests: Vec<String> = (0..10_000).map(|i| format!("GET /users/{i}")).collect();
/// let mut selected = None;
/// let out = JBList::new("history", requests.len(), &mut selected)
///     .height(240.0)
///     .show(ui, |i| ColoredListCellRenderer::new(&requests[i]).icon(Glyph::Send));
/// if let Some(i) = out.activated { /* open requests[i] */ }
/// # });
/// ```
#[must_use = "show it with `list.show(ui, row)`"]
pub struct JBList<'a> {
    id: Id,
    name: String,
    len: usize,
    selected: &'a mut Option<usize>,
    height: f32,
}

impl<'a> JBList<'a> {
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
    pub fn show<'r>(
        self,
        ui: &mut Ui,
        mut row: impl FnMut(usize) -> ColoredListCellRenderer<'r>,
    ) -> CollectionResponse {
        // Whether the selected row is an expanded or collapsed node, from the row itself.
        let mut selected_node = None;
        let (mut response, keys) = collection(
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
                let row = row(index);
                if selected {
                    selected_node = Some((index, row.is_expanded()));
                }
                let shown = row
                    .selected(selected)
                    .inactive(!focused)
                    .focusable(false)
                    .show(ui);
                DrawnRow {
                    response: shown.response,
                    toggled: shown.toggled,
                    handled: false,
                }
            },
        );
        if let (Some(key), Some((index, Some(expanded)))) = (keys.horizontal, selected_node)
            && ((key == Key::ArrowRight && !expanded) || (key == Key::ArrowLeft && expanded))
        {
            response.toggled = Some(index);
        }
        response
    }
}

/// A visible row of a [`Tree`]: the caller flattens its tree into the rows that are
/// visible (children of expanded nodes) and keeps the expanded state.
#[derive(Debug, Clone)]
pub struct TreeNode {
    label: String,
    secondary: Option<String>,
    icon: Option<Icon>,
    icon_tone: Option<Tone>,
    busy: bool,
    depth: usize,
    children: bool,
    expanded: bool,
    editable: bool,
}

impl TreeNode {
    /// A row reading `label` at `depth` (0 is the root level).
    pub fn new(label: impl Into<String>, depth: usize) -> Self {
        Self {
            label: label.into(),
            secondary: None,
            icon: None,
            icon_tone: None,
            busy: false,
            depth,
            children: false,
            expanded: false,
            editable: false,
        }
    }

    /// The label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The key (first column of a [`TreeTable`]) can be edited inline.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
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

    /// Color the row's icon by meaning, using the theme's tones.
    pub fn icon_tone(mut self, tone: Tone) -> Self {
        self.icon_tone = Some(tone);
        self
    }

    /// Show an accessible loading indicator before the row's icon while children load.
    /// The caller owns the loading state and the row remains navigable.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
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
/// Keys as in [`JBList`], plus <kbd>→</kbd> expands a collapsed node or moves to its
/// first child, and <kbd>←</kbd> collapses an expanded node or moves to its parent.
/// Expanding and collapsing is reported in [`CollectionResponse::toggled`]: flip your
/// expanded state and rebuild the rows.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{Tree, TreeNode};
/// use archipel_kit::Glyph;
///
/// let mut src_open = true;
/// let mut selected = Some(1);
/// let mut rows = vec![TreeNode::new("src", 0).icon(Glyph::Folder).parent(src_open)];
/// if src_open {
///     rows.push(TreeNode::new("main.rs", 1));
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
    rows: &'a [TreeNode],
    selected: &'a mut Option<usize>,
    height: f32,
}

impl<'a> Tree<'a> {
    /// A tree of the visible `rows` with a unique `id_salt`, editing `selected`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        rows: &'a [TreeNode],
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
        self.show_with(ui, |_, row| row)
    }

    /// Like [`show`](Self::show), and `decorate(index, row)` may change each visible row
    /// before it is drawn: add a context menu or drag source with
    /// [`ColoredListCellRenderer::with`], a tint, a trailing slot, a semibold label …
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use archipel_kit::behaviour::{PopupHandler, DragSource, DropTarget};
    /// use archipel_kit::{action_system::ActionMenuItem, ui::{Tree, TreeNode}};
    ///
    /// let rows = vec![TreeNode::new("src", 0).parent(true), TreeNode::new("main.rs", 1)];
    /// let mut selected = None;
    /// let dropped = std::cell::Cell::new(None::<usize>);
    /// Tree::new("files", &rows, &mut selected).show_with(ui, |index, row| {
    ///     let row = row
    ///         .with(DragSource(index))
    ///         .with(PopupHandler::new(|ui| {
    ///             ui.add(ActionMenuItem::new("Rename…"));
    ///         }));
    ///     if rows[index].has_children() {
    ///         row.with(DropTarget::shared(&dropped))
    ///     } else {
    ///         row
    ///     }
    /// });
    /// # });
    /// ```
    pub fn show_with<'r>(
        self,
        ui: &mut Ui,
        mut decorate: impl FnMut(usize, ColoredListCellRenderer<'r>) -> ColoredListCellRenderer<'r>,
    ) -> CollectionResponse {
        let rows = self.rows;
        let (mut response, keys) = collection(
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
                let mut row = ColoredListCellRenderer::new(tree_row.label.as_str())
                    .depth(tree_row.depth)
                    .expanded(tree_row.children.then_some(tree_row.expanded))
                    .selected(selected)
                    .inactive(!focused)
                    .focusable(false);
                if let Some(icon) = tree_row.icon {
                    row = row.icon(icon);
                }
                if let Some(tone) = tree_row.icon_tone {
                    row = row.icon_tone(tone);
                }
                if tree_row.busy {
                    let name = format!("Loading {}", tree_row.label);
                    row = row.leading(crate::parts::Slot::ui_width(size::ICON, move |ui| {
                        ui.add(crate::ui::AsyncProcessIcon::new().label(name));
                    }));
                }
                if let Some(secondary) = &tree_row.secondary {
                    row = row.secondary(secondary.as_str());
                }
                let shown = decorate(index, row).show(ui);
                DrawnRow {
                    response: shown.response,
                    toggled: shown.toggled,
                    handled: false,
                }
            },
        );
        apply_tree_key(rows, self.selected, keys.horizontal, &mut response);
        response
    }
}

/// <kbd>→</kbd>/<kbd>←</kbd> on the selected row of a tree: expand, collapse, or move to the
/// first child or the parent.
fn apply_tree_key(
    rows: &[TreeNode],
    selected: &mut Option<usize>,
    key: Option<Key>,
    response: &mut CollectionResponse,
) {
    let (Some(key), Some(index)) = (key, *selected) else {
        return;
    };
    let row = &rows[index];
    let mut select = |target: usize| {
        if *selected != Some(target) {
            *selected = Some(target);
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

/// Sort direction of a [`JBTable`] column.
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

/// A [`JBTable`] column. The caller owns the columns; dragging a header edge changes
/// `width`.
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    title: String,
    /// Width in points.
    pub width: f32,
    align: Align,
    mono: bool,
}

impl ColumnInfo {
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

/// How a cell's text is colored: by meaning (a tone) or by value type.
#[derive(Debug, Clone, Copy)]
enum CellColor {
    Tone(Tone),
    Kind(ValueKind),
}

/// The content of a table cell.
#[derive(Debug, Clone, Default)]
pub struct Cell {
    text: String,
    icon: Option<Icon>,
    color: Option<CellColor>,
    editable: bool,
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
        self.color = Some(CellColor::Tone(tone));
        self
    }

    /// The cell can be edited inline (<kbd>F2</kbd>, <kbd>Enter</kbd> or a double-click) in
    /// widgets that support editing; the new text arrives in
    /// [`CollectionResponse::edited`].
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// The type of the value, colored like the code editor does ([`ValueKind::String`],
    /// [`ValueKind::Number`], [`ValueKind::Null`] …): for document and JSON views. Like a
    /// tone it keeps its color when the row is selected. Replaces an earlier
    /// [`tone`](Self::tone) (and the other way round): the last call wins.
    pub fn value_kind(mut self, kind: ValueKind) -> Self {
        self.color = Some(CellColor::Kind(kind));
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
pub(super) const CELL_PADDING: f32 = 8.0;
/// Width of the column resize handle, centered on the column edge.
const HANDLE_WIDTH: f32 = 7.0;
/// Narrowest a column can be dragged.
pub(super) const MIN_COLUMN: f32 = 40.0;

/// A virtualized table: a header with sort indicators and resizable columns, and rows
/// that are drawn only when visible. Query results, request history, headers.
///
/// Keys as in [`JBList`]. Clicking a header reports
/// [`CollectionResponse::sort_clicked`]; sort your rows and pass the new order with
/// [`JBTable::sort`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{Cell, ColumnInfo, SortOrder, JBTable};
///
/// let mut columns = vec![ColumnInfo::new("_id", 220.0).mono(), ColumnInfo::new("name", 140.0)];
/// let users = [("64f1…a1", "Ada"), ("64f1…a2", "Linus")];
/// let mut selected = None;
/// let mut sort = Some((1, SortOrder::Ascending));
/// let out = JBTable::new("users", &mut columns, users.len(), &mut selected)
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
pub struct JBTable<'a> {
    id: Id,
    name: String,
    columns: &'a mut [ColumnInfo],
    len: usize,
    selected: &'a mut Option<usize>,
    sort: Option<(usize, SortOrder)>,
    height: f32,
    reorderable: bool,
}

impl<'a> JBTable<'a> {
    /// A table of `len` rows in `columns` with a unique `id_salt`, editing `selected`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        columns: &'a mut [ColumnInfo],
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
            reorderable: false,
        }
    }

    /// The column and direction the rows are sorted by (shown in the header).
    pub fn sort(mut self, sort: Option<(usize, SortOrder)>) -> Self {
        self.sort = sort;
        self
    }

    /// Headers can be dragged to reorder the columns. Dropping one on another header reports
    /// the new order in [`CollectionResponse::column_order`]; you own the column order, so
    /// reorder your columns (and the indices your cell callback gets) with it.
    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
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
        let ctx = ui.ctx().clone();
        let (sort_clicked, column_order) =
            header(ui, self.id, self.columns, self.sort, self.reorderable);
        let columns: &[ColumnInfo] = self.columns;
        let id = self.id;
        let mut editing = edit::Frame::new(&ctx, id);
        let (mut response, keys) = collection(
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
                let mut x = rect.left() + size::ROW_PILL_INSET;
                let column_rects: Vec<Rect> = columns
                    .iter()
                    .map(|column| {
                        let cell_rect = Rect::from_min_max(
                            pos2(x, rect.top()),
                            pos2(x + column.width, rect.bottom()),
                        );
                        x += column.width;
                        cell_rect.intersect(rect)
                    })
                    .collect();
                // The edited column is the one clicked last; a double-click on an editable
                // cell opens the editor instead of activating the row.
                let clicked_column = || {
                    response.interact_pointer_pos().and_then(|pos| {
                        column_rects
                            .iter()
                            .position(|r| r.x_range().contains(pos.x))
                    })
                };
                let mut handled = false;
                if (response.clicked() || response.double_clicked())
                    && let Some(col) = clicked_column()
                {
                    edit::set_active_column(ui.ctx(), id, col);
                    if response.double_clicked() && cells[col].editable {
                        edit::begin(ui.ctx(), id, index, col, &cells[col].text);
                        handled = true;
                    }
                }
                if ui.is_rect_visible(rect) {
                    let state = crate::parts::State::of(&response)
                        .selected(selected)
                        .inactive(!focused);
                    let fill = state.row_fill(&colors);
                    common::row_pill(ui.painter(), rect, fill);
                    for (col, (column, cell)) in columns.iter().zip(cells).enumerate() {
                        if editing.is_editing(index, col) {
                            editing.show(ui, column_rects[col], column.title(), column.mono);
                        } else {
                            paint_cell(ui, column_rects[col], column, cell, selected);
                        }
                    }
                }
                DrawnRow {
                    response,
                    toggled: false,
                    handled,
                }
            },
        );
        editing.finish(
            &ctx,
            &mut response,
            &keys,
            *self.selected,
            columns.len(),
            |row, col| {
                let cell = cell(row, col);
                (cell.editable, cell.text)
            },
        );
        response.sort_clicked = sort_clicked;
        response.column_order = column_order;
        response
    }
}

/// A column header being dragged: the table it belongs to and its index.
#[derive(Clone)]
struct ColumnDrag {
    table: Id,
    index: usize,
}

/// Draws the header row; returns the column whose title was clicked, and the new column
/// order if a header was dropped on another.
pub(super) fn header(
    ui: &mut Ui,
    id: Id,
    columns: &mut [ColumnInfo],
    sort: Option<(usize, SortOrder)>,
    reorder: bool,
) -> (Option<usize>, Option<Vec<usize>>) {
    let colors = Theme::colors(ui.ctx());
    let height = size::ROW_HEIGHT + 4.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let font = typography::semibold(ui.ctx(), typography::BODY);
    let mut clicked = None;
    let mut moved: Option<(usize, usize)> = None;
    let mut x = rect.left() + size::ROW_PILL_INSET;
    for (index, column) in columns.iter_mut().enumerate() {
        let width = if column.width.is_finite() {
            column.width.max(0.0)
        } else {
            MIN_COLUMN
        };
        let full_cell = Rect::from_min_max(pos2(x, rect.top()), pos2(x + width, rect.bottom()));
        x += width;
        let cell = full_cell.intersect(rect);
        if cell.width() <= HANDLE_WIDTH || !ui.is_rect_visible(cell) {
            continue;
        }
        // The title's target stops short of the edges, which belong to the resize handles.
        let title = ui.interact(
            cell.shrink2(vec2(HANDLE_WIDTH / 2.0, 0.0)),
            id.with(("header", index)),
            Sense::click(),
        );
        crate::cursor::on_hover_action(&title);
        title.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &column.title));
        // Reordering: the header is a drag source and a drop target for columns of this
        // table. The drag cursor (open and closed hand) replaces the pointing hand.
        let title = if reorder {
            let title = DragSource(ColumnDrag { table: id, index }).attach(ui, title);
            let mut over: Option<ColumnDrag> = None;
            let title = DropTarget::new(&mut over).attach(ui, title);
            if let Some(from) = over.filter(|drag| drag.table == id) {
                moved = Some((from.index, index));
            }
            title
        } else {
            title
        };
        // After the drag source: its response senses the click too.
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
        // Only a column's actual edge is resizable. A clipped column must not acquire a
        // phantom resize handle at the viewport edge; fully hidden columns get no targets.
        if full_cell.right() > rect.right() {
            continue;
        }
        let edge = Rect::from_center_size(
            pos2(cell.right(), cell.center().y),
            vec2(HANDLE_WIDTH, height),
        )
        .intersect(rect);
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
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, colors.separator),
    );
    let order = moved.filter(|(from, to)| from != to).map(|(from, to)| {
        let mut order: Vec<usize> = (0..columns.len()).collect();
        let moved = order.remove(from);
        order.insert(to, moved);
        order
    });
    (clicked, order)
}

pub(super) fn paint_cell(ui: &Ui, rect: Rect, column: &ColumnInfo, cell: Cell, selected: bool) {
    let colors = Theme::colors(ui.ctx());
    let painter = ui
        .painter()
        .with_clip_rect(rect.shrink2(vec2(CELL_PADDING / 2.0, 0.0)));
    let font = if column.mono {
        egui::FontId::monospace(typography::CODE)
    } else {
        typography::body(typography::BODY)
    };
    let color = match (cell.color, selected) {
        (Some(CellColor::Tone(tone)), _) => tone.color(&colors),
        (Some(CellColor::Kind(kind)), _) => kind.resolve(ui.ctx()),
        (None, true) => colors.text_strong,
        (None, false) => colors.text,
    };
    let icon_width = if cell.icon.is_some() {
        size::ICON + size::ICON_TEXT_GAP
    } else {
        0.0
    };
    // Text wider than the column ends in an ellipsis, as in Android Studio's tables.
    let room = (rect.width() - 2.0 * CELL_PADDING - icon_width).max(0.0);
    let galley = common::galley_elided(ui, cell.text.as_str(), font, room);
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
