//! Widget ids and the per-frame hit map.
//!
//! Every interactive component registers a rectangle under its [`Id`] while it draws. The
//! map is rebuilt every frame, so it always matches the screen. Mouse events are resolved
//! against it: the topmost (last registered) rectangle that contains the pointer wins, and a
//! modal layer registers a [block](HitMap::block) first so that nothing below it is hit.

use ratatui::layout::{Position, Rect};

/// A stable widget identity: a hash of a name, optionally combined with an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u64);

impl Id {
    /// The id for `name` (FNV-1a hash).
    pub const fn new(name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
            i += 1;
        }
        Id(hash)
    }

    /// A child id: this id combined with `index` (a row, a tab, a segment).
    pub const fn with(self, index: u64) -> Self {
        let mut hash = self.0 ^ index.wrapping_add(0x9e37_79b9_7f4a_7c15);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
        hash ^= hash >> 29;
        Id(hash)
    }
}

#[derive(Debug, Clone, Copy)]
struct Hit {
    rect: Rect,
    id: Option<Id>,
}

/// The rectangles of this frame's interactive widgets, back to front.
#[derive(Debug, Clone, Default)]
pub struct HitMap {
    hits: Vec<Hit>,
}

impl HitMap {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything; called before each frame is drawn.
    pub fn clear(&mut self) {
        self.hits.clear();
    }

    /// Registers `rect` for `id`, above everything registered so far.
    pub fn add(&mut self, rect: Rect, id: Id) {
        if !rect.is_empty() {
            self.hits.push(Hit { rect, id: Some(id) });
        }
    }

    /// Blocks everything registered so far inside `rect` (a modal layer's area): a pointer
    /// there hits nothing below.
    pub fn block(&mut self, rect: Rect) {
        if !rect.is_empty() {
            self.hits.push(Hit { rect, id: None });
        }
    }

    /// The topmost widget at (`x`, `y`), if any.
    pub fn hit(&self, x: u16, y: u16) -> Option<Id> {
        let pos = Position::new(x, y);
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(pos))
            .and_then(|hit| hit.id)
    }

    /// The last registered rectangle of `id`.
    pub fn rect_of(&self, id: Id) -> Option<Rect> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.id == Some(id))
            .map(|hit| hit.rect)
    }

    /// Whether (`x`, `y`) hits `id`.
    pub fn is_hit(&self, id: Id, x: u16, y: u16) -> bool {
        self.hit(x, y) == Some(id)
    }

    /// How many rectangles are registered.
    pub fn len(&self) -> usize {
        self.hits.len()
    }

    /// Whether nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topmost_registered_rect_wins() {
        let mut map = HitMap::new();
        let (a, b) = (Id::new("a"), Id::new("b"));
        map.add(Rect::new(0, 0, 10, 10), a);
        map.add(Rect::new(2, 2, 3, 3), b);
        assert_eq!(map.hit(3, 3), Some(b));
        assert_eq!(map.hit(1, 1), Some(a));
        assert_eq!(map.hit(20, 20), None);
    }

    #[test]
    fn a_block_hides_what_is_below() {
        let mut map = HitMap::new();
        let a = Id::new("a");
        map.add(Rect::new(0, 0, 10, 10), a);
        map.block(Rect::new(0, 0, 10, 10));
        assert_eq!(map.hit(1, 1), None);
        map.add(Rect::new(0, 0, 2, 2), Id::new("popup"));
        assert_eq!(map.hit(1, 1), Some(Id::new("popup")));
    }

    #[test]
    fn child_ids_differ() {
        let id = Id::new("tabs");
        assert_ne!(id.with(0), id.with(1));
        assert_ne!(id.with(0), id);
    }
}
