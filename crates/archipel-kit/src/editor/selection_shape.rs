//! The Islands selection outline (spec §8.1): one shape per contiguous block of selected rows,
//! outer corners rounded with radius `line height / 6`, and where rows of different widths
//! meet, concave fillets of the same radius so the rows read as one outline.
//!
//! The outline is delivered as pieces egui can fill with anti-aliasing: one rectangle per row
//! with the rounding of each of its corners, and a small fan for every concave fillet.

use egui::{Pos2, Rect, pos2};

/// A row rectangle and the radius of each corner: north-west, north-east, south-east,
/// south-west.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowRect {
    pub rect: Rect,
    pub radii: [f32; 4],
}

/// A concave corner: the area between `vertex` and a quarter circle of `radius`, in the
/// quadrant `(sx, sy)` away from the vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fillet {
    pub vertex: Pos2,
    pub sx: f32,
    pub sy: f32,
    pub radius: f32,
}

impl Fillet {
    /// The vertex, then the points of the arc from one edge to the other.
    pub fn fan(&self, steps: usize) -> Vec<Pos2> {
        let center = pos2(
            self.vertex.x + self.sx * self.radius,
            self.vertex.y + self.sy * self.radius,
        );
        let mut points = Vec::with_capacity(steps + 2);
        points.push(self.vertex);
        for step in 0..=steps {
            let angle = std::f32::consts::FRAC_PI_2 * step as f32 / steps.max(1) as f32;
            points.push(pos2(
                center.x - self.sx * self.radius * angle.cos(),
                center.y - self.sy * self.radius * angle.sin(),
            ));
        }
        points
    }
}

/// The pieces of one selection outline.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Outline {
    pub rows: Vec<RowRect>,
    pub fillets: Vec<Fillet>,
}

/// The outline of consecutive rows. `rows` holds the `(left, right)` edges of each row, top
/// row first; `top` is the y of the first row, `height` the height of every row and `radius`
/// the corner radius.
pub(crate) fn outline(rows: &[(f32, f32)], top: f32, height: f32, radius: f32) -> Outline {
    let mut out = Outline::default();
    for (i, &(left, right)) in rows.iter().enumerate() {
        let y = top + i as f32 * height;
        out.rows.push(RowRect {
            rect: Rect::from_min_max(pos2(left, y), pos2(right, y + height)),
            radii: [0.0; 4],
        });
    }
    let n = rows.len();
    if n == 0 {
        return out;
    }
    // The ends of the block.
    out.rows[0].radii[0] = radius;
    out.rows[0].radii[1] = radius;
    out.rows[n - 1].radii[2] = radius;
    out.rows[n - 1].radii[3] = radius;
    // Each junction between a row and the one below it.
    for i in 0..n - 1 {
        let (upper, lower) = (rows[i], rows[i + 1]);
        let y = top + (i + 1) as f32 * height;
        // Left edges.
        if upper.0 < lower.0 {
            // The upper row sticks out to the left.
            let r = radius.min((lower.0 - upper.0) / 2.0);
            out.rows[i].radii[3] = r;
            push_fillet(&mut out, pos2(lower.0, y), -1.0, 1.0, r);
        } else if upper.0 > lower.0 {
            let r = radius.min((upper.0 - lower.0) / 2.0);
            out.rows[i + 1].radii[0] = r;
            push_fillet(&mut out, pos2(upper.0, y), -1.0, -1.0, r);
        }
        // Right edges.
        if upper.1 > lower.1 {
            let r = radius.min((upper.1 - lower.1) / 2.0);
            out.rows[i].radii[2] = r;
            push_fillet(&mut out, pos2(lower.1, y), 1.0, 1.0, r);
        } else if upper.1 < lower.1 {
            let r = radius.min((lower.1 - upper.1) / 2.0);
            out.rows[i + 1].radii[1] = r;
            push_fillet(&mut out, pos2(upper.1, y), 1.0, -1.0, r);
        }
    }
    // A corner cannot be rounder than half its row.
    for row in &mut out.rows {
        let limit = (row.rect.width().min(row.rect.height()) / 2.0).max(0.0);
        for r in &mut row.radii {
            *r = r.min(limit);
        }
    }
    out
}

fn push_fillet(out: &mut Outline, vertex: Pos2, sx: f32, sy: f32, radius: f32) {
    if radius >= 0.75 {
        out.fillets.push(Fillet {
            vertex,
            sx,
            sy,
            radius,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_row_has_four_round_corners() {
        let o = outline(&[(10.0, 50.0)], 0.0, 20.0, 3.0);
        assert_eq!(o.rows.len(), 1);
        assert_eq!(o.rows[0].radii, [3.0; 4]);
        assert!(o.fillets.is_empty());
    }

    #[test]
    fn a_shorter_row_below_rounds_the_outer_corner_and_adds_a_fillet() {
        // Row 0 spans 10..80, row 1 spans 10..40: the right side steps in at y = 20.
        let o = outline(&[(10.0, 80.0), (10.0, 40.0)], 0.0, 20.0, 3.0);
        assert_eq!(o.rows[0].radii, [3.0, 3.0, 3.0, 0.0], "upper south-east");
        assert_eq!(o.rows[1].radii, [0.0, 0.0, 3.0, 3.0]);
        assert_eq!(
            o.fillets,
            vec![Fillet {
                vertex: pos2(40.0, 20.0),
                sx: 1.0,
                sy: 1.0,
                radius: 3.0
            }]
        );
    }

    #[test]
    fn a_longer_row_below_rounds_its_north_east_corner() {
        let o = outline(&[(10.0, 40.0), (10.0, 80.0)], 0.0, 20.0, 3.0);
        assert_eq!(o.rows[1].radii[1], 3.0);
        assert_eq!(o.fillets[0].vertex, pos2(40.0, 20.0));
        assert_eq!((o.fillets[0].sx, o.fillets[0].sy), (1.0, -1.0));
    }

    #[test]
    fn left_steps_mirror_right_steps() {
        let o = outline(&[(30.0, 80.0), (10.0, 80.0)], 0.0, 20.0, 3.0);
        assert_eq!(o.rows[1].radii[0], 3.0, "the lower row sticks out left");
        assert_eq!((o.fillets[0].sx, o.fillets[0].sy), (-1.0, -1.0));
        let o = outline(&[(10.0, 80.0), (30.0, 80.0)], 0.0, 20.0, 3.0);
        assert_eq!(o.rows[0].radii[3], 3.0);
        assert_eq!((o.fillets[0].sx, o.fillets[0].sy), (-1.0, 1.0));
    }

    #[test]
    fn a_step_of_one_point_gets_a_smaller_radius_or_none() {
        let o = outline(&[(10.0, 41.0), (10.0, 40.0)], 0.0, 20.0, 3.0);
        assert!(o.fillets.is_empty(), "radius 0.5 draws nothing");
        assert_eq!(o.rows[0].radii[2], 0.5);
    }

    #[test]
    fn the_fan_starts_at_the_vertex_and_ends_on_the_two_edges() {
        let f = Fillet {
            vertex: pos2(40.0, 20.0),
            sx: 1.0,
            sy: 1.0,
            radius: 3.0,
        };
        let fan = f.fan(4);
        assert_eq!(fan[0], pos2(40.0, 20.0));
        let first = fan[1];
        let last = *fan.last().expect("a fan has points");
        assert!((first.x - 40.0).abs() < 1e-4 && (first.y - 23.0).abs() < 1e-4);
        assert!((last.x - 43.0).abs() < 1e-4 && (last.y - 20.0).abs() < 1e-4);
    }
}
