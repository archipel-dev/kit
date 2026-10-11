//! Icon set snapshot: every parameterless icon, so a changed drawing shows up in review.

mod common;

use archipel_kit::Icon;
use archipel_kit::testing;

#[test]
fn icon_set_snapshot() {
    testing::snapshot_themes("icons", common::IMG, |ui| {
        let color = archipel_kit::Theme::colors(ui.ctx()).icon;
        for row in Icon::ALL.chunks(13) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (icon, _) in row {
                    archipel_kit::icons::show(ui, *icon, 16.0, color);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for name in [
                "main.rs",
                "App.java",
                "build.gradle.kts",
                "a.kt",
                "a.json",
                "pom.xml",
                "a.md",
                "a.py",
                "a.ts",
                "a.sh",
                "a.png",
                "a.zip",
                "Cargo.lock",
            ] {
                archipel_kit::icons::show(ui, Icon::for_file(name), 16.0, color);
            }
        });
    });
}
