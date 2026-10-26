//! The mouse as the components see it: [`Pointer`] events resolved against the hit map,
//! with click counting and the press origin.

use crate::hit::{HitMap, Id};
use crate::input::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::crossterm::event::KeyModifiers;

/// Two presses of one button within this many seconds at the same cell are a double click.
pub const DOUBLE_CLICK_SECONDS: f64 = 0.5;

/// What the mouse did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PointerKind {
    /// A button went down.
    Down(MouseButton),
    /// A button went up.
    Up(MouseButton),
    /// The pointer moved with a button down.
    Drag(MouseButton),
    /// The pointer moved with no button down.
    Move,
    /// The wheel turned up (away from the user).
    ScrollUp,
    /// The wheel turned down.
    ScrollDown,
    /// The wheel turned left.
    ScrollLeft,
    /// The wheel turned right.
    ScrollRight,
}

/// A mouse event resolved against the hit map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    /// The column.
    pub x: u16,
    /// The row.
    pub y: u16,
    /// What happened.
    pub kind: PointerKind,
    /// The modifier keys held.
    pub mods: KeyModifiers,
    /// The widget under the pointer.
    pub target: Option<Id>,
    /// The widget under the pointer when the current (or just released) button went down.
    pub origin: Option<Id>,
    /// The cell where the button went down.
    pub origin_pos: Option<(u16, u16)>,
    /// How many presses in a row: 1 single, 2 double, 3 triple (for [`PointerKind::Down`] and
    /// [`PointerKind::Up`]).
    pub clicks: u8,
    /// Whether the cell differs from that of the previous mouse event.
    pub moved: bool,
}

impl Pointer {
    /// A left press on `id`.
    pub fn pressed_left(&self, id: Id) -> bool {
        self.kind == PointerKind::Down(MouseButton::Left) && self.target == Some(id)
    }

    /// A left release that completes a click on `id`: pressed on it and released on it.
    pub fn clicked(&self, id: Id) -> bool {
        self.kind == PointerKind::Up(MouseButton::Left)
            && self.target == Some(id)
            && self.origin == Some(id)
    }

    /// Whether the pointer is over `id`.
    pub fn over(&self, id: Id) -> bool {
        self.target == Some(id)
    }

    /// Whether this is a left button press, release or drag.
    pub fn left(&self) -> bool {
        matches!(
            self.kind,
            PointerKind::Down(MouseButton::Left)
                | PointerKind::Up(MouseButton::Left)
                | PointerKind::Drag(MouseButton::Left)
        )
    }

    /// Whether this is a press of the middle button.
    pub fn middle_press(&self) -> bool {
        self.kind == PointerKind::Down(MouseButton::Middle)
    }

    /// Whether this is a press of the right button (a context menu request).
    pub fn right_press(&self) -> bool {
        self.kind == PointerKind::Down(MouseButton::Right)
    }

    /// Whether Shift is held.
    pub fn shift(&self) -> bool {
        self.mods.contains(KeyModifiers::SHIFT)
    }

    /// Whether Ctrl or Cmd is held: the toggle-selection modifier.
    pub fn command(&self) -> bool {
        self.mods
            .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER)
    }

    /// The wheel direction as rows: -1 for up, 1 for down, 0 otherwise.
    pub fn wheel(&self) -> i32 {
        match self.kind {
            PointerKind::ScrollUp => -1,
            PointerKind::ScrollDown => 1,
            _ => 0,
        }
    }

    /// A synthetic event, for tests: `kind` at (`x`, `y`) with no hit-testing (`target` is
    /// resolved by the harness).
    pub fn at(x: u16, y: u16, kind: PointerKind) -> Self {
        Self {
            x,
            y,
            kind,
            mods: KeyModifiers::NONE,
            target: None,
            origin: None,
            origin_pos: None,
            clicks: 1,
            moved: true,
        }
    }
}

/// Turns crossterm mouse events into [`Pointer`]s: tracks the press origin and counts clicks.
#[derive(Debug, Clone, Default)]
pub struct PointerTracker {
    origin: Option<Id>,
    origin_pos: Option<(u16, u16)>,
    last_pos: Option<(u16, u16)>,
    last_press: Option<(f64, u16, u16, MouseButton, u8)>,
    clicks: u8,
}

impl PointerTracker {
    /// Resolves `event` at time `now` against `hits`.
    pub fn update(&mut self, event: &MouseEvent, hits: &HitMap, now: f64) -> Pointer {
        let (x, y) = (event.column, event.row);
        let moved = self.last_pos != Some((x, y));
        self.last_pos = Some((x, y));
        let target = hits.hit(x, y);
        let kind = match event.kind {
            MouseEventKind::Down(b) => PointerKind::Down(b),
            MouseEventKind::Up(b) => PointerKind::Up(b),
            MouseEventKind::Drag(b) => PointerKind::Drag(b),
            MouseEventKind::Moved => PointerKind::Move,
            MouseEventKind::ScrollUp => PointerKind::ScrollUp,
            MouseEventKind::ScrollDown => PointerKind::ScrollDown,
            MouseEventKind::ScrollLeft => PointerKind::ScrollLeft,
            MouseEventKind::ScrollRight => PointerKind::ScrollRight,
        };
        if let PointerKind::Down(button) = kind {
            self.clicks = match self.last_press {
                Some((at, px, py, b, n))
                    if b == button && px == x && py == y && now - at <= DOUBLE_CLICK_SECONDS =>
                {
                    (n + 1).min(3)
                }
                _ => 1,
            };
            self.last_press = Some((now, x, y, button, self.clicks));
            self.origin = target;
            self.origin_pos = Some((x, y));
        }
        let pointer = Pointer {
            x,
            y,
            kind,
            mods: event.modifiers,
            target,
            origin: self.origin,
            origin_pos: self.origin_pos,
            clicks: self.clicks,
            moved,
        };
        if matches!(kind, PointerKind::Up(_)) {
            self.origin = None;
            self.origin_pos = None;
        }
        pointer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn down(x: u16, y: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn two_presses_at_one_cell_are_a_double_click() {
        let mut tracker = PointerTracker::default();
        let hits = HitMap::new();
        assert_eq!(tracker.update(&down(3, 3), &hits, 1.0).clicks, 1);
        assert_eq!(tracker.update(&down(3, 3), &hits, 1.3).clicks, 2);
        assert_eq!(tracker.update(&down(3, 3), &hits, 1.5).clicks, 3);
        assert_eq!(tracker.update(&down(3, 3), &hits, 1.7).clicks, 3);
        assert_eq!(tracker.update(&down(3, 3), &hits, 5.0).clicks, 1);
        assert_eq!(tracker.update(&down(4, 3), &hits, 5.1).clicks, 1);
    }

    #[test]
    fn a_click_needs_press_and_release_on_the_same_widget() {
        let mut tracker = PointerTracker::default();
        let mut hits = HitMap::new();
        let (a, b) = (Id::new("a"), Id::new("b"));
        hits.add(Rect::new(0, 0, 5, 1), a);
        hits.add(Rect::new(5, 0, 5, 1), b);
        let up = |x| MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: x,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        tracker.update(&down(1, 0), &hits, 0.0);
        assert!(tracker.update(&up(2), &hits, 0.1).clicked(a));
        tracker.update(&down(1, 0), &hits, 1.0);
        assert!(!tracker.update(&up(7), &hits, 1.1).clicked(b));
    }
}
