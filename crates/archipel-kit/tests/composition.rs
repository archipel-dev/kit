//! Slots and behaviours: content and interaction composed into components, checked for
//! behavior, accessibility, conformance and the audit.

mod common;

use archipel_kit::behaviour::{Cursor, DragSource, DropTarget, PopupHandler, WidgetExt};
use archipel_kit::parts::Slot;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Glyph, Icon, Theme, Tone, cursor};
use archipel_kit::{
    action_system::{ActionButton, ActionMenuItem},
    ide::HelpTooltip,
    ui::{Button, ColoredListCellRenderer},
};
use egui::CursorIcon;

#[test]
fn colored_list_cell_renderer_slots_snapshot() {
    testing::snapshot_themes("colored_list_cell_renderer_slots", common::IMG, |ui| {
        ColoredListCellRenderer::new("Problems")
            .icon(Glyph::Warning)
            .trailing(Slot::text("12"))
            .show(ui);
        ColoredListCellRenderer::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .trailing(Slot::text_tone("M", Tone::Modified))
            .show(ui);
        ColoredListCellRenderer::new("app")
            .icon(Glyph::Folder)
            .selected(true)
            .trailing(Slot::ui(|ui| {
                ui.add(ActionButton::new(Glyph::Close, "Remove app").small(true));
            }))
            .show(ui);
        ColoredListCellRenderer::new("lib")
            .icon(Glyph::Folder)
            .trailing(Slot::icon_tone(Glyph::Error, Tone::Error))
            .enabled(false)
            .show(ui);
    });
}

#[test]
fn trailing_actions_click_without_selecting_the_row() {
    let mut harness = testing::harness_state(Theme::dark(), (0, 0), |ui, (row_clicks, removes)| {
        let row = ColoredListCellRenderer::new("app")
            .trailing(Slot::ui(|ui| {
                if ui
                    .add(ActionButton::new(Glyph::Close, "Remove app").small(true))
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
        ColoredListCellRenderer::new("app")
            .selected(*selected)
            .trailing_on_hover(true)
            .trailing(Slot::ui(|ui| {
                ui.add(ActionButton::new(Glyph::Close, "Remove app").small(true));
            }))
            .show(ui);
        ColoredListCellRenderer::new("lib").show(ui);
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
        ColoredListCellRenderer::new("Problems")
            .trailing(Slot::text("12"))
            .enabled(enabled)
            .show(ui)
            .response
    })
    .run();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ColoredListCellRenderer::new("app")
            .trailing(Slot::ui(|ui| {
                ui.add(ActionButton::new(Glyph::Close, "Remove app").small(true));
            }))
            .show(ui);
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn popup_handler_opens_on_secondary_click() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, renames| {
        ColoredListCellRenderer::new("main.rs")
            .with(PopupHandler::new(|ui| {
                if ui.add(ActionMenuItem::new("Rename…")).clicked() {
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
                .with(HelpTooltip::new("Run 'app'").shortcut("Shift+F10"))
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
fn help_tooltip_behaviour_names_the_control() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(ActionButton::new(Glyph::Run, "Run").with(HelpTooltip::new("Run 'app'")));
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
        ColoredListCellRenderer::new("main.rs")
            .with(DragSource("main.rs"))
            .show(ui);
        ui.add_space(24.0);
        ColoredListCellRenderer::new("src")
            .with(DropTarget::new(dropped))
            .show(ui);
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
        let row = ColoredListCellRenderer::new("main.rs")
            .with(DragSource(1u32))
            .show(ui);
        if row.response.clicked() {
            *clicks += 1;
        }
    });
    harness.get_by_label("main.rs").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn draggable_expandable_row_keeps_its_tab_order() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ColoredListCellRenderer::new("src")
            .icon(Glyph::Folder)
            .expanded(Some(true))
            .with(DragSource("src".to_string()))
            .with(PopupHandler::new(|ui| {
                ui.add(ActionMenuItem::new("Rename…"));
            }))
            .show(ui);
        ColoredListCellRenderer::new("main.rs")
            .depth(1)
            .with(DragSource("main.rs".to_string()))
            .show(ui);
    });
    harness.run();
    // Row, chevron and the file row: every one reachable by Tab, in order.
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn draggable_folder_still_toggles_and_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), (0, 0), |ui, (clicks, toggles)| {
        let row = ColoredListCellRenderer::new("src")
            .expanded(Some(true))
            .with(DragSource(1u32))
            .show(ui);
        *clicks += row.response.clicked() as u32;
        *toggles += row.toggled as u32;
    });
    harness.get_by_label("Collapse src").click();
    harness.run();
    assert_eq!(harness.state().1, 1);
    harness.get_by_label("src").click();
    harness.run();
    assert_eq!(harness.state().0, 1);
}

#[test]
fn colored_list_cell_renderer_options_snapshot() {
    testing::snapshot_themes("colored_list_cell_renderer_options", common::IMG, |ui| {
        ui.set_width(300.0);
        ColoredListCellRenderer::new("Gradle")
            .icon(Glyph::Folder)
            .expanded(Some(true))
            .strong(true)
            .show(ui);
        ColoredListCellRenderer::new("Search hit in main.rs")
            .icon(Icon::for_file("main.rs"))
            .tint(Tone::Warning)
            .depth(1)
            .show(ui);
        ColoredListCellRenderer::new("app (running)")
            .icon(Glyph::Run)
            .icon_tone(Tone::Run)
            .tint(Tone::Success)
            .strong(true)
            .selected(true)
            .show(ui);
        ColoredListCellRenderer::new("Failed: tests")
            .icon(Glyph::Error)
            .icon_tone(Tone::Error)
            .tint(Tone::Error)
            .show(ui);
        ColoredListCellRenderer::new("theme.rs")
            .icon(Icon::for_file("theme.rs"))
            .leading(Slot::ui_width(22.0, |ui| {
                ui.add(
                    archipel_kit::ui::JBCheckBox::new(&mut true, "Include theme.rs")
                        .label_visible(false),
                );
            }))
            .secondary("src/")
            .show(ui);
        ColoredListCellRenderer::new("Badged.rs")
            .icon_painter(|painter, rect, color| {
                painter.rect_filled(rect.shrink(2.0), 3.0, color);
                painter.circle_filled(
                    rect.right_bottom() - egui::vec2(3.0, 3.0),
                    3.5,
                    Tone::Error.resolve(painter.ctx()),
                );
            })
            .show(ui);
        ColoredListCellRenderer::new(
            "A very long file name that does not fit into this row at all.kt",
        )
        .icon(Icon::for_file("a.kt"))
        .secondary("with/a/long/path")
        .trailing(Slot::text("12"))
        .show(ui);
        ColoredListCellRenderer::new(
            "A very long file name that does not fit into this row at all.kt",
        )
        .icon(Icon::for_file("a.kt"))
        .enabled(false)
        .show(ui);
    });
}

#[test]
fn long_label_is_elided_with_its_full_text_as_tooltip() {
    let long = "A very long file name that does not fit into this row at all.kt";
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.set_width(160.0);
        ColoredListCellRenderer::new(long)
            .trailing(Slot::text("12"))
            .show(ui);
    });
    harness.run();
    harness.get_by_label(long).hover();
    harness.run_steps(40);
    // The row's name plus the tooltip showing the label in full.
    assert_eq!(harness.query_all_by_label(long).count(), 2);
}

#[test]
fn short_label_has_no_tooltip() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ColoredListCellRenderer::new("main.rs").show(ui);
    });
    harness.get_by_label("main.rs").hover();
    harness.run_steps(40);
    assert_eq!(harness.query_all_by_label("main.rs").count(), 1);
}

#[test]
fn rows_with_leading_slot_pass_conformance_and_the_audit() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let mut checked = true;
        ColoredListCellRenderer::new("theme.rs")
            .leading(Slot::ui_width(22.0, |ui| {
                ui.add(
                    archipel_kit::ui::JBCheckBox::new(&mut checked, "Include theme.rs")
                        .label_visible(false),
                );
            }))
            .tint(Tone::Warning)
            .strong(true)
            .show(ui);
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    testing::conformance("theme.rs", |ui, enabled| {
        ColoredListCellRenderer::new("theme.rs")
            .strong(true)
            .tint(Tone::Success)
            .icon_painter(|painter, rect, color| {
                painter.circle_filled(rect.center(), 4.0, color);
            })
            .enabled(enabled)
            .show(ui)
            .response
    })
    .run();
}

#[test]
fn rows_with_a_leading_check_box_and_a_drag_source_conform() {
    testing::conformance("theme.rs", |ui, enabled| {
        let mut checked = true;
        ColoredListCellRenderer::new("theme.rs")
            .leading(Slot::ui_width(22.0, |ui| {
                ui.add(
                    archipel_kit::ui::JBCheckBox::new(&mut checked, "Include theme.rs")
                        .label_visible(false),
                );
            }))
            .enabled(enabled)
            .show(ui)
            .response
    })
    .run();
    testing::conformance("src", |ui, enabled| {
        ColoredListCellRenderer::new("src")
            .icon(Glyph::Folder)
            .expanded(Some(true))
            .with(DragSource("src".to_string()))
            .enabled(enabled)
            .show(ui)
            .response
    })
    .run();
}

/// Nexus HTTP's sidebar (kit issue #21): draggable `ColoredListCellRenderer`s in a `JBList` are
/// reached with the arrow keys after Tab focuses the list. `DragSource` must not make the
/// rows Tab stops of their own, or the audit reports them as unreachable.
#[test]
fn draggable_rows_in_a_jb_list_pass_the_audit() {
    use archipel_kit::ui::JBList;
    let names = ["Auth", "Login", "List users", "Get invoice"];
    let mut selected = None;
    for theme in [Theme::dark(), Theme::light()] {
        let mut harness = testing::harness(theme, |ui| {
            let _ = JBList::new("sidebar", names.len(), &mut selected)
                .name("Requests")
                .height(200.0)
                .show(ui, |i| {
                    ColoredListCellRenderer::new(names[i])
                        .icon(Glyph::Folder)
                        .with(DragSource(i))
                });
        });
        harness.set_size(egui::vec2(300.0, 720.0));
        harness.run();
        testing::audit(&mut harness).assert_clean();
    }
}
