//! `SegmentedControl`, `TabBar` and `ListRow` tests: behavior, accessibility and snapshots.

mod common;

use archipel_kit::components::{ListRow, SegmentedControl, Tab, TabBar};
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::{Icon, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Pretty,
    Raw,
    Preview,
}

const VIEWS: [(View, &str); 3] = [
    (View::Pretty, "Pretty"),
    (View::Raw, "Raw"),
    (View::Preview, "Preview"),
];

fn tabs() -> Vec<Tab> {
    vec![
        Tab::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .closable(true),
        Tab::new("Cargo.toml")
            .icon(Icon::for_file("Cargo.toml"))
            .closable(true)
            .modified(true),
        Tab::new("README.md")
            .icon(Icon::for_file("README.md"))
            .closable(true),
    ]
}

#[test]
fn segmented_snapshot() {
    testing::snapshot_themes("segmented_control", common::IMG, |ui| {
        ui.add(SegmentedControl::new(&mut View::Pretty, VIEWS));
        ui.add(SegmentedControl::new(&mut View::Raw, VIEWS).enabled(false));
    });
}

#[test]
fn segmented_selects_clicked_option() {
    let mut harness = testing::harness_state(Theme::dark(), View::Pretty, |ui, view| {
        ui.add(SegmentedControl::new(view, VIEWS));
    });
    assert!(harness.get_by_label("Pretty").is_checked());
    harness.get_by_label("Raw").click();
    harness.run();
    assert_eq!(*harness.state(), View::Raw);
    assert!(harness.get_by_label("Raw").is_checked());
    assert!(!harness.get_by_label("Pretty").is_checked());
}

#[test]
fn tab_bar_snapshot() {
    testing::snapshot_themes("tab_bar", common::IMG, |ui| {
        TabBar::new(&tabs(), 0).show(ui);
    });
}

#[test]
fn tab_bar_reports_selection_and_close() {
    let tabs = tabs();
    let mut harness =
        testing::harness_state(Theme::dark(), (0usize, None), |ui, (current, closed)| {
            let out = TabBar::new(&tabs, *current).show(ui);
            if let Some(index) = out.selected {
                *current = index;
            }
            if out.closed.is_some() {
                *closed = out.closed;
            }
        });
    harness.get_by_label("README.md").click();
    harness.run();
    assert_eq!(harness.state().0, 2);
    assert!(harness.get_by_label("README.md").is_checked());
    harness.get_by_label("Close README.md").click();
    harness.run();
    assert_eq!(harness.state().1, Some(2));
}

#[test]
fn list_row_snapshot() {
    testing::snapshot_themes("list_row", common::IMG, |ui| {
        let colors = Theme::colors(ui.ctx());
        ListRow::new("archipel")
            .icon(Icon::Project)
            .expanded(Some(true))
            .show(ui);
        ListRow::new("src")
            .icon(Icon::Folder)
            .depth(1)
            .expanded(Some(true))
            .show(ui);
        ListRow::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .depth(2)
            .selected(true)
            .show(ui);
        ListRow::new("theme.rs")
            .icon(Icon::for_file("theme.rs"))
            .depth(2)
            .label_color(colors.vcs_modified)
            .show(ui);
        ListRow::new("target")
            .icon(Icon::Folder)
            .depth(1)
            .expanded(Some(false))
            .secondary("excluded")
            .show(ui);
        ListRow::new("Cargo.toml")
            .icon(Icon::for_file("Cargo.toml"))
            .depth(1)
            .secondary("1 KB")
            .show(ui);
        ListRow::new("Disabled")
            .icon(Icon::File)
            .depth(1)
            .enabled(false)
            .show(ui);
    });
}

#[derive(Default)]
struct TreeState {
    expanded: bool,
    selected: Option<&'static str>,
    opened: Option<&'static str>,
}

fn tree(ui: &mut egui::Ui, state: &mut TreeState) {
    let row = ListRow::new("src")
        .icon(Icon::Folder)
        .expanded(Some(state.expanded))
        .show(ui);
    if row.toggled {
        state.expanded = !state.expanded;
    }
    if row.response.clicked() {
        state.selected = Some("src");
    }
    if state.expanded {
        let row = ListRow::new("main.rs")
            .depth(1)
            .icon(Icon::File)
            .selected(state.selected == Some("main.rs"))
            .show(ui);
        if row.response.clicked() {
            state.selected = Some("main.rs");
        }
        if row.response.double_clicked() {
            state.opened = Some("main.rs");
        }
    }
}

#[test]
fn list_row_click_selects_and_chevron_expands() {
    let mut harness = testing::harness_state(Theme::dark(), TreeState::default(), tree);
    assert!(harness.query_by_label("main.rs").is_none());
    // The chevron sits at the start of the row.
    let row = harness.get_by_label("src").rect();
    harness.hover_at(egui::pos2(row.left() + 14.0, row.center().y));
    harness.event(egui::Event::PointerButton {
        pos: egui::pos2(row.left() + 14.0, row.center().y),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos: egui::pos2(row.left() + 14.0, row.center().y),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    harness.run();
    assert!(harness.state().expanded);
    harness.get_by_label("main.rs").click();
    harness.run();
    assert_eq!(harness.state().selected, Some("main.rs"));
    assert!(harness.get_by_label("main.rs").is_checked());
}

#[test]
fn arrow_keys_expand_and_collapse_a_focused_row() {
    let mut harness = testing::harness_state(Theme::dark(), TreeState::default(), tree);
    harness.get_by_label("src").focus();
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert!(harness.state().expanded);
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().expanded);
}

#[test]
fn double_click_toggles_a_node() {
    let mut harness = testing::harness_state(Theme::dark(), TreeState::default(), tree);
    let center = harness.get_by_label("src").rect().center();
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
        }
    }
    harness.run();
    assert!(harness.state().expanded);
}
