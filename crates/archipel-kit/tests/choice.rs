//! `choice` component tests: behavior, accessibility and dark/light snapshots.

mod common;

use archipel_kit::Theme;
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::ui::{JBCheckBox, JBRadioButton, OnOffButton};

#[test]
fn snapshot() {
    testing::snapshot_themes("choice", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(JBCheckBox::new(&mut true, "Checked"));
                ui.add(JBCheckBox::new(&mut false, "Unchecked"));
                ui.add(JBCheckBox::new(&mut false, "Mixed").mixed(true));
                ui.add(JBCheckBox::new(&mut true, "Disabled").enabled(false));
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                let mut indent = 1;
                ui.add(JBRadioButton::new(&mut indent, 0, "Tabs"));
                ui.add(JBRadioButton::new(&mut indent, 1, "Spaces"));
                ui.add(JBRadioButton::new(&mut indent, 2, "Smart").enabled(false));
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.add(OnOffButton::new(&mut true, "On"));
                ui.add(OnOffButton::new(&mut false, "Off"));
                ui.add(OnOffButton::new(&mut true, "Disabled").enabled(false));
            });
        });
    });
}

#[test]
fn check_box_toggles_from_box_and_label() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(JBCheckBox::new(checked, "Reformat on save"));
    });
    let node = harness.get_by_label("Reformat on save");
    assert!(!node.is_checked());
    node.click();
    harness.run();
    assert!(*harness.state());
    assert!(harness.get_by_label("Reformat on save").is_checked());
    harness.get_by_label("Reformat on save").click();
    harness.run();
    assert!(!*harness.state());
}

#[test]
fn mixed_check_box_becomes_checked() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(JBCheckBox::new(checked, "All files").mixed(true));
    });
    harness.get_by_label("All files").click();
    harness.run();
    assert!(*harness.state());
}

#[test]
fn space_toggles_a_focused_check_box() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(JBCheckBox::new(checked, "Wrap"));
    });
    harness.get_by_label("Wrap").focus();
    harness.run();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(*harness.state());
}

#[test]
fn disabled_check_box_stays() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(JBCheckBox::new(checked, "Locked").enabled(false));
    });
    assert!(harness.get_by_label("Locked").is_disabled());
    harness.get_by_label("Locked").click();
    harness.run();
    assert!(!*harness.state());
}

#[test]
fn radio_selects_its_value() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, value| {
        ui.add(JBRadioButton::new(value, 0, "Tabs"));
        ui.add(JBRadioButton::new(value, 1, "Spaces"));
    });
    assert!(harness.get_by_label("Tabs").is_checked());
    harness.get_by_label("Spaces").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
    assert!(harness.get_by_label("Spaces").is_checked());
    assert!(!harness.get_by_label("Tabs").is_checked());
}

#[test]
fn toggle_flips() {
    let mut harness = testing::harness_state(Theme::light(), false, |ui, on| {
        ui.add(OnOffButton::new(on, "Auto save"));
    });
    harness.get_by_label("Auto save").click();
    harness.run();
    assert!(*harness.state());
    assert!(harness.get_by_label("Auto save").is_checked());
}
