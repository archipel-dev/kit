//! The completion popup of text inputs (IntelliJ `Lookup`): suggestions under the caret,
//! operated by the keyboard and the pointer. [`JBTextField`](crate::ui::JBTextField) and
//! [`EditorComponent`](crate::editor::EditorComponent) share it.
//!
//! A [`Completer`] (IntelliJ `CompletionContributor`) answers with [`Completion`]s
//! ([`LookupElement`]s); the list filters them by what was typed ([`PrefixMatcher`], camel
//! humps included), sorts them, draws them and reports the one chosen. See `docs/components/
//! code.md` for the keys.

use super::lookup::{LookupElementPresentation, PrefixMatch, PrefixMatcher, is_identifier_char};
use crate::Theme;
use crate::icons::Icon;
use crate::tokens::{lookup as tk, popup, radius, scroll, size, typography};
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{Color32, FontId, Id, Key, Modifiers, Rect, Response, Stroke, Ui, pos2, vec2};
use std::sync::Arc;

/// One suggestion of a [`Completer`] (IntelliJ `LookupElement` with its
/// `LookupElementPresentation`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// What the list shows (the item text).
    pub label: String,
    /// A gray note at the row's right edge (a variable's value, a field's type): the type
    /// text.
    pub detail: String,
    /// The text that replaces [`Completions::start`]..[`Completions::end`].
    pub insert: String,
    /// Where the caret goes in `insert` (chars); `None` puts it after the insertion. A
    /// field with arguments puts it between the parentheses.
    pub caret: Option<usize>,
    /// A gray note right after the label (a method's parameters): the tail text.
    pub tail: String,
    /// The icon before the label.
    pub icon: Option<Icon>,
    /// The label is drawn bold.
    pub bold: bool,
    /// The label is struck out (deprecated).
    pub strikeout: bool,
    /// What the typed text is matched against; the label when `None`.
    pub lookup_string: Option<String>,
    /// The text of the documentation popup (<kbd>Ctrl</kbd>+<kbd>Q</kbd>).
    pub documentation: Option<String>,
}

/// A suggestion: IntelliJ's name for [`Completion`].
pub type LookupElement = Completion;

impl Completion {
    /// A suggestion inserting `insert`, listed as `label`.
    pub fn new(label: impl Into<String>, insert: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: String::new(),
            insert: insert.into(),
            caret: None,
            tail: String::new(),
            icon: None,
            bold: false,
            strikeout: false,
            lookup_string: None,
            documentation: None,
        }
    }

    /// A gray note at the row's right edge: the type text.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// The type text (IntelliJ `setTypeText`): the same as [`detail`](Self::detail).
    pub fn type_text(self, text: impl Into<String>) -> Self {
        self.detail(text)
    }

    /// A gray note right after the label (IntelliJ `setTailText`).
    pub fn tail_text(mut self, text: impl Into<String>) -> Self {
        self.tail = text.into();
        self
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Draws the label bold (a preferred item).
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Strikes the label out (a deprecated item).
    pub fn strikeout(mut self) -> Self {
        self.strikeout = true;
        self
    }

    /// The text the typed prefix is matched against, when it is not the label.
    pub fn lookup_string(mut self, text: impl Into<String>) -> Self {
        self.lookup_string = Some(text.into());
        self
    }

    /// The text of the documentation popup beside the list.
    pub fn documentation(mut self, text: impl Into<String>) -> Self {
        self.documentation = Some(text.into());
        self
    }

    /// Puts the caret `chars` into the insertion instead of after it.
    pub fn caret(mut self, chars: usize) -> Self {
        self.caret = Some(chars);
        self
    }

    /// The text the typed prefix is matched against.
    pub fn lookup(&self) -> &str {
        self.lookup_string.as_deref().unwrap_or(&self.label)
    }

    /// What a row shows for this item, with the fragments `matcher` matched.
    pub fn presentation(&self, matcher: Option<&PrefixMatcher>) -> LookupElementPresentation {
        let fragments = matcher
            .filter(|_| self.lookup_string.is_none())
            .and_then(|m| m.matches(&self.label))
            .map(|m| m.fragments)
            .unwrap_or_default();
        LookupElementPresentation {
            item_text: self.label.clone(),
            tail_text: self.tail.clone(),
            type_text: self.detail.clone(),
            icon: self.icon,
            bold: self.bold,
            strikeout: self.strikeout,
            fragments,
        }
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
    /// The suggestions, best first: the list keeps this order among items that match the
    /// typed text equally well.
    pub items: Vec<Completion>,
}

/// Suggests completions for a [`JBTextField`](crate::ui::JBTextField) or a
/// [`EditorComponent`](crate::editor::EditorComponent) (IntelliJ `CompletionContributor`):
/// `{{variables}}` in a URL, fields in a query. Given the text and the caret (a char index),
/// it returns what to offer, or `None` when nothing completes there. Any
/// `Fn(&str, usize) -> Option<Completions>` is one.
///
/// The list narrows what is returned by what the user typed between the identifier start and
/// the caret, so a completer may return everything it knows at the caret.
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

    /// The suggestions of a second <kbd>Ctrl</kbd>+<kbd>Space</kbd> while the list is open
    /// (IntelliJ's second invocation shows more: static members, other packages). The same
    /// as [`complete_on_request`](Self::complete_on_request) by default.
    fn complete_extended(&self, text: &str, caret: usize) -> Option<Completions> {
        self.complete_on_request(text, caret)
    }

    /// Whether the list filters and sorts what the completer returns by the typed prefix
    /// (camel humps included) and follows the caret keys of a lookup. Default `true`; a
    /// completer that filters itself and replaces whole values (a combo box) returns `false`.
    fn filters_by_prefix(&self) -> bool {
        true
    }

    /// How recently the user chose the item with this lookup string: higher is more recent,
    /// 0 never. Recent items sort first among equally good matches. The caller keeps the
    /// statistics ([`LookupStats`](crate::ui::LookupStats)); the kit keeps none.
    fn recency(&self, _lookup_string: &str) -> u64 {
        0
    }

    /// Seconds between a typed identifier character and the popup opening (IntelliJ: none).
    fn auto_popup_delay(&self) -> f32 {
        0.0
    }

    /// Whether an explicit <kbd>Ctrl</kbd>+<kbd>Space</kbd> that finds exactly one item
    /// inserts it at once (`AUTOCOMPLETE_ON_CODE_COMPLETION`, default on).
    fn auto_insert_single(&self) -> bool {
        true
    }

    /// A line under the rows (an advertiser: "Press Ctrl+Enter to ...").
    fn advertisement(&self) -> Option<String> {
        None
    }

    /// Whether the documentation of the selected item shows by itself after 500 ms (off in
    /// IntelliJ; <kbd>Ctrl</kbd>+<kbd>Q</kbd> toggles it either way).
    fn auto_show_documentation(&self) -> bool {
        false
    }
}

impl<F: Fn(&str, usize) -> Option<Completions>> Completer for F {
    fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
        self(text, caret)
    }
}

/// The completer of a combo box ([`JBTextField::suggestions`](crate::ui::JBTextField::suggestions)):
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

    fn filters_by_prefix(&self) -> bool {
        false
    }

    fn auto_insert_single(&self) -> bool {
        false
    }
}

/// The characters that insert the selected item and are then typed themselves
/// (IntelliJ `DefaultCharFilter`: "select and finish").
const COMMIT_CHARS: [char; 7] = ['.', ',', ';', '=', ' ', ':', '('];

/// Where the list is, between frames.
#[derive(Debug, Clone, Copy, Default)]
struct State {
    open: bool,
    selected: usize,
    count: usize,
    /// First visible row.
    top: usize,
    /// Char index where the typed prefix starts: the caret leaving it ahead closes the list.
    anchor: usize,
    /// Opened by <kbd>Ctrl</kbd>+<kbd>Space</kbd>: an empty prefix does not close it.
    explicit: bool,
    /// How many times <kbd>Ctrl</kbd>+<kbd>Space</kbd> was pressed since the list opened.
    invocations: u8,
    /// The documentation popup was toggled on (<kbd>Ctrl</kbd>+<kbd>Q</kbd>).
    doc_open: bool,
    /// Input time the selection last changed (for the documentation delay).
    selected_at: f64,
    /// The row `selected_at` belongs to.
    selected_for: usize,
    /// Input time of the typed identifier character waiting for the auto-popup delay.
    pending: Option<f64>,
    /// Where the popup was drawn, for the wheel and the clicks.
    rect: Option<Rect>,
}

/// The keys the open list took this frame.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Keys {
    down: bool,
    up: bool,
    page_down: bool,
    page_up: bool,
    accept: bool,
    /// Tab chose: the item replaces the rest of the identifier after the caret.
    replace: bool,
    /// A context character (or <kbd>Ctrl</kbd>+<kbd>.</kbd>'s dot) chose and is typed next.
    commit: Option<char>,
    /// Escape closed the list (egui also took the focus away; the input takes it back).
    pub(crate) escaped: bool,
    /// Ctrl+Space asked for the list.
    explicit: bool,
    /// The text typed this frame ends in an identifier character.
    typed_ident: bool,
    /// Backspace was pressed.
    backspace: bool,
    /// The documentation toggle was pressed.
    toggle_doc: bool,
    /// Wheel rows over the popup.
    wheel: f32,
}

impl Keys {
    /// Enter (or Tab) picked a suggestion: the input must not treat it as its own.
    pub(crate) fn accepted(&self) -> bool {
        self.accept
    }
}

/// An accepted suggestion and what to do around it.
pub(crate) struct Chosen {
    /// Char index where the replaced part starts.
    pub(crate) start: usize,
    /// Char index where the replaced part ends (the completer's `end`).
    pub(crate) end: usize,
    pub(crate) item: Completion,
    /// Tab: the rest of the identifier after the caret goes too.
    pub(crate) replace_tail: bool,
    /// A context character typed after the insertion.
    pub(crate) then: Option<char>,
}

/// The completion list of the input `key` names.
pub(crate) struct List {
    key: Id,
    state: State,
    keys: Keys,
    lookup: bool,
    /// The field the list hangs below, as wide as it (a combo box); else it follows the caret.
    below: Option<Rect>,
    /// The editor's monospace font and cell width.
    code: Option<(FontId, f32)>,
}

impl List {
    /// Loads the list and, while it is open, takes the keys it handles before the input
    /// sees them. Call it before the input is drawn; `focused` is whether it has the focus.
    pub(crate) fn begin(
        ui: &Ui,
        key: Id,
        focused: bool,
        completer: Option<&dyn Completer>,
    ) -> (Self, Keys) {
        let key = key.with("archipel-completions");
        let mut state: State = ui.data(|d| d.get_temp(key)).unwrap_or_default();
        let lookup = completer.is_none_or(|c| c.filters_by_prefix());
        let mut keys = Keys::default();
        if focused {
            let pointer_over = state
                .rect
                .zip(ui.input(|i| i.pointer.hover_pos()))
                .is_some_and(|(rect, pos)| state.open && rect.contains(pos));
            ui.input_mut(|i| {
                keys.explicit = i.consume_key(Modifiers::CTRL, Key::Space);
                keys.typed_ident = i.events.iter().any(|e| {
                    matches!(e, egui::Event::Text(t) if t.chars().last().is_some_and(is_identifier_char))
                });
                keys.backspace = i.key_pressed(Key::Backspace);
                if state.open && state.count > 0 {
                    keys.down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                    keys.up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                    keys.page_down = i.consume_key(Modifiers::NONE, Key::PageDown);
                    keys.page_up = i.consume_key(Modifiers::NONE, Key::PageUp);
                    keys.accept = i.consume_key(Modifiers::NONE, Key::Enter);
                    if i.consume_key(Modifiers::NONE, Key::Tab) {
                        keys.accept = true;
                        keys.replace = true;
                    }
                    if lookup {
                        if i.consume_key(Modifiers::CTRL, Key::Period) {
                            keys.accept = true;
                            keys.commit = Some('.');
                        }
                        keys.toggle_doc = i.consume_key(Modifiers::CTRL, Key::Q)
                            | i.consume_key(Modifiers::CTRL, Key::J)
                            | i.consume_key(Modifiers::NONE, Key::F1);
                        let typed = i.events.iter().position(|e| {
                            matches!(e, egui::Event::Text(t)
                                if t.chars().count() == 1 && t.chars().all(|c| COMMIT_CHARS.contains(&c)))
                        });
                        if let Some(index) = typed
                            && let egui::Event::Text(t) = i.events.remove(index)
                        {
                            keys.accept = true;
                            keys.commit = t.chars().next();
                        }
                    }
                    if i.consume_key(Modifiers::NONE, Key::Escape) {
                        state.open = false;
                        keys.escaped = true;
                    }
                    if pointer_over && i.raw_scroll_delta.y != 0.0 {
                        keys.wheel = -i.raw_scroll_delta.y / tk::ROW_HEIGHT;
                        i.raw_scroll_delta = egui::Vec2::ZERO;
                        i.smooth_scroll_delta = egui::Vec2::ZERO;
                    }
                }
            });
        }
        (
            Self {
                key,
                state,
                keys,
                lookup,
                below: None,
                code: None,
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

    /// Draws the rows in the editor's monospace `font` of cell width `cell`, and lines the
    /// item text up with the typed prefix.
    pub(crate) fn code_font(&mut self, font: FontId, cell: f32) {
        self.code = Some((font, cell));
    }

    /// Updates, draws and applies the list after the input was drawn. `edited` is the
    /// input's response (marked changed when a suggestion is inserted), `caret` the caret
    /// (chars) and where it is on screen.
    pub(crate) fn end(
        self,
        ui: &Ui,
        completer: &dyn Completer,
        text: &mut String,
        edited: &mut Response,
        caret: Option<(usize, Rect)>,
    ) -> Option<Completion> {
        let chosen = self.end_with(ui, completer, text.as_str(), edited, caret);
        let chosen = chosen?;
        let item = chosen.item.clone();
        apply(ui.ctx(), edited.id, text, &chosen);
        Some(item)
    }

    /// Like [`end`](Self::end) for an input that is not a `String` with an egui caret (the
    /// code editor): returns the accepted suggestion instead of applying it.
    pub(crate) fn end_with(
        mut self,
        ui: &Ui,
        completer: &dyn Completer,
        text: &str,
        edited: &mut Response,
        caret: Option<(usize, Rect)>,
    ) -> Option<Chosen> {
        let keys = self.keys;
        let ctx = ui.ctx();
        let now = ui.input(|i| i.time);
        let caret_rect = caret.map(|c| c.1);
        let caret = caret.map(|c| c.0);
        // Tab moves the focus before the input runs (a single-line field): the list took the
        // key, so the field is still the one being completed.
        let focused = edited.has_focus() || keys.escaped || keys.explicit || keys.accept;
        if keys.escaped {
            // egui drops the focus on Escape before any widget runs; this Escape only
            // closed the list, so the input keeps it (a second Escape leaves).
            edited.request_focus();
        }
        let Some(caret) = caret else {
            self.state.open = false;
            self.store(ui);
            return None;
        };
        let lookup = self.lookup;
        let was_open = self.state.open;
        let changed = edited.changed();
        let mut open = was_open;

        // What the caret did to an open list: it left the prefix, or the prefix is no longer
        // an identifier (a typed `)`), or Backspace emptied it.
        if was_open && lookup {
            if caret < self.state.anchor {
                open = false;
            } else {
                let prefix = chars_between(text, self.state.anchor, caret);
                if prefix.chars().any(|c| !is_identifier_char(c) && c != '-')
                    || (keys.backspace && prefix.is_empty() && !self.state.explicit)
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
                self.state.pending = Some(now);
                auto = false;
            } else if changed {
                self.state.pending = None;
            }
            if let Some(since) = self.state.pending {
                let remaining = f64::from(delay) - (now - since);
                if remaining <= 0.0 && focused {
                    self.state.pending = None;
                    auto = !open;
                } else if focused {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                        remaining.max(0.0),
                    ));
                } else {
                    self.state.pending = None;
                }
            }
        }
        if keys.explicit {
            self.state.invocations = if was_open {
                self.state.invocations.saturating_add(1)
            } else {
                1
            };
            if !was_open {
                self.state.explicit = true;
            }
        }
        let asked = keys.explicit || open;
        let mut found = None;
        if asked || auto {
            found = if (keys.explicit || open) && self.state.invocations >= 2 {
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
            self.state.selected = 0;
            self.state.top = 0;
            self.state.doc_open = false;
            if !keys.explicit {
                self.state.explicit = false;
                self.state.invocations = 0;
            }
            self.state.anchor = match &found {
                Some(found) if lookup => run_start(text, caret).max(found.start),
                _ => caret,
            };
        } else if changed && open {
            // The prefix changed: the best match is selected again.
            self.state.selected = 0;
            self.state.top = 0;
        }

        // Filter, match and sort by the typed prefix.
        let prefix = if lookup && open {
            chars_between(text, self.state.anchor.min(caret), caret)
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
        let clicking = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
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
            self.state.open = false;
            self.state.count = 0;
            self.store(ui);
            edited.mark_changed();
            return Some(Chosen {
                start: found.start,
                end: found.end.max(found.start),
                item: found.items[ranked[0].0].clone(),
                replace_tail: false,
                then: None,
            });
        }

        self.state.open = open;
        self.state.count = count;
        let last = count.saturating_sub(1);
        if open {
            if keys.down {
                self.state.selected = if self.state.selected >= last {
                    0
                } else {
                    self.state.selected + 1
                };
            }
            if keys.up {
                self.state.selected = if self.state.selected == 0 {
                    last
                } else {
                    self.state.selected - 1
                };
            }
            let page = tk::MAX_ROWS - 1;
            if keys.page_down {
                self.state.selected = (self.state.selected + page).min(last);
            }
            if keys.page_up {
                self.state.selected = self.state.selected.saturating_sub(page);
            }
            if keys.toggle_doc {
                self.state.doc_open = !self.state.doc_open;
            }
        }
        self.state.selected = self.state.selected.min(last);
        let visible = count.min(tk::MAX_ROWS);
        let max_top = count.saturating_sub(tk::MAX_ROWS);
        if keys.wheel != 0.0 {
            let rows = if keys.wheel.abs() < 1.0 {
                keys.wheel.signum()
            } else {
                keys.wheel.round()
            };
            self.state.top = (self.state.top as f32 + rows).clamp(0.0, max_top as f32) as usize;
        } else if self.state.selected < self.state.top {
            self.state.top = self.state.selected;
        } else if self.state.selected >= self.state.top + visible {
            self.state.top = self.state.selected + 1 - visible;
        }
        self.state.top = self.state.top.min(max_top);
        if self.state.selected != self.state.selected_for || changed || opening {
            self.state.selected_for = self.state.selected;
            self.state.selected_at = now;
        }

        let mut chosen = None;
        if let (Some(found), true) = (&found, open && count > 0) {
            if keys.accept
                && let Some((i, _)) = ranked.get(self.state.selected)
            {
                chosen = Some(Chosen {
                    start: found.start,
                    end: found.end.max(found.start),
                    item: found.items[*i].clone(),
                    replace_tail: keys.replace,
                    then: keys.commit,
                });
            }
            let rows: Vec<LookupElementPresentation> = ranked
                .iter()
                .map(|(i, m)| {
                    let mut p = found.items[*i].presentation(None);
                    if let Some(m) = m {
                        p.fragments = m.fragments.clone();
                    }
                    p
                })
                .collect();
            if let Some(field) = self.below {
                // A combo box's list is a plain popup list below the field, not a lookup:
                // one click picks a row.
                let mut still_open = true;
                let selected = self.state.selected;
                let picked = crate::ui::popup::JBPopup::new(self.key.with("list"), field)
                    .width(field.width())
                    .show(ctx, &mut still_open, |ui| {
                        let mut picked = None;
                        for (i, row) in rows.iter().enumerate() {
                            let mut item =
                                crate::action_system::ActionMenuItem::new(row.item_text.as_str())
                                    .highlighted(i == selected);
                            if !row.type_text.is_empty() {
                                item = item.shortcut(row.type_text.as_str());
                            }
                            if ui.add(item).clicked() {
                                picked = Some(i);
                            }
                        }
                        picked
                    });
                if let Some(Some(row)) = picked
                    && let Some((i, _)) = ranked.get(row)
                {
                    chosen = Some(Chosen {
                        start: found.start,
                        end: found.end.max(found.start),
                        item: found.items[*i].clone(),
                        replace_tail: false,
                        then: None,
                    });
                }
                self.state.open &= still_open;
            } else if let Some(anchor) = caret_rect {
                let selected = self.state.selected;
                let docs = ranked
                    .get(selected)
                    .and_then(|(i, _)| found.items[*i].documentation.clone());
                let doc_visible = docs.is_some()
                    && (self.state.doc_open
                        || (completer.auto_show_documentation()
                            && now - self.state.selected_at >= f64::from(tk::DOC_DELAY)));
                if completer.auto_show_documentation() && !self.state.doc_open {
                    let wait = f64::from(tk::DOC_DELAY) - (now - self.state.selected_at);
                    if wait > 0.0 {
                        ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
                    }
                }
                let prefix_width = if self.below.is_some() {
                    0.0
                } else {
                    self.prefix_width(ui, &prefix)
                };
                let shown = show_popup(
                    ui,
                    PopupArgs {
                        key: self.key,
                        rows: &rows,
                        selected,
                        top: self.state.top,
                        anchor,
                        field: self.below,
                        prefix_width,
                        font: self.font(),
                        advertisement: completer.advertisement(),
                        docs: doc_visible.then_some(docs).flatten(),
                    },
                );
                self.state.rect = Some(shown.rect);
                if let Some(row) = shown.activate {
                    if let Some((i, _)) = ranked.get(row) {
                        chosen = Some(Chosen {
                            start: found.start,
                            end: found.end.max(found.start),
                            item: found.items[*i].clone(),
                            replace_tail: false,
                            then: None,
                        });
                    }
                } else if let Some(row) = shown.select {
                    self.state.selected = row;
                    self.state.selected_for = row;
                    self.state.selected_at = now;
                }
                if shown.pressed_inside {
                    // The input keeps the keyboard while the pointer works the list.
                    edited.request_focus();
                }
            }
        } else {
            self.state.rect = None;
        }
        if chosen.is_some() {
            edited.mark_changed();
            self.state.open = false;
        }
        self.store(ui);
        chosen
    }

    fn font(&self) -> FontId {
        self.code
            .as_ref()
            .map_or_else(|| typography::body(typography::BODY), |(f, _)| f.clone())
    }

    /// The width of the typed prefix on screen, to line the item text up with it.
    fn prefix_width(&self, ui: &Ui, prefix: &str) -> f32 {
        match &self.code {
            Some((_, cell)) => prefix.chars().count() as f32 * cell,
            None => {
                ui.painter()
                    .layout_no_wrap(prefix.to_owned(), self.font(), Color32::PLACEHOLDER)
                    .size()
                    .x
            }
        }
    }

    fn store(&self, ui: &Ui) {
        ui.data_mut(|d| d.insert_temp(self.key, self.state));
    }
}

/// The text between two char indices.
fn chars_between(text: &str, from: usize, to: usize) -> String {
    text.chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

/// Char index where the identifier ending at `caret` starts.
fn run_start(text: &str, caret: usize) -> usize {
    let before: Vec<char> = text.chars().take(caret).collect();
    let run = before
        .iter()
        .rev()
        .take_while(|c| is_identifier_char(**c))
        .count();
    caret - run
}

/// How many chars after `caret` continue the identifier.
pub(crate) fn tail_len(text: &str, caret: usize) -> usize {
    text.chars()
        .skip(caret)
        .take_while(|c| is_identifier_char(*c))
        .count()
}

/// Replaces the chars of the accepted completion in `text`, puts the caret where it asks
/// (then types the context character) and keeps the focus in the input `id`.
fn apply(ctx: &egui::Context, id: Id, text: &mut String, chosen: &Chosen) {
    let state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let caret = state
        .cursor
        .char_range()
        .map_or(chosen.end, |r| r.primary.index);
    let start = chosen.start;
    let mut end = chosen.end;
    if chosen.replace_tail {
        end = end.max(caret + tail_len(text, caret));
    }
    let item = &chosen.item;
    let byte = |text: &str, chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b)
    };
    let (from, to) = (byte(text, start), byte(text, end));
    text.replace_range(from..to, &item.insert);
    let inserted = item.insert.chars().count();
    let mut after = start + item.caret.unwrap_or(inserted).min(inserted);
    if let Some(c) = chosen.then {
        let at = byte(text, after);
        text.insert(at, c);
        after += 1;
    }
    let mut state = state;
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(after))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

struct PopupArgs<'a> {
    key: Id,
    rows: &'a [LookupElementPresentation],
    selected: usize,
    top: usize,
    anchor: Rect,
    field: Option<Rect>,
    prefix_width: f32,
    font: FontId,
    advertisement: Option<String>,
    docs: Option<String>,
}

struct Shown {
    rect: Rect,
    select: Option<usize>,
    activate: Option<usize>,
    pressed_inside: bool,
}

/// One row, laid out.
struct RowLayout {
    item: Arc<egui::Galley>,
    tail: Option<Arc<egui::Galley>>,
    kind: Option<Arc<egui::Galley>>,
    icon: Option<Icon>,
}

/// Draws the popup and the documentation beside it. Returns where it is and what the pointer
/// did.
fn show_popup(ui: &Ui, a: PopupArgs<'_>) -> Shown {
    let ctx = ui.ctx();
    let colors = Theme::colors(ctx);
    let n = a.rows.len();
    let visible = n.min(tk::MAX_ROWS);
    let bold_font = if a.font.family == egui::FontFamily::Monospace {
        typography::mono_bold(ctx, a.font.size)
    } else {
        typography::semibold(ctx, a.font.size)
    };
    let small = FontId::new(a.font.size, a.font.family.clone());
    let layout = |row: &LookupElementPresentation| -> RowLayout {
        let mut job = LayoutJob::default();
        let chars: Vec<char> = row.item_text.chars().collect();
        let mut position = 0;
        let push = |from: usize, to: usize, matched: bool, job: &mut LayoutJob| {
            if from >= to {
                return;
            }
            let part: String = chars[from..to].iter().collect();
            job.append(
                &part,
                0.0,
                TextFormat {
                    font_id: if row.bold {
                        bold_font.clone()
                    } else {
                        a.font.clone()
                    },
                    color: if matched { colors.link } else { colors.text },
                    strikethrough: if row.strikeout {
                        Stroke::new(1.0_f32, colors.text_secondary)
                    } else {
                        Stroke::NONE
                    },
                    ..Default::default()
                },
            );
        };
        for fragment in &row.fragments {
            let (from, to) = (
                fragment.start.min(chars.len()),
                fragment.end.min(chars.len()),
            );
            push(position, from, false, &mut job);
            push(from.max(position), to, true, &mut job);
            position = position.max(to);
        }
        push(position, chars.len(), false, &mut job);
        let gray = |text: &str| {
            (!text.is_empty()).then(|| {
                ui.painter()
                    .layout_no_wrap(text.to_owned(), small.clone(), colors.text_secondary)
            })
        };
        RowLayout {
            item: ui.painter().layout_job(job),
            tail: gray(&row.tail_text),
            kind: gray(&row.type_text),
            icon: row.icon,
        }
    };
    let any_icon = a.rows.iter().any(|r| r.icon.is_some());
    let icon_col = if any_icon {
        size::ICON + size::ICON_TEXT_GAP
    } else {
        0.0
    };
    let row_width = |l: &RowLayout| {
        tk::ICON_INSET
            + icon_col
            + l.item.size().x
            + l.tail.as_ref().map_or(0.0, |t| tk::TAIL_GAP + t.size().x)
            + l.kind.as_ref().map_or(0.0, |t| tk::TYPE_GAP + t.size().x)
            + tk::RIGHT_PADDING
    };
    let screen = ctx.content_rect();
    let frame_extra = 2.0 * (tk::BODY_INSET + 1.0);
    let inner_w = match a.field {
        Some(field) => field.width() - frame_extra,
        None => {
            let widest = a
                .rows
                .iter()
                .take(300)
                .map(|r| row_width(&layout(r)))
                .fold(0.0, f32::max);
            (widest + 2.0 * tk::SELECTION_INSET)
                .max(tk::MIN_WIDTH - frame_extra)
                .min((screen.width() - frame_extra - 8.0).max(40.0))
        }
    };
    let advert_h = a.advertisement.as_ref().map(|_| {
        ui.painter()
            .layout_no_wrap(
                "M".to_owned(),
                FontId::proportional(typography::SMALL),
                Color32::PLACEHOLDER,
            )
            .size()
            .y
            + 2.0 * tk::ADVERTISER_INSET_Y
    });
    let rows_h = visible as f32 * tk::ROW_HEIGHT;
    let outer = vec2(
        inner_w + frame_extra,
        rows_h + advert_h.unwrap_or(0.0) + frame_extra,
    );

    // Below the caret's line at the start of the prefix, above it when there is no room.
    let (mut x, y) = match a.field {
        Some(field) => {
            let below = field.bottom() + size::POPUP_PADDING;
            let y = if below + outer.y > screen.bottom() && field.top() > outer.y {
                field.top() - outer.y - size::POPUP_PADDING
            } else {
                below
            };
            (field.left(), y)
        }
        None => {
            let text_x = a.anchor.left() - a.prefix_width;
            let lead = tk::BODY_INSET + 1.0 + tk::SELECTION_INSET + tk::ICON_INSET + icon_col;
            let below = a.anchor.bottom() + tk::CARET_GAP;
            let y = if below + outer.y > screen.bottom() && a.anchor.top() - outer.y > screen.top()
            {
                a.anchor.top() - tk::CARET_GAP - outer.y
            } else {
                below
            };
            (text_x - lead, y)
        }
    };
    x = x
        .min(screen.right() - outer.x - 2.0)
        .max(screen.left() + 2.0);
    let pos = pos2(x, y);

    let mut shown = Shown {
        rect: Rect::NOTHING,
        select: None,
        activate: None,
        pressed_inside: false,
    };
    let click_inserts = a.field.is_some();
    let area = egui::Area::new(a.key.with("list"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            let frame = crate::parts::Surface::Popup
                .frame(ctx)
                .inner_margin(egui::Margin::same(tk::BODY_INSET as i8));
            frame.show(ui, |ui| {
                ui.set_width(inner_w);
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let (rect, _) = ui.allocate_exact_size(vec2(inner_w, rows_h), egui::Sense::hover());
                let painter = ui.painter().clone();
                for k in 0..visible {
                    let i = a.top + k;
                    let Some(row) = a.rows.get(i) else { break };
                    let row_rect = egui::Rect::from_min_size(
                        rect.min + vec2(0.0, k as f32 * tk::ROW_HEIGHT),
                        vec2(inner_w, tk::ROW_HEIGHT),
                    );
                    let pill = row_rect.shrink2(vec2(tk::SELECTION_INSET, 0.0));
                    let response =
                        ui.interact(row_rect, a.key.with(("row", i)), egui::Sense::CLICK);
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            true,
                            i == a.selected,
                            row.item_text.clone(),
                        )
                    });
                    if response.double_clicked() || (click_inserts && response.clicked()) {
                        shown.activate = Some(i);
                    } else if response.clicked() {
                        shown.select = Some(i);
                    }
                    if i == a.selected {
                        painter.rect_filled(pill, radius::ROW, colors.selection_popup);
                    }
                    let l = layout(row);
                    let mut x = pill.left() + tk::ICON_INSET;
                    if let Some(icon) = l.icon {
                        let icon_rect = Rect::from_min_size(
                            pos2(x, pill.center().y - size::ICON / 2.0),
                            vec2(size::ICON, size::ICON),
                        );
                        crate::icons::paint(&painter, icon_rect, icon, colors.text_secondary);
                    }
                    x += icon_col;
                    let center = pill.center().y;
                    painter.galley(
                        pos2(x, center - l.item.size().y / 2.0),
                        l.item.clone(),
                        colors.text,
                    );
                    x += l.item.size().x;
                    if let Some(tail) = &l.tail {
                        painter.galley(
                            pos2(x + tk::TAIL_GAP, center - tail.size().y / 2.0),
                            tail.clone(),
                            colors.text_secondary,
                        );
                    }
                    if let Some(kind) = &l.kind {
                        painter.galley(
                            pos2(
                                pill.right() - tk::RIGHT_PADDING - kind.size().x,
                                center - kind.size().y / 2.0,
                            ),
                            kind.clone(),
                            colors.text_secondary,
                        );
                    }
                }
                if n > visible {
                    let thumb_h = (rows_h * visible as f32 / n as f32).max(size::ICON);
                    let travel = rows_h - thumb_h;
                    let y = rect.top() + travel * a.top as f32 / (n - visible) as f32;
                    let thumb = Rect::from_min_size(
                        pos2(rect.right() - scroll::MARGIN - scroll::THUMB, y),
                        vec2(scroll::THUMB, thumb_h),
                    );
                    painter.rect_filled(
                        thumb,
                        radius::PILL,
                        colors.text.gamma_multiply(scroll::THUMB_OPACITY),
                    );
                }
                if let (Some(text), Some(height)) = (&a.advertisement, advert_h) {
                    let (strip, _) =
                        ui.allocate_exact_size(vec2(inner_w, height), egui::Sense::hover());
                    let full = strip
                        .expand2(vec2(tk::BODY_INSET, 0.0))
                        .expand2(vec2(0.0, 0.0));
                    let full =
                        Rect::from_min_max(full.min, pos2(full.max.x, full.max.y + tk::BODY_INSET));
                    painter.rect_filled(
                        full,
                        egui::CornerRadius {
                            nw: 0,
                            ne: 0,
                            sw: radius::POPUP.saturating_sub(1),
                            se: radius::POPUP.saturating_sub(1),
                        },
                        colors.frame,
                    );
                    painter.text(
                        pos2(
                            strip.left() + tk::ADVERTISER_INSET_X - tk::BODY_INSET,
                            strip.center().y,
                        ),
                        egui::Align2::LEFT_CENTER,
                        text,
                        FontId::proportional(typography::SMALL),
                        colors.text_placeholder,
                    );
                }
            });
        });
    let rect = area.response.rect;
    shown.rect = rect;
    shown.pressed_inside = ctx.input(|i| {
        // egui may take the focus from the input on the press and again on the click.
        (i.pointer.any_pressed() || i.pointer.any_released())
            && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
    });

    if let Some(docs) = &a.docs {
        let doc_w = tk::DOC_WIDTH;
        let right = rect.right() + tk::CARET_GAP;
        // The frame's side margins and stroke.
        let doc_outer = doc_w + 2.0 * size::CONTENT_INSET + 2.0;
        let left_of = right + doc_outer > screen.right()
            && rect.left() - tk::CARET_GAP - doc_outer > screen.left();
        let doc_x = if left_of {
            rect.left() - tk::CARET_GAP - doc_outer
        } else {
            right
        };
        egui::Area::new(a.key.with("docs"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(doc_x, rect.top()))
            .sense(egui::Sense::hover())
            .show(ctx, |ui| {
                crate::parts::Surface::Popup
                    .frame(ctx)
                    .inner_margin(egui::Margin::symmetric(
                        size::CONTENT_INSET as i8,
                        popup::PADDING as i8,
                    ))
                    .show(ui, |ui| {
                        ui.set_width(doc_w);
                        let response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(docs.as_str())
                                    .color(colors.text)
                                    .size(typography::BODY),
                            )
                            .wrap(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Label,
                                true,
                                format!("Documentation: {docs}"),
                            )
                        });
                    });
            });
    }
    shown
}
