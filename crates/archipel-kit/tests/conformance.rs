//! Every interactive component passes the conformance battery, and the audit catches the
//! mistakes it is meant to catch.

mod common;

use archipel_kit::components::{
    Button, Cell, Checkbox, CodeEditor, CodeView, Column, Dropdown, IconButton, Link, ListRow,
    MenuItem, Radio, SegmentedControl, Tab, TabBar, Table, TextField, Toggle, Tree, TreeRow,
    TreeTable, VirtualList,
};
use archipel_kit::testing::{self, Problem, Queryable};
use archipel_kit::{Glyph, Theme, Tone};

#[test]
fn buttons_conform() {
    for variant in ["primary", "secondary", "ghost", "danger"] {
        testing::conformance("Run", move |ui, enabled| {
            let button = match variant {
                "primary" => Button::primary("Run"),
                "secondary" => Button::secondary("Run"),
                "ghost" => Button::ghost("Run"),
                _ => Button::danger("Run"),
            };
            ui.add(button.icon(Glyph::Run).enabled(enabled))
        })
        .run();
    }
    testing::conformance("Small", |ui, enabled| {
        ui.add(Button::secondary("Small").small(true).enabled(enabled))
    })
    .run();
}

#[test]
fn icon_button_conforms() {
    testing::conformance("Reload", |ui, enabled| {
        ui.add(IconButton::new(Glyph::Refresh, "Reload").enabled(enabled))
    })
    .run();
}

#[test]
fn choices_conform() {
    let mut on = false;
    testing::conformance("Format on save", move |ui, enabled| {
        ui.add_enabled(enabled, Checkbox::new(&mut on, "Format on save"))
    })
    .run();
    let mut indent = "tabs";
    testing::conformance("Spaces", move |ui, enabled| {
        ui.add_enabled(enabled, Radio::new(&mut indent, "spaces", "Spaces"))
    })
    .run();
    let mut toggle = true;
    testing::conformance("Power save", move |ui, enabled| {
        ui.add_enabled(enabled, Toggle::new(&mut toggle, "Power save"))
    })
    .run();
}

#[test]
fn link_and_menu_item_conform() {
    testing::conformance("Configure…", |ui, enabled| {
        ui.add_enabled(enabled, Link::new("Configure…"))
    })
    .run();
    testing::conformance("Copy", |ui, enabled| {
        ui.add(MenuItem::new("Copy").shortcut("Ctrl+C").enabled(enabled))
    })
    .run();
}

#[test]
fn list_row_conforms() {
    testing::conformance("main.rs", |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            ListRow::new("main.rs").enabled(enabled).show(ui)
        })
        .inner
        .response
    })
    .run();
}

#[test]
fn text_field_conforms() {
    let mut text = String::from("archipel");
    testing::conformance("Project name", move |ui, enabled| {
        ui.add(
            TextField::new(&mut text)
                .label("Project name")
                .enabled(enabled),
        )
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn code_editor_conforms() {
    let mut text = String::from("let a = 1;");
    testing::conformance("Script", move |ui, enabled| {
        ui.add(
            CodeEditor::new(&mut text)
                .label("Script")
                .enabled(enabled)
                .height(80.0),
        )
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn code_view_and_editor_compose_audit_clean() {
    let mut text = String::from("{\n  \"a\": 1\n}");
    let mut harness = testing::harness(Theme::dark(), |ui| {
        CodeView::new("{\n  \"a\": 1\n}")
            .label("Response")
            .line_numbers(true)
            .folding(true)
            .height(80.0)
            .show(ui);
        ui.add(
            CodeEditor::new(&mut text)
                .label("Request")
                .line_numbers(true)
                .height(80.0),
        );
    });
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn dropdown_conforms() {
    let mut method = "GET";
    testing::conformance("HTTP method", move |ui, enabled| {
        ui.add(
            Dropdown::new("method", &mut method, ["GET", "POST"])
                .label("HTTP method")
                .enabled(enabled),
        )
    })
    .run();
}

fn tree_table_rows() -> Vec<TreeRow> {
    vec![
        TreeRow::new("name", 0).editable(true),
        TreeRow::new("address", 0).icon(Glyph::Folder).parent(true),
        TreeRow::new("city", 1),
    ]
}

/// The collection holds the focus as a whole (Tab reaches it once); its rows are pointer
/// targets and its keys are the arrows, not Enter/Space.
#[test]
fn tree_table_conforms() {
    let rows = tree_table_rows();
    let mut columns = vec![Column::new("Key", 120.0), Column::new("Value", 120.0)];
    let mut selected = None;
    testing::conformance("Document", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            TreeTable::new("doc", &mut columns, &rows, &mut selected)
                .name("Document")
                .height(96.0)
                .show(ui, |_, _| Cell::new("value").editable(true))
                .response
        })
        .inner
    })
    .no_click()
    .keys(&[])
    .run();
}

/// A log that follows its tail and a tree with a busy row hold the focus like any collection.
#[test]
fn follow_tail_list_and_busy_tree_conform() {
    let mut selected = None;
    let mut follow = true;
    testing::conformance("Messages", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            VirtualList::new("log", 30, &mut selected)
                .name("Messages")
                .height(96.0)
                .follow_tail(&mut follow)
                .show(ui, |i| ListRow::new(format!("message {i}")))
                .response
        })
        .inner
    })
    .no_click()
    .keys(&[])
    .run();

    let rows = vec![
        TreeRow::new("orders", 0)
            .icon(Glyph::Folder)
            .icon_tone(Tone::Warning)
            .parent(true)
            .busy(true),
        TreeRow::new("users", 0).icon(Glyph::Folder).parent(false),
    ];
    let mut selected = None;
    testing::conformance("Databases", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            Tree::new("db", &rows, &mut selected)
                .name("Databases")
                .height(72.0)
                .show(ui)
                .response
        })
        .inner
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn reorderable_editable_table_conforms_and_passes_the_audit() {
    let mut columns = vec![
        Column::new("Name", 120.0),
        Column::new("Size", 80.0).right(),
    ];
    let mut selected = None;
    testing::conformance("Files", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            Table::new("files", &mut columns, 5, &mut selected)
                .name("Files")
                .reorderable(true)
                .height(96.0)
                .show(ui, |r, c| Cell::new(format!("{r}:{c}")).editable(c == 0))
                .response
        })
        .inner
    })
    .no_click()
    .keys(&[])
    .run();

    let mut columns = vec![
        Column::new("Name", 120.0),
        Column::new("Size", 80.0).right(),
    ];
    let mut selected = Some(0);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = Table::new("files", &mut columns, 5, &mut selected)
            .name("Files")
            .reorderable(true)
            .height(96.0)
            .show(ui, |r, c| Cell::new(format!("{r}:{c}")).editable(c == 0));
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Files").focus();
    harness.key_press(egui::Key::F2);
    harness.run();
    assert!(harness.query_by_label("Edit Name").is_some());
    let audit = testing::audit(&mut harness);
    for problem in [Problem::MissingName, Problem::SmallTarget, Problem::Overlap] {
        assert!(audit.of(problem).is_empty(), "{problem:?}: {audit:?}");
    }
}

/// The audit passes at rest, and with the editor open the field is named, big enough and
/// clear of other targets.
#[test]
fn tree_table_passes_the_audit_while_editing() {
    let rows = tree_table_rows();
    let mut columns = vec![Column::new("Key", 120.0), Column::new("Value", 120.0)];
    let mut selected = Some(0);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = TreeTable::new("doc", &mut columns, &rows, &mut selected)
            .name("Document")
            .height(96.0)
            .show(ui, |_, _| Cell::new("value").editable(true));
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Document").focus();
    harness.key_press(egui::Key::F2);
    harness.run();
    assert!(harness.query_by_label("Edit Key").is_some());
    // Tab leaves (and so commits) the field, so reachability is not meaningful while it is
    // open; the field must still be named, big enough and clear of the other targets.
    let audit = testing::audit(&mut harness);
    for problem in [Problem::MissingName, Problem::SmallTarget, Problem::Overlap] {
        assert!(audit.of(problem).is_empty(), "{problem:?}: {audit:?}");
    }
}

#[test]
fn screens_with_navigation_and_collections_pass_the_audit() {
    let mut selected = 1;
    let tabs = vec![Tab::new("main.rs").closable(true), Tab::new("lib.rs")];
    let active = 0;
    let mut row = Some(0);
    let mut tree_row = None;
    let rows = vec![
        TreeRow::new("src", 0).icon(Glyph::Folder).parent(true),
        TreeRow::new("main.rs", 1),
    ];
    let mut columns = vec![
        Column::new("Name", 120.0),
        Column::new("Size", 80.0).right(),
    ];
    let mut table_row = None;
    let mut tree_columns = columns.clone();
    let mut tree_table_row = None;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(SegmentedControl::new(
            &mut selected,
            [(0, "Tree"), (1, "List")],
        ));
        let _ = TabBar::new(&tabs, active).show(ui);
        let _ = VirtualList::new("list", 50, &mut row)
            .height(72.0)
            .show(ui, |i| ListRow::new(format!("Item {i}")));
        let _ = Tree::new("tree", &rows, &mut tree_row)
            .height(48.0)
            .show(ui);
        let _ = Table::new("table", &mut columns, 20, &mut table_row)
            .height(72.0)
            .show(ui, |r, c| format!("{r}:{c}").into());
        let _ = TreeTable::new("tree-table", &mut tree_columns, &rows, &mut tree_table_row)
            .height(48.0)
            .show(ui, |r, c| format!("{r}:{c}").into());
    });
    testing::audit(&mut harness).assert_clean();
}

// The audit must catch what it promises to catch.

#[test]
fn audit_catches_unnamed_controls() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        // A raw click area with no name.
        let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 24.0), egui::Sense::click());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::GRAY);
        let _ = response;
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::MissingName).len(), 1, "{audit:?}");
}

#[test]
fn audit_catches_small_targets() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let (_, response) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Tiny"));
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::SmallTarget).len(), 1, "{audit:?}");
}

#[test]
fn audit_catches_overlapping_controls() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(60.0, 24.0));
        ui.put(rect, Button::secondary("One"));
        ui.put(
            rect.translate(egui::vec2(20.0, 0.0)),
            Button::secondary("Two"),
        );
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::Overlap).len(), 1, "{audit:?}");
}

#[test]
fn audit_passes_a_clean_form() {
    let mut name = String::new();
    let mut agree = false;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(TextField::new(&mut name).label("Name"));
        ui.add(Checkbox::new(&mut agree, "I agree"));
        ui.add(Button::primary("Create"));
    });
    testing::audit(&mut harness).assert_clean();
}

#[test]
#[should_panic(expected = "a click on \"Broken\" did not activate it")]
fn conformance_catches_a_button_that_ignores_clicks() {
    testing::conformance("Broken", |ui, enabled| {
        // Looks like a button, but senses only hover.
        let (_, response) = ui.allocate_exact_size(egui::vec2(80.0, 28.0), egui::Sense::hover());
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Broken"));
        response
    })
    .run();
}

#[test]
#[should_panic(expected = "doesn't report disabled")]
fn conformance_catches_a_fake_disabled_state() {
    testing::conformance("Run", |ui, _enabled| ui.add(Button::primary("Run"))).run();
}

#[test]
fn split_pane_divider_conforms() {
    use archipel_kit::components::SplitPane;
    let mut ratio = 0.5;
    testing::conformance("Resize sidebar", move |ui, enabled| {
        ui.allocate_ui(egui::vec2(320.0, 120.0), |ui| {
            SplitPane::horizontal("conformance", &mut ratio)
                .label("Resize sidebar")
                .enabled(enabled)
                .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"))
                .divider
        })
        .inner
    })
    .run();
}

#[test]
fn collapsible_split_pane_divider_conforms_open_and_collapsed() {
    use archipel_kit::components::{Side, SplitPane};
    for collapsed in [false, true] {
        let mut ratio = 0.5;
        testing::conformance("Resize sidebar", move |ui, enabled| {
            let mut collapsed = collapsed;
            ui.allocate_ui(egui::vec2(320.0, 120.0), |ui| {
                SplitPane::horizontal("conformance", &mut ratio)
                    .label("Resize sidebar")
                    .enabled(enabled)
                    .collapsible(Side::First, &mut collapsed)
                    .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"))
                    .divider
            })
            .inner
        })
        .run();
    }
}
