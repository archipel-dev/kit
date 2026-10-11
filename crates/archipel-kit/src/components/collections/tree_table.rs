//! `TreeTable`: a tree whose rows have columns.

use super::edit::{self, Outcome};
use super::{
    Cell, CollectionResponse, Column, DrawnRow, SortOrder, Spec, TreeRow, apply_tree_key,
    collection, header, paint_cell,
};
use crate::components::common;
use crate::icons::{self, Glyph};
use crate::parts::{RowContent, State};
use crate::theme::Theme;
use crate::tokens::size;
use egui::{CursorIcon, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2};

/// A virtualized tree whose rows have columns: a MongoDB document or nested JSON as
/// key | value | type.
///
/// It is [`Tree`](super::Tree) and [`Table`](super::Table) in one, with the same model: you
/// own the data, the selection, the expanded state and the column widths; the widget owns
/// the keys and draws only the visible rows, so 100 000 rows stay fast.
///
/// - The **first column is the tree column**: chevron, icon and key come from the
///   [`TreeRow`]. The other columns come from `cell(row, column)`.
/// - <kbd>↑</kbd>/<kbd>↓</kbd>, <kbd>Home</kbd>/<kbd>End</kbd>, <kbd>PgUp</kbd>/<kbd>PgDn</kbd>
///   move the selection; <kbd>→</kbd>/<kbd>←</kbd> expand, collapse or walk to the first
///   child or the parent; <kbd>Enter</kbd> or a double-click activates a row.
/// - **Inline edit**: mark cells [`editable`](Cell::editable) (the key with
///   [`TreeRow::editable`]). <kbd>F2</kbd>, <kbd>Enter</kbd> or a double-click on such a
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
/// use archipel_kit::components::{Cell, Column, TreeRow, TreeTable};
/// use archipel_kit::Glyph;
///
/// let mut columns = vec![
///     Column::new("Key", 180.0),
///     Column::new("Value", 200.0).mono(),
///     Column::new("Type", 80.0),
/// ];
/// let mut name = "Ada".to_owned();
/// let mut address_open = true;
/// let mut rows = vec![
///     TreeRow::new("name", 0).editable(true),
///     TreeRow::new("address", 0).icon(Glyph::Folder).parent(address_open),
/// ];
/// if address_open {
///     rows.push(TreeRow::new("city", 1));
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
    columns: &'a mut [Column],
    rows: &'a [TreeRow],
    selected: &'a mut Option<usize>,
    sort: Option<(usize, SortOrder)>,
    height: f32,
}

impl<'a> TreeTable<'a> {
    /// A tree table of the visible `rows` in `columns` with a unique `id_salt`, editing
    /// `selected`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        columns: &'a mut [Column],
        rows: &'a [TreeRow],
        selected: &'a mut Option<usize>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            name: "Tree table".into(),
            columns,
            rows,
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

    /// Draws the header and the visible rows; `cell(row, column)` gives the cells of the
    /// columns after the first.
    pub fn show(
        self,
        ui: &mut Ui,
        mut cell: impl FnMut(usize, usize) -> Cell,
    ) -> CollectionResponse {
        let Self {
            id,
            name,
            columns,
            rows,
            selected,
            sort,
            height,
        } = self;
        let ctx = ui.ctx().clone();
        let colors = Theme::colors(&ctx);
        let sort_clicked = header(ui, id, columns, sort);
        let columns: &[Column] = columns;
        let editing = edit::current(&ctx, id);
        let mut editor_drawn = false;
        let mut outcome = None;

        let (mut response, keys) = collection(
            ui,
            Spec {
                id,
                len: rows.len(),
                height,
                role: egui::accesskit::Role::TreeGrid,
                name: &name,
            },
            selected,
            |ui, index, selected, focused| {
                let tree_row = &rows[index];
                let (rect, response) = ui.allocate_exact_size(
                    vec2(ui.available_width(), size::ROW_HEIGHT),
                    Sense::CLICK,
                );
                crate::cursor::on_hover(&response, CursorIcon::Default);
                let cells: Vec<Cell> = (1..columns.len()).map(|col| cell(index, col)).collect();
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

                // The chevron sits where a `ListRow`'s does, so the key column lines up with
                // a `Tree`.
                let chevron = Rect::from_min_size(
                    pos2(
                        rect.left() + size::indent(tree_row.depth) - size::ROW_PILL_INSET,
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
                let mut x = rect.left() + size::ROW_PILL_INSET;
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
                    let state = State::of(&response).selected(selected).inactive(!focused);
                    common::row_pill(ui.painter(), rect, state.row_fill(&colors));
                    let editing_here = |col: usize| editing == Some((index, col));

                    // The key column: chevron, icon and key (or its editor).
                    let key_rect = column_rects[0];
                    let mut clip = ui.new_child(UiBuilder::new().max_rect(key_rect));
                    clip.set_clip_rect(key_rect.intersect(ui.clip_rect()));
                    if tree_row.children {
                        let glyph = if tree_row.expanded {
                            Glyph::ChevronDown
                        } else {
                            Glyph::ChevronRight
                        };
                        icons::paint(clip.painter(), chevron, glyph, colors.text_secondary);
                    }
                    let content_x = chevron.right() + 2.0;
                    let label_x = content_x
                        + if tree_row.icon.is_some() {
                            size::ICON + size::ICON_TEXT_GAP
                        } else {
                            0.0
                        };
                    let key_editing = editing_here(0);
                    let content = RowContent {
                        icon: tree_row.icon,
                        label: if key_editing { "" } else { &tree_row.label },
                        secondary: tree_row.secondary.as_deref().filter(|_| !key_editing),
                        ..RowContent::new("")
                    };
                    content.paint(&clip, key_rect, content_x, state);
                    if key_editing {
                        let field = Rect::from_min_max(
                            pos2(label_x - super::CELL_PADDING, key_rect.top()),
                            key_rect.right_bottom(),
                        );
                        editor_drawn = true;
                        outcome = Some(edit::show(ui, id, field, columns[0].title(), false));
                    }

                    for ((col, column), cell) in columns.iter().enumerate().skip(1).zip(cells) {
                        if editing_here(col) {
                            editor_drawn = true;
                            outcome = Some(edit::show(
                                ui,
                                id,
                                column_rects[col],
                                column.title(),
                                column.mono,
                            ));
                        } else {
                            paint_cell(ui, column_rects[col], column, cell, selected);
                        }
                    }
                }
                DrawnRow {
                    response,
                    toggled,
                    handled,
                }
            },
        );

        // A row scrolled out of view takes its editor with it: cancel rather than commit
        // something the user can't see.
        if editing.is_some() && !editor_drawn {
            edit::end(&ctx, id, false);
        }
        match outcome {
            Some(Outcome::Commit(edited, refocus)) => {
                edit::end(&ctx, id, refocus);
                response.edited = Some(edited);
            }
            Some(Outcome::Cancel) => edit::end(&ctx, id, true),
            Some(Outcome::Open) | None => {}
        }

        // F2 and Enter start editing the active column of the selected row.
        if editing.is_none()
            && (keys.f2 || keys.enter)
            && let Some(row) = *selected
        {
            let mut editable = |col: usize| match col {
                0 => rows[row].editable,
                _ => cell(row, col).editable,
            };
            let active = edit::active_column(&ctx, id).filter(|col| *col < columns.len());
            let target = active
                .filter(|col| editable(*col))
                .or_else(|| (0..columns.len()).find(|col| editable(*col)));
            if let Some(col) = target {
                let text = match col {
                    0 => rows[row].label.clone(),
                    _ => cell(row, col).text,
                };
                edit::begin(&ctx, id, row, col, &text);
                if keys.enter {
                    response.activated = None;
                }
            }
        }

        apply_tree_key(
            rows,
            selected,
            keys.horizontal.filter(|_| editing.is_none()),
            &mut response,
        );
        response.sort_clicked = sort_clicked;
        response
    }
}
