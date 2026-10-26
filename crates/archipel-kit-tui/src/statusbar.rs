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
