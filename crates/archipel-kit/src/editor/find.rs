//! The find and replace bar of the [`EditorComponent`](crate::editor::EditorComponent)
//! (spec section 7): a header docked at the top of the editor, built from kit components.
//!
//! The logic is split like the IDE does it: [`FindState`] holds what the bar edits (query,
//! options, scope), [`FindState::keys`] turns key chords into [`Cmd`]s, [`FindState::apply`]
//! runs them against the model (the current match **is** the editor selection) and [`bar`]
//! draws it.

use super::editor_component::key_modifiers;
use super::model::{
    CaretSnapshot, Editor, KeyModifiers, MAX_SEARCH_RESULTS, Os, SearchHistory, SearchOptions,
    SearchResults, Searcher, find_all, match_at, next_match, word_range_at,
};
use crate::Glyph;
use crate::action_system::{ActionButton, ActionMenuItem};
use crate::theme::Theme;
use crate::tokens::{editor as metrics, radius, size, typography};
use crate::ui::popup::JBPopup;
use crate::ui::{Button, JBTextField};
use egui::{
    Event, Id, Key, Layout, Rect, RichText, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo,
    WidgetType, pos2, text::CCursor, text::CCursorRange, vec2,
};
use std::ops::Range;

/// What a key chord or a button asks the bar to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cmd {
    /// Find Next (Enter, F3, Cmd+G).
    Next,
    /// Find Previous (Shift+Enter, Shift+F3).
    Previous,
    /// Replace the current match, then select the next one.
    Replace,
    /// Replace every match as one undo step.
    ReplaceAll,
    /// One caret per match.
    SelectAll,
    /// Ctrl+Enter: remember the query and go back to the editor.
    Accept,
    /// Find the word at the caret (Ctrl+F3 / Ctrl+Shift+F3); the flag is forward.
    WordAtCaret(bool),
}

/// Where the carets and the view were when the search started: restored when the query is
/// cleared.
#[derive(Clone, Debug)]
struct Origin {
    carets: CaretSnapshot,
    scroll: Vec2,
}

/// The first Next past the last match shows a hint and waits for a second press.
#[derive(Clone, Debug)]
struct PendingWrap {
    forward: bool,
    carets: CaretSnapshot,
}

/// The search the live selection last ran for.
type Applied = (String, SearchOptions, Option<Range<usize>>);

/// Find state of one view, kept in the egui context between frames.
#[derive(Clone, Default)]
pub(super) struct FindState {
    pub open: bool,
    /// The replace row is shown (Ctrl+R).
    pub replace: bool,
    pub query: String,
    pub replacement: String,
    pub options: SearchOptions,
    /// Find in selection: matches are searched in this range only.
    pub scope: Option<Range<usize>>,
    /// Move focus to the query field on the next frame (and select its text).
    pub focus_query: bool,
    pub focus_replace: bool,
    pub select_field_text: bool,
    base: Option<Id>,
    pub history_open: bool,
    pub history_pick: usize,
    pub cmds: Vec<Cmd>,
    /// The query to remember when the bar was closed this frame.
    close_history: Option<String>,
    origin: Option<Origin>,
    search_from: usize,
    applied: Option<Applied>,
    wrap: Option<PendingWrap>,
    synced: CaretSnapshot,
}

/// What the keys of a frame did that the component must act on.
#[derive(Default)]
pub(super) struct KeysOut {
    /// The bar was closed (Escape): the editor takes the focus back.
    pub closed: bool,
}

/// What [`FindState::apply`] did that the component must act on.
#[derive(Default)]
pub(super) struct ApplyOut {
    pub text_changed: bool,
    pub hint: Option<String>,
    /// Scroll here (and unfold) so the match shows.
    pub jump: Option<usize>,
    /// The query was cleared: the view goes back here.
    pub restore_scroll: Option<Vec2>,
    /// Focus moves to the editor (Ctrl+Enter, Select All Occurrences).
    pub focus_editor: bool,
}

/// What [`FindState::apply`] needs besides the editor.
pub(super) struct ApplyEnv {
    pub os: Os,
    pub viewer: bool,
    /// The text has the keyboard focus (a move of the carets then is the user's).
    pub editor_focused: bool,
    pub scroll: Vec2,
}

impl FindState {
    pub fn load(ctx: &egui::Context, id: Id) -> Self {
        let mut state: Self = ctx
            .data(|d| d.get_temp(id.with("archipel-code-find")))
            .unwrap_or_default();
        state.base = Some(id);
        state
    }

    pub fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id.with("archipel-code-find"), self));
    }

    /// The id of the query field.
    pub fn field(&self) -> Id {
        self.base.unwrap_or(Id::NULL).with("archipel-find-query")
    }

    /// The id of the replace field.
    pub fn replace_field(&self) -> Id {
        self.base.unwrap_or(Id::NULL).with("archipel-find-replace")
    }

    fn focused(&self, ctx: &egui::Context, id: Id) -> bool {
        self.open && ctx.memory(|m| m.has_focus(id))
    }

    /// Whether the query field has the keyboard focus.
    pub fn query_focused(&self, ctx: &egui::Context) -> bool {
        self.focused(ctx, self.field())
    }

    fn replace_focused(&self, ctx: &egui::Context) -> bool {
        self.replace && self.focused(ctx, self.replace_field())
    }

    /// Whether one of the fields has the keyboard focus.
    pub fn field_focused(&self, ctx: &egui::Context) -> bool {
        self.query_focused(ctx) || self.replace_focused(ctx)
    }

    /// The key of the search, to tell a changed query from an unchanged one.
    fn key(&self) -> Applied {
        (self.query.clone(), self.options, self.scope.clone())
    }

    /// Height of the bar for this state, 0 while closed.
    pub fn height(&self, viewer: bool) -> f32 {
        if !self.open {
            return 0.0;
        }
        let lines = self.query_rows();
        let rows = if self.replace && !viewer { 2.0 } else { 1.0 };
        rows * metrics::FIND_ROW_HEIGHT + (lines - 1) as f32 * size::CODE_LINE_HEIGHT
    }

    fn query_rows(&self) -> usize {
        (self.query.split('\n').count()).clamp(1, metrics::FIND_MAX_QUERY_ROWS)
    }

    /// Opens the bar (Ctrl+F, Ctrl+R, F3): a single-line selection becomes the query, a
    /// multi-line one the scope of the search; otherwise the previous query stays. The query
    /// field gets the focus with its text selected.
    pub fn open_with(&mut self, editor: &Editor, replace: bool, seed: bool, focus: bool) {
        if !self.open {
            self.origin = None;
            self.wrap = None;
        }
        self.open = true;
        self.replace = replace;
        if seed {
            let primary = editor.carets.primary();
            if primary.has_selection() {
                let (s, e) = (primary.selection_start(), primary.selection_end());
                let text = editor.document.slice(s, e);
                if text.contains('\n') {
                    self.scope = Some(s..e);
                } else {
                    self.query = text.to_owned();
                    self.scope = None;
                }
            } else {
                self.scope = None;
            }
            self.search_from = primary.selection_start();
        }
        if focus {
            self.focus_query = true;
            self.select_field_text = true;
        }
        self.synced = editor.carets.snapshot();
        // Opening alone does not move the selection: only edits of the query do.
        self.applied = Some(self.key());
    }

    fn close(&mut self) {
        if self.open {
            self.close_history = Some(self.query.clone());
        }
        self.open = false;
        self.history_open = false;
        self.origin = None;
        self.wrap = None;
        self.cmds.clear();
    }

    /// Takes the keys of the bar and of find from the frame's input: Ctrl+F, Ctrl+R, F3,
    /// the keys in the fields, Escape. `editor_focused` is the text's focus.
    #[allow(clippy::too_many_arguments)]
    pub fn keys(
        &mut self,
        ui: &Ui,
        os: Os,
        editor_focused: bool,
        viewer: bool,
        editor: &Editor,
        history: &[String],
    ) -> KeysOut {
        let mut out = KeysOut::default();
        let ctx = ui.ctx();
        let query_focused = self.query_focused(ctx);
        let replace_focused = self.replace_focused(ctx);
        if !(editor_focused || query_focused || replace_focused) {
            return out;
        }
        let primary = match os {
            Os::WindowsLinux => KeyModifiers::CTRL,
            Os::MacOs => KeyModifiers::CMD,
        };
        let shift = |m: KeyModifiers| KeyModifiers { shift: true, ..m };
        let take = |key: Key, want: KeyModifiers| take_key(ui, os, key, want);
        // Open.
        let alt_f3 = os == Os::WindowsLinux && take(Key::F3, KeyModifiers::ALT);
        if take(Key::F, primary) || alt_f3 {
            self.open_with(
                editor,
                false,
                editor_focused && !self.field_focused(ctx),
                true,
            );
        } else if take(Key::R, primary) {
            self.open_with(
                editor,
                !viewer,
                editor_focused && !self.field_focused(ctx),
                true,
            );
        }
        // Next and previous, also from the editor.
        let next = take(Key::F3, KeyModifiers::NONE)
            || match os {
                Os::WindowsLinux => take(Key::L, KeyModifiers::CTRL),
                Os::MacOs => take(Key::G, KeyModifiers::CMD),
            };
        let previous = take(Key::F3, KeyModifiers::SHIFT)
            || match os {
                Os::WindowsLinux => take(Key::L, shift(KeyModifiers::CTRL)),
                Os::MacOs => take(Key::G, shift(KeyModifiers::CMD)),
            };
        let word_next = os == Os::WindowsLinux && take(Key::F3, KeyModifiers::CTRL);
        let word_previous = os == Os::WindowsLinux && take(Key::F3, shift(KeyModifiers::CTRL));
        if (next || previous) && !self.open && !self.query.is_empty() {
            self.open_with(editor, false, false, false);
        }
        if (word_next || word_previous) && !self.open {
            self.open_with(editor, false, false, false);
        }
        if !self.open {
            return out;
        }
        if next {
            self.cmds.push(Cmd::Next);
        }
        if previous {
            self.cmds.push(Cmd::Previous);
        }
        if word_next {
            self.cmds.push(Cmd::WordAtCaret(true));
        }
        if word_previous {
            self.cmds.push(Cmd::WordAtCaret(false));
        }
        // The history popup owns the arrows, Enter and Escape while it is open.
        if self.history_open {
            let last = history.len().saturating_sub(1);
            if take(Key::ArrowDown, KeyModifiers::NONE) {
                self.history_pick = (self.history_pick + 1).min(last);
            }
            if take(Key::ArrowUp, KeyModifiers::NONE) {
                self.history_pick = self.history_pick.saturating_sub(1);
            }
            if take(Key::Enter, KeyModifiers::NONE) {
                if let Some(entry) = history.get(self.history_pick) {
                    self.query = entry.clone();
                    self.focus_query = true;
                }
                self.history_open = false;
            }
            if take(Key::Escape, KeyModifiers::NONE) {
                self.history_open = false;
            }
            return out;
        }
        // The fields.
        if (query_focused || replace_focused) && self.replace && !viewer {
            // Tab and Shift+Tab move between the two fields.
            if take(Key::Tab, KeyModifiers::NONE) || take(Key::Tab, KeyModifiers::SHIFT) {
                if query_focused {
                    self.focus_replace = true;
                } else {
                    self.focus_query = true;
                }
            }
        }
        if query_focused || replace_focused {
            let multiline = self.query.contains('\n');
            if take(Key::Enter, shift(primary)) && query_focused {
                insert_newline(ctx, self.field(), &mut self.query);
            }
            if take(Key::Enter, primary) {
                self.cmds.push(Cmd::Accept);
            }
            if take(Key::Enter, KeyModifiers::ALT) {
                self.cmds.push(Cmd::SelectAll);
            }
            if take(Key::Enter, KeyModifiers::SHIFT) {
                self.cmds.push(Cmd::Previous);
            }
            if take(Key::Enter, KeyModifiers::NONE) {
                self.cmds.push(if replace_focused {
                    Cmd::Replace
                } else {
                    Cmd::Next
                });
            }
            if query_focused && !multiline {
                if take(Key::ArrowDown, KeyModifiers::NONE) {
                    self.cmds.push(Cmd::Next);
                }
                if take(Key::ArrowUp, KeyModifiers::NONE) {
                    self.cmds.push(Cmd::Previous);
                }
            }
            if query_focused && take(Key::ArrowDown, KeyModifiers::ALT) && !history.is_empty() {
                self.history_open = true;
                self.history_pick = 0;
            }
            if take(Key::C, KeyModifiers::ALT) {
                self.options.match_case ^= true;
            }
            if take(Key::W, KeyModifiers::ALT) {
                self.options.whole_words ^= true;
            }
            if take(Key::X, KeyModifiers::ALT) {
                self.options.regex ^= true;
            }
            if self.replace && !viewer {
                if take(Key::A, KeyModifiers::ALT) {
                    self.cmds.push(Cmd::ReplaceAll);
                }
                if take(Key::P, KeyModifiers::ALT) {
                    self.options.preserve_case ^= true;
                }
            }
        }
        if take(Key::Escape, KeyModifiers::NONE) {
            self.close();
            out.closed = true;
        }
        out
    }

    /// Runs the live update and the commands of this frame against `editor`. `results` are
    /// the matches of the current query for the current text.
    pub fn apply(
        &mut self,
        editor: &mut Editor,
        results: &SearchResults,
        env: &ApplyEnv,
        history: &mut SearchHistory,
    ) -> ApplyOut {
        let mut out = ApplyOut::default();
        if let Some(query) = self.close_history.take() {
            history.add_query(&query);
        }
        if !self.open {
            self.cmds.clear();
            return out;
        }
        let snapshot = editor.carets.snapshot();
        let carets_before = snapshot.clone();
        if self.origin.is_none() {
            self.origin = Some(Origin {
                carets: snapshot.clone(),
                scroll: env.scroll,
            });
        }
        if env.editor_focused && snapshot != self.synced {
            // The user moved the carets: the search starts from there now.
            self.origin = Some(Origin {
                carets: snapshot.clone(),
                scroll: env.scroll,
            });
            self.search_from = editor.carets.primary().selection_start();
            self.wrap = None;
        }

        // Live update: typing or a changed option selects the first match from the caret;
        // clearing the query puts the carets and the view back.
        let key = self.key();
        if self.applied.as_ref() != Some(&key) {
            self.applied = Some(key);
            self.wrap = None;
            if self.query.is_empty() || results.ranges.is_empty() {
                self.restore_origin(editor, &mut out);
            } else {
                let at = next_match(&results.ranges, self.search_from, true).unwrap_or(0);
                self.select(editor, &results.ranges[at].clone(), &mut out);
            }
        }

        let cmds = std::mem::take(&mut self.cmds);
        let mut fresh: Option<SearchResults> = None;
        for cmd in cmds {
            let current = fresh.as_ref().unwrap_or(results);
            match cmd {
                Cmd::Next | Cmd::Previous => {
                    let ranges = current.ranges.clone();
                    self.step(editor, &ranges, cmd == Cmd::Next, env.os, &mut out);
                }
                Cmd::WordAtCaret(forward) => {
                    let caret = editor.carets.primary().offset;
                    if let Some((s, e)) = word_range_at(&editor.document, caret, false) {
                        self.query = editor.document.slice(s, e).to_owned();
                        self.options.regex = false;
                        self.options.whole_words = true;
                        self.scope = None;
                        self.applied = Some(self.key());
                        let found = self.search(editor);
                        let from = editor.carets.primary().selection_start();
                        self.search_from = from;
                        let ranges = found.ranges.clone();
                        fresh = Some(found);
                        self.step(editor, &ranges, forward, env.os, &mut out);
                    }
                }
                Cmd::Accept => {
                    history.add_query(&self.query);
                    if !self.replacement.is_empty() {
                        history.add_replacement(&self.replacement);
                    }
                    if self.query.is_empty() {
                        self.close();
                    }
                    out.focus_editor = true;
                }
                Cmd::SelectAll => {
                    let ranges = current.ranges.clone();
                    if !ranges.is_empty() {
                        if !editor.select_ranges(&ranges) {
                            out.hint = Some("Too many carets".to_owned());
                        }
                        self.search_from = ranges[0].start;
                        out.jump = Some(ranges[0].start);
                        out.focus_editor = true;
                    }
                }
                Cmd::Replace if !env.viewer => {
                    self.replace_current(editor, &mut fresh, results, history, env.os, &mut out);
                }
                Cmd::ReplaceAll if !env.viewer => {
                    self.replace_all(editor, history, &mut out);
                    fresh = None;
                }
                Cmd::Replace | Cmd::ReplaceAll => {}
            }
        }
        self.synced = editor.carets.snapshot();
        if self.synced != carets_before {
            // Moving to a match is not a caret-only step for undo: the next Ctrl+Z undoes
            // the last edit.
            editor.undo.set_baseline(&editor.carets);
        }
        out
    }

    fn search(&self, editor: &Editor) -> SearchResults {
        find_all(
            editor.document.text(),
            &self.query,
            self.options,
            self.scope.clone(),
            MAX_SEARCH_RESULTS,
        )
    }

    fn restore_origin(&mut self, editor: &mut Editor, out: &mut ApplyOut) {
        if let Some(origin) = &self.origin {
            editor.carets.restore(&editor.document, &origin.carets);
            out.restore_scroll = Some(origin.scroll);
        }
    }

    /// Makes `range` the editor's selection: the current match.
    fn select(&mut self, editor: &mut Editor, range: &Range<usize>, out: &mut ApplyOut) {
        editor
            .carets
            .set_selection(&editor.document, range.start, range.end);
        self.search_from = range.start;
        out.jump = Some(range.start);
    }

    /// Next or previous match from the selection; past the last one the first press only
    /// shows a hint, the second wraps.
    fn step(
        &mut self,
        editor: &mut Editor,
        ranges: &[Range<usize>],
        forward: bool,
        os: Os,
        out: &mut ApplyOut,
    ) {
        if ranges.is_empty() {
            out.hint = Some(format!("\"{}\" not found", short(&self.query)));
            return;
        }
        let primary = editor.carets.primary();
        let from = match (primary.has_selection(), forward) {
            (true, true) => primary.selection_end(),
            (true, false) => primary.selection_start(),
            (false, _) => primary.offset,
        };
        let snapshot = editor.carets.snapshot();
        let found = next_match(ranges, from, forward);
        let wrapped = self
            .wrap
            .take()
            .is_some_and(|w| w.forward == forward && w.carets == snapshot);
        match (found, wrapped) {
            (Some(i), _) => self.select(editor, &ranges[i].clone(), out),
            (None, true) => {
                let i = if forward { 0 } else { ranges.len() - 1 };
                self.select(editor, &ranges[i].clone(), out);
            }
            (None, false) => {
                self.wrap = Some(PendingWrap {
                    forward,
                    carets: snapshot,
                });
                let (key, edge) = match (os, forward) {
                    (Os::WindowsLinux, true) => ("F3", "the top"),
                    (Os::WindowsLinux, false) => ("Shift+F3", "the bottom"),
                    (Os::MacOs, true) => ("Cmd+G", "the top"),
                    (Os::MacOs, false) => ("Shift+Cmd+G", "the bottom"),
                };
                out.hint = Some(format!(
                    "\"{}\" not found, press {key} to search from {edge}",
                    short(&self.query)
                ));
            }
        }
    }

    fn replace_current(
        &mut self,
        editor: &mut Editor,
        fresh: &mut Option<SearchResults>,
        results: &SearchResults,
        history: &mut SearchHistory,
        os: Os,
        out: &mut ApplyOut,
    ) {
        let current = fresh.as_ref().unwrap_or(results);
        let primary = editor.carets.primary();
        let selection = primary.selection_start()..primary.selection_end();
        if match_at(&current.ranges, &selection).is_none() {
            // Nothing selected to replace: the first press only finds the next match.
            let ranges = current.ranges.clone();
            self.step(editor, &ranges, true, os, out);
            return;
        }
        let Ok(searcher) = Searcher::new(&self.query, self.options) else {
            return;
        };
        let text =
            searcher.replacement(editor.document.text(), selection.clone(), &self.replacement);
        history.add_replacement(&self.replacement);
        let old_len = selection.len();
        editor.replace_range(selection.start, selection.end, &text);
        out.text_changed = true;
        let end = selection.start + text.len();
        editor.carets.set_caret(&editor.document, end);
        if let Some(scope) = &mut self.scope {
            scope.end = (scope.end + text.len()).saturating_sub(old_len);
        }
        // After a replace the caret moves to the next match, wrapping silently.
        let found = self.search(editor);
        let at = next_match(&found.ranges, end, true).or(if found.ranges.is_empty() {
            None
        } else {
            Some(0)
        });
        if let Some(i) = at {
            let range = found.ranges[i].clone();
            self.select(editor, &range, out);
        } else {
            self.search_from = end;
        }
        *fresh = Some(found);
    }

    fn replace_all(
        &mut self,
        editor: &mut Editor,
        history: &mut SearchHistory,
        out: &mut ApplyOut,
    ) {
        let Ok(searcher) = Searcher::new(&self.query, self.options) else {
            return;
        };
        if self.query.is_empty() {
            return;
        }
        let (ranges, _) = searcher.find_all(editor.document.text(), self.scope.clone(), usize::MAX);
        if ranges.is_empty() {
            return;
        }
        let text = editor.document.text();
        let replacements: Vec<(Range<usize>, String)> = ranges
            .iter()
            .map(|r| {
                (
                    r.clone(),
                    searcher.replacement(text, r.clone(), &self.replacement),
                )
            })
            .collect();
        history.add_replacement(&self.replacement);
        if let Some(scope) = &mut self.scope {
            let removed: usize = replacements.iter().map(|(r, _)| r.len()).sum();
            let inserted: usize = replacements.iter().map(|(_, t)| t.len()).sum();
            scope.end = (scope.end + inserted).saturating_sub(removed);
        }
        let count = replacements.len();
        editor.replace_ranges(&replacements);
        let caret = editor.carets.primary().offset;
        out.text_changed = true;
        out.hint = Some(format!(
            "{count} occurrence{} replaced",
            if count == 1 { "" } else { "s" }
        ));
        self.wrap = None;
        self.search_from = caret;
    }
}

/// A query shortened for a hint.
fn short(query: &str) -> String {
    let line = query.lines().next().unwrap_or("");
    if line.chars().count() > 40 || query.contains('\n') {
        format!("{}...", line.chars().take(40).collect::<String>())
    } else {
        line.to_owned()
    }
}

/// Takes a key press with exactly the modifiers `want` (as the keymap of `os` sees them) from
/// the frame's input.
fn take_key(ui: &Ui, os: Os, key: Key, want: KeyModifiers) -> bool {
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| {
            matches!(e, Event::Key { key: k, pressed: true, modifiers, .. }
                if *k == key && key_modifiers(*modifiers, os) == want)
        });
        match at {
            Some(at) => {
                i.events.remove(at);
                true
            }
            None => false,
        }
    })
}

/// Inserts a line break at the caret of the field `id` (Ctrl+Shift+Enter).
fn insert_newline(ctx: &egui::Context, id: Id, text: &mut String) {
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let chars = text.chars().count();
    let at = state
        .cursor
        .char_range()
        .map_or(chars, |r| r.primary.index.min(chars));
    let byte = text.char_indices().nth(at).map_or(text.len(), |(b, _)| b);
    text.insert(byte, '\n');
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(at + 1))));
    state.store(ctx, id);
}

/// What the bar shows besides its state.
pub(super) struct BarView<'a> {
    pub status: String,
    /// No match, or a bad pattern: the field turns red.
    pub not_found: bool,
    pub viewer: bool,
    /// Recent queries for the history popup.
    pub history: &'a [String],
}

/// An option toggle with a text label inside the field's row (Cc, W, .*): selected looks
/// pushed. Returns whether it was clicked.
fn option_toggle(ui: &mut Ui, text: &str, name: &str, shortcut: &str, on: bool) -> bool {
    let colors = Theme::colors(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(vec2(size::ICON_BUTTON, size::ICON_BUTTON), Sense::CLICK);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, name));
    crate::cursor::on_hover_action(&response);
    let fill = if on || response.is_pointer_button_down_on() {
        colors.pressed
    } else if response.hovered() {
        colors.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, radius::ICON_BUTTON_SMALL, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        typography::body(typography::COMMENT),
        if on {
            colors.text
        } else {
            colors.text_secondary
        },
    );
    crate::ide::HelpTooltip::new(name)
        .shortcut(shortcut)
        .show_for(&response);
    response.clicked()
}

/// Draws the bar in `rect` (its top `state.height()` points) and applies what the user did
/// with the mouse to `state`. Returns the rect it filled.
pub(super) fn bar(ui: &mut Ui, rect: Rect, state: &mut FindState, view: &BarView<'_>) -> Rect {
    let theme = Theme::current(ui.ctx());
    let (colors, editor) = (theme.colors, theme.editor);
    let height = state.height(view.viewer).min(rect.height());
    let bar_rect = Rect::from_min_size(rect.min, vec2(rect.width(), height));
    ui.painter()
        .rect_filled(bar_rect, 0.0, editor.background_panel);
    ui.painter().line_segment(
        [bar_rect.left_bottom(), bar_rect.right_bottom()],
        Stroke::new(1.0_f32, colors.border),
    );
    let extra = (state.query_rows() - 1) as f32 * size::CODE_LINE_HEIGHT;
    let row1 = Rect::from_min_size(
        bar_rect.min,
        vec2(bar_rect.width(), metrics::FIND_ROW_HEIGHT + extra),
    );
    let inset = |row: Rect| {
        Rect::from_min_max(
            pos2(row.left() + metrics::FIND_INSET_LEFT, row.top()),
            pos2(row.right() - metrics::FIND_INSET_RIGHT, row.bottom()),
        )
    };
    let id = state.field();
    let tabs = state.replace && !view.viewer;
    let gap = size::TOOLBAR_GAP;
    let mut lead_width: f32 = 0.0;

    // Row 1, the right cluster first: Close, Next, Previous, status, Regex, Words, Case.
    let mut right = ui.new_child(
        UiBuilder::new()
            .id_salt(id.with("right"))
            .max_rect(inset(row1))
            .layout(Layout::right_to_left(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = gap;
    if right
        .add(ActionButton::new(Glyph::Close, "Close find").small(true))
        .clicked()
    {
        state.close();
    }
    if right
        .add(ActionButton::new(Glyph::ChevronDown, "Next match").small(true))
        .clicked()
    {
        state.cmds.push(Cmd::Next);
    }
    if right
        .add(ActionButton::new(Glyph::ChevronUp, "Previous match").small(true))
        .clicked()
    {
        state.cmds.push(Cmd::Previous);
    }
    // A narrow bar gives the status less room, so the field keeps some.
    let status_width = (inset(row1).width() - metrics::FIND_NARROW_RESERVE)
        .clamp(metrics::FIND_STATUS_MIN_WIDTH, metrics::FIND_STATUS_WIDTH);
    let status_color = if view.not_found {
        colors.error
    } else {
        colors.text_secondary
    };
    right.add_sized(
        [status_width, size::CONTROL_HEIGHT],
        egui::Label::new(
            RichText::new(view.status.clone())
                .font(typography::body(typography::COMMENT))
                .color(status_color),
        )
        .truncate(),
    );
    if option_toggle(&mut right, ".*", "Regex", "Alt+X", state.options.regex) {
        state.options.regex ^= true;
    }
    if option_toggle(&mut right, "W", "Words", "Alt+W", state.options.whole_words) {
        state.options.whole_words ^= true;
    }
    if option_toggle(
        &mut right,
        "Cc",
        "Match Case",
        "Alt+C",
        state.options.match_case,
    ) {
        state.options.match_case ^= true;
    }
    let right_edge = right.min_rect().left() - gap;

    // Row 1, the left part: history and the query field.
    let mut left = ui.new_child(
        UiBuilder::new()
            .id_salt(id.with("left"))
            .max_rect(Rect::from_min_max(
                inset(row1).min,
                pos2(right_edge.max(inset(row1).min.x), row1.bottom()),
            ))
            .layout(Layout::left_to_right(egui::Align::Min)),
    );
    left.spacing_mut().item_spacing.x = gap;
    left.add_space((metrics::FIND_ROW_HEIGHT - size::CONTROL_HEIGHT) / 2.0);
    let history_button = left.add(
        ActionButton::new(Glyph::Search, "Search history")
            .small(true)
            .dropdown(true)
            .shortcut("Alt+Down"),
    );
    if history_button.clicked() && !view.history.is_empty() {
        state.history_open = !state.history_open;
        state.history_pick = 0;
    }
    lead_width = lead_width.max(history_button.rect.width());
    let field_width = left.available_width().max(24.0);
    if std::mem::take(&mut state.focus_query) {
        left.memory_mut(|m| m.request_focus(state.field()));
    }
    let rows = state.query_rows();
    let placeholder = if state.scope.is_some() {
        "Search in selection..."
    } else {
        "Search"
    };
    let query_id = state.field();
    let mut field = JBTextField::new(&mut state.query)
        .id(query_id)
        .name("Find")
        .placeholder(placeholder)
        .monospace(true)
        .width(field_width);
    if rows > 1 {
        field = field.multiline(rows);
    }
    let field_frame = Rect::from_min_size(
        left.next_widget_position(),
        vec2(
            field_width,
            if rows > 1 {
                rows as f32 * size::CODE_LINE_HEIGHT + size::CODE_PADDING
            } else {
                size::CONTROL_HEIGHT
            },
        ),
    );
    let output = field.show(&mut left);
    let edit = output.response;
    left.ctx()
        .accesskit_node_builder(edit.id, |node| node.set_label("Find"));
    keep_keys(&left, &edit, tabs);
    if state.select_field_text && edit.has_focus() {
        state.select_field_text = false;
        let len = state.query.chars().count();
        let mut text_state =
            egui::text_edit::TextEditState::load(left.ctx(), edit.id).unwrap_or_default();
        text_state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
        text_state.store(left.ctx(), edit.id);
    }
    if view.not_found && !state.query.is_empty() {
        paint_not_found(ui, field_frame);
    }
    if state.history_open {
        let mut open = true;
        let pick = state.history_pick;
        let width = field_frame.width().max(metrics::FIND_HISTORY_MIN_WIDTH);
        let chosen = JBPopup::below(id.with("history"), &edit).width(width).show(
            ui.ctx(),
            &mut open,
            |ui| {
                let mut chosen = None;
                for (i, entry) in view.history.iter().enumerate() {
                    let line = entry.lines().next().unwrap_or("");
                    if ui
                        .add(ActionMenuItem::new(line).highlighted(i == pick))
                        .clicked()
                    {
                        chosen = Some(entry.clone());
                    }
                }
                chosen
            },
        );
        if let Some(Some(entry)) = chosen {
            state.query = entry;
            state.focus_query = true;
            open = false;
        }
        state.history_open = open;
    }

    // Row 2: the replace field and its buttons.
    if state.replace && !view.viewer {
        let row2 = Rect::from_min_size(
            pos2(bar_rect.left(), row1.bottom()),
            vec2(bar_rect.width(), metrics::FIND_ROW_HEIGHT),
        );
        let mut right = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("right2"))
                .max_rect(inset(row2))
                .layout(Layout::right_to_left(egui::Align::Center)),
        );
        right.spacing_mut().item_spacing.x = gap;
        if option_toggle(
            &mut right,
            "AB",
            "Preserve Case",
            "Alt+P",
            state.options.preserve_case,
        ) {
            state.options.preserve_case ^= true;
        }
        if right
            .add(Button::secondary("Replace All").small(true))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            state.cmds.push(Cmd::ReplaceAll);
        }
        if right
            .add(Button::secondary("Replace").small(true))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            state.cmds.push(Cmd::Replace);
        }
        let edge = right.min_rect().left() - gap;
        let left_min = inset(row2).min.x + lead_width + gap;
        let mut left = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("left2"))
                .max_rect(Rect::from_min_max(
                    pos2(left_min, row2.top()),
                    pos2(edge.max(left_min), row2.bottom()),
                ))
                .layout(Layout::left_to_right(egui::Align::Min)),
        );
        left.add_space((metrics::FIND_ROW_HEIGHT - size::CONTROL_HEIGHT) / 2.0);
        if std::mem::take(&mut state.focus_replace) {
            left.memory_mut(|m| m.request_focus(state.replace_field()));
        }
        let width = left.available_width().max(24.0);
        let replace_id = state.replace_field();
        let edit = left.add(
            JBTextField::new(&mut state.replacement)
                .id(replace_id)
                .name("Replace with")
                .placeholder("Replace")
                .monospace(true)
                .width(width),
        );
        left.ctx()
            .accesskit_node_builder(edit.id, |node| node.set_label("Replace with"));
        keep_keys(&left, &edit, tabs);
    }
    bar_rect
}

/// The not-found state of the query field: a red wash and outline (spec §7).
fn paint_not_found(ui: &Ui, rect: Rect) {
    let colors = Theme::colors(ui.ctx());
    ui.painter()
        .rect_filled(rect, radius::CONTROL, colors.error.gamma_multiply(0.12));
    ui.painter().rect_stroke(
        rect,
        radius::CONTROL,
        Stroke::new(1.0_f32, colors.error),
        egui::StrokeKind::Inside,
    );
}

/// While a field of the bar has the focus, Escape and the arrows are the bar's (close, next
/// and previous), not egui's focus navigation.
fn keep_keys(ui: &Ui, field: &egui::Response, tab: bool) {
    if field.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                field.id,
                egui::EventFilter {
                    escape: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    tab,
                },
            );
        });
    }
}
