//! The find and replace bar without a screen: [`FindSession`] holds what the bar edits (query,
//! replacement, options, scope) and what the search remembers (where it started, where the
//! carets were left, the pending wrap), and [`FindSession::apply`] runs a frame of it against
//! the editor model: the live selection while typing, next and previous with the "press again
//! to wrap" hint, replace, replace all as one undo step, select all occurrences.
//!
//! The current match **is** the editor selection. The GUI bar and the terminal prompt own the
//! fields, the keys and the drawing; both run the same session, so a search behaves the same
//! on either. `S` is whatever the view needs to restore its scroll position (a vector in the
//! GUI, a row and column in the terminal).

use crate::editor::model::{
    CaretSnapshot, Editor, MAX_SEARCH_RESULTS, Os, SearchHistory, SearchOptions, SearchResults,
    Searcher, find_all, match_at, next_match, word_range_at,
};
use std::ops::Range;

/// What a key chord or a button asks the bar to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindCmd {
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
struct Origin<S> {
    carets: CaretSnapshot,
    scroll: S,
}

/// The first Next past the last match shows a hint and waits for a second press.
#[derive(Clone, Debug)]
struct PendingWrap {
    forward: bool,
    carets: CaretSnapshot,
}

/// The search the live selection last ran for.
type Applied = (String, SearchOptions, Option<Range<usize>>);

/// The find state of one editor view.
#[derive(Clone, Debug)]
pub struct FindSession<S> {
    /// The text searched for.
    pub query: String,
    /// The replacement text.
    pub replacement: String,
    /// Match Case, Words, Regex, Preserve Case.
    pub options: SearchOptions,
    /// Find in selection: matches are searched in this range only.
    pub scope: Option<Range<usize>>,
    /// The commands of the keys and buttons, run by the next [`apply`](Self::apply).
    pub cmds: Vec<FindCmd>,
    origin: Option<Origin<S>>,
    search_from: usize,
    applied: Option<Applied>,
    wrap: Option<PendingWrap>,
    synced: CaretSnapshot,
}

impl<S> Default for FindSession<S> {
    fn default() -> Self {
        Self {
            query: String::new(),
            replacement: String::new(),
            options: SearchOptions::default(),
            scope: None,
            cmds: Vec::new(),
            origin: None,
            search_from: 0,
            applied: None,
            wrap: None,
            synced: CaretSnapshot::default(),
        }
    }
}

/// What [`FindSession::apply`] did that the view must act on.
#[derive(Debug, Clone, PartialEq)]
pub struct FindOut<S> {
    /// The text changed (a replace).
    pub text_changed: bool,
    /// A hint to show ("Not found, press F3 to search from the top").
    pub hint: Option<String>,
    /// Scroll here (and unfold) so the match shows.
    pub jump: Option<usize>,
    /// The query was cleared: the view goes back here.
    pub restore_scroll: Option<S>,
    /// Focus moves to the editor (Ctrl+Enter, Select All Occurrences).
    pub focus_editor: bool,
    /// The bar closes (Ctrl+Enter with an empty query).
    pub close: bool,
}

impl<S> Default for FindOut<S> {
    fn default() -> Self {
        Self {
            text_changed: false,
            hint: None,
            jump: None,
            restore_scroll: None,
            focus_editor: false,
            close: false,
        }
    }
}

/// What [`FindSession::apply`] needs besides the editor.
pub struct FindEnv<S> {
    /// The keymap that names the keys in hints.
    pub os: Os,
    /// Read-only: no replacing.
    pub viewer: bool,
    /// The text has the keyboard focus (a move of the carets then is the user's).
    pub editor_focused: bool,
    /// The view's scroll position now.
    pub scroll: S,
}

impl<S: Clone> FindSession<S> {
    /// The query, options and scope the live selection compares with.
    pub fn key(&self) -> (String, SearchOptions, Option<Range<usize>>) {
        (self.query.clone(), self.options, self.scope.clone())
    }

    /// The matches of the current query in the current text.
    pub fn search(&self, editor: &Editor) -> SearchResults {
        find_all(
            editor.document.text(),
            &self.query,
            self.options,
            self.scope.clone(),
            MAX_SEARCH_RESULTS,
        )
    }

    /// Where the next search starts.
    pub fn search_from(&self) -> usize {
        self.search_from
    }

    /// Opens the session (Ctrl+F, Ctrl+R, F3): with `seed`, a single-line selection becomes
    /// the query and a multi-line one the scope of the search; otherwise the previous query
    /// stays. `was_open` is whether the bar was open before this call.
    pub fn open(&mut self, editor: &Editor, was_open: bool, seed: bool) {
        if !was_open {
            self.origin = None;
            self.wrap = None;
        }
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
        self.synced = editor.carets.snapshot();
        // Opening alone does not move the selection: only edits of the query do.
        self.applied = Some(self.key());
    }

    /// Closes the session: forgets where the search started and any pending wrap.
    pub fn close(&mut self) {
        self.origin = None;
        self.wrap = None;
        self.cmds.clear();
    }

    /// A caret move of the user (not the bar): the search starts from there now.
    pub fn user_moved(&mut self, editor: &Editor, scroll: S) {
        let snapshot = editor.carets.snapshot();
        if snapshot != self.synced {
            self.origin = Some(Origin {
                carets: snapshot.clone(),
                scroll,
            });
            self.search_from = editor.carets.primary().selection_start();
            self.wrap = None;
            self.synced = snapshot;
        }
    }

    /// Records the query as already searched for, so the next frame does not select a match
    /// (the query was set by the application).
    pub fn mark_applied(&mut self) {
        self.applied = Some(self.key());
    }

    /// Runs the live update and the commands of this frame against `editor`. `results` are
    /// the matches of the current query for the current text. The bar must be open.
    pub fn apply(
        &mut self,
        editor: &mut Editor,
        results: &SearchResults,
        env: &FindEnv<S>,
        history: &mut SearchHistory,
    ) -> FindOut<S> {
        let mut out = FindOut::default();
        let snapshot = editor.carets.snapshot();
        let carets_before = snapshot.clone();
        if self.origin.is_none() {
            self.origin = Some(Origin {
                carets: snapshot.clone(),
                scroll: env.scroll.clone(),
            });
        }
        if env.editor_focused && snapshot != self.synced {
            // The user moved the carets: the search starts from there now.
            self.origin = Some(Origin {
                carets: snapshot.clone(),
                scroll: env.scroll.clone(),
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
                FindCmd::Next | FindCmd::Previous => {
                    let ranges = current.ranges.clone();
                    self.step(editor, &ranges, cmd == FindCmd::Next, env.os, &mut out);
                }
                FindCmd::WordAtCaret(forward) => {
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
                FindCmd::Accept => {
                    history.add_query(&self.query);
                    if !self.replacement.is_empty() {
                        history.add_replacement(&self.replacement);
                    }
                    if self.query.is_empty() {
                        out.close = true;
                    }
                    out.focus_editor = true;
                }
                FindCmd::SelectAll => {
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
                FindCmd::Replace if !env.viewer => {
                    self.replace_current(editor, &mut fresh, results, history, env.os, &mut out);
                }
                FindCmd::ReplaceAll if !env.viewer => {
                    self.replace_all(editor, history, &mut out);
                    fresh = None;
                }
                FindCmd::Replace | FindCmd::ReplaceAll => {}
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

    fn restore_origin(&mut self, editor: &mut Editor, out: &mut FindOut<S>) {
        if let Some(origin) = &self.origin {
            editor.carets.restore(&editor.document, &origin.carets);
            out.restore_scroll = Some(origin.scroll.clone());
        }
    }

    /// Makes `range` the editor's selection: the current match.
    fn select(&mut self, editor: &mut Editor, range: &Range<usize>, out: &mut FindOut<S>) {
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
        out: &mut FindOut<S>,
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
        out: &mut FindOut<S>,
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
        out: &mut FindOut<S>,
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

/// The current match among `results`: the one the primary caret selects.
pub fn current_match(editor: &Editor, results: &SearchResults) -> Option<usize> {
    let primary = editor.carets.primary();
    match_at(
        &results.ranges,
        &(primary.selection_start()..primary.selection_end()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(text: &str, query: &str) -> (Editor, FindSession<()>, SearchHistory) {
        let editor = Editor::new(text);
        let mut s = FindSession::<()>::default();
        s.open(&editor, false, false);
        s.query = query.to_owned();
        (editor, s, SearchHistory::new())
    }

    fn run(
        editor: &mut Editor,
        s: &mut FindSession<()>,
        history: &mut SearchHistory,
        cmds: &[FindCmd],
    ) -> FindOut<()> {
        s.cmds.extend_from_slice(cmds);
        let results = s.search(editor);
        s.apply(
            editor,
            &results,
            &FindEnv {
                os: Os::WindowsLinux,
                viewer: false,
                editor_focused: false,
                scroll: (),
            },
            history,
        )
    }

    fn selected(editor: &Editor) -> (usize, usize) {
        let c = editor.carets.primary();
        (c.selection_start(), c.selection_end())
    }

    #[test]
    fn typing_selects_the_first_match_from_the_caret() {
        let (mut editor, mut s, mut h) = session("a foo b foo", "foo");
        let out = run(&mut editor, &mut s, &mut h, &[]);
        assert_eq!(selected(&editor), (2, 5));
        assert_eq!(out.jump, Some(2));
    }

    #[test]
    fn clearing_the_query_restores_the_carets_and_the_view() {
        let (mut editor, mut s, mut h) = session("a foo b foo", "foo");
        editor.carets.set_caret(&editor.document, 1);
        s.user_moved(&editor, ());
        let results = s.search(&editor);
        let env = FindEnv {
            os: Os::WindowsLinux,
            viewer: false,
            editor_focused: false,
            scroll: (),
        };
        s.apply(&mut editor, &results, &env, &mut h);
        assert_eq!(selected(&editor), (2, 5));
        s.query.clear();
        let results = s.search(&editor);
        let out = s.apply(&mut editor, &results, &env, &mut h);
        assert_eq!(out.restore_scroll, Some(()));
        assert_eq!(selected(&editor), (1, 1));
    }

    #[test]
    fn next_past_the_last_match_hints_and_the_second_press_wraps() {
        let (mut editor, mut s, mut h) = session("foo foo", "foo");
        run(&mut editor, &mut s, &mut h, &[]);
        run(&mut editor, &mut s, &mut h, &[FindCmd::Next]);
        assert_eq!(selected(&editor), (4, 7));
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::Next]);
        assert_eq!(
            out.hint.as_deref(),
            Some("\"foo\" not found, press F3 to search from the top")
        );
        assert_eq!(selected(&editor), (4, 7));
        run(&mut editor, &mut s, &mut h, &[FindCmd::Next]);
        assert_eq!(selected(&editor), (0, 3));
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::Previous]);
        let hint = out.hint.expect("a hint");
        assert!(hint.contains("Shift+F3"), "{hint}");
    }

    #[test]
    fn replace_replaces_the_selected_match_and_moves_on() {
        let (mut editor, mut s, mut h) = session("foo foo foo", "foo");
        s.replacement = "x".to_owned();
        run(&mut editor, &mut s, &mut h, &[]);
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::Replace]);
        assert!(out.text_changed);
        assert_eq!(editor.document.text(), "x foo foo");
        assert_eq!(selected(&editor), (2, 5));
        assert_eq!(h.replacements().first().map(String::as_str), Some("x"));
    }

    #[test]
    fn replace_all_is_one_undo_step_and_counts() {
        let (mut editor, mut s, mut h) = session("foo foo foo", "foo");
        s.replacement = "bar".to_owned();
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::ReplaceAll]);
        assert_eq!(editor.document.text(), "bar bar bar");
        assert_eq!(out.hint.as_deref(), Some("3 occurrences replaced"));
        let language = crate::editor::model::Language::c_like();
        let ctx = crate::editor::model::ActionContext::new(&language);
        editor.execute(&crate::editor::model::EditorAction::Undo, &ctx);
        assert_eq!(editor.document.text(), "foo foo foo");
    }

    #[test]
    fn a_viewer_does_not_replace() {
        let (mut editor, mut s, mut h) = session("foo", "foo");
        s.replacement = "x".to_owned();
        s.cmds.push(FindCmd::ReplaceAll);
        let results = s.search(&editor);
        s.apply(
            &mut editor,
            &results,
            &FindEnv {
                os: Os::WindowsLinux,
                viewer: true,
                editor_focused: false,
                scroll: (),
            },
            &mut h,
        );
        assert_eq!(editor.document.text(), "foo");
    }

    #[test]
    fn select_all_puts_a_caret_on_every_match() {
        let (mut editor, mut s, mut h) = session("a a a", "a");
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::SelectAll]);
        assert_eq!(editor.carets.caret_count(), 3);
        assert!(out.focus_editor);
    }

    #[test]
    fn the_word_at_the_caret_becomes_a_whole_word_query() {
        let (mut editor, mut s, mut h) = session("foo foobar foo", "");
        editor.carets.set_caret(&editor.document, 1);
        run(&mut editor, &mut s, &mut h, &[FindCmd::WordAtCaret(true)]);
        assert_eq!(s.query, "foo");
        assert!(s.options.whole_words);
        assert_eq!(
            selected(&editor),
            (11, 14),
            "foobar is not a whole-word match"
        );
    }

    #[test]
    fn accept_remembers_the_query_and_closes_when_it_is_empty() {
        let (mut editor, mut s, mut h) = session("foo", "foo");
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::Accept]);
        assert!(out.focus_editor && !out.close);
        assert_eq!(h.queries().first().map(String::as_str), Some("foo"));
        s.query.clear();
        let out = run(&mut editor, &mut s, &mut h, &[FindCmd::Accept]);
        assert!(out.close);
    }

    #[test]
    fn a_selection_seeds_the_query_or_the_scope() {
        let mut editor = Editor::new("foo bar\nbaz");
        let mut s = FindSession::<()>::default();
        editor.carets.set_selection(&editor.document, 0, 3);
        s.open(&editor, false, true);
        assert_eq!(s.query, "foo");
        editor.carets.set_selection(&editor.document, 0, 9);
        s.open(&editor, true, true);
        assert_eq!(s.scope, Some(0..9));
        assert_eq!(s.query, "foo");
    }

    #[test]
    fn an_invalid_regex_finds_nothing_and_never_panics() {
        let (mut editor, mut s, mut h) = session("a(b", "(");
        s.options.regex = true;
        let out = run(
            &mut editor,
            &mut s,
            &mut h,
            &[FindCmd::Next, FindCmd::ReplaceAll],
        );
        assert_eq!(editor.document.text(), "a(b");
        assert!(out.hint.is_some());
    }
}
