//! The interactive status bar: hover feedback, tooltips, clicks on slots, and the hovered
//! action's description (the GUI's status bar info channel).
//!
//! The slots follow NvChad's statusline; the kit's status bar widgets map onto them (a
//! `TextPresentation` is a slot's text, a `MultipleTextValuesPresentation` a slot that opens a
//! list popup when clicked). Click handling is the caller's: [`StatusEvent::Clicked`] says
//! which slot, and the caller opens the popup (for example a [`crate::popup::ListPopup`]
//! above the slot).

use crate::components::{StatusBar, StatusSlot};
use crate::hit::Id;
use crate::input::{Event, MouseButton};
use crate::pointer::PointerKind;
use crate::runtime::Runtime;
use crate::tooltip::Tooltip;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// What the status bar did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusEvent {
    /// Nothing for the caller.
    None,
    /// A slot was clicked (left button, on release over the slot pressed on).
    Clicked(StatusSlot, Rect),
    /// A context menu was requested (right press) on a slot, or on the bar (`None`).
    ContextMenu(Option<StatusSlot>, u16, u16),
}

fn slot_index(slot: StatusSlot) -> u64 {
    match slot {
        StatusSlot::Mode => 0,
        StatusSlot::File => 1,
        StatusSlot::Git => 2,
        StatusSlot::Center => 3,
        StatusSlot::Diagnostics => 4,
        StatusSlot::Lsp => 5,
        StatusSlot::Cwd => 6,
        StatusSlot::Cursor => 7,
        StatusSlot::Run => 8,
    }
}

const SLOTS: [StatusSlot; 9] = [
    StatusSlot::Mode,
    StatusSlot::File,
    StatusSlot::Git,
    StatusSlot::Center,
    StatusSlot::Diagnostics,
    StatusSlot::Lsp,
    StatusSlot::Cwd,
    StatusSlot::Cursor,
    StatusSlot::Run,
];

/// The interactive wrapper of [`StatusBar`].
#[derive(Debug, Clone)]
pub struct StatusBarControl {
    /// The identity; slots use child ids.
    pub id: Id,
    tooltips: Vec<(StatusSlot, Tooltip)>,
    rects: Vec<(StatusSlot, Rect)>,
    bar: Rect,
}

impl StatusBarControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            tooltips: Vec::new(),
            rects: Vec::new(),
            bar: Rect::default(),
        }
    }

    /// Sets the tooltip of a slot.
    #[must_use]
    pub fn tooltip(mut self, slot: StatusSlot, tooltip: Tooltip) -> Self {
        self.tooltips.push((slot, tooltip));
        self
    }

    fn slot_id(&self, slot: StatusSlot) -> Id {
        self.id.with(slot_index(slot))
    }

    /// Draws `bar` in `area` and registers its slots. While an action with a description is
    /// hovered, the description replaces the centered text (`Runtime::hint`).
    pub fn show(&mut self, rt: &mut Runtime, bar: StatusBar<'_>, area: Rect, buf: &mut Buffer) {
        self.bar = area;
        let hovered = SLOTS
            .into_iter()
            .find(|s| rt.hovered() == Some(self.slot_id(*s)));
        let slots = bar.slots(area);
        self.rects = slots.clone();
        bar.hovered(hovered).render(area, buf);
        rt.hits.add(area, self.id);
        for (slot, rect) in &slots {
            rt.clickable(self.slot_id(*slot), *rect);
            if let Some((_, tip)) = self.tooltips.iter().find(|(s, _)| s == slot) {
                rt.tooltip.offer(self.slot_id(*slot), *rect, tip.clone());
            }
        }
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> StatusEvent {
        let Event::Mouse(_) = event else {
            return StatusEvent::None;
        };
        let Some(p) = rt.pointer().copied() else {
            return StatusEvent::None;
        };
        if p.kind == PointerKind::Down(MouseButton::Right)
            && (p.over(self.id) || SLOTS.iter().any(|s| p.over(self.slot_id(*s))))
        {
            let slot = SLOTS.into_iter().find(|s| p.over(self.slot_id(*s)));
            return StatusEvent::ContextMenu(slot, p.x, p.y);
        }
        for (slot, rect) in &self.rects {
            if p.clicked(self.slot_id(*slot)) {
                return StatusEvent::Clicked(*slot, *rect);
            }
        }
        StatusEvent::None
    }
}

// ----- the GUI's status bar: breadcrumbs and an ordered list of widgets -----

use crate::breadcrumbs::BreadcrumbsControl;
use crate::chrome::{self, Placed, Tokens};
use crate::components::{clip, fill, put, text_width};
use archipel_kit_model::{Outline, OutlineEntry, StatusBar as StatusProps, StatusBarEvent};
use ratatui::style::Style;

/// Cells between widgets (the GUI's 18 to 21 px).
const ITEM_GAP: u16 = 3;
/// The braille frames of the busy spinner, 10 per second.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// Cells of a progress bar.
const PROGRESS_W: u16 = 12;
/// The room breadcrumbs keep before widgets are dropped.
const MIN_CRUMBS: u16 = 12;

fn item_cells(item: &archipel_kit_model::StatusBarItem, text: &str) -> u16 {
    let lead = if item.progress.is_some() {
        PROGRESS_W + 1
    } else if item.spinner || item.icon.is_some() {
        2
    } else {
        0
    };
    lead + text_width(text)
}

/// The interactive wrapper of the model's [`StatusBar`](archipel_kit_model::StatusBar): the
/// breadcrumbs at the left, the widgets at the right in the GUI's order. When the row is too
/// narrow the widget with the lowest priority is dropped first, then the next, and a message
/// marked `elide` shortens before anything is dropped; the breadcrumbs shorten from the
/// left.
#[derive(Debug, Clone)]
pub struct StatusItemsControl {
    /// The identity; items use child ids.
    pub id: Id,
    crumbs: BreadcrumbsControl,
    placed: Vec<Placed>,
    pressed: Option<Id>,
    animating: bool,
}

impl StatusItemsControl {
    /// A control.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            crumbs: BreadcrumbsControl::new(id.with(1)),
            placed: Vec::new(),
            pressed: None,
            animating: false,
        }
    }

    /// Whether a spinner is on screen: the app should redraw every 100 ms.
    pub fn animating(&self) -> bool {
        self.animating
    }

    /// Where the item `key` is.
    pub fn rect_of(&self, key: &str) -> Option<Rect> {
        self.placed.iter().find(|p| p.key == key).map(|p| p.rect)
    }

    /// The outline of what was drawn: breadcrumbs, then the widgets, texts from the screen.
    pub fn outline(&self, buf: &Buffer) -> Outline {
        let mut outline = self.crumbs.outline(buf);
        outline
            .entries
            .extend(chrome::outline_of(&self.placed, buf, |_| true).entries);
        outline
    }

    /// Draws `props` in `area` (one row) over the `frame` surface.
    pub fn draw(&mut self, rt: &mut Runtime, props: &StatusProps, area: Rect, buf: &mut Buffer) {
        self.placed.clear();
        self.animating = false;
        let row = Rect::new(area.x, area.y, area.width, 1.min(area.height));
        if row.is_empty() {
            return;
        }
        let look = rt.look.clone();
        let c = &look.theme.colors;
        let notation = chrome::notation(rt);
        let t = Tokens::new(&look, c.frame);
        fill(
            buf,
            row,
            row.x,
            row.y,
            row.width,
            t.style(t.of(&look, c.text)),
        );
        let crumbs_w = BreadcrumbsControl::width_of(&props.breadcrumbs)
            .min((row.width * 3 / 10).max(MIN_CRUMBS));
        // Decide which widgets stay: while the row is too long the widget with the lowest
        // priority goes first (the rightmost among equals); a message that may be elided
        // shortens to what is left before it is dropped.
        let mut keep: Vec<usize> = (0..props.items.len()).collect();
        let mut shown: Vec<String> = props.items.iter().map(|i| i.text.clone()).collect();
        let mut shrunk: Vec<bool> = vec![false; props.items.len()];
        let budget = row.width.saturating_sub(2 + crumbs_w + 2);
        let used = |keep: &[usize], shown: &[String]| -> u16 {
            keep.iter()
                .map(|i| item_cells(&props.items[*i], &shown[*i]))
                .sum::<u16>()
                + ITEM_GAP * keep.len().saturating_sub(1) as u16
        };
        while used(&keep, &shown) > budget && !keep.is_empty() {
            let cand = keep
                .iter()
                .copied()
                .min_by_key(|i| (props.items[*i].priority, std::cmp::Reverse(*i)))
                .unwrap_or(0);
            let shortfall = used(&keep, &shown) - budget;
            let w = text_width(&shown[cand]);
            if props.items[cand].elide && !shrunk[cand] && w > 4 {
                shown[cand] = clip(&props.items[cand].text, w.saturating_sub(shortfall).max(4));
                shrunk[cand] = true;
            } else {
                keep.retain(|i| *i != cand);
            }
        }
        let texts: Vec<(usize, String)> = keep.iter().map(|i| (*i, shown[*i].clone())).collect();
        let total: u16 = texts
            .iter()
            .map(|(i, text)| item_cells(&props.items[*i], text))
            .sum::<u16>()
            + ITEM_GAP * texts.len().saturating_sub(1) as u16;
        let mut x = row.right().saturating_sub(1 + total).max(row.x + 1);
        let items_start = x;
        for (i, text) in &texts {
            let item = &props.items[*i];
            let w = item_cells(item, text);
            let rect = Rect::new(x, row.y, w, 1).intersection(row);
            let id = chrome::child(self.id, &item.id);
            let hovered = rt.hovered() == Some(id) && item.clickable;
            let pressed = self.pressed == Some(id) && hovered;
            let bg = if pressed {
                t.of(&look, c.pressed)
            } else if hovered {
                t.of(&look, c.hover)
            } else {
                t.bg
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let tone_color = t.text(&look, item.tone);
            let mut cx = rect.x;
            if let Some(progress) = item.progress {
                let filled = ((progress.clamp(0.0, 1.0)) * f32::from(PROGRESS_W)).round() as u16;
                for k in 0..PROGRESS_W {
                    let on = k < filled;
                    put(
                        buf,
                        rect,
                        cx + k,
                        rect.y,
                        if on { "█" } else { "░" },
                        Style::default()
                            .fg(if on {
                                t.of(&look, c.accent)
                            } else {
                                t.of(&look, c.progress_track)
                            })
                            .bg(bg),
                    );
                }
                cx += PROGRESS_W + 1;
            } else if item.spinner {
                self.animating = true;
                let frame = if look.ascii {
                    ["|", "/", "-", "\\"][((rt.now * 10.0) as usize) % 4]
                } else {
                    SPINNER[((rt.now * 10.0) as usize) % SPINNER.len()]
                };
                put(
                    buf,
                    rect,
                    cx,
                    rect.y,
                    frame,
                    Style::default().fg(t.of(&look, c.accent)).bg(bg),
                );
                cx += 2;
            } else if let Some(icon) = &item.icon {
                let (ch, color) = chrome::icon_cell(&look, &t, icon, item.tone);
                put(
                    buf,
                    rect,
                    cx,
                    rect.y,
                    &ch,
                    Style::default().fg(color).bg(bg),
                );
                cx += 2;
            }
            put(
                buf,
                rect,
                cx,
                rect.y,
                text,
                Style::default().fg(tone_color).bg(bg),
            );
            if item.clickable {
                rt.clickable(id, rect);
            }
            if let Some(tip) = &item.tooltip {
                rt.tooltip.offer(id, rect, chrome::tooltip(tip, notation));
            }
            let mut entry = OutlineEntry::new("status", item.id.clone(), item.text.clone());
            entry.enabled = item.clickable;
            entry.tooltip = item.tooltip.as_ref().map(|t| t.text(notation));
            let text_rect = Rect::new(cx, row.y, text_width(text), 1).intersection(row);
            self.placed.push(Placed {
                key: item.id.clone(),
                id,
                rect: text_rect,
                entry,
            });
            x += w + ITEM_GAP;
        }
        // Breadcrumbs take the room that is left at the left.
        let crumbs_area = Rect::new(
            row.x + 1,
            row.y,
            items_start.saturating_sub(row.x + 1 + 2),
            1,
        );
        self.crumbs
            .draw(rt, &props.breadcrumbs, crumbs_area, c.frame, buf);
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Vec<StatusBarEvent> {
        let Event::Mouse(_) = event else {
            return Vec::new();
        };
        let Some(p) = rt.pointer().copied() else {
            return Vec::new();
        };
        let mut out: Vec<StatusBarEvent> = Vec::new();
        match p.kind {
            PointerKind::Down(MouseButton::Left) => self.pressed = p.target,
            PointerKind::Up(MouseButton::Left) => {
                self.pressed = None;
                for placed in &self.placed {
                    if p.clicked(placed.id) && placed.entry.enabled {
                        out.push(StatusBarEvent::Click {
                            id: placed.key.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
        for event in self.crumbs.handle(rt, event) {
            if let archipel_kit_model::BreadcrumbEvent::Click { id } = event {
                out.push(StatusBarEvent::Crumb { id });
            }
        }
        out
    }
}
