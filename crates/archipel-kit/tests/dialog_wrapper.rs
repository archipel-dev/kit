//! `DialogWrapper` behavior from IntelliJ spec section 7: exit codes, keys, initial focus,
//! validation and the message helpers.

mod common;

use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{
    Theme,
    openapi::ui::{
        CANCEL_EXIT_CODE, CLOSE_EXIT_CODE, DialogValidation, DialogWrapper, ExitCode, Messages,
        NEXT_USER_EXIT_CODE, OK_EXIT_CODE, ValidationInfo,
    },
    ui::JBTextField,
};
use egui::Key;

#[test]
fn exit_codes_are_intellij_s() {
    assert_eq!(
        (
            OK_EXIT_CODE,
            CANCEL_EXIT_CODE,
            CLOSE_EXIT_CODE,
            NEXT_USER_EXIT_CODE
        ),
        (0, 1, 1, 2)
    );
    assert_eq!(ExitCode::Primary.code(), OK_EXIT_CODE);
    assert_eq!(ExitCode::Secondary.code(), CANCEL_EXIT_CODE);
    assert_eq!(ExitCode::Dismissed.code(), CLOSE_EXIT_CODE);
    assert_eq!(ExitCode::Tertiary.code(), NEXT_USER_EXIT_CODE);
    assert!(ExitCode::Primary.is_ok());
    assert!(!ExitCode::Dismissed.is_ok());
}

/// A dialog with three buttons, recording the first answer.
fn three_buttons() -> testing::Harness<'static, Option<ExitCode>> {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, answer| {
        let shown = DialogWrapper::new("Select files")
            .tertiary("Reset")
            .secondary("Cancel")
            .primary("Open")
            .show(ui.ctx());
        if let Some(done) = shown {
            answer.get_or_insert(done);
        }
    });
    harness.set_size(egui::vec2(520.0, 240.0));
    harness.run();
    harness
}

fn focused(harness: &testing::Harness<'_, Option<ExitCode>>, label: &str) -> bool {
    harness.get_by_label(label).is_focused()
}

#[test]
fn right_and_left_move_focus_between_the_buttons() {
    let mut harness = three_buttons();
    harness.get_by_label("Reset").focus();
    harness.run();
    assert!(focused(&harness, "Reset"));
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(focused(&harness, "Cancel"), "Reset, then Right: Cancel");
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(focused(&harness, "Open"));
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(focused(&harness, "Open"), "Right of the last button stays");
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(focused(&harness, "Cancel"));
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(focused(&harness, "Reset"));
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(focused(&harness, "Reset"), "Left of the first button stays");
    assert_eq!(*harness.state(), None, "moving focus answers nothing");
}

#[test]
fn enter_presses_the_focused_button_not_the_default_one() {
    let mut harness = three_buttons();
    harness.get_by_label("Reset").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), Some(ExitCode::Tertiary));
}

/// A yes/no dialog, with a text field next to the message when asked.
fn yes_no(
    with_field: bool,
    yes_no_keys: bool,
) -> testing::Harness<'static, (Option<ExitCode>, String)> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (None, String::new()),
        move |ui, (answer, text)| {
            let mut dialog = DialogWrapper::new("Delete branch?")
                .message("It has unmerged commits.")
                .primary("Yes")
                .secondary("No")
                .yes_no_keys(yes_no_keys);
            if with_field {
                dialog = dialog.content(|ui| {
                    ui.add(JBTextField::new(text).label("Note"));
                });
            }
            if let Some(done) = dialog.show(ui.ctx()) {
                answer.get_or_insert(done);
            }
        },
    );
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run();
    harness
}

#[test]
fn y_answers_yes_and_n_answers_no_in_a_yes_no_dialog() {
    let mut harness = yes_no(false, true);
    harness.key_press(Key::Y);
    harness.run();
    assert_eq!(harness.state().0, Some(ExitCode::Primary));

    let mut harness = yes_no(false, true);
    harness.key_press(Key::N);
    harness.run();
    assert_eq!(harness.state().0, Some(ExitCode::Secondary));
}

#[test]
fn y_and_n_do_nothing_without_the_yes_no_keys() {
    let mut harness = yes_no(false, false);
    harness.key_press(Key::Y);
    harness.key_press(Key::N);
    harness.run();
    assert_eq!(harness.state().0, None);
}

#[test]
fn y_and_n_are_text_while_a_field_has_the_focus() {
    let mut harness = yes_no(true, true);
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("yn");
    harness.run();
    assert_eq!(harness.state().0, None, "the keys went to the field");
    assert_eq!(harness.state().1, "yn");
}

#[test]
fn messages_yes_no_dialog_answers_with_y_and_n() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, answer| {
        if let Some(done) = Messages::yes_no_dialog("Delete?", "Really?").show(ui.ctx()) {
            answer.get_or_insert(done);
        }
    });
    harness.set_size(egui::vec2(520.0, 240.0));
    harness.run();
    assert!(harness.query_by_label("Yes").is_some());
    assert!(harness.query_by_label("No").is_some());
    harness.key_press(Key::N);
    harness.run();
    assert_eq!(*harness.state(), Some(ExitCode::Secondary));
}

#[test]
fn messages_info_error_and_ok_cancel_have_their_buttons() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = Messages::error_dialog("Failed", "Could not save.").show(ui.ctx());
    });
    harness.set_size(egui::vec2(520.0, 240.0));
    harness.run();
    assert!(harness.query_by_label("OK").is_some());
    assert!(harness.query_by_label("Cancel").is_none());

    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = Messages::ok_cancel_dialog("Proceed?", "This cannot be undone.").show(ui.ctx());
    });
    harness.set_size(egui::vec2(520.0, 240.0));
    harness.run();
    assert!(harness.query_by_label("OK").is_some());
    assert!(harness.query_by_label("Cancel").is_some());

    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = Messages::info_dialog("Done", "Saved.").show(ui.ctx());
    });
    harness.set_size(egui::vec2(520.0, 240.0));
    harness.run();
    assert!(harness.query_by_label("OK").is_some());
}

#[test]
fn buttons_are_12_apart() {
    let harness = three_buttons();
    let cancel = harness.get_by_label("Cancel").rect();
    let open = harness.get_by_label("Open").rect();
    assert!(
        (open.left() - cancel.right() - 12.0).abs() < 0.5,
        "{cancel:?} {open:?}"
    );
}

/// Two fields; `preferred` names the second one.
fn two_fields(preferred: bool) -> testing::Harness<'static, (String, String)> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (String::new(), String::new()),
        move |ui, (first, second)| {
            let mut dialog = DialogWrapper::new("New Project")
                .content(|ui| {
                    ui.add(JBTextField::new(first).label("Name"));
                    ui.add(
                        JBTextField::new(second)
                            .label("Location")
                            .id(egui::Id::new("location")),
                    );
                })
                .primary("Create")
                .secondary("Cancel");
            if preferred {
                dialog = dialog.preferred_focus(egui::Id::new("location"));
            }
            let _ = dialog.show(ui.ctx());
        },
    );
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run_steps(4);
    harness
}

#[test]
fn the_first_field_of_the_content_has_the_initial_focus() {
    let harness = two_fields(false);
    assert!(harness.get_by_label("Name").is_focused());
    assert!(!harness.get_by_label("Create").is_focused());
}

#[test]
fn the_preferred_component_has_the_initial_focus() {
    let harness = two_fields(true);
    assert!(harness.get_by_label("Location").is_focused());
}

/// A dialog that validates a name; the state keeps what the validation found.
struct Form {
    name: String,
    validation: DialogValidation,
    runs: u32,
    answer: Option<ExitCode>,
}

fn validated(name: &str) -> testing::Harness<'static, Form> {
    let state = Form {
        name: name.into(),
        validation: DialogValidation::new(),
        runs: 0,
        answer: None,
    };
    let mut harness = testing::harness_state(Theme::dark(), state, |ui, form| {
        let Form {
            name,
            validation,
            runs,
            answer,
        } = form;
        validation.update(ui.ctx(), &*name, || {
            *runs += 1;
            let mut infos = Vec::new();
            if name.trim().is_empty() {
                infos.push(ValidationInfo::error("Enter a name"));
            } else if name.contains(' ') {
                infos.push(ValidationInfo::warning("Spaces are unusual"));
            }
            infos
        });
        if let Some(done) = DialogWrapper::new("New Branch")
            .primary("Create")
            .secondary("Cancel")
            .validation(validation)
            .show(ui.ctx())
        {
            answer.get_or_insert(done);
        }
    });
    harness.set_size(egui::vec2(520.0, 300.0));
    harness.run_steps(3);
    harness
}

#[test]
fn validation_runs_at_once_for_the_initial_input() {
    let harness = validated("");
    assert_eq!(harness.state().runs, 1);
    assert!(harness.query_by_label("Enter a name").is_some());
}

#[test]
fn validation_waits_300_ms_after_the_last_change() {
    let mut harness = validated("");
    harness.state_mut().name = "feature".into();
    harness.step();
    common::advance_by(&mut harness, 0.25);
    assert!(
        harness.query_by_label("Enter a name").is_some(),
        "still the old result after 250 ms"
    );
    assert_eq!(harness.state().runs, 1);
    common::advance_by(&mut harness, 0.1);
    assert!(
        harness.query_by_label("Enter a name").is_none(),
        "validated after 350 ms"
    );
    assert_eq!(harness.state().runs, 2);
}

#[test]
fn every_change_restarts_the_validation_delay() {
    let mut harness = validated("a");
    for text in ["ab", "abc", "abcd"] {
        harness.state_mut().name = text.into();
        harness.step();
        common::advance_by(&mut harness, 0.2);
    }
    assert_eq!(
        harness.state().runs,
        1,
        "typing faster than 300 ms never validates"
    );
    common::advance_by(&mut harness, 0.2);
    assert_eq!(harness.state().runs, 2);
}

#[test]
fn a_blocking_error_disables_ok_and_enter() {
    let mut harness = validated("");
    assert!(harness.get_by_label("Create").is_disabled());
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(harness.state().answer, None);
}

#[test]
fn a_warning_does_not_block_ok() {
    let mut harness = validated("my branch");
    assert!(harness.query_by_label("Spaces are unusual").is_some());
    assert!(!harness.get_by_label("Create").is_disabled());
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(harness.state().answer, Some(ExitCode::Primary));
}

#[test]
fn an_error_marked_ok_enabled_does_not_block() {
    let mut validation = DialogValidation::new();
    assert!(validation.validate_now(vec![ValidationInfo::error("Too long").with_ok_enabled()]));
    assert!(!validation.blocks_ok());
    assert!(!validation.validate_now(vec![ValidationInfo::error("Empty")]));
}
