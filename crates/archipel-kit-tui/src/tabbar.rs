//! [`TabBarControl`]: a [`model::TabBar`] as the editor's tab strip (TUI-GUI-085..088).
//!
//! A tab is the file icon, the name and one state cell between one cell of padding at each
//! end: `✕` close (on the selected tab and on hover), `●` for unsaved changes (shown
//! instead of the close button until the pointer is over the tab), a pin for pinned tabs
//! (a click unpins). A preview tab is italic, a file with errors has a red curly underline
//! under its name (a plain underline where the terminal has no undercurl), the selected tab
//! has the `tab_selected` fill, the strong text color and an underline in the tab border
//! color; the others use the tab text color and the hover fill.
//!
//! A press selects, a middle click or Shift+click closes, the `✕` closes 150 ms after the
//! pointer entered the tab, a right press asks for the tab menu, a drag reorders (after a
//! 2-cell dead zone), the wheel scrolls a strip that is wider than its area, and `⋮` at the
//! end lists every tab when the strip overflows.

use crate::Glyph;
use crate::chrome::{self, Placed, Tokens};
use crate::components::{fill, put, text_width};
use crate::hit::Id;
use crate::input::{Event, MouseButton};
use crate::menubar::{MenuPopup, menu_popup};
use crate::pointer::PointerKind;
use crate::popup::{Placement, PopupState};
use crate::runtime::Runtime;
use archipel_kit_model::{
    IconKey, MenuEntry, MenuItem, Outline, OutlineEntry, TabBar, TabEvent, Tone,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// Cells the pointer must move before a pressed tab starts to drag.
pub const DRAG_DEAD_ZONE: u16 = 2;
/// Seconds after the pointer entered a tab before its `✕` closes it.
pub const CLOSE_GUARD: f64 = 0.15;

fn tab_width(title: &str) -> u16 {
    text_width(title) + 6
}

/// The interactive wrapper of a [`TabBar`].
pub struct TabBarControl {
    /// The identity; tabs use child ids.
    pub id: Id,
    offset: usize,
    placed: Vec<Placed>,
    closes: Vec<(Id, String)>,
    pins: Vec<(Id, String)>,
    hover: Option<String>,
    hover_since: f64,
    drag: Option<Drag>,
    overflow: Option<MenuPopup>,
    overflow_id: Id,
    order: Vec<String>,
    strip: Rect,
    scroll_to_selected: bool,
    last_selected: Option<String>,
}

#[derive(Debug, Clone)]
struct Drag {
    index: usize,
    start_x: u16,
    active: bool,
}

impl TabBarControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            offset: 0,
            placed: Vec::new(),
            closes: Vec::new(),
            pins: Vec::new(),
            hover: None,
            hover_since: 0.0,
            drag: None,
            overflow: None,
            overflow_id: chrome::child(id, "overflow"),
            order: Vec::new(),
            strip: Rect::default(),
            scroll_to_selected: true,
            last_selected: None,
        }
    }

    /// Where the tab `key` is.
    pub fn rect_of(&self, key: &str) -> Option<Rect> {
        self.placed.iter().find(|p| p.key == key).map(|p| p.rect)
    }

    /// The outline of what was drawn (visible tabs), names read from the screen.
    pub fn outline(&self, buf: &Buffer) -> Outline {
        chrome::outline_of(&self.placed, buf, |p| p.entry.role == "tab")
    }

    /// Whether the overflow list is open.
    pub fn overflow_open(&self) -> bool {
        self.overflow.as_ref().is_some_and(|p| p.is_open())
    }

    /// The first tab shown (scroll position).
    pub fn first_visible(&self) -> usize {
        self.offset
    }

    /// Draws `props` in `area` (one row) over the `island` surface.
    pub fn draw(&mut self, rt: &mut Runtime, props: &TabBar, area: Rect, buf: &mut Buffer) {
        self.placed.clear();
        self.closes.clear();
        self.pins.clear();
        self.order = props.tabs.iter().map(|t| t.id.clone()).collect();
        let row = Rect::new(area.x, area.y, area.width, 1.min(area.height));
        self.strip = row;
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
        let widths: Vec<u16> = props.tabs.iter().map(|tab| tab_width(&tab.title)).collect();
        let total: u16 = widths.iter().sum();
        let overflowing = total > row.width;
        let strip_w = if overflowing {
            row.width.saturating_sub(3)
        } else {
            row.width
        };
        // Keep the selected tab in view when it changed.
        let selected = props.tabs.iter().position(|tab| tab.selected);
        let selected_id = selected.map(|i| props.tabs[i].id.clone());
        if selected_id != self.last_selected {
            self.scroll_to_selected = true;
            self.last_selected.clone_from(&selected_id);
        }
        self.offset = self.offset.min(props.tabs.len().saturating_sub(1));
        if overflowing
            && self.scroll_to_selected
            && let Some(sel) = selected
        {
            if sel < self.offset {
                self.offset = sel;
            }
            while self.offset < sel && widths[self.offset..=sel].iter().sum::<u16>() > strip_w {
                self.offset += 1;
            }
        }
        if !overflowing {
            self.offset = 0;
        }
        self.scroll_to_selected = false;
        let strip = Rect::new(row.x, row.y, strip_w, 1);
        let mut x = strip.x;
        for (index, tab) in props.tabs.iter().enumerate().skip(self.offset) {
            let w = widths[index];
            if x >= strip.right() {
                break;
            }
            let rect = Rect::new(x, row.y, w, 1).intersection(strip);
            let id = chrome::child(self.id, &tab.id);
            let hovered = rt.hovered() == Some(id)
                || self.closes.iter().any(|_| false)
                || rt
                    .hovered()
                    .is_some_and(|h| h == chrome::child(self.id, &format!("close:{}", tab.id)));
            let bg = if tab.selected {
                t.of(&look, c.tab_selected)
            } else if hovered {
                t.of(&look, c.hover)
            } else {
                t.bg
            };
            let fg = if tab.selected {
                t.of(&look, c.text_strong)
            } else {
                t.of(&look, c.tab_text)
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let mut base = Style::default().fg(fg).bg(bg);
            if tab.selected {
                let focused = true;
                let border = if focused {
                    c.tab_selected_border
                } else {
                    c.tab_scroll_thumb
                };
                base = base
                    .add_modifier(Modifier::UNDERLINED)
                    .underline_color(t.of(&look, border));
            }
            let (icon, icon_color) = chrome::icon_cell(&look, &t, &tab.icon, Tone::Default);
            let ix = rect.x + 1;
            put(buf, rect, ix, rect.y, &icon, base.fg(icon_color));
            let nx = ix + 2;
            let mut name_style = base;
            if tab.preview {
                name_style = name_style.add_modifier(Modifier::ITALIC);
            }
            let name_end = put(buf, rect, nx, rect.y, &tab.title, name_style);
            if tab.error {
                let error = t.of(&look, c.error);
                for cx in nx..name_end.min(rect.right()) {
                    if let Some(cell) = buf.cell_mut((cx, rect.y)) {
                        cell.modifier |= Modifier::UNDERLINED;
                        cell.underline_color = error;
                    }
                }
            }
            rt.clickable(id, rect);
            // The state cell.
            let sx = nx + text_width(&tab.title) + 1;
            let close_id = chrome::child(self.id, &format!("close:{}", tab.id));
            let over_dot = rt.hovered() == Some(close_id);
            let over_tab = hovered;
            let secondary = t.of(&look, c.text_secondary);
            let state = if tab.pinned {
                let pin_id = chrome::child(self.id, &format!("pin:{}", tab.id));
                self.pins.push((pin_id, tab.id.clone()));
                rt.clickable(pin_id, Rect::new(sx, rect.y, 1, 1));
                Some((look.glyph(Glyph::Pin), secondary))
            } else if tab.dirty && !over_tab && !over_dot {
                Some((
                    if look.ascii {
                        "*".to_owned()
                    } else {
                        "●".to_owned()
                    },
                    t.of(&look, c.vcs_modified),
                ))
            } else if tab.selected || over_tab || over_dot {
                self.closes.push((close_id, tab.id.clone()));
                rt.clickable(close_id, Rect::new(sx, rect.y, 1, 1));
                Some((look.glyph(Glyph::Close), secondary))
            } else {
                None
            };
            if tab.dirty && !tab.pinned && !self.closes.iter().any(|(i, _)| *i == close_id) {
                // The dot is also the hover target that turns into the close button.
                rt.clickable(close_id, Rect::new(sx, rect.y, 1, 1));
            }
            if let Some((glyph, color)) = state {
                put(buf, rect, sx, rect.y, &glyph, base.fg(color));
            }
            if let Some(tip) = &tab.tooltip {
                rt.tooltip.offer(id, rect, chrome::tooltip(tip, notation));
            }
            let mut entry = OutlineEntry::new("tab", tab.id.clone(), tab.title.clone());
            entry.checked = Some(tab.selected);
            entry.badge = if tab.dirty {
                Some("modified".to_owned())
            } else if tab.pinned {
                Some("pinned".to_owned())
            } else if tab.error {
                Some("error".to_owned())
            } else {
                None
            };
            entry.tooltip = tab.tooltip.as_ref().map(|t| t.text(notation));
            self.placed.push(Placed {
                label_rect: None,
                key: tab.id.clone(),
                id,
                rect: Rect::new(nx, rect.y, text_width(&tab.title), 1).intersection(rect),
                entry,
            });
            x += w;
        }
        if overflowing {
            let rect = Rect::new(row.right().saturating_sub(3), row.y, 3, 1);
            let hovered = rt.hovered() == Some(self.overflow_id);
            let bg = if hovered { t.of(&look, c.hover) } else { t.bg };
            let style = Style::default().bg(bg);
            fill(buf, rect, rect.x, rect.y, rect.width, style);
            put(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                &look.glyph(Glyph::MoreVertical),
                style.fg(t.icon(&look, Tone::Default)),
            );
            rt.clickable(self.overflow_id, rect);
            rt.tooltip.offer(
                self.overflow_id,
                rect,
                crate::tooltip::Tooltip::new("Show all tabs"),
            );
        }
        if let Some(i) = props.tabs.iter().position(|t| t.selected) {
            let _ = i;
        }
        // The overflow list is rebuilt from the props every frame.
        if let Some(popup) = &mut self.overflow {
            let entries: Vec<MenuEntry> = props
                .tabs
                .iter()
                .map(|tab| {
                    let mut item = MenuItem::new(tab.id.clone(), tab.title.clone());
                    item.icon = Some(match &tab.icon {
                        IconKey::File(name) => IconKey::File(name.clone()),
                        other => other.clone(),
                    });
                    if tab.selected {
                        item.checked = Some(true);
                    }
                    MenuEntry::Item(item)
                })
                .collect();
            popup.source.set_entries(entries);
        }
    }

    /// Draws the overflow list above everything.
    pub fn draw_popup(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        if let Some(popup) = &mut self.overflow {
            popup.draw(rt, buf);
        }
    }

    /// The next time a timer needs a tick.
    pub fn deadline(&self) -> Option<f64> {
        self.overflow.as_ref().and_then(|p| p.deadline())
    }

    /// Runs the overflow list's timers.
    pub fn tick(&mut self, rt: &Runtime) {
        if let Some(popup) = &mut self.overflow {
            popup.tick(rt);
        }
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, props: &TabBar, event: &Event) -> Vec<TabEvent> {
        let mut out = Vec::new();
        if let Some(popup) = &mut self.overflow
            && popup.is_open()
        {
            match popup.event(rt, event) {
                PopupState::Chosen => {
                    if let Some(id) = popup.take_choice() {
                        out.push(TabEvent::Select { id });
                        self.scroll_to_selected = true;
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
        // Track which tab the pointer entered, for the close guard.
        let over_tab: Option<String> = self
            .placed
            .iter()
            .find(|pl| p.over(pl.id))
            .map(|pl| pl.key.clone())
            .or_else(|| {
                self.closes
                    .iter()
                    .find(|(id, _)| p.over(*id))
                    .map(|(_, k)| k.clone())
            })
            .or_else(|| {
                self.pins
                    .iter()
                    .find(|(id, _)| p.over(*id))
                    .map(|(_, k)| k.clone())
            });
        if over_tab != self.hover {
            self.hover.clone_from(&over_tab);
            self.hover_since = rt.now;
        }
        match p.kind {
            PointerKind::Down(MouseButton::Left) => {
                if let Some((_, id)) = self.pins.iter().find(|(i, _)| p.over(*i)) {
                    out.push(TabEvent::Unpin { id: id.clone() });
                } else if self.closes.iter().any(|(i, _)| p.over(*i)) {
                    // Closing happens on release, with the guard.
                } else if let Some(pl) = self.placed.iter().find(|pl| p.over(pl.id)) {
                    if p.shift() {
                        out.push(TabEvent::Close { id: pl.key.clone() });
                    } else {
                        out.push(TabEvent::Select { id: pl.key.clone() });
                        let index = self.order.iter().position(|id| *id == pl.key).unwrap_or(0);
                        self.drag = Some(Drag {
                            index,
                            start_x: p.x,
                            active: false,
                        });
                    }
                }
            }
            PointerKind::Drag(MouseButton::Left) => {
                let centers: Vec<(usize, u16)> = (0..self.order.len())
                    .map(|i| (i, self.tab_center(i)))
                    .collect();
                if let Some(drag) = &mut self.drag {
                    if !drag.active && p.x.abs_diff(drag.start_x) >= DRAG_DEAD_ZONE {
                        drag.active = true;
                    }
                    if drag.active {
                        // Swap with a neighbor when the pointer crosses its center.
                        let target = self.placed.iter().find_map(|pl| {
                            let i = self.order.iter().position(|id| *id == pl.key)?;
                            (i != drag.index
                                && p.x >= pl.rect.x.saturating_sub(2)
                                && p.x < pl.rect.right() + 3)
                                .then_some(i)
                        });
                        if let Some(to) = target {
                            let center = centers
                                .iter()
                                .find(|(i, _)| *i == to)
                                .map_or(0, |(_, c)| *c);
                            let center_ok = if to > drag.index {
                                p.x >= center
                            } else {
                                p.x <= center
                            };
                            if center_ok {
                                out.push(TabEvent::Reorder {
                                    from: drag.index,
                                    to,
                                });
                                drag.index = to;
                            }
                        }
                    }
                }
            }
            PointerKind::Up(MouseButton::Left) => {
                self.drag = None;
                if let Some((_, id)) = self.closes.iter().find(|(i, _)| p.clicked(*i)) {
                    if rt.now - self.hover_since >= CLOSE_GUARD {
                        out.push(TabEvent::Close { id: id.clone() });
                    }
                } else if p.clicked(self.overflow_id) {
                    let entries: Vec<MenuEntry> = props
                        .tabs
                        .iter()
                        .map(|tab| {
                            let mut item = MenuItem::new(tab.id.clone(), tab.title.clone());
                            item.icon = Some(tab.icon.clone());
                            if tab.selected {
                                item.checked = Some(true);
                            }
                            MenuEntry::Item(item)
                        })
                        .collect();
                    let anchor =
                        Rect::new(self.strip.right().saturating_sub(3), self.strip.y, 3, 1);
                    let mut popup =
                        menu_popup(entries, chrome::notation(rt), Placement::Below(anchor));
                    popup.open(rt, false);
                    self.overflow = Some(popup);
                }
            }
            PointerKind::Down(MouseButton::Middle) => {
                if let Some(pl) = self.placed.iter().find(|pl| p.over(pl.id)) {
                    out.push(TabEvent::Close { id: pl.key.clone() });
                }
            }
            PointerKind::Down(MouseButton::Right) => {
                if let Some(pl) = self.placed.iter().find(|pl| p.over(pl.id)) {
                    out.push(TabEvent::Context { id: pl.key.clone() });
                }
            }
            PointerKind::ScrollDown | PointerKind::ScrollRight
                if p.over(self.id) || over_tab.is_some() =>
            {
                self.offset = (self.offset + 1).min(self.order.len().saturating_sub(1));
            }
            PointerKind::ScrollUp | PointerKind::ScrollLeft
                if p.over(self.id) || over_tab.is_some() =>
            {
                self.offset = self.offset.saturating_sub(1);
            }
            _ => {}
        }
        out
    }

    fn tab_center(&self, index: usize) -> u16 {
        self.order
            .get(index)
            .and_then(|id| self.placed.iter().find(|pl| pl.key == *id))
            .map_or(0, |pl| pl.rect.x + pl.rect.width / 2)
    }
}
