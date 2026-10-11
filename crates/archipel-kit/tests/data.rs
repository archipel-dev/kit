//! `Counter`, `Tag`, `Kbd`, `Avatar`, `PropertyGrid`, `Panel` and `SectionHeader` tests.

mod common;

use archipel_kit::components::{
    Avatar, Counter, IconButton, Kbd, Panel, PropertyGrid, SectionHeader, Separator, Tag,
};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Glyph, Theme, Tone, ValueKind, tokens};

#[test]
fn badges_snapshot() {
    testing::snapshot_themes("badges", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(Counter::new(3));
            ui.add(Counter::new(12).accent(true));
            ui.add(Counter::new(1234));
            ui.add(Separator);
            ui.add(Tag::new("beta"));
            ui.add(Tag::new("GET").tone(Tone::Success));
            ui.add(Tag::new("DELETE").tone(Tone::Error));
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
                ui.add(IconButton::new(Glyph::More, "More"));
                ui.add(IconButton::new(Glyph::Copy, "Copy"));
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
                *copied |= ui.add(IconButton::new(Glyph::Copy, "Copy")).clicked();
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

#[test]
fn data_colors_snapshot() {
    testing::snapshot_themes("data-colors", common::IMG, |ui| {
        ui.set_width(360.0);
        ui.add(SectionHeader::new("Data colors"));
        let theme = Theme::current(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            for kind in ValueKind::ALL {
                ui.label(
                    egui::RichText::new(kind.label())
                        .monospace()
                        .color(kind.color(&theme)),
                );
            }
        });
        ui.horizontal(|ui| {
            for index in 0..8 {
                swatch(ui, theme.chart(index));
            }
        });
        ui.horizontal(|ui| {
            for step in 0..=10 {
                swatch(ui, theme.colors.ramp(step as f32 / 10.0));
            }
        });
        ui.horizontal(|ui| {
            for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
                ui.add(Tag::new(method).tone(Tone::method(method)));
            }
            for code in [200, 301, 404, 503] {
                ui.add(Tag::new(code.to_string()).tone(Tone::status(code)));
            }
        });
    });
}

fn swatch(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, tokens::radius::CONTROL, color);
}

#[test]
fn value_kinds_follow_the_theme_of_the_context() {
    let dark = testing::harness(Theme::dark(), |_| {});
    let light = testing::harness(Theme::light(), |_| {});
    let kind = ValueKind::String;
    assert_eq!(kind.resolve(&dark.ctx), Theme::dark().editor.string);
    assert_eq!(kind.resolve(&light.ctx), Theme::light().editor.string);
}

#[test]
fn tool_window_header_snapshot() {
    testing::snapshot_themes("panel_tool_window", common::IMG, |ui| {
        Panel::new("Project")
            .actions(|ui| {
                ui.add(IconButton::new(Glyph::CollapseAll, "Collapse all"));
                ui.add(IconButton::new(Glyph::Search, "Find"));
            })
            .on_hide(|| {})
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.label("src/");
            });
    });
}

#[test]
fn panel_hide_button_hides_and_passes_the_audit() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, hidden| {
        Panel::new("Project")
            .actions(|ui| {
                ui.add(IconButton::new(Glyph::CollapseAll, "Collapse all"));
            })
            .on_hide(|| *hidden += 1)
            .show(ui, |ui| {
                ui.label("src/");
            });
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Collapse all").click();
    harness.run();
    assert_eq!(*harness.state(), 0);
    harness.get_by_label("Hide Project").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn panel_without_on_hide_has_no_hide_button() {
    let harness = testing::harness(Theme::dark(), |ui| {
        Panel::new("Project").show(ui, |ui| {
            ui.label("src/");
        });
    });
    assert!(harness.query_by_label("Hide Project").is_none());
}
