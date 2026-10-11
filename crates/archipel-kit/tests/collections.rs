//! `VirtualList`, `Tree`, `Table` and `TreeTable` tests.

mod common;

use archipel_kit::components::{
    Cell, CellEdit, Column, ListRow, SortOrder, Table, Tree, TreeRow, TreeTable, VirtualList,
};
use archipel_kit::icons::Language;
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::{Glyph, Theme, Tone};
use egui::Key;

fn label(i: usize) -> String {
    format!("GET /users/{i}")
}

#[test]
fn list_snapshot() {
    testing::snapshot_themes("virtual_list", common::IMG, |ui| {
        ui.set_width(280.0);
        let mut selected = Some(2);
        let _ = VirtualList::new("history", 1_000, &mut selected)
            .height(144.0)
            .show(ui, |i| {
                ListRow::new(label(i))
                    .icon(Glyph::Send)
                    .secondary(format!("{} ms", 20 + i * 7 % 90))
            });
    });
}

#[test]
fn list_draws_only_visible_rows() {
    let harness = testing::harness_state(Theme::dark(), None, |ui, selected| {
        let _ = VirtualList::new("big", 100_000, selected)
            .height(120.0)
            .show(ui, |i| ListRow::new(label(i)));
    });
    assert!(harness.query_by_label(&label(0)).is_some());
    assert!(harness.query_by_label(&label(4)).is_some());
    assert!(harness.query_by_label(&label(50)).is_none());
    assert!(harness.query_by_label(&label(99_999)).is_none());
}

/// A list of 1000 rows; the state records the selection and the last activated row.
fn list_harness() -> testing::Harness<'static, (Option<usize>, Option<usize>)> {
    let mut harness =
        testing::harness_state(Theme::dark(), (None, None), |ui, (selected, opened)| {
            let out = VirtualList::new("keys", 1_000, selected)
                .height(120.0)
                .show(ui, |i| ListRow::new(label(i)));
            if out.activated.is_some() {
                *opened = out.activated;
            }
        });
    harness.run();
    harness
}

#[test]
fn list_click_selects_and_keys_move() {
    let mut harness = list_harness();
    harness.get_by_label(&label(1)).click();
    harness.run();
    assert_eq!(harness.state().0, Some(1));
    assert!(harness.get_by_label(&label(1)).is_checked());

    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().0, Some(2));
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().0, Some(0));

    harness.key_press(Key::End);
    harness.run();
    harness.run();
    assert_eq!(harness.state().0, Some(999));
    // The selection is scrolled into view.
    assert!(harness.query_by_label(&label(999)).is_some());

    harness.key_press(Key::Home);
    harness.run();
    harness.key_press(Key::PageDown);
    harness.run();
    assert!(harness.state().0 >= Some(4), "{:?}", harness.state());
}

#[test]
fn list_enter_and_double_click_activate() {
    let mut harness = list_harness();
    harness.get_by_label(&label(3)).click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().1, Some(3));

    let mut harness = list_harness();
    let row = harness.get_by_label(&label(2)).rect().center();
    for _ in 0..2 {
        harness.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        harness.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(harness.state().1, Some(2));
}

#[test]
fn keys_do_nothing_without_focus() {
    let mut harness = list_harness();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().0, None);
}

/// The visible rows of a small project tree.
fn project_rows(src: bool, tests: bool) -> Vec<TreeRow> {
    let mut rows = vec![TreeRow::new("kit", 0).icon(Glyph::Folder).parent(true)];
    rows.push(TreeRow::new("src", 1).icon(Glyph::Folder).parent(src));
    if src {
        rows.push(TreeRow::new("lib.rs", 2).icon(Language::Rust));
        rows.push(TreeRow::new("theme.rs", 2).icon(Language::Rust));
    }
    rows.push(TreeRow::new("tests", 1).icon(Glyph::Folder).parent(tests));
    if tests {
        rows.push(TreeRow::new("docs.rs", 2).icon(Language::Rust));
    }
    rows.push(
        TreeRow::new("Cargo.toml", 1)
            .icon(Language::Toml)
            .secondary("workspace"),
    );
    rows
}

#[test]
fn tree_snapshot() {
    testing::snapshot_themes("tree", common::IMG, |ui| {
        ui.set_width(280.0);
        let rows = project_rows(true, false);
        let mut selected = Some(2);
        let _ = Tree::new("project", &rows, &mut selected)
            .height(144.0)
            .show(ui);
    });
}

#[derive(Default)]
struct TreeState {
    src: bool,
    tests: bool,
    selected: Option<usize>,
}

fn tree_harness() -> testing::Harness<'static, TreeState> {
    let state = TreeState {
        src: true,
        ..TreeState::default()
    };
    let mut harness = testing::harness_state(Theme::dark(), state, |ui, state| {
        let rows = project_rows(state.src, state.tests);
        let out = Tree::new("project", &rows, &mut state.selected)
            .height(200.0)
            .show(ui);
        if let Some(index) = out.toggled {
            match rows[index].depth() {
                1 if index == 1 => state.src = !state.src,
                1 => state.tests = !state.tests,
                _ => {}
            }
        }
    });
    harness.run();
    harness
}

#[test]
fn tree_arrows_expand_collapse_and_walk() {
    let mut harness = tree_harness();
    harness.get_by_label("lib.rs").click();
    harness.run();
    assert_eq!(harness.state().selected, Some(2));
    // ← on a leaf goes to its parent; ← on an expanded parent collapses it.
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selected, Some(1));
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().src);
    assert!(harness.query_by_label("lib.rs").is_none());
    // → expands again, a second → moves to the first child.
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.state().src);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selected, Some(2));
}

#[test]
fn tree_chevron_click_toggles() {
    let mut harness = tree_harness();
    assert!(harness.query_by_label("docs.rs").is_none());
    let tests = harness.get_by_label("tests").rect();
    // The chevron sits left of the icon, one indent level in.
    harness.get_by_label("tests").hover();
    harness.run();
    let chevron = egui::pos2(tests.left() + 24.0, tests.center().y);
    harness.event(egui::Event::PointerButton {
        pos: chevron,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.event(egui::Event::PointerButton {
        pos: chevron,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    assert!(harness.state().tests);
    assert!(harness.query_by_label("docs.rs").is_some());
}

const STATUS: [u16; 6] = [200, 201, 404, 200, 500, 304];

fn request_columns() -> Vec<Column> {
    vec![
        Column::new("Method", 70.0),
        Column::new("URL", 190.0).mono(),
        Column::new("Status", 64.0).right(),
        Column::new("Time", 70.0).right(),
    ]
}

fn request_cell(row: usize, col: usize) -> Cell {
    let status = STATUS[row % STATUS.len()];
    match col {
        0 => Cell::new(if row.is_multiple_of(3) { "POST" } else { "GET" }),
        1 => Cell::new(format!("/api/users/{row}")),
        2 => Cell::new(status.to_string()).tone(match status {
            500.. => Tone::Error,
            400.. => Tone::Warning,
            300.. => Tone::Secondary,
            _ => Tone::Success,
        }),
        _ => Cell::new(format!("{} ms", 12 + row * 37 % 300)).muted(),
    }
}

#[test]
fn table_snapshot() {
    testing::snapshot_themes("table", common::IMG, |ui| {
        ui.set_width(420.0);
        let mut columns = request_columns();
        let mut selected = Some(1);
        let _ = Table::new("requests", &mut columns, 500, &mut selected)
            .sort(Some((3, SortOrder::Descending)))
            .height(144.0)
            .show(ui, request_cell);
    });
}

#[derive(Default)]
struct TableState {
    columns: Vec<Column>,
    selected: Option<usize>,
    sort: Option<(usize, SortOrder)>,
}

fn table_harness() -> testing::Harness<'static, TableState> {
    let state = TableState {
        columns: request_columns(),
        ..TableState::default()
    };
    let mut harness = testing::harness_state(Theme::dark(), state, |ui, state| {
        let out = Table::new("requests", &mut state.columns, 10_000, &mut state.selected)
            .sort(state.sort)
            .height(160.0)
            .show(ui, request_cell);
        if let Some(col) = out.sort_clicked {
            state.sort = Some(match state.sort {
                Some((c, order)) if c == col => (col, order.reversed()),
                _ => (col, SortOrder::Ascending),
            });
        }
    });
    harness.run();
    harness
}

#[test]
fn table_header_click_reports_sort() {
    let mut harness = table_harness();
    harness.get_by_label("Status").click();
    harness.run();
    assert_eq!(harness.state().sort, Some((2, SortOrder::Ascending)));
    harness.get_by_label("Status").click();
    harness.run();
    assert_eq!(harness.state().sort, Some((2, SortOrder::Descending)));
}

#[test]
fn table_rows_are_virtualized_and_selectable() {
    let mut harness = table_harness();
    let row = "GET, /api/users/1, 201, 49 ms";
    assert!(harness.query_by_label(row).is_some());
    assert!(
        harness
            .query_by_label("POST, /api/users/9999, 304, 255 ms")
            .is_none()
    );
    harness.get_by_label(row).click();
    harness.run();
    assert_eq!(harness.state().selected, Some(1));
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selected, Some(2));
}

#[test]
fn table_columns_resize_by_dragging_the_header_edge() {
    let mut harness = table_harness();
    let method = harness.get_by_label("Method").rect();
    let edge = egui::pos2(method.right(), method.center().y);
    harness.hover_at(edge);
    harness.run();
    harness.drag_at(edge);
    harness.run();
    harness.hover_at(edge + egui::vec2(30.0, 0.0));
    harness.run();
    harness.drop_at(edge + egui::vec2(30.0, 0.0));
    harness.run();
    let width = harness.state().columns[0].width;
    assert!((width - 100.0).abs() < 1.0, "width {width}");
}

// ---- TreeTable ----

/// The visible rows of a small MongoDB document: key | value | type.
fn document_rows(address_open: bool) -> Vec<TreeRow> {
    let mut rows = vec![
        TreeRow::new("name", 0).editable(true),
        TreeRow::new("age", 0),
        TreeRow::new("address", 0)
            .icon(Glyph::Folder)
            .parent(address_open),
    ];
    if address_open {
        rows.push(TreeRow::new("city", 1));
        rows.push(TreeRow::new("zip", 1));
    }
    rows.push(TreeRow::new("active", 0));
    rows
}

fn document_columns() -> Vec<Column> {
    vec![
        Column::new("Key", 130.0),
        Column::new("Value", 150.0).mono(),
        Column::new("Type", 70.0),
    ]
}

/// Value and type of the visible row `row` (`address_open` decides which rows exist).
fn document_cell(row: usize, col: usize, address_open: bool, name: &str, city: &str) -> Cell {
    let keys = if address_open {
        ["name", "age", "address", "city", "zip", "active"].as_slice()
    } else {
        ["name", "age", "address", "active"].as_slice()
    };
    match (keys[row], col) {
        ("name", 1) => Cell::new(name).editable(true),
        ("name", _) => Cell::new("String").muted(),
        ("age", 1) => Cell::new("36").editable(true),
        ("age", _) => Cell::new("Int32").muted(),
        ("address", 1) => Cell::new("{ 2 fields }").muted(),
        ("address", _) => Cell::new("Object").muted(),
        ("city", 1) => Cell::new(city).editable(true),
        ("zip", 1) => Cell::new("12345"),
        ("active", 1) => Cell::new("true").tone(Tone::Success),
        ("active", _) => Cell::new("Boolean").muted(),
        _ => Cell::new("String").muted(),
    }
}

#[test]
fn tree_table_snapshot() {
    testing::snapshot_themes("tree_table", common::IMG, |ui| {
        ui.set_width(380.0);
        let rows = document_rows(true);
        let mut columns = document_columns();
        let mut selected = Some(3);
        let _ = TreeTable::new("document", &mut columns, &rows, &mut selected)
            .name("Document")
            .height(168.0)
            .show(ui, |row, col| {
                document_cell(row, col, true, "Ada", "London")
            });
    });
}

#[derive(Default)]
struct DocState {
    columns: Vec<Column>,
    selected: Option<usize>,
    address_open: bool,
    name: String,
    city: String,
    sort: Option<(usize, SortOrder)>,
    edited: Vec<CellEdit>,
    activated: Vec<usize>,
    cursor: egui::CursorIcon,
}

fn doc_ui(ui: &mut egui::Ui, state: &mut DocState) {
    let rows = document_rows(state.address_open);
    let (open, name, city) = (state.address_open, state.name.clone(), state.city.clone());
    let out = TreeTable::new("document", &mut state.columns, &rows, &mut state.selected)
        .name("Document")
        .sort(state.sort)
        .height(168.0)
        .show(ui, |row, col| document_cell(row, col, open, &name, &city));
    if out.toggled.is_some() {
        state.address_open = !state.address_open;
    }
    if let Some(index) = out.activated {
        state.activated.push(index);
    }
    if let Some(col) = out.sort_clicked {
        state.sort = Some((col, SortOrder::Ascending));
    }
    if let Some(edit) = out.edited {
        match (edit.row, edit.column) {
            (0, 1) => state.name = edit.text.clone(),
            (3, 1) => state.city = edit.text.clone(),
            _ => {}
        }
        state.edited.push(edit);
    }
    state.cursor = archipel_kit::cursor::current(ui.ctx());
}

fn doc_state() -> DocState {
    DocState {
        columns: document_columns(),
        address_open: true,
        name: "Ada".into(),
        city: "London".into(),
        ..DocState::default()
    }
}

fn doc_harness() -> testing::Harness<'static, DocState> {
    let mut harness = testing::harness_state(Theme::dark(), doc_state(), doc_ui);
    harness.run();
    harness
}

const NAME_ROW: &str = "name, Ada, String";

/// Where the key and the value column of `row` are, for raw clicks.
fn key_at(row: egui::Rect) -> egui::Pos2 {
    egui::pos2(row.left() + 60.0, row.center().y)
}

#[test]
fn tree_table_keys_walk_expand_and_collapse() {
    let mut harness = doc_harness();
    harness.get_by_label(NAME_ROW).click();
    harness.run();
    assert_eq!(harness.state().selected, Some(0));
    for (key, expected) in [
        (Key::ArrowDown, 1),
        (Key::ArrowDown, 2),
        (Key::End, 5),
        (Key::Home, 0),
        (Key::PageDown, 5),
        (Key::PageUp, 0),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().selected, Some(expected), "after {key:?}");
    }
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    // ← collapses the expanded parent, → expands it, a second → walks to the first child.
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().address_open);
    assert!(harness.query_by_label_contains("city, ").is_none());
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.state().address_open);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selected, Some(3));
    // ← on a leaf goes to its parent.
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selected, Some(2));
}

#[test]
fn tree_table_chevron_toggles_a_parent() {
    let mut harness = doc_harness();
    harness.get_by_label("Collapse address").click();
    harness.run();
    assert!(!harness.state().address_open);
    harness.get_by_label("Expand address").click();
    harness.run();
    assert!(harness.state().address_open);
}

#[test]
fn tree_table_double_click_toggles_a_parent_instead_of_activating_it() {
    let mut harness = doc_harness();
    let row = harness.get_by_label("address, { 2 fields }, Object").rect();
    double_click(&mut harness, key_at(row));
    assert!(!harness.state().address_open);
    assert!(harness.state().activated.is_empty());
}

fn click_at(harness: &mut testing::Harness<'_, DocState>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}

fn double_click(harness: &mut testing::Harness<'_, DocState>, pos: egui::Pos2) {
    click_at(harness, pos);
    click_at(harness, pos);
    harness.run();
}

#[test]
fn tree_table_enter_activates_rows_with_nothing_to_edit() {
    let mut harness = doc_harness();
    harness.get_by_label("active, true, Boolean").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().activated, vec![5]);
    assert!(harness.query_by_label("Edit Value").is_none());
}

#[test]
fn tree_table_f2_edits_commits_with_enter_and_cancels_with_escape() {
    let mut harness = doc_harness();
    let row = harness.get_by_label(NAME_ROW).rect();
    click_at(&mut harness, key_at(row));
    harness.run();
    // F2 edits the first editable column (the key of `name` is editable, so column 0).
    harness.key_press(Key::F2);
    harness.run();
    assert!(harness.query_by_label("Edit Key").is_some());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("Edit Key").is_none());
    assert!(harness.state().edited.is_empty(), "Escape cancels");

    // Clicking a value cell makes the value the column F2 and Enter edit.
    let row = harness.get_by_label(NAME_ROW).rect();
    click_at(&mut harness, egui::pos2(row.left() + 160.0, row.center().y));
    harness.run();
    harness.key_press(Key::F2);
    harness.run();
    let field = harness.get_by_label("Edit Value");
    field.type_text("m");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().edited,
        vec![CellEdit {
            row: 0,
            column: 1,
            text: "Adam".into()
        }]
    );
    assert_eq!(harness.state().name, "Adam");
    assert!(harness.query_by_label("Edit Value").is_none());
    assert!(
        harness.state().activated.is_empty(),
        "Enter edited, not activated"
    );
    // The collection has the focus again: the arrows move the selection.
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selected, Some(1));
}

#[test]
fn tree_table_enter_starts_an_edit_and_escape_cancels_it() {
    let mut harness = doc_harness();
    harness.get_by_label("age, 36, Int32").click();
    harness.run();
    // `age` has an editable value but no editable key: Enter finds the value.
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.query_by_label("Edit Value").is_some());
    assert!(harness.state().activated.is_empty());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("Edit Value").is_none());
    assert!(harness.state().edited.is_empty());
}

#[test]
fn tree_table_double_click_edits_an_editable_cell() {
    let mut harness = doc_harness();
    let row = harness.get_by_label("city, London, String").rect();
    double_click(&mut harness, egui::pos2(row.left() + 160.0, row.center().y));
    harness.get_by_label("Edit Value");
    assert!(harness.state().activated.is_empty());
    harness.get_by_label("Edit Value").type_text("!");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().city, "London!");
}

#[test]
fn tree_table_leaving_the_field_commits() {
    let mut harness = doc_harness();
    harness.get_by_label(NAME_ROW).click();
    harness.run();
    let row = harness.get_by_label(NAME_ROW).rect();
    click_at(&mut harness, egui::pos2(row.left() + 160.0, row.center().y));
    harness.key_press(Key::F2);
    harness.run();
    harness.get_by_label("Edit Value").type_text("x");
    harness.run();
    harness.get_by_label("age, 36, Int32").click();
    harness.run();
    assert_eq!(harness.state().name, "Adax");
    assert_eq!(harness.state().selected, Some(1));
}

#[test]
fn tree_table_cursors_are_arrow_on_rows_and_text_in_the_field() {
    let mut harness = doc_harness();
    harness.get_by_label(NAME_ROW).hover();
    harness.run();
    assert_eq!(harness.state().cursor, egui::CursorIcon::Default);
    harness.get_by_label(NAME_ROW).click();
    harness.run();
    let row = harness.get_by_label(NAME_ROW).rect();
    click_at(&mut harness, egui::pos2(row.left() + 160.0, row.center().y));
    harness.key_press(Key::F2);
    harness.run();
    harness.get_by_label("Edit Value").hover();
    harness.run();
    assert_eq!(harness.state().cursor, egui::CursorIcon::Text);
}

#[test]
fn tree_table_header_sorts_and_edge_resizes_the_column() {
    let mut harness = doc_harness();
    harness.get_by_label("Type").click();
    harness.run();
    assert_eq!(harness.state().sort, Some((2, SortOrder::Ascending)));
    let key = harness.get_by_label("Key").rect();
    let edge = egui::pos2(key.right(), key.center().y);
    harness.hover_at(edge);
    harness.run();
    assert_eq!(harness.state().cursor, egui::CursorIcon::ResizeColumn);
    harness.drag_at(edge);
    harness.run();
    harness.hover_at(edge + egui::vec2(30.0, 0.0));
    harness.run();
    harness.drop_at(edge + egui::vec2(30.0, 0.0));
    harness.run();
    let width = harness.state().columns[0].width;
    assert!((width - 160.0).abs() < 1.0, "width {width}");
}

#[test]
fn tree_table_with_100k_rows_draws_only_the_visible_ones() {
    let rows: Vec<TreeRow> = (0..100_000)
        .map(|i| TreeRow::new(format!("field{i}"), i % 3))
        .collect();
    let mut state = (document_columns(), Some(0usize));
    let mut harness = testing::harness_state(Theme::dark(), (), move |ui, _| {
        let _ = TreeTable::new("big", &mut state.0, &rows, &mut state.1)
            .name("Big")
            .height(160.0)
            .show(ui, |row, col| Cell::new(format!("{row}:{col}")));
    });
    harness.run();
    assert!(harness.query_by_label_contains("field0, ").is_some());
    assert!(harness.query_by_label_contains("field99999, ").is_none());
    let drawn = harness.get_all_by_label_contains("field").count();
    assert!(drawn < 20, "{drawn} rows drawn");
}

/// Renders the document with the value of `age` open for editing, in both themes at the
/// docs' 2× scale (so the image matches the others), driven by the keyboard only.
#[test]
fn tree_table_editing_snapshot() {
    let mut results = testing::SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let egui_theme = if theme.dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        let mut pending = Some(theme);
        let state = DocState {
            selected: Some(1),
            ..doc_state()
        };
        let mut harness = testing::builder::<DocState>()
            .with_theme(egui_theme)
            .with_pixels_per_point(testing::SNAPSHOT_SCALE)
            .build_ui_state(
                move |ui, state| {
                    if let Some(theme) = pending.take() {
                        archipel_kit::install(ui.ctx(), theme);
                        ui.ctx().request_repaint();
                        return;
                    }
                    archipel_kit::parts::Surface::Island
                        .frame(ui.ctx())
                        .show(ui, |ui| {
                            ui.set_width(380.0);
                            doc_ui(ui, state);
                        });
                },
                state,
            );
        harness.run_ok();
        harness.get_by_label("Document").focus();
        harness.run();
        harness.key_press(Key::F2);
        harness.run();
        harness.fit_contents();
        results.add(harness.try_snapshot_options(
            format!("tree_table_edit.{suffix}"),
            &testing::SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

// ---- Table: inline edit and column reorder ----

#[derive(Default)]
struct EditableTable {
    columns: Vec<Column>,
    /// The data column shown at each position.
    order: Vec<usize>,
    selected: Option<usize>,
    sort: Option<(usize, SortOrder)>,
    urls: std::collections::HashMap<usize, String>,
    edited: Vec<CellEdit>,
    activated: Vec<usize>,
    reordered: Vec<Vec<usize>>,
    cursor: egui::CursorIcon,
}

fn editable_table_ui(ui: &mut egui::Ui, state: &mut EditableTable) {
    let order = state.order.clone();
    let urls = state.urls.clone();
    let out = Table::new("requests", &mut state.columns, 10_000, &mut state.selected)
        .sort(state.sort)
        .reorderable(true)
        .height(160.0)
        .show(ui, |row, position| {
            let data = order[position];
            match (data, urls.get(&row)) {
                (1, Some(url)) => Cell::new(url.as_str()).editable(true),
                (1, None) => request_cell(row, 1).editable(true),
                _ => request_cell(row, data),
            }
        });
    if let Some(col) = out.sort_clicked {
        state.sort = Some((col, SortOrder::Ascending));
    }
    if let Some(index) = out.activated {
        state.activated.push(index);
    }
    if let Some(edit) = out.edited {
        state.urls.insert(edit.row, edit.text.clone());
        state.edited.push(edit);
    }
    // The caller owns the column order: apply what the table reports.
    if let Some(new_order) = out.column_order {
        state.columns = new_order
            .iter()
            .map(|&i| state.columns[i].clone())
            .collect();
        state.order = new_order.iter().map(|&i| state.order[i]).collect();
        state.reordered.push(new_order);
    }
    state.cursor = archipel_kit::cursor::current(ui.ctx());
}

fn editable_table_state() -> EditableTable {
    EditableTable {
        columns: request_columns(),
        order: vec![0, 1, 2, 3],
        ..EditableTable::default()
    }
}

fn editable_table_harness() -> testing::Harness<'static, EditableTable> {
    let mut harness =
        testing::harness_state(Theme::dark(), editable_table_state(), editable_table_ui);
    harness.run();
    harness
}

const GET_ROW: &str = "GET, /api/users/1, 201, 49 ms";

#[test]
fn table_header_drag_reports_the_new_column_order() {
    let mut harness = editable_table_harness();
    let from = harness.get_by_label("Method").rect().center();
    let to = harness.get_by_label("Status").rect().center();
    harness.hover_at(from);
    harness.run();
    assert_eq!(harness.state().cursor, egui::CursorIcon::Grab);
    harness.drag_at(from);
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.drop_at(to);
    harness.run();
    // Method moved to Status's place: URL, Status, Method, Time.
    assert_eq!(harness.state().reordered, vec![vec![1, 2, 0, 3]]);
    assert_eq!(harness.state().order, vec![1, 2, 0, 3]);
    // The caller applied it: the headers and the cells follow.
    assert!(
        harness
            .query_by_label("/api/users/1, 201, GET, 49 ms")
            .is_some()
    );
    // A drop on its own header changes nothing, and sorting by click still works.
    harness.get_by_label("Status").click();
    harness.run();
    assert_eq!(harness.state().sort, Some((1, SortOrder::Ascending)));
    assert_eq!(harness.state().reordered.len(), 1);
}

#[test]
fn table_without_reorderable_keeps_headers_as_before() {
    let mut harness = table_harness();
    let from = harness.get_by_label("Method").rect().center();
    let to = harness.get_by_label("Status").rect().center();
    harness.hover_at(from);
    harness.run();
    assert_eq!(
        harness.get_by_label("Method").rect().center(),
        from,
        "headers stay put"
    );
    harness.drag_at(from);
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.drop_at(to);
    harness.run();
    assert_eq!(harness.state().sort, None);
}

#[test]
fn table_columns_still_resize_when_reorderable() {
    let mut harness = editable_table_harness();
    let method = harness.get_by_label("Method").rect();
    let edge = egui::pos2(method.right(), method.center().y);
    harness.hover_at(edge);
    harness.run();
    assert_eq!(harness.state().cursor, egui::CursorIcon::ResizeColumn);
    harness.drag_at(edge);
    harness.run();
    harness.hover_at(edge + egui::vec2(30.0, 0.0));
    harness.run();
    harness.drop_at(edge + egui::vec2(30.0, 0.0));
    harness.run();
    assert!((harness.state().columns[0].width - 100.0).abs() < 1.0);
    assert!(harness.state().reordered.is_empty());
}

#[test]
fn table_f2_and_enter_edit_the_clicked_cell_and_enter_commits() {
    let mut harness = editable_table_harness();
    let row = harness.get_by_label(GET_ROW).rect();
    // The URL column starts at 4 + 70: click inside it, then F2.
    click_at_table(&mut harness, egui::pos2(row.left() + 120.0, row.center().y));
    harness.run();
    harness.key_press(Key::F2);
    harness.run();
    harness.get_by_label("Edit URL").type_text("?x=1");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().edited,
        vec![CellEdit {
            row: 1,
            column: 1,
            text: "/api/users/1?x=1".into()
        }]
    );
    assert!(harness.state().activated.is_empty());
    assert!(harness.query_by_label("Edit URL").is_none());
    // Escape cancels.
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.query_by_label("Edit URL").is_some());
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().edited.len(), 1);
}

fn click_at_table(harness: &mut testing::Harness<'_, EditableTable>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}

#[test]
fn table_double_click_edits_and_the_field_uses_the_text_cursor() {
    let mut harness = editable_table_harness();
    let row = harness.get_by_label(GET_ROW).rect();
    let at = egui::pos2(row.left() + 120.0, row.center().y);
    click_at_table(&mut harness, at);
    click_at_table(&mut harness, at);
    harness.run();
    assert!(harness.state().activated.is_empty());
    harness.get_by_label("Edit URL").hover();
    harness.run();
    assert_eq!(harness.state().cursor, egui::CursorIcon::Text);
    harness.get_by_label("Edit URL").type_text("!");
    harness.run();
    harness.get_by_label("Method").hover();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().edited[0].text, "/api/users/1!");
}

#[test]
fn table_with_only_non_editable_cells_is_unchanged() {
    let mut harness = table_harness();
    harness
        .get_by_label("GET, /api/users/1, 201, 49 ms")
        .click();
    harness.run();
    harness.key_press(Key::F2);
    harness.run();
    assert!(harness.query_by_label_contains("Edit ").is_none());
}

#[test]
fn table_editing_snapshot() {
    let mut results = testing::SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let egui_theme = if theme.dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        let mut pending = Some(theme);
        let state = EditableTable {
            selected: Some(1),
            ..editable_table_state()
        };
        let mut harness = testing::builder::<EditableTable>()
            .with_theme(egui_theme)
            .with_pixels_per_point(testing::SNAPSHOT_SCALE)
            .build_ui_state(
                move |ui, state| {
                    if let Some(theme) = pending.take() {
                        archipel_kit::install(ui.ctx(), theme);
                        ui.ctx().request_repaint();
                        return;
                    }
                    archipel_kit::parts::Surface::Island
                        .frame(ui.ctx())
                        .show(ui, |ui| {
                            ui.set_width(420.0);
                            editable_table_ui(ui, state);
                        });
                },
                state,
            );
        harness.run_ok();
        harness.get_by_label("Table").focus();
        harness.run();
        harness.key_press(Key::F2);
        harness.run();
        harness.fit_contents();
        results.add(harness.try_snapshot_options(
            format!("table_edit.{suffix}"),
            &testing::SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}
