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

use crate::Look;
use crate::glyphs;
use crate::input::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crate::runtime::{App, Runtime};
use archipel_icons::{Glyph, Language, LanguageStyle, Symbol, SymbolKind, Tones, parts_to_svg};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Cell size in SVG pixels.
const CELL_W: f32 = 8.4;
const CELL_H: f32 = 18.0;
/// Icon size: Nerd Font icons are about 1.5 cells wide and draw into the next cell.
const ICON: f32 = 12.5;

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

/// The box-drawing character `symbol` as an SVG stroke path, so lines join across cells
/// as they do in a terminal.
fn box_path(symbol: &str, px: f32, py: f32) -> Option<(String, f32)> {
    let (w, h) = (CELL_W, CELL_H);
    let (cx, cy) = (px + w / 2.0, py + h / 2.0);
    let (r, b) = (px + w, py + h);
    let r4 = w / 2.0;
    Some(match symbol {
        "─" => (format!("M{px:.1} {cy:.1}H{r:.1}"), 1.0),
        "━" => (format!("M{px:.1} {cy:.1}H{r:.1}"), 2.5),
        "│" => (format!("M{cx:.1} {py:.1}V{b:.1}"), 1.0),
        "▏" => (
            format!("M{:.1} {:.1}V{:.1}", px + 1.0, py + 2.0, b - 2.0),
            1.5,
        ),
        "▔" => (format!("M{px:.1} {:.1}H{r:.1}", py + 1.0), 2.0),
        "╭" => (
            format!(
                "M{r:.1} {cy:.1}H{:.1}A{r4:.1} {r4:.1} 0 0 0 {cx:.1} {:.1}V{b:.1}",
                cx + r4,
                cy + r4
            ),
            1.0,
        ),
        "╮" => (
            format!(
                "M{px:.1} {cy:.1}H{:.1}A{r4:.1} {r4:.1} 0 0 1 {cx:.1} {:.1}V{b:.1}",
                cx - r4,
                cy + r4
            ),
            1.0,
        ),
        "╰" => (
            format!(
                "M{cx:.1} {py:.1}V{:.1}A{r4:.1} {r4:.1} 0 0 0 {:.1} {cy:.1}H{r:.1}",
                cy - r4,
                cx + r4
            ),
            1.0,
        ),
        "╯" => (
            format!(
                "M{cx:.1} {py:.1}V{:.1}A{r4:.1} {r4:.1} 0 0 1 {:.1} {cy:.1}H{px:.1}",
                cy - r4,
                cx - r4
            ),
            1.0,
        ),
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
        glyphs::TOGGLE_ON | glyphs::TOGGLE_OFF => {
            let (x, y) = (ix, iy + 3.0);
            let (tw, th) = (ICON + 2.0, ICON - 6.0);
            let body = if ch == glyphs::TOGGLE_ON {
                format!(
                    "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{tw:.1}\" height=\"{th:.1}\" rx=\"{r:.1}\" fill=\"{color}\"/>\
                     <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{back}\"/>",
                    x + tw - th / 2.0,
                    y + th / 2.0,
                    th / 2.0 - 1.5,
                    r = th / 2.0
                )
            } else {
                format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{r:.1}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.3\"/>\
                     <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{color}\"/>",
                    x + 0.65,
                    y + 0.65,
                    tw - 1.3,
                    th - 1.3,
                    x + th / 2.0,
                    y + th / 2.0,
                    th / 2.0 - 2.0,
                    r = th / 2.0 - 0.65
                )
            };
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

/// The foreground and background of a cell as a terminal draws them: swapped when reversed.
fn cell_colors(cell: &ratatui::buffer::Cell) -> (Color, Color) {
    if cell.modifier.contains(Modifier::REVERSED) {
        (cell.bg, cell.fg)
    } else {
        (cell.fg, cell.bg)
    }
}

/// `buf` as an SVG image.
pub fn to_svg(buf: &Buffer) -> String {
    to_svg_with_cursor(buf, None)
}

/// `buf` as an SVG image with the terminal cursor drawn where `cursor` says: a bar, a block or
/// an underline over the cell.
pub fn to_svg_with_cursor(buf: &Buffer, cursor: Option<crate::runtime::CursorRequest>) -> String {
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
            let bg = hex(cell_colors(&buf[(x, y)]).1, 0);
            let start = x;
            while x < area.right() && hex(cell_colors(&buf[(x, y)]).1, 0) == bg {
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
            let (cell_fg, cell_bg) = cell_colors(cell);
            let fg = rgb(cell_fg).unwrap_or(0xDFE1E5);
            let bg = rgb(cell_bg).unwrap_or(0);
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
            } else if let Some((d, stroke)) = box_path(symbol, px, py) {
                let _ = writeln!(
                    svg,
                    "<path d=\"{d}\" fill=\"none\" stroke=\"#{fg:06X}\" stroke-width=\"{stroke}\" shape-rendering=\"geometricPrecision\"/>"
                );
            } else if let Some(preview) = single.and_then(|ch| nerd_preview(ch, px, py, fg, bg)) {
                svg.push_str(&preview);
                svg.push('\n');
            } else if !symbol.trim().is_empty() {
                let mut attrs = String::new();
                if cell.modifier.contains(Modifier::BOLD) {
                    attrs += " font-weight=\"bold\"";
                }
                if cell.modifier.contains(Modifier::ITALIC) {
                    attrs += " font-style=\"italic\"";
                }
                if cell.modifier.contains(Modifier::UNDERLINED)
                    && rgb(cell.underline_color).is_none()
                {
                    attrs += " text-decoration=\"underline\"";
                }
                let _ = writeln!(
                    svg,
                    "<text x=\"{px:.1}\" y=\"{:.1}\" fill=\"#{fg:06X}\"{attrs}>{}</text>",
                    py + 13.5,
                    escape(symbol)
                );
            }
            if cell.modifier.contains(Modifier::UNDERLINED)
                && let Some(color) = rgb(cell.underline_color)
            {
                // Undercurl: one wave period per cell, in the underline color.
                let y = py + CELL_H - 2.5;
                let _ = writeln!(
                    svg,
                    "<polyline points=\"{:.1},{y:.1} {:.1},{:.1} {:.1},{y:.1} {:.1},{:.1} {:.1},{y:.1}\" \
                     fill=\"none\" stroke=\"#{color:06X}\" stroke-width=\"1.2\" \
                     shape-rendering=\"geometricPrecision\"/>",
                    px,
                    px + CELL_W * 0.25,
                    y - 1.5,
                    px + CELL_W * 0.5,
                    px + CELL_W * 0.75,
                    y + 1.5,
                    px + CELL_W,
                );
            }
            x += width;
        }
    }
    if let Some(cursor) = cursor
        && cursor.x >= area.left()
        && cursor.x < area.right()
        && cursor.y >= area.top()
        && cursor.y < area.bottom()
    {
        let px = (cursor.x - area.x) as f32 * CELL_W;
        let py = (cursor.y - area.y) as f32 * CELL_H;
        let (x, y, w, h, opacity) = match cursor.shape {
            crate::runtime::CursorShape::Bar => (px, py + 1.0, 1.6, CELL_H - 2.0, 0.95),
            crate::runtime::CursorShape::Block => (px, py, CELL_W, CELL_H, 0.7),
            _ => (px, py + CELL_H - 2.5, CELL_W, 1.6, 0.95),
        };
        let _ = writeln!(
            svg,
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#DFE1E5\" fill-opacity=\"{opacity}\"/>"
        );
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
    snapshot_with_cursor(name, buf, None);
}

/// Like [`snapshot`], with the terminal cursor drawn at `cursor` (what a terminal shows for
/// [`Runtime::cursor`](crate::runtime::Runtime::cursor)).
///
/// # Panics
///
/// When the snapshot is missing or differs.
pub fn snapshot_with_cursor(
    name: &str,
    buf: &Buffer,
    cursor: Option<crate::runtime::CursorRequest>,
) {
    let path = snapshot_dir().join(format!("{name}.svg"));
    let svg = to_svg_with_cursor(buf, cursor);
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

/// Writes `text` at (`x`, `y`) in `fg` on `bg`, clipped to `area`: for test apps that label the
/// areas they got.
pub fn put_text(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
) {
    crate::components::put(
        buf,
        area,
        x,
        y,
        text,
        ratatui::style::Style::default().fg(fg).bg(bg),
    );
}

/// An [`App`] made of closures over some state, for tests and small examples.
pub struct FnApp<S, E, D> {
    /// The state the closures share.
    pub state: S,
    on_event: E,
    on_draw: D,
}

/// An [`App`] from `state`, an event handler and a draw function.
pub fn app_fn<S, E, D>(state: S, on_event: E, on_draw: D) -> FnApp<S, E, D>
where
    E: FnMut(&mut S, &mut Runtime, &Event),
    D: FnMut(&mut S, &mut Runtime, Rect, &mut Buffer),
{
    FnApp {
        state,
        on_event,
        on_draw,
    }
}

impl<S, E, D> App for FnApp<S, E, D>
where
    E: FnMut(&mut S, &mut Runtime, &Event),
    D: FnMut(&mut S, &mut Runtime, Rect, &mut Buffer),
{
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        (self.on_event)(&mut self.state, rt, event);
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        (self.on_draw)(&mut self.state, rt, area, buf);
    }
}

/// Runs an [`App`] against an in-memory screen with synthetic events and a manual clock: the
/// same code path as [`crate::runtime::run`], without a terminal. Every input redraws, as the
/// real loop does, so the hit map is always the one of the last frame.
pub struct Harness<A: App> {
    /// The app under test.
    pub app: A,
    /// The runtime: look, keymap, clock, hit map, focus.
    pub rt: Runtime,
    buffer: Buffer,
}

impl<A: App> Harness<A> {
    /// An app on a `width` × `height` screen in the dark look.
    pub fn new(app: A, width: u16, height: u16) -> Self {
        Self::with_runtime(app, Runtime::new(Look::dark()), width, height)
    }

    /// An app with a prepared runtime.
    pub fn with_runtime(app: A, rt: Runtime, width: u16, height: u16) -> Self {
        let mut harness = Self {
            app,
            rt,
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        };
        harness.draw();
        harness
    }

    /// Draws a frame and returns the screen.
    pub fn draw(&mut self) -> &Buffer {
        let area = self.buffer.area;
        self.buffer = Buffer::empty(area);
        self.rt.begin_frame(area);
        self.app.draw(&mut self.rt, area, &mut self.buffer);
        let look = self.rt.look.clone();
        self.rt.tooltip.draw(&look, area, &mut self.buffer);
        self.rt.end_frame();
        &self.buffer
    }

    /// The last drawn screen.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Compares the last drawn screen, with the terminal cursor the app asked for, against the
    /// snapshot `name` (see [`snapshot`]).
    pub fn snapshot(&self, name: &str) {
        snapshot_with_cursor(name, &self.buffer, self.rt.cursor);
    }

    /// The text of each screen row, trailing spaces trimmed.
    pub fn screen(&self) -> Vec<String> {
        lines(&self.buffer)
    }

    /// The screen as one string, rows joined by newlines.
    pub fn text(&self) -> String {
        self.screen().join("\n")
    }

    /// The cell where `needle` first appears on the screen.
    pub fn find(&self, needle: &str) -> Option<(u16, u16)> {
        for (y, line) in self.screen().iter().enumerate() {
            if let Some(byte) = line.find(needle) {
                let x = unicode_width::UnicodeWidthStr::width(&line[..byte]);
                return Some((x as u16, y as u16));
            }
        }
        None
    }

    /// Sends `event` (preprocessed like the real loop does) and redraws.
    pub fn send(&mut self, event: Event) -> &mut Self {
        self.rt.preprocess(&event);
        self.app.event(&mut self.rt, &event);
        self.draw();
        self
    }

    /// A key press.
    pub fn press(&mut self, code: KeyCode) -> &mut Self {
        self.send(crate::input::key(code))
    }

    /// A key press with modifiers.
    pub fn press_with(&mut self, code: KeyCode, mods: KeyModifiers) -> &mut Self {
        self.send(crate::input::key_with(code, mods))
    }

    /// Ctrl plus a character.
    pub fn ctrl(&mut self, c: char) -> &mut Self {
        self.send(crate::input::ctrl(c))
    }

    /// Alt plus a navigation key.
    pub fn alt_arrow(&mut self, code: KeyCode) -> &mut Self {
        self.send(crate::input::key_with(code, KeyModifiers::ALT))
    }

    /// Alt plus a character.
    pub fn alt(&mut self, c: char) -> &mut Self {
        self.send(crate::input::alt(c))
    }

    /// Types `text`, one key press per character.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for event in crate::input::typing(text) {
            self.send(event);
        }
        self
    }

    fn mouse(&mut self, kind: MouseEventKind, x: u16, y: u16, mods: KeyModifiers) -> &mut Self {
        self.send(Event::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: mods,
        }))
    }

    /// Moves the pointer.
    pub fn hover(&mut self, x: u16, y: u16) -> &mut Self {
        self.mouse(MouseEventKind::Moved, x, y, KeyModifiers::NONE)
    }

    /// Presses and releases the left button.
    pub fn click(&mut self, x: u16, y: u16) -> &mut Self {
        self.click_with(x, y, KeyModifiers::NONE)
    }

    /// A left click with modifiers held.
    pub fn click_with(&mut self, x: u16, y: u16, mods: KeyModifiers) -> &mut Self {
        self.mouse(MouseEventKind::Down(MouseButton::Left), x, y, mods);
        self.mouse(MouseEventKind::Up(MouseButton::Left), x, y, mods)
    }

    /// Two left clicks in a row (no time passes between them).
    pub fn double_click(&mut self, x: u16, y: u16) -> &mut Self {
        self.click(x, y);
        self.click(x, y)
    }

    /// A click of the middle button.
    pub fn middle_click(&mut self, x: u16, y: u16) -> &mut Self {
        self.mouse(
            MouseEventKind::Down(MouseButton::Middle),
            x,
            y,
            KeyModifiers::NONE,
        );
        self.mouse(
            MouseEventKind::Up(MouseButton::Middle),
            x,
            y,
            KeyModifiers::NONE,
        )
    }

    /// A click of the right button (a context menu request).
    pub fn right_click(&mut self, x: u16, y: u16) -> &mut Self {
        self.mouse(
            MouseEventKind::Down(MouseButton::Right),
            x,
            y,
            KeyModifiers::NONE,
        );
        self.mouse(
            MouseEventKind::Up(MouseButton::Right),
            x,
            y,
            KeyModifiers::NONE,
        )
    }

    /// Presses the left button at `from`, drags to `to` in single steps, and releases.
    pub fn drag(&mut self, from: (u16, u16), to: (u16, u16)) -> &mut Self {
        self.mouse(
            MouseEventKind::Down(MouseButton::Left),
            from.0,
            from.1,
            KeyModifiers::NONE,
        );
        let steps = from.0.abs_diff(to.0).max(from.1.abs_diff(to.1)).max(1);
        for i in 1..=steps {
            let lerp = |a: u16, b: u16| {
                (i32::from(a) + (i32::from(b) - i32::from(a)) * i32::from(i) / i32::from(steps))
                    as u16
            };
            self.mouse(
                MouseEventKind::Drag(MouseButton::Left),
                lerp(from.0, to.0),
                lerp(from.1, to.1),
                KeyModifiers::NONE,
            );
        }
        self.mouse(
            MouseEventKind::Up(MouseButton::Left),
            to.0,
            to.1,
            KeyModifiers::NONE,
        )
    }

    /// Turns the wheel at (`x`, `y`): negative scrolls up.
    pub fn scroll(&mut self, x: u16, y: u16, rows: i32) -> &mut Self {
        let kind = if rows < 0 {
            MouseEventKind::ScrollUp
        } else {
            MouseEventKind::ScrollDown
        };
        for _ in 0..rows.unsigned_abs() {
            self.mouse(kind, x, y, KeyModifiers::NONE);
        }
        self
    }

    /// Lets `seconds` pass: advances the clock, runs the timers and redraws.
    pub fn advance(&mut self, seconds: f64) -> &mut Self {
        self.rt.now += seconds;
        self.rt.tick();
        self.app.tick(&mut self.rt);
        self.draw();
        self
    }

    /// Resizes the screen.
    pub fn resize(&mut self, width: u16, height: u16) -> &mut Self {
        self.buffer = Buffer::empty(Rect::new(0, 0, width, height));
        self.send(Event::Resize(width, height))
    }
}
