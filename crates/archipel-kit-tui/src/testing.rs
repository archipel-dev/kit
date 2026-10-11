//! Test helpers: render widgets into an in-memory buffer, read its text, and compare it
//! against an SVG snapshot (`docs/tui/img/<name>.svg`). The SVGs are the docs images: one
//! rect per background run and one text element per cell, in a monospace font.
//!
//! Nerd Font characters are not text in the SVG (a viewer has no Nerd Font, and the kit
//! ships no third-party artwork): icon cells show the matching Archipel icon, drawn from
//! `archipel-icons`, and the Powerline half circles and checkbox and radio marks are drawn
//! as shapes. In a terminal the Nerd Font draws them; they look alike.
//!
//! `UPDATE_SNAPSHOTS=1 cargo test -p archipel-kit-tui` rewrites changed snapshots.

use crate::glyphs;
use archipel_icons::{Glyph, Language, LanguageStyle, Symbol, SymbolKind, Tones, parts_to_svg};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Cell size in SVG pixels.
const CELL_W: f32 = 8.4;
const CELL_H: f32 = 18.0;
/// Icon size: Nerd Font icons are about 1.7 cells wide and draw into the next cell.
const ICON: f32 = 14.0;

/// A buffer of `width` × `height` cells drawn by `draw`.
pub fn render(width: u16, height: u16, draw: impl FnOnce(Rect, &mut Buffer)) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    draw(area, &mut buf);
    buf
}

/// The text of each row, trailing spaces trimmed.
pub fn lines(buf: &Buffer) -> Vec<String> {
    let area = buf.area;
    (area.top()..area.bottom())
        .map(|y| {
            let mut line = String::new();
            for x in area.left()..area.right() {
                line.push_str(buf[(x, y)].symbol());
            }
            line.trim_end().to_owned()
        })
        .collect()
}

fn rgb(color: Color) -> Option<u32> {
    match color {
        Color::Rgb(r, g, b) => Some(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)),
        _ => None,
    }
}

fn hex(color: Color, fallback: u32) -> String {
    format!("#{:06X}", rgb(color).unwrap_or(fallback))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The block element `symbol` as rects in cell fractions (x, y, w, h).
fn block(symbol: &str) -> Option<&'static [(f32, f32, f32, f32)]> {
    Some(match symbol {
        "█" => &[(0.0, 0.0, 1.0, 1.0)],
        "▌" => &[(0.0, 0.0, 0.5, 1.0)],
        "▐" => &[(0.5, 0.0, 0.5, 1.0)],
        "▀" => &[(0.0, 0.0, 1.0, 0.5)],
        "▄" => &[(0.0, 0.5, 1.0, 0.5)],
        "▟" => &[(0.5, 0.0, 0.5, 1.0), (0.0, 0.5, 0.5, 0.5)],
        "▙" => &[(0.0, 0.0, 0.5, 1.0), (0.5, 0.5, 0.5, 0.5)],
        "▜" => &[(0.0, 0.0, 1.0, 0.5), (0.5, 0.5, 0.5, 0.5)],
        "▛" => &[(0.0, 0.0, 1.0, 0.5), (0.0, 0.5, 0.5, 0.5)],
        _ => return None,
    })
}

/// An SVG fragment standing in for the Nerd Font character `ch` in a cell at (`px`, `py`),
/// or `None` when `ch` is ordinary text.
fn nerd_preview(ch: char, px: f32, py: f32, fg: u32, bg: u32) -> Option<String> {
    let (w, h) = (CELL_W, CELL_H);
    let color = format!("#{fg:06X}");
    let back = format!("#{bg:06X}");
    // Icon box, overflowing into the next cell like the font.
    let (ix, iy) = (px + 0.5, py + (h - ICON) / 2.0);
    let shape = |body: String| {
        Some(format!(
            "<g shape-rendering=\"geometricPrecision\">{body}</g>"
        ))
    };
    match ch {
        glyphs::CAP_LEFT => {
            return shape(format!(
                "<path d=\"M{x1:.1} {py:.1} A{w:.1} {r:.1} 0 0 0 {x1:.1} {y1:.1} Z\" fill=\"{color}\"/>",
                x1 = px + w + 0.3,
                r = h / 2.0,
                y1 = py + h
            ));
        }
        glyphs::CAP_RIGHT => {
            return shape(format!(
                "<path d=\"M{x0:.1} {py:.1} A{w:.1} {r:.1} 0 0 1 {x0:.1} {y1:.1} Z\" fill=\"{color}\"/>",
                x0 = px - 0.3,
                r = h / 2.0,
                y1 = py + h
            ));
        }
        glyphs::CHECKBOX_ON | glyphs::CHECKBOX_OFF | glyphs::CHECKBOX_MIXED => {
            let (x, y, s) = (ix + 1.5, iy + 1.5, ICON - 3.0);
            let mut body = if ch == glyphs::CHECKBOX_OFF {
                format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"2\" \
                     fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\"/>",
                    x + 0.75,
                    y + 0.75,
                    s - 1.5,
                    s - 1.5
                )
            } else {
                format!(
                    "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{s:.1}\" height=\"{s:.1}\" rx=\"2\" fill=\"{color}\"/>"
                )
            };
            if ch == glyphs::CHECKBOX_ON {
                let _ = write!(
                    body,
                    "<polyline points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"none\" \
                     stroke=\"{back}\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                    x + s * 0.22,
                    y + s * 0.52,
                    x + s * 0.42,
                    y + s * 0.72,
                    x + s * 0.78,
                    y + s * 0.3
                );
            } else if ch == glyphs::CHECKBOX_MIXED {
                let _ = write!(
                    body,
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{back}\" \
                     stroke-width=\"1.6\" stroke-linecap=\"round\"/>",
                    x + s * 0.25,
                    y + s / 2.0,
                    x + s * 0.75,
                    y + s / 2.0
                );
            }
            return shape(body);
        }
        glyphs::RADIO_ON | glyphs::RADIO_OFF => {
            let (cx, cy) = (ix + ICON / 2.0, iy + ICON / 2.0);
            let mut body = format!(
                "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"5.5\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\"/>"
            );
            if ch == glyphs::RADIO_ON {
                let _ = write!(
                    body,
                    "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"2.8\" fill=\"{color}\"/>"
                );
            }
            return shape(body);
        }
        _ => {}
    }
    let tones = Tones {
        ink: fg,
        knockout: bg,
        ..Tones::dark()
    };
    let icon = if let Some(glyph) = Glyph::ALL
        .iter()
        .copied()
        .find(|g| glyphs::nerd_font(*g) == ch)
    {
        glyph.to_svg(ICON, &tones)
    } else if let Some(kind) = SymbolKind::ALL
        .into_iter()
        .find(|k| glyphs::symbol(*k) == ch)
    {
        Symbol::new(kind).to_svg(ICON, &tones)
    } else {
        let language = Language::ALL
            .iter()
            .copied()
            .find(|l| glyphs::language(*l) == ch)?;
        parts_to_svg(
            &language.parts_styled(LanguageStyle::FileBand),
            ICON,
            &tones,
        )
    };
    Some(icon.replacen("<svg ", &format!("<svg x=\"{ix:.1}\" y=\"{iy:.1}\" "), 1))
}

/// `buf` as an SVG image.
pub fn to_svg(buf: &Buffer) -> String {
    let area = buf.area;
    let (w, h) = (area.width as f32 * CELL_W, area.height as f32 * CELL_H);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.1}\" height=\"{h:.1}\" \
         viewBox=\"0 0 {w:.1} {h:.1}\" font-family=\"'JetBrains Mono', 'DejaVu Sans Mono', monospace\" \
         font-size=\"14\" shape-rendering=\"crispEdges\">\n"
    );
    for y in area.top()..area.bottom() {
        let py = (y - area.y) as f32 * CELL_H;
        // Background runs.
        let mut x = area.left();
        while x < area.right() {
            let bg = hex(buf[(x, y)].bg, 0);
            let start = x;
            while x < area.right() && hex(buf[(x, y)].bg, 0) == bg {
                x += 1;
            }
            let _ = writeln!(
                svg,
                "<rect x=\"{:.1}\" y=\"{py:.1}\" width=\"{:.1}\" height=\"{CELL_H}\" fill=\"{bg}\"/>",
                (start - area.x) as f32 * CELL_W,
                (x - start) as f32 * CELL_W,
            );
        }
        // Foreground, one element per cell so every character stays on the grid.
        let mut x = area.left();
        while x < area.right() {
            let cell = &buf[(x, y)];
            let symbol = cell.symbol();
            let width = unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
            let px = (x - area.x) as f32 * CELL_W;
            let fg = rgb(cell.fg).unwrap_or(0xDFE1E5);
            let bg = rgb(cell.bg).unwrap_or(0);
            let mut chars = symbol.chars();
            let single = match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(ch),
                _ => None,
            };
            if let Some(rects) = block(symbol) {
                for (bx, by, bw, bh) in rects {
                    let _ = writeln!(
                        svg,
                        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#{fg:06X}\"/>",
                        px + bx * CELL_W,
                        py + by * CELL_H,
                        bw * CELL_W,
                        bh * CELL_H,
                    );
                }
            } else if let Some(preview) = single.and_then(|ch| nerd_preview(ch, px, py, fg, bg)) {
                svg.push_str(&preview);
                svg.push('\n');
            } else if !symbol.trim().is_empty() {
                let mut attrs = String::new();
                if cell.modifier.contains(Modifier::BOLD) {
                    attrs += " font-weight=\"bold\"";
                }
                if cell.modifier.contains(Modifier::UNDERLINED) {
                    attrs += " text-decoration=\"underline\"";
                }
                let _ = writeln!(
                    svg,
                    "<text x=\"{px:.1}\" y=\"{:.1}\" fill=\"#{fg:06X}\"{attrs}>{}</text>",
                    py + 13.5,
                    escape(symbol)
                );
            }
            x += width;
        }
    }
    svg.push_str("</svg>\n");
    svg
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/tui/img")
}

/// Compares `buf` as SVG with `docs/tui/img/<name>.svg`, or writes it when
/// `UPDATE_SNAPSHOTS` is set.
///
/// # Panics
///
/// When the snapshot is missing or differs.
pub fn snapshot(name: &str, buf: &Buffer) {
    let path = snapshot_dir().join(format!("{name}.svg"));
    let svg = to_svg(buf);
    let current = std::fs::read_to_string(&path).ok();
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        if current.as_deref() != Some(svg.as_str()) {
            std::fs::create_dir_all(snapshot_dir()).expect("create snapshot dir");
            std::fs::write(&path, svg).expect("write snapshot");
        }
        return;
    }
    assert!(
        current.as_deref() == Some(svg.as_str()),
        "{} changed; text now:\n{}\nrun UPDATE_SNAPSHOTS=1 cargo test -p archipel-kit-tui",
        path.display(),
        lines(buf).join("\n")
    );
}
