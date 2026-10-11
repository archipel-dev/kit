//! `Dropdown`, `MenuItem` and `Link` tests.

mod common;

use archipel_kit::components::{Dropdown, Link, MenuItem, MenuSeparator};
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme};

#[test]
fn menu_snapshot() {
    testing::snapshot_themes("menu_item", common::IMG, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(240.0);
            ui.add(MenuItem::new("Copy").icon(Glyph::Copy).shortcut("Ctrl+C"));
            ui.add(
                MenuItem::new("Find in Files…")
                    .icon(Glyph::FindInFiles)
                    .shortcut("Ctrl+Shift+F"),
            );
            ui.add(MenuSeparator);
            ui.add(MenuItem::new("Soft-Wrap").checked(true));
            ui.add(MenuItem::new("Run").icon(Glyph::Run).enabled(false));
        });
    });
}

#[test]
fn menu_item_clicks_and_reports_check_state() {
    let mut harness = testing::harness_state(Theme::dark(), (0, true), |ui, (copies, wrap)| {
        if ui.add(MenuItem::new("Copy").shortcut("Ctrl+C")).clicked() {
            *copies += 1;
        }
        if ui.add(MenuItem::new("Soft-Wrap").checked(*wrap)).clicked() {
            *wrap = !*wrap;
        }
    });
    harness.get_by_label("Copy").click();
    harness.run();
    assert_eq!(harness.state().0, 1);
    assert!(harness.get_by_label("Soft-Wrap").is_checked());
    harness.get_by_label("Soft-Wrap").click();
    harness.run();
    assert!(!harness.state().1);
}

#[test]
fn dropdown_snapshot() {
    testing::snapshot_themes("dropdown", common::IMG, |ui| {
        ui.add(Dropdown::new("method", &mut "GET", ["GET", "POST"]).width(120.0));
        ui.add(
            Dropdown::new("off", &mut "Disabled", ["Disabled"])
                .width(120.0)
                .enabled(false),
        );
    });
}

#[test]
fn dropdown_picks_an_option() {
    let mut harness = testing::harness_state(Theme::dark(), "GET", |ui, method| {
        ui.add(Dropdown::new("method", method, ["GET", "POST", "PUT"]));
    });
    // Make room for the popup below the button.
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 240.0));
    harness.run();
    let combo = harness.get_by_role(Role::ComboBox);
    assert_eq!(combo.accessible_label().as_deref(), Some("GET"));
    combo.click();
    harness.run();
    harness.get_by_label("PUT").click();
    harness.run();
    assert_eq!(*harness.state(), "PUT");
}

#[test]
fn link_snapshot() {
    testing::snapshot_themes("link", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.label("Gradle JVM is not set.");
            ui.add(Link::new("Configure…"));
        });
    });
}

#[test]
fn link_has_link_role() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, clicked| {
        *clicked |= ui.add(Link::new("Open docs")).clicked();
    });
    let node = harness.get_by_label("Open docs");
    assert_eq!(node.role(), Role::Link);
    node.click();
    harness.run();
    assert!(*harness.state());
}
