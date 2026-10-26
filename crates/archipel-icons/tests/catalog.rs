//! Catalog images: every glyph rendered for the docs (`docs/icons/img/`), and the review
//! sheets designers look at before a glyph is accepted.
//!
//! - `{category}.{dark,light}.png`: labeled glyphs at 2×, shown in `docs/icons/catalog.md`.
//! - `actual-size.{dark,light}.png`: every glyph at 16 px and 1×, the size it is used at,
//!   to judge crispness.
//! - `REVIEW=1 cargo test -p archipel-icons --test catalog` also writes large review sheets
//!   with the 16×16 grid and the live-area keyline to `target/icon-review/`.

use archipel_icons::{Category, Glyph, Tones};
use egui::{Color32, Rect, Stroke, pos2, vec2};
use egui_kittest::{Harness, SnapshotOptions, SnapshotResults};

const DOCS: &str = "../../docs/icons/img";

struct Look {
    name: &'static str,
    background: Color32,
    ink: Color32,
    text: Color32,
    tones: Tones,
}

const LOOKS: [Look; 2] = [
    Look {
        name: "dark",
        background: Color32::from_rgb(0x19, 0x1A, 0x1C),
        ink: Color32::from_rgb(0xCE, 0xD0, 0xD6),
        text: Color32::from_rgb(0x9F, 0xA2, 0xA8),
        tones: Tones::dark(),
    },
    Look {
        name: "light",
        background: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        ink: Color32::from_rgb(0x6C, 0x70, 0x7E),
        text: Color32::from_rgb(0x5F, 0x62, 0x69),
        tones: Tones::light(),
    },
];

fn sheet<'a>(
    size: egui::Vec2,
    scale: f32,
    background: Color32,
    draw: impl FnMut(&egui::Painter) + 'a,
) -> Harness<'a> {
    let mut draw = draw;
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(scale)
        .wgpu()
        .build(move |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(background))
                .show(ctx, |ui| draw(ui.painter()));
        });
    harness.run();
    harness
}

#[test]
fn category_sheets() {
    let mut results = SnapshotResults::new();
    const COLUMNS: usize = 6;
    const CELL: egui::Vec2 = vec2(104.0, 52.0);
    for look in &LOOKS {
        for category in Category::ALL {
            let glyphs: Vec<Glyph> = category.glyphs().collect();
            let rows = glyphs.len().div_ceil(COLUMNS);
            let size = vec2(COLUMNS as f32 * CELL.x + 16.0, rows as f32 * CELL.y + 16.0);
            let mut harness = sheet(size, 2.0, look.background, |painter| {
                for (index, glyph) in glyphs.iter().enumerate() {
                    let cell = pos2(
                        8.0 + (index % COLUMNS) as f32 * CELL.x,
                        8.0 + (index / COLUMNS) as f32 * CELL.y,
                    );
                    let icon =
                        Rect::from_center_size(cell + vec2(CELL.x / 2.0, 16.0), vec2(16.0, 16.0));
                    archipel_icons::paint(painter, icon, *glyph, look.ink, &look.tones);
                    painter.text(
                        cell + vec2(CELL.x / 2.0, 36.0),
                        egui::Align2::CENTER_CENTER,
                        glyph.name(),
                        egui::FontId::proportional(10.0),
                        look.text,
                    );
                }
            });
            let slug = category
                .label()
                .to_lowercase()
                .replace([' ', '&'], "-")
                .replace("--", "-")
                .replace("--", "-");
            results.add(harness.try_snapshot_options(
                format!("{slug}.{}", look.name),
                &SnapshotOptions::new().output_path(DOCS),
            ));
        }
    }
    results.unwrap();
}

#[test]
fn actual_size_sheet() {
    let mut results = SnapshotResults::new();
    const COLUMNS: usize = 16;
    for look in &LOOKS {
        let rows = Glyph::ALL.len().div_ceil(COLUMNS);
        let size = vec2(COLUMNS as f32 * 24.0 + 8.0, rows as f32 * 24.0 + 8.0);
        let mut harness = sheet(size, 1.0, look.background, |painter| {
            for (index, glyph) in Glyph::ALL.iter().enumerate() {
                let min = pos2(
                    8.0 + (index % COLUMNS) as f32 * 24.0,
                    8.0 + (index / COLUMNS) as f32 * 24.0,
                );
                archipel_icons::paint(
                    painter,
                    Rect::from_min_size(min, vec2(16.0, 16.0)),
                    *glyph,
                    look.ink,
                    &look.tones,
                );
            }
        });
        results.add(harness.try_snapshot_options(
            format!("actual-size.{}", look.name),
            &SnapshotOptions::new().output_path(DOCS),
        ));
    }
    results.unwrap();
}

/// Large sheets with the grid, for reviewing geometry (not committed).
#[test]
fn review_sheets() {
    if std::env::var_os("REVIEW").is_none() {
        return;
    }
    const COLUMNS: usize = 6;
    const ICON: f32 = 96.0;
    for look in &LOOKS {
        for (page, chunk) in Glyph::ALL.chunks(24).enumerate() {
            let rows = chunk.len().div_ceil(COLUMNS);
            let size = vec2(
                COLUMNS as f32 * (ICON + 24.0) + 24.0,
                rows as f32 * (ICON + 40.0) + 24.0,
            );
            let mut harness = sheet(size, 1.0, look.background, |painter| {
                for (index, glyph) in chunk.iter().enumerate() {
                    let min = pos2(
                        24.0 + (index % COLUMNS) as f32 * (ICON + 24.0),
                        24.0 + (index / COLUMNS) as f32 * (ICON + 40.0),
                    );
                    let rect = Rect::from_min_size(min, vec2(ICON, ICON));
                    let unit = ICON / 16.0;
                    for i in 0..=16 {
                        let color = if i == 1 || i == 15 {
                            Color32::from_rgb(0x80, 0x40, 0x40)
                        } else {
                            if look.name == "dark" {
                                Color32::from_gray(0x2C)
                            } else {
                                Color32::from_gray(0xE8)
                            }
                        };
                        let offset = i as f32 * unit;
                        painter.vline(
                            rect.left() + offset,
                            rect.y_range(),
                            Stroke::new(1.0_f32, color),
                        );
                        painter.hline(
                            rect.x_range(),
                            rect.top() + offset,
                            Stroke::new(1.0_f32, color),
                        );
                    }
                    archipel_icons::paint(painter, rect, *glyph, look.ink, &look.tones);
                    painter.text(
                        rect.center_bottom() + vec2(0.0, 14.0),
                        egui::Align2::CENTER_CENTER,
                        glyph.name(),
                        egui::FontId::proportional(13.0),
                        look.text,
                    );
                }
            });
            let image = harness.render().expect("render");
            std::fs::create_dir_all("../../target/icon-review").expect("review dir");
            image
                .save(format!(
                    "../../target/icon-review/page-{page}.{}.png",
                    look.name
                ))
                .expect("save review sheet");
        }
    }
}
