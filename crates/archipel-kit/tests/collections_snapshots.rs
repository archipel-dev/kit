//! Snapshots of the selection, speed search, empty text, loading and collapsible states
//! shown in the docs.

mod common;

use archipel_kit::parts::SimpleTextAttributes;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::ui::{
    Cell, CollapsiblePanel, ColoredListCellRenderer, ColumnInfo, JBList, JBLoadingPanel, JBTable,
    ListSelectionModel, SpeedSearch, SpeedSearchState, StatusText, Tree, TreeNode,
};
use archipel_kit::{Glyph, Icon};
use egui::{Event, Modifiers, PointerButton};

const FILES: [&str; 7] = [
    "Cargo.toml",
    "README.md",
    "docs",
    "NullPointerException",
    "readme.txt",
    "src",
    "tests",
];

/// Clicks the row named `label`, with `mods` held.
fn click(harness: &mut testing::Harness<'_>, label: &str, mods: Modifiers) {
    // Snapshots render at 2x; AccessKit bounds are in physical pixels, events in points.
    let pos = harness.get_by_label(label).rect().center() / testing::SNAPSHOT_SCALE;
    harness.hover_at(pos);
    harness.run_ok();
    for pressed in [true, false] {
        harness.event_modifiers(
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: mods,
            },
            mods,
        );
    }
    harness.run_ok();
}

#[test]
fn multi_selection_snapshot() {
    let shift = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    let mut selection = ListSelectionModel::multiple();
    testing::snapshot_themes_after(
        "jb_list_multi_selection",
        common::IMG,
        |ui| {
            ui.set_width(280.0);
            let _ = JBList::new("multi", FILES.len(), &mut selection)
                .name("Files")
                .height(180.0)
                .show(ui, |i| {
                    ColoredListCellRenderer::new(FILES[i]).icon(Icon::for_file(FILES[i]))
                });
        },
        |harness| {
            click(harness, "Cargo.toml", Modifiers::NONE);
            click(harness, "docs", shift);
            click(harness, "readme.txt", ctrl);
            click(harness, "tests", ctrl);
        },
    );
}

#[test]
fn speed_search_snapshot() {
    let mut selected = None;
    let mut search = SpeedSearchState::new();
    testing::snapshot_themes_after(
        "speed_search",
        common::IMG,
        |ui| {
            ui.set_width(280.0);
            let _ = JBList::new("search", FILES.len(), &mut selected)
                .name("Files")
                .height(180.0)
                .speed_search(SpeedSearch::new(&mut search))
                .show(ui, |i| {
                    ColoredListCellRenderer::new(FILES[i]).icon(Icon::for_file(FILES[i]))
                });
        },
        |harness| {
            click(harness, "Cargo.toml", Modifiers::NONE);
            harness.event(Event::Text("re".into()));
            harness.run_ok();
        },
    );
}

#[test]
fn speed_search_filter_snapshot() {
    let mut selected = None;
    let mut search = SpeedSearchState::new();
    testing::snapshot_themes_after(
        "speed_search_filter",
        common::IMG,
        |ui| {
            ui.set_width(280.0);
            let _ = JBList::new("filter", FILES.len(), &mut selected)
                .name("Files")
                .height(180.0)
                .speed_search(SpeedSearch::new(&mut search).filter(true))
                .show(ui, |i| {
                    ColoredListCellRenderer::new(FILES[i]).icon(Icon::for_file(FILES[i]))
                });
        },
        |harness| {
            click(harness, "Cargo.toml", Modifiers::NONE);
            harness.event(Event::Text("re".into()));
            harness.run_ok();
        },
    );
}

#[test]
fn table_speed_search_snapshot() {
    let mut selected = None;
    let mut search = SpeedSearchState::new();
    let mut columns = vec![
        ColumnInfo::new("Method", 80.0),
        ColumnInfo::new("Path", 180.0),
    ];
    testing::snapshot_themes_after(
        "table_speed_search",
        common::IMG,
        |ui| {
            let _ = JBTable::new("table-search", &mut columns, 5, &mut selected)
                .name("Requests")
                .height(120.0)
                .speed_search(SpeedSearch::new(&mut search))
                .show(ui, |row, col| match col {
                    0 => Cell::new(["GET", "POST", "GET", "PUT", "GET"][row]),
                    _ => Cell::new(format!("/orders/{row}")),
                });
        },
        |harness| {
            click(harness, "GET, /orders/0", Modifiers::NONE);
            harness.event(Event::Text("po".into()));
            harness.run_ok();
        },
    );
}

#[test]
fn empty_collection_snapshot() {
    testing::snapshot_themes("jb_list_empty", common::IMG, |ui| {
        ui.set_width(300.0);
        let mut selected = None;
        let _ = JBList::new("empty", 0, &mut selected)
            .name("Requests")
            .height(100.0)
            .empty_text(
                StatusText::text("No requests yet. ")
                    .link("Create one")
                    .line("or press Ctrl+N"),
            )
            .show(ui, |_| ColoredListCellRenderer::new("never"));
    });
}

#[test]
fn status_text_lines_snapshot() {
    testing::snapshot_themes("status_text_lines", common::IMG, |ui| {
        ui.set_width(300.0);
        let _ = StatusText::text("Nothing is selected")
            .line_with(
                "Select a request to see its details",
                SimpleTextAttributes::GRAYED_ATTRIBUTES,
            )
            .append(" or ")
            .link("create one")
            .show_with_links(ui);
    });
}

#[test]
fn fragments_snapshot() {
    testing::snapshot_themes("colored_list_cell_renderer_fragments", common::IMG, |ui| {
        ui.set_width(320.0);
        ColoredListCellRenderer::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .append("  src/bin", SimpleTextAttributes::GRAYED_ATTRIBUTES)
            .append("  2 errors", SimpleTextAttributes::ERROR_ATTRIBUTES)
            .show(ui);
        ColoredListCellRenderer::new("lib.rs")
            .icon(Icon::for_file("lib.rs"))
            .append(
                "  modified",
                SimpleTextAttributes::REGULAR_ITALIC_ATTRIBUTES,
            )
            .append("  see ", SimpleTextAttributes::GRAYED_SMALL_ATTRIBUTES)
            .append("docs", SimpleTextAttributes::LINK_ATTRIBUTES)
            .show(ui);
        ColoredListCellRenderer::new("old.rs")
            .icon(Icon::for_file("old.rs"))
            .append("  removed", SimpleTextAttributes::EXCLUDED_ATTRIBUTES)
            .append("  bold", SimpleTextAttributes::REGULAR_BOLD_ATTRIBUTES)
            .show(ui);
    });
}

#[test]
fn tree_multi_selection_snapshot() {
    let rows = vec![
        TreeNode::new("kit", 0).icon(Glyph::Folder).parent(true),
        TreeNode::new("src", 1).icon(Glyph::Folder).parent(true),
        TreeNode::new("lib.rs", 2).icon(Icon::for_file("lib.rs")),
        TreeNode::new("theme.rs", 2).icon(Icon::for_file("theme.rs")),
        TreeNode::new("tests", 1).icon(Glyph::Folder).parent(false),
    ];
    let shift = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    let mut selection = ListSelectionModel::multiple();
    testing::snapshot_themes_after(
        "tree_multi_selection",
        common::IMG,
        |ui| {
            ui.set_width(280.0);
            let _ = Tree::new("project", &rows, &mut selection)
                .name("Project")
                .height(140.0)
                .show(ui);
        },
        |harness| {
            click(harness, "src", Modifiers::NONE);
            click(harness, "theme.rs", shift);
        },
    );
}

#[test]
fn loading_panel_snapshot() {
    testing::snapshot_themes("loading_panel", common::IMG, |ui| {
        ui.set_width(300.0);
        JBLoadingPanel::new("results", true)
            .text("Loading results…")
            .show(ui, |ui| {
                let mut selected = Some(1);
                let _ = JBList::new("under", 4, &mut selected)
                    .height(96.0)
                    .show(ui, |i| {
                        ColoredListCellRenderer::new(format!("GET /users/{i}"))
                    });
            });
    });
}

#[test]
fn collapsible_panel_snapshot() {
    testing::snapshot_themes("collapsible_panel", common::IMG, |ui| {
        ui.set_width(300.0);
        let mut expanded = false;
        CollapsiblePanel::new("Advanced settings", &mut expanded).show(ui, |ui| {
            ui.label("Timeout, proxy and TLS options");
        });
        let mut collapsed = true;
        CollapsiblePanel::new("Experimental", &mut collapsed).show(ui, |ui| {
            ui.label("Hidden");
        });
        let mut at_end = false;
        CollapsiblePanel::new("Chevron at the end", &mut at_end)
            .collapse_button_at_left(false)
            .show(ui, |ui| {
                ui.label("Body of the panel");
            });
    });
}
