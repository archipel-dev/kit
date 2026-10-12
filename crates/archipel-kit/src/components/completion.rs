//! The completion list of text inputs: suggestions under the caret, operated by the keyboard
//! and the pointer. [`TextField`](super::TextField) and [`CodeEditor`](super::CodeEditor)
//! share it.

use crate::tokens::size;
use egui::text::{CCursor, CCursorRange};
use egui::{Id, Key, Modifiers, Rect, Response, Ui};

/// One suggestion of a [`Completer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// What the list shows.
    pub label: String,
    /// A gray note after the label (a variable's value, a field's type).
    pub detail: String,
    /// The text that replaces [`Completions::start`]..[`Completions::end`].
    pub insert: String,
    /// Where the caret goes in `insert` (chars); `None` puts it after the insertion. A
    /// field with arguments puts it between the parentheses.
    pub caret: Option<usize>,
}

impl Completion {
    /// A suggestion inserting `insert`, listed as `label`.
    pub fn new(label: impl Into<String>, insert: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: String::new(),
            insert: insert.into(),
            caret: None,
        }
    }

    /// A gray note after the label.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Puts the caret `chars` into the insertion instead of after it.
    pub fn caret(mut self, chars: usize) -> Self {
        self.caret = Some(chars);
        self
    }
}

/// The suggestions for the text around the caret.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completions {
    /// Char index where the replaced part starts.
    pub start: usize,
    /// Char index where the replaced part ends: usually the caret, or after a closing
    /// `}}` already typed behind it.
    pub end: usize,
    /// The suggestions, best first.
    pub items: Vec<Completion>,
}

/// Suggests completions for a [`TextField`](super::TextField) or a
/// [`CodeEditor`](super::CodeEditor): `{{variables}}` in a URL, fields in a query. Given the
/// text and the caret (a char index), it returns what to offer, or `None` when nothing
/// completes there. Any `Fn(&str, usize) -> Option<Completions>` is one.
pub trait Completer {
    /// The suggestions at `caret` in `text`, while the user types.
    fn complete(&self, text: &str, caret: usize) -> Option<Completions>;

    /// The suggestions when the user asked for them (<kbd>Ctrl</kbd>+<kbd>Space</kbd>) or the
    /// list is already open. The same as [`complete`](Self::complete) by default; override it
    /// to offer everything where `complete` waits for a typed prefix (a code editor should
    /// not open a list after every `{`).
    fn complete_on_request(&self, text: &str, caret: usize) -> Option<Completions> {
        self.complete(text, caret)
    }
}

impl<F: Fn(&str, usize) -> Option<Completions>> Completer for F {
    fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
        self(text, caret)
    }
}

/// The completer of a combo box ([`TextField::suggestions`](super::TextField::suggestions)):
/// the suggestions containing what was typed (ignoring case), replacing the whole text.
/// Asked for (the chevron, <kbd>Ctrl</kbd>+<kbd>Space</kbd>) on an empty field or one
/// holding a suggestion, it offers them all.
pub(crate) struct Suggestions<'a>(pub(crate) &'a [String]);

impl Suggestions<'_> {
    fn matching(&self, text: &str, all: bool) -> Option<Completions> {
        let needle = text.trim().to_lowercase();
        let items: Vec<Completion> = self
            .0
            .iter()
            .filter(|s| all || s.to_lowercase().contains(&needle))
            .map(|s| Completion::new(s.as_str(), s.as_str()))
            .collect();
        Some(Completions {
            start: 0,
            end: text.chars().count(),
            items,
        })
    }
}

impl Completer for Suggestions<'_> {
    fn complete(&self, text: &str, _caret: usize) -> Option<Completions> {
        if text.trim().is_empty() || self.0.iter().any(|s| s == text) {
            return None;
        }
        self.matching(text, false)
    }

    fn complete_on_request(&self, text: &str, _caret: usize) -> Option<Completions> {
        let all = text.trim().is_empty() || self.0.iter().any(|s| s == text);
        self.matching(text, all)
    }
}

/// Where the list is, between frames.
#[derive(Debug, Clone, Copy, Default)]
struct State {
    open: bool,
    selected: usize,
    count: usize,
}

/// The keys the open list took this frame.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Keys {
    down: bool,
    up: bool,
    accept: bool,
    /// Escape closed the list (egui also took the focus away; the input takes it back).
    pub(crate) escaped: bool,
    /// Ctrl+Space asked for the list.
    explicit: bool,
}

impl Keys {
    /// Enter (or Tab) picked a suggestion: the input must not treat it as its own.
    pub(crate) fn accepted(&self) -> bool {
        self.accept
    }
}

/// The completion list of the input `key` names.
pub(crate) struct List {
    key: Id,
    state: State,
    keys: Keys,
    /// The field the list hangs below, as wide as it (a combo box); else it follows the caret.
    below: Option<Rect>,
}

impl List {
    /// Loads the list and, while it is open, takes the keys it handles before the input
    /// sees them. Call it before the input is drawn; `focused` is whether it has the focus.
    pub(crate) fn begin(ui: &Ui, key: Id, focused: bool) -> (Self, Keys) {
        let key = key.with("archipel-completions");
        let mut state: State = ui.data(|d| d.get_temp(key)).unwrap_or_default();
        let mut keys = Keys::default();
        if focused {
            ui.input_mut(|i| {
                keys.explicit = i.consume_key(Modifiers::CTRL, Key::Space);
                if state.open && state.count > 0 {
                    keys.down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                    keys.up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                    keys.accept = i.consume_key(Modifiers::NONE, Key::Enter)
                        | i.consume_key(Modifiers::NONE, Key::Tab);
                    if i.consume_key(Modifiers::NONE, Key::Escape) {
                        state.open = false;
                        keys.escaped = true;
                    }
                }
            });
        }
        (
            Self {
                key,
                state,
                keys,
                below: None,
            },
            keys,
        )
    }

    /// Opens the list with everything on offer, as <kbd>Ctrl</kbd>+<kbd>Space</kbd> does: a
    /// combo box's chevron.
    pub(crate) fn open(&mut self) {
        self.keys.explicit = true;
    }

    /// Whether the list was open at the start of the frame.
    pub(crate) fn is_open(&self) -> bool {
        self.state.open
    }

    /// Closes the list: a second click on a combo box's chevron.
    pub(crate) fn close(&mut self) {
        self.state.open = false;
    }

    /// Hangs the list below `field`, as wide as it, instead of under the caret.
    pub(crate) fn below(&mut self, field: Rect) {
        self.below = Some(field);
    }

    /// Updates, draws and applies the list after the input was drawn. `edited` is the
    /// input's response (marked changed when a suggestion is inserted), `caret` the caret
    /// (chars) and where it is on screen.
    pub(crate) fn end(
        mut self,
        ui: &Ui,
        completer: &dyn Completer,
        text: &mut String,
        edited: &mut Response,
        caret: Option<(usize, Rect)>,
    ) {
        let keys = self.keys;
        let caret_rect = caret.map(|c| c.1);
        let caret = caret.map(|c| c.0);
        let focused = edited.has_focus() || keys.escaped || keys.explicit;
        if keys.escaped {
            // egui drops the focus on Escape before any widget runs; this Escape only
            // closed the list, so the input keeps it (a second Escape leaves).
            edited.request_focus();
        }
        let Some(caret) = caret else {
            self.state.open = false;
            self.store(ui);
            return;
        };
        let asked = keys.explicit || self.state.open;
        let found = if asked {
            completer.complete_on_request(text, caret)
        } else {
            completer.complete(text, caret)
        }
        .filter(|c| !c.items.is_empty());
        if edited.changed() || keys.explicit {
            self.state.open = true;
            self.state.selected = 0;
        }
        let clicking = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
        if found.is_none() || (!focused && !clicking) {
            self.state.open = false;
        }
        let count = found.as_ref().map_or(0, |c| c.items.len());
        self.state.count = count;
        let last = count.saturating_sub(1);
        if keys.down {
            self.state.selected = (self.state.selected + 1).min(last);
        }
        if keys.up {
            self.state.selected = self.state.selected.saturating_sub(1);
        }
        self.state.selected = self.state.selected.min(last);
        let mut chosen = None;
        if let Some(found) = &found {
            if keys.accept {
                chosen = found.items.get(self.state.selected).cloned();
            }
            if self.state.open
                && let Some(anchor) = self.below.or(caret_rect)
            {
                let mut open = true;
                let selected = self.state.selected;
                let width = self
                    .below
                    .map_or(size::COMPLETION_WIDTH, |field| field.width());
                let picked = crate::parts::Popover::new(self.key.with("list"), anchor)
                    .width(width)
                    .show(ui.ctx(), &mut open, |ui| {
                        let mut picked = None;
                        for (i, item) in found.items.iter().enumerate() {
                            let mut row = super::MenuItem::new(item.label.as_str())
                                .highlighted(i == selected);
                            if !item.detail.is_empty() {
                                row = row.shortcut(item.detail.as_str());
                            }
                            if ui.add(row).clicked() {
                                picked = Some(item.clone());
                            }
                        }
                        picked
                    });
                if let Some(Some(item)) = picked {
                    chosen = Some(item);
                }
                self.state.open &= open;
            }
        }
        if let (Some(item), Some(found)) = (chosen, &found) {
            apply(
                ui.ctx(),
                edited.id,
                text,
                found.start,
                found.end.max(found.start),
                &item,
            );
            edited.mark_changed();
            self.state.open = false;
        }
        self.store(ui);
    }

    fn store(&self, ui: &Ui) {
        ui.data_mut(|d| d.insert_temp(self.key, self.state));
    }
}

/// Replaces the chars `start..end` of `text` with the completion, puts the caret where it
/// asks and keeps the focus in the input `id`.
fn apply(
    ctx: &egui::Context,
    id: Id,
    text: &mut String,
    start: usize,
    end: usize,
    item: &Completion,
) {
    let byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b)
    };
    let (from, to) = (byte(start), byte(end));
    text.replace_range(from..to, &item.insert);
    let inserted = item.insert.chars().count();
    let after = start + item.caret.unwrap_or(inserted).min(inserted);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(after))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}
