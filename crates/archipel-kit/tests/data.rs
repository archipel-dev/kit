//! `Counter`, `Tag`, `Kbd`, `Avatar`, `PropertyGrid`, `Panel` and `SectionHeader` tests.

mod common;

use archipel_kit::components::{
    Avatar, Counter, IconButton, Kbd, Panel, PropertyGrid, SectionHeader, Separator, Tag,
};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Icon, Theme};

#[test]
fn badges_snapshot() {
    testing::snapshot_themes("badges", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(Counter::new(3));
            ui.add(Counter::new(12).accent(true));
            ui.add(Counter::new(1234));
            ui.add(Separator);
            ui.add(Tag::new("beta"));
            ui.add(Tag::new("GET").color(Theme::colors(ui.ctx()).success));
            ui.add(Tag::new("DELETE").color(Theme::colors(ui.ctx()).error));
        });
        ui.horizontal(|ui| {
            ui.add(Kbd::parse("Ctrl+Shift+F"));
            ui.add(Kbd::new(["⌘", "K"]));
            ui.add(Separator);
            ui.add(Avatar::new("archipel-ide"));
            ui.add(Avatar::new("nexus-http"));
            ui.add(Avatar::new("Ada Lovelace").size(24.0).round(true));
        });
    });
}

#[test]
fn badges_have_readable_names() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(Counter::new(250));
        ui.add(Kbd::parse("Ctrl+K"));
        ui.add(Avatar::new("archipel"));
    });
    harness.get_by_label("99+");
    harness.get_by_label("Ctrl+K");
    harness.get_by_label("archipel");
}

#[test]
fn panel_snapshot() {
    testing::snapshot_themes("panel", common::IMG, |ui| {
        Panel::new("Response")
            .actions(|ui| {
                ui.add(IconButton::new(Icon::More, "More"));
                ui.add(IconButton::new(Icon::Copy, "Copy"));
            })
            .show(ui, |ui| {
                ui.add_space(4.0);
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(12, 0))
                    .show(ui, |ui| {
                        ui.add(SectionHeader::new("General"));
                        PropertyGrid::new("response")
                            .row("Status", "200 OK")
                            .row("Time", "142 ms")
                            .row("Size", "2.4 KB")
                            .mono_row("Content-Type", "application/json")
                            .show(ui);
                    });
            });
    });
}

#[test]
fn panel_title_and_actions_are_accessible() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, copied| {
        Panel::new("Response")
            .actions(|ui| {
                *copied |= ui.add(IconButton::new(Icon::Copy, "Copy")).clicked();
            })
            .show(ui, |ui| {
                PropertyGrid::new("g").row("Status", "200 OK").show(ui);
            });
    });
    harness.get_by_label("Status");
    harness.get_by_label("200 OK");
    harness.get_by_label("Copy").click();
    harness.run();
    assert!(*harness.state());
}
