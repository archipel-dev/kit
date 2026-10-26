//! The completion list as a state machine (IntelliJ `Lookup`), without a screen: when it opens,
//! how typed text narrows and ranks the items, what the keys do, which item is chosen, and how
//! an item is inserted into the editor model at every caret.
//!
//! The GUI list (`ui::completion`) and the terminal menu of `archipel-kit-tui` feed it the
//! keys, the text and the caret of a frame ([`LookupInput`]) and draw what it answers
//! ([`LookupFrame`]); so the two agree on every rule: camel-hump matching, the sort order,
//! the commit characters, Tab replacing the identifier's tail, Ctrl+Space asking twice for more,
//! a lone answer to Ctrl+Space going in at once, Backspace closing on an empty prefix.

use crate::editor::model::{ActionContext, Edit, Editor, EditorAction, Language, UndoGroup};
use crate::tokens::lookup as tk;
use crate::ui::{
    Completer, Completion, Completions, LookupElementPresentation, PrefixMatch, PrefixMatcher,
};

/// The characters that insert the selected item and are then typed themselves (IntelliJ
/// `DefaultCharFilter`: "select and finish").
pub const COMMIT_CHARS: [char; 7] = ['.', ',', ';', '=', ' ', ':', '('];

/// Whether `c` belongs to an identifier: the characters that open the list when typed and
/// narrow it while it is open.
pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where the list is between frames.
#[derive(Debug, Clone, Copy, Default)]
pub struct LookupState {
    /// The list is showing.
    pub open: bool,
    /// The selected row (an index into [`LookupFrame::ranked`]).
    pub selected: usize,
    /// How many rows match.
    pub count: usize,
    /// The first visible row.
    pub top: usize,
    /// Char index where the typed prefix starts: the caret leaving it ahead closes the list.
    pub anchor: usize,
    /// Opened by Ctrl+Space: an empty prefix does not close it.
    pub explicit: bool,
    /// How many times Ctrl+Space was pressed since the list opened.
    pub invocations: u8,
    /// The documentation popup was toggled on (Ctrl+Q).
    pub doc_open: bool,
    /// Input time the selection last changed (for the documentation delay).
    selected_at: f64,
    /// The row `selected_at` belongs to.
    selected_for: usize,
    /// Input time of the typed identifier character waiting for the auto-popup delay.
    pending: Option<f64>,
}

/// The keys the list took in a frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct LookupKeys {
    /// Down arrow.
    pub down: bool,
    /// Up arrow.
    pub up: bool,
    /// Page Down.
    pub page_down: bool,
    /// Page Up.
    pub page_up: bool,
    /// Enter (or Tab) picked the selected item.
    pub accept: bool,
    /// Tab chose: the item replaces the rest of the identifier after the caret.
    pub replace: bool,
    /// A context character (or Ctrl+`.`'s dot) chose and is typed next.
    pub commit: Option<char>,
    /// Escape closed the list.
    pub escaped: bool,
    /// Ctrl+Space asked for the list.
    pub explicit: bool,
    /// The text typed this frame ends in an identifier character.
    pub typed_ident: bool,
    /// Backspace was pressed.
    pub backspace: bool,
    /// The documentation toggle (Ctrl+Q, Ctrl+J, F1) was pressed.
    pub toggle_doc: bool,
    /// Wheel rows over the list (positive scrolls down).
    pub wheel: f32,
}

impl LookupKeys {
    /// Enter (or Tab) picked a suggestion: the input must not treat it as its own.
    pub fn accepted(&self) -> bool {
        self.accept
    }
}

/// An accepted suggestion and what to do around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    /// Char index where the replaced part starts.
    pub start: usize,
    /// Char index where the replaced part ends (the completer's `end`).
    pub end: usize,
    /// The item.
    pub item: Completion,
    /// Tab: the rest of the identifier after the caret goes too.
    pub replace_tail: bool,
    /// A context character typed after the insertion.
    pub then: Option<char>,
}

/// What one frame gives [`LookupState::update`].
pub struct LookupInput<'a> {
    /// The text.
    pub text: &'a str,
    /// The caret in chars; `None` when the input has no caret (closes the list).
    pub caret: Option<usize>,
    /// The keys of the frame.
    pub keys: LookupKeys,
    /// Whether the list is a lookup (filters and ranks by the typed prefix); false for a
    /// combo box's list.
    pub lookup: bool,
    /// The text changed in this frame.
    pub changed: bool,
    /// The input has the keyboard focus (or this frame's keys were the list's).
    pub focused: bool,
    /// The pointer was pressed or released (a click on the list keeps it open).
    pub clicking: bool,
    /// Input time in seconds.
    pub now: f64,
    /// Rows shown before the list scrolls.
    pub max_rows: usize,
}

/// What [`LookupState::update`] answers: the rows to draw and the item chosen.
#[derive(Debug, Clone, Default)]
pub struct LookupFrame {
    /// The list shows after this frame.
    pub open: bool,
    /// What the completer offered, while the list is open.
    pub found: Option<Completions>,
    /// The matching items best first: an index into `found.items` and the match.
    pub ranked: Vec<(usize, Option<PrefixMatch>)>,
    /// What was typed since the anchor (empty for a combo box's list).
    pub prefix: String,
    /// The selected row.
    pub selected: usize,
    /// The first visible row.
    pub top: usize,
    /// The item chosen this frame (Enter, Tab, a commit character, or the lone answer of
    /// Ctrl+Space).
    pub chosen: Option<Chosen>,
    /// `chosen` is the lone answer to Ctrl+Space inserted at once.
    pub auto_inserted: bool,
    /// The documentation to show beside the list now.
    pub doc: Option<String>,
    /// The completer's advertisement line.
    pub advertisement: Option<String>,
    /// The list needs another frame in this many seconds (a delay is running).
    pub repaint_after: Option<f64>,
}

impl LookupFrame {
    /// What each row shows, with the matched fragments.
    pub fn rows(&self) -> Vec<LookupElementPresentation> {
        let Some(found) = &self.found else {
            return Vec::new();
        };
        self.ranked
            .iter()
            .map(|(i, m)| {
                let mut p = found.items[*i].presentation(None);
                if let Some(m) = m {
                    p.fragments = m.fragments.clone();
                }
                p
            })
            .collect()
    }

    /// The choice of the item in `row` (a click or a double click).
    pub fn chosen_row(&self, row: usize) -> Option<Chosen> {
        let found = self.found.as_ref()?;
        let (i, _) = self.ranked.get(row)?;
        Some(Chosen {
            start: found.start,
            end: found.end.max(found.start),
            item: found.items[*i].clone(),
            replace_tail: false,
            then: None,
        })
    }

    fn closed() -> Self {
        Self::default()
    }
}

/// The text between two char indices.
pub fn chars_between(text: &str, from: usize, to: usize) -> String {
    text.chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

/// Char index where the identifier ending at `caret` starts.
pub fn run_start(text: &str, caret: usize) -> usize {
    let before: Vec<char> = text.chars().take(caret).collect();
    let run = before
        .iter()
        .rev()
        .take_while(|c| is_identifier_char(**c))
        .count();
    caret - run
}

/// How many chars after `caret` continue the identifier.
pub fn tail_len(text: &str, caret: usize) -> usize {
    text.chars()
        .skip(caret)
        .take_while(|c| is_identifier_char(*c))
        .count()
}

impl LookupState {
    /// Closes the list.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Selects `row` (the pointer moved over it).
    pub fn select(&mut self, row: usize, now: f64) {
        self.selected = row;
        self.selected_for = row;
        self.selected_at = now;
    }

    /// Runs one frame: what the caret did to an open list, what may open it, the filtering
    /// and ranking, the keys, and the choice.
    pub fn update(&mut self, completer: &dyn Completer, input: &LookupInput<'_>) -> LookupFrame {
        let LookupInput {
            text,
            caret,
            keys,
            lookup,
            changed,
            focused,
            clicking,
            now,
            max_rows,
        } = *input;
        let Some(caret) = caret else {
            self.open = false;
            return LookupFrame::closed();
        };
        let mut repaint_after: Option<f64> = None;
        let mut want_repaint = |after: f64| {
            repaint_after = Some(repaint_after.map_or(after, |a| a.min(after)));
        };
        let was_open = self.open;
        let mut open = was_open;

        // What the caret did to an open list: it left the prefix, or the prefix is no longer
        // an identifier (a typed `)`), or Backspace emptied it.
        if was_open && lookup {
            if caret < self.anchor {
                open = false;
            } else {
                let prefix = chars_between(text, self.anchor, caret);
                if prefix.chars().any(|c| !is_identifier_char(c) && c != '-')
                    || (keys.backspace && prefix.is_empty() && !self.explicit)
                {
                    open = false;
                }
            }
        }

        // What may open it: Ctrl+Space, or an edit (an identifier character, in a lookup),
        // after the completer's delay.
        let delay = if lookup {
            completer.auto_popup_delay()
        } else {
            0.0
        };
        let mut auto = changed && (keys.typed_ident || !lookup) && !open;
        if delay > 0.0 && lookup {
            if auto {
                self.pending = Some(now);
                auto = false;
            } else if changed {
                self.pending = None;
            }
            if let Some(since) = self.pending {
                let remaining = f64::from(delay) - (now - since);
                if remaining <= 0.0 && focused {
                    self.pending = None;
                    auto = !open;
                } else if focused {
                    want_repaint(remaining.max(0.0));
                } else {
                    self.pending = None;
                }
            }
        }
        if keys.explicit {
            self.invocations = if was_open {
                self.invocations.saturating_add(1)
            } else {
                1
            };
            if !was_open {
                self.explicit = true;
            }
        }
        let asked = keys.explicit || open;
        let mut found = None;
        if asked || auto {
            found = if (keys.explicit || open) && self.invocations >= 2 {
                completer.complete_extended(text, caret)
            } else if asked {
                completer.complete_on_request(text, caret)
            } else {
                completer.complete(text, caret)
            }
            .filter(|c| !c.items.is_empty());
        }
        let opening = !open && (keys.explicit || auto) && found.is_some();
        if opening {
            open = true;
            self.selected = 0;
            self.top = 0;
            self.doc_open = false;
            if !keys.explicit {
                self.explicit = false;
                self.invocations = 0;
            }
            self.anchor = match &found {
                Some(found) if lookup => run_start(text, caret).max(found.start),
                _ => caret,
            };
        } else if changed && open {
            // The prefix changed: the best match is selected again.
            self.selected = 0;
            self.top = 0;
        }

        // Filter, match and sort by the typed prefix.
        let prefix = if lookup && open {
            chars_between(text, self.anchor.min(caret), caret)
        } else {
            String::new()
        };
        let matcher = PrefixMatcher::new(&prefix);
        let mut ranked: Vec<(usize, Option<PrefixMatch>)> = Vec::new();
        if let Some(found) = &found {
            if lookup {
                for (i, item) in found.items.iter().enumerate() {
                    if let Some(m) = matcher.matches(item.lookup()) {
                        ranked.push((i, Some(m)));
                    }
                }
                ranked.sort_by_key(|(i, m)| {
                    let item = &found.items[*i];
                    (
                        m.as_ref().map(|m| m.kind),
                        m.as_ref().is_some_and(|m| !m.exact_case),
                        std::cmp::Reverse(completer.recency(item.lookup())),
                        *i,
                    )
                });
            } else {
                ranked.extend((0..found.items.len()).map(|i| (i, None)));
            }
        }
        let count = ranked.len();
        if count == 0 || (!focused && !clicking) {
            open = false;
        }

        // A single answer to an explicit request is inserted at once.
        if keys.explicit
            && !was_open
            && open
            && count == 1
            && lookup
            && completer.auto_insert_single()
            && let Some(found) = &found
        {
            self.open = false;
            self.count = 0;
            return LookupFrame {
                chosen: Some(Chosen {
                    start: found.start,
                    end: found.end.max(found.start),
                    item: found.items[ranked[0].0].clone(),
                    replace_tail: false,
                    then: None,
                }),
                auto_inserted: true,
                ..LookupFrame::closed()
            };
        }

        self.open = open;
        self.count = count;
        let last = count.saturating_sub(1);
        let selected_before = self.selected;
        if open {
            if keys.down {
                self.selected = if self.selected >= last {
                    0
                } else {
                    self.selected + 1
                };
            }
            if keys.up {
                self.selected = if self.selected == 0 {
                    last
                } else {
                    self.selected - 1
                };
            }
            let page = max_rows.saturating_sub(1).max(1);
            if keys.page_down {
                self.selected = (self.selected + page).min(last);
            }
            if keys.page_up {
                self.selected = self.selected.saturating_sub(page);
            }
            if keys.toggle_doc {
                self.doc_open = !self.doc_open;
            }
        }
        self.selected = self.selected.min(last);
        let visible = count.min(max_rows);
        let max_top = count.saturating_sub(max_rows);
        if keys.wheel != 0.0 {
            let rows = if keys.wheel.abs() < 1.0 {
                keys.wheel.signum()
            } else {
                keys.wheel.round()
            };
            self.top = (self.top as f32 + rows).clamp(0.0, max_top as f32) as usize;
        } else if self.selected != selected_before || !was_open {
            // The view follows the selection when the selection moves. Not otherwise: after
            // the wheel scrolled the list the selected row may be out of view, and the next
            // frame must not snap back to it.
            if self.selected < self.top {
                self.top = self.selected;
            } else if self.selected >= self.top + visible {
                self.top = self.selected + 1 - visible;
            }
        }
        self.top = self.top.min(max_top);
        if self.selected != self.selected_for || changed || opening {
            self.selected_for = self.selected;
            self.selected_at = now;
        }

        let mut chosen = None;
        let mut doc = None;
        let mut advertisement = None;
        if let (Some(found), true) = (&found, open && count > 0) {
            if keys.accept
                && let Some((i, _)) = ranked.get(self.selected)
            {
                chosen = Some(Chosen {
                    start: found.start,
                    end: found.end.max(found.start),
                    item: found.items[*i].clone(),
                    replace_tail: keys.replace,
                    then: keys.commit,
                });
            }
            let docs = ranked
                .get(self.selected)
                .and_then(|(i, _)| found.items[*i].documentation.clone());
            let auto_docs = completer.auto_show_documentation();
            let visible_docs = docs.is_some()
                && (self.doc_open
                    || (auto_docs && now - self.selected_at >= f64::from(tk::DOC_DELAY)));
            if auto_docs && !self.doc_open {
                let wait = f64::from(tk::DOC_DELAY) - (now - self.selected_at);
                if wait > 0.0 {
                    want_repaint(wait);
                }
            }
            if visible_docs {
                doc = docs;
            }
            advertisement = completer.advertisement();
        }
        if chosen.is_some() {
            self.open = false;
        }
        LookupFrame {
            open: open && count > 0,
            found,
            ranked,
            prefix,
            selected: self.selected,
            top: self.top,
            chosen,
            auto_inserted: false,
            doc,
            advertisement,
            repaint_after,
        }
    }
}

/// Inserts the chosen item as one undo step.
///
/// The primary caret decides what is replaced: the typed prefix before it (from the
/// completer's start) and, for Tab, the rest of the identifier after it. Every other caret
/// whose text before it ends in the same prefix gets the same replacement; carets elsewhere
/// are left alone. The carets end where the item's caret offset says, and a context character
/// is typed at all carets afterwards.
pub fn insert_chosen(editor: &mut Editor, language: &Language, chosen: &Chosen) {
    let before = editor.carets.snapshot();
    let text = editor.document.text().to_owned();
    let byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b)
    };
    let primary = editor.carets.primary().offset.min(text.len());
    let from = byte(chosen.start).min(primary);
    let end = byte(chosen.end.max(chosen.start));
    let left = primary - from;
    let prefix = &text[from..primary];
    let right_of = |offset: usize| {
        let mut right = if offset == primary {
            end.saturating_sub(primary)
        } else {
            0
        };
        if chosen.replace_tail {
            let tail: usize = text[offset..]
                .chars()
                .take_while(|c| is_identifier_char(*c))
                .map(char::len_utf8)
                .sum();
            right = right.max(tail);
        }
        right
    };

    let inserted = &chosen.item.insert;
    let chars = inserted.chars().count();
    let into: usize = inserted
        .chars()
        .take(chosen.item.caret.unwrap_or(chars).min(chars))
        .map(char::len_utf8)
        .sum();

    let mut targets: Vec<(usize, usize, usize)> = Vec::new();
    let mut last_end = 0;
    let offsets: Vec<(usize, usize, bool)> = editor
        .carets
        .carets()
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.offset.min(text.len()), c.has_selection()))
        .collect();
    for (index, offset, selected) in offsets {
        let is_primary = offset == primary;
        if (selected && !is_primary) || offset < left {
            continue;
        }
        let start = offset - left;
        if !is_primary && text.get(start..offset) != Some(prefix) {
            continue;
        }
        let stop = (offset + right_of(offset)).min(text.len());
        if text.get(offset..stop).is_none() || start < last_end {
            continue;
        }
        last_end = stop;
        targets.push((index, start, stop));
    }

    let mut edits: Vec<Edit> = Vec::new();
    let mut moved: Vec<(usize, isize)> = Vec::new();
    let mut delta: isize = 0;
    for (index, start, stop) in targets {
        let (s, e) = (
            (start as isize + delta) as usize,
            (stop as isize + delta) as usize,
        );
        if let Some(edit) = editor.document.replace_string(s, e, inserted) {
            editor.carets.apply_edit(&edit);
            editor.folding.apply_edit(&edit);
            edits.push(edit);
        }
        moved.push((index, s as isize + into as isize));
        delta += inserted.len() as isize - (stop - start) as isize;
    }
    for (index, offset) in moved {
        if let Some(caret) = editor.carets.carets_mut().get_mut(index) {
            caret.move_to(offset as usize);
        }
    }
    editor.carets.normalize();
    editor
        .undo
        .record(edits, before, editor.carets.snapshot(), UndoGroup::None);
    if let Some(c) = chosen.then {
        editor.execute(&EditorAction::Type(c), &ActionContext::new(language));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str, caret: usize) -> Option<Completions> {
        let start = run_start(text, caret);
        Some(Completions {
            start,
            end: caret,
            items: ["getString", "get", "size", "Gap"]
                .iter()
                .map(|w| Completion::new(*w, *w))
                .collect(),
        })
    }

    fn input<'a>(text: &'a str, caret: usize, keys: LookupKeys) -> LookupInput<'a> {
        LookupInput {
            text,
            caret: Some(caret),
            keys,
            lookup: true,
            changed: false,
            focused: true,
            clicking: false,
            now: 0.0,
            max_rows: 8,
        }
    }

    fn typed(text: &str, caret: usize) -> LookupInput<'_> {
        LookupInput {
            changed: true,
            keys: LookupKeys {
                typed_ident: true,
                ..LookupKeys::default()
            },
            ..input(text, caret, LookupKeys::default())
        }
    }

    #[test]
    fn typing_an_identifier_opens_and_ranks_by_the_prefix() {
        let mut state = LookupState::default();
        let frame = state.update(&words, &typed("ge", 2));
        assert!(frame.open);
        let labels: Vec<_> = frame.rows().iter().map(|r| r.item_text.clone()).collect();
        assert_eq!(
            labels,
            ["getString", "get"],
            "prefix matches keep the offered order"
        );
        assert_eq!(frame.prefix, "ge");
        assert_eq!(state.anchor, 0);
    }

    #[test]
    fn camel_humps_match_and_a_wrong_case_sorts_later() {
        let mut state = LookupState::default();
        let frame = state.update(&words, &typed("gS", 2));
        let labels: Vec<_> = frame.rows().iter().map(|r| r.item_text.clone()).collect();
        assert_eq!(labels, ["getString"]);
        let mut state = LookupState::default();
        let frame = state.update(&words, &typed("ga", 2));
        assert_eq!(frame.rows()[0].item_text, "Gap");
    }

    #[test]
    fn keys_move_wrap_and_accept() {
        let mut state = LookupState::default();
        state.update(&words, &typed("g", 1));
        let down = LookupKeys {
            down: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("g", 1, down));
        assert_eq!(frame.selected, 1);
        let up = LookupKeys {
            up: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("g", 1, up));
        assert_eq!(frame.selected, 0);
        let frame = state.update(&words, &input("g", 1, up));
        assert_eq!(frame.selected, frame.ranked.len() - 1, "wraps");
        let accept = LookupKeys {
            accept: true,
            replace: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("g", 1, accept));
        let chosen = frame.chosen.expect("chosen");
        assert!(chosen.replace_tail);
        assert!(!state.open);
    }

    #[test]
    fn a_commit_character_chooses_and_is_typed_after() {
        let mut state = LookupState::default();
        state.update(&words, &typed("si", 2));
        let keys = LookupKeys {
            accept: true,
            commit: Some('.'),
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("si", 2, keys));
        let chosen = frame.chosen.expect("chosen");
        assert_eq!(chosen.item.label, "size");
        assert_eq!(chosen.then, Some('.'));
    }

    #[test]
    fn the_caret_leaving_the_prefix_or_a_non_identifier_closes() {
        let mut state = LookupState::default();
        state.update(&words, &typed("a ge", 4));
        assert!(state.open);
        // The caret moved in front of the prefix.
        let frame = state.update(&words, &input("a ge", 1, LookupKeys::default()));
        assert!(!frame.open);
        let mut state = LookupState::default();
        state.update(&words, &typed("ge", 2));
        let frame = state.update(&words, &input("ge)", 3, LookupKeys::default()));
        assert!(!frame.open, "a typed `)` ends the identifier");
    }

    #[test]
    fn backspace_on_an_empty_prefix_closes_unless_it_was_asked_for() {
        let mut state = LookupState::default();
        state.update(&words, &typed("g", 1));
        let keys = LookupKeys {
            backspace: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("", 0, keys));
        assert!(!frame.open);
        // Ctrl+Space on an empty prefix stays open through Backspace.
        let mut state = LookupState::default();
        let ask = LookupKeys {
            explicit: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("", 0, ask));
        assert!(frame.open);
        let frame = state.update(&words, &input("", 0, keys));
        assert!(frame.open);
    }

    #[test]
    fn a_lone_answer_to_ctrl_space_goes_in_at_once() {
        let mut state = LookupState::default();
        let ask = LookupKeys {
            explicit: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&words, &input("si", 2, ask));
        assert!(frame.auto_inserted);
        assert_eq!(frame.chosen.expect("chosen").item.label, "size");
        assert!(!state.open);
    }

    #[test]
    fn the_second_ctrl_space_asks_for_more() {
        use std::cell::Cell;
        struct Counting(Cell<u8>);
        impl Completer for Counting {
            fn complete(&self, _: &str, _: usize) -> Option<Completions> {
                None
            }
            fn complete_on_request(&self, text: &str, caret: usize) -> Option<Completions> {
                words(text, caret)
            }
            fn complete_extended(&self, text: &str, caret: usize) -> Option<Completions> {
                self.0.set(self.0.get() + 1);
                words(text, caret)
            }
        }
        let completer = Counting(Cell::new(0));
        let mut state = LookupState::default();
        let ask = LookupKeys {
            explicit: true,
            ..LookupKeys::default()
        };
        state.update(&completer, &input("g", 1, ask));
        assert_eq!(completer.0.get(), 0);
        state.update(&completer, &input("g", 1, ask));
        assert_eq!(completer.0.get(), 1);
        assert_eq!(state.invocations, 2);
    }

    #[test]
    fn the_documentation_shows_when_toggled() {
        struct Docs;
        impl Completer for Docs {
            fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
                let mut c = words(text, caret)?;
                c.items[0] = Completion::new("getString", "getString").documentation("Gets it.");
                Some(c)
            }
        }
        let mut state = LookupState::default();
        let frame = state.update(&Docs, &typed("get", 3));
        assert_eq!(frame.doc, None);
        let keys = LookupKeys {
            toggle_doc: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&Docs, &input("get", 3, keys));
        assert_eq!(frame.doc.as_deref(), Some("Gets it."));
    }

    #[test]
    fn the_list_scrolls_to_keep_the_selection_visible() {
        let many = |text: &str, caret: usize| {
            Some(Completions {
                start: run_start(text, caret),
                end: caret,
                items: (0..30)
                    .map(|i| Completion::new(format!("a{i:02}"), format!("a{i:02}")))
                    .collect(),
            })
        };
        let mut state = LookupState::default();
        state.update(&many, &typed("a", 1));
        let keys = LookupKeys {
            page_down: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&many, &input("a", 1, keys));
        assert_eq!(frame.selected, 7);
        let frame = state.update(&many, &input("a", 1, keys));
        assert_eq!(frame.selected, 14);
        assert_eq!(frame.top, 7);
    }

    #[test]
    fn wheeling_a_long_list_keeps_the_view_until_the_selection_moves() {
        let many = |text: &str, caret: usize| {
            Some(Completions {
                start: run_start(text, caret),
                end: caret,
                items: (0..30)
                    .map(|i| Completion::new(format!("a{i:02}"), format!("a{i:02}")))
                    .collect(),
            })
        };
        let mut state = LookupState::default();
        state.update(&many, &typed("a", 1));
        let wheel = LookupKeys {
            wheel: 5.0,
            ..LookupKeys::default()
        };
        let frame = state.update(&many, &input("a", 1, wheel));
        assert_eq!((frame.top, frame.selected), (5, 0));
        // The next frame without the wheel does not snap back to the selected row.
        let frame = state.update(&many, &input("a", 1, LookupKeys::default()));
        assert_eq!(frame.top, 5);
        // Moving the selection brings it into view again.
        let down = LookupKeys {
            down: true,
            ..LookupKeys::default()
        };
        let frame = state.update(&many, &input("a", 1, down));
        assert_eq!((frame.top, frame.selected), (1, 1));
    }

    #[test]
    fn insertion_goes_to_every_caret_with_the_same_prefix() {
        let mut editor = Editor::new("ge\nge\nxx\n");
        // The primary caret is the newest: its prefix `ge` decides.
        editor.carets.set_caret(&editor.document, 5);
        editor.carets.add_caret(&editor.document, 8);
        editor.carets.add_caret(&editor.document, 2);
        let chosen = Chosen {
            start: 0,
            end: 2,
            item: Completion::new("getString", "getString()").caret(10),
            replace_tail: false,
            then: None,
        };
        insert_chosen(&mut editor, &Language::c_like(), &chosen);
        assert_eq!(editor.document.text(), "getString()\ngetString()\nxx\n");
        let offsets: Vec<_> = editor.carets.carets().iter().map(|c| c.offset).collect();
        assert_eq!(offsets[..2], [10, 22]);
        // One undo step.
        editor.execute(
            &EditorAction::Undo,
            &ActionContext::new(&Language::c_like()),
        );
        assert_eq!(editor.document.text(), "ge\nge\nxx\n");
    }

    #[test]
    fn tab_replaces_the_tail_of_the_identifier_and_a_commit_char_follows() {
        let mut editor = Editor::new("foo.barBaz");
        editor.carets.set_caret(&editor.document, 6);
        let chosen = Chosen {
            start: 4,
            end: 6,
            item: Completion::new("bar", "bar"),
            replace_tail: true,
            then: Some('('),
        };
        insert_chosen(&mut editor, &Language::c_like(), &chosen);
        assert!(
            editor.document.text().starts_with("foo.bar("),
            "{}",
            editor.document.text()
        );
        assert!(!editor.document.text().contains("Baz"));
    }
}
