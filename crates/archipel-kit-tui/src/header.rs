//! [`PanelHeaderControl`]: a [`model::PanelHeader`] as the one-row header of a tool window
//! (TUI-GUI-070..075): the title in strong bold at one cell from the edge, a chevron after it
//! when the title is a view selector, content tabs (the selected one underlined in the tab
//! border color, the count in the secondary tone, `✕` on closable tabs), action buttons right
//! aligned and the Hide button last.
//!
//! Buttons are three cells wide (the GUI's 26 px pitch) with the toolbar's hover, pressed and
//! disabled states and tooltips. When the row is narrower than the title, tabs and actions,
//! the actions that do not fit move into a `⋮` overflow menu. A double click on the header
//! (outside its widgets) asks to maximize the window.

use crate::Glyph;
use crate::chrome::{self, Placed, Tokens};
use crate::components::{clip, fill, put, text_width};
use crate::hit::Id;
use crate::input::{Event, MouseButton};
use crate::menubar::{MenuPopup, menu_popup};
use crate::pointer::PointerKind;
use crate::popup::{Placement, PopupState};
use crate::runtime::Runtime;
use archipel_kit_model::{
    MenuEntry, MenuItem, Outline, OutlineEntry, PanelHeader, PanelHeaderEvent, Tone,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

const BUTTON_W: u16 = 3;

/// The interactive wrapper of a [`PanelHeader`].
pub struct PanelHeaderControl {
    /// The identity; widgets use child ids.
    pub id: Id,
    placed: Vec<Placed>,
    pressed: Option<Id>,
    overflow: Option<MenuPopup>,
    overflow_actions: Vec<MenuEntry>,
    bar: Rect,
    title_id: Id,
    close_ids: Vec<(Id, String)>,
}

impl PanelHeaderControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            placed: Vec::new(),
            pressed: None,
            overflow: None,
            overflow_actions: Vec::new(),
            bar: Rect::default(),
            title_id: chrome::child(id, "title"),
            close_ids: Vec::new(),
        }
    }

    /// Where the widget `key` is (`title`, `hide`, an action or tab id).
    pub fn rect_of(&self, key: &str) -> Option<Rect> {
        self.placed.iter().find(|p| p.key == key).map(|p| p.rect)
    }

    /// The outline of what was drawn.
    pub fn outline(&self, buf: &Buffer) -> Outline {
        // Left to right, as the props list them (the right-aligned actions are placed from
        // the right edge).
        let mut placed = self.placed.clone();
        placed.sort_by_key(|p| p.rect.x);
        chrome::outline_of(&placed, buf, |p| matches!(p.entry.role.as_str(), "title"))
    }

    /// Whether the overflow menu is open.
    pub fn overflow_open(&self) -> bool {
        self.overflow.as_ref().is_some_and(|p| p.is_open())
    }

    /// Draws `props` in `area` (one row) over the `island` surface.
    pub fn draw(&mut self, rt: &mut Runtime, props: &PanelHeader, area: Rect, buf: &mut Buffer) {
        self.placed.clear();
        self.close_ids.clear();
        self.bar = area;
        let row = Rect::new(area.x, area.y, area.width, 1.min(area.height));
        if row.is_empty() {
            return;
        }
        let look = rt.look.clone();
        let c = &look.theme.colors;
        let notation = chrome::notation(rt);
        let t = Tokens::new(&look, c.island);
        fill(
            buf,
            row,
            row.x,
            row.y,
            row.width,
            t.style(t.of(&look, c.text)),
        );
        rt.hits.add(row, self.id);

        // Right side first: Hide, then the actions as far as they fit.
        let right = row.right().saturating_sub(1);
        let title_w = text_width(&props.title) + u16::from(props.view_selector) * 2;
        let tabs_w: u16 = props.tabs.iter().map(tab_width).sum::<u16>();
        let left_need = 1 + title_w + if props.tabs.is_empty() { 0 } else { 2 + tabs_w };
        let budget = right.saturating_sub(row.x + left_need);
        let hide_w = if props.hide { BUTTON_W } else { 0 };
        let mut fit = 0usize;
        let mut used = hide_w;
        for (i, _) in props.actions.iter().enumerate().rev() {
            let _ = i;
            if used + BUTTON_W <= budget {
                used += BUTTON_W;
                fit += 1;
            } else {
                break;
            }
        }
        let overflowed = fit < props.actions.len();
        if overflowed && used + BUTTON_W > budget && fit > 0 {
            // Room for the overflow button itself.
            fit -= 1;
        }
        let hidden = props.actions.len() - fit;
        let overflowed = hidden > 0;
        self.overflow_actions.clear();

        let place_button = |this: &mut Self,
                            rt: &mut Runtime,
                            buf: &mut Buffer,
                            x: u16,
                            key: &str,
                            glyph: Glyph,
                            name: &str,
                            tone: Tone,
                            enabled: bool,
                            tip: Option<&archipel_kit_model::Tooltip>| {
            let rect = Rect::new(x, row.y, BUTTON_W, 1).intersection(row);
            let id = chrome::child(this.id, key);
            let hovered = rt.hovered() == Some(id);
            let pressed = this.pressed == Some(id) && hovered;
            let bg = if enabled && pressed {
                t.of(&look, c.pressed)
            } else if enabled && hovered {
                t.of(&look, c.hover)
            } else {
                t.bg
            };
            let style = Style::default().bg(bg);
            fill(buf, rect, rect.x, rect.y, rect.width, style);
            let tone = if enabled { tone } else { Tone::Disabled };
            put(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                &look.glyph(glyph),
                style.fg(t.icon(&look, tone)),
            );
            rt.clickable(id, rect);
            if let Some(tip) = tip {
                rt.tooltip.offer(id, rect, chrome::tooltip(tip, notation));
            }
            let mut entry = OutlineEntry::new("button", key, name);
            entry.enabled = enabled;
            entry.tooltip = tip.map(|t| t.text(notation));
            this.placed.push(Placed {
                label_rect: None,
                key: key.to_owned(),
                id,
                rect,
                entry,
            });
        };

        if props.hide {
            place_button(
                self,
                rt,
                buf,
                row.right().saturating_sub(1 + BUTTON_W).max(row.x),
                "hide",
                Glyph::Minimize,
                "Hide",
                Tone::Default,
                true,
                Some(&archipel_kit_model::Tooltip::new("Hide")),
            );
        }
        let mut x = row.right().saturating_sub(1 + hide_w);
        for action in props.actions[props.actions.len() - fit..].iter().rev() {
            x = x.saturating_sub(BUTTON_W);
            let glyph = action
                .icon
                .as_ref()
                .and_then(|i| i.glyph())
                .unwrap_or(Glyph::More);
            place_button(
                self,
                rt,
                buf,
                x,
                &action.id,
                glyph,
                &action.name,
                action.tone,
                action.enabled,
                action.tooltip.as_ref(),
            );
        }
        if overflowed {
            x = x.saturating_sub(BUTTON_W);
            place_button(
                self,
                rt,
                buf,
                x,
                "overflow",
                Glyph::MoreVertical,
                "More actions",
                Tone::Default,
                true,
                Some(&archipel_kit_model::Tooltip::new("More actions")),
            );
            for action in &props.actions[..hidden] {
                let mut item = MenuItem::new(action.id.clone(), action.name.clone());
                item.enabled = action.enabled;
                self.overflow_actions.push(MenuEntry::Item(item));
            }
        }

        // Left side: title, chevron, tabs.
        let limit = x.saturating_sub(1).max(row.x);
        let left = Rect::new(row.x, row.y, limit.saturating_sub(row.x), 1);
        let strong = t.of(&look, c.text_strong);
        let focus_style = if props.focused {
            t.style(strong)
        } else {
            t.style(t.of(&look, c.text))
        };
        let title = clip(&props.title, left.width.saturating_sub(3));
        let mut cx = put(
            buf,
            left,
            row.x + 1,
            row.y,
            &title,
            focus_style.add_modifier(Modifier::BOLD),
        );
        if props.view_selector {
            cx = put(
                buf,
                left,
                cx + 1,
                row.y,
                &look.glyph(Glyph::ChevronDown),
                t.style(t.of(&look, c.text_secondary)),
            );
        }
        let title_rect =
            Rect::new(row.x + 1, row.y, cx.saturating_sub(row.x + 1), 1).intersection(row);
        rt.clickable(self.title_id, title_rect);
        let mut entry = OutlineEntry::new("title", props.id.clone(), props.title.clone());
        entry.expanded = props.view_selector.then_some(false);
        self.placed.push(Placed {
            label_rect: None,
            key: "title".to_owned(),
            id: self.title_id,
            rect: Rect::new(row.x + 1, row.y, text_width(&title), 1),
            entry,
        });
        if !props.tabs.is_empty() {
            cx += 2;
        }
        for tab in &props.tabs {
            let w = tab_width(tab);
            let rect = Rect::new(cx, row.y, w, 1).intersection(left);
            if rect.width < w {
                break;
            }
            let id = chrome::child(self.id, &format!("tab:{}", tab.id));
            let hovered = rt.hovered() == Some(id);
            let bg = if hovered { t.of(&look, c.hover) } else { t.bg };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let mut style = Style::default()
                .fg(if tab.selected {
                    strong
                } else {
                    t.of(&look, c.tab_text)
                })
                .bg(bg);
            if tab.selected {
                style = style
                    .add_modifier(Modifier::UNDERLINED)
                    .underline_color(t.of(&look, c.tab_selected_border));
            }
            let mut tx = put(buf, rect, rect.x + 1, rect.y, &tab.title, style);
            if let Some(count) = tab.count {
                tx = put(
                    buf,
                    rect,
                    tx + 1,
                    rect.y,
                    &count.to_string(),
                    Style::default().fg(t.of(&look, c.text_secondary)).bg(bg),
                );
            }
            if tab.closable {
                let close_id = chrome::child(self.id, &format!("close:{}", tab.id));
                let cx2 = tx + 1;
                put(
                    buf,
                    rect,
                    cx2,
                    rect.y,
                    &look.glyph(Glyph::Close),
                    Style::default().fg(t.of(&look, c.text_secondary)).bg(bg),
                );
                rt.clickable(close_id, Rect::new(cx2, rect.y, 1, 1));
                self.close_ids.push((close_id, tab.id.clone()));
            }
            rt.clickable(id, rect);
            let label = match tab.count {
                Some(count) => format!("{} {count}", tab.title),
                None => tab.title.clone(),
            };
            let mut entry = OutlineEntry::new("tab", tab.id.clone(), label);
            entry.checked = Some(tab.selected);
            self.placed.push(Placed {
                label_rect: None,
                key: format!("tab:{}", tab.id),
                id,
                rect,
                entry,
            });
            cx += w;
        }
    }

    /// Draws the overflow menu above everything.
    pub fn draw_popup(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        if let Some(popup) = &mut self.overflow {
            popup.draw(rt, buf);
        }
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Vec<PanelHeaderEvent> {
        let mut out = Vec::new();
        if let Some(popup) = &mut self.overflow
            && popup.is_open()
        {
            match popup.event(rt, event) {
                PopupState::Chosen => {
                    if let Some(id) = popup.take_choice() {
                        out.push(PanelHeaderEvent::Action { id });
                    }
                    self.overflow = None;
                }
                PopupState::Cancelled => self.overflow = None,
                PopupState::Open => {}
            }
            return out;
        }
        let Event::Mouse(_) = event else {
            return out;
        };
        let Some(p) = rt.pointer().copied() else {
            return out;
        };
        match p.kind {
            PointerKind::Down(MouseButton::Left) => {
                self.pressed = p.target;
            }
            PointerKind::Up(MouseButton::Left) => {
                self.pressed = None;
                if let Some((_, tab)) = self.close_ids.iter().find(|(id, _)| p.clicked(*id)) {
                    out.push(PanelHeaderEvent::CloseTab { id: tab.clone() });
                    return out;
                }
                for placed in &self.placed {
                    if !p.clicked(placed.id) || !placed.entry.enabled {
                        continue;
                    }
                    match placed.entry.role.as_str() {
                        "title" => out.push(PanelHeaderEvent::Title),
                        "tab" => out.push(PanelHeaderEvent::SelectTab {
                            id: placed.key.trim_start_matches("tab:").to_owned(),
                        }),
                        _ if placed.key == "hide" => out.push(PanelHeaderEvent::Hide),
                        _ if placed.key == "overflow" => {
                            let mut popup = menu_popup(
                                self.overflow_actions.clone(),
                                chrome::notation(rt),
                                Placement::Below(placed.rect),
                            );
                            popup.open(rt, false);
                            self.overflow = Some(popup);
                        }
                        _ => out.push(PanelHeaderEvent::Action {
                            id: placed.key.clone(),
                        }),
                    }
                }
                if p.clicks >= 2 && p.over(self.id) {
                    out.push(PanelHeaderEvent::Maximize);
                }
            }
            PointerKind::Down(MouseButton::Middle) => {
                for placed in &self.placed {
                    if p.over(placed.id) && placed.entry.role == "tab" {
                        out.push(PanelHeaderEvent::CloseTab {
                            id: placed.key.trim_start_matches("tab:").to_owned(),
                        });
                    }
                }
            }
            _ => {}
        }
        out
    }
}

fn tab_width(tab: &archipel_kit_model::PanelTab) -> u16 {
    1 + text_width(&tab.title)
        + tab.count.map_or(0, |c| 1 + text_width(&c.to_string()))
        + if tab.closable { 2 } else { 0 }
        + 1
}
