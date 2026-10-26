//! The find and replace bar, a port of the GUI editor's behavior: Ctrl+F and Ctrl+R open it,
//! Match Case, Words and Regex toggle with Alt+C, Alt+W and Alt+X, the status reads `n/m`,
//! Enter, Shift+Enter and F3 step through the matches (the first step past the last match
//! only shows a hint, the second wraps), Replace and Replace All (one undo step), a history of
//! queries and Alt+Enter to select all occurrences. The current match **is** the editor
//! selection, and the matching is the model's.
//!
//! It is drawn as an NvChad prompt row at the top of the editor: a colored title pill and the
//! prompt on `black2`, with the status and the option toggles at the right.

use super::{EditorOutput, EditorState, Opts, reveal};
use crate::components::{field_scroll, fill, put, text_width, title_block};
use crate::controls::{FieldEvent, TextFieldState};
use crate::input::{KeyCode, KeyEvent, KeyModifiers};
use crate::runtime::{CursorShape, Runtime};
use crate::{Glyph, Look};
use archipel_kit::editor::logic::find::{FindEnv, FindSession, current_match as match_index};
use archipel_kit::editor::model::{
    Editor, MAX_SEARCH_RESULTS, Os, SearchOptions, SearchResults, find_all,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::ops::Range;
use std::rc::Rc;

pub(crate) use archipel_kit::editor::logic::find::FindCmd as Cmd;

/// Which part of the editor has the keyboard focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum FindFocus {
    /// The text.
    #[default]
    Editor,
    /// The query field.
    Query,
    /// The replacement field.
    Replace,
}

/// A part of the bar that takes clicks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BarHit {
    Query,
    Replace,
    MatchCase,
    Words,
    Regex,
    Preserve,
    Previous,
    Next,
    Close,
    ReplaceOne,
    ReplaceAll,
    History,
    HistoryItem(usize),
}

type Applied = (String, SearchOptions, Option<Range<usize>>);

/// The find bar of one editor: the fields the user types in, and the shared
/// [`FindSession`] that searches (the GUI bar runs the same one).
#[derive(Default)]
pub(crate) struct FindState {
    pub open: bool,
    /// The replace row is shown.
    pub replace: bool,
    pub query: TextFieldState,
    pub replacement: TextFieldState,
    /// The search: options, scope, where it started, the pending wrap.
    pub core: FindSession<(usize, usize)>,
    pub focus: FindFocus,
    pub history_open: bool,
    pub history_pick: usize,
    results: Option<(u64, Applied, Rc<SearchResults>)>,
    /// The clickable parts of the last frame.
    pub hits: Vec<(Rect, BarHit)>,
    /// The text areas of the two fields of the last frame, for placing the caret by click.
    field_rects: [Rect; 2],
}

impl FindState {
    /// The rows the bar takes.
    pub fn rows(&self, viewer: bool) -> u16 {
        if !self.open {
            0
        } else if self.replace && !viewer {
            2
        } else {
            1
        }
    }

    /// The session's query and replacement are the fields' text.
    fn sync_fields(&mut self) {
        if self.core.query != self.query.text() {
            self.core.query = self.query.text().to_owned();
        }
        if self.core.replacement != self.replacement.text() {
            self.core.replacement = self.replacement.text().to_owned();
        }
    }

    /// The fields show what the session says (the word at the caret became the query).
    fn sync_back(&mut self) {
        if self.query.text() != self.core.query {
            self.query.set_text(&self.core.query);
        }
    }

    fn key(&self) -> Applied {
        (
            self.query.text().to_owned(),
            self.core.options,
            self.core.scope.clone(),
        )
    }

    /// The matches of the current query in the current text, cached.
    pub fn results(&mut self, editor: &Editor) -> Rc<SearchResults> {
        if !self.open {
            return Rc::new(SearchResults::default());
        }
        let stamp = editor.document.modification_stamp();
        let key = self.key();
        if let Some((s, k, results)) = &self.results
            && *s == stamp
            && *k == key
        {
            return results.clone();
        }
        let results = Rc::new(self.search(editor));
        self.results = Some((stamp, key, results.clone()));
        results
    }

    fn search(&self, editor: &Editor) -> SearchResults {
        find_all(
            editor.document.text(),
            self.query.text(),
            self.core.options,
            self.core.scope.clone(),
            MAX_SEARCH_RESULTS,
        )
    }

    fn close(&mut self) {
        self.open = false;
        self.history_open = false;
        self.core.close();
        self.focus = FindFocus::Editor;
        self.hits.clear();
    }
}

/// The current match of the open bar, as an index into the results.
pub(crate) fn current_match(editor: &Editor, results: &SearchResults) -> Option<usize> {
    match_index(editor, results)
}

/// Opens the bar (Ctrl+F, Ctrl+R, F3): a single-line selection becomes the query, a
/// multi-line one the scope of the search; otherwise the previous query stays. The query
/// field gets the focus with its text selected.
pub(crate) fn open_with(
    editor: &Editor,
    st: &mut EditorState,
    replace: bool,
    seed: bool,
    focus: bool,
) {
    let find = &mut st.find;
    let was_open = find.open;
    find.sync_fields();
    find.open = true;
    find.replace = replace;
    find.core.open(editor, was_open, seed);
    find.sync_back();
    if focus {
        find.focus = if replace && was_open && find.focus == FindFocus::Query {
            FindFocus::Replace
        } else {
            FindFocus::Query
        };
        match find.focus {
            FindFocus::Replace => find.replacement.select_all(),
            _ => find.query.select_all(),
        }
    }
    find.history_open = false;
}

/// A caret move of the user (not the bar): the search starts from there now.
pub(crate) fn after_user_move(editor: &Editor, st: &mut EditorState) {
    if !st.find.open {
        return;
    }
    let scroll = (st.first_row, st.first_col);
    st.find.core.user_moved(editor, scroll);
}

/// Runs a frame of the shared session: the live selection of an edited query or option, and
/// the commands the keys and buttons queued; then what the view must do about the result.
fn run_session(
    editor: &mut Editor,
    st: &mut EditorState,
    opts: &Opts<'_>,
    now: f64,
    out: &mut EditorOutput,
) {
    if !st.find.open {
        st.find.core.cmds.clear();
        return;
    }
    st.find.sync_fields();
    let results = st.find.results(editor);
    let env = FindEnv {
        os: opts.os,
        viewer: opts.viewer,
        // Moves of the carets by the user are reported by `after_user_move`.
        editor_focused: false,
        scroll: (st.first_row, st.first_col),
    };
    let done = st.find.core.apply(editor, &results, &env, &mut st.history);
    st.find.sync_back();
    if let Some((row, col)) = done.restore_scroll {
        st.first_row = row;
        st.first_col = col;
    }
    if let Some(offset) = done.jump {
        reveal(editor, st, offset);
    }
    out.text_changed |= done.text_changed;
    if let Some(hint) = done.hint {
        st.set_hint(hint.clone(), now);
        out.hint = Some(hint);
    }
    if done.focus_editor {
        st.find.focus = FindFocus::Editor;
    }
    if done.close {
        st.find.close();
    }
    out.consumed = true;
}

/// Typing or a changed option selects the first match from the caret; clearing the query puts
/// the carets and the view back.
fn live_update(
    editor: &mut Editor,
    st: &mut EditorState,
    opts: &Opts<'_>,
    now: f64,
    out: &mut EditorOutput,
) {
    run_session(editor, st, opts, now, out);
}

fn run_cmd(
    editor: &mut Editor,
    st: &mut EditorState,
    cmd: Cmd,
    opts: &Opts<'_>,
    now: f64,
    out: &mut EditorOutput,
) {
    st.find.core.cmds.push(cmd);
    run_session(editor, st, opts, now, out);
}

fn close(editor: &mut Editor, st: &mut EditorState) {
    if st.find.open {
        let query = st.find.query.text().to_owned();
        st.history.add_query(&query);
    }
    let _ = editor;
    st.find.close();
}

/// Keys that open the bar or step through matches from the text: Ctrl+F, Ctrl+R, F3,
/// Shift+F3, Alt+F3, Ctrl+L, Ctrl+F3 and Escape while the bar is open.
pub(crate) fn editor_key(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    ev: &KeyEvent,
    out: &mut EditorOutput,
) -> bool {
    let mods = ev.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    let shift = mods.contains(KeyModifiers::SHIFT);
    let sup = mods.contains(KeyModifiers::SUPER);
    let mac = opts.os == Os::MacOs;
    let primary = if mac { sup } else { ctrl && !alt };
    let plain = !ctrl && !alt && !sup;
    let ch = |c: char| matches!(ev.code, KeyCode::Char(k) if k.eq_ignore_ascii_case(&c));
    if st.find.open && ev.code == KeyCode::Esc && plain && !shift {
        close(editor, st);
        return true;
    }
    // Find: Ctrl+F (Cmd+F), and Alt+F3. Replace: Ctrl+R (Cmd+R; Ctrl+R is free on macOS in a
    // terminal too). A viewer has no replace row.
    let open_find = (primary && !shift && ch('f')) || (ev.code == KeyCode::F(3) && alt);
    let open_replace = (primary || (mac && ctrl && !alt)) && !shift && ch('r');
    if open_find || open_replace {
        let seed = !st.find.open || st.find.focus == FindFocus::Editor;
        open_with(editor, st, open_replace && !opts.viewer, seed, true);
        return true;
    }
    let next = (ev.code == KeyCode::F(3) && plain && !shift)
        || (!mac && ctrl && !shift && !alt && ch('l'))
        || (mac && sup && !shift && ch('g'));
    let previous = (ev.code == KeyCode::F(3) && !ctrl && !alt && !sup && shift)
        || (!mac && ctrl && shift && !alt && ch('l'))
        || (mac && sup && shift && ch('g'));
    let word_next = !mac && ctrl && !shift && ev.code == KeyCode::F(3);
    let word_previous = !mac && ctrl && shift && ev.code == KeyCode::F(3);
    if (next || previous) && !st.find.open && !st.find.query.is_empty() {
        open_with(editor, st, false, false, false);
    }
    if (word_next || word_previous) && !st.find.open {
        open_with(editor, st, false, false, false);
    }
    if !st.find.open {
        return false;
    }
    let cmd = if next {
        Cmd::Next
    } else if previous {
        Cmd::Previous
    } else if word_next {
        Cmd::WordAtCaret(true)
    } else if word_previous {
        Cmd::WordAtCaret(false)
    } else {
        return false;
    };
    run_cmd(editor, st, cmd, opts, rt.now, out);
    out.consumed = true;
    true
}

fn edit_field(rt: &mut Runtime, field: &mut TextFieldState, ev: &KeyEvent) -> FieldEvent {
    let actions: Vec<String> = rt.actions().to_vec();
    let before = field.revision;
    for id in &actions {
        if field.apply_action(id, &mut rt.clipboard) {
            return if field.revision != before {
                FieldEvent::Changed
            } else {
                FieldEvent::Consumed
            };
        }
    }
    match ev.code {
        KeyCode::Tab
        | KeyCode::BackTab
        | KeyCode::Up
        | KeyCode::Down
        | KeyCode::PageUp
        | KeyCode::PageDown => FieldEvent::Ignored,
        KeyCode::Char(_) if ev.modifiers.contains(KeyModifiers::CONTROL) => FieldEvent::Ignored,
        _ => field.apply_key(ev),
    }
}

/// Keys while a field of the bar has the focus. Returns whether the key was used.
pub(crate) fn key(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    ev: &KeyEvent,
    out: &mut EditorOutput,
) -> bool {
    if st.find.focus == FindFocus::Editor {
        return false;
    }
    let mods = ev.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    let shift = mods.contains(KeyModifiers::SHIFT);
    let plain = !ctrl && !alt;
    let in_query = st.find.focus == FindFocus::Query;
    let ch = |c: char| matches!(ev.code, KeyCode::Char(k) if k.eq_ignore_ascii_case(&c));
    // The history popup owns the arrows, Enter and Escape while it is open.
    if st.find.history_open {
        let last = st.history.queries().len().saturating_sub(1);
        match ev.code {
            KeyCode::Down => st.find.history_pick = (st.find.history_pick + 1).min(last),
            KeyCode::Up => st.find.history_pick = st.find.history_pick.saturating_sub(1),
            KeyCode::Enter => {
                if let Some(entry) = st.history.queries().get(st.find.history_pick).cloned() {
                    st.find.query.set_text(entry);
                    live_update(editor, st, opts, rt.now, out);
                }
                st.find.history_open = false;
            }
            KeyCode::Esc => st.find.history_open = false,
            _ => return false,
        }
        return true;
    }
    match ev.code {
        KeyCode::Esc if plain => {
            close(editor, st);
            return true;
        }
        KeyCode::Tab | KeyCode::BackTab if plain && st.find.replace && !opts.viewer => {
            st.find.focus = if in_query {
                FindFocus::Replace
            } else {
                FindFocus::Query
            };
            return true;
        }
        KeyCode::Enter if ctrl => {
            run_cmd(editor, st, Cmd::Accept, opts, rt.now, out);
            return true;
        }
        // Ctrl+Enter arrives as Ctrl+J in a legacy terminal.
        KeyCode::Char('j') if ctrl && !alt => {
            run_cmd(editor, st, Cmd::Accept, opts, rt.now, out);
            return true;
        }
        KeyCode::Enter if alt => {
            run_cmd(editor, st, Cmd::SelectAll, opts, rt.now, out);
            return true;
        }
        KeyCode::Enter if shift => {
            run_cmd(editor, st, Cmd::Previous, opts, rt.now, out);
            return true;
        }
        KeyCode::Enter => {
            let cmd = if in_query { Cmd::Next } else { Cmd::Replace };
            run_cmd(editor, st, cmd, opts, rt.now, out);
            return true;
        }
        KeyCode::Down if alt && in_query => {
            if !st.history.queries().is_empty() {
                st.find.history_open = true;
                st.find.history_pick = 0;
            }
            return true;
        }
        KeyCode::Down if plain && in_query => {
            run_cmd(editor, st, Cmd::Next, opts, rt.now, out);
            return true;
        }
        KeyCode::Up if plain && in_query => {
            run_cmd(editor, st, Cmd::Previous, opts, rt.now, out);
            return true;
        }
        KeyCode::Char(_) if alt && !ctrl => {
            let before = st.find.core.options;
            if ch('c') {
                st.find.core.options.match_case ^= true;
            } else if ch('w') {
                st.find.core.options.whole_words ^= true;
            } else if ch('x') {
                st.find.core.options.regex ^= true;
            } else if ch('p') && st.find.replace && !opts.viewer {
                st.find.core.options.preserve_case ^= true;
            } else if ch('a') && st.find.replace && !opts.viewer {
                run_cmd(editor, st, Cmd::ReplaceAll, opts, rt.now, out);
                return true;
            } else {
                return false;
            }
            if st.find.core.options != before {
                live_update(editor, st, opts, rt.now, out);
            }
            return true;
        }
        _ => {}
    }
    let field = if in_query {
        &mut st.find.query
    } else {
        &mut st.find.replacement
    };
    match edit_field(rt, field, ev) {
        FieldEvent::Ignored => false,
        FieldEvent::Changed => {
            if in_query {
                live_update(editor, st, opts, rt.now, out);
            }
            true
        }
        _ => true,
    }
}

/// A bracketed paste into the focused field. Returns whether a field took it.
pub(crate) fn paste(
    opts: &Opts<'_>,
    editor: &mut Editor,
    st: &mut EditorState,
    now: f64,
    text: &str,
    out: &mut EditorOutput,
) -> bool {
    match st.find.focus {
        FindFocus::Editor => false,
        FindFocus::Query => {
            st.find.query.insert(text);
            live_update(editor, st, opts, now, out);
            true
        }
        FindFocus::Replace => {
            st.find.replacement.insert(text);
            true
        }
    }
}

/// A left press on the bar at (`x`, `y`).
pub(crate) fn click(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    x: u16,
    y: u16,
    out: &mut EditorOutput,
) {
    let hit = st
        .find
        .hits
        .iter()
        .rev()
        .find(|(rect, _)| rect.contains((x, y).into()))
        .map(|(_, hit)| *hit);
    let Some(hit) = hit else {
        return;
    };
    out.consumed = true;
    let options = st.find.core.options;
    match hit {
        BarHit::Query | BarHit::Replace => {
            let query = hit == BarHit::Query;
            st.find.focus = if query {
                FindFocus::Query
            } else {
                FindFocus::Replace
            };
            st.find.history_open = false;
            let rect = st.find.field_rects[usize::from(!query)];
            let field = if query {
                &mut st.find.query
            } else {
                &mut st.find.replacement
            };
            let chars: Vec<char> = field.text().chars().collect();
            let scroll = field_scroll(&chars, field.caret(), rect.width as usize);
            let target = x.saturating_sub(rect.x);
            let mut cells = 0u16;
            let mut index = scroll;
            while index < chars.len() {
                let w = text_width(&chars[index].to_string()).max(1);
                if target < cells + w.div_ceil(2) {
                    break;
                }
                cells += w;
                index += 1;
            }
            field.move_to(index, false);
        }
        BarHit::MatchCase => st.find.core.options.match_case ^= true,
        BarHit::Words => st.find.core.options.whole_words ^= true,
        BarHit::Regex => st.find.core.options.regex ^= true,
        BarHit::Preserve => st.find.core.options.preserve_case ^= true,
        BarHit::Previous => run_cmd(editor, st, Cmd::Previous, opts, rt.now, out),
        BarHit::Next => run_cmd(editor, st, Cmd::Next, opts, rt.now, out),
        BarHit::ReplaceOne => run_cmd(editor, st, Cmd::Replace, opts, rt.now, out),
        BarHit::ReplaceAll => run_cmd(editor, st, Cmd::ReplaceAll, opts, rt.now, out),
        BarHit::Close => close(editor, st),
        BarHit::History => {
            st.find.history_open = !st.find.history_open && !st.history.queries().is_empty();
            st.find.history_pick = 0;
        }
        BarHit::HistoryItem(i) => {
            if let Some(entry) = st.history.queries().get(i).cloned() {
                st.find.query.set_text(entry);
                live_update(editor, st, opts, rt.now, out);
            }
            st.find.history_open = false;
        }
    }
    if st.find.core.options != options {
        live_update(editor, st, opts, rt.now, out);
    }
}

// ---- drawing --------------------------------------------------------------------------

struct Chip<'a> {
    text: &'a str,
    hit: BarHit,
    active: bool,
}

/// Draws the bar into `geo.bar` and registers its hit rectangle. Also puts the terminal
/// cursor into the focused field.
pub(crate) fn draw(
    opts: &Opts<'_>,
    editor: &Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    bar: Rect,
    buf: &mut Buffer,
) {
    st.find.hits.clear();
    if !st.find.open || bar.height == 0 || bar.width < 12 {
        return;
    }
    rt.clickable(opts.bar_id(), bar);
    let look = rt.look.clone();
    let p = look.palette;
    let results = st.find.results(editor);
    let current = current_match(editor, &results);
    let status = results.status(st.find.query.is_empty(), current);
    let not_found =
        results.problem.is_some() || (!st.find.query.is_empty() && results.ranges.is_empty());
    let focused = rt.focused(opts.id);
    let find_row = Rect::new(bar.x, bar.y, bar.width, 1);
    let pill_w = text_width(" Replace ");
    let base = Style::default().fg(p.white).bg(p.black2);
    fill(buf, bar, bar.x, bar.y, bar.width, base);
    // The title pill.
    let pill_bg = if not_found { p.red } else { p.nord_blue };
    title_block(
        buf,
        find_row,
        bar.x,
        bar.y,
        &format!("{:<7}", "Find"),
        p.on(pill_bg),
        pill_bg,
    );
    // The right end: status, toggles, buttons.
    let mut chips: Vec<Chip<'_>> = vec![
        Chip {
            text: " Cc ",
            hit: BarHit::MatchCase,
            active: st.find.core.options.match_case,
        },
        Chip {
            text: " W ",
            hit: BarHit::Words,
            active: st.find.core.options.whole_words,
        },
        Chip {
            text: " .* ",
            hit: BarHit::Regex,
            active: st.find.core.options.regex,
        },
    ];
    let close_icon = format!("{} ", look.glyph(Glyph::Close));
    let history_icon = format!("{} ", look.glyph(Glyph::History));
    let mut x = bar.right();
    let min_left = bar.x + pill_w + 6;
    // The status keeps its room: the buttons give way before it does.
    let status_text = if status.is_empty() {
        String::new()
    } else {
        format!(" {status} ")
    };
    let status_w = text_width(&status_text);
    let put_chip = |buf: &mut Buffer,
                    x: &mut u16,
                    text: &str,
                    hit: BarHit,
                    style: Style,
                    hits: &mut Vec<(Rect, BarHit)>|
     -> bool {
        let w = text_width(text);
        if *x < min_left + status_w + w {
            return false;
        }
        *x -= w;
        put(buf, find_row, *x, bar.y, text, style);
        hits.push((Rect::new(*x, bar.y, w, 1), hit));
        true
    };
    let mut hits = Vec::new();
    put_chip(
        buf,
        &mut x,
        &close_icon,
        BarHit::Close,
        Style::default().fg(p.red).bg(p.black2),
        &mut hits,
    );
    let nav = Style::default().fg(p.light_grey).bg(p.black2);
    put_chip(buf, &mut x, " ↓ ", BarHit::Next, nav, &mut hits);
    put_chip(buf, &mut x, " ↑ ", BarHit::Previous, nav, &mut hits);
    for chip in chips.drain(..).rev() {
        let style = if chip.active {
            Style::default()
                .fg(p.on(p.nord_blue))
                .bg(p.nord_blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.light_grey).bg(p.one_bg2)
        };
        if put_chip(buf, &mut x, chip.text, chip.hit, style, &mut hits) && x > min_left {
            x -= 1;
        }
    }
    // The status.
    let status_style = Style::default()
        .fg(if not_found { p.red } else { p.grey_fg })
        .bg(p.black2);
    if x >= min_left + status_w {
        x -= status_w;
        put(buf, find_row, x, bar.y, &status_text, status_style);
    }
    if x >= min_left + 2 && !st.history.queries().is_empty() {
        x -= 2;
        put(
            buf,
            find_row,
            x,
            bar.y,
            &history_icon,
            Style::default().fg(p.light_grey).bg(p.black2),
        );
        hits.push((Rect::new(x, bar.y, 2, 1), BarHit::History));
    }
    // The query field.
    let field_x = bar.x + pill_w + 1;
    let field_w = x.saturating_sub(field_x + 1);
    let query_rect = Rect::new(field_x, bar.y, field_w, 1);
    draw_field(
        &look,
        buf,
        query_rect,
        &st.find.query,
        "find in file",
        focused && st.find.focus == FindFocus::Query,
    );
    hits.push((
        Rect::new(field_x, bar.y, x.saturating_sub(field_x), 1),
        BarHit::Query,
    ));
    st.find.field_rects[0] = query_rect;
    if focused && st.find.focus == FindFocus::Query {
        place_cursor(rt, query_rect, &st.find.query);
    }
    // The replace row.
    if bar.height > 1 {
        let row = Rect::new(bar.x, bar.y + 1, bar.width, 1);
        fill(buf, bar, bar.x, bar.y + 1, bar.width, base);
        title_block(
            buf,
            row,
            bar.x,
            row.y,
            &format!("{:<7}", "Replace"),
            p.on(p.orange),
            p.orange,
        );
        let mut rx = bar.right();
        let mut row_hits = Vec::new();
        let mut put_row = |buf: &mut Buffer, x: &mut u16, text: &str, hit: BarHit, style: Style| {
            let w = text_width(text);
            if *x < min_left + w {
                return;
            }
            *x -= w;
            put(buf, row, *x, row.y, text, style);
            row_hits.push((Rect::new(*x, row.y, w, 1), hit));
        };
        let button = Style::default().fg(p.white).bg(p.one_bg3);
        put_row(buf, &mut rx, " All ", BarHit::ReplaceAll, button);
        rx = rx.saturating_sub(1);
        put_row(buf, &mut rx, " Replace ", BarHit::ReplaceOne, button);
        rx = rx.saturating_sub(1);
        let pc = if st.find.core.options.preserve_case {
            Style::default()
                .fg(p.on(p.nord_blue))
                .bg(p.nord_blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.light_grey).bg(p.one_bg2)
        };
        put_row(buf, &mut rx, " AB ", BarHit::Preserve, pc);
        let replace_rect = Rect::new(field_x, row.y, rx.saturating_sub(field_x + 1), 1);
        draw_field(
            &look,
            buf,
            replace_rect,
            &st.find.replacement,
            "replace with",
            focused && st.find.focus == FindFocus::Replace,
        );
        row_hits.push((
            Rect::new(field_x, row.y, rx.saturating_sub(field_x), 1),
            BarHit::Replace,
        ));
        st.find.field_rects[1] = replace_rect;
        if focused && st.find.focus == FindFocus::Replace {
            place_cursor(rt, replace_rect, &st.find.replacement);
        }
        hits.extend(row_hits);
    }
    // The history list under the query field.
    if st.find.history_open {
        let items: Vec<String> = st.history.queries().iter().take(6).cloned().collect();
        let top = bar.bottom();
        let w = field_w
            .max(12)
            .min(bar.width.saturating_sub(field_x - bar.x));
        let list = Rect::new(field_x, top, w, items.len() as u16);
        let area = Rect::new(bar.x, bar.y, bar.width, bar.height + items.len() as u16);
        rt.clickable(
            opts.bar_id(),
            Rect::new(list.x, list.y, list.width, list.height),
        );
        for (i, item) in items.iter().enumerate() {
            let y = top + i as u16;
            let selected = i == st.find.history_pick;
            let style = if selected {
                Style::default().fg(p.on(p.pmenu_bg)).bg(p.pmenu_bg)
            } else {
                Style::default().fg(p.white).bg(p.one_bg)
            };
            fill(buf, area, list.x, y, list.width, style);
            put(
                buf,
                area,
                list.x + 1,
                y,
                &crate::components::clip(item, list.width.saturating_sub(2)),
                style,
            );
            hits.push((Rect::new(list.x, y, list.width, 1), BarHit::HistoryItem(i)));
        }
    }
    st.find.hits = hits;
}

fn draw_field(
    look: &Look,
    buf: &mut Buffer,
    rect: Rect,
    field: &TextFieldState,
    placeholder: &str,
    focused: bool,
) {
    let p = look.palette;
    if rect.width == 0 {
        return;
    }
    let base = Style::default().fg(p.white).bg(p.black2);
    fill(buf, rect, rect.x, rect.y, rect.width, base);
    if field.is_empty() {
        put(
            buf,
            rect,
            rect.x,
            rect.y,
            placeholder,
            Style::default().fg(p.grey_fg).bg(p.black2),
        );
        return;
    }
    let chars: Vec<char> = field.text().chars().collect();
    let scroll = field_scroll(&chars, field.caret(), rect.width as usize);
    let selection = field.selection();
    let mut x = rect.x;
    for (i, c) in chars.iter().enumerate().skip(scroll) {
        if x >= rect.right() {
            break;
        }
        let selected = selection.as_ref().is_some_and(|r| r.contains(&i));
        let style = if selected {
            Style::default().fg(p.white).bg(p.one_bg3)
        } else {
            base
        };
        x = put(buf, rect, x, rect.y, &c.to_string(), style);
    }
    let _ = focused;
}

fn place_cursor(rt: &mut Runtime, rect: Rect, field: &TextFieldState) {
    let chars: Vec<char> = field.text().chars().collect();
    let scroll = field_scroll(&chars, field.caret(), rect.width as usize);
    let x: u16 = chars
        .iter()
        .take(field.caret())
        .skip(scroll)
        .map(|c| text_width(&c.to_string()))
        .sum();
    rt.set_cursor(
        (rect.x + x).min(rect.right().saturating_sub(1).max(rect.x)),
        rect.y,
        CursorShape::Bar,
    );
}
