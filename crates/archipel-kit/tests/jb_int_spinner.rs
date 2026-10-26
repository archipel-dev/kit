//! `JBIntSpinner`: integers in range, written only when valid.

mod common;

use archipel_kit::Theme;
use archipel_kit::egui::{self, Key};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::ui::{FieldLabel, Form, JBIntSpinner};

fn harness(value: u64) -> testing::Harness<'static, u64> {
    testing::harness_state(Theme::dark(), value, |ui, limit| {
        ui.add(
            JBIntSpinner::new(limit)
                .name("Limit")
                .range(1..=500)
                .step(10),
        );
    })
}

fn select_all(h: &mut testing::Harness<'static, u64>) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.run();
}

#[test]
fn a_valid_number_is_written_at_once() {
    let mut h = harness(50);
    h.get_by_label("Limit").focus();
    h.run();
    select_all(&mut h);
    h.get_by_label("Limit").type_text("120");
    h.run();
    assert_eq!(*h.state(), 120);
}

#[test]
fn invalid_text_keeps_the_value_shows_an_error_and_reverts_on_leaving() {
    let mut h = harness(50);
    h.get_by_label("Limit").focus();
    h.run();
    select_all(&mut h);
    h.get_by_label("Limit").type_text("9000");
    h.run();
    assert_eq!(*h.state(), 50, "out of range: not written");
    assert!(
        h.query_by_label_contains("Enter a number from 1 to 500")
            .is_some()
    );
    h.get_by_label("Limit").type_text("x");
    h.run();
    assert_eq!(*h.state(), 50);
    h.key_press(Key::Tab);
    h.run();
    h.run();
    assert_eq!(
        h.get_by_label("Limit").value().as_deref(),
        Some("50"),
        "shows the value again"
    );
    assert!(h.query_by_label_contains("Enter a number").is_none());
}

#[test]
fn arrows_step_within_the_range() {
    let mut h = harness(495);
    h.get_by_label("Limit").focus();
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(*h.state(), 500, "clamped to the maximum");
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(*h.state(), 490);
}

#[test]
fn changed_reports_the_value_not_the_text() {
    let mut h = testing::harness_state(Theme::dark(), (7_i32, 0_u32), |ui, (value, changes)| {
        if ui.add(JBIntSpinner::new(value).name("Count")).changed() {
            *changes += 1;
        }
    });
    h.get_by_label("Count").focus();
    h.run();
    h.get_by_label("Count").type_text("-");
    h.run();
    assert_eq!(*h.state(), (7, 0), "7- is no number");
    h.key_press(Key::Backspace);
    h.run();
    assert_eq!(h.state().1, 0, "back to 7: no change");
    h.get_by_label("Count").type_text("5");
    h.run();
    assert_eq!(*h.state(), (75, 1));
}

#[test]
fn snapshot() {
    testing::snapshot_themes_after(
        "jb_int_spinner",
        common::IMG,
        |ui| {
            ui.set_width(320.0);
            let id = egui::Id::new("number-snapshot");
            let (mut size, mut timeout) =
                ui.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || (50_u32, 30_u32)));
            Form::new().show(ui, |form| {
                form.fields(|ui| {
                    ui.add(FieldLabel::new("Page size:"));
                    ui.add(
                        JBIntSpinner::new(&mut size)
                            .name("Page size")
                            .range(1..=1000),
                    );
                });
                form.fields(|ui| {
                    ui.add(FieldLabel::new("Timeout (s):"));
                    ui.add(
                        JBIntSpinner::new(&mut timeout)
                            .name("Timeout")
                            .range(1..=600),
                    );
                });
            });
            ui.data_mut(|d| d.insert_temp(id, (size, timeout)));
            ui.add_space(24.0);
        },
        |h| {
            h.get_by_label("Timeout").focus();
            h.run();
            h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
            h.run();
            h.get_by_label("Timeout").type_text("900");
            h.run();
        },
    );
}
