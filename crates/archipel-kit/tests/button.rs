//! `button` component tests: behavior, accessibility and dark/light snapshots.

mod common;

use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme, Tone};
use archipel_kit::{action_system::ActionButton, ui::Button};

#[test]
fn snapshot() {
    testing::snapshot_themes("button", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(Button::primary("Create"));
            ui.add(Button::secondary("Cancel"));
            ui.add(Button::ghost("Skip"));
            ui.add(Button::danger("Delete"));
        });
        ui.horizontal(|ui| {
            ui.add(Button::primary("Create").enabled(false));
            ui.add(Button::danger("Delete").enabled(false));
            ui.add(Button::secondary("Cancel").enabled(false));
            ui.add(Button::secondary("Run").icon(Glyph::Run));
            ui.add(Button::ghost("Small").small(true));
        });
    });
}

#[test]
fn action_button_snapshot() {
    testing::snapshot_themes("action_button", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(ActionButton::new(Glyph::Refresh, "Reload"));
            ui.add(ActionButton::new(Glyph::Run, "Run").tone(Tone::Run));
            ui.add(ActionButton::new(Glyph::Stop, "Stop").tone(Tone::Stop));
            ui.add(ActionButton::new(Glyph::Pin, "Pin").selected(true));
            ui.add(ActionButton::new(Glyph::Debug, "Debug").enabled(false));
        });
    });
}

#[test]
fn click_fires_once() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui.add(Button::primary("Run")).clicked() {
            *clicks += 1;
        }
    });
    harness.get_by_label("Run").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn disabled_button_ignores_clicks_and_says_so() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui.add(Button::primary("Run").enabled(false)).clicked() {
            *clicks += 1;
        }
    });
    let node = harness.get_by_role_and_label(Role::Button, "Run");
    assert!(node.is_disabled());
    node.click();
    harness.run();
    assert_eq!(*harness.state(), 0);
}

#[test]
fn keyboard_activates_focused_button() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui.add(Button::secondary("Save")).clicked() {
            *clicks += 1;
        }
    });
    harness.get_by_label("Save").focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn action_button_is_named_by_its_label_and_reports_toggle_state() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, pinned| {
        if ui
            .add(ActionButton::new(Glyph::Pin, "Pin tab").selected(*pinned))
            .clicked()
        {
            *pinned = !*pinned;
        }
    });
    assert!(!harness.get_by_label("Pin tab").is_checked());
    harness.get_by_label("Pin tab").click();
    harness.run();
    assert!(*harness.state());
    assert!(harness.get_by_label("Pin tab").is_checked());
}
