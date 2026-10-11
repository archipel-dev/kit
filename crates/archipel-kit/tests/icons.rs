//! File-type icons snapshot: the icons the kit draws itself (letter badges, Java types,
//! colored folders). The glyph set is reviewed in `archipel-icons` (`docs/icons/`).

mod common;

use archipel_kit::icons::{FolderKind, Symbol, SymbolKind, Visibility};
use archipel_kit::testing;
use archipel_kit::{Icon, Theme};

#[test]
fn file_icons_snapshot() {
    testing::snapshot_themes("file_icons", common::IMG, |ui| {
        let color = Theme::colors(ui.ctx()).icon;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for name in [
                "main.rs",
                "build.gradle.kts",
                "App.kt",
                "data.json",
                "pom.xml",
                "README.md",
                "app.py",
                "index.ts",
                "run.sh",
                "logo.png",
                "lib.zip",
                "Cargo.lock",
                "notes.txt",
            ] {
                archipel_kit::icons::show(ui, Icon::for_file(name), 16.0, color);
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for symbol in [
                Symbol::new(SymbolKind::Class),
                Symbol::new(SymbolKind::Interface).visibility(Visibility::Internal),
                Symbol::new(SymbolKind::Enum),
                Symbol::new(SymbolKind::Method).visibility(Visibility::Private),
                Symbol::new(SymbolKind::Field).static_(true),
            ] {
                archipel_kit::icons::show(ui, symbol, 16.0, color);
            }
            for kind in [
                FolderKind::Source,
                FolderKind::Test,
                FolderKind::Resources,
                FolderKind::Excluded,
                FolderKind::Hidden,
                FolderKind::Package,
            ] {
                archipel_kit::icons::show(ui, Icon::FolderKind(kind), 16.0, color);
            }
        });
    });
}
