//! Painting with egui.

use crate::{Glyph, Paint, Shape, Tones};
use egui::{
    Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, epaint::PathShape, vec2,
};

fn color(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// Paints `glyph` into the square centered in `rect`, scaled from its 16×16 grid, with
/// `ink` as its main color and `tones` for the semantic parts.
pub fn paint(painter: &Painter, rect: Rect, glyph: Glyph, ink: Color32, tones: &Tones) {
    paint_parts(painter, rect, glyph.parts(), ink, tones);
}

/// Paints the icon of `language` into the square centered in `rect`.
pub fn paint_language(painter: &Painter, rect: Rect, language: crate::Language) {
    paint_parts(
        painter,
        rect,
        &language.parts(),
        Color32::WHITE,
        &Tones::dark(),
    );
}

/// Paints `parts` (16×16 grid) into the square centered in `rect`.
pub fn paint_parts(
    painter: &Painter,
    rect: Rect,
    parts: &[crate::Part],
    ink: Color32,
    tones: &Tones,
) {
    let side = rect.width().min(rect.height());
    let scale = side / 16.0;
    let origin = rect.center() - vec2(8.0 * scale, 8.0 * scale);
    let p = |(x, y): crate::Point| -> Pos2 { origin + vec2(x * scale, y * scale) };
    let resolve = |tone| match tone {
        crate::Tone::Ink => ink,
        tone => color(tones.get(tone, 0)),
    };
    for part in parts {
        let (fill, stroke) = match part.paint {
            Paint::Stroke { width, tone } => (
                Color32::TRANSPARENT,
                Stroke::new((width * scale).max(1.0), resolve(tone)),
            ),
            Paint::Fill(tone) => (resolve(tone), Stroke::NONE),
        };
        match part.shape {
            Shape::Line(points) => {
                painter.add(PathShape::line(
                    points.iter().map(|&q| p(q)).collect(),
                    stroke,
                ));
            }
            Shape::Polygon(points) => {
                painter.add(PathShape {
                    points: points.iter().map(|&q| p(q)).collect(),
                    closed: true,
                    fill,
                    stroke: stroke.into(),
                });
            }
            Shape::Circle { center, radius } => {
                painter.circle(p(center), radius * scale, fill, stroke);
            }
            Shape::Arc {
                center,
                radius,
                from,
                to,
            } => {
                let points = crate::arc_points(center, radius, from, to);
                painter.add(PathShape::line(points.into_iter().map(p).collect(), stroke));
            }
            Shape::Letter { .. } => {
                for line in part.shape.letter_lines() {
                    painter.add(PathShape::line(line.into_iter().map(p).collect(), stroke));
                }
            }
            Shape::Rect { min, max, radius } => {
                painter.rect(
                    Rect::from_min_max(p(min), p(max)),
                    CornerRadius::same((radius * scale).round() as u8),
                    fill,
                    stroke,
                    StrokeKind::Middle,
                );
            }
        }
    }
}

/// Allocates a `size`-point square in `ui` and paints `glyph` into it.
pub fn show(
    ui: &mut egui::Ui,
    glyph: Glyph,
    size: f32,
    ink: Color32,
    tones: &Tones,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    paint(ui.painter(), rect, glyph, ink, tones);
    response
}
