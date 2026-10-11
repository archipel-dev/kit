//! The TUI gallery: every component in a terminal.
//!
//! `cargo run -p archipel-kit-tui --example gallery`
//!
//! Keys: `←`/`→` or `h`/`l` pages, `↑`/`↓` or `j`/`k` move, `space` toggles,
//! `t` dark/light, `q` quits. Use a Nerd Font in the terminal (JetBrainsMono Nerd Font).

use archipel_kit::components::Severity;
use archipel_kit_tui::components::*;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::{Glyph, Language, Look, Symbol, SymbolKind};

const PAGES: [&str; 4] = ["Controls", "Lists", "Feedback", "Icons"];

struct App {
    look: Look,
    page: usize,
    row: usize,
    checks: [bool; 3],
    toggle: bool,
    radio: usize,
}

impl App {
    fn draw(&self, area: Rect, buf: &mut Buffer) {
        let look = &self.look;
        Frame::new(look).render(area, buf);
        if area.width < 20 || area.height < 8 {
            return;
        }
        // Header island with the page switcher.
        let header = Rect::new(1, 1, area.width - 2, 3);
        Island::new(look).render(header, buf);
        buf.set_string(3, 2, "Archipel Kit", look.strong());
        SegmentedControl::new(look, &PAGES, self.page).render(Rect::new(17, 2, 50, 1), buf);
        Kbd::new(look, &["t", "q"]).render(Rect::new(area.width.saturating_sub(15), 2, 13, 1), buf);
        let body = Rect::new(1, 5, area.width - 2, area.height - 6);
        let island = Island::new(look).title(PAGES[self.page]);
        let inner = island.inner(body);
        island.render(body, buf);
        let row = |n: u16| Rect::new(inner.x, inner.y + n, inner.width, 1);
        match self.page {
            0 => {
                let mut x = inner.x;
                for button in [
                    Button::primary(look, "Run").icon(Glyph::Run),
                    Button::secondary(look, "Cancel"),
                    Button::ghost(look, "Details"),
                    Button::danger(look, "Delete"),
                    Button::primary(look, "Disabled").enabled(false),
                ] {
                    let width = button.width();
                    button.render(Rect::new(x, inner.y, width, 1), buf);
                    x += width + 1;
                }
                let labels = ["Format on save", "Optimize imports", "Mixed"];
                for (i, label) in labels.iter().enumerate() {
                    Checkbox::new(look, self.checks[i], label)
                        .mixed(i == 2 && !self.checks[2])
                        .focused(self.row == i)
                        .render(row(2 + i as u16), buf);
                }
                for (i, label) in ["Spaces", "Tabs"].iter().enumerate() {
                    Radio::new(look, self.radio == i, label).render(row(6 + i as u16), buf);
                }
                Toggle::new(look, self.toggle, "Power save mode").render(row(9), buf);
                TextField::new(look, "")
                    .placeholder("Search everywhere")
                    .render(row(11), buf);
                TextField::new(look, "8o80")
                    .label("Port")
                    .error(Some("Must be a number"))
                    .render(
                        Rect::new(inner.x, inner.y + 13, inner.width.min(40), 3),
                        buf,
                    );
            }
            1 => {
                let tabs = [
                    (Some(Language::Rust.into()), "main.rs", false),
                    (Some(Language::Toml.into()), "Cargo.toml", true),
                    (Some(Glyph::FileText.into()), "README.md", false),
                ];
                TabBar::new(look, &tabs, self.row % 3)
                    .render(Rect::new(inner.x, inner.y, inner.width, 1), buf);
                let rows: [(&str, u16, Option<bool>, RowIcon); 8] = [
                    ("kit", 0, Some(true), Glyph::Folder.into()),
                    ("src", 1, Some(true), Glyph::Folder.into()),
                    ("main.rs", 2, None, Language::Rust.into()),
                    ("theme.py", 2, None, Language::Python.into()),
                    ("Cargo.toml", 1, None, Language::Toml.into()),
                    (
                        "Theme",
                        0,
                        Some(true),
                        Symbol::new(SymbolKind::Class).into(),
                    ),
                    (
                        "apply(ctx)",
                        1,
                        None,
                        Symbol::new(SymbolKind::Method).into(),
                    ),
                    ("colors", 1, None, Symbol::new(SymbolKind::Field).into()),
                ];
                for (i, (label, depth, expanded, icon)) in rows.into_iter().enumerate() {
                    ListRow::new(look, label)
                        .depth(depth)
                        .expanded(expanded)
                        .icon(icon)
                        .selected(self.row % rows_len() == i)
                        .render(
                            Rect::new(body.x, inner.y + 3 + i as u16, body.width, 1),
                            buf,
                        );
                }
            }
            2 => {
                let severities = [
                    (Severity::Info, "Indexing finished"),
                    (Severity::Success, "Build succeeded"),
                    (Severity::Warning, "Low disk space"),
                    (Severity::Error, "Connection lost"),
                ];
                for (i, (severity, text)) in severities.into_iter().enumerate() {
                    Banner::new(look, severity, text, &["Retry", "Dismiss"])
                        .render(row(2 * i as u16), buf);
                }
                ProgressBar::new(look, 0.6).render(row(9), buf);
                let mut x = inner.x;
                for chip in [Chip::tag(look, "draft"), Chip::counter(look, "12", true)] {
                    chip.render(Rect::new(x, inner.y + 11, 10, 1), buf);
                    x += 9;
                }
            }
            _ => {
                let mut x = inner.x;
                let mut y = inner.y;
                for language in Language::ALL.iter().copied() {
                    if x + 2 > inner.right() {
                        x = inner.x;
                        y += 1;
                    }
                    LanguageIcon::new(look, language).render(Rect::new(x, y, 3, 1), buf);
                    x += 3;
                }
                let mut x = inner.x;
                y += 2;
                for kind in SymbolKind::ALL {
                    SymbolIcon::new(look, Symbol::new(kind)).render(Rect::new(x, y, 3, 1), buf);
                    x += 3;
                }
                let (mut x, mut y) = (inner.x, y + 2);
                for glyph in Glyph::ALL.iter().copied() {
                    if x + 2 > inner.right() {
                        x = inner.x;
                        y += 1;
                    }
                    buf.set_string(
                        x,
                        y,
                        look.glyph(glyph),
                        look.body().fg(look.glyph_color(glyph)),
                    );
                    x += 3;
                }
            }
        }
    }

    fn key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                self.page = (self.page + 1) % PAGES.len();
                self.row = 0;
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => {
                self.page = (self.page + PAGES.len() - 1) % PAGES.len();
                self.row = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.row += 1,
            KeyCode::Up | KeyCode::Char('k') => self.row = self.row.saturating_sub(1),
            KeyCode::Char(' ') | KeyCode::Enter if self.page == 0 => {
                if let Some(check) = self.checks.get_mut(self.row) {
                    *check = !*check;
                } else {
                    self.toggle = !self.toggle;
                    self.radio = 1 - self.radio;
                }
            }
            KeyCode::Char('t') => {
                self.look = if self.look.theme.dark {
                    Look::light()
                } else {
                    Look::dark()
                };
            }
            _ => {}
        }
        true
    }
}

fn rows_len() -> usize {
    8
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        look: Look::dark(),
        page: 0,
        row: 0,
        checks: [true, false, false],
        toggle: true,
        radio: 0,
    };
    let mut terminal = archipel_kit_tui::ratatui::init();
    let result = loop {
        if let Err(error) = terminal.draw(|frame| app.draw(frame.area(), frame.buffer_mut())) {
            break Err(error);
        }
        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                if !app.key(key.code) {
                    break Ok(());
                }
            }
            Ok(_) => {}
            Err(error) => break Err(error),
        }
    };
    archipel_kit_tui::ratatui::restore();
    result
}
