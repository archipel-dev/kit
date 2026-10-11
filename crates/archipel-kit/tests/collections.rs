//! `VirtualList`, `Tree` and `Table` tests.

mod common;

use archipel_kit::components::{
    Cell, Column, ListRow, SortOrder, Table, Tree, TreeRow, VirtualList,
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

#[test]
fn tree_rows_take_behaviours_and_pass_the_audit() {
    use archipel_kit::behaviour::{ContextMenu, DragSource, DropTarget};
    use archipel_kit::components::MenuItem;

    #[derive(Default)]
    struct State {
        selected: Option<usize>,
        renamed: std::cell::RefCell<Vec<usize>>,
        dropped: Option<usize>,
    }
    let mut harness = testing::harness_state(Theme::dark(), State::default(), |ui, state| {
        let rows = project_rows(true, false);
        let State {
            selected,
            renamed,
            dropped,
        } = state;
        let renamed = &*renamed;
        let drop_slot = std::cell::Cell::new(None);
        Tree::new("project", &rows, selected)
            .height(200.0)
            .show_with(ui, |index, row| {
                let row = row
                    .with(DragSource(index))
                    .with(ContextMenu::new(move |ui| {
                        if ui.add(MenuItem::new("Rename…")).clicked() {
                            renamed.borrow_mut().push(index);
                        }
                    }));
                if rows[index].has_children() {
                    row.with(DropTarget::shared(&drop_slot))
                } else {
                    row
                }
            });
        if let Some(target) = drop_slot.take() {
            *dropped = Some(target);
        }
    });
    harness.run();
    // Draggable folders keep the Tab order the audit checks.
    testing::audit(&mut harness).assert_clean();

    harness.get_by_label("lib.rs").click_secondary();
    harness.run();
    harness.get_by_label("Rename…").click();
    harness.run();
    assert_eq!(*harness.state().renamed.borrow(), [2]);

    // Dragging a file onto a folder drops it there.
    let from = harness.get_by_label("Cargo.toml workspace").rect().center();
    let to = harness.get_by_label("src").rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=8 {
        harness.hover_at(from + (to - from) * (step as f32 / 8.0));
        harness.run();
    }
    harness.drop_at(to);
    harness.run();
    assert_eq!(
        harness.state().dropped,
        Some(5),
        "the dragged row's payload"
    );
}

/// A flat list of tree nodes drawn from `ListRow`s: group 0 has two children.
fn node_harness() -> testing::Harness<'static, (Option<usize>, bool)> {
    let mut harness =
        testing::harness_state(Theme::dark(), (None, true), |ui, (selected, open)| {
            let rows = if *open { 3 } else { 1 };
            let out = VirtualList::new("nodes", rows, selected)
                .height(120.0)
                .show(ui, |i| match i {
                    0 => ListRow::new("Group").expanded(Some(*open)),
                    n => ListRow::new(format!("Child {n}")).depth(1),
                });
            if out.toggled == Some(0) {
                *open = !*open;
            }
        });
    harness.run();
    harness
}

#[test]
fn list_arrows_collapse_and_expand_nodes() {
    let mut harness = node_harness();
    harness.get_by_label("Group").click();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().1, "← collapses the expanded node");
    assert!(harness.query_by_label("Child 1").is_none());
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().1, "← on a collapsed node does nothing");
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.state().1, "→ expands the collapsed node");
    // Plain rows ignore the arrows.
    harness.get_by_label("Child 1").click();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(harness.state().1);
    assert_eq!(harness.state().0, Some(1));
}
