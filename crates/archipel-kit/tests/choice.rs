//! `choice` component tests: behavior, accessibility and dark/light snapshots.

mod common;

use archipel_kit::Theme;
use archipel_kit::components::{Checkbox, Radio, Toggle};
use archipel_kit::testing::{self, NodeExt, Queryable};

#[test]
fn snapshot() {
    testing::snapshot_themes("choice", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(Checkbox::new(&mut true, "Checked"));
                ui.add(Checkbox::new(&mut false, "Unchecked"));
                ui.add(Checkbox::new(&mut false, "Mixed").mixed(true));
                ui.add(Checkbox::new(&mut true, "Disabled").enabled(false));
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                let mut indent = 1;
                ui.add(Radio::new(&mut indent, 0, "Tabs"));
                ui.add(Radio::new(&mut indent, 1, "Spaces"));
                ui.add(Radio::new(&mut indent, 2, "Smart").enabled(false));
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.add(Toggle::new(&mut true, "On"));
                ui.add(Toggle::new(&mut false, "Off"));
                ui.add(Toggle::new(&mut true, "Disabled").enabled(false));
            });
        });
    });
}

#[test]
fn checkbox_toggles_from_box_and_label() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(Checkbox::new(checked, "Reformat on save"));
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
fn mixed_checkbox_becomes_checked() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(Checkbox::new(checked, "All files").mixed(true));
    });
    harness.get_by_label("All files").click();
    harness.run();
    assert!(*harness.state());
}

#[test]
fn space_toggles_a_focused_checkbox() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(Checkbox::new(checked, "Wrap"));
    });
    harness.get_by_label("Wrap").focus();
    harness.run();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(*harness.state());
}

#[test]
fn disabled_checkbox_stays() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(Checkbox::new(checked, "Locked").enabled(false));
    });
    assert!(harness.get_by_label("Locked").is_disabled());
    harness.get_by_label("Locked").click();
    harness.run();
    assert!(!*harness.state());
}

#[test]
fn radio_selects_its_value() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, value| {
        ui.add(Radio::new(value, 0, "Tabs"));
        ui.add(Radio::new(value, 1, "Spaces"));
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
        ui.add(Toggle::new(on, "Auto save"));
    });
    harness.get_by_label("Auto save").click();
    harness.run();
    assert!(*harness.state());
    assert!(harness.get_by_label("Auto save").is_checked());
}
