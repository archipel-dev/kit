//! Language icons: labels, colors, file matching, and the catalog images.

use archipel_icons::{Language, contrast};
use egui::{Color32, Rect, pos2, vec2};
use egui_kittest::{Harness, SnapshotOptions, SnapshotResults};
use std::collections::HashSet;

#[test]
fn labels_are_unique_and_short() {
    let mut seen = HashSet::new();
    for language in Language::ALL {
        let label = language.label();
        assert!(seen.insert(label), "{label} is used twice");
        assert!(
            (1..=3).contains(&label.chars().count()),
            "{label}: one to three characters"
        );
        assert!(label.is_ascii(), "{label}: ASCII, so every font renders it");
    }
}

/// Labels are text: WCAG 2.1 asks for 4.5:1 against the tile.
#[test]
fn labels_contrast_with_their_tile() {
    for language in Language::ALL {
        let label = if language.dark_label() {
            0x000000
        } else {
            0xFFFFFF
        };
        let ratio = contrast(label, language.color());
        assert!(
            ratio >= 4.5,
            "{}: label contrast {ratio:.2}:1",
            language.name()
        );
    }
}

#[test]
fn files_map_to_languages() {
    for (file, expected) in [
        ("main.rs", Some(Language::Rust)),
        ("App.java", Some(Language::Java)),
        ("Main.kt", Some(Language::Kotlin)),
        ("build.gradle.kts", Some(Language::GradleKotlin)),
        ("build.gradle", Some(Language::Gradle)),
        ("script.py", Some(Language::Python)),
        ("index.ts", Some(Language::TypeScript)),
        ("App.tsx", Some(Language::Tsx)),
        ("app.js", Some(Language::JavaScript)),
        ("main.go", Some(Language::Go)),
        ("lib.c", Some(Language::C)),
        ("lib.cpp", Some(Language::Cpp)),
        ("Program.cs", Some(Language::CSharp)),
        ("view.swift", Some(Language::Swift)),
        ("Dockerfile", Some(Language::Dockerfile)),
        ("Makefile", Some(Language::Makefile)),
        ("CMakeLists.txt", Some(Language::CMake)),
        ("data.json", Some(Language::Json)),
        ("pom.xml", Some(Language::Xml)),
        ("README.md", Some(Language::Markdown)),
        ("Cargo.toml", Some(Language::Toml)),
        ("ci.yml", Some(Language::Yaml)),
        (".gitignore", Some(Language::IgnoreList)),
        ("src/deep/path/mod.RS", Some(Language::Rust)),
        ("notes", None),
        ("rs", None),
    ] {
        assert_eq!(Language::from_file_name(file), expected, "{file}");
    }
}

#[test]
fn extensions_are_claimed_once() {
    let mut seen = HashSet::new();
    for language in Language::ALL {
        for ext in language.extensions() {
            assert!(seen.insert(*ext), "{ext} is claimed twice");
        }
    }
}

#[test]
fn slugs_are_unique_kebab_case() {
    let mut seen = HashSet::new();
    for language in Language::ALL {
        let slug = language.slug();
        assert!(
            slug.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{slug}"
        );
        assert!(seen.insert(slug.clone()), "{slug} twice");
    }
}

/// (name, background, caption text, icon ink): the Archipel Kit island colors.
const LOOKS: [(&str, Color32, Color32, Color32); 2] = [
    (
        "dark",
        Color32::from_rgb(0x19, 0x1A, 0x1C),
        Color32::from_rgb(0x9F, 0xA2, 0xA8),
        Color32::from_rgb(0xCE, 0xD0, 0xD6),
    ),
    (
        "light",
        Color32::from_rgb(0xFF, 0xFF, 0xFF),
        Color32::from_rgb(0x5F, 0x62, 0x69),
        Color32::from_rgb(0x73, 0x76, 0x7C),
    ),
];

/// Registers the bold label font the way Archipel Kit does (family `semibold`), so the
/// sheets show the labels users see.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "inter-semibold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../archipel-kit/assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.families.insert(
        egui::FontFamily::Name("semibold".into()),
        vec!["inter-semibold".into()],
    );
    ctx.set_fonts(fonts);
}

fn sheet<'a>(
    size: egui::Vec2,
    scale: f32,
    background: Color32,
    mut draw: impl FnMut(&egui::Painter) + 'a,
) -> Harness<'a> {
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(scale)
        .wgpu()
        .build(move |ctx| {
            if !installed {
                installed = true;
                install_fonts(ctx);
                ctx.request_repaint();
                return;
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(background))
                .show(ctx, |ui| draw(ui.painter()));
        });
    harness.run();
    harness.run();
    harness
}

#[test]
fn language_sheets() {
    let mut results = SnapshotResults::new();
    const COLUMNS: usize = 8;
    const CELL: egui::Vec2 = vec2(80.0, 52.0);
    for (name, background, text, ink) in LOOKS {
        let rows = Language::ALL.len().div_ceil(COLUMNS);
        let size = vec2(COLUMNS as f32 * CELL.x + 16.0, rows as f32 * CELL.y + 16.0);
        let mut harness = sheet(size, 2.0, background, |painter| {
            for (index, language) in Language::ALL.iter().enumerate() {
                let cell = pos2(
                    8.0 + (index % COLUMNS) as f32 * CELL.x,
                    8.0 + (index / COLUMNS) as f32 * CELL.y,
                );
                let icon =
                    Rect::from_center_size(cell + vec2(CELL.x / 2.0, 16.0), vec2(16.0, 16.0));
                archipel_icons::paint_language(painter, icon, *language, ink);
                painter.text(
                    cell + vec2(CELL.x / 2.0, 36.0),
                    egui::Align2::CENTER_CENTER,
                    language.name(),
                    egui::FontId::proportional(9.5),
                    text,
                );
            }
        });
        results.add(harness.try_snapshot_options(
            format!("languages.{name}"),
            &SnapshotOptions::new().output_path("../../docs/icons/img"),
        ));
        // Actual size, 1×.
        let size = vec2(
            20.0 * 24.0 + 8.0,
            Language::ALL.len().div_ceil(20) as f32 * 24.0 + 8.0,
        );
        let mut harness = sheet(size, 1.0, background, |painter| {
            for (index, language) in Language::ALL.iter().enumerate() {
                let min = pos2(
                    8.0 + (index % 20) as f32 * 24.0,
                    8.0 + (index / 20) as f32 * 24.0,
                );
                archipel_icons::paint_language(
                    painter,
                    Rect::from_min_size(min, vec2(16.0, 16.0)),
                    *language,
                    ink,
                );
            }
        });
        results.add(harness.try_snapshot_options(
            format!("languages-actual-size.{name}"),
            &SnapshotOptions::new().output_path("../../docs/icons/img"),
        ));
    }
    results.unwrap();
}
