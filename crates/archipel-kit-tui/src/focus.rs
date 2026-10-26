//! Keyboard focus: the focus order and the focused widget.

use crate::hit::Id;

/// Which widget has keyboard focus, and the order <kbd>Tab</kbd> walks.
///
/// Widgets register themselves while they draw ([`FocusManager::register`]); the order of
/// registration is the focus order, as the order of drawing is the visual order. A modal
/// layer calls [`FocusManager::trap`] before it registers its widgets, so <kbd>Tab</kbd>
/// cycles inside it. Focus is a plain id: the caller (or the component) decides what a
/// focused widget does with keys.
#[derive(Debug, Clone, Default)]
pub struct FocusManager {
    order: Vec<Id>,
    current: Option<Id>,
    wanted: Option<Id>,
}

impl FocusManager {
    /// A manager with nothing focused.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a frame: forgets the order (the focused id stays).
    pub fn begin_frame(&mut self) {
        self.order.clear();
    }

    /// Adds `id` to the focus order. A pending [`request`](Self::request) for it takes focus.
    pub fn register(&mut self, id: Id) {
        if !self.order.contains(&id) {
            self.order.push(id);
        }
        if self.wanted == Some(id) {
            self.current = Some(id);
            self.wanted = None;
        }
    }

    /// Makes the widgets registered after this call the only ones <kbd>Tab</kbd> reaches
    /// (a modal layer).
    pub fn trap(&mut self) {
        self.order.clear();
    }

    /// Ends a frame: if the focused widget is gone, focus moves to the first one.
    pub fn end_frame(&mut self) {
        if let Some(current) = self.current
            && !self.order.contains(&current)
        {
            self.current = self.order.first().copied();
        }
    }

    /// Focuses `id` now (it must be registered to receive keys).
    pub fn focus(&mut self, id: Id) {
        self.current = Some(id);
        self.wanted = None;
    }

    /// Focuses `id` as soon as it registers (when it is drawn the first time).
    pub fn request(&mut self, id: Id) {
        self.wanted = Some(id);
    }

    /// Removes focus from everything.
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// The focused id.
    pub fn current(&self) -> Option<Id> {
        self.current
    }

    /// Whether `id` has focus.
    pub fn is_focused(&self, id: Id) -> bool {
        self.current == Some(id)
    }

    /// Moves focus to the next widget in order (<kbd>Tab</kbd>), wrapping.
    pub fn next(&mut self) {
        self.step(1);
    }

    /// Moves focus to the previous widget (<kbd>Shift+Tab</kbd>), wrapping.
    pub fn previous(&mut self) {
        self.step(-1);
    }

    fn step(&mut self, by: isize) {
        if self.order.is_empty() {
            return;
        }
        let len = self.order.len() as isize;
        let at = self
            .current
            .and_then(|c| self.order.iter().position(|id| *id == c));
        let next = match at {
            Some(i) => (i as isize + by).rem_euclid(len),
            None if by > 0 => 0,
            None => len - 1,
        };
        self.current = Some(self.order[next as usize]);
    }

    /// Whether `id` is in this frame's focus order.
    pub fn contains(&self, id: Id) -> bool {
        self.order.contains(&id)
    }

    /// The registered ids in order.
    pub fn order(&self) -> &[Id] {
        &self.order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> (FocusManager, [Id; 3]) {
        let ids = [Id::new("a"), Id::new("b"), Id::new("c")];
        let mut m = FocusManager::new();
        m.begin_frame();
        for id in ids {
            m.register(id);
        }
        (m, ids)
    }

    #[test]
    fn tab_wraps_forward_and_back() {
        let (mut m, [a, b, c]) = manager();
        m.next();
        assert_eq!(m.current(), Some(a));
        m.next();
        m.next();
        assert_eq!(m.current(), Some(c));
        m.next();
        assert_eq!(m.current(), Some(a));
        m.previous();
        assert_eq!(m.current(), Some(c));
        let _ = b;
    }

    #[test]
    fn a_trap_limits_the_order() {
        let (mut m, [a, ..]) = manager();
        m.focus(a);
        m.begin_frame();
        m.register(Id::new("base"));
        m.trap();
        let dialog = Id::new("dialog");
        m.register(dialog);
        m.end_frame();
        assert_eq!(m.current(), Some(dialog));
        m.next();
        assert_eq!(m.current(), Some(dialog));
    }

    #[test]
    fn requested_focus_applies_when_the_widget_registers() {
        let mut m = FocusManager::new();
        let field = Id::new("field");
        m.request(field);
        m.begin_frame();
        m.register(field);
        assert!(m.is_focused(field));
    }
}
