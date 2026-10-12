//! The project color glow on the window frame.

use crate::tokens::glow;
use egui::{Color32, Mesh, Painter, Pos2, Rect, pos2, vec2};

/// Paints the soft glow of a project's color that Android Studio's Islands UI shows on the
/// window frame behind the project widget, clipped to `painter`'s clip rect. `window` is the
/// whole window (or viewport) rect: every part of the frame that paints the glow anchors it
/// to the same place, so the toolbar and the stripes below it continue one another.
pub fn project_glow(painter: &Painter, window: Rect, color: Color32) {
    if color.a() == 0 || !window.is_positive() {
        return;
    }
    // Light themes glow in a pastel of the color, dark ones in the color itself.
    let (color, alpha) = if crate::Theme::current(painter.ctx()).dark {
        (color, glow::ALPHA)
    } else {
        (
            color.lerp_to_gamma(Color32::WHITE, glow::LIGHT_PASTEL),
            glow::LIGHT_ALPHA,
        )
    };
    let center = window.min + vec2(glow::CENTER_X, glow::CENTER_Y);
    let radii = vec2(glow::RADIUS_X, glow::RADIUS_Y);
    let bounds = Rect::from_center_size(center, 2.0 * radii);
    if !painter.clip_rect().intersects(bounds) {
        return;
    }
    let rings = 24;
    let segments = 72;
    let mut mesh = Mesh::default();
    let at = |ring: usize, segment: usize| -> Pos2 {
        let t = ring as f32 / rings as f32;
        let angle = segment as f32 / segments as f32 * std::f32::consts::TAU;
        pos2(
            center.x + radii.x * t * angle.cos(),
            center.y + radii.y * t * angle.sin(),
        )
    };
    // Strongest in the middle, easing out to nothing at the edge.
    let shade = |ring: usize| {
        let t = ring as f32 / rings as f32;
        let falloff = 1.0 - t * t * (3.0 - 2.0 * t);
        color.gamma_multiply(alpha * falloff)
    };
    mesh.colored_vertex(center, shade(0));
    for ring in 1..=rings {
        for segment in 0..segments {
            mesh.colored_vertex(at(ring, segment), shade(ring));
        }
    }
    let index = |ring: usize, segment: usize| -> u32 {
        if ring == 0 {
            0
        } else {
            (1 + (ring - 1) * segments + segment % segments) as u32
        }
    };
    for segment in 0..segments {
        mesh.add_triangle(index(0, 0), index(1, segment), index(1, segment + 1));
    }
    for ring in 1..rings {
        for segment in 0..segments {
            let (a, b) = (index(ring, segment), index(ring, segment + 1));
            let (c, d) = (index(ring + 1, segment), index(ring + 1, segment + 1));
            mesh.add_triangle(a, c, b);
            mesh.add_triangle(b, c, d);
        }
    }
    painter.add(mesh);
}
