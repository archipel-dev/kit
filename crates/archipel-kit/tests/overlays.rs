//! `Tooltip`, `Dialog`, `MenuSeparator` and cursor tests.

mod common;

use archipel_kit::components::{
    Button, Dialog, DialogAction, IconButton, Link, ListRow, MenuItem, MenuSeparator, TextField,
    Tooltip, TooltipExt,
};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Glyph, Theme, cursor};
use egui::CursorIcon;

#[test]
fn tooltip_snapshot() {
    testing::snapshot_themes("tooltip", common::IMG, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        for tooltip in [
            Tooltip::new("Run 'gallery'").shortcut("Shift+F10"),
            Tooltip::new("Search Everywhere")
                .shortcut("Double Shift")
                .hint("Find files, actions, classes and symbols in one place."),
        ] {
            Tooltip::frame(ui.ctx()).show(ui, |ui| tooltip.content(ui));
        }
    });
}

#[test]
fn tooltip_shows_after_hovering() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(IconButton::new(Glyph::Run, "Run"))
            .tooltip_with(Tooltip::new("Run 'gallery'").shortcut("Shift+F10"));
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 160.0));
    assert!(harness.query_by_label("Run 'gallery'").is_none());
    harness.get_by_label("Run").hover();
    harness.run_steps(60);
    assert!(harness.query_by_label("Run 'gallery'").is_some());
}

#[test]
fn tooltip_explains_disabled_widgets() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(Button::primary("Deploy").enabled(false))
            .tooltip("Connect a cluster first");
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 160.0));
    harness.get_by_label("Deploy").hover();
    harness.run_steps(60);
    assert!(harness.query_by_label("Connect a cluster first").is_some());
}

#[test]
fn menu_separator_divides_items() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(MenuItem::new("Open…"));
        ui.add(MenuSeparator);
        ui.add(MenuItem::new("Exit"));
    });
    let open = harness.get_by_label("Open…").rect();
    let exit = harness.get_by_label("Exit").rect();
    assert!(exit.top() - open.bottom() >= 8.0, "{open:?} {exit:?}");
}

fn delete_dialog(ui: &mut egui::Ui) -> Option<DialogAction> {
    Dialog::new("Delete “main.rs”?")
        .icon(Glyph::Warning)
        .message("The file will be moved to the trash. You can restore it from there.")
        .primary("Delete")
        .danger(true)
        .secondary("Cancel")
        .show(ui.ctx())
}

#[test]
fn dialog_snapshot() {
    testing::snapshot_themes_sized("dialog", common::IMG, egui::vec2(520.0, 240.0), |ui| {
        ui.label("Editor content behind the dialog");
        let _ = delete_dialog(ui);
    });
}

#[test]
fn dialog_with_content_snapshot() {
    testing::snapshot_themes_sized("dialog_form", common::IMG, egui::vec2(520.0, 260.0), |ui| {
        ui.label("Editor content behind the dialog");
        let mut name = String::from("feature/islands");
        let _ = Dialog::new("New Branch")
            .content(|ui| {
                ui.add(TextField::new(&mut name).label("Branch name"));
            })
            .primary("Create")
            .secondary("Cancel")
            .width(360.0)
            .show(ui.ctx());
    });
}

/// Runs `delete_dialog` and records the first action.
fn dialog_harness() -> testing::Harness<'static, Option<DialogAction>> {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, action| {
        if let Some(done) = delete_dialog(ui) {
            action.get_or_insert(done);
        }
    });
    harness.set_size(egui::vec2(520.0, 240.0));
    harness.run();
    harness
}

#[test]
fn dialog_reports_buttons() {
    let mut harness = dialog_harness();
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(*harness.state(), Some(DialogAction::Primary));

    let mut harness = dialog_harness();
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(*harness.state(), Some(DialogAction::Secondary));
}

#[test]
fn dialog_keys_confirm_and_dismiss() {
    let mut harness = dialog_harness();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), Some(DialogAction::Primary));

    let mut harness = dialog_harness();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(*harness.state(), Some(DialogAction::Dismissed));
}

#[test]
fn dialog_disabled_primary_ignores_enter() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, action| {
        let shown = Dialog::new("Rename")
            .primary("Rename")
            .primary_enabled(false)
            .secondary("Cancel")
            .show(ui.ctx());
        if let Some(done) = shown {
            action.get_or_insert(done);
        }
    });
    harness.set_size(egui::vec2(520.0, 200.0));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), None);
}

/// Records the cursor at the end of each frame of `app`.
fn cursor_harness(
    mut app: impl FnMut(&mut egui::Ui) + 'static,
) -> testing::Harness<'static, CursorIcon> {
    testing::harness_state(Theme::dark(), CursorIcon::Default, move |ui, cursor| {
        app(ui);
        *cursor = cursor::current(ui.ctx());
    })
}

#[test]
fn components_set_their_cursor() {
    let mut harness = cursor_harness(|ui| {
        ui.add(Link::new("Configure…"));
        ui.add(Button::primary("Run"));
        ui.add(Button::secondary("Stop").enabled(false));
        ListRow::new("main.rs").show(ui);
    });
    harness.get_by_label("Configure…").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::PointingHand);
    harness.get_by_label("Run").hover();
    harness.run();
    assert_eq!(
        *harness.state(),
        CursorIcon::PointingHand,
        "actions show the hand"
    );
    harness.get_by_label("Stop").hover();
    harness.run();
    assert_eq!(
        *harness.state(),
        CursorIcon::Default,
        "disabled actions keep the arrow"
    );
    harness.get_by_label("main.rs").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::Default, "rows keep the arrow");
}

#[test]
fn theme_picks_the_action_cursor() {
    let theme = Theme {
        action_cursor: CursorIcon::Default,
        ..Theme::dark()
    };
    let mut harness = testing::harness_state(theme, CursorIcon::Wait, |ui, cursor| {
        ui.add(Button::primary("Run"));
        *cursor = cursor::current(ui.ctx());
    });
    harness.get_by_label("Run").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::Default);
}

#[test]
fn parent_override_wins_over_children() {
    let mut harness = cursor_harness(|ui| {
        cursor::override_in(ui, Some(CursorIcon::Progress), |ui| {
            ui.add(Link::new("Configure…"));
        });
    });
    harness.get_by_label("Configure…").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::Progress);
}

#[test]
fn no_override_leaves_children_alone() {
    let mut harness = cursor_harness(|ui| {
        cursor::override_in(ui, None, |ui| {
            ui.add(Link::new("Configure…"));
        });
    });
    harness.get_by_label("Configure…").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::PointingHand);
}

#[test]
fn fallback_loses_to_children() {
    let mut harness = cursor_harness(|ui| {
        cursor::fallback_in(ui, CursorIcon::Grab, |ui| {
            ui.add(Link::new("Configure…"));
            ui.add_space(40.0);
        });
    });
    harness.get_by_label("Configure…").hover();
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::PointingHand);
    let link = harness.get_by_label("Configure…").rect();
    harness.hover_at(egui::pos2(link.left() + 2.0, link.bottom() + 20.0));
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::Grab);
}

/// A dialog with a multiline field (focused) and a single-line one.
fn field_dialog_harness(
    multiline: bool,
) -> testing::Harness<'static, (Option<DialogAction>, String)> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (None, String::new()),
        move |ui, (action, text)| {
            let shown = Dialog::new("Note")
                .content(|ui| {
                    let edit = if multiline {
                        egui::TextEdit::multiline(text)
                    } else {
                        egui::TextEdit::singleline(text)
                    };
                    ui.add(edit.id(egui::Id::new("note")));
                })
                .primary("Save")
                .secondary("Cancel")
                .show(ui.ctx());
            if let Some(done) = shown {
                action.get_or_insert(done);
            }
        },
    );
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run();
    harness
        .get_by_role(egui::accesskit::Role::MultilineTextInput)
        .click();
    harness.run();
    harness
}

#[test]
fn dialog_enter_in_multiline_field_is_a_new_line() {
    let mut harness = field_dialog_harness(true);
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().0, None);
    assert_eq!(harness.state().1, "\n");
}

#[test]
fn dialog_ctrl_enter_confirms_from_a_multiline_field() {
    for modifiers in [egui::Modifiers::CTRL, egui::Modifiers::COMMAND] {
        let mut harness = field_dialog_harness(true);
        harness.key_combination_modifiers(modifiers, &[egui::Key::Enter]);
        harness.run();
        assert_eq!(harness.state().0, Some(DialogAction::Primary));
    }
}

#[test]
fn dialog_enter_in_single_line_field_confirms() {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (None, String::from("x")),
        |ui, (action, text)| {
            let shown = Dialog::new("Note")
                .content(|ui| {
                    ui.add(egui::TextEdit::singleline(text));
                })
                .primary("Save")
                .show(ui.ctx());
            if let Some(done) = shown {
                action.get_or_insert(done);
            }
        },
    );
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run();
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().0, Some(DialogAction::Primary));
}

#[test]
fn dialog_enter_on_a_focused_button_keeps_working() {
    let mut harness = field_dialog_harness(true);
    harness.get_by_label("Cancel").focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().0, Some(DialogAction::Secondary));
}

fn list_dialog(ui: &mut egui::Ui, lines: usize) {
    let _ = Dialog::new("Select files")
        .content(|ui| {
            for i in 0..lines {
                ui.label(format!("file-{i}.rs"));
            }
        })
        .primary("Open")
        .secondary("Cancel")
        .show(ui.ctx());
}

#[test]
fn dialog_is_as_tall_as_its_content() {
    let height = |lines: usize| {
        let mut harness = testing::harness(Theme::dark(), move |ui| list_dialog(ui, lines));
        harness.set_size(egui::vec2(520.0, 600.0));
        harness.run_steps(3);
        harness.get_by_label("Open").rect().bottom()
            - harness.get_by_label("Select files").rect().top()
    };
    let (one, three) = (height(1), height(3));
    assert!(three > one + 20.0, "{one} {three}");
    assert!(
        three < 200.0,
        "a short dialog shrinks to its content: {three}"
    );
}

#[test]
fn dialog_scrolls_beyond_its_max_height() {
    let mut harness = testing::harness(Theme::dark(), |ui| list_dialog(ui, 60));
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run_steps(3);
    let buttons = harness.get_by_label("Open").rect();
    let title = harness.get_by_label("Select files").rect();
    assert!(
        buttons.bottom() <= 300.0,
        "buttons stay on screen: {buttons:?}"
    );
    assert!(title.top() >= 0.0, "title stays on screen: {title:?}");
}

#[test]
fn dialog_scroll_snapshot() {
    testing::snapshot_themes_sized(
        "dialog_scroll",
        common::IMG,
        egui::vec2(520.0, 300.0),
        |ui| {
            ui.label("Editor content behind the dialog");
            list_dialog(ui, 60);
        },
    );
}

#[test]
fn dialog_short_content_snapshot() {
    testing::snapshot_themes_sized(
        "dialog_short",
        common::IMG,
        egui::vec2(520.0, 400.0),
        |ui| {
            ui.label("Editor content behind the dialog");
            list_dialog(ui, 2);
        },
    );
}

#[test]
fn dialog_tertiary_button_reports_and_sits_left() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, action| {
        if let Some(done) = tertiary_dialog(ui, 2) {
            action.get_or_insert(done);
        }
    });
    harness.set_size(egui::vec2(520.0, 400.0));
    harness.run_steps(3);
    let reset = harness.get_by_label("Reset").rect();
    let cancel = harness.get_by_label("Cancel").rect();
    assert!(reset.right() < cancel.left(), "{reset:?} {cancel:?}");
    harness.get_by_label("Reset").click();
    harness.run_steps(3);
    assert_eq!(*harness.state(), Some(DialogAction::Tertiary));
}

#[test]
fn dialog_tertiary_stays_reachable_when_content_scrolls() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = tertiary_dialog(ui, 60);
    });
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run_steps(3);
    let reset = harness.get_by_label("Reset").rect();
    assert!(reset.bottom() <= 300.0 && reset.top() >= 0.0, "{reset:?}");
}

fn tertiary_dialog(ui: &mut egui::Ui, lines: usize) -> Option<DialogAction> {
    Dialog::new("Select files")
        .content(|ui| {
            for i in 0..lines {
                ui.label(format!("file-{i}.rs"));
            }
        })
        .tertiary("Reset")
        .primary("Open")
        .secondary("Cancel")
        .show(ui.ctx())
}

#[test]
fn dialog_tertiary_snapshot() {
    testing::snapshot_themes_sized(
        "dialog_tertiary",
        common::IMG,
        egui::vec2(520.0, 300.0),
        |ui| {
            ui.label("Editor content behind the dialog");
            let _ = tertiary_dialog(ui, 60);
        },
    );
}
