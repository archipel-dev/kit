//! [`BreadcrumbsControl`]: [`model::Breadcrumbs`] as one row of text (TUI-GUI-094): crumbs
//! separated by `›`, an icon before a crumb that has one, shortened from the left with `…`
//! when the room runs out, never dropping the last crumb. A click on a crumb reports it.

use crate::Glyph;
use crate::chrome::{self, Placed, Tokens};
use crate::components::{fill, put, text_width};
use crate::hit::Id;
use crate::input::Event;
use crate::runtime::Runtime;
use archipel_kit::egui::Color32;
use archipel_kit_model::{BreadcrumbEvent, Breadcrumbs, Outline, OutlineEntry};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

/// Cells around the separator: ` › `.
const SEPARATOR_W: u16 = 3;

fn crumb_width(crumb: &archipel_kit_model::Crumb) -> u16 {
    text_width(&crumb.text) + if crumb.icon.is_some() { 2 } else { 0 }
}

/// The interactive wrapper of [`Breadcrumbs`].
#[derive(Debug, Clone)]
pub struct BreadcrumbsControl {
    /// The identity; crumbs use child ids.
    pub id: Id,
    placed: Vec<Placed>,
}

impl BreadcrumbsControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            placed: Vec::new(),
        }
    }

    /// The outline of the crumbs drawn, texts read from the screen.
    pub fn outline(&self, buf: &Buffer) -> Outline {
        chrome::outline_of(&self.placed, buf, |_| true)
    }

    /// How many cells `props` need.
    pub fn width_of(props: &Breadcrumbs) -> u16 {
        let widths: u16 = props.crumbs.iter().map(crumb_width).sum();
        widths + SEPARATOR_W * props.crumbs.len().saturating_sub(1) as u16
    }

    /// Draws the crumbs in `area` over `surface`, returning the column after the last one.
    pub fn draw(
        &mut self,
        rt: &mut Runtime,
        props: &Breadcrumbs,
        area: Rect,
        surface: Color32,
        buf: &mut Buffer,
    ) -> u16 {
        self.placed.clear();
        let look = rt.look.clone();
        let c = &look.theme.colors;
        let notation = chrome::notation(rt);
        let t = Tokens::new(&look, surface);
        let n = props.crumbs.len();
        if n == 0 || area.is_empty() {
            return area.x;
        }
        // Drop crumbs from the left until the rest (plus an ellipsis) fits.
        let mut first = 0usize;
        let width_from = |first: usize| -> u16 {
            let widths: u16 = props.crumbs[first..].iter().map(crumb_width).sum();
            let seps = SEPARATOR_W * (n - first).saturating_sub(1) as u16;
            let ellipsis = if first > 0 { 1 + SEPARATOR_W } else { 0 };
            widths + seps + ellipsis
        };
        while first + 1 < n && width_from(first) > area.width {
            first += 1;
        }
        let mut x = area.x;
        let sep_style = t.style(t.of(&look, c.text_placeholder));
        if first > 0 {
            x = put(
                buf,
                area,
                x,
                area.y,
                "…",
                t.style(t.of(&look, c.text_secondary)),
            );
            x = put(buf, area, x, area.y, " › ", sep_style);
        }
        for (i, crumb) in props.crumbs.iter().enumerate().skip(first) {
            let id = chrome::child(self.id, &crumb.id);
            let hovered = rt.hovered() == Some(id);
            let bg = if hovered && crumb.clickable {
                t.of(&look, c.hover)
            } else {
                t.bg
            };
            let start = x;
            let w = crumb_width(crumb).min(area.right().saturating_sub(x));
            let rect = Rect::new(x, area.y, w, 1);
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let mut cx = x;
            if let Some(icon) = &crumb.icon {
                let (ch, color) = chrome::icon_cell(&look, &t, icon, crumb.tone);
                cx = put(
                    buf,
                    area,
                    cx,
                    area.y,
                    &ch,
                    Style::default().fg(color).bg(bg),
                );
                cx += 1;
            }
            let text = if i + 1 == n {
                crate::components::clip(&crumb.text, area.right().saturating_sub(cx))
            } else {
                crumb.text.clone()
            };
            let end = put(
                buf,
                area,
                cx,
                area.y,
                &text,
                Style::default().fg(t.text(&look, crumb.tone)).bg(bg),
            );
            let rect = Rect::new(start, area.y, end.saturating_sub(start), 1);
            if crumb.clickable {
                rt.clickable(id, rect);
            }
            if let Some(tip) = &crumb.tooltip {
                rt.tooltip.offer(id, rect, chrome::tooltip(tip, notation));
            }
            let mut entry = OutlineEntry::new("crumb", crumb.id.clone(), crumb.text.clone());
            entry.enabled = crumb.clickable;
            entry.tooltip = crumb.tooltip.as_ref().map(|t| t.text(notation));
            // The label is read from the cells after the icon.
            let text_x = start + if crumb.icon.is_some() { 2 } else { 0 };
            self.placed.push(Placed {
                key: crumb.id.clone(),
                id,
                rect: Rect::new(text_x, area.y, end.saturating_sub(text_x), 1),
                entry,
            });
            x = end;
            if i + 1 < n {
                x = put(buf, area, x, area.y, " ", t.style(t.bg));
                x = put(
                    buf,
                    area,
                    x,
                    area.y,
                    if look.ascii { ">" } else { "›" },
                    sep_style,
                );
                x = put(buf, area, x, area.y, " ", t.style(t.bg));
            }
        }
        let _ = Glyph::Menu;
        x
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Vec<BreadcrumbEvent> {
        let Event::Mouse(_) = event else {
            return Vec::new();
        };
        let Some(p) = rt.pointer().copied() else {
            return Vec::new();
        };
        self.placed
            .iter()
            .filter(|pl| p.clicked(pl.id) && pl.entry.enabled)
            .map(|pl| BreadcrumbEvent::Click { id: pl.key.clone() })
            .collect()
    }
}
