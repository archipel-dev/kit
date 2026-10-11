//! Slots and behaviours: content and interaction composed into components, checked for
//! behavior, accessibility, conformance and the audit.

mod common;

use archipel_kit::behaviour::{ContextMenu, Cursor, DragSource, DropTarget, WidgetExt};
use archipel_kit::components::{Button, IconButton, ListRow, MenuItem, Tooltip};
use archipel_kit::parts::Slot;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Glyph, Icon, Theme, Tone, cursor};
use egui::CursorIcon;

#[test]
fn list_row_slots_snapshot() {
    testing::snapshot_themes("list_row_slots", common::IMG, |ui| {
        ListRow::new("Problems")
            .icon(Glyph::Warning)
            .trailing(Slot::text("12"))
            .show(ui);
        ListRow::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .trailing(Slot::text_tone("M", Tone::Modified))
            .show(ui);
        ListRow::new("app")
            .icon(Glyph::Folder)
            .selected(true)
            .trailing(Slot::ui(|ui| {
                ui.add(IconButton::new(Glyph::Close, "Remove app").small(true));
            }))
            .show(ui);
        ListRow::new("lib")
            .icon(Glyph::Folder)
            .trailing(Slot::icon_tone(Glyph::Error, Tone::Error))
            .enabled(false)
            .show(ui);
    });
}

#[test]
fn trailing_actions_click_without_selecting_the_row() {
    let mut harness = testing::harness_state(Theme::dark(), (0, 0), |ui, (row_clicks, removes)| {
        let row = ListRow::new("app")
            .trailing(Slot::ui(|ui| {
                if ui
                    .add(IconButton::new(Glyph::Close, "Remove app").small(true))
                    .clicked()
                {
                    *removes += 1;
                }
            }))
            .show(ui);
        if row.response.clicked() {
            *row_clicks += 1;
        }
    });
    harness.get_by_label("Remove app").click();
    harness.run();
    assert_eq!(*harness.state(), (0, 1));
    harness.get_by_label("app").click();
    harness.run();
    assert_eq!(*harness.state(), (1, 1));
}

#[test]
fn trailing_on_hover_shows_only_while_hovered_or_selected() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, selected| {
        ListRow::new("app")
            .selected(*selected)
            .trailing_on_hover(true)
            .trailing(Slot::ui(|ui| {
                ui.add(IconButton::new(Glyph::Close, "Remove app").small(true));
            }))
            .show(ui);
        ListRow::new("lib").show(ui);
    });
    harness.get_by_label("lib").hover();
    harness.run();
    assert!(harness.query_by_label("Remove app").is_none());
    harness.get_by_label("app").hover();
    harness.run();
    assert!(harness.query_by_label("Remove app").is_some());
    harness.get_by_label("lib").hover();
    *harness.state_mut() = true;
    harness.run();
    assert!(
        harness.query_by_label("Remove app").is_some(),
        "a selected row keeps its actions"
    );
}

#[test]
fn rows_with_slots_conform_and_pass_the_audit() {
    testing::conformance("Problems", |ui, enabled| {
        ListRow::new("Problems")
            .trailing(Slot::text("12"))
            .enabled(enabled)
            .show(ui)
            .response
    })
    .run();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ListRow::new("app")
            .trailing(Slot::ui(|ui| {
                ui.add(IconButton::new(Glyph::Close, "Remove app").small(true));
            }))
            .show(ui);
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn context_menu_opens_on_secondary_click() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, renames| {
        ListRow::new("main.rs")
            .with(ContextMenu::new(|ui| {
                if ui.add(MenuItem::new("Rename…")).clicked() {
                    *renames += 1;
                }
            }))
            .show(ui);
    });
    assert!(harness.query_by_label("Rename…").is_none());
    harness.get_by_label("main.rs").click_secondary();
    harness.run();
    harness.get_by_label("Rename…").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn behaviours_chain_on_any_widget() {
    let mut harness = testing::harness_state(Theme::dark(), (0, false), |ui, (clicks, seen)| {
        let response = ui.add(
            Button::primary("Run")
                .with(Tooltip::new("Run 'app'").shortcut("Shift+F10"))
                .with(|_ui: &mut egui::Ui, response: egui::Response| {
                    if response.clicked() {
                        *clicks += 1;
                    }
                    response
                }),
        );
        *seen = response.hovered();
    });
    harness.get_by_label("Run").click();
    harness.run();
    assert_eq!(harness.state().0, 1, "the closure saw the click");
    assert!(harness.state().1, "the outer response is the widget's");
}

#[test]
fn tooltip_behaviour_names_the_control() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(IconButton::new(Glyph::Run, "Run").with(Tooltip::new("Run 'app'")));
    });
    harness.get_by_label("Run").hover();
    harness.run_steps(40);
    assert!(harness.query_by_label("Run 'app'").is_some());
}

#[test]
fn cursor_behaviour_wins_over_the_component() {
    let mut harness = testing::harness_state(Theme::dark(), CursorIcon::Default, |ui, seen| {
        ui.add(Button::secondary("Split").with(Cursor(CursorIcon::ResizeHorizontal)));
        *seen = cursor::current(ui.ctx());
    });
    harness.get_by_label("Split").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::ResizeHorizontal);
}

#[test]
fn drag_source_drops_on_drop_target() {
    let mut harness = testing::harness_state(Theme::dark(), None::<&str>, |ui, dropped| {
        ListRow::new("main.rs").with(DragSource("main.rs")).show(ui);
        ui.add_space(24.0);
        ListRow::new("src").with(DropTarget::new(dropped)).show(ui);
    });
    harness.run();
    let from = harness.get_by_label("main.rs").rect().center();
    let to = harness.get_by_label("src").rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        harness.hover_at(from + (to - from) * t);
        harness.run();
    }
    harness.drop_at(to);
    harness.run();
    assert_eq!(*harness.state(), Some("main.rs"));
}

#[test]
fn drag_source_keeps_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        let row = ListRow::new("main.rs").with(DragSource(1u32)).show(ui);
        if row.response.clicked() {
            *clicks += 1;
        }
    });
    harness.get_by_label("main.rs").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}
