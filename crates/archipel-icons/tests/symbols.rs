//! Code symbol icons: colors, contrast, LSP mapping, geometry and the catalog images.

mod common;

use archipel_icons::{Paint, Shape, Symbol, SymbolKind, SymbolShape, Tones, Visibility, contrast};
use common::{LOOKS, sheet};
use egui::{Rect, pos2, vec2};
use egui_kittest::{SnapshotOptions, SnapshotResults};
use std::collections::HashSet;

/// Every symbol with each modifier, as shown in the legend.
fn variants() -> Vec<Symbol> {
    let mut all: Vec<Symbol> = SymbolKind::ALL.iter().map(|k| Symbol::new(*k)).collect();
    all.extend(legend());
    all
}

fn legend() -> [Symbol; 6] {
    [
        Symbol::new(SymbolKind::Class).abstract_(true),
        Symbol::new(SymbolKind::Method).abstract_(true),
        Symbol::new(SymbolKind::Method).static_(true),
        Symbol::new(SymbolKind::Field).visibility(Visibility::Private),
        Symbol::new(SymbolKind::Method).visibility(Visibility::Protected),
        Symbol::new(SymbolKind::Class)
            .visibility(Visibility::Internal)
            .static_(true),
    ]
}

fn luminance(value: u32) -> f32 {
    let channel = |shift: u32| {
        let c = ((value >> shift) & 0xFF) as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// White labels need 4.5:1 and the shape 3:1 on both themes: luminance 0.129..0.183.
#[test]
fn colors_sit_in_the_white_label_band() {
    for kind in SymbolKind::ALL {
        let color = kind.color();
        let l = luminance(color);
        assert!(
            (0.129..=0.183).contains(&l),
            "{}: luminance {l:.3}",
            kind.name()
        );
        assert!(contrast(color, 0xFFFFFF) >= 4.5, "{}: label", kind.name());
        assert!(
            contrast(color, Tones::dark().knockout) >= 3.0,
            "{}: on dark",
            kind.name()
        );
        assert!(
            contrast(color, Tones::light().knockout) >= 3.0,
            "{}: on light",
            kind.name()
        );
    }
}

#[test]
fn labels_are_unique_within_a_shape() {
    let mut seen = HashSet::new();
    for kind in SymbolKind::ALL {
        if kind.shape() != SymbolShape::Namespace {
            assert!(
                seen.insert((kind.shape() == SymbolShape::Type, kind.label())),
                "{}",
                kind.name()
            );
        }
    }
}

#[test]
fn lsp_kinds_map() {
    assert_eq!(SymbolKind::from_lsp(5), Some(SymbolKind::Class));
    assert_eq!(SymbolKind::from_lsp(6), Some(SymbolKind::Method));
    assert_eq!(SymbolKind::from_lsp(11), Some(SymbolKind::Interface));
    assert_eq!(SymbolKind::from_lsp(22), Some(SymbolKind::EnumMember));
    assert_eq!(SymbolKind::from_lsp(26), Some(SymbolKind::TypeParameter));
    assert_eq!(SymbolKind::from_lsp(1), None);
    assert_eq!(SymbolKind::from_lsp(99), None);
}

#[test]
fn symbols_stay_inside_the_live_area() {
    for symbol in variants() {
        for part in symbol.parts() {
            let pad = match part.paint {
                Paint::Stroke { width, .. } => width / 2.0,
                Paint::Fill(_) => 0.0,
            };
            let (lo, hi) = match part.shape {
                Shape::Circle { center, radius } => (
                    (center.0 - radius - pad, center.1 - radius - pad),
                    (center.0 + radius + pad, center.1 + radius + pad),
                ),
                Shape::Rect { min, max, .. } => {
                    ((min.0 - pad, min.1 - pad), (max.0 + pad, max.1 + pad))
                }
                Shape::Polygon(points) | Shape::Line(points) => points.iter().fold(
                    ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN)),
                    |(lo, hi), p| {
                        (
                            (lo.0.min(p.0 - pad), lo.1.min(p.1 - pad)),
                            (hi.0.max(p.0 + pad), hi.1.max(p.1 + pad)),
                        )
                    },
                ),
                _ => continue,
            };
            assert!(
                lo.0 >= 1.0 - 1e-3
                    && lo.1 >= 1.0 - 1e-3
                    && hi.0 <= 15.0 + 1e-3
                    && hi.1 <= 15.0 + 1e-3,
                "{}: {:?} reaches {lo:?}..{hi:?}",
                symbol.slug(),
                part.shape
            );
        }
    }
}

#[test]
fn symbol_sheets() {
    let mut results = SnapshotResults::new();
    const COLUMNS: usize = 7;
    const CELL: egui::Vec2 = vec2(92.0, 52.0);
    let symbols = variants();
    for (name, background, text, ink) in LOOKS {
        let tones = if name == "dark" {
            Tones::dark()
        } else {
            Tones::light()
        };
        let rows = symbols.len().div_ceil(COLUMNS);
        let size = vec2(COLUMNS as f32 * CELL.x + 16.0, rows as f32 * CELL.y + 16.0);
        let mut harness = sheet(size, 2.0, background, |painter| {
            for (index, symbol) in symbols.iter().enumerate() {
                let cell = pos2(
                    8.0 + (index % COLUMNS) as f32 * CELL.x,
                    8.0 + (index / COLUMNS) as f32 * CELL.y,
                );
                let icon =
                    Rect::from_center_size(cell + vec2(CELL.x / 2.0, 16.0), vec2(16.0, 16.0));
                archipel_icons::paint_symbol(painter, icon, *symbol, ink, &tones);
                painter.text(
                    cell + vec2(CELL.x / 2.0, 36.0),
                    egui::Align2::CENTER_CENTER,
                    symbol.slug().replace('-', " "),
                    egui::FontId::proportional(9.5),
                    text,
                );
            }
        });
        results.add(harness.try_snapshot_options(
            format!("symbols.{name}"),
            &SnapshotOptions::new().output_path("../../docs/icons/img"),
        ));
        let size = vec2(symbols.len() as f32 * 24.0 + 8.0, 32.0);
        let mut harness = sheet(size, 1.0, background, |painter| {
            for (index, symbol) in symbols.iter().enumerate() {
                let min = pos2(8.0 + index as f32 * 24.0, 8.0);
                archipel_icons::paint_symbol(
                    painter,
                    Rect::from_min_size(min, vec2(16.0, 16.0)),
                    *symbol,
                    ink,
                    &tones,
                );
            }
        });
        results.add(harness.try_snapshot_options(
            format!("symbols-actual-size.{name}"),
            &SnapshotOptions::new().output_path("../../docs/icons/img"),
        ));
    }
    results.unwrap();
}
