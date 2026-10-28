//! [`ToolbarControl`]: a [`model::Toolbar`] as one row of cells (TUI-GUI-020..026): icon
//! buttons three cells wide with one blank cell between them, dropdown widgets with an
//! avatar or icon, a label and a chevron, a segmented control; the glyph of the first
//! button is at column 1 and that of the last at the second last column. The leading items start at the left edge, the trailing items end at the right
//! edge; when the row is too narrow the Code | Agents switch hides first, then the labels
//! of dropdowns shorten to their icon in their `shrink_order`.
//!
//! Hover fills the whole widget with the `hover` token, a press with `pressed`; a disabled
//! button is drawn in the disabled tone and keeps its tooltip (the reason). Toolbars are not
//! in the focus order (as in IntelliJ).

use crate::chrome::{self, Placed, Tokens};
use crate::components::{clip, fill, put, text_width};
use crate::hit::Id;
use crate::input::Event;
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use crate::{Glyph, Look, model};
use archipel_kit::egui::Color32;
use archipel_kit_model::{
    Notation, Outline, OutlineEntry, Tone, Toolbar, ToolbarButton, ToolbarDropdown, ToolbarEvent,
    ToolbarItem,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// Cells at both ends of the row: none, a button's own padding is the margin (the hamburger
/// glyph is at column 1).
const MARGIN: u16 = 0;
/// Cells between widgets.
const GAP: u16 = 1;

/// The widths below which the Code | Agents switch, the labels of the widgets of
/// `shrink_order` 1 and those of `shrink_order` 2 are dropped (TUI-GUI-025), whether or not
/// the row would still fit.
const HIDE_SWITCH_BELOW: u16 = 100;
const HIDE_ORDER_1_BELOW: u16 = 90;
const HIDE_ORDER_2_BELOW: u16 = 80;

/// How the items of a toolbar are shown at the current width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fit {
    segmented: bool,
    /// Dropdowns with a `shrink_order` from 1 up to this level lose their label.
    shrink_level: u8,
}

fn avatar_width(d: &ToolbarDropdown) -> u16 {
    if d.avatar.is_some() { 4 } else { 0 }
}

fn dropdown_label(d: &ToolbarDropdown, fit: Fit) -> Option<String> {
    let shrinks = d.shrink_order > 0 && d.shrink_order <= fit.shrink_level;
    if shrinks && (d.icon.is_some() || d.avatar.is_some()) {
        return None;
    }
    if d.label.is_empty() {
        return None;
    }
    Some(match d.max_chars {
        Some(max) => clip(&d.label, max),
        None => d.label.clone(),
    })
}

fn dropdown_width(d: &ToolbarDropdown, fit: Fit) -> u16 {
    let label = dropdown_label(d, fit);
    let lead = if d.avatar.is_some() {
        avatar_width(d)
    } else if d.icon.is_some() {
        1
    } else {
        0
    };
    let text = label
        .as_deref()
        .map_or(0, |l| text_width(l) + u16::from(lead > 0));
    let chevron = if d.chevron { 2 } else { 0 };
    1 + lead + text + chevron + 1
}

fn segmented_width(options: &[String]) -> u16 {
    options.iter().map(|o| text_width(o) + 2).sum()
}

fn item_width(item: &ToolbarItem, fit: Fit) -> u16 {
    match item {
        ToolbarItem::Button(b) => match &b.label {
            Some(label) if b.icon.is_some() => 1 + 1 + 1 + text_width(label) + 1,
            Some(label) => 1 + text_width(label) + 1,
            None => 3,
        },
        ToolbarItem::Dropdown(d) => dropdown_width(d, fit),
        ToolbarItem::Segmented { options, .. } if fit.segmented => segmented_width(options),
        ToolbarItem::Segmented { .. } => 0,
        ToolbarItem::Separator => 1,
    }
}

fn total(items: &[&ToolbarItem], fit: Fit) -> u16 {
    let widths: Vec<u16> = items
        .iter()
        .map(|i| item_width(i, fit))
        .filter(|w| *w > 0)
        .collect();
    widths.iter().sum::<u16>() + GAP * widths.len().saturating_sub(1) as u16
}

/// The interactive wrapper of a [`Toolbar`].
#[derive(Debug, Clone)]
pub struct ToolbarControl {
    /// The identity; items use child ids.
    pub id: Id,
    placed: Vec<Placed>,
    pressed: Option<Id>,
    bar: Rect,
    segments: Vec<(Id, String, usize)>,
}

impl ToolbarControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            placed: Vec::new(),
            pressed: None,
            bar: Rect::default(),
            segments: Vec::new(),
        }
    }

    /// Where the item `key` was drawn (the anchor of its popup).
    pub fn rect_of(&self, key: &str) -> Option<Rect> {
        self.placed.iter().find(|p| p.key == key).map(|p| p.rect)
    }

    /// The outline of what was drawn: labels of dropdowns are read from `buf`.
    pub fn outline(&self, buf: &Buffer) -> Outline {
        chrome::outline_of(&self.placed, buf, |p| p.entry.role == "dropdown")
    }

    fn fit(items: &[&ToolbarItem], width: u16) -> Fit {
        let mut fit = Fit {
            segmented: true,
            shrink_level: 0,
        };
        let budget = width.saturating_sub(2 * MARGIN + 2);
        if width < HIDE_ORDER_2_BELOW {
            fit.shrink_level = 2;
        } else if width < HIDE_ORDER_1_BELOW {
            fit.shrink_level = 1;
        }
        if width < HIDE_SWITCH_BELOW {
            fit.segmented = false;
        }
        if total(items, fit) <= budget {
            return fit;
        }
        fit.segmented = false;
        let max_order = items
            .iter()
            .filter_map(|i| match i {
                ToolbarItem::Dropdown(d) => Some(d.shrink_order),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        // Labels go in the order of `shrink_order`: 1 first.
        for level in 1..=max_order {
            if total(items, fit) <= budget {
                return fit;
            }
            fit.shrink_level = fit.shrink_level.max(level);
        }
        fit
    }

    /// Draws `props` in `area` (its first row) over the `frame` surface, registering every
    /// item with the hit map and the tooltips.
    pub fn draw(&mut self, rt: &mut Runtime, props: &Toolbar, area: Rect, buf: &mut Buffer) {
        self.placed.clear();
        self.segments.clear();
        self.bar = area;
        let look = rt.look.clone();
        let notation = chrome::notation(rt);
        let surface = look.theme.colors.frame;
        let t = Tokens::new(&look, surface);
        let row = Rect::new(area.x, area.y, area.width, 1.min(area.height));
        if row.is_empty() {
            return;
        }
        fill(
            buf,
            row,
            row.x,
            row.y,
            row.width,
            t.style(t.of(&look, look.theme.colors.text)),
        );
        let items: Vec<&ToolbarItem> = props.items().collect();
        let fit = Self::fit(&items, area.width);
        let lead: Vec<&ToolbarItem> = props.leading.iter().collect();
        let trail: Vec<&ToolbarItem> = props.trailing.iter().collect();
        let mut x = row.x + MARGIN;
        for item in lead {
            x = self.place(rt, &look, &t, notation, item, fit, x, row, buf);
        }
        let trail_w = total(&trail, fit);
        let mut x = (row.right().saturating_sub(MARGIN + trail_w)).max(x);
        for item in trail {
            x = self.place(rt, &look, &t, notation, item, fit, x, row, buf);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        rt: &mut Runtime,
        look: &Look,
        t: &Tokens,
        notation: Notation,
        item: &ToolbarItem,
        fit: Fit,
        x: u16,
        row: Rect,
        buf: &mut Buffer,
    ) -> u16 {
        let w = item_width(item, fit);
        if w == 0 {
            return x;
        }
        let rect = Rect::new(x, row.y, w, 1).intersection(row);
        match item {
            ToolbarItem::Button(b) => self.draw_button(rt, look, t, notation, b, rect, buf),
            ToolbarItem::Dropdown(d) => {
                self.draw_dropdown(rt, look, t, notation, d, fit, rect, buf)
            }
            ToolbarItem::Segmented {
                id,
                options,
                selected,
            } => self.draw_segmented(rt, look, t, id, options, *selected, rect, buf),
            ToolbarItem::Separator => {
                let c = &look.theme.colors;
                put(
                    buf,
                    row,
                    rect.x,
                    rect.y,
                    "│",
                    t.style(t.of(look, c.separator)),
                );
            }
        }
        x + w + GAP
    }

    #[allow(clippy::too_many_arguments)]
    fn widget_fill(
        &self,
        rt: &Runtime,
        look: &Look,
        t: &Tokens,
        id: Id,
        rect: Rect,
        enabled: bool,
        buf: &mut Buffer,
    ) -> Style {
        let c = &look.theme.colors;
        let hovered = rt.hovered() == Some(id);
        let pressed = self.pressed == Some(id) && hovered;
        let bg_token: Option<Color32> = if !enabled {
            None
        } else if pressed {
            Some(c.pressed)
        } else if hovered {
            Some(c.hover)
        } else {
            None
        };
        let bg = bg_token.map_or(t.bg, |token| t.of(look, token));
        let style = Style::default().bg(bg);
        fill(buf, rect, rect.x, rect.y, rect.width, style);
        style
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_button(
        &mut self,
        rt: &mut Runtime,
        look: &Look,
        t: &Tokens,
        notation: Notation,
        b: &ToolbarButton,
        rect: Rect,
        buf: &mut Buffer,
    ) {
        let id = chrome::child(self.id, &b.id);
        let c = &look.theme.colors;
        let base = self.widget_fill(rt, look, t, id, rect, b.enabled, buf);
        let tone = if b.enabled { b.tone } else { Tone::Disabled };
        let fg = if b.selected && b.enabled {
            t.of(look, c.text_strong)
        } else {
            t.icon(look, tone)
        };
        let style = base.fg(fg);
        match (&b.icon, &b.label) {
            (Some(icon), None) => {
                let (ch, color) = chrome::icon_cell(look, t, icon, tone);
                put(
                    buf,
                    rect,
                    rect.x + 1,
                    rect.y,
                    &ch,
                    base.fg(if b.enabled { color } else { fg }),
                );
            }
            (Some(icon), Some(label)) => {
                let (ch, color) = chrome::icon_cell(look, t, icon, tone);
                put(buf, rect, rect.x + 1, rect.y, &ch, base.fg(color));
                put(
                    buf,
                    rect,
                    rect.x + 3,
                    rect.y,
                    label,
                    base.fg(t.text(
                        look,
                        if b.enabled {
                            Tone::Default
                        } else {
                            Tone::Disabled
                        },
                    )),
                );
            }
            (None, Some(label)) => {
                put(
                    buf,
                    rect,
                    rect.x + 1,
                    rect.y,
                    label,
                    base.fg(t.text(
                        look,
                        if b.enabled {
                            Tone::Default
                        } else {
                            Tone::Disabled
                        },
                    )),
                );
            }
            (None, None) => {}
        }
        let _ = style;
        if let Some(badge) = b.badge {
            chrome::badge(
                look,
                t,
                buf,
                rect,
                rect.right().saturating_sub(1),
                rect.y,
                badge,
                base.bg.unwrap_or(t.bg),
            );
        }
        rt.clickable(id, rect);
        if let Some(tip) = &b.tooltip {
            rt.tooltip.offer(id, rect, chrome::tooltip(tip, notation));
        }
        let mut entry = OutlineEntry::new("button", b.id.clone(), b.name.clone());
        entry.enabled = b.enabled;
        entry.checked = b.selected.then_some(true);
        entry.badge = b.badge.map(|t| format!("{t:?}").to_ascii_lowercase());
        entry.tooltip = b.tooltip.as_ref().map(|t| t.text(notation));
        entry.shortcut = b
            .tooltip
            .as_ref()
            .and_then(|t| t.shortcut.as_ref())
            .map(|s| s.label(notation));
        self.placed.push(Placed {
            label_rect: None,
            key: b.id.clone(),
            id,
            rect,
            entry,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_dropdown(
        &mut self,
        rt: &mut Runtime,
        look: &Look,
        t: &Tokens,
        notation: Notation,
        d: &ToolbarDropdown,
        fit: Fit,
        rect: Rect,
        buf: &mut Buffer,
    ) {
        let id = chrome::child(self.id, &d.id);
        let c = &look.theme.colors;
        let base = self.widget_fill(rt, look, t, id, rect, d.enabled, buf);
        let bg = base.bg.unwrap_or(t.bg);
        let tone = if d.enabled { d.tone } else { Tone::Disabled };
        let label = dropdown_label(d, fit);
        let mut x = rect.x + 1;
        if let Some(avatar) = &d.avatar {
            let rgb = ratatui::style::Color::Rgb(
                (avatar.rgb >> 16) as u8,
                (avatar.rgb >> 8) as u8,
                avatar.rgb as u8,
            );
            put(buf, rect, x, rect.y, "▐", Style::default().fg(rgb).bg(bg));
            let initials: String = avatar.initials.chars().take(2).collect();
            let pad = if initials.chars().count() == 1 {
                format!("{initials} ")
            } else {
                initials
            };
            put(
                buf,
                rect,
                x + 1,
                rect.y,
                &pad,
                Style::default()
                    .fg(t.of(look, c.text_on_accent))
                    .bg(rgb)
                    .add_modifier(Modifier::BOLD),
            );
            put(
                buf,
                rect,
                x + 3,
                rect.y,
                "▌",
                Style::default().fg(rgb).bg(bg),
            );
            x += 4;
        } else if let Some(icon) = &d.icon {
            let (ch, color) = chrome::icon_cell(look, t, icon, Tone::Default);
            put(buf, rect, x, rect.y, &ch, base.fg(color));
            x += 1;
        }
        let mut label_rect = Some(Rect::new(x, rect.y, 0, 1));
        if let Some(label) = &label {
            if x > rect.x + 1 {
                x += 1;
            }
            let st = base.fg(t.text(look, tone));
            let start = x;
            x = put(buf, rect, x, rect.y, label, st);
            label_rect = Some(Rect::new(start, rect.y, x.saturating_sub(start), 1));
        }
        if d.chevron {
            put(
                buf,
                rect,
                x + 1,
                rect.y,
                &look.glyph(Glyph::ChevronDown),
                base.fg(t.of(look, c.text_secondary)),
            );
        }
        rt.clickable(id, rect);
        if let Some(tip) = &d.tooltip {
            rt.tooltip.offer(id, rect, chrome::tooltip(tip, notation));
        }
        let mut entry = OutlineEntry::new("dropdown", d.id.clone(), d.label.clone());
        entry.enabled = d.enabled;
        entry.tooltip = d.tooltip.as_ref().map(|t| t.text(notation));
        // The label on screen: the full label unless the width forced a shorter form.
        self.placed.push(Placed {
            label_rect,
            key: d.id.clone(),
            id,
            rect,
            entry,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_segmented(
        &mut self,
        rt: &mut Runtime,
        look: &Look,
        t: &Tokens,
        key: &str,
        options: &[String],
        selected: usize,
        rect: Rect,
        buf: &mut Buffer,
    ) {
        let c = &look.theme.colors;
        let mut x = rect.x;
        for (i, option) in options.iter().enumerate() {
            let w = text_width(option) + 2;
            let seg = Rect::new(x, rect.y, w, 1).intersection(rect);
            let id = chrome::child(self.id, &format!("{key}/{i}"));
            let on = i == selected;
            let hovered = rt.hovered() == Some(id);
            let bg = if on {
                t.of(look, c.segment_selected)
            } else if hovered {
                t.of(look, c.hover)
            } else {
                t.bg
            };
            fill(buf, seg, seg.x, seg.y, seg.width, Style::default().bg(bg));
            let fg = if on {
                t.of(look, c.text_strong)
            } else {
                t.of(look, c.text_secondary)
            };
            let mut st = Style::default().fg(fg).bg(bg);
            if on {
                st = st.add_modifier(Modifier::BOLD);
            }
            put(buf, seg, seg.x + 1, seg.y, option, st);
            rt.clickable(id, seg);
            self.segments.push((id, key.to_owned(), i));
            x += w;
        }
        let mut entry = OutlineEntry::new("segmented", key, options.join("|"));
        entry.checked = Some(selected < options.len());
        self.placed.push(Placed {
            label_rect: None,
            key: key.to_owned(),
            id: chrome::child(self.id, key),
            rect,
            entry,
        });
    }

    /// Applies `event`: clicks on buttons, dropdowns and segments.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Vec<ToolbarEvent> {
        let Event::Mouse(_) = event else {
            return Vec::new();
        };
        let Some(p) = rt.pointer().copied() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        match p.kind {
            PointerKind::Down(crate::input::MouseButton::Left) => {
                self.pressed = p.target;
            }
            PointerKind::Up(crate::input::MouseButton::Left) => {
                self.pressed = None;
                for placed in &self.placed {
                    if placed.entry.role != "segmented"
                        && p.clicked(placed.id)
                        && placed.entry.enabled
                    {
                        out.push(ToolbarEvent::Click {
                            id: placed.key.clone(),
                        });
                    }
                }
                for (id, key, index) in &self.segments {
                    if p.clicked(*id) {
                        out.push(ToolbarEvent::Select {
                            id: key.clone(),
                            index: *index,
                        });
                    }
                }
            }
            PointerKind::Down(crate::input::MouseButton::Right) => {
                for placed in &self.placed {
                    if p.over(placed.id) && placed.entry.role != "segmented" {
                        out.push(ToolbarEvent::ContextClick {
                            id: placed.key.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
        out
    }
}

#[allow(dead_code)]
fn _use(_: &model::Tone) {}
