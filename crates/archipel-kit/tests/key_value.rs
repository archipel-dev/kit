//! `key_value` component tests: behavior, accessibility and dark/light snapshots.

mod common;

use archipel_kit::Theme;
use archipel_kit::testing::{self, NodeExt, NodeT, Queryable};
use archipel_kit::ui::{Change, KeyValueEntry, KeyValueRow, KeyValueTable, ValueCell};
use egui::Key;

type Rows = Vec<KeyValueEntry>;

fn params() -> Rows {
    vec![
        KeyValueEntry::new("page", "1"),
        KeyValueEntry::new("limit", "50"),
    ]
}

/// Rows plus every change the table reported.
#[derive(Default)]
struct State {
    rows: Rows,
    changes: Vec<Change>,
}

fn table_harness(rows: Rows) -> testing::Harness<'static, State> {
    testing::harness_state(
        Theme::dark(),
        State {
            rows,
            changes: Vec::new(),
        },
        |ui, state| {
            let out = KeyValueTable::new("params", &mut state.rows)
                .key_placeholder("Add parameter")
                .value_placeholder("value")
                .show(ui);
            state.changes.extend(out.changes);
        },
    )
}

#[test]
fn snapshot() {
    testing::snapshot_themes("key_value", common::IMG, |ui| {
        ui.set_max_width(500.0);
        let mut rows = vec![
            KeyValueEntry::new("page", "1"),
            KeyValueEntry::new("limit", "50"),
            KeyValueEntry {
                enabled: false,
                ..KeyValueEntry::new("debug", "true")
            },
        ];
        KeyValueTable::new("params", &mut rows)
            .key_placeholder("Add parameter")
            .value_placeholder("value")
            .show(ui);
        ui.add_space(12.0);
        let mut fixed = vec![KeyValueEntry::new("Accept", "application/json")];
        KeyValueTable::new("read-only", &mut fixed)
            .read_only(true)
            .show(ui);
    });
}

#[test]
fn cells_have_accessible_names() {
    let harness = table_harness(params());
    for name in [
        "Enabled, row 1",
        "Key, row 1",
        "Value, row 1",
        "Remove, row 1",
        "Enabled, row 2",
        "Key, row 2",
        "Value, row 2",
        "Remove, row 2",
        "Key, new row",
        "Value, new row",
    ] {
        assert!(
            harness.query_by_label(name).is_some(),
            "no cell named {name:?}"
        );
    }
    assert!(harness.get_by_label("Enabled, row 1").is_checked());
    assert_eq!(
        harness.get_by_label("Key, row 2").value().as_deref(),
        Some("limit")
    );
}

#[test]
fn typing_into_the_trailing_row_appends_rows_and_keeps_focus() {
    let mut harness = table_harness(Vec::new());
    for (i, key) in ["page", "limit", "sort"].into_iter().enumerate() {
        let cell = harness.get_by_label("Key, new row");
        cell.focus();
        cell.type_text(key);
        harness.run();
        assert_eq!(harness.state().rows.len(), i + 1);
        assert_eq!(harness.state().rows[i].key, key);
        assert!(harness.state().rows[i].enabled, "new rows are enabled");
        assert!(
            harness
                .get_by_label(&format!("Key, row {}", i + 1))
                .accesskit_node()
                .is_focused(),
            "focus stays in the cell being typed into"
        );
    }
    let keys: Vec<_> = harness
        .state()
        .rows
        .iter()
        .map(|r| r.key.as_str())
        .collect();
    assert_eq!(keys, ["page", "limit", "sort"]);
    assert_eq!(
        harness.state().changes,
        [Change::Added(0), Change::Added(1), Change::Added(2)]
    );
    // One more table's worth of typing continues in the cell that was typed into.
    harness.get_by_label("Key, row 3").type_text("ed");
    harness.run();
    assert_eq!(harness.state().rows[2].key, "sorted");
}

#[test]
fn typing_a_value_in_the_trailing_row_appends_a_row_once() {
    let mut harness = table_harness(Vec::new());
    let cell = harness.get_by_label("Value, new row");
    cell.focus();
    cell.type_text("42");
    harness.run();
    assert_eq!(harness.state().rows, [KeyValueEntry::new("", "42")]);
    assert_eq!(harness.state().changes, [Change::Added(0)]);
}

#[test]
fn editing_toggling_and_removing_report_what_changed() {
    let mut harness = table_harness(params());
    harness.get_by_label("Value, row 1").focus();
    harness.get_by_label("Value, row 1").type_text("2");
    harness.run();
    assert_eq!(harness.state().rows[0].value, "12");

    harness.get_by_label("Enabled, row 2").click();
    harness.run();
    assert!(!harness.state().rows[1].enabled);
    assert!(!harness.get_by_label("Enabled, row 2").is_checked());
    // A disabled row stays editable.
    harness.get_by_label("Key, row 2").focus();
    harness.get_by_label("Key, row 2").type_text("s");
    harness.run();
    assert_eq!(harness.state().rows[1].key, "limits");

    harness.get_by_label("Remove, row 1").click();
    harness.run();
    assert_eq!(harness.state().rows.len(), 1);
    assert_eq!(harness.state().rows[0].key, "limits");
    assert_eq!(
        harness.state().changes,
        [
            Change::ValueEdited(0),
            Change::Toggled(1),
            Change::KeyEdited(1),
            Change::Removed(0)
        ]
    );
}

#[test]
fn tab_walks_the_cells_row_by_row() {
    let mut harness = table_harness(params());
    harness.get_by_label("Enabled, row 1").focus();
    harness.run();
    for next in [
        "Key, row 1",
        "Value, row 1",
        "Remove, row 1",
        "Enabled, row 2",
        "Key, row 2",
    ] {
        harness.key_press(Key::Tab);
        harness.run();
        assert!(
            harness.get_by_label(next).accesskit_node().is_focused(),
            "Tab should reach {next:?}"
        );
    }
    harness.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert!(
        harness
            .get_by_label("Enabled, row 2")
            .accesskit_node()
            .is_focused()
    );
}

#[test]
fn enter_in_a_key_moves_to_its_value() {
    let mut harness = table_harness(params());
    harness.get_by_label("Key, row 1").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        harness
            .get_by_label("Value, row 1")
            .accesskit_node()
            .is_focused()
    );
    assert!(harness.state().changes.is_empty());
}

#[test]
fn delete_and_backspace_on_the_empty_new_row_do_nothing() {
    let mut harness = table_harness(params());
    harness.get_by_label("Key, new row").focus();
    harness.run();
    for key in [Key::Delete, Key::Backspace] {
        harness.key_press(key);
        harness.run();
    }
    assert_eq!(harness.state().rows, params());
    assert!(harness.state().changes.is_empty());
    assert!(
        harness
            .get_by_label("Key, new row")
            .accesskit_node()
            .is_focused()
    );
}

#[test]
fn read_only_tables_have_no_trailing_row_or_delete_buttons() {
    let mut harness = testing::harness_state(Theme::dark(), params(), |ui, rows| {
        KeyValueTable::new("params", rows).read_only(true).show(ui);
    });
    assert!(harness.query_by_label("Key, new row").is_none());
    assert!(harness.query_by_label("Remove, row 1").is_none());
    assert!(harness.get_by_label("Key, row 1").is_disabled());
    assert!(harness.get_by_label("Enabled, row 1").is_disabled());
    harness.get_by_label("Enabled, row 1").click();
    harness.run();
    assert_eq!(*harness.state(), params());
}

#[test]
fn a_custom_value_cell_replaces_the_jb_text_field() {
    let mut drawn = 0;
    let mut rows = params();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        KeyValueTable::new("params", &mut rows)
            .value_cell(|ui, cell: ValueCell<'_>| {
                drawn += 1;
                ui.set_width(cell.width);
                cell.text_field(ui)
            })
            .show(ui);
    });
    harness.run();
    drop(harness);
    assert!(
        drawn >= 3,
        "rows and the trailing row use the cell: {drawn}"
    );
}

/// Rows of a caller's own model type are edited in place.
#[derive(Default)]
struct Header {
    on: bool,
    name: String,
    content: String,
    id: u32,
}

impl KeyValueRow for Header {
    fn enabled_mut(&mut self) -> &mut bool {
        &mut self.on
    }
    fn key_mut(&mut self) -> &mut String {
        &mut self.name
    }
    fn value_mut(&mut self) -> &mut String {
        &mut self.content
    }
}

#[test]
fn edits_the_callers_own_row_type_in_place() {
    let rows = vec![Header {
        on: true,
        name: "Accept".into(),
        content: "*/*".into(),
        id: 7,
    }];
    let mut harness = testing::harness_state(Theme::dark(), rows, |ui, rows| {
        KeyValueTable::new("headers", rows).show(ui);
    });
    harness.get_by_label("Value, row 1").focus();
    harness.get_by_label("Value, row 1").type_text("x");
    harness.get_by_label("Key, new row").focus();
    harness.get_by_label("Key, new row").type_text("Host");
    harness.run();
    let rows = harness.state();
    assert_eq!(rows[0].content, "*/*x");
    assert_eq!(rows[0].id, 7, "other fields are untouched");
    assert_eq!((rows[1].on, rows[1].name.as_str()), (true, "Host"));
}

#[test]
fn the_demo_screen_passes_the_audit() {
    let mut rows = vec![
        KeyValueEntry::new("page", "1"),
        KeyValueEntry {
            enabled: false,
            ..KeyValueEntry::new("debug", "true")
        },
    ];
    let mut fixed = params();
    for theme in [Theme::dark(), Theme::light()] {
        let mut harness = testing::harness(theme, |ui| {
            KeyValueTable::new("params", &mut rows).show(ui);
            KeyValueTable::new("fixed", &mut fixed)
                .read_only(true)
                .show(ui);
        });
        testing::audit(&mut harness).assert_clean();
    }
}
