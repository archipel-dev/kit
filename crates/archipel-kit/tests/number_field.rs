//! `NumberField` tests: stepping, clamping, invalid input, accessibility, snapshots.

mod common;

use archipel_kit::Theme;
use archipel_kit::components::NumberField;
use archipel_kit::egui::{Key, Modifiers};
use archipel_kit::testing::{self, NodeExt, Queryable, Role};

fn harness(value: i32, min: i32, max: i32) -> testing::Harness<'static, (i32, bool)> {
    let mut harness = testing::harness_state(Theme::dark(), (value, false), move |ui, state| {
        let response = ui.add(
            NumberField::new(&mut state.0)
                .label("Iterations")
                .range(min..=max)
                .step(2),
        );
        state.1 |= response.changed();
    });
    harness.get_by_label("Iterations").focus();
    harness.run();
    harness
}

#[test]
fn snapshot() {
    let (mut a, mut b, mut c, mut port) = (10_u32, 0.5_f64, 7_i32, 8080_u32);
    testing::snapshot_themes_after(
        "number_field",
        common::IMG,
        |ui| {
            ui.add(NumberField::new(&mut a).label("Iterations").range(1..=1000));
            ui.add(
                NumberField::new(&mut b)
                    .label("Delay (s)")
                    .min(0.0)
                    .step(0.1),
            );
            ui.add(NumberField::new(&mut c).label("Locked").enabled(false));
            ui.add(NumberField::new(&mut port).label("Port").range(1..=65535));
        },
        |harness| {
            // Invalid input: error outline and message, the value stays 8080.
            let port = harness.get_by_label("Port");
            port.focus();
            port.type_text("x");
            harness.run();
        },
    );
}

#[test]
fn arrows_step_and_shift_steps_by_ten() {
    let mut harness = harness(10, 0, 1000);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().0, 12);
    assert!(harness.state().1, "reports changed");
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().0, 8);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().0, 28);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().0, 8);
    assert_eq!(
        harness.get_by_label("Iterations").value().as_deref(),
        Some("8")
    );
}

#[test]
fn stepping_clamps_to_the_range() {
    let mut harness = harness(3, 0, 5);
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().0, 5);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().0, 0);
}

#[test]
fn typing_a_valid_number_updates_the_value() {
    let mut harness = harness(10, 0, 100);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_label("Iterations").type_text("42");
    harness.run();
    assert_eq!(harness.state().0, 42);
    assert!(harness.state().1);
}

#[test]
fn invalid_input_shows_the_error_and_keeps_the_last_valid_value() {
    let mut harness = harness(10, 0, 100);
    harness.get_by_label("Iterations").type_text("x");
    harness.run();
    assert_eq!(harness.state().0, 10, "value untouched");
    assert!(
        harness.query_by_label_contains("Enter a number").is_some(),
        "error message shown"
    );
    // Out of range is invalid too.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_label("Iterations").type_text("500");
    harness.run();
    assert_eq!(harness.state().0, 10);
    assert!(harness.query_by_label_contains("from 0 to 100").is_some());
    // Leaving the field restores the last valid value.
    harness.key_press(Key::Tab);
    harness.run();
    harness.run();
    assert!(harness.query_by_label_contains("Enter a number").is_none());
    assert_eq!(harness.state().0, 10);
    assert_eq!(
        harness.get_by_label("Iterations").value().as_deref(),
        Some("10")
    );
}

#[test]
fn stepping_a_float_does_not_drift() {
    let mut harness = testing::harness_state(Theme::dark(), 0.1_f64, |ui, v| {
        ui.add(NumberField::new(v).label("Delay").step(0.1));
    });
    harness.get_by_label("Delay").focus();
    harness.run();
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(*harness.state(), 0.3);
}

#[test]
fn has_the_text_input_role() {
    let harness = harness(1, 0, 3);
    assert_eq!(harness.get_by_label("Iterations").role(), Role::TextInput);
}

#[test]
fn audit_is_clean_valid_and_invalid() {
    let mut harness = harness(1, 0, 3);
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Iterations").type_text("x");
    harness.run();
    testing::audit(&mut harness).assert_clean();
}
