//! [`TextArea`]: a light read-only text view over lines the caller owns. For code, use the
//! [`EditorComponent`](crate::editor::EditorComponent), which runs on the kit's editor model.
//!
//! It draws like Neovim: a number column in grey with the current line bright, the cursor
//! line on `black2`, thin indent guides in the `line` color and a light syntax tint. It moves
//! a cursor with the arrows, Home, End, PageUp, PageDown and Ctrl+Home/End, scrolls with the
//! wheel and sets the cursor with a click. There is no text model here: the caller owns the
//! lines.

use crate::components::{fill, fill_rect, put, text_width};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "pub", "struct", "enum", "impl", "trait", "use", "mod", "match", "if",
    "else", "for", "while", "loop", "return", "self", "Self", "const", "static", "type", "where",
    "as", "in", "ref", "move", "true", "false", "def", "class", "import", "from", "function",
    "var", "async", "await",
];

/// The cursor and scroll state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextAreaState {
    /// The cursor line (0-based).
    pub line: usize,
    /// The cursor column (character index).
    pub col: usize,
    top: usize,
    page: usize,
    area: Rect,
    gutter: u16,
}

impl TextAreaState {
    /// A state at the top.
    pub fn new() -> Self {
        Self::default()
    }

    /// The first visible line.
    pub fn top(&self) -> usize {
        self.top
    }

    fn clamp(&mut self, lines: &[String]) {
        self.line = self.line.min(lines.len().saturating_sub(1));
        let len = lines.get(self.line).map_or(0, |l| l.chars().count());
        self.col = self.col.min(len);
    }

    fn reveal(&mut self) {
        let page = self.page.max(1);
        if self.line < self.top {
            self.top = self.line;
        } else if self.line >= self.top + page {
            self.top = self.line + 1 - page;
        }
    }

    /// Applies a key press; returns whether it moved the cursor.
    pub fn key(&mut self, code: KeyCode, mods: KeyModifiers, lines: &[String]) -> bool {
        let before = (self.line, self.col);
        let ctrl = mods.contains(KeyModifiers::CONTROL);
        let len = |l: usize| lines.get(l).map_or(0, |s| s.chars().count());
        match code {
            KeyCode::Up => self.line = self.line.saturating_sub(1),
            KeyCode::Down => self.line = (self.line + 1).min(lines.len().saturating_sub(1)),
            KeyCode::Left => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.line > 0 {
                    self.line -= 1;
                    self.col = len(self.line);
                }
            }
            KeyCode::Right => {
                if self.col < len(self.line) {
                    self.col += 1;
                } else if self.line + 1 < lines.len() {
                    self.line += 1;
                    self.col = 0;
                }
            }
            KeyCode::Home if ctrl => {
                self.line = 0;
                self.col = 0;
            }
            KeyCode::End if ctrl => {
                self.line = lines.len().saturating_sub(1);
                self.col = len(self.line);
            }
            KeyCode::Home => self.col = 0,
            KeyCode::End => self.col = len(self.line),
            KeyCode::PageUp => self.line = self.line.saturating_sub(self.page.max(2) - 1),
            KeyCode::PageDown => {
                self.line = (self.line + self.page.max(2) - 1).min(lines.len().saturating_sub(1));
            }
            _ => return false,
        }
        self.clamp(lines);
        self.reveal();
        before != (self.line, self.col)
    }
}

/// How a [`TextArea`] tints its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TextLanguage {
    /// Keywords, strings, numbers and comments of C-like languages.
    #[default]
    Code,
    /// Headings, code spans, links and list bullets.
    Markdown,
    /// No tint.
    Plain,
}

impl TextLanguage {
    /// The language for a file name by its extension.
    pub fn of_file(name: &str) -> Self {
        match name.rsplit('.').next() {
            Some("md" | "markdown") => TextLanguage::Markdown,
            Some("txt" | "lock") => TextLanguage::Plain,
            _ => TextLanguage::Code,
        }
    }
}

/// The read-only text view.
#[derive(Debug, Clone)]
pub struct TextArea {
    /// The identity.
    pub id: Id,
    /// Whether to tint the text at all.
    pub syntax: bool,
    /// The tint.
    pub language: TextLanguage,
}

impl TextArea {
    /// A view.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            syntax: true,
            language: TextLanguage::Code,
        }
    }

    /// Sets the language.
    #[must_use]
    pub fn language(mut self, language: TextLanguage) -> Self {
        self.language = language;
        self
    }

    /// Draws `lines` in `area`.
    pub fn show(
        &self,
        rt: &mut Runtime,
        state: &mut TextAreaState,
        lines: &[String],
        area: Rect,
        buf: &mut Buffer,
    ) {
        let p = rt.look.palette;
        let focused = rt.focused(self.id);
        state.area = area;
        state.page = area.height as usize;
        state.clamp(lines);
        let max_top = lines.len().saturating_sub(area.height as usize);
        state.top = state.top.min(max_top);
        let digits = lines.len().max(1).to_string().len() as u16;
        let gutter = (digits + 2).min(area.width);
        state.gutter = gutter;
        fill_rect(buf, area, Style::default().bg(p.black).fg(p.white));
        rt.widget(self.id, area);
        for row in 0..area.height {
            let index = state.top + row as usize;
            let Some(text) = lines.get(index) else {
                break;
            };
            let y = area.y + row;
            let current = index == state.line;
            let bg = if current { p.black2 } else { p.black };
            fill(buf, area, area.x, y, area.width, Style::default().bg(bg));
            let number = format!("{:>width$} ", index + 1, width = digits as usize);
            put(
                buf,
                area,
                area.x,
                y,
                &number,
                Style::default()
                    .fg(if current { p.white } else { p.grey_fg })
                    .bg(bg)
                    .add_modifier(if current {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            );
            let text_x = area.x + gutter + 1;
            let room = area.right().saturating_sub(text_x);
            self.paint_line(&rt.look, buf, area, text_x, y, text, bg, room);
            // Indent guides at every fourth column of the leading whitespace.
            let indent = text.chars().take_while(|c| *c == ' ').count();
            for level in (4..=indent).step_by(4) {
                put(
                    buf,
                    area,
                    text_x + level as u16 - 4,
                    y,
                    "│",
                    Style::default().fg(p.line).bg(bg),
                );
            }
            if current && focused {
                let cx = text_x + state.col as u16;
                if cx < area.right() {
                    let ch = text.chars().nth(state.col).unwrap_or(' ');
                    put(
                        buf,
                        area,
                        cx,
                        y,
                        ch.encode_utf8(&mut [0; 4]),
                        Style::default().fg(p.black).bg(p.white),
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_line(
        &self,
        look: &crate::Look,
        buf: &mut Buffer,
        area: Rect,
        x: u16,
        y: u16,
        text: &str,
        bg: Color,
        room: u16,
    ) {
        let p = look.palette;
        let chars: Vec<char> = text.chars().collect();
        if self.syntax && self.language == TextLanguage::Markdown {
            self.paint_markdown(look, buf, area, x, y, text, bg, room);
            return;
        }
        let mut i = 0usize;
        let mut cx = x;
        let limit = x.saturating_add(room);
        let mut emit = |cx: &mut u16, s: &str, color: Color, bold: bool| {
            if *cx < limit {
                let st = Style::default().fg(color).bg(bg).add_modifier(if bold {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });
                *cx = put(
                    buf,
                    Rect::new(
                        area.x,
                        area.y,
                        limit.min(area.right()) - area.x,
                        area.height,
                    ),
                    *cx,
                    y,
                    s,
                    st,
                );
            }
        };
        while i < chars.len() {
            let c = chars[i];
            if !self.syntax || self.language == TextLanguage::Plain {
                emit(&mut cx, &c.to_string(), p.white, false);
                i += 1;
                continue;
            }
            if c == '/' && chars.get(i + 1) == Some(&'/')
                || c == '#' && !chars[..i].iter().any(|c| !c.is_whitespace())
            {
                let rest: String = chars[i..].iter().collect();
                emit(&mut cx, &rest, p.light_grey, false);
                break;
            }
            if c == '"' {
                let end = chars[i + 1..]
                    .iter()
                    .position(|c| *c == '"')
                    .map_or(chars.len(), |e| i + 2 + e);
                let s: String = chars[i..end.min(chars.len())].iter().collect();
                emit(&mut cx, &s, p.green, false);
                i = end.min(chars.len());
                continue;
            }
            if c.is_ascii_digit() {
                let end = chars[i..]
                    .iter()
                    .position(|c| !c.is_ascii_alphanumeric() && *c != '.' && *c != '_')
                    .map_or(chars.len(), |e| i + e);
                let s: String = chars[i..end].iter().collect();
                emit(&mut cx, &s, p.orange, false);
                i = end;
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                let end = chars[i..]
                    .iter()
                    .position(|c| !c.is_alphanumeric() && *c != '_')
                    .map_or(chars.len(), |e| i + e);
                let word: String = chars[i..end].iter().collect();
                let color = if KEYWORDS.contains(&word.as_str()) {
                    p.purple
                } else if word.chars().next().is_some_and(char::is_uppercase) {
                    p.yellow
                } else if chars.get(end) == Some(&'(') {
                    p.blue
                } else {
                    p.white
                };
                emit(&mut cx, &word, color, false);
                i = end;
                continue;
            }
            emit(&mut cx, &c.to_string(), p.light_grey, false);
            i += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_markdown(
        &self,
        look: &crate::Look,
        buf: &mut Buffer,
        area: Rect,
        x: u16,
        y: u16,
        text: &str,
        bg: Color,
        room: u16,
    ) {
        let p = look.palette;
        let clip = Rect::new(
            area.x,
            area.y,
            x.saturating_add(room).min(area.right()) - area.x,
            area.height,
        );
        let trimmed = text.trim_start();
        let heading = trimmed.starts_with('#');
        let bullet =
            trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("| ");
        let mut cx = x;
        let mut in_code = false;
        let indent = text.len() - trimmed.len();
        for (at, c) in text.chars().enumerate() {
            let (color, bold) = if heading {
                (p.blue, true)
            } else if c == '`' {
                in_code = !in_code;
                (p.green, false)
            } else if in_code {
                (p.green, false)
            } else if bullet && at == indent {
                (p.orange, true)
            } else if c == '[' || c == ']' || c == '(' || c == ')' {
                (p.cyan, false)
            } else {
                (p.white, false)
            };
            let style = Style::default().fg(color).bg(bg).add_modifier(if bold {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
            cx = put(buf, clip, cx, y, c.encode_utf8(&mut [0; 4]), style);
        }
    }

    /// Applies `event`; returns true when the cursor moved or the view scrolled.
    pub fn handle(
        &self,
        rt: &mut Runtime,
        state: &mut TextAreaState,
        lines: &[String],
        event: &Event,
    ) -> bool {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release && rt.focused(self.id) => {
                state.key(key.code, key.modifiers, lines)
            }
            Event::Mouse(_) => {
                let Some(p) = rt.pointer().copied() else {
                    return false;
                };
                match p.kind {
                    PointerKind::Down(MouseButton::Left) if p.over(self.id) => {
                        rt.focus.focus(self.id);
                        let row = (p.y.saturating_sub(state.area.y)) as usize;
                        state.line = (state.top + row).min(lines.len().saturating_sub(1));
                        let text_x = state.area.x + state.gutter + 1;
                        state.col = (p.x.saturating_sub(text_x)) as usize;
                        state.clamp(lines);
                        true
                    }
                    PointerKind::ScrollUp | PointerKind::ScrollDown if p.over(self.id) => {
                        let max = lines.len().saturating_sub(state.page);
                        let next = (state.top as i32 + p.wheel() * 3).clamp(0, max as i32) as usize;
                        let moved = next != state.top;
                        state.top = next;
                        moved
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

/// The cells a line of `text` takes, for callers that size panes to their text.
pub fn line_width(text: &str) -> u16 {
    text_width(text)
}
