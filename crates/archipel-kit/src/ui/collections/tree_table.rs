//! `TreeTable`: a tree whose rows have columns.

use super::engine::{DrawnRow, RowCtx, Rows, SelectionTarget, Spec, TreeMove, collection};
use super::{edit, header, paint_cell, row_pill_joined, tree_horizontal};
use crate::common;
use crate::parts::{SimpleColoredComponent, State, TextFragment};
use crate::theme::Theme;
use crate::tokens::size;
use crate::ui::{
    Cell, CollectionResponse, ColumnInfo, SortOrder, SpeedSearch, StatusText, TreeNode,
};
use egui::{CursorIcon, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2};

/// A virtualized tree whose rows have columns: a MongoDB document or nested JSON as
/// key | value | type.
///
/// It is [`Tree`](crate::ui::Tree) and [`JBTable`](crate::ui::JBTable) in one, with the same model: you
/// own the data, the selection, the expanded state and the column widths; the widget owns
/// the keys and draws only the visible rows, so 100 000 rows stay fast.
///
/// - The **first column is the tree column**: chevron, icon and key come from the
///   [`TreeNode`]. The other columns come from `cell(row, column)`.
/// - Selection and keys as in [`JBList`](crate::ui::JBList) (<kbd>Shift</kbd> and
///   <kbd>Ctrl</kbd>/<kbd>Cmd</kbd> select several rows with a
///   [`ListSelectionModel`](crate::ui::ListSelectionModel)); <kbd>→</kbd>/<kbd>←</kbd> expand,
///   collapse or walk to the first child or the parent; <kbd>Enter</kbd> or a double-click
///   expands a parent and activates a leaf.
/// - **Inline edit**: mark cells [`editable`](Cell::editable) (the key with
///   [`TreeNode::editable`]). <kbd>F2</kbd>, <kbd>Enter</kbd> or a double-click on such a
///   cell opens a field in the cell; <kbd>Enter</kbd> commits, <kbd>Escape</kbd> cancels. A
///   commit arrives as [`CollectionResponse::edited`] (row, column and the new text): store
///   it and show it from your callback. The widget never changes your data.
///   <kbd>F2</kbd> and <kbd>Enter</kbd> edit the column you last clicked (the first
///   editable one otherwise); a row with nothing editable there activates on
///   <kbd>Enter</kbd> as usual.
/// - Clicking a header reports [`CollectionResponse::sort_clicked`]; dragging its right edge
///   resizes the column.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{Cell, ColumnInfo, TreeNode, TreeTable};
/// use archipel_kit::Glyph;
///
/// let mut columns = vec![
///     ColumnInfo::new("Key", 180.0),
///     ColumnInfo::new("Value", 200.0).mono(),
///     ColumnInfo::new("Type", 80.0),
/// ];
/// let mut name = "Ada".to_owned();
/// let mut address_open = true;
/// let mut rows = vec![
///     TreeNode::new("name", 0).editable(true),
///     TreeNode::new("address", 0).icon(Glyph::Folder).parent(address_open),
/// ];
/// if address_open {
///     rows.push(TreeNode::new("city", 1));
/// }
/// let mut selected = None;
/// let out = TreeTable::new("document", &mut columns, &rows, &mut selected).show(
///     ui,
///     |row, col| match (row, col) {
///         (0, 1) => Cell::new(name.as_str()).editable(true),
///         (0, _) => Cell::new("String").muted(),
///         (1, 1) => Cell::new("{ 1 field }").muted(),
///         (1, _) => Cell::new("Object").muted(),
///         (_, 1) => Cell::new("London"),
///         _ => Cell::new("String").muted(),
///     },
/// );
/// if let Some(i) = out.toggled {
///     address_open = !address_open;
/// }
/// if let Some(edit) = out.edited {
///     if (edit.row, edit.column) == (0, 1) {
///         name = edit.text;
///     }
/// }
/// # });
/// ```
#[must_use = "show it with `tree_table.show(ui, cell)`"]
pub struct TreeTable<'a> {
    id: Id,
    name: String,
    columns: &'a mut [ColumnInfo],
    rows: &'a [TreeNode],
    selected: SelectionTarget<'a>,
    sort: Option<(usize, SortOrder)>,
    height: f32,
    search: Option<SpeedSearch<'a>>,
    empty: Option<StatusText>,
}

impl<'a> TreeTable<'a> {
    /// A tree table of the visible `rows` in `columns` with a unique `id_salt`, editing
    /// `selected` (a `&mut Option<usize>` or a `&mut ListSelectionModel`).
    pub fn new(
        id_salt: impl std::hash::Hash,
        columns: &'a mut [ColumnInfo],
        rows: &'a [TreeNode],
        selected: impl Into<SelectionTarget<'a>>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "Tree table".into(),
            columns,
            rows,
            selected: selected.into(),
            sort: None,
            height: 240.0,
            search: None,
            empty: None,
        }
    }

    /// The column and direction the rows are sorted by (shown in the header).
    pub fn sort(mut self, sort: Option<(usize, SortOrder)>) -> Self {
        self.sort = sort;
        self
    }

    /// The accessible name, e.g. "Document" (default "Tree table").
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Height of the rows area in points, without the header (default 240).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Lets the user type to search the visible rows: a row matches when its key or one of
    /// its cells does (see [`JBList::speed_search`](crate::ui::JBList::speed_search)).
    pub fn speed_search(mut self, search: SpeedSearch<'a>) -> Self {
        self.search = Some(search);
        self
    }

    /// What to show when there are no rows (default: "Nothing to show").
    pub fn empty_text(mut self, text: StatusText) -> Self {
        self.empty = Some(text);
        self
    }

    /// Draws the header and the visible rows; `cell(row, column)` gives the cells of the
    /// columns after the first.
    pub fn show(
        self,
        ui: &mut Ui,
        mut cell: impl FnMut(usize, usize) -> Cell,
    ) -> CollectionResponse {
        struct TreeTableRows<'t, F> {
            id: Id,
            columns: &'t [ColumnInfo],
            rows: &'t [TreeNode],
            cell: &'t mut F,
            editing: &'t mut edit::Frame,
        }
        impl<F: FnMut(usize, usize) -> Cell> Rows for TreeTableRows<'_, F> {
            fn texts(&mut self, index: usize) -> Vec<String> {
                let mut texts = vec![self.rows[index].search_text()];
                texts.extend((1..self.columns.len()).map(|col| (self.cell)(index, col).text));
                texts
            }

            fn horizontal(&mut self, index: usize, left: bool) -> Option<TreeMove> {
                tree_horizontal(self.rows, index, left)
            }

            fn enter_toggles(&mut self, index: usize) -> bool {
                self.rows.get(index).is_some_and(|row| row.has_children())
            }

            fn row(&mut self, ui: &mut Ui, ctx: RowCtx<'_>) -> DrawnRow {
                let colors = Theme::colors(ui.ctx());
                let (index, selected) = (ctx.index, ctx.selected);
                let (id, columns, editing) = (self.id, self.columns, &mut *self.editing);
                let tree_row = &self.rows[index];
                let (rect, response) = ui.allocate_exact_size(
                    vec2(
                        ui.available_width(),
                        crate::tokens::metrics(ui.ctx()).row_height,
                    ),
                    Sense::CLICK,
                );
                crate::cursor::on_hover(&response, CursorIcon::Default);
                let cells: Vec<Cell> = (1..columns.len())
                    .map(|col| (self.cell)(index, col))
                    .collect();
                response.widget_info(|| {
                    let mut label = vec![tree_row.label.as_str()];
                    label.extend(cells.iter().map(|c| c.text.as_str()));
                    WidgetInfo::selected(
                        WidgetType::SelectableLabel,
                        true,
                        selected,
                        label.join(", "),
                    )
                });

                // The chevron sits where a `ColoredListCellRenderer`'s does, so the key column lines up with
                // a `Tree`.
                let chevron = Rect::from_min_size(
                    pos2(
                        rect.left() + crate::tokens::metrics(ui.ctx()).indent(tree_row.depth)
                            - crate::tokens::metrics(ui.ctx()).row_pill_inset,
                        rect.center().y - size::CHEVRON_COLUMN / 2.0,
                    ),
                    vec2(size::CHEVRON_COLUMN, size::CHEVRON_COLUMN),
                );
                let mut toggled = false;
                if tree_row.children {
                    let target = ui.interact(
                        chevron.expand(2.0),
                        response.id.with("chevron"),
                        Sense::CLICK,
                    );
                    let action = if tree_row.expanded {
                        "Collapse"
                    } else {
                        "Expand"
                    };
                    target.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            true,
                            format!("{action} {}", tree_row.label),
                        )
                    });
                    toggled = target.clicked();
                }

                let mut column_rects = Vec::with_capacity(columns.len());
                let mut x = rect.left() + crate::tokens::metrics(ui.ctx()).row_pill_inset;
                for column in columns {
                    column_rects.push(
                        Rect::from_min_max(
                            pos2(x, rect.top()),
                            pos2(x + column.width, rect.bottom()),
                        )
                        .intersect(rect),
                    );
                    x += column.width;
                }
                let editable = |col: usize| match col {
                    0 => tree_row.editable,
                    _ => cells[col - 1].editable,
                };
                let text_of = |col: usize| match col {
                    0 => tree_row.label.clone(),
                    _ => cells[col - 1].text.clone(),
                };
                let clicked_column = || {
                    response.interact_pointer_pos().and_then(|pos| {
                        column_rects
                            .iter()
                            .position(|r| r.x_range().contains(pos.x))
                    })
                };
                if (response.clicked() || response.double_clicked())
                    && !toggled
                    && let Some(col) = clicked_column()
                {
                    edit::set_active_column(ui.ctx(), id, col);
                }
                let mut handled = false;
                if response.double_clicked() && !toggled {
                    match clicked_column() {
                        Some(col) if editable(col) => {
                            edit::begin(ui.ctx(), id, index, col, &text_of(col));
                            handled = true;
                        }
                        _ if tree_row.children => toggled = true,
                        _ => {}
                    }
                }

                if ui.is_rect_visible(rect) {
                    let state = State::of(&response)
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
                            rect.shrink2(vec2(
                                crate::tokens::metrics(ui.ctx()).row_pill_inset,
                                0.0,
                            ))
                            .shrink(1.0),
                            crate::tokens::metrics(ui.ctx()).row_radius,
                            &colors,
                        );
                    }

                    // The key column: chevron, icon and key (or its editor).
                    let key_rect = column_rects[0];
                    let mut clip = ui.new_child(UiBuilder::new().max_rect(key_rect));
                    clip.set_clip_rect(key_rect.intersect(ui.clip_rect()));
                    if tree_row.children {
                        crate::parts::paint_expander(
                            clip.painter(),
                            chevron,
                            tree_row.expanded,
                            colors.text_secondary,
                        );
                    }
                    let mut content_x = chevron.right() + 2.0;
                    if tree_row.busy {
                        let name = format!("Loading {}", tree_row.label);
                        crate::parts::Slot::ui_width(size::ICON, move |ui| {
                            ui.add(crate::ui::AsyncProcessIcon::new().label(name));
                        })
                        .show(
                            &mut clip,
                            Rect::from_min_size(
                                pos2(content_x, key_rect.top()),
                                vec2(size::ICON, key_rect.height()),
                            ),
                            state,
                        );
                        content_x += size::ICON + size::ROW_LEADING_GAP;
                    }
                    let label_x = content_x
                        + if tree_row.icon.is_some() {
                            size::ICON + size::ICON_TEXT_GAP
                        } else {
                            0.0
                        };
                    let key_editing = editing.is_editing(index, 0);
                    let fragments: Vec<TextFragment> = tree_row
                        .fragments
                        .iter()
                        .map(|(text, attributes)| TextFragment::new(text.as_str(), *attributes))
                        .collect();
                    let matches = ctx
                        .search
                        .and_then(|search| search.matching_fragments(&tree_row.search_text()))
                        .unwrap_or_default();
                    let content = SimpleColoredComponent {
                        icon: tree_row.icon,
                        icon_tone: tree_row.icon_tone,
                        label: if key_editing { "" } else { &tree_row.label },
                        secondary: tree_row.secondary.as_deref().filter(|_| !key_editing),
                        fragments: if key_editing { &[] } else { &fragments },
                        matches: if key_editing { &[] } else { &matches },
                        ..SimpleColoredComponent::new("")
                    };
                    content.paint(&clip, key_rect, content_x, state);
                    if key_editing {
                        let field = Rect::from_min_max(
                            pos2(label_x - super::CELL_PADDING, key_rect.top()),
                            key_rect.right_bottom(),
                        );
                        editing.show(ui, field, columns[0].title(), false);
                    }

                    for ((col, column), cell) in columns.iter().enumerate().skip(1).zip(cells) {
                        if editing.is_editing(index, col) {
                            editing.show(ui, column_rects[col], column.title(), column.mono);
                        } else {
                            paint_cell(ui, column_rects[col], column, cell, state, ctx.search);
                        }
                    }
                }
                DrawnRow {
                    response,
                    toggled,
                    handled,
                }
            }
        }

        let Self {
            id,
            name,
            columns,
            rows,
            selected,
            sort,
            height,
            search,
            empty,
        } = self;
        let ctx = ui.ctx().clone();
        let (sort_clicked, _) = header(ui, id, columns, sort, false);
        let columns: &[ColumnInfo] = columns;
        let mut editing = edit::Frame::new(&ctx, id);

        let (mut response, keys) = collection(
            ui,
            Spec {
                id,
                len: rows.len(),
                height,
                role: egui::accesskit::Role::TreeGrid,
                name: &name,
                search,
                empty,
            },
            selected,
            &mut TreeTableRows {
                id,
                columns,
                rows,
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
            |row, col| match col {
                0 => (rows[row].editable, rows[row].label.clone()),
                _ => {
                    let cell = cell(row, col);
                    (cell.editable, cell.text)
                }
            },
        );
        response.sort_clicked = sort_clicked;
        response
    }
}
