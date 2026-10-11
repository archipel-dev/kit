//! SVG export.

use crate::{Glyph, Paint, Shape, Tones};

use std::fmt::Write;

impl Glyph {
    /// The glyph as a standalone SVG document of `size` pixels, in `tones` (ink is
    /// `tones.ink`). Ink strokes and fills use `currentColor` when `tones.ink` is `0`, so a
    /// page can color the icon with CSS.
    pub fn to_svg(self, size: f32, tones: &Tones) -> String {
        parts_to_svg(self.parts(), size, tones)
    }
}

/// `parts` as a standalone SVG document; see [`Glyph::to_svg`].
pub fn parts_to_svg(parts: &[crate::Part], size: f32, tones: &Tones) -> String {
    let color = |tone| {
        let value = tones.get(tone, tones.ink);
        match tone {
            crate::Tone::Ink if tones.ink == 0 => "currentColor".to_owned(),
            // Closes the color attribute and adds its opacity (both are attribute values).
            crate::Tone::Rgba(rgba) => {
                format!(
                    "#{value:06X}\" fill-opacity=\"{:.2}",
                    (rgba & 0xFF) as f32 / 255.0
                )
            }
            _ => format!("#{value:06X}"),
        }
    };
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" \
             viewBox=\"0 0 16 16\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">"
    );
    for part in parts {
        let paint = match part.paint {
            Paint::Stroke { width, tone } => {
                format!(
                    "fill=\"none\" stroke=\"{}\" stroke-width=\"{width}\"",
                    color(tone)
                )
            }
            Paint::Fill(tone) => format!("fill=\"{}\"", color(tone)),
        };
        let points = |points: &[crate::Point]| {
            points
                .iter()
                .map(|(x, y)| format!("{x} {y}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        // Writing to a String cannot fail.
        let _ = match part.shape {
            Shape::Line(pts) => write!(out, "<polyline points=\"{}\" {paint}/>", points(pts)),
            Shape::Polygon(pts) => write!(out, "<polygon points=\"{}\" {paint}/>", points(pts)),
            Shape::Circle { center, radius } => write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{radius}\" {paint}/>",
                center.0, center.1
            ),
            Shape::Arc {
                center,
                radius,
                from,
                to,
            } => {
                let point = |deg: f32| {
                    let a = deg.to_radians();
                    (center.0 + radius * a.cos(), center.1 + radius * a.sin())
                };
                let (start, end) = (point(from), point(to));
                let large = u8::from((to - from).abs() > 180.0);
                let sweep = u8::from(to > from);
                write!(
                    out,
                    "<path d=\"M{:.3} {:.3} A{radius} {radius} 0 {large} {sweep} {:.3} {:.3}\" {paint}/>",
                    start.0, start.1, end.0, end.1
                )
            }
            Shape::Letter { .. } => {
                for line in part.shape.letter_lines() {
                    let _ = write!(out, "<polyline points=\"{}\" {paint}/>", points(&line));
                }
                Ok(())
            }
            Shape::Text { text, center, size } => write!(
                out,
                "<text x=\"{}\" y=\"{}\" font-family=\"Inter, system-ui, sans-serif\" \
                 font-size=\"{size}\" font-weight=\"600\" text-anchor=\"middle\" \
                 dominant-baseline=\"central\" {paint}>{}</text>",
                center.0,
                center.1,
                text.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            ),
            Shape::Rect { min, max, radius } => write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" {paint}/>",
                min.0,
                min.1,
                max.0 - min.0,
                max.1 - min.1
            ),
        };
    }
    out.push_str("</svg>");
    out
}
