//! Language icons: labels, colors, file matching, and the catalog images.

use archipel_icons::{Language, contrast};
mod common;

use common::{LOOKS, sheet};
use egui::{Rect, pos2, vec2};
use egui_kittest::{SnapshotOptions, SnapshotResults};
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

#[test]
fn pixel_labels_are_unique_and_in_the_font() {
    let mut seen = HashSet::new();
    for language in Language::ALL {
        let label = language.pixel_label();
        assert!(seen.insert(label), "{label} is used twice");
        assert!((1..=2).contains(&label.chars().count()), "{label}");
        for c in label.chars() {
            assert!(
                archipel_icons::letters::strokes(c).is_some(),
                "{}: no {c:?} in the font",
                language.name()
            );
        }
    }
}

/// The tinted style draws label and outline in the language color: it must read on both.
#[test]
fn tinted_ink_reads_on_both_themes() {
    use archipel_icons::{LanguageStyle, Paint, Tone, Tones};
    for language in Language::ALL {
        for part in language.parts_styled(LanguageStyle::TintedFile) {
            if let Paint::Stroke {
                tone: Tone::Rgb(color),
                ..
            } = part.paint
            {
                for background in [Tones::dark().knockout, Tones::light().knockout] {
                    assert!(
                        contrast(color, background) >= 1.9,
                        "{}: {color:06X} on {background:06X}",
                        language.name()
                    );
                }
            }
        }
    }
}

#[test]
fn every_style_stays_inside_the_tile() {
    use archipel_icons::{LanguageStyle, Paint, Shape};
    for style in LanguageStyle::ALL {
        for language in Language::ALL {
            for part in language.parts_styled(style) {
                let pad = match part.paint {
                    Paint::Stroke { width, .. } => width / 2.0,
                    Paint::Fill(_) => 0.0,
                };
                let points: Vec<(f32, f32)> = match part.shape {
                    Shape::Rect { min, max, .. } => vec![min, max],
                    Shape::Circle { center, radius } => {
                        vec![
                            (center.0 - radius, center.1 - radius),
                            (center.0 + radius, center.1 + radius),
                        ]
                    }
                    Shape::Line(p) | Shape::Polygon(p) => p.to_vec(),
                    Shape::Letter { .. } => part.shape.letter_lines().concat(),
                    _ => continue,
                };
                for (x, y) in points {
                    assert!(
                        x - pad >= 1.0 - 1e-3
                            && y - pad >= 1.0 - 1e-3
                            && x + pad <= 15.0 + 1e-3
                            && y + pad <= 15.0 + 1e-3,
                        "{} {:?}: ({x}, {y})",
                        language.name(),
                        style
                    );
                }
            }
        }
    }
}

#[test]
fn style_comparison_sheets() {
    use archipel_icons::LanguageStyle;
    let languages = [
        Language::Rust,
        Language::Java,
        Language::Kotlin,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Go,
        Language::C,
        Language::Cpp,
        Language::CSharp,
        Language::Swift,
        Language::Ruby,
        Language::Php,
        Language::Html,
        Language::Css,
        Language::Json,
        Language::Yaml,
        Language::Markdown,
        Language::Shell,
        Language::Dockerfile,
    ];
    let mut results = SnapshotResults::new();
    for (name, background, text, ink) in LOOKS {
        let size = vec2(
            110.0 + languages.len() as f32 * 26.0,
            8.0 + LanguageStyle::ALL.len() as f32 * 30.0,
        );
        let mut harness = sheet(size, 2.0, background, |painter| {
            for (row, style) in LanguageStyle::ALL.iter().enumerate() {
                let y = 8.0 + row as f32 * 30.0;
                painter.text(
                    pos2(10.0, y + 8.0),
                    egui::Align2::LEFT_CENTER,
                    style.name(),
                    egui::FontId::proportional(10.0),
                    text,
                );
                for (i, language) in languages.iter().enumerate() {
                    let rect =
                        Rect::from_min_size(pos2(110.0 + i as f32 * 26.0, y), vec2(16.0, 16.0));
                    archipel_icons::paint_language_styled(painter, rect, *language, *style, ink);
                }
            }
        });
        results.add(harness.try_snapshot_options(
            format!("language-styles.{name}"),
            &SnapshotOptions::new().output_path("../../docs/icons/img"),
        ));
    }
    results.unwrap();
}
