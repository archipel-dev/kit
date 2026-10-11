//! The design rules of the icon set, checked for every glyph. A failing rule names the
//! glyph and the offending coordinate, so a new design can be fixed before review.

use archipel_icons::{Category, Glyph, Paint, Shape, Tone, Tones};
use std::collections::HashSet;

/// Live area: nothing may be drawn outside 1..15 (a 1-unit margin on the 16 grid).
const MIN: f32 = 1.0;
const MAX: f32 = 15.0;

fn is_half(value: f32) -> bool {
    (value.fract() - 0.5).abs() < 1e-3
}

fn is_whole(value: f32) -> bool {
    value.fract().abs() < 1e-3
}

/// Every point a part touches, including half the stroke width around it.
fn extent(shape: &Shape, pad: f32) -> (f32, f32, f32, f32) {
    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut grow = pad;
    match *shape {
        Shape::Line(pts) | Shape::Polygon(pts) => points.extend_from_slice(pts),
        Shape::Circle { center, radius } => {
            grow += radius;
            points.push(center);
        }
        Shape::Arc {
            center,
            radius,
            from,
            to,
        } => {
            points.extend(archipel_icons::arc_points(center, radius, from, to));
        }
        Shape::Rect { min, max, .. } => {
            points.push(min);
            points.push(max);
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in points {
        x0 = x0.min(x - grow);
        y0 = y0.min(y - grow);
        x1 = x1.max(x + grow);
        y1 = y1.max(y + grow);
    }
    (x0, y0, x1, y1)
}

#[test]
fn glyphs_stay_inside_the_live_area() {
    let mut errors = Vec::new();
    for glyph in Glyph::ALL {
        for part in glyph.parts() {
            let pad = match part.paint {
                // Round caps reach half the width past the end point.
                Paint::Stroke { width, .. } => width / 2.0,
                Paint::Fill(_) => 0.0,
            };
            let (x0, y0, x1, y1) = extent(&part.shape, pad);
            if x0 < MIN - 1e-3 || y0 < MIN - 1e-3 || x1 > MAX + 1e-3 || y1 > MAX + 1e-3 {
                errors.push(format!(
                    "{}: {:?} reaches ({x0:.2}, {y0:.2})..({x1:.2}, {y1:.2})",
                    glyph.name(),
                    part.shape
                ));
            }
        }
    }
    assert!(errors.is_empty(), "outside 1..15:\n{}", errors.join("\n"));
}

#[test]
fn straight_strokes_cover_whole_pixels() {
    let mut errors = Vec::new();
    for glyph in Glyph::ALL {
        for part in glyph.parts() {
            let Paint::Stroke { width, .. } = part.paint else {
                continue;
            };
            if (width - 1.0).abs() > 1e-3 {
                continue;
            }
            let segments: Vec<((f32, f32), (f32, f32))> = match part.shape {
                Shape::Line(pts) => pts.windows(2).map(|w| (w[0], w[1])).collect(),
                Shape::Polygon(pts) => pts
                    .iter()
                    .zip(pts.iter().cycle().skip(1))
                    .map(|(a, b)| (*a, *b))
                    .collect(),
                Shape::Rect { min, max, .. } => {
                    for value in [min.0, min.1, max.0, max.1] {
                        if !is_half(value) {
                            errors.push(format!("{}: rect edge {value}", glyph.name()));
                        }
                    }
                    Vec::new()
                }
                _ => Vec::new(),
            };
            for (a, b) in segments {
                let vertical = (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() > 1.0;
                let horizontal = (a.1 - b.1).abs() < 1e-3 && (a.0 - b.0).abs() > 1.0;
                if vertical && !is_half(a.0) {
                    errors.push(format!("{}: vertical stroke at x = {}", glyph.name(), a.0));
                }
                if horizontal && !is_half(a.1) {
                    errors.push(format!(
                        "{}: horizontal stroke at y = {}",
                        glyph.name(),
                        a.1
                    ));
                }
            }
        }
    }
    assert!(
        errors.is_empty(),
        "blurry 1-unit strokes (put them on x.5):\n{}",
        errors.join("\n")
    );
}

#[test]
fn filled_shapes_have_crisp_edges_and_are_convex() {
    let mut errors = Vec::new();
    for glyph in Glyph::ALL {
        for part in glyph.parts() {
            if !matches!(part.paint, Paint::Fill(_)) {
                continue;
            }
            match part.shape {
                Shape::Rect { min, max, .. } => {
                    for value in [min.0, min.1, max.0, max.1] {
                        if !is_whole(value) {
                            errors.push(format!("{}: filled rect edge {value}", glyph.name()));
                        }
                    }
                }
                Shape::Polygon(pts) => {
                    let mut sign = 0.0_f32;
                    for i in 0..pts.len() {
                        let (a, b, c) =
                            (pts[i], pts[(i + 1) % pts.len()], pts[(i + 2) % pts.len()]);
                        let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
                        if cross.abs() > 1e-3 {
                            if sign != 0.0 && cross.signum() != sign {
                                errors.push(format!(
                                    "{}: filled polygon is not convex",
                                    glyph.name()
                                ));
                                break;
                            }
                            sign = cross.signum();
                        }
                    }
                    for (a, b) in pts.iter().zip(pts.iter().cycle().skip(1)) {
                        let axis = (a.0 - b.0).abs() < 1e-3 || (a.1 - b.1).abs() < 1e-3;
                        let coordinate = if (a.0 - b.0).abs() < 1e-3 { a.0 } else { a.1 };
                        if axis && !is_whole(coordinate) {
                            errors.push(format!("{}: filled edge at {coordinate}", glyph.name()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn strokes_use_the_standard_widths() {
    for glyph in Glyph::ALL {
        for part in glyph.parts() {
            if let Paint::Stroke { width, tone } = part.paint {
                assert!(
                    width == 1.0 || (width == 1.5 && tone != Tone::Ink),
                    "{}: stroke width {width} (1 for ink, 1.5 only for status marks)",
                    glyph.name()
                );
            }
        }
    }
}

#[test]
fn names_are_unique_kebab_case_and_round_trip() {
    let mut seen = HashSet::new();
    for glyph in Glyph::ALL {
        let name = glyph.name();
        assert!(seen.insert(name), "duplicate name {name}");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !name.starts_with('-')
                && !name.ends_with('-'),
            "{name} is not kebab-case"
        );
        assert_eq!(Glyph::from_name(name), Some(*glyph));
        assert!(!glyph.keywords().is_empty(), "{name} has no keywords");
    }
}

#[test]
fn every_category_has_glyphs_and_search_finds_keywords() {
    for category in Category::ALL {
        assert!(
            category.glyphs().count() >= 3,
            "{} is nearly empty",
            category.label()
        );
    }
    assert!(Glyph::search("trash").any(|g| g == Glyph::Delete));
    assert!(Glyph::search("http").any(|g| g == Glyph::Globe));
}

#[test]
fn svg_export_is_well_formed() {
    let tones = archipel_icons::Tones {
        ink: 0,
        ..archipel_icons::Tones::dark()
    };
    for glyph in Glyph::ALL {
        let svg = glyph.to_svg(16.0, &tones);
        assert!(
            svg.starts_with("<svg") && svg.ends_with("</svg>"),
            "{}",
            glyph.name()
        );
        assert_eq!(
            svg.matches('<').count(),
            glyph.parts().len() + 2,
            "{}",
            glyph.name()
        );
        assert!(!svg.contains("NaN"), "{}", glyph.name());
    }
}

/// WCAG 2.1 non-text contrast (1.4.11): icon colors need 3:1 against their background.
#[test]
fn tones_contrast_with_the_background() {
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
    fn ratio(a: u32, b: u32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }
    for (name, tones) in [("dark", Tones::dark()), ("light", Tones::light())] {
        for (tone, value) in [
            ("ink", tones.ink),
            ("error", tones.error),
            ("warning", tones.warning),
            ("muted", tones.muted),
            ("success", tones.success),
            ("info", tones.info),
            ("run", tones.run),
            ("stop", tones.stop),
        ] {
            let contrast = ratio(value, tones.knockout);
            assert!(
                contrast >= 3.0,
                "{name} {tone}: {contrast:.2}:1 against the background"
            );
        }
    }
}
