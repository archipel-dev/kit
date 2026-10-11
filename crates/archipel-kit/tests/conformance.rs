//! Every interactive component passes the conformance battery, and the audit catches the
//! mistakes it is meant to catch.

mod common;

use archipel_kit::components::{
    Button, Checkbox, Column, Dropdown, IconButton, Link, ListRow, MenuItem, Radio,
    SegmentedControl, Tab, TabBar, Table, TextField, Toggle, Tree, TreeRow, VirtualList,
};
use archipel_kit::testing::{self, Problem};
use archipel_kit::{Glyph, Theme};

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
