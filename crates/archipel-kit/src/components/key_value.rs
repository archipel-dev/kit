//! An editable list of key/value rows: query parameters, headers, form fields, environment
//! variables.

use super::{Checkbox, IconButton, TextField};
use crate::icons::Glyph;
use crate::tokens::size;
use egui::{Align, Key, Layout, Response, Ui, WidgetInfo, WidgetType};

/// A row the [`KeyValueTable`] can edit. Implement it for your own model type to edit your
/// rows in place; [`KeyValueEntry`] is the ready-made row.
///
/// New rows are created with `Default`, then enabled and filled through these accessors.
pub trait KeyValueRow {
    /// Whether the row is in effect. Disabled rows are dimmed but stay editable.
    fn enabled_mut(&mut self) -> &mut bool;
    /// The key text.
    fn key_mut(&mut self) -> &mut String;
    /// The value text.
    fn value_mut(&mut self) -> &mut String;
}

/// The kit's own row: an enable flag, a key and a value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyValueEntry {
    /// Whether the row is in effect.
    pub enabled: bool,
    /// The key text.
    pub key: String,
    /// The value text.
    pub value: String,
}

impl KeyValueEntry {
    /// An enabled row.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
        }
    }
}

impl KeyValueRow for KeyValueEntry {
    fn enabled_mut(&mut self) -> &mut bool {
        &mut self.enabled
    }
    fn key_mut(&mut self) -> &mut String {
        &mut self.key
    }
    fn value_mut(&mut self) -> &mut String {
        &mut self.value
    }
}

/// What the user did to the table in one frame. Row numbers are indices into the caller's
/// `Vec`, as it was when the change happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The key of this row was edited.
    KeyEdited(usize),
    /// The value of this row was edited.
    ValueEdited(usize),
    /// A row was appended because the user typed into the trailing empty row.
    Added(usize),
    /// This row was removed with its delete button.
    Removed(usize),
    /// The enable checkbox of this row was toggled.
    Toggled(usize),
}

/// What [`KeyValueTable::show`] reports.
#[derive(Debug)]
pub struct KeyValueTableResponse {
    /// The response of the whole table.
    pub response: Response,
    /// Everything the user changed this frame, in order.
    pub changes: Vec<Change>,
}

impl KeyValueTableResponse {
    /// Whether the rows changed this frame.
    pub fn changed(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// The value cell of one row, handed to a [`KeyValueTable::value_cell`] closure.
pub struct ValueCell<'a> {
    /// The value to edit.
    pub text: &'a mut String,
    /// The width the cell has to fill.
    pub width: f32,
    /// The column placeholder.
    pub placeholder: &'a str,
    /// The accessible name ("Value, row 2"): put it on the widget you return.
    pub name: &'a str,
    /// Whether the table is read-only: the cell must not take edits.
    pub read_only: bool,
}

impl ValueCell<'_> {
    /// The default cell: a kit [`TextField`].
    pub fn text_field(self, ui: &mut Ui) -> Response {
        let response = ui.add(
            TextField::new(&mut *self.text)
                .placeholder(self.placeholder)
                .width(self.width)
                .enabled(!self.read_only),
        );
        name_text_edit(
            &response,
            self.name,
            self.placeholder,
            self.text,
            !self.read_only,
        );
        response
    }
}

type ValueCellFn<'a> = Box<dyn FnMut(&mut Ui, ValueCell<'_>) -> Response + 'a>;

/// An editable list of key/value rows: an enable checkbox, a key, a value and a delete
/// button per row, and a trailing empty row that appends a row as soon as the user types
/// into it.
///
/// The caller owns the rows, a `Vec` of any type implementing [`KeyValueRow`] (or the
/// ready-made [`KeyValueEntry`]); the table edits them in place and reports what happened
/// in [`KeyValueTableResponse::changes`]. <kbd>Tab</kbd> walks the cells row by row,
/// <kbd>Enter</kbd> in a key moves to its value. Disabled rows are dimmed but stay editable.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Change, KeyValueEntry, KeyValueTable};
///
/// let mut params = vec![KeyValueEntry::new("page", "1")];
/// let mut sent = vec![false]; // caller state, parallel to `params`
/// let out = KeyValueTable::new("params", &mut params)
///     .key_placeholder("Add parameter")
///     .value_placeholder("value")
///     .show(ui);
/// for change in &out.changes {
///     if let Change::Removed(index) = change {
///         // `index` refers to the Vec before removal; the table already removed
///         // the row from `params`. Apply it to anything you keep per row.
///         sent.remove(*index);
///     }
/// }
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct KeyValueTable<'a, T: KeyValueRow + Default> {
    id_salt: egui::Id,
    rows: &'a mut Vec<T>,
    key_placeholder: String,
    value_placeholder: String,
    read_only: bool,
    value_cell: Option<ValueCellFn<'a>>,
}

impl<'a, T: KeyValueRow + Default> KeyValueTable<'a, T> {
    /// A table editing `rows`. `id_salt` tells tables on one screen apart.
    pub fn new(id_salt: impl std::hash::Hash, rows: &'a mut Vec<T>) -> Self {
        Self {
            id_salt: egui::Id::new(id_salt),
            rows,
            key_placeholder: "Key".to_owned(),
            value_placeholder: "Value".to_owned(),
            read_only: false,
            value_cell: None,
        }
    }

    /// Hint of empty key cells, e.g. "Add parameter" (default "Key").
    pub fn key_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.key_placeholder = placeholder.into();
        self
    }

    /// Hint of empty value cells (default "Value").
    pub fn value_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.value_placeholder = placeholder.into();
        self
    }

    /// A read-only table: no trailing row, no delete buttons, cells and checkboxes dimmed
    /// and inert.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Draws the value cells yourself, e.g. to highlight `{{variables}}`. Return the
    /// response of the edit widget (`changed()` after an edit) and keep the cell's
    /// accessible name; [`ValueCell::text_field`] is the default.
    pub fn value_cell(mut self, cell: impl FnMut(&mut Ui, ValueCell<'_>) -> Response + 'a) -> Self {
        self.value_cell = Some(Box::new(cell));
        self
    }

    /// Draws the table in the available width.
    pub fn show(mut self, ui: &mut Ui) -> KeyValueTableResponse {
        let mut changes = Vec::new();
        let len = self.rows.len();
        let mut remove = None;
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = size::TOOLBAR_GAP;
                for index in 0..len {
                    if let Some(removed) = self.row(ui, index, false, &mut changes) {
                        remove = Some(removed);
                    }
                }
                if !self.read_only {
                    self.row(ui, len, true, &mut changes);
                }
            })
            .response;
        if let Some(index) = remove {
            self.rows.remove(index);
            changes.push(Change::Removed(index));
        }
        KeyValueTableResponse { response, changes }
    }

    /// One row; `trailing` is the virtual empty row after the last one. Returns the row's
    /// index when its delete button was pressed.
    fn row(
        &mut self,
        ui: &mut Ui,
        index: usize,
        trailing: bool,
        changes: &mut Vec<Change>,
    ) -> Option<usize> {
        let read_only = self.read_only;
        let (key_name, value_name, check_name, remove_name) = if trailing {
            (
                "Key, new row".to_owned(),
                "Value, new row".to_owned(),
                String::new(),
                String::new(),
            )
        } else {
            let n = index + 1;
            (
                format!("Key, row {n}"),
                format!("Value, row {n}"),
                format!("Enabled, row {n}"),
                format!("Remove, row {n}"),
            )
        };
        // Cells of the trailing row edit scratch text, which becomes a row on the first edit.
        let mut scratch = T::default();
        let row = if trailing {
            &mut scratch
        } else {
            &mut self.rows[index]
        };
        let mut removed = false;
        let mut typed = false;

        // Fixed parts (box, delete button, gaps) first; keys and values split the rest.
        let gap = size::TOOLBAR_GAP;
        let fixed = size::ICON + size::ICON_BUTTON_SMALL + size::KEY_VALUE_ROW_GAPS * gap;
        let fields = (ui.available_width() - fixed).max(0.0);
        let key_width = (fields * size::KEY_COLUMN_SHARE).floor();
        let value_width = fields - key_width;

        let key_placeholder = &self.key_placeholder;
        let value_placeholder = &self.value_placeholder;
        let value_cell = &mut self.value_cell;
        let row_id = self.id_salt.with(index);
        ui.push_id(row_id, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.set_height(size::CONTROL_HEIGHT);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // Row parts keep their position in the trailing row, which has
                    // nothing to toggle or delete, so the columns line up.
                    if trailing {
                        ui.allocate_space(egui::vec2(size::ICON, size::CONTROL_HEIGHT));
                    } else {
                        let enabled = *row.enabled_mut();
                        let mut on = enabled;
                        let response = cell(ui, row_id.with("enabled"), |ui| {
                            ui.add(Checkbox::new(&mut on, "").enabled(!read_only))
                        });
                        response.widget_info(|| {
                            WidgetInfo::selected(WidgetType::Checkbox, !read_only, on, &check_name)
                        });
                        if on != enabled {
                            *row.enabled_mut() = on;
                            changes.push(Change::Toggled(index));
                        }
                    }

                    // Disabled rows dim their cells; they stay editable.
                    let dim = !trailing && !*row.enabled_mut();
                    cell(ui, row_id.with("cells"), |ui| {
                        if dim {
                            let alpha = ui.visuals().disabled_alpha();
                            ui.multiply_opacity(alpha);
                        }
                        let key = cell(ui, row_id.with("key"), |ui| {
                            let field = ui.add(
                                TextField::new(row.key_mut())
                                    .placeholder(key_placeholder.as_str())
                                    .width(key_width)
                                    .enabled(!read_only),
                            );
                            name_text_edit(
                                &field,
                                &key_name,
                                key_placeholder,
                                row.key_mut(),
                                !read_only,
                            );
                            field
                        });
                        let value = cell(ui, row_id.with("value"), |ui| {
                            let slot = ValueCell {
                                text: row.value_mut(),
                                width: value_width,
                                placeholder: value_placeholder,
                                name: &value_name,
                                read_only,
                            };
                            match value_cell {
                                Some(custom) => custom(ui, slot),
                                None => slot.text_field(ui),
                            }
                        });
                        // Enter in a key moves on to its value.
                        if key.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            value.request_focus();
                        }
                        if trailing {
                            // Only text makes a row: the empty scratch row stays virtual.
                            let filled = !row.key_mut().is_empty() || !row.value_mut().is_empty();
                            typed = (key.changed() || value.changed()) && filled;
                        } else {
                            if key.changed() {
                                changes.push(Change::KeyEdited(index));
                            }
                            if value.changed() {
                                changes.push(Change::ValueEdited(index));
                            }
                        }
                    });

                    if !trailing && !read_only {
                        let button = cell(ui, row_id.with("remove"), |ui| {
                            ui.add(IconButton::new(Glyph::Close, "Remove").small(true))
                        });
                        button.widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Button, true, &remove_name)
                        });
                        removed = button.clicked();
                    } else {
                        ui.allocate_space(egui::vec2(
                            size::ICON_BUTTON_SMALL,
                            size::CONTROL_HEIGHT,
                        ));
                    }
                });
            });
        });

        if typed {
            // The scratch row is a real row from now on, under the same ids, so focus and
            // cursor stay in the cell being typed into.
            *scratch.enabled_mut() = true;
            self.rows.push(scratch);
            changes.push(Change::Added(index));
            ui.ctx().request_repaint();
        }
        removed.then_some(index)
    }
}

/// Runs `add` in a scope whose id depends only on `id`, not on how many widgets were drawn
/// before it: the trailing row has no checkbox, but its cells must keep their ids (and with
/// them the focus and the cursor) when it becomes a real row.
fn cell<R>(ui: &mut Ui, id: egui::Id, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope_builder(egui::UiBuilder::new().id_salt(id).global_scope(true), add)
        .inner
}

/// Gives a text edit its accessible name ("Key, row 2") instead of its placeholder.
fn name_text_edit(response: &Response, name: &str, placeholder: &str, text: &str, enabled: bool) {
    response.widget_info(|| {
        let mut info = WidgetInfo::text_edit(enabled, text, text, placeholder);
        info.label = Some(name.to_owned());
        info
    });
}
