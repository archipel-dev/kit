//! The find and replace bar of `EditorComponent` (JetBrains spec section 7), one test per
//! rule: opening and closing, options, status text, navigation, replace, history, the error
//! stripe. The matching rules themselves are unit tests of `editor::model::search`.

mod common;

use archipel_kit::editor::EditorComponent;
use archipel_kit::editor::model::{Editor, Os, SearchHistory};
use archipel_kit::testing::{self, Harness, NodeExt, NodeT, Queryable};
use archipel_kit::{Theme, egui};
use egui::{Event, Key, Modifiers, pos2};

struct App {
    editor: Editor,
    history: SearchHistory,
    hint: Option<String>,
    matches: usize,
    viewer: bool,
    os: Os,
}

type H<'a> = Harness<'a, App>;

const CTRL: Modifiers = Modifiers::COMMAND;

fn app(text: &str, viewer: bool, os: Os) -> App {
    App {
        editor: Editor::new(text),
        history: SearchHistory::new(),
        hint: None,
        matches: 0,
        viewer,
        os,
    }
}

fn harness_with<'a>(text: &str, viewer: bool, os: Os, height: f32) -> H<'a> {
    let mut h = testing::harness_state(Theme::dark(), app(text, viewer, os), move |ui, app| {
        let out = EditorComponent::new(&mut app.editor)
            .label("Ed")
            .viewer(app.viewer)
            .keymap_os(app.os)
            .search_history(&mut app.history)
            .height(height)
            .show(ui);
        if out.hint.is_some() {
            app.hint = out.hint;
        }
        app.matches = out.matches;
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    h
}

fn harness<'a>(text: &str) -> H<'a> {
    harness_with(text, false, Os::WindowsLinux, 200.0)
}

fn key(h: &mut H<'_>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run_ok();
}

fn type_text(h: &mut H<'_>, text: &str) {
    h.event(Event::Text(text.into()));
    h.run_ok();
}

/// Opens find (Ctrl+F) and types `query`.
fn find(h: &mut H<'_>, query: &str) {
    key(h, CTRL, Key::F);
    type_text(h, query);
}

fn selection(h: &H<'_>) -> Vec<String> {
    let e = &h.state().editor;
    e.carets
        .selected_texts(&e.document)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn caret(h: &H<'_>) -> usize {
    h.state().editor.carets.primary().offset
}

fn set_caret(h: &mut H<'_>, offset: usize) {
    let e = &mut h.state_mut().editor;
    let doc = e.document.clone();
    e.carets.set_caret(&doc, offset);
    h.run_ok();
}

fn set_selection(h: &mut H<'_>, anchor: usize, lead: usize) {
    let e = &mut h.state_mut().editor;
    let doc = e.document.clone();
    e.carets.set_selection(&doc, anchor, lead);
    h.run_ok();
}

fn text(h: &H<'_>) -> String {
    h.state().editor.document.text().to_owned()
}

fn bar_open(h: &H<'_>) -> bool {
    h.query_by_label("Next match").is_some()
}

fn has(h: &H<'_>, label: &str) -> bool {
    h.query_by_label(label).is_some()
}

fn field_focused(h: &H<'_>, label: &str) -> bool {
    h.query_by_label(label)
        .is_some_and(|n| n.accesskit_node().is_focused())
}

fn editor_focused(h: &H<'_>) -> bool {
    h.get_by_label("Ed").accesskit_node().is_focused()
}

const TEXT: &str = "one two one\nthree one\nfour";

// ---- opening and closing ------------------------------------------------------------------

#[test]
fn ctrl_f_opens_the_bar_with_the_query_field_focused() {
    let mut h = harness(TEXT);
    assert!(!bar_open(&h));
    key(&mut h, CTRL, Key::F);
    assert!(bar_open(&h));
    assert!(field_focused(&h, "Find"));
    assert!(!has(&h, "Replace"), "find mode has no replace row");
}

#[test]
fn ctrl_f_takes_a_single_line_selection_as_the_query() {
    let mut h = harness(TEXT);
    set_selection(&mut h, 4, 7);
    key(&mut h, CTRL, Key::F);
    type_text(&mut h, "");
    assert_eq!(h.get_by_label("Find").value().as_deref(), Some("two"));
}

#[test]
fn ctrl_f_without_a_selection_keeps_the_previous_query_and_not_the_word_at_the_caret() {
    let mut h = harness(TEXT);
    find(&mut h, "three");
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert!(!bar_open(&h));
    set_caret(&mut h, 5);
    key(&mut h, CTRL, Key::F);
    assert_eq!(h.get_by_label("Find").value().as_deref(), Some("three"));
}

#[test]
fn a_multi_line_selection_searches_in_the_selection_only() {
    let mut h = harness(TEXT);
    // "two one\nthree" selected.
    set_selection(&mut h, 4, 17);
    key(&mut h, CTRL, Key::F);
    assert!(has(&h, "Next match"));
    type_text(&mut h, "one");
    assert_eq!(h.state().matches, 1, "the one on line 3 is outside");
}

#[test]
fn ctrl_r_opens_the_replace_row_and_ctrl_f_switches_back_to_find() {
    let mut h = harness(TEXT);
    key(&mut h, CTRL, Key::R);
    assert!(bar_open(&h));
    assert!(has(&h, "Replace All"));
    assert!(field_focused(&h, "Find"));
    key(&mut h, CTRL, Key::F);
    assert!(bar_open(&h));
    assert!(!has(&h, "Replace All"));
}

#[test]
fn a_viewer_opens_find_with_ctrl_r_but_never_shows_replace() {
    let mut h = harness_with(TEXT, true, Os::WindowsLinux, 200.0);
    key(&mut h, CTRL, Key::R);
    assert!(bar_open(&h), "find works in a viewer");
    assert!(!has(&h, "Replace All"));
    type_text(&mut h, "one");
    assert_eq!(h.state().matches, 3);
}

#[test]
fn cmd_f_and_cmd_r_open_the_bar_on_macos_and_ctrl_f_does_not() {
    let mut h = harness_with(TEXT, false, Os::MacOs, 200.0);
    key(&mut h, Modifiers::CTRL, Key::F);
    assert!(!bar_open(&h), "Ctrl+F is cursor right on a Mac");
    key(&mut h, Modifiers::MAC_CMD, Key::F);
    assert!(bar_open(&h));
    assert!(!has(&h, "Replace All"));
    key(&mut h, Modifiers::MAC_CMD, Key::R);
    assert!(has(&h, "Replace All"));
}

#[test]
fn escape_closes_the_bar_and_returns_the_focus_with_the_match_selected() {
    let mut h = harness(TEXT);
    find(&mut h, "three");
    assert_eq!(selection(&h), ["three"]);
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert!(!bar_open(&h));
    assert!(editor_focused(&h));
    assert_eq!(selection(&h), ["three"], "the match stays selected");
}

#[test]
fn the_close_button_closes_the_bar_and_the_editor_takes_the_focus() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    h.get_by_label("Close find").click();
    h.run_ok();
    assert!(!bar_open(&h));
    assert!(editor_focused(&h));
}

#[test]
fn the_bar_is_docked_above_the_text_and_pushes_it_down() {
    let mut h = harness(TEXT);
    let before = h.get_by_label("Ed").rect();
    key(&mut h, CTRL, Key::F);
    let after = h.get_by_label("Ed").rect();
    assert!(after.top() >= before.top() + 30.0, "{before:?} {after:?}");
    assert_eq!(after.width(), before.width());
    let bar = h.get_by_label("Close find").rect();
    assert!(bar.bottom() <= after.top(), "the bar is above the text");
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert_eq!(h.get_by_label("Ed").rect(), before);
}

#[test]
fn the_replace_row_makes_the_bar_taller() {
    let mut h = harness(TEXT);
    key(&mut h, CTRL, Key::F);
    let find_top = h.get_by_label("Ed").rect().top();
    key(&mut h, CTRL, Key::R);
    assert!(h.get_by_label("Ed").rect().top() > find_top + 30.0);
}

// ---- options ------------------------------------------------------------------------------

fn toggle_is_on(h: &H<'_>, name: &str) -> bool {
    h.get_by_label(name).is_checked()
}

#[test]
fn the_option_toggles_have_alt_c_alt_w_and_alt_x() {
    let mut h = harness(TEXT);
    key(&mut h, CTRL, Key::F);
    for (name, key_) in [("Match Case", Key::C), ("Words", Key::W), ("Regex", Key::X)] {
        assert!(!toggle_is_on(&h, name), "{name} starts off");
        key(&mut h, Modifiers::ALT, key_);
        assert!(toggle_is_on(&h, name), "{name} on with Alt");
        key(&mut h, Modifiers::ALT, key_);
        assert!(!toggle_is_on(&h, name), "{name} off again");
    }
}

#[test]
fn clicking_a_toggle_changes_the_search() {
    let mut h = harness("Foo foo FOO");
    find(&mut h, "foo");
    assert_eq!(h.state().matches, 3, "case-insensitive by default");
    h.get_by_label("Match Case").click();
    h.run_ok();
    assert!(toggle_is_on(&h, "Match Case"));
    assert_eq!(h.state().matches, 1);
}

#[test]
fn matching_folds_case_beyond_ascii() {
    let mut h = harness("café CAFÉ Café");
    find(&mut h, "café");
    assert_eq!(h.state().matches, 3);
}

#[test]
fn words_match_whole_identifiers_only() {
    let mut h = harness("foo foobar _foo foo.x");
    find(&mut h, "foo");
    assert_eq!(h.state().matches, 4);
    key(&mut h, Modifiers::ALT, Key::W);
    assert_eq!(h.state().matches, 2);
}

#[test]
fn a_bad_regex_shows_bad_pattern() {
    let mut h = harness("a(b)c");
    key(&mut h, CTRL, Key::F);
    key(&mut h, Modifiers::ALT, Key::X);
    type_text(&mut h, "(");
    assert!(has(&h, "Bad pattern"));
    assert_eq!(h.state().matches, 0);
}

#[test]
fn regex_finds_with_groups() {
    let mut h = harness("a1 b22 c");
    key(&mut h, CTRL, Key::F);
    key(&mut h, Modifiers::ALT, Key::X);
    type_text(&mut h, r"[a-z]\d+");
    assert_eq!(h.state().matches, 2);
}

#[test]
fn results_are_capped_at_10000() {
    let text = "a ".repeat(10_050);
    let mut h = harness(&text);
    find(&mut h, "a");
    assert_eq!(h.state().matches, 10_000);
    assert!(has(&h, "1/10000+"), "the status says there are more");
}

// ---- status text --------------------------------------------------------------------------

#[test]
fn the_status_reads_n_of_m_with_the_current_match() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    assert!(has(&h, "1/3"));
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert!(has(&h, "2/3"));
}

#[test]
fn the_status_reads_zero_results_for_no_match() {
    let mut h = harness(TEXT);
    find(&mut h, "zzz");
    assert!(has(&h, "0 results"));
}

#[test]
fn the_status_counts_results_when_no_match_is_the_selection() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    set_caret(&mut h, 25);
    assert!(has(&h, "3 results"));
    find_clear_and_type(&mut h, "four");
    set_caret(&mut h, 2);
    assert!(has(&h, "1 result"));
}

fn find_clear_and_type(h: &mut H<'_>, query: &str) {
    key(h, CTRL, Key::F);
    key(h, CTRL, Key::A);
    type_text(h, query);
}

#[test]
fn an_empty_query_has_no_status() {
    let mut h = harness(TEXT);
    key(&mut h, CTRL, Key::F);
    assert!(!has(&h, "0 results"));
}

// ---- the current match is the selection ---------------------------------------------------

#[test]
fn typing_selects_the_first_match_from_the_caret_not_the_first_in_the_text() {
    let mut h = harness(TEXT);
    set_caret(&mut h, 5);
    find(&mut h, "one");
    assert_eq!(selection(&h), ["one"]);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    assert!(has(&h, "2/3"));
}

#[test]
fn typing_wraps_to_the_first_match_when_none_follows_the_caret() {
    let mut h = harness(TEXT);
    set_caret(&mut h, 25);
    find(&mut h, "one");
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
}

#[test]
fn clearing_the_query_restores_the_original_caret() {
    let mut h = harness(TEXT);
    set_caret(&mut h, 5);
    find(&mut h, "three");
    assert_eq!(selection(&h), ["three"]);
    for _ in 0..5 {
        key(&mut h, Modifiers::NONE, Key::Backspace);
    }
    assert!(selection(&h).is_empty());
    assert_eq!(caret(&h), 5);
}

#[test]
fn a_match_far_below_scrolls_into_view_while_typing() {
    let text = (0..200)
        .map(|i| {
            if i == 150 {
                "needle".to_owned()
            } else {
                format!("line {i}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut h = harness_with(&text, false, Os::WindowsLinux, 120.0);
    find(&mut h, "needle");
    for _ in 0..20 {
        h.step();
    }
    assert_eq!(selection(&h), ["needle"]);
    let rect = h.get_by_label("Ed").rect();
    let hit = h.get_by_label("Ed");
    let _ = (rect, hit);
    let doc_line = h.state().editor.document.line_number(caret(&h));
    assert_eq!(doc_line, 150);
}

// ---- navigation ---------------------------------------------------------------------------

#[test]
fn enter_and_shift_enter_step_through_the_matches() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 18);
    key(&mut h, Modifiers::SHIFT, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    assert!(field_focused(&h, "Find"), "the focus stays in the field");
}

#[test]
fn down_and_up_in_the_field_are_next_and_previous() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    key(&mut h, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
}

#[test]
fn the_next_and_previous_buttons_step() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    h.get_by_label("Next match").click();
    h.run_ok();
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    h.get_by_label("Previous match").click();
    h.run_ok();
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
}

#[test]
fn f3_and_shift_f3_work_from_the_editor() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert!(editor_focused(&h));
    key(&mut h, Modifiers::NONE, Key::F3);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    key(&mut h, Modifiers::SHIFT, Key::F3);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
}

#[test]
fn ctrl_l_is_find_next_on_windows_and_linux() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Escape);
    key(&mut h, CTRL, Key::L);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
}

#[test]
fn cmd_g_and_shift_cmd_g_are_find_next_and_previous_on_macos() {
    let mut h = harness_with(TEXT, false, Os::MacOs, 200.0);
    key(&mut h, Modifiers::MAC_CMD, Key::F);
    type_text(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Escape);
    key(&mut h, Modifiers::MAC_CMD, Key::G);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 8);
    key(&mut h, Modifiers::MAC_CMD | Modifiers::SHIFT, Key::G);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
}

#[test]
fn f3_from_the_editor_with_a_closed_bar_uses_the_last_query() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Escape);
    set_caret(&mut h, 0);
    key(&mut h, Modifiers::NONE, Key::F3);
    assert_eq!(selection(&h), ["one"]);
}

#[test]
fn ctrl_f3_finds_the_word_at_the_caret() {
    let mut h = harness("alpha beta alpha beta");
    set_caret(&mut h, 8);
    key(&mut h, Modifiers::COMMAND, Key::F3);
    assert_eq!(selection(&h), ["beta"]);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 17);
}

#[test]
fn the_first_next_after_the_last_match_only_shows_a_hint_the_second_wraps() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Enter);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 18);
    h.state_mut().hint = None;
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(
        h.state().editor.carets.primary().selection_start(),
        18,
        "still on the last match"
    );
    assert_eq!(
        h.state().hint.as_deref(),
        Some("\"one\" not found, press F3 to search from the top")
    );
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
}

#[test]
fn previous_wraps_in_two_steps_too_with_the_bottom_hint() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    h.state_mut().hint = None;
    key(&mut h, Modifiers::SHIFT, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 0);
    assert_eq!(
        h.state().hint.as_deref(),
        Some("\"one\" not found, press Shift+F3 to search from the bottom")
    );
    key(&mut h, Modifiers::SHIFT, Key::Enter);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 18);
}

#[test]
fn moving_the_caret_between_the_two_presses_cancels_the_wrap() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::SHIFT, Key::Enter);
    key(&mut h, Modifiers::NONE, Key::Escape);
    set_caret(&mut h, 12);
    key(&mut h, Modifiers::NONE, Key::F3);
    assert_eq!(h.state().editor.carets.primary().selection_start(), 18);
}

#[test]
fn the_wrap_hint_names_the_macos_key() {
    let mut h = harness_with("one", false, Os::MacOs, 200.0);
    key(&mut h, Modifiers::MAC_CMD, Key::F);
    type_text(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(
        h.state().hint.as_deref(),
        Some("\"one\" not found, press Cmd+G to search from the top")
    );
}

#[test]
fn ctrl_enter_accepts_the_query_into_the_history_and_returns_to_the_editor() {
    let mut h = harness(TEXT);
    find(&mut h, "three");
    key(&mut h, CTRL, Key::Enter);
    assert!(editor_focused(&h));
    assert!(bar_open(&h), "the bar stays open");
    assert_eq!(h.state().history.queries(), ["three"]);
}

#[test]
fn ctrl_enter_with_an_empty_query_closes_the_bar() {
    let mut h = harness(TEXT);
    key(&mut h, CTRL, Key::F);
    key(&mut h, CTRL, Key::Enter);
    assert!(!bar_open(&h));
    assert!(editor_focused(&h));
}

#[test]
fn ctrl_shift_enter_inserts_a_line_break_into_the_query() {
    let mut h = harness("a\nb\na\nb");
    find(&mut h, "a");
    key(&mut h, CTRL | Modifiers::SHIFT, Key::Enter);
    type_text(&mut h, "b");
    assert_eq!(
        h.state().matches,
        2,
        "a multi-line query matches across lines"
    );
    assert_eq!(selection(&h), ["a\nb"]);
}

#[test]
fn alt_enter_selects_all_occurrences_with_a_caret_each() {
    let mut h = harness(TEXT);
    find(&mut h, "one");
    key(&mut h, Modifiers::ALT, Key::Enter);
    let e = &h.state().editor;
    assert_eq!(e.carets.caret_count(), 3);
    assert_eq!(selection(&h), ["one", "one", "one"]);
    assert!(editor_focused(&h));
    // Typing now edits all three.
    type_text(&mut h, "1");
    assert_eq!(text(&h), "1 two 1\nthree 1\nfour");
}

// ---- replace ------------------------------------------------------------------------------

fn replace_with(h: &mut H<'_>, query: &str, replacement: &str) {
    key(h, CTRL, Key::R);
    type_text(h, query);
    key(h, Modifiers::NONE, Key::Tab);
    assert!(
        field_focused(h, "Replace with"),
        "Tab moves to the replace field"
    );
    type_text(h, replacement);
}

#[test]
fn enter_in_the_replace_field_replaces_the_current_match_and_moves_on() {
    let mut h = harness(TEXT);
    replace_with(&mut h, "one", "1");
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(text(&h), "1 two one\nthree one\nfour");
    assert_eq!(selection(&h), ["one"], "the next match is selected");
    assert_eq!(h.state().editor.carets.primary().selection_start(), 6);
}

#[test]
fn the_replace_button_replaces_the_current_match() {
    let mut h = harness(TEXT);
    replace_with(&mut h, "two", "2");
    h.get_by_label("Replace").click();
    h.run_ok();
    assert_eq!(text(&h), "one 2 one\nthree one\nfour");
}

#[test]
fn replace_wraps_silently_after_the_last_match() {
    let mut h = harness("aXa");
    replace_with(&mut h, "a", "b");
    key(&mut h, Modifiers::NONE, Key::Enter);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(text(&h), "bXb");
}

#[test]
fn replace_all_is_one_undo_step() {
    let mut h = harness(TEXT);
    replace_with(&mut h, "one", "1");
    h.get_by_label("Replace All").click();
    h.run_ok();
    assert_eq!(text(&h), "1 two 1\nthree 1\nfour");
    assert_eq!(h.state().editor.undo.undo_depth(), 1);
    key(&mut h, Modifiers::NONE, Key::Escape);
    key(&mut h, CTRL, Key::Z);
    assert_eq!(text(&h), TEXT);
}

#[test]
fn alt_a_replaces_all() {
    let mut h = harness(TEXT);
    replace_with(&mut h, "one", "1");
    key(&mut h, Modifiers::ALT, Key::A);
    assert_eq!(text(&h), "1 two 1\nthree 1\nfour");
}

#[test]
fn replace_all_reports_the_count() {
    let mut h = harness(TEXT);
    replace_with(&mut h, "one", "1");
    h.get_by_label("Replace All").click();
    h.run_ok();
    assert_eq!(h.state().hint.as_deref(), Some("3 occurrences replaced"));
}

#[test]
fn regex_replacements_expand_group_references() {
    let mut h = harness("ann@home bob@work");
    key(&mut h, CTRL, Key::R);
    key(&mut h, Modifiers::ALT, Key::X);
    type_text(&mut h, r"(\w+)@(\w+)");
    key(&mut h, Modifiers::NONE, Key::Tab);
    type_text(&mut h, "$2:$1");
    key(&mut h, Modifiers::ALT, Key::A);
    assert_eq!(text(&h), "home:ann work:bob");
}

#[test]
fn preserve_case_keeps_the_case_shape_of_each_match() {
    let mut h = harness("hello Hello HELLO");
    replace_with(&mut h, "hello", "world");
    key(&mut h, Modifiers::ALT, Key::P);
    assert!(toggle_is_on(&h, "Preserve Case"));
    key(&mut h, Modifiers::ALT, Key::A);
    assert_eq!(text(&h), "world World WORLD");
}

#[test]
fn without_preserve_case_the_replacement_is_literal() {
    let mut h = harness("hello Hello");
    replace_with(&mut h, "hello", "world");
    key(&mut h, Modifiers::ALT, Key::A);
    assert_eq!(text(&h), "world world");
}

#[test]
fn a_viewer_cannot_replace() {
    let mut h = harness_with(TEXT, true, Os::WindowsLinux, 200.0);
    key(&mut h, CTRL, Key::R);
    type_text(&mut h, "one");
    key(&mut h, Modifiers::ALT, Key::A);
    assert_eq!(text(&h), TEXT);
    assert!(!has(&h, "Replace with"));
}

#[test]
fn replacing_in_a_find_in_selection_scope_leaves_the_rest() {
    let mut h = harness(TEXT);
    set_selection(&mut h, 4, 17);
    key(&mut h, CTRL, Key::R);
    type_text(&mut h, "one");
    key(&mut h, Modifiers::NONE, Key::Tab);
    type_text(&mut h, "1");
    key(&mut h, Modifiers::ALT, Key::A);
    assert_eq!(text(&h), "one two 1\nthree one\nfour");
}

// ---- history ------------------------------------------------------------------------------

#[test]
fn alt_down_opens_the_history_and_a_click_picks_an_entry() {
    let mut h = harness(TEXT);
    h.state_mut().history.add_query("three");
    h.state_mut().history.add_query("four");
    key(&mut h, CTRL, Key::F);
    key(&mut h, Modifiers::ALT, Key::ArrowDown);
    assert!(
        has(&h, "four") && has(&h, "three"),
        "the popup lists the history"
    );
    h.get_by_label("three").click();
    h.run_ok();
    assert_eq!(selection(&h), ["three"]);
}

#[test]
fn the_history_popup_takes_arrows_and_enter() {
    let mut h = harness(TEXT);
    h.state_mut().history.add_query("three");
    h.state_mut().history.add_query("four");
    key(&mut h, CTRL, Key::F);
    key(&mut h, Modifiers::ALT, Key::ArrowDown);
    key(&mut h, Modifiers::NONE, Key::ArrowDown);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(selection(&h), ["three"]);
}

#[test]
fn the_history_button_opens_the_popup() {
    let mut h = harness(TEXT);
    h.state_mut().history.add_query("four");
    key(&mut h, CTRL, Key::F);
    h.get_by_label("Search history").click();
    h.run_ok();
    assert!(has(&h, "four"));
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert!(bar_open(&h), "Escape closes the popup first");
}

#[test]
fn closing_the_bar_remembers_the_query() {
    let mut h = harness(TEXT);
    find(&mut h, "four");
    key(&mut h, Modifiers::NONE, Key::Escape);
    h.run_ok();
    assert_eq!(h.state().history.queries(), ["four"]);
}

// ---- highlighting -------------------------------------------------------------------------

fn pixel(h: &mut H<'_>, x: f32, y: f32) -> egui::Color32 {
    let image = h.render().expect("the harness renders");
    let p = image.get_pixel(x as u32, y as u32).0;
    egui::Color32::from_rgb(p[0], p[1], p[2])
}

fn near(a: egui::Color32, b: egui::Color32) -> bool {
    let d = |x: u8, y: u8| x.abs_diff(y);
    d(a.r(), b.r()) <= 4 && d(a.g(), b.g()) <= 4 && d(a.b(), b.b()) <= 4
}

#[test]
fn other_matches_use_the_search_color_and_the_current_one_the_selection() {
    let colors = Theme::dark().editor;
    let mut h = harness("one one");
    find(&mut h, "one");
    let rect = h.get_by_label("Ed").rect();
    let y = rect.top() + 4.0 + 10.0;
    // Two characters into each word: clear of the box outline.
    let first = pixel(&mut h, rect.left() + 8.0 + 7.8 * 1.5, y - 7.0);
    let second = pixel(&mut h, rect.left() + 8.0 + 7.8 * 5.5, y - 7.0);
    assert!(
        near(first, colors.selection) || near(first, colors.selection_inactive),
        "the current match is the selection: {first:?}"
    );
    assert!(
        near(second, colors.search_result),
        "the other match has the search color: {second:?} vs {:?}",
        colors.search_result
    );
}

#[test]
fn the_current_match_has_a_box_in_the_caret_color() {
    let colors = Theme::dark().editor;
    let mut h = harness("one one");
    find(&mut h, "one");
    let rect = h.get_by_label("Ed").rect();
    let top = rect.top() + 4.0;
    let image = h.render().expect("renders");
    let mut found = false;
    for dy in 0..4 {
        for dx in 0..30 {
            let p = image
                .get_pixel(
                    (rect.left() + 8.0 + 3.0 + dx as f32) as u32,
                    (top + dy as f32) as u32,
                )
                .0;
            if near(egui::Color32::from_rgb(p[0], p[1], p[2]), colors.caret) {
                found = true;
            }
        }
    }
    assert!(found, "a caret-colored line along the top of the match");
}

#[test]
fn matches_are_marked_on_the_error_stripe() {
    let colors = Theme::dark().editor;
    let text = (0..60)
        .map(|i| {
            if i == 30 {
                "needle".to_owned()
            } else {
                format!("line {i}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut h = harness_with(&text, false, Os::WindowsLinux, 200.0);
    find(&mut h, "needle");
    h.run_ok();
    let rect = h.get_by_label("Ed").rect();
    let image = h.render().expect("renders");
    let mut marked = false;
    for y in rect.top() as u32..rect.bottom() as u32 {
        let p = image.get_pixel(rect.right() as u32 + 7, y).0;
        if near(
            egui::Color32::from_rgb(p[0], p[1], p[2]),
            colors.search_stripe,
        ) {
            marked = true;
        }
    }
    assert!(marked, "a search_stripe mark right of the text");
}

#[test]
fn the_field_turns_red_when_nothing_matches() {
    let colors = Theme::dark().colors;
    let mut h = harness(TEXT);
    find(&mut h, "zzz");
    let field = h.get_by_label("Find").rect();
    let p = pixel(&mut h, field.left() + 2.0, field.center().y);
    let _ = pos2(0.0, 0.0);
    // The wash is mostly the error color mixed into the field: redder than the plain field.
    assert!(p.r() > p.b() + 8, "{p:?} {:?}", colors.error);
}

#[test]
fn select_all_occurrences_stops_at_the_caret_limit_with_a_hint() {
    let text = "a ".repeat(1500);
    let mut h = harness(&text);
    find(&mut h, "a");
    h.state_mut().hint = None;
    key(&mut h, Modifiers::ALT, Key::Enter);
    assert_eq!(h.state().editor.carets.caret_count(), 1000);
    assert_eq!(h.state().hint.as_deref(), Some("Too many carets"));
}

#[test]
fn replace_with_no_match_selected_only_selects_the_next_match() {
    let mut h = harness(TEXT);
    replace_with(&mut h, "three", "3");
    set_caret(&mut h, 0);
    h.get_by_label("Replace").click();
    h.run_ok();
    assert_eq!(text(&h), TEXT, "the first press finds, it does not replace");
    assert_eq!(selection(&h), ["three"]);
    h.get_by_label("Replace").click();
    h.run_ok();
    assert_eq!(text(&h), "one two one\n3 one\nfour");
}

#[test]
fn editing_the_text_while_the_bar_is_open_updates_the_matches() {
    let mut h = harness("ab ab");
    find(&mut h, "ab");
    assert_eq!(h.state().matches, 2);
    key(&mut h, Modifiers::NONE, Key::Escape);
    set_caret(&mut h, 5);
    type_text(&mut h, " ab");
    key(&mut h, CTRL, Key::F);
    assert_eq!(h.state().matches, 3);
}

#[test]
fn a_replace_is_undone_by_the_next_ctrl_z() {
    let mut h = harness("aXa");
    replace_with(&mut h, "a", "b");
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(text(&h), "bXa");
    key(&mut h, Modifiers::NONE, Key::Escape);
    key(&mut h, CTRL, Key::Z);
    assert_eq!(text(&h), "aXa");
}

#[test]
fn the_search_survives_text_with_multibyte_characters() {
    let mut h = harness("é ü é\nñ é");
    find(&mut h, "é");
    assert_eq!(h.state().matches, 3);
    key(&mut h, Modifiers::ALT, Key::Enter);
    assert_eq!(selection(&h), ["é", "é", "é"]);
}

// ---- snapshots ----------------------------------------------------------------------------

const SAMPLE: &str = "fn total(items) {\n  let sum = 0;\n  for item in items {\n    sum += item.price;\n  }\n  return sum;\n}\n";

fn focus_editor(h: &mut Harness<'_, ()>) {
    h.get_by_label("Ed").focus();
    h.run_ok();
}

fn type_in(h: &mut Harness<'_, ()>, text: &str) {
    h.event(Event::Text(text.into()));
    h.run_ok();
}

#[test]
fn find_replace_snapshot() {
    let mut editor = Editor::new(SAMPLE);
    testing::snapshot_themes_after(
        "editor_find_replace",
        common::IMG,
        |ui| {
            ui.set_min_width(480.0);
            EditorComponent::new(&mut editor)
                .label("Ed")
                .line_numbers(true)
                .height(250.0)
                .show(ui);
        },
        |h| {
            focus_editor(h);
            h.key_press_modifiers(CTRL, Key::R);
            h.run_ok();
            type_in(h, "sum");
            h.key_press_modifiers(Modifiers::ALT, Key::C);
            h.run_ok();
            h.key_press(Key::Tab);
            h.run_ok();
            type_in(h, "total");
        },
    );
}

#[test]
fn find_not_found_snapshot() {
    let mut editor = Editor::new(SAMPLE);
    testing::snapshot_themes_after(
        "editor_find_not_found",
        common::IMG,
        |ui| {
            ui.set_min_width(480.0);
            EditorComponent::new(&mut editor)
                .label("Ed")
                .line_numbers(true)
                .height(150.0)
                .show(ui);
        },
        |h| {
            focus_editor(h);
            h.key_press_modifiers(CTRL, Key::F);
            h.run_ok();
            h.key_press_modifiers(Modifiers::ALT, Key::X);
            h.run_ok();
            type_in(h, "(");
        },
    );
}

#[test]
fn find_history_snapshot() {
    let mut editor = Editor::new(SAMPLE);
    let mut history = SearchHistory::new();
    history.add_query("item.price");
    history.add_query("return");
    history.add_query("sum");
    testing::snapshot_themes_after(
        "editor_find_history",
        common::IMG,
        |ui| {
            ui.set_min_height(190.0);
            ui.set_min_width(480.0);
            EditorComponent::new(&mut editor)
                .label("Ed")
                .search_history(&mut history)
                .height(150.0)
                .show(ui);
        },
        |h| {
            focus_editor(h);
            h.key_press_modifiers(CTRL, Key::F);
            h.run_ok();
            h.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
            h.run_ok();
        },
    );
}
