//! 24-bit color to the xterm 256-color palette, for terminals without truecolor.
//!
//! The nearest color is searched in CIELAB (perceptual distance) among the indices 16 to 255
//! (the 6×6×6 cube and the 24 grays); indices 0 to 15 are the user's own palette and never
//! chosen. [`quantize_buffer`] rewrites the colors of a drawn frame in place.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// The level of cube axis `i` (0..6) in xterm's palette.
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

fn srgb_to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// CIELAB (D65) of an sRGB color.
fn lab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));
    let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = (0.0193339 * r + 0.119192 * g + 0.9503041 * b) / 1.08883;
    let f = |t: f32| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// The RGB of xterm palette index `i` (16..=255).
pub fn xterm_rgb(i: u8) -> (u8, u8, u8) {
    match i {
        16..=231 => {
            let n = i - 16;
            (
                CUBE[(n / 36) as usize],
                CUBE[((n / 6) % 6) as usize],
                CUBE[(n % 6) as usize],
            )
        }
        232..=255 => {
            let v = 8 + 10 * (i - 232);
            (v, v, v)
        }
        _ => (0, 0, 0),
    }
}

/// The index (16..=255) of the xterm color nearest to (`r`, `g`, `b`) in CIELAB.
pub fn nearest_xterm(r: u8, g: u8, b: u8) -> u8 {
    let target = lab(r, g, b);
    let mut best = (16u8, f32::MAX);
    for i in 16..=255u8 {
        let (cr, cg, cb) = xterm_rgb(i);
        let l = lab(cr, cg, cb);
        let d =
            (l[0] - target[0]).powi(2) + (l[1] - target[1]).powi(2) + (l[2] - target[2]).powi(2);
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

/// `color` for a 256-color terminal: RGB becomes the nearest indexed color, everything else
/// stays.
pub fn quantize(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Indexed(nearest_xterm(r, g, b)),
        other => other,
    }
}

/// Rewrites every foreground, background and underline color of `buf` with [`quantize`].
pub fn quantize_buffer(buf: &mut Buffer) {
    let area = buf.area;
    let mut cache: std::collections::HashMap<(u8, u8, u8), u8> = std::collections::HashMap::new();
    let mut map = |color: Color| match color {
        Color::Rgb(r, g, b) => Color::Indexed(
            *cache
                .entry((r, g, b))
                .or_insert_with(|| nearest_xterm(r, g, b)),
        ),
        other => other,
    };
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.fg = map(cell.fg);
            cell.bg = map(cell.bg);
            cell.underline_color = map(cell.underline_color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_cube_colors_map_to_themselves() {
        assert_eq!(nearest_xterm(0, 0, 0), 16);
        assert_eq!(nearest_xterm(255, 255, 255), 231);
        assert_eq!(nearest_xterm(95, 135, 175), 16 + 36 + 2 * 6 + 3);
    }

    #[test]
    fn grays_prefer_the_gray_ramp_and_never_the_user_palette() {
        let i = nearest_xterm(0x2B, 0x2D, 0x30);
        assert!(i >= 16);
        let (r, _, b) = xterm_rgb(i);
        assert!((i32::from(r) - 0x2B).abs() < 16 && (i32::from(b) - 0x30).abs() < 16);
        for v in 0..=255u8 {
            assert!(nearest_xterm(v, v, v) >= 16);
        }
    }

    #[test]
    fn buffers_are_rewritten_in_place() {
        let mut buf = Buffer::empty(ratatui::layout::Rect::new(0, 0, 2, 1));
        buf[(0, 0)]
            .set_fg(Color::Rgb(255, 0, 0))
            .set_bg(Color::Rgb(0, 0, 0));
        quantize_buffer(&mut buf);
        assert_eq!(buf[(0, 0)].fg, Color::Indexed(196));
        assert_eq!(buf[(0, 0)].bg, Color::Indexed(16));
        assert_eq!(buf[(1, 0)].fg, Color::Reset);
    }
}
