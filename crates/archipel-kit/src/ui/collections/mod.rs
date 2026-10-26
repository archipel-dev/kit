//! Virtualized collections: lists, trees and tables that draw only the visible rows, so they
//! stay fast with hundreds of thousands of items.
//!
//! All of them share one model: the caller owns the data and the selection (a
//! `&mut Option<usize>`, or a [`ListSelectionModel`](crate::ui::ListSelectionModel) for
//! several rows), the widget draws the visible rows, moves the selection with the keyboard
//! and the mouse, searches as the user types ([`SpeedSearch`]) and reports what happened in a
//! [`CollectionResponse`].

mod edit;
mod engine;
mod tree_table;

pub use edit::CellEdit;
pub use engine::SelectionTarget;
pub use tree_table::TreeTable;

use crate::behaviour::{Behaviour, DragSource, DropTarget};
use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::{SimpleTextAttributes, Tone, ValueKind};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use crate::ui::{ColoredListCellRenderer, SpeedSearch, SpeedSearchComparator, StatusText};
use egui::{
    Align, CornerRadius, CursorIcon, Id, Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};
use engine::{DrawnRow, RowCtx, Rows, Spec, TreeMove, collection};

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
    /// <kbd>Enter</kbd> on a parent, <kbd>→</kbd>/<kbd>←</kbd>.
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
    /// The row that was right-clicked this frame. The click has already followed the IDE's
    /// rule: the selection stays when the row is inside it, else it moves to the row. Open
    /// your context menu for the selection; attach it with `PopupHandler` on the rows, or
    /// react to this.
    pub secondary_clicked: Option<usize>,
    /// The link of the collection's [empty text](StatusText) that was clicked, counting the
    /// links from 0.
    pub empty_text_link: Option<usize>,
    /// The row the keyboard acts on: the lead row if it is selected, else the first
    /// selected row.
    pub current: Option<usize>,
}

/// A scrolling list of [`ColoredListCellRenderer`]s that draws only the visible ones.
///
/// - <kbd>↑</kbd>/<kbd>↓</kbd>, <kbd>Home</kbd>/<kbd>End</kbd> and
///   <kbd>PgUp</kbd>/<kbd>PgDn</kbd> move the selection (the page keys first go to the
///   edge of the view, then a page further); <kbd>Shift</kbd> extends it, <kbd>Ctrl</kbd>
///   (<kbd>Cmd</kbd> on macOS) moves the lead without selecting, <kbd>Ctrl</kbd>+<kbd>A</kbd>
///   selects all. <kbd>Enter</kbd> or a double-click activates a row.
/// - Click selects, <kbd>Ctrl</kbd>/<kbd>Cmd</kbd>+click toggles, <kbd>Shift</kbd>+click
///   extends from the anchor. A right click keeps the selection if it lands inside it, else
///   selects the row.
/// - Typing opens a [speed search](Self::speed_search).
/// - Rows with an [`expanded`](ColoredListCellRenderer::expanded) state (a tree drawn from
///   `ColoredListCellRenderer`s) report <kbd>→</kbd> on a collapsed and <kbd>←</kbd> on an
///   expanded selected row in [`CollectionResponse::toggled`], like [`Tree`] does.
///
/// Pass `&mut Option<usize>` for a single selection or a
/// [`ListSelectionModel`](crate::ui::ListSelectionModel) for several rows.
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
///
/// Several rows, with a speed search:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{
///     ColoredListCellRenderer, JBList, ListSelectionModel, SpeedSearch, SpeedSearchState,
/// };
///
/// let files = ["Cargo.toml", "README.md", "src"];
/// let mut selection = ListSelectionModel::multiple();
/// let mut search = SpeedSearchState::new();
/// JBList::new("files", files.len(), &mut selection)
///     .speed_search(SpeedSearch::new(&mut search))
///     .show(ui, |i| ColoredListCellRenderer::new(files[i]));
/// for index in selection.iter() { /* act on files[index] */ }
/// # });
/// ```
#[must_use = "show it with `list.show(ui, row)`"]
pub struct JBList<'a> {
    id: Id,
    name: String,
    len: usize,
    selected: SelectionTarget<'a>,
    height: f32,
    search: Option<SpeedSearch<'a>>,
    empty: Option<StatusText>,
}

impl<'a> JBList<'a> {
    /// A list of `len` rows with a unique `id_salt`, editing `selected` (a
    /// `&mut Option<usize>` or a `&mut ListSelectionModel`).
    pub fn new(
        id_salt: impl std::hash::Hash,
        len: usize,
        selected: impl Into<SelectionTarget<'a>>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "List".into(),
            len,
            selected: selected.into(),
            height: 240.0,
            search: None,
            empty: None,
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

    /// Lets the user type to search: opens on a letter, digit or punctuation mark, moves
    /// the selection to the first match, highlights the matches in the rows and keeps the
    /// text in the caller's [`SpeedSearchState`](crate::ui::SpeedSearchState).
    pub fn speed_search(mut self, search: SpeedSearch<'a>) -> Self {
        self.search = Some(search);
        self
    }

    /// What to show when the list has no rows (default: "Nothing to show").
    pub fn empty_text(mut self, text: StatusText) -> Self {
        self.empty = Some(text);
        self
    }

    /// Draws the visible rows; `row(index)` describes row `index` (the list sets its
    /// selection).
    pub fn show<'r>(
        self,
        ui: &mut Ui,
        row: impl FnMut(usize) -> ColoredListCellRenderer<'r>,
    ) -> CollectionResponse {
        struct ListRows<F>(F);
        impl<'r, F: FnMut(usize) -> ColoredListCellRenderer<'r>> Rows for ListRows<F> {
            fn texts(&mut self, index: usize) -> Vec<String> {
                vec![(self.0)(index).search_text()]
            }

            fn row(&mut self, ui: &mut Ui, ctx: RowCtx<'_>) -> DrawnRow {
                let shown = (self.0)(ctx.index)
                    .selected(ctx.selected)
                    .inactive(!ctx.focused)
                    .focusable(false)
                    .joined(ctx.joined_above, ctx.joined_below)
                    .lead(ctx.lead)
                    .search(ctx.search)
                    .show(ui);
                DrawnRow {
                    response: shown.response,
                    toggled: shown.toggled,
                    handled: false,
                }
            }

            fn horizontal(&mut self, index: usize, left: bool) -> Option<TreeMove> {
                // A row with an expanded state toggles on the arrow that changes it.
                match (self.0)(index).is_expanded() {
                    Some(expanded) if left == expanded => Some(TreeMove::Toggle),
                    _ => None,
                }
            }
        }
        let (response, _) = collection(
            ui,
            Spec {
                id: self.id,
                len: self.len,
                height: self.height,
                role: egui::accesskit::Role::List,
                name: &self.name,
                search: self.search,
                empty: self.empty,
            },
            self.selected,
            &mut ListRows(row),
        );
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
    fragments: Vec<(String, SimpleTextAttributes)>,
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
            fragments: Vec::new(),
        }
    }

    /// Adds a fragment of text after the label with its own [`SimpleTextAttributes`]
    /// (grayed, bold, error …). A speed search matches and highlights it with the label.
    pub fn append(mut self, text: impl Into<String>, attributes: SimpleTextAttributes) -> Self {
        self.fragments.push((text.into(), attributes));
        self
    }

    /// The label and the fragments, the text a speed search matches against.
    pub(crate) fn search_text(&self) -> String {
        let mut text = self.label.clone();
        for (fragment, _) in &self.fragments {
            text.push_str(fragment);
        }
        text
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
/// first child, <kbd>←</kbd> collapses an expanded node or moves to its parent, and
/// <kbd>Enter</kbd> expands or collapses a node with children (a leaf is activated).
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
    selected: SelectionTarget<'a>,
    height: f32,
    search: Option<SpeedSearch<'a>>,
    empty: Option<StatusText>,
}

impl<'a> Tree<'a> {
    /// A tree of the visible `rows` with a unique `id_salt`, editing `selected` (a
    /// `&mut Option<usize>` or a `&mut ListSelectionModel`).
    pub fn new(
        id_salt: impl std::hash::Hash,
        rows: &'a [TreeNode],
        selected: impl Into<SelectionTarget<'a>>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "Tree".into(),
            rows,
            selected: selected.into(),
            height: 240.0,
            search: None,
            empty: None,
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

    /// Lets the user type to search the visible rows (see
    /// [`JBList::speed_search`]).
    pub fn speed_search(mut self, search: SpeedSearch<'a>) -> Self {
        self.search = Some(search);
        self
    }

    /// What to show when the tree has no rows (default: "Nothing to show").
    pub fn empty_text(mut self, text: StatusText) -> Self {
        self.empty = Some(text);
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
        decorate: impl FnMut(usize, ColoredListCellRenderer<'r>) -> ColoredListCellRenderer<'r>,
    ) -> CollectionResponse {
        struct TreeRows<'t, F> {
            rows: &'t [TreeNode],
            decorate: F,
        }
        impl<'r, F: FnMut(usize, ColoredListCellRenderer<'r>) -> ColoredListCellRenderer<'r>> Rows
            for TreeRows<'_, F>
        {
            fn texts(&mut self, index: usize) -> Vec<String> {
                vec![self.rows[index].search_text()]
            }

            fn row(&mut self, ui: &mut Ui, ctx: RowCtx<'_>) -> DrawnRow {
                let tree_row = &self.rows[ctx.index];
                let mut row = ColoredListCellRenderer::new(tree_row.label.as_str())
                    .depth(tree_row.depth)
                    .expanded(tree_row.children.then_some(tree_row.expanded))
                    .selected(ctx.selected)
                    .inactive(!ctx.focused)
                    .focusable(false)
                    .joined(ctx.joined_above, ctx.joined_below)
                    .lead(ctx.lead);
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
                for (text, attributes) in &tree_row.fragments {
                    row = row.append(text.as_str(), *attributes);
                }
                if let Some(secondary) = &tree_row.secondary {
                    row = row.secondary(secondary.as_str());
                }
                let shown = (self.decorate)(ctx.index, row).search(ctx.search).show(ui);
                DrawnRow {
                    response: shown.response,
                    toggled: shown.toggled,
                    handled: false,
                }
            }

            fn horizontal(&mut self, index: usize, left: bool) -> Option<TreeMove> {
                tree_horizontal(self.rows, index, left)
            }

            fn enter_toggles(&mut self, index: usize) -> bool {
                self.rows.get(index).is_some_and(|row| row.children)
            }
        }
        let rows = self.rows;
        let (response, _) = collection(
            ui,
            Spec {
                id: self.id,
                len: rows.len(),
                height: self.height,
                role: egui::accesskit::Role::Tree,
                name: &self.name,
                search: self.search,
                empty: self.empty,
            },
            self.selected,
            &mut TreeRows { rows, decorate },
        );
        response
    }
}

/// <kbd>→</kbd>/<kbd>←</kbd> on the lead row of a tree: expand, collapse, or move to the first
/// child or the parent.
fn tree_horizontal(rows: &[TreeNode], index: usize, left: bool) -> Option<TreeMove> {
    let row = rows.get(index)?;
    if !left {
        if row.children && !row.expanded {
            return Some(TreeMove::Toggle);
        }
        if row.is_expanded() {
            return rows
                .get(index + 1)
                .filter(|next| next.depth > row.depth)
                .map(|_| TreeMove::Select(index + 1));
        }
        return None;
    }
    if row.is_expanded() {
        return Some(TreeMove::Toggle);
    }
    rows[..index]
        .iter()
        .rposition(|r| r.depth < row.depth)
        .map(TreeMove::Select)
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
    selected: SelectionTarget<'a>,
    sort: Option<(usize, SortOrder)>,
    height: f32,
    reorderable: bool,
    search: Option<SpeedSearch<'a>>,
    empty: Option<StatusText>,
}

impl<'a> JBTable<'a> {
    /// A table of `len` rows in `columns` with a unique `id_salt`, editing `selected` (a
    /// `&mut Option<usize>` or a `&mut ListSelectionModel`).
    pub fn new(
        id_salt: impl std::hash::Hash,
        columns: &'a mut [ColumnInfo],
        len: usize,
        selected: impl Into<SelectionTarget<'a>>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "Table".into(),
            columns,
            len,
            selected: selected.into(),
            sort: None,
            height: 240.0,
            reorderable: false,
            search: None,
            empty: None,
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

    /// Lets the user type to search the cells: a row matches when one of its cells does, and
    /// the matching part of the cell is highlighted (see [`JBList::speed_search`]).
    pub fn speed_search(mut self, search: SpeedSearch<'a>) -> Self {
        self.search = Some(search);
        self
    }

    /// What to show when the table has no rows (default: "Nothing to show").
    pub fn empty_text(mut self, text: StatusText) -> Self {
        self.empty = Some(text);
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
        struct TableRows<'t, F> {
            id: Id,
            columns: &'t [ColumnInfo],
            cell: &'t mut F,
            editing: &'t mut edit::Frame,
        }
        impl<F: FnMut(usize, usize) -> Cell> Rows for TableRows<'_, F> {
            fn texts(&mut self, index: usize) -> Vec<String> {
                (0..self.columns.len())
                    .map(|col| (self.cell)(index, col).text)
                    .collect()
            }

            fn row(&mut self, ui: &mut Ui, ctx: RowCtx<'_>) -> DrawnRow {
                let colors = Theme::colors(ui.ctx());
                let (index, selected) = (ctx.index, ctx.selected);
                let (rect, response) = ui.allocate_exact_size(
                    vec2(ui.available_width(), size::ROW_HEIGHT),
                    Sense::CLICK,
                );
                crate::cursor::on_hover(&response, CursorIcon::Default);
                let columns = self.columns;
                let cells: Vec<Cell> = (0..columns.len())
                    .map(|col| (self.cell)(index, col))
                    .collect();
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
                    edit::set_active_column(ui.ctx(), self.id, col);
                    if response.double_clicked() && cells[col].editable {
                        edit::begin(ui.ctx(), self.id, index, col, &cells[col].text);
                        handled = true;
                    }
                }
                if ui.is_rect_visible(rect) {
                    let state = crate::parts::State::of(&response)
                        .selected(selected)
                        .inactive(!ctx.focused);
                    row_pill_joined(
                        ui.painter(),
                        rect,
                        state.row_fill(&colors),
                        ctx.joined_above,
                        ctx.joined_below,
                    );
                    if ctx.lead {
                        common::focus_ring(
                            ui.painter(),
                            rect.shrink2(vec2(size::ROW_PILL_INSET, 0.0)).shrink(1.0),
                            radius::ROW,
                            &colors,
                        );
                    }
                    for (col, (column, cell)) in columns.iter().zip(cells).enumerate() {
                        if self.editing.is_editing(index, col) {
                            self.editing
                                .show(ui, column_rects[col], column.title(), column.mono);
                        } else {
                            paint_cell(ui, column_rects[col], column, cell, selected, ctx.search);
                        }
                    }
                }
                DrawnRow {
                    response,
                    toggled: false,
                    handled,
                }
            }
        }

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
                search: self.search,
                empty: self.empty,
            },
            self.selected,
            &mut TableRows {
                id,
                columns,
                cell: &mut cell,
                editing: &mut editing,
            },
        );
        let current = response.current;
        editing.finish(
            &ctx,
            &mut response,
            &keys,
            current,
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

/// The rounded highlight behind a row spanning `rect` (inset from the edges), in `fill`.
/// Corners that touch a selected neighbor row (`above`, `below`) are square, so a block of
/// selected rows reads as one shape.
pub(super) fn row_pill_joined(
    painter: &egui::Painter,
    rect: Rect,
    fill: egui::Color32,
    above: bool,
    below: bool,
) {
    if fill == egui::Color32::TRANSPARENT {
        return;
    }
    let pill = rect.shrink2(vec2(size::ROW_PILL_INSET, 0.0));
    let (top, bottom) = (
        if above { 0 } else { radius::ROW },
        if below { 0 } else { radius::ROW },
    );
    painter.rect_filled(
        pill,
        CornerRadius {
            nw: top,
            ne: top,
            sw: bottom,
            se: bottom,
        },
        fill,
    );
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

pub(super) fn paint_cell(
    ui: &Ui,
    rect: Rect,
    column: &ColumnInfo,
    cell: Cell,
    selected: bool,
    search: Option<&SpeedSearchComparator>,
) {
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
    // What the speed search matched, behind the text.
    if let Some(ranges) = search.and_then(|search| search.matching_fragments(&cell.text)) {
        crate::parts::paint_ranges(
            &painter,
            &galley,
            pos2(x, rect.center().y - galley.size().y / 2.0),
            &ranges,
            colors.search_match,
        );
    }
    common::text_left(&painter, rect, x, galley, color);
}
