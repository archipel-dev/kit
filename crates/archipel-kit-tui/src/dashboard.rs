//! [`Dashboard`]: the start screen shown while no file is open (NvChad's nvdash): a centered
//! header and a column of key-hinted buttons.

use crate::components::{fill_rect, put, text_width};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crate::runtime::Runtime;
use crate::{Glyph, Look};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// One dashboard button.
#[derive(Debug, Clone)]
pub struct DashboardButton {
    /// The icon.
    pub icon: Glyph,
    /// The label.
    pub label: String,
    /// The key that triggers it from the dashboard, shown at the right.
    pub key: String,
    /// The character that triggers it (plain key press).
    pub trigger: char,
}

impl DashboardButton {
    /// A button.
    pub fn new(
        icon: Glyph,
        label: impl Into<String>,
        key: impl Into<String>,
        trigger: char,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            key: key.into(),
            trigger,
        }
    }
}

/// The dashboard.
#[derive(Debug, Clone)]
pub struct Dashboard {
    /// The identity; buttons use child ids.
    pub id: Id,
    /// The header lines.
    pub header: Vec<String>,
    /// The buttons.
    pub buttons: Vec<DashboardButton>,
}

impl Dashboard {
    /// A dashboard.
    pub fn new(id: Id, header: Vec<String>, buttons: Vec<DashboardButton>) -> Self {
        Self {
            id,
            header,
            buttons,
        }
    }

    fn button_width(&self) -> u16 {
        self.buttons
            .iter()
            .map(|b| text_width(&b.label) + text_width(&b.key) + 10)
            .max()
            .unwrap_or(20)
            .max(28)
    }

    /// Draws the dashboard centered in `area`.
    pub fn show(&self, rt: &mut Runtime, look: &Look, area: Rect, buf: &mut Buffer) {
        let p = look.palette;
        fill_rect(buf, area, Style::default().bg(p.black).fg(p.white));
        let head_h = self.header.len() as u16;
        let total = head_h + 2 + self.buttons.len() as u16 * 2;
        let top = area.y + area.height.saturating_sub(total) / 2;
        for (i, line) in self.header.iter().enumerate() {
            let w = text_width(line);
            let x = area.x + area.width.saturating_sub(w) / 2;
            put(
                buf,
                area,
                x,
                top + i as u16,
                line,
                Style::default().fg(p.nord_blue).bg(p.black),
            );
        }
        let bw = self.button_width().min(area.width);
        let bx = area.x + area.width.saturating_sub(bw) / 2;
        for (i, button) in self.buttons.iter().enumerate() {
            let y = top + head_h + 2 + i as u16 * 2;
            let rect = Rect::new(bx, y, bw, 1).intersection(area);
            if rect.is_empty() {
                continue;
            }
            let id = self.id.with(i as u64);
            let hover = rt.hovered() == Some(id);
            let bg = if hover { p.one_bg } else { p.black2 };
            crate::components::fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let x = put(
                buf,
                rect,
                rect.x + 2,
                y,
                &look.glyph(button.icon),
                Style::default().fg(p.nord_blue).bg(bg),
            );
            put(
                buf,
                rect,
                x + 2,
                y,
                &button.label,
                Style::default().fg(p.white).bg(bg),
            );
            let kx = rect.right().saturating_sub(text_width(&button.key) + 2);
            put(
                buf,
                rect,
                kx,
                y,
                &button.key,
                Style::default()
                    .fg(p.grey_fg)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            );
            rt.clickable(id, rect);
        }
    }

    /// Applies `event`; returns the index of the pressed button (a click, or its trigger key).
    pub fn handle(&self, rt: &Runtime, event: &Event) -> Option<usize> {
        match event {
            Event::Key(k)
                if k.kind != KeyEventKind::Release
                    && !k
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let KeyCode::Char(c) = k.code else {
                    return None;
                };
                self.buttons.iter().position(|b| b.trigger == c)
            }
            Event::Mouse(_) => (0..self.buttons.len()).find(|i| {
                rt.pointer()
                    .is_some_and(|p| p.clicked(self.id.with(*i as u64)))
            }),
            _ => None,
        }
    }
}
