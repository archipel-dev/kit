use super::*;

pub(super) struct State {
    checks: [bool; 3],
    expanded: [bool; 2],
    selected: usize,
    loading_selected: Option<usize>,
    columns: Vec<Column>,
    row: Option<usize>,
    sort: Option<(usize, SortOrder)>,
    column_order: Vec<usize>,
    requests: std::collections::HashMap<usize, String>,
    documents: Documents,
}

impl Default for State {
    fn default() -> Self {
        Self {
            checks: [true, false, false],
            expanded: [true, true],
            selected: 2,
            loading_selected: None,
            columns: vec![
                Column::new("#", 70.0).right(),
                Column::new("Request", 200.0).mono(),
                Column::new("Status", 64.0).right(),
                Column::new("Time", 80.0).right(),
            ],
            row: None,
            sort: None,
            column_order: vec![0, 1, 2, 3],
            requests: Default::default(),
            documents: Documents::new(),
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(SectionHeader::new("Tree"));
        let mut row = |ui: &mut egui::Ui, index: usize, row: ListRow| {
            let out = row
                .selected(self.selected == index)
                .enabled(enabled)
                .show(ui);
            if out.response.clicked() {
                self.selected = index;
            }
            out.toggled
        };
        if row(
            ui,
            0,
            ListRow::new("archipel")
                .icon(Glyph::Project)
                .expanded(Some(self.expanded[0])),
        ) {
            self.expanded[0] = !self.expanded[0];
        }
        if self.expanded[0] {
            if row(
                ui,
                1,
                ListRow::new("src")
                    .icon(Glyph::Folder)
                    .depth(1)
                    .expanded(Some(self.expanded[1])),
            ) {
                self.expanded[1] = !self.expanded[1];
            }
            if self.expanded[1] {
                row(
                    ui,
                    2,
                    ListRow::new("main.rs")
                        .icon(Icon::for_file("main.rs"))
                        .depth(2),
                );
                row(
                    ui,
                    3,
                    ListRow::new("theme.rs")
                        .icon(Icon::for_file("theme.rs"))
                        .depth(2)
                        .label_tone(Tone::Modified),
                );
            }
            row(
                ui,
                4,
                ListRow::new("Cargo.toml")
                    .icon(Icon::for_file("Cargo.toml"))
                    .depth(1)
                    .secondary("1 KB"),
            );
        }
        ui.add(SectionHeader::new("Slots and behaviours"));
        let loading = [TreeRow::new("Collections", 0)
            .icon(Glyph::Database)
            .icon_tone(Tone::Warning)
            .busy(true)
            .parent(false)];
        ui.add_enabled_ui(enabled, |ui| {
            Tree::new("gallery-busy-tree", &loading, &mut self.loading_selected)
                .height(32.0)
                .show(ui);
        });
        ListRow::new("Problems")
            .icon(Glyph::Warning)
            .trailing(Slot::text("12"))
            .enabled(enabled)
            .with(Tooltip::new("Problems").hint("12 in 3 files"))
            .show(ui);
        ListRow::new("app")
            .icon(Glyph::Folder)
            .enabled(enabled)
            .trailing_on_hover(true)
            .trailing(Slot::ui(|ui| {
                ui.add(
                    IconButton::new(Glyph::Close, "Remove app")
                        .small(true)
                        .enabled(enabled),
                );
            }))
            .with(ContextMenu::new(|ui| {
                ui.add(MenuItem::new("Open Module Settings"));
                ui.add(MenuSeparator);
                ui.add(MenuItem::new("Remove"));
            }))
            .show(ui);
        ui.add(SectionHeader::new("Row options"));
        ListRow::new("Gradle")
            .icon(Glyph::Folder)
            .expanded(Some(true))
            .strong(true)
            .enabled(enabled)
            .show(ui);
        ListRow::new("Search hit in main.rs")
            .icon(Icon::for_file("main.rs"))
            .depth(1)
            .tint(Tone::Warning)
            .enabled(enabled)
            .show(ui);
        ListRow::new("app (running)")
            .icon(Glyph::Run)
            .icon_tone(Tone::Run)
            .strong(true)
            .tint(Tone::Success)
            .enabled(enabled)
            .show(ui);
        let mut included = self.checks[0];
        ListRow::new("theme.rs")
            .icon(Icon::for_file("theme.rs"))
            .secondary("src/")
            .leading(Slot::ui_width(22.0, |ui| {
                ui.add(
                    Checkbox::new(&mut included, "Include theme.rs")
                        .label_visible(false)
                        .enabled(enabled),
                );
            }))
            .enabled(enabled)
            .show(ui);
        self.checks[0] = included;
        ListRow::new("A very long file name that does not fit into this row at all.kt")
            .icon(Icon::for_file("a.kt"))
            .secondary("with/a/long/path")
            .trailing(Slot::text("12"))
            .enabled(enabled)
            .show(ui);
        ui.add(SectionHeader::new(
            "Virtualized table: 100 000 rows, drag headers to reorder, edit requests",
        ));
        let len = 100_000;
        let descending = self
            .sort
            .is_some_and(|(_, order)| order == SortOrder::Descending);
        let order = self.column_order.clone();
        let shown_sort = self
            .sort
            .and_then(|(data, order_)| Some((order.iter().position(|d| *d == data)?, order_)));
        let requests = &self.requests;
        let out = Table::new("gallery-table", &mut self.columns, len, &mut self.row)
            .sort(shown_sort)
            .reorderable(true)
            .height(220.0)
            .show(ui, |row, position| {
                let n = if descending { len - 1 - row } else { row };
                let status = [200, 201, 404, 200, 500, 304][n % 6];
                match order[position] {
                    0 => Cell::new(n.to_string()).muted(),
                    1 => Cell::new(
                        requests
                            .get(&n)
                            .cloned()
                            .unwrap_or_else(|| format!("GET /api/users/{n}")),
                    )
                    .editable(true),
                    2 => Cell::new(status.to_string()).tone(match status {
                        500.. => Tone::Error,
                        400.. => Tone::Warning,
                        _ => Tone::Success,
                    }),
                    _ => Cell::new(format!("{} ms", 12 + n * 37 % 300)).muted(),
                }
            });
        if let Some(position) = out.sort_clicked {
            let data = order[position];
            self.sort = Some(match self.sort {
                Some((c, o)) if c == data => (data, o.reversed()),
                _ => (data, SortOrder::Ascending),
            });
        }
        if let Some(edit) = out.edited {
            let n = if descending {
                len - 1 - edit.row
            } else {
                edit.row
            };
            self.requests.insert(n, edit.text);
        }
        // The table reports the new order; the gallery owns the columns and applies it.
        if let Some(new) = out.column_order {
            self.columns = new.iter().map(|&i| self.columns[i].clone()).collect();
            self.column_order = new.iter().map(|&i| order[i]).collect();
        }
        ui.add(SectionHeader::new("Tree table: 100 000 rows, inline edit"));
        self.documents.show(ui);
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}

/// Documents in the tree table page, and fields per document: 1 000 × (1 + 99) rows.
const DOCUMENTS: usize = 1_000;
const FIELDS: usize = 99;

/// The TreeTable page's data: 1 000 documents of 99 fields (100 000 rows while all are
/// open). The gallery owns it like an app would: the expanded state, the edited texts and the
/// flattened visible rows, rebuilt when one of them changes.
struct Documents {
    columns: Vec<Column>,
    selected: Option<usize>,
    open: Vec<bool>,
    /// Edited keys and values by (document, field).
    keys: std::collections::HashMap<(usize, usize), String>,
    values: std::collections::HashMap<(usize, usize), String>,
    rows: Vec<TreeRow>,
    /// The (document, field) of each visible row; `None` is the document itself.
    index: Vec<(usize, Option<usize>)>,
    last: String,
}

impl Documents {
    fn new() -> Self {
        let mut documents = Self {
            columns: vec![
                Column::new("Key", 220.0),
                Column::new("Value", 260.0).mono(),
                Column::new("Type", 90.0),
            ],
            selected: None,
            open: vec![true; DOCUMENTS],
            keys: Default::default(),
            values: Default::default(),
            rows: Vec::new(),
            index: Vec::new(),
            last: "Select a row and press F2 or Enter, or double-click a value".into(),
        };
        documents.rebuild();
        documents
    }

    fn key(&self, doc: usize, field: usize) -> String {
        self.keys
            .get(&(doc, field))
            .cloned()
            .unwrap_or_else(|| format!("field{field}"))
    }

    fn value(&self, doc: usize, field: usize) -> String {
        self.values
            .get(&(doc, field))
            .cloned()
            .unwrap_or_else(|| match field % 3 {
                0 => format!("{}", doc * 1000 + field),
                1 => format!("value-{doc}-{field}"),
                _ => field.is_multiple_of(2).to_string(),
            })
    }

    fn rebuild(&mut self) {
        self.rows.clear();
        self.index.clear();
        for doc in 0..DOCUMENTS {
            self.rows.push(
                TreeRow::new(format!("doc{doc}"), 0)
                    .icon(Glyph::Folder)
                    .parent(self.open[doc]),
            );
            self.index.push((doc, None));
            if self.open[doc] {
                for field in 0..FIELDS {
                    self.rows
                        .push(TreeRow::new(self.key(doc, field), 1).editable(true));
                    self.index.push((doc, Some(field)));
                }
            }
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        // Taken out so the cell callback can read `self` while the widget edits the rest.
        let mut columns = std::mem::take(&mut self.columns);
        let mut selected = self.selected;
        let out = TreeTable::new(
            "gallery-tree-table",
            &mut columns,
            &self.rows,
            &mut selected,
        )
        .name("Documents")
        .height(220.0)
        .show(ui, |row, col| match self.index[row] {
            (_, None) if col == 1 => {
                Cell::new(format!("{{ {FIELDS} fields }}")).value_kind(ValueKind::Punctuation)
            }
            (_, None) => Cell::new("Object").muted(),
            (doc, Some(field)) if col == 1 => Cell::new(self.value(doc, field))
                .value_kind(match field % 3 {
                    0 => ValueKind::Number,
                    1 => ValueKind::String,
                    _ => ValueKind::Boolean,
                })
                .editable(true),
            (_, Some(field)) => Cell::new(["Int32", "String", "Boolean"][field % 3]).muted(),
        });
        self.columns = columns;
        self.selected = selected;
        if let Some(row) = out.activated {
            self.last = format!("Activated {}", self.rows[row].label());
        }
        if let Some(edit) = out.edited
            && let (doc, Some(field)) = self.index[edit.row]
        {
            self.last = format!(
                "Edited row {} column {}: {:?}",
                edit.row, edit.column, edit.text
            );
            if edit.column == 0 {
                self.keys.insert((doc, field), edit.text);
                self.rebuild();
            } else {
                self.values.insert((doc, field), edit.text);
            }
        }
        // Last: it rebuilds the rows, and `row` above refers to the rows drawn this frame.
        if let Some(row) = out.toggled {
            let (doc, _) = self.index[row];
            self.open[doc] = !self.open[doc];
            self.rebuild();
        }
        ui.label(egui::RichText::new(&self.last).color(Theme::colors(ui.ctx()).text_secondary));
    }
}
