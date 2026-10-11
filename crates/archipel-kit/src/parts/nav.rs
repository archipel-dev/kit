//! Keyboard navigation through a list of items.

use egui::Key;

/// Where `key` moves a selection at `current` in a list of `len` items, `page` items per
/// screen: ↑/↓ by one, Home/End to the ends, PgUp/PgDn by a page. `None` if the key
/// doesn't navigate or the list is empty. With nothing selected, any of these keys selects
/// the first item.
pub fn step(key: Key, current: Option<usize>, len: usize, page: usize) -> Option<usize> {
    let last = len.checked_sub(1)?;
    let page = page.max(1);
    let from = |f: &dyn Fn(usize) -> usize| Some(current.map_or(0, |c| f(c.min(last))));
    match key {
        Key::ArrowDown => from(&|c| (c + 1).min(last)),
        Key::ArrowUp => from(&|c| c.saturating_sub(1)),
        Key::Home => Some(0),
        Key::End => Some(last),
        Key::PageDown => from(&|c| (c + page).min(last)),
        Key::PageUp => from(&|c| c.saturating_sub(page)),
        _ => None,
    }
}

/// The navigation keys [`step`] understands.
pub const KEYS: [Key; 6] = [
    Key::ArrowDown,
    Key::ArrowUp,
    Key::Home,
    Key::End,
    Key::PageDown,
    Key::PageUp,
];

/// Reads the navigation key pressed this frame (if any) and returns the new selection.
pub fn pressed(ui: &egui::Ui, current: Option<usize>, len: usize, page: usize) -> Option<usize> {
    ui.input(|input| {
        KEYS.iter()
            .find(|key| input.key_pressed(**key))
            .and_then(|key| step(*key, current, len, page))
    })
}
