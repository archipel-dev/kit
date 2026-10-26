//! Tests of the JetBrains-faithful editor model (`archipel_kit::editor::model`).
//!
//! One test per rule, named after the rule, with the section of the editor UX spec it comes
//! from in a comment (`spec 1.2` = section 1.2 of `editor-ux-spec.md`).
//!
//! Notation of the helpers: `|` is a caret, `‹abc›` a forward selection (caret at the end),
//! `«abc»` a backward selection (caret at the start).

#![allow(clippy::unwrap_used)]

use archipel_kit::editor::model::*;
use egui::Key;

// ---- helpers ---------------------------------------------------------------------------

fn parse(marked: &str) -> (String, Vec<(usize, usize)>) {
    let mut text = String::new();
    let mut carets = Vec::new();
    let mut forward = None;
    let mut backward = None;
    for c in marked.chars() {
        match c {
            '|' => carets.push((text.len(), text.len())),
            '‹' => forward = Some(text.len()),
            '›' => {
                if let Some(start) = forward.take() {
                    carets.push((start, text.len()));
                }
            }
            '«' => backward = Some(text.len()),
            '»' => {
                if let Some(start) = backward.take() {
                    carets.push((text.len(), start));
                }
            }
            c => text.push(c),
        }
    }
    (text, carets)
}

fn editor(marked: &str) -> Editor {
    let (text, carets) = parse(marked);
    let mut editor = Editor::new(&text);
    place(&mut editor, &carets);
    editor
}

fn place(editor: &mut Editor, carets: &[(usize, usize)]) {
    let doc = &editor.document;
    match carets.split_first() {
        None => editor.carets.set_caret(doc, 0),
        Some((first, rest)) => {
            editor.carets.set_selection(doc, first.0, first.1);
            for (anchor, lead) in rest {
                editor.carets.add_caret_with_selection(doc, *anchor, *lead);
            }
        }
    }
    editor.undo.set_baseline(&editor.carets);
}

fn show(editor: &Editor) -> String {
    let mut marks: Vec<(usize, u8, char)> = Vec::new();
    for caret in editor.carets.carets() {
        if !caret.has_selection() {
            marks.push((caret.offset, 0, '|'));
        } else if caret.offset > caret.anchor {
            marks.push((caret.anchor, 1, '‹'));
            marks.push((caret.offset, 0, '›'));
        } else {
            marks.push((caret.offset, 1, '«'));
            marks.push((caret.anchor, 0, '»'));
        }
    }
    marks.sort_by_key(|m| std::cmp::Reverse((m.0, m.1)));
    let mut text = editor.document.text().to_owned();
    for (offset, _, c) in marks {
        text.insert(offset, c);
    }
    text
}

fn plain() -> ActionContext<'static> {
    ActionContext::plain_text()
}

fn run(editor: &mut Editor, action: EditorAction) -> EditOutcome {
    editor.execute(&action, &plain())
}

/// Runs an action and returns the marked text afterwards.
fn after(marked: &str, action: EditorAction) -> String {
    let mut e = editor(marked);
    run(&mut e, action);
    show(&e)
}

fn after_lang(marked: &str, action: EditorAction, language: &Language) -> String {
    let mut e = editor(marked);
    e.execute(&action, &ActionContext::new(language));
    show(&e)
}

fn type_str(editor: &mut Editor, text: &str, language: &Language) {
    for c in text.chars() {
        editor.execute(&EditorAction::Type(c), &ActionContext::new(language));
    }
}

fn paste(marked: &str, clip: ClipboardContent, action: EditorAction) -> String {
    let mut e = editor(marked);
    let language = Language::plain_text();
    e.execute(
        &action,
        &ActionContext::new(&language).with_clipboard(&clip),
    );
    show(&e)
}

use EditorAction as A;

// ---- document -------------------------------------------------------------------------

#[test]
fn document_line_index_stays_exact_under_edits() {
    // The incrementally maintained index equals a rebuild after every edit.
    let mut doc = Document::new("one\ntwo\n\nfour");
    let mut seed = 7u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let pieces = ["", "x", "\n", "ab\ncd", "é\n", "\n\n", "😀"];
    for _ in 0..500 {
        let len = doc.len() as u64;
        let a = (next() % (len + 1)) as usize;
        let b = (next() % (len + 1)) as usize;
        doc.replace_string(a, b, pieces[(next() % pieces.len() as u64) as usize]);
        let rebuilt = Document::new(doc.text());
        assert_eq!(doc.line_count(), rebuilt.line_count());
        for line in 0..doc.line_count() {
            assert_eq!(doc.line_start_offset(line), rebuilt.line_start_offset(line));
            assert_eq!(doc.line_end_offset(line), rebuilt.line_end_offset(line));
        }
    }
}

#[test]
fn document_clamps_and_snaps_offsets() {
    let mut doc = Document::new("aé😀b");
    assert_eq!(doc.snap(2), 1, "inside é snaps back");
    assert_eq!(doc.snap(1000), doc.len());
    assert_eq!(doc.line_number(9999), 0);
    assert_eq!(doc.line_start_offset(5), doc.len());
    doc.replace_string(500, 2, "Z");
    assert!(doc.text().is_char_boundary(doc.len()));
    assert!(doc.delete_string(10_000, 20_000).is_none());
}

#[test]
fn document_normalizes_crlf_to_lf() {
    // The model holds `\n` only; callers remember the file's separator.
    let doc = Document::new("a\r\nb\rc");
    assert_eq!(doc.text(), "a\nb\nc");
    assert_eq!(doc.line_count(), 3);
    let mut doc = doc;
    doc.insert_string(0, "x\r\ny");
    assert_eq!(doc.text(), "x\nya\nb\nc");
}

#[test]
fn document_trailing_break_has_an_empty_last_line() {
    let doc = Document::new("a\n");
    assert_eq!(doc.line_count(), 2);
    assert_eq!(doc.line_text(1), "");
    assert_eq!(Document::new("").line_count(), 1);
}

#[test]
fn document_modification_stamp_counts_changes() {
    let mut doc = Document::new("a");
    let stamp = doc.modification_stamp();
    doc.insert_string(1, "b");
    assert!(doc.modification_stamp() > stamp);
    let stamp = doc.modification_stamp();
    assert!(doc.insert_string(0, "").is_none());
    assert_eq!(doc.modification_stamp(), stamp);
}

#[test]
fn logical_columns_expand_tabs() {
    let doc = Document::new("\tab");
    assert_eq!(column_of(&doc, 1, 4), 4);
    assert_eq!(column_of(&doc, 3, 4), 6);
    assert_eq!(offset_to_logical(&doc, 3, 4), LogicalPosition::new(0, 6));
    assert_eq!(logical_to_offset(&doc, LogicalPosition::new(0, 5), 4), 2);
    // Inside a tab the caret snaps to the nearer side.
    assert_eq!(logical_to_offset(&doc, LogicalPosition::new(0, 1), 4), 0);
    assert_eq!(logical_to_offset(&doc, LogicalPosition::new(0, 3), 4), 1);
    assert_eq!(logical_to_offset(&doc, LogicalPosition::new(9, 99), 4), 3);
}

// ---- word motion (spec 1.2) -------------------------------------------------------------

#[test]
fn ctrl_right_stops_at_word_ends_only() {
    // spec 1.2: forward stops only at word ends.
    assert_eq!(after("|foo bar", A::NextWord), "foo| bar");
    assert_eq!(after("foo| bar", A::NextWord), "foo bar|");
}

#[test]
fn ctrl_right_at_line_end_goes_to_next_line_start() {
    // spec 1.2: the limit is the start of the next line.
    assert_eq!(after("foo bar|\nbaz", A::NextWord), "foo bar\n|baz");
}

#[test]
fn ctrl_right_skips_trailing_blanks_to_next_line() {
    // spec 1.2: from `foo|   ` the caret lands at column 0 of the next line.
    assert_eq!(after("foo|   \nbar", A::NextWord), "foo   \n|bar");
}

#[test]
fn ctrl_right_on_last_line_is_limited_by_document_end() {
    assert_eq!(after("a |b", A::NextWord), "a b|");
    assert_eq!(after("ab  |", A::NextWord), "ab  |");
}

#[test]
fn ctrl_left_stops_at_word_starts_only() {
    // spec 1.2: backward stops only at word starts.
    assert_eq!(after("foo bar|", A::PreviousWord), "foo |bar");
    assert_eq!(after("foo |bar", A::PreviousWord), "|foo bar");
}

#[test]
fn ctrl_left_without_word_start_jumps_to_previous_line_end() {
    // spec 1.2: the limit is the end of the previous line, even mid-line.
    assert_eq!(after("foo\n  |bar", A::PreviousWord), "foo|\n  bar");
    assert_eq!(after("foo\n|bar", A::PreviousWord), "foo|\nbar");
}

#[test]
fn punctuation_runs_are_one_word() {
    // spec 1.2: `->` is one word.
    let mut e = editor("|a->b");
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "a|->b");
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "a->|b");
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "a->b|");
}

#[test]
fn camel_humps_are_off_by_default() {
    // spec 0 / 1.2: default off, so a camel-case identifier is one word.
    assert_eq!(after("|fooBarBaz x", A::NextWord), "fooBarBaz| x");
}

#[test]
fn camel_humps_add_hump_boundaries() {
    // spec 1.2: lower->upper, uppercase runs, underscores.
    let mut e = editor("|fooBarBaz");
    e.settings.camel_humps = true;
    for expected in ["foo|BarBaz", "fooBar|Baz", "fooBarBaz|"] {
        run(&mut e, A::NextWord);
        assert_eq!(show(&e), expected);
    }
    for expected in ["fooBar|Baz", "foo|BarBaz", "|fooBarBaz"] {
        run(&mut e, A::PreviousWord);
        assert_eq!(show(&e), expected);
    }
    let mut e = editor("|XMLHttp");
    e.settings.camel_humps = true;
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "XML|Http");
    let mut e = editor("|foo_bar");
    e.settings.camel_humps = true;
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "foo|_bar");
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "foo_|bar");
}

#[test]
fn caret_stop_presets_change_the_stops() {
    // spec 1.2: DEFAULT_WINDOWS stops at word starts, DEFAULT_UNIX has no line stops.
    let mut e = editor("|foo bar");
    e.settings.caret_stop = CaretStopOptions::WINDOWS;
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "foo |bar");
    let mut e = editor("foo bar|\nbaz");
    e.settings.caret_stop = CaretStopOptions::UNIX;
    run(&mut e, A::NextWord);
    assert_eq!(show(&e), "foo bar\nbaz|");
}

#[test]
fn word_motion_counts_unicode_letters_as_word_chars() {
    // spec 1.2: identifier part includes Unicode letters.
    assert_eq!(after("|héllo wörld", A::NextWord), "héllo| wörld");
    assert_eq!(after("|日本語 x", A::NextWord), "日本語| x");
}

#[test]
fn word_motion_selection_variants_keep_the_anchor() {
    assert_eq!(after("|foo bar", A::NextWordWithSelection), "‹foo› bar");
    assert_eq!(after("foo bar|", A::PreviousWordWithSelection), "foo «bar»");
}

#[test]
fn ctrl_backspace_and_ctrl_delete_delete_to_word_boundaries() {
    // spec 1.2: Ctrl+Backspace to the word start, Ctrl+Delete to the next boundary.
    assert_eq!(after("foo bar|", A::DeleteToWordStart), "foo |");
    assert_eq!(after("foo\n|bar", A::DeleteToWordStart), "foo|bar");
    assert_eq!(after("|foo bar", A::DeleteToWordEnd), "| bar");
    assert_eq!(after("foo|\nbar", A::DeleteToWordEnd), "foo|bar");
}

// ---- basic caret movement (spec 1.1) ---------------------------------------------------

#[test]
fn left_and_right_collapse_a_selection_to_its_edges() {
    // spec 1.1: no extra step.
    assert_eq!(after("a‹bcd›e", A::Left), "a|bcde");
    assert_eq!(after("a‹bcd›e", A::Right), "abcd|e");
    assert_eq!(after("a«bcd»e", A::Left), "a|bcde");
}

#[test]
fn up_and_down_with_a_selection_first_jump_to_its_edge() {
    // spec 1.1.
    let mut e = editor("a‹b\nc›d\nef");
    run(&mut e, A::Down);
    assert_eq!(show(&e), "ab\ncd\ne|f");
    let mut e = editor("ab\nc‹d\ne›f");
    run(&mut e, A::Up);
    assert_eq!(show(&e), "a|b\ncd\nef");
}

#[test]
fn left_at_column_zero_goes_to_previous_line_end() {
    assert_eq!(after("ab\n|cd", A::Left), "ab|\ncd");
    assert_eq!(after("ab|\ncd", A::Right), "ab\n|cd");
    assert_eq!(after("|ab", A::Left), "|ab");
    assert_eq!(after("ab|", A::Right), "ab|");
}

#[test]
fn left_and_right_move_by_whole_characters() {
    assert_eq!(after("a|😀b", A::Right), "a😀|b");
    assert_eq!(after("a😀|b", A::Left), "a|😀b");
    assert_eq!(after("é|", A::Left), "|é");
}

#[test]
fn up_on_first_line_goes_to_column_zero_and_down_on_last_to_the_end() {
    // spec 1.1: neither is a no-op.
    assert_eq!(after("ab|cd\nef", A::Up), "|abcd\nef");
    assert_eq!(after("ab\ncd|ef", A::Down), "ab\ncdef|");
    assert_eq!(after("ab|cd\nef", A::UpWithSelection), "«ab»cd\nef");
}

#[test]
fn vertical_motion_remembers_the_desired_column() {
    // spec 1.1: shorter line lands at its end, longer line returns to the column.
    let mut e = editor("abcdef\nab\nabcdef");
    place(&mut e, &[(5, 5)]);
    run(&mut e, A::Down);
    assert_eq!(show(&e), "abcdef\nab|\nabcdef");
    run(&mut e, A::Down);
    assert_eq!(show(&e), "abcdef\nab\nabcde|f");
    run(&mut e, A::Left);
    run(&mut e, A::Up);
    assert_eq!(show(&e), "abcdef\nab|\nabcdef");
}

#[test]
fn shift_motion_keeps_the_anchor() {
    assert_eq!(after("a|bc", A::RightWithSelection), "a‹b›c");
    assert_eq!(after("ab|c", A::LeftWithSelection), "a«b»c");
    let mut e = editor("a|bcd");
    run(&mut e, A::RightWithSelection);
    run(&mut e, A::RightWithSelection);
    run(&mut e, A::LeftWithSelection);
    assert_eq!(show(&e), "a‹b›cd");
}

#[test]
fn motion_works_for_all_carets() {
    assert_eq!(after("a|bc\nd|ef", A::Right), "ab|c\nde|f");
}

#[test]
fn carets_that_meet_merge() {
    // spec 1.5: identical carets merge.
    let mut e = editor("a|b|c");
    run(&mut e, A::Right);
    assert_eq!(e.carets.caret_count(), 2);
    let mut e = editor("a|b|");
    run(&mut e, A::Right);
    assert_eq!(e.carets.caret_count(), 1);
}

// ---- Home / End (spec 1.3) -------------------------------------------------------------

#[test]
fn smart_home_toggles_between_indent_and_column_zero() {
    // spec 1.3.
    let mut e = editor("  ab|c");
    run(&mut e, A::LineStart);
    assert_eq!(show(&e), "  |abc");
    run(&mut e, A::LineStart);
    assert_eq!(show(&e), "|  abc");
    run(&mut e, A::LineStart);
    assert_eq!(show(&e), "  |abc");
}

#[test]
fn smart_home_inside_the_indentation_goes_to_column_zero() {
    assert_eq!(after("  | x", A::LineStart), "|   x");
}

#[test]
fn home_without_smart_home_goes_to_column_zero() {
    let mut e = editor("  ab|c");
    e.settings.smart_home = false;
    run(&mut e, A::LineStart);
    assert_eq!(show(&e), "|  abc");
    run(&mut e, A::LineStart);
    assert_eq!(show(&e), "|  abc");
}

#[test]
fn home_on_a_blank_line_uses_the_indent_of_the_line_above() {
    // spec 1.3.
    assert_eq!(after("    foo\n  |", A::LineStart), "    foo\n|  ");
    assert_eq!(after("    foo\n|  ", A::LineStart), "    foo\n  |");
}

#[test]
fn smart_end_stops_before_trailing_whitespace_first() {
    // spec 1.3.
    let mut e = editor("ab|c   ");
    run(&mut e, A::LineEnd);
    assert_eq!(show(&e), "abc|   ");
    run(&mut e, A::LineEnd);
    assert_eq!(show(&e), "abc   |");
    assert_eq!(after("a|b", A::LineEnd), "ab|");
}

#[test]
fn home_and_end_extend_the_selection_with_shift() {
    assert_eq!(after("  ab|c", A::LineStartWithSelection), "  «ab»c");
    assert_eq!(after("a|bc   ", A::LineEndWithSelection), "a‹bc›   ");
}

#[test]
fn ctrl_home_and_end_drop_secondary_carets_and_selection() {
    // spec 1.3.
    assert_eq!(after("ab|c\nd‹e›f", A::TextStart), "|abc\ndef");
    assert_eq!(after("ab|c\nd‹e›f", A::TextEnd), "abc\ndef|");
    let mut e = editor("a|b\nc|d");
    let out = run(&mut e, A::TextEnd);
    assert!(matches!(
        out.scroll,
        Some(ScrollRequest::ToCaret {
            scroll_type: ScrollType::Center,
            animate: false
        })
    ));
}

// ---- page movement (spec 1.4) ----------------------------------------------------------

fn numbered(lines: usize) -> String {
    (0..lines)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn page_down_moves_the_caret_and_scrolls_by_the_same_lines() {
    // spec 1.4.
    let mut e = Editor::new(&numbered(100));
    let ctx = ActionContext::plain_text().with_viewport(0, 11);
    let out = e.execute(&A::PageDown, &ctx);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 10);
    assert_eq!(out.scroll, Some(ScrollRequest::Lines(10)));
    let out = e.execute(&A::PageUp, &ctx);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 0);
    assert_eq!(out.scroll, Some(ScrollRequest::Lines(-10)));
}

#[test]
fn page_motion_keeps_the_column() {
    let mut e = Editor::new(&numbered(30));
    e.carets.set_caret(&e.document, 3);
    let ctx = ActionContext::plain_text().with_viewport(0, 6);
    e.execute(&A::PageDown, &ctx);
    let offset = e.carets.primary().offset;
    assert_eq!(column_of(&e.document, offset, 4), 3);
}

#[test]
fn ctrl_page_moves_to_the_first_and_last_visible_line() {
    // spec 1.4.
    let mut e = Editor::new(&numbered(50));
    e.carets.set_caret(&e.document, 3);
    let ctx = ActionContext::plain_text().with_viewport(20, 10);
    e.execute(&A::MoveToPageBottom, &ctx);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 29);
    e.execute(&A::MoveToPageTop, &ctx);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 20);
}

#[test]
fn ctrl_up_scrolls_and_drags_the_caret_only_when_it_would_leave_the_view() {
    // spec 1.4.
    let mut e = Editor::new(&numbered(50));
    let line_offset = |e: &Editor, l: usize| e.document.line_start_offset(l);
    e.carets.set_caret(&e.document, line_offset(&e, 25));
    let ctx = ActionContext::plain_text().with_viewport(20, 10);
    let out = e.execute(&A::ScrollDown, &ctx);
    assert_eq!(out.scroll, Some(ScrollRequest::Lines(1)));
    assert_eq!(e.document.line_number(e.carets.primary().offset), 25);
    e.carets.set_caret(&e.document, line_offset(&e, 20));
    e.execute(&A::ScrollDown, &ctx);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 22);
}

#[test]
fn escape_clears_selection_and_all_but_the_oldest_caret() {
    // spec 1.1 / 1.5.
    let mut e = editor("ab|c\nd‹e›f\ng|h");
    let out = run(&mut e, A::Escape);
    assert!(out.handled);
    assert_eq!(e.carets.caret_count(), 1);
    assert!(!e.carets.primary().has_selection());
    assert_eq!(show(&e), "ab|c\ndef\ngh");
    let out = run(&mut e, A::Escape);
    assert!(!out.handled, "nothing to do: a popup may take Escape");
}

#[test]
fn insert_key_toggles_overwrite_mode() {
    let mut e = editor("|abc");
    run(&mut e, A::ToggleInsertState);
    assert!(e.carets.overwrite);
    type_str(&mut e, "xy", &Language::plain_text());
    assert_eq!(show(&e), "xy|c");
}

#[test]
fn motion_skips_collapsed_folds() {
    // spec 1.1 / 6: Right across a fold jumps over its whole range.
    let mut e = editor("a|{bcd}e");
    e.folding.add_region(&e.document, 1, 6, "{...}", false);
    run(&mut e, A::Right);
    assert_eq!(show(&e), "a{bcd}|e");
    run(&mut e, A::Left);
    assert_eq!(show(&e), "a|{bcd}e");
}

// ---- multi-caret and selection actions (spec 1.5 to 1.7) ------------------------------

#[test]
fn select_next_occurrence_selects_the_word_then_adds_occurrences() {
    // spec 1.5.
    let mut e = editor("fo|o x foo y foo");
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(show(&e), "‹foo› x foo y foo");
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(show(&e), "‹foo› x ‹foo› y foo");
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(show(&e), "‹foo› x ‹foo› y ‹foo›");
    assert_eq!(e.carets.caret_count(), 3);
}

#[test]
fn select_next_occurrence_from_a_bare_word_matches_whole_words_only() {
    // spec 1.5: whole-word search when it started from a bare word.
    let mut e = editor("fo|o foobar foo");
    run(&mut e, A::SelectNextOccurrence);
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(show(&e), "‹foo› foobar ‹foo›");
}

#[test]
fn select_next_occurrence_with_a_selection_is_case_sensitive_substring() {
    let mut e = editor("‹ab› AB abab");
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(show(&e), "‹ab› AB ‹ab›ab");
}

#[test]
fn select_next_occurrence_hints_then_wraps_from_the_top() {
    // spec 1.5: "Not found, try again to search from top".
    let mut e = editor("foo x fo|o");
    run(&mut e, A::SelectNextOccurrence);
    let out = run(&mut e, A::SelectNextOccurrence);
    assert_eq!(
        out.hint.as_deref(),
        Some("Not found, try again to search from top")
    );
    assert_eq!(e.carets.caret_count(), 1);
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(show(&e), "‹foo› x ‹foo›");
}

#[test]
fn unselect_previous_occurrence_removes_the_newest_caret_then_the_selection() {
    // spec 1.5.
    let mut e = editor("fo|o foo");
    run(&mut e, A::SelectNextOccurrence);
    run(&mut e, A::SelectNextOccurrence);
    assert_eq!(e.carets.caret_count(), 2);
    run(&mut e, A::UnselectPreviousOccurrence);
    assert_eq!(show(&e), "‹foo› foo");
    run(&mut e, A::UnselectPreviousOccurrence);
    assert!(!e.carets.primary().has_selection());
}

#[test]
fn select_all_occurrences_selects_every_match() {
    // spec 1.5.
    let mut e = editor("a‹b›c abc b");
    run(&mut e, A::SelectAllOccurrences);
    assert_eq!(show(&e), "a‹b›c a‹b›c ‹b›");
    let mut e = editor("fo|o bar foo");
    run(&mut e, A::SelectAllOccurrences);
    assert_eq!(show(&e), "‹foo› bar ‹foo›");
}

#[test]
fn clone_caret_below_and_above_keep_the_column() {
    // spec 1.5.
    let mut e = editor("abc|d\nefgh\nij");
    run(&mut e, A::CloneCaretBelow);
    assert_eq!(show(&e), "abc|d\nefg|h\nij");
    run(&mut e, A::CloneCaretBelow);
    assert_eq!(show(&e), "abc|d\nefg|h\nij|");
    let mut e = editor("abcd\nef|gh");
    run(&mut e, A::CloneCaretAbove);
    assert_eq!(show(&e), "ab|cd\nef|gh");
}

#[test]
fn clone_caret_with_selection_skips_lines_that_are_too_short() {
    // spec 1.5.
    let mut e = editor("a‹bcd›e\nx\nabcde");
    run(&mut e, A::CloneCaretBelow);
    assert_eq!(show(&e), "a‹bcd›e\nx\na‹bcd›e");
}

#[test]
fn clone_caret_at_the_document_edge_does_nothing() {
    let mut e = editor("a|b");
    let out = run(&mut e, A::CloneCaretAbove);
    assert!(!out.handled);
    assert_eq!(e.carets.caret_count(), 1);
}

#[test]
fn add_caret_per_selected_line_puts_carets_at_line_ends() {
    // spec 1.5.
    assert_eq!(
        after("‹ab\ncde\nf›g", A::AddCaretPerSelectedLine),
        "ab|\ncde|\nfg|"
    );
}

#[test]
fn add_caret_per_selected_line_excludes_a_last_line_selected_at_column_zero() {
    // spec 1.5.
    assert_eq!(
        after("‹ab\ncd\n›ef", A::AddCaretPerSelectedLine),
        "ab|\ncd|\nef"
    );
}

#[test]
fn add_and_remove_caret_by_click() {
    // spec 1.5: Alt+Shift+click adds, clicking an existing caret removes.
    let mut e = editor("abc\ndef");
    assert!(e.carets.add_caret(&e.document, 5));
    assert_eq!(show(&e), "|abc\nd|ef");
    assert!(!e.carets.add_caret(&e.document, 5));
    assert!(e.carets.remove_caret_at(5));
    assert_eq!(e.carets.caret_count(), 1);
    assert!(!e.carets.remove_caret_at(0), "the last caret stays");
}

#[test]
fn the_primary_caret_is_the_most_recently_added() {
    let mut e = editor("abc");
    e.carets.add_caret(&e.document, 2);
    assert_eq!(e.carets.primary().offset, 2);
    e.carets.add_caret(&e.document, 1);
    assert_eq!(e.carets.primary().offset, 1);
}

#[test]
fn overlapping_selections_merge() {
    let mut e = editor("abcdef");
    e.carets.set_selection(&e.document, 0, 3);
    e.carets.add_caret_with_selection(&e.document, 2, 5);
    assert_eq!(e.carets.caret_count(), 1);
    assert_eq!(show(&e), "‹abcde›f");
}

#[test]
fn max_carets_is_enforced() {
    let mut e = Editor::new(&"a\n".repeat(2000));
    e.carets.max_carets = 1000;
    for line in 0..1500 {
        e.carets
            .add_caret(&e.document, e.document.line_start_offset(line));
    }
    assert_eq!(e.carets.caret_count(), 1000);
}

#[test]
fn run_for_each_caret_visits_every_caret_and_merges() {
    let mut e = editor("a|b|c");
    e.carets.run_for_each_caret(|caret| caret.move_to(0));
    assert_eq!(e.carets.caret_count(), 1);
}

#[test]
fn extend_selection_grows_word_line_text() {
    // spec 1.7: word, then line, then the whole text.
    let mut e = editor("one\ntw|o three\nfour");
    run(&mut e, A::SelectWord);
    assert_eq!(show(&e), "one\n‹two› three\nfour");
    let mut grown = vec![show(&e)];
    for _ in 0..4 {
        run(&mut e, A::SelectWord);
        grown.push(show(&e));
    }
    assert!(grown.contains(&"one\n‹two three›\nfour".to_owned()));
    assert_eq!(grown.last().unwrap(), "‹one\ntwo three\nfour›");
}

#[test]
fn extend_selection_picks_up_quotes_and_brackets() {
    // spec 1.7: pairs enter the chain as candidates (smallest strictly containing wins).
    let mut e = editor("f(a, b|c)");
    run(&mut e, A::SelectWord);
    assert_eq!(show(&e), "f(a, ‹bc›)");
    let mut seen = Vec::new();
    for _ in 0..6 {
        run(&mut e, A::SelectWord);
        seen.push(show(&e));
    }
    assert!(seen.contains(&"f‹(a, bc)›".to_owned()), "{seen:?}");
    let mut e = editor("say \"hel|lo world\" now");
    let mut seen = Vec::new();
    for _ in 0..8 {
        run(&mut e, A::SelectWord);
        seen.push(show(&e));
    }
    assert!(
        seen.contains(&"say ‹\"hello world\"› now".to_owned()),
        "{seen:?}"
    );
}

#[test]
fn extend_selection_treats_apostrophe_and_hyphen_as_word_chars() {
    // spec 1.7: `don't` and `well-known` select whole.
    let mut e = editor("it don|'t work");
    run(&mut e, A::SelectWord);
    run(&mut e, A::SelectWord);
    assert!(show(&e).contains("‹don't›"), "{}", show(&e));
    let mut e = editor("a well-kn|own b");
    run(&mut e, A::SelectWord);
    run(&mut e, A::SelectWord);
    assert!(show(&e).contains("‹well-known›"), "{}", show(&e));
}

#[test]
fn extend_selection_reaches_sentence_and_paragraph() {
    // spec 1.7.
    let mut e = editor("First one. Sec|ond two. Third.\n\nOther para.");
    let mut seen = Vec::new();
    for _ in 0..8 {
        run(&mut e, A::SelectWord);
        seen.push(show(&e));
    }
    assert!(
        seen.contains(&"First one. ‹Second two.› Third.\n\nOther para.".to_owned()),
        "{seen:?}"
    );
    assert!(
        seen.contains(&"‹First one. Second two. Third.›\n\nOther para.".to_owned()),
        "{seen:?}"
    );
}

#[test]
fn shrink_selection_walks_the_extend_chain_back() {
    // spec 1.7: shrink steps back down the previous chain.
    let mut e = editor("one\ntw|o three\nfour");
    let mut chain = Vec::new();
    for _ in 0..5 {
        run(&mut e, A::SelectWord);
        chain.push(show(&e));
    }
    chain.dedup();
    for expected in chain.iter().rev().skip(1) {
        run(&mut e, A::UnSelectWord);
        assert_eq!(&show(&e), expected);
    }
}

#[test]
fn shrink_selection_without_a_chain_picks_the_largest_inner_range() {
    let mut e = editor("‹ab cd›");
    run(&mut e, A::UnSelectWord);
    assert!(e.carets.primary().has_selection());
    assert!(e.carets.primary().selection_end() - e.carets.primary().selection_start() < 5);
}

#[test]
fn extend_selection_works_for_every_caret() {
    let mut e = editor("a|b cd e|f gh");
    run(&mut e, A::SelectWord);
    assert_eq!(show(&e), "‹ab› cd ‹ef› gh");
}

#[test]
fn select_all_selects_the_whole_text_with_the_caret_at_the_end() {
    let mut e = editor("a|b\ncd");
    run(&mut e, A::SelectAll);
    assert_eq!(show(&e), "‹ab\ncd›");
}

// ---- column selection (spec 1.6 / 3.11) -------------------------------------------------

#[test]
fn alt_drag_builds_one_caret_per_line() {
    // spec 1.6.
    let mut e = Editor::new("abcdef\nabcdef\nabcdef");
    e.carets
        .start_block_selection(&e.document, 4, LogicalPosition::new(0, 1));
    e.carets
        .extend_block_selection(&e.document, 4, LogicalPosition::new(2, 3));
    assert_eq!(show(&e), "a‹bc›def\na‹bc›def\na‹bc›def");
    assert!(e.carets.column_mode);
    assert_eq!(
        e.carets.primary().offset,
        17,
        "the lead line holds the primary caret"
    );
}

#[test]
fn block_selection_short_lines_get_a_caret_at_their_end() {
    // spec 1.6: the rectangle stays continuous, short lines hold an empty selection.
    let mut e = Editor::new("abcdef\nab\nabcdef");
    e.carets
        .start_block_selection(&e.document, 4, LogicalPosition::new(0, 3));
    e.carets
        .extend_block_selection(&e.document, 4, LogicalPosition::new(2, 5));
    assert_eq!(show(&e), "abc‹de›f\nab|\nabc‹de›f");
    let short = &e.carets.carets()[1];
    assert_eq!(
        short.virtual_spaces, 3,
        "columns beyond the line end are virtual"
    );
    assert_eq!(
        e.carets.block.unwrap().columns(),
        (3, 5),
        "the rectangle itself keeps its columns"
    );
}

#[test]
fn block_selection_backwards_keeps_the_lead_at_the_left() {
    let mut e = Editor::new("abcdef\nabcdef");
    e.carets
        .start_block_selection(&e.document, 4, LogicalPosition::new(0, 4));
    e.carets
        .extend_block_selection(&e.document, 4, LogicalPosition::new(1, 2));
    assert_eq!(show(&e), "ab«cd»ef\nab«cd»ef");
}

#[test]
fn typing_in_column_mode_inserts_on_every_line() {
    // spec 3.11.
    let mut e = Editor::new("abc\nabc\nabc");
    e.carets
        .start_block_selection(&e.document, 4, LogicalPosition::new(0, 1));
    e.carets
        .extend_block_selection(&e.document, 4, LogicalPosition::new(2, 1));
    type_str(&mut e, "XY", &Language::plain_text());
    assert_eq!(show(&e), "aXY|bc\naXY|bc\naXY|bc");
}

#[test]
fn typing_beyond_a_short_line_pads_it_with_spaces() {
    // spec 3.11: padding on first insert.
    let mut e = Editor::new("abcdef\na\nabcdef");
    e.carets
        .start_block_selection(&e.document, 4, LogicalPosition::new(0, 4));
    e.carets
        .extend_block_selection(&e.document, 4, LogicalPosition::new(2, 4));
    type_str(&mut e, "Z", &Language::plain_text());
    assert_eq!(e.document.text(), "abcdZef\na   Z\nabcdZef");
}

#[test]
fn shift_arrows_grow_the_rectangle_in_column_mode() {
    // spec 1.6.
    let mut e = Editor::new("abcd\nabcd\nabcd");
    e.carets.set_caret(&e.document, 1);
    run(&mut e, A::ToggleColumnMode);
    assert!(e.carets.column_mode);
    run(&mut e, A::DownWithSelection);
    run(&mut e, A::RightWithSelection);
    run(&mut e, A::RightWithSelection);
    assert_eq!(show(&e), "a‹bc›d\na‹bc›d\nabcd");
    run(&mut e, A::Left);
    assert_eq!(
        e.carets.caret_count(),
        1,
        "a plain move collapses the block"
    );
}

#[test]
fn toggling_column_mode_off_turns_the_block_into_a_stream_selection() {
    let mut e = Editor::new("abcd\nabcd");
    e.carets
        .start_block_selection(&e.document, 4, LogicalPosition::new(0, 1));
    e.carets
        .extend_block_selection(&e.document, 4, LogicalPosition::new(1, 2));
    run(&mut e, A::ToggleColumnMode);
    assert!(!e.carets.column_mode);
    assert_eq!(e.carets.caret_count(), 1);
    assert_eq!(show(&e), "a‹bcd\nab›cd");
}

#[test]
fn pasting_n_lines_into_one_caret_in_column_mode_creates_n_carets() {
    // spec 1.6.
    let mut e = Editor::new("ab\ncd\nef");
    e.carets.set_caret(&e.document, 1);
    e.carets.column_mode = true;
    let clip = ClipboardContent::text("1\n2\n3");
    let lang = Language::plain_text();
    e.execute(&A::Paste, &ActionContext::new(&lang).with_clipboard(&clip));
    assert_eq!(e.document.text(), "a1b\nc2d\ne3f");
    assert_eq!(e.carets.caret_count(), 3);
}

// ---- typing (spec 3.1 to 3.4) -----------------------------------------------------------

#[test]
fn typing_replaces_the_selection() {
    let mut e = editor("a‹bc›d");
    type_str(&mut e, "x", &Language::plain_text());
    assert_eq!(show(&e), "ax|d");
}

#[test]
fn typing_inserts_at_every_caret() {
    let mut e = editor("a|b\nc|d\n|");
    type_str(&mut e, "xy", &Language::plain_text());
    assert_eq!(show(&e), "axy|b\ncxy|d\nxy|");
}

#[test]
fn typing_handles_multi_byte_characters() {
    let mut e = editor("é|😀");
    type_str(&mut e, "ñ日", &Language::plain_text());
    assert_eq!(show(&e), "éñ日|😀");
}

#[test]
fn typing_an_opener_in_plain_text_does_not_pair() {
    // spec 3.2: without a language no bracket auto-pairing.
    let mut e = editor("|");
    type_str(&mut e, "(", &Language::plain_text());
    assert_eq!(show(&e), "(|");
}

#[test]
fn typing_an_opener_inserts_the_closer() {
    // spec 3.2.
    let c = Language::c_like();
    assert_eq!(after_lang("|", A::Type('('), &c), "(|)");
    assert_eq!(after_lang("|", A::Type('{'), &c), "{|}");
    assert_eq!(after_lang("|", A::Type('['), &c), "[|]");
}

#[test]
fn typing_an_opener_before_a_word_does_not_pair() {
    // spec 3.2: pairing only where it is allowed.
    let c = Language::c_like();
    assert_eq!(after_lang("|foo", A::Type('('), &c), "(|foo");
    assert_eq!(after_lang("|)", A::Type('('), &c), "(|)");
}

#[test]
fn typing_an_opener_with_an_unmatched_closer_nearby_does_not_pair() {
    // spec 3.2: only when the new opener would otherwise be unmatched.
    let c = Language::c_like();
    assert_eq!(after_lang("|) x )", A::Type('('), &c), "(|) x )");
}

#[test]
fn typing_a_closer_overtypes_its_partner() {
    // spec 3.2.
    let c = Language::c_like();
    assert_eq!(after_lang("(|)", A::Type(')'), &c), "()|");
    assert_eq!(after_lang("f(a|)", A::Type(')'), &c), "f(a)|");
}

#[test]
fn typing_a_closer_without_a_partner_inserts_it() {
    // spec 3.2.
    let c = Language::c_like();
    assert_eq!(after_lang("|)", A::Type(')'), &c), ")|)");
}

#[test]
fn typing_a_quote_inserts_a_pair_and_overtypes_the_closing_one() {
    // spec 3.3.
    let c = Language::c_like();
    assert_eq!(after_lang("|", A::Type('"'), &c), "\"|\"");
    assert_eq!(after_lang("\"ab|\"", A::Type('"'), &c), "\"ab\"|");
}

#[test]
fn typing_a_quote_after_a_letter_stays_single() {
    // spec 3.3: `don't`.
    let c = Language::c_like();
    assert_eq!(after_lang("don|", A::Type('\''), &c), "don'|");
}

#[test]
fn typing_a_quote_before_an_identifier_or_after_a_backslash_does_not_pair() {
    // spec 3.3.
    let c = Language::c_like();
    assert_eq!(after_lang("|abc", A::Type('"'), &c), "\"|abc");
    assert_eq!(after_lang("\"\\|\"", A::Type('"'), &c), "\"\\\"|\"");
}

#[test]
fn auto_pairing_can_be_switched_off() {
    // spec 3.2 / 3.3.
    let c = Language::c_like();
    let mut e = editor("|");
    e.settings.auto_pair_brackets = false;
    e.settings.auto_pair_quotes = false;
    type_str(&mut e, "(\"", &c);
    assert_eq!(show(&e), "(\"|");
}

#[test]
fn typing_a_bracket_over_a_selection_wraps_it() {
    // spec 3.3: the text stays selected.
    let c = Language::c_like();
    assert_eq!(after_lang("a‹bc›d", A::Type('('), &c), "a(‹bc›)d");
    assert_eq!(after_lang("a‹bc›d", A::Type('"'), &c), "a\"‹bc›\"d");
    assert_eq!(after_lang("a«bc»d", A::Type('['), &c), "a[«bc»]d");
}

#[test]
fn typing_a_quote_over_a_wrapped_selection_replaces_the_pair() {
    // spec 3.3: `"x"` + `'` -> `'x'`.
    let c = Language::c_like();
    assert_eq!(after_lang("‹\"x\"›", A::Type('\''), &c), "'‹x›'");
    assert_eq!(after_lang("‹(x)›", A::Type('['), &c), "[‹x›]");
}

#[test]
fn typing_less_than_over_a_comparison_selection_replaces_it() {
    // spec 3.3: not wrapped when the selection is only comparison operators.
    let p = Language::plain_text();
    assert_eq!(after_lang("a‹<=›b", A::Type('<'), &p), "a<|b");
    assert_eq!(after_lang("a‹x›b", A::Type('<'), &p), "a<‹x›>b");
}

#[test]
fn surrounding_can_be_switched_off() {
    let c = Language::c_like();
    let mut e = editor("a‹bc›d");
    e.settings.surround_selection_on_quote_typed = false;
    e.execute(&A::Type('('), &ActionContext::new(&c));
    assert_eq!(show(&e), "a(|d");
}

#[test]
fn wrapping_works_for_every_caret() {
    let c = Language::c_like();
    assert_eq!(after_lang("‹a› ‹b›", A::Type('('), &c), "(‹a›) (‹b›)");
}

#[test]
fn tab_jumps_over_an_auto_inserted_closer() {
    // spec 3.4.
    let c = Language::c_like();
    let mut e = editor("|");
    type_str(&mut e, "(a", &c);
    assert_eq!(show(&e), "(a|)");
    e.execute(&A::Tab, &ActionContext::new(&c));
    assert_eq!(show(&e), "(a)|");
}

#[test]
fn tab_does_not_jump_when_the_caret_left_the_scope_or_the_setting_is_off() {
    let c = Language::c_like();
    let mut e = editor("|");
    type_str(&mut e, "(a", &c);
    e.settings.tab_exits_brackets_and_quotes = false;
    e.execute(&A::Tab, &ActionContext::new(&c));
    assert_eq!(show(&e), "(a  |)");
}

// ---- Enter (spec 3.5) -------------------------------------------------------------------

#[test]
fn enter_copies_the_indent_up_to_the_caret() {
    // spec 3.5.
    assert_eq!(after("  ab|cd", A::Enter), "  ab\n  |cd");
    assert_eq!(after("    |  x", A::Enter), "    \n    |  x");
    assert_eq!(after("  | ab", A::Enter), "  \n  | ab");
    assert_eq!(after("|ab", A::Enter), "\n|ab");
}

#[test]
fn enter_replaces_the_selection() {
    assert_eq!(after("a‹bc›d", A::Enter), "a\n|d");
}

#[test]
fn enter_works_for_every_caret() {
    assert_eq!(after("  a|b\nc|d", A::Enter), "  a\n  |b\nc\n|d");
}

#[test]
fn enter_between_braces_makes_three_lines() {
    // spec 3.5 (language layer).
    let c = Language::c_like();
    assert_eq!(after_lang("{|}", A::Enter, &c), "{\n    |\n}");
    assert_eq!(after_lang("  f(|)", A::Enter, &c), "  f(\n      |\n  )");
}

#[test]
fn enter_after_an_unmatched_curly_inserts_the_closer() {
    // spec 3.5: "insert pair curly brace on Enter".
    let c = Language::c_like();
    assert_eq!(after_lang("fn x() {|", A::Enter, &c), "fn x() {\n    |\n}");
    assert_eq!(after_lang("{ {|}", A::Enter, &c), "{ {\n    |\n}");
}

#[test]
fn enter_after_an_opener_indents_one_unit() {
    let c = Language::c_like();
    assert_eq!(after_lang("f(|x", A::Enter, &c), "f(\n    |x");
}

#[test]
fn enter_in_plain_text_after_a_brace_just_copies_the_indent() {
    assert_eq!(after("  {|", A::Enter), "  {\n  |");
}

#[test]
fn enter_without_smart_indent_copies_nothing_special() {
    let c = Language::c_like();
    let mut e = editor("{|}");
    e.settings.smart_indent_on_enter = false;
    e.execute(&A::Enter, &ActionContext::new(&c));
    assert_eq!(show(&e), "{\n|}");
}

#[test]
fn enter_in_overwrite_mode_moves_to_the_next_line_start() {
    // spec 1.1.
    let mut e = editor("a|b\ncd");
    e.carets.overwrite = true;
    run(&mut e, A::Enter);
    assert_eq!(show(&e), "ab\n|cd");
    let mut e = editor("ab|");
    e.carets.overwrite = true;
    run(&mut e, A::Enter);
    assert_eq!(show(&e), "ab\n|");
}

#[test]
fn start_new_line_opens_a_line_below_from_anywhere() {
    // spec 3.5.
    assert_eq!(after("a|b\nc", A::StartNewLine), "ab\n|\nc");
    assert_eq!(after("  a|b", A::StartNewLine), "  ab\n  |");
    assert_eq!(after("‹ab›", A::StartNewLine), "ab\n|");
}

#[test]
fn start_new_line_works_for_every_caret_and_dedupes_a_shared_line() {
    assert_eq!(after("a|b\nc|d", A::StartNewLine), "ab\n|\ncd\n|");
    let mut e = editor("a|b|c");
    run(&mut e, A::StartNewLine);
    assert_eq!(e.document.text(), "abc\n");
}

#[test]
fn start_new_line_before_opens_an_indented_line_above() {
    // spec 3.5.
    assert_eq!(after("  a|b", A::StartNewLineBefore), "  |\n  ab");
    assert_eq!(after("a|b", A::StartNewLineBefore), "|\nab");
    assert_eq!(after("x\na|b", A::StartNewLineBefore), "x\n|\nab");
}

#[test]
fn smart_enter_without_a_language_processor_is_start_new_line() {
    // spec 3.5.
    assert_eq!(
        after("a|b", A::CompleteStatement),
        after("a|b", A::StartNewLine)
    );
}

#[test]
fn split_line_keeps_the_caret_before_the_break() {
    // spec 3.5.
    assert_eq!(after("ab|cd", A::SplitLine), "ab|\ncd");
    assert_eq!(after("  ab|cd", A::SplitLine), "  ab|\n  cd");
}

#[test]
fn split_line_before_the_text_inserts_an_indented_empty_line() {
    // spec 3.5.
    assert_eq!(after("  |ab", A::SplitLine), "  \n  |ab");
}

// ---- Tab, indent, unindent (spec 3.6) ---------------------------------------------------

#[test]
fn tab_inserts_spaces_to_the_next_indent_multiple() {
    // spec 3.6: the indent size, from the current column.
    assert_eq!(after("|", A::Tab), "    |");
    assert_eq!(after("a|", A::Tab), "a   |");
    assert_eq!(after("abcd|", A::Tab), "abcd    |");
}

#[test]
fn tab_uses_the_indent_size_not_the_tab_size() {
    let mut e = editor("|");
    e.settings.indent_size = 2;
    e.settings.tab_size = 8;
    run(&mut e, A::Tab);
    assert_eq!(show(&e), "  |");
}

#[test]
fn tab_inserts_a_tab_character_when_configured() {
    let mut e = editor("|");
    e.settings.use_tab_character = true;
    run(&mut e, A::Tab);
    assert_eq!(show(&e), "\t|");
    let mut e = editor("a|");
    e.settings.use_tab_character = true;
    e.settings.smart_tabs = true;
    run(&mut e, A::Tab);
    assert_eq!(show(&e), "a   |", "smart tabs: spaces after text");
}

#[test]
fn tab_with_a_selection_indents_every_touched_line() {
    // spec 3.6.
    assert_eq!(after("a‹b\nc›d\nef", A::Tab), "    a‹b\n    c›d\nef");
}

#[test]
fn tab_with_a_one_line_selection_indents_the_line() {
    // spec 3.6: even within one line.
    assert_eq!(after("a‹b›c", A::Tab), "    a‹b›c");
}

#[test]
fn tab_excludes_a_last_line_the_selection_touches_at_column_zero() {
    // spec 3.6: and the selection is extended to cover the new indents.
    assert_eq!(after("‹a\nb\n›c", A::Tab), "‹    a\n    b\n›c");
}

#[test]
fn tab_with_a_whitespace_only_selection_replaces_it() {
    assert_eq!(after("a‹  ›b", A::Tab), "a   |b");
}

#[test]
fn tab_skips_empty_lines_when_indenting_a_block() {
    assert_eq!(after("‹a\n\nb›", A::Tab), "‹    a\n\n    b›");
}

#[test]
fn tab_indents_for_every_caret() {
    assert_eq!(after("a|b\nc|d", A::Tab), "a   |b\nc   |d");
    assert_eq!(after("‹a›\n‹b›", A::Tab), "‹    a›\n‹    b›");
}

#[test]
fn shift_tab_unindents_the_caret_line_and_selected_lines() {
    // spec 3.6: always enabled, selection or not.
    assert_eq!(after("      a|b", A::UnindentSelection), "  a|b");
    assert_eq!(after("‹    a\n  b›", A::UnindentSelection), "‹a\nb›");
    assert_eq!(after("a|b", A::UnindentSelection), "a|b");
}

#[test]
fn shift_tab_removes_a_leading_tab_character() {
    assert_eq!(after("\t\ta|b", A::UnindentSelection), "\ta|b");
}

#[test]
fn shift_tab_unindents_for_every_caret() {
    assert_eq!(after("  a|\n  b|", A::UnindentSelection), "a|\nb|");
}

// ---- Backspace and Delete (spec 3.6, 3.10) -----------------------------------------------

#[test]
fn backspace_removes_one_code_point() {
    // spec 3.6: surrogate pairs as one.
    assert_eq!(after("a😀|", A::Backspace), "a|");
    assert_eq!(after("aé|", A::Backspace), "a|");
}

#[test]
fn backspace_at_column_zero_joins_with_the_previous_line() {
    assert_eq!(after("ab\n|cd", A::Backspace), "ab|cd");
    assert_eq!(after("|ab", A::Backspace), "|ab");
}

#[test]
fn backspace_removes_the_selection() {
    assert_eq!(after("a‹bc›d", A::Backspace), "a|d");
}

#[test]
fn backspace_works_for_every_caret() {
    assert_eq!(after("ab|\ncd|", A::Backspace), "a|\nc|");
}

#[test]
fn backspace_in_leading_whitespace_deletes_one_char_without_a_formatter() {
    // spec 3.6: AutoIndent falls back to one char.
    assert_eq!(after("      |a", A::Backspace), "     |a");
}

#[test]
fn backspace_in_indent_mode_removes_back_to_the_previous_multiple() {
    // spec 3.6: the "indent" alternative.
    let mut e = editor("      |a");
    e.settings.smart_backspace = SmartBackspace::Indent;
    run(&mut e, A::Backspace);
    assert_eq!(show(&e), "    |a");
    run(&mut e, A::Backspace);
    assert_eq!(show(&e), "|a");
}

#[test]
fn backspace_removes_an_empty_pair() {
    // spec 3.2 / 3.3.
    let c = Language::c_like();
    assert_eq!(after_lang("(|)", A::Backspace, &c), "|");
    assert_eq!(after_lang("\"|\"", A::Backspace, &c), "|");
    assert_eq!(after_lang("(a|)", A::Backspace, &c), "(|)");
    assert_eq!(after("(|)", A::Backspace), "|)", "plain text has no pairs");
}

#[test]
fn backspace_pair_removal_works_for_every_caret() {
    let c = Language::c_like();
    assert_eq!(after_lang("(|) [|]", A::Backspace, &c), "| |");
}

#[test]
fn delete_removes_the_next_code_point_and_joins_at_the_line_end() {
    // spec 3.10.
    assert_eq!(after("a|😀b", A::Delete), "a|b");
    assert_eq!(after("ab|\ncd", A::Delete), "ab|cd");
    assert_eq!(after("ab|", A::Delete), "ab|");
    assert_eq!(after("a‹bc›d", A::Delete), "a|d");
    assert_eq!(after("a|b\nc|d", A::Delete), "a|\nc|");
}

// ---- line operations (spec 3.7) -----------------------------------------------------------

#[test]
fn duplicate_without_selection_duplicates_the_line_below() {
    // spec 3.7: the caret moves to the same offset in the copy.
    assert_eq!(after("a|b\ncd", A::Duplicate), "ab\na|b\ncd");
}

#[test]
fn duplicate_adds_a_break_after_a_last_line_without_one() {
    assert_eq!(after("a|b", A::Duplicate), "ab\na|b");
}

#[test]
fn duplicate_with_a_selection_duplicates_the_text_and_selects_the_copy() {
    // spec 3.7.
    assert_eq!(after("a‹bc›d", A::Duplicate), "abc‹bc›d");
}

#[test]
fn duplicate_covers_every_line_of_a_multi_line_caret_group() {
    assert_eq!(after("a|b\nc|d\nef", A::Duplicate), "ab\na|b\ncd\nc|d\nef");
}

#[test]
fn duplicate_with_two_carets_on_one_line_duplicates_it_once() {
    let mut e = editor("a|b|c");
    run(&mut e, A::Duplicate);
    assert_eq!(e.document.text(), "abc\nabc");
}

#[test]
fn delete_line_removes_the_line_with_its_break() {
    // spec 3.7.
    assert_eq!(after("a\nb|c\nd", A::DeleteLine), "a\nd|");
}

#[test]
fn delete_line_keeps_the_visual_column() {
    // spec 3.7: caret on the line that moved up at the same column.
    assert_eq!(after("a\nb|c\ndefgh", A::DeleteLine), "a\nd|efgh");
    assert_eq!(after("a\nbcd|\nef", A::DeleteLine), "a\nef|");
}

#[test]
fn delete_line_on_the_last_line_removes_the_previous_break() {
    // spec 3.7.
    assert_eq!(after("a\nb|c", A::DeleteLine), "a|");
    assert_eq!(after("a|bc", A::DeleteLine), "|");
}

#[test]
fn delete_line_with_a_selection_excludes_a_last_line_touched_at_column_zero() {
    // spec 3.7.
    assert_eq!(after("‹a\nb\n›c", A::DeleteLine), "|c");
    assert_eq!(after("x\n‹a\nb›\nc", A::DeleteLine), "x\nc|");
}

#[test]
fn delete_line_works_for_every_caret() {
    assert_eq!(after("a|\nb\nc|\nd", A::DeleteLine), "b|\nd|");
}

#[test]
fn move_line_down_swaps_with_the_next_line() {
    // spec 3.7: caret column and selection are kept.
    assert_eq!(after("a|b\ncd\nef", A::MoveLineDown), "cd\na|b\nef");
    assert_eq!(after("‹ab\ncd›\nef", A::MoveLineDown), "ef\n‹ab\ncd›");
}

#[test]
fn move_line_up_swaps_with_the_previous_line() {
    assert_eq!(after("ab\nc|d\nef", A::MoveLineUp), "c|d\nab\nef");
    assert_eq!(after("ab\n‹cd\nef›", A::MoveLineUp), "‹cd\nef›\nab");
}

#[test]
fn move_line_does_nothing_at_the_document_edges() {
    // spec 3.7: disabled at start/end.
    assert_eq!(after("a|b\ncd", A::MoveLineUp), "a|b\ncd");
    assert_eq!(after("ab\nc|d", A::MoveLineDown), "ab\nc|d");
}

#[test]
fn move_line_works_when_the_last_line_has_no_break() {
    assert_eq!(after("ab\nc|d", A::MoveLineUp), "c|d\nab");
    assert_eq!(after("a|b\ncd", A::MoveLineDown), "cd\na|b");
}

#[test]
fn move_line_excludes_a_last_line_the_selection_touches_at_column_zero() {
    assert_eq!(after("‹a\n›b\nc", A::MoveLineDown), "b\n‹a\n›c");
}

#[test]
fn move_line_never_reindents() {
    assert_eq!(after("  a|\nb", A::MoveLineDown), "b\n  a|");
}

#[test]
fn move_line_moves_every_caret_line_and_keeps_adjacent_blocks_together() {
    assert_eq!(after("a|\nb|\nc\nd", A::MoveLineDown), "c\na|\nb|\nd");
    assert_eq!(after("a\nb|\nc|\nd", A::MoveLineUp), "b|\nc|\na\nd");
}

#[test]
fn move_line_is_blocked_for_a_group_next_to_a_blocked_one() {
    assert_eq!(after("a|\nb|\nc", A::MoveLineUp), "a|\nb|\nc");
}

#[test]
fn move_statement_falls_back_to_the_line_mover() {
    // spec 3.7.
    assert_eq!(
        after("a|b\ncd", A::MoveStatementDown),
        after("a|b\ncd", A::MoveLineDown)
    );
}

#[test]
fn join_lines_makes_the_break_and_leading_blanks_one_space() {
    // spec 3.7: the caret goes right after the inserted space.
    assert_eq!(after("a|\n   b\nc", A::JoinLines), "a |b\nc");
    assert_eq!(after("ab\n|", A::JoinLines), "ab\n|");
}

#[test]
fn join_lines_does_nothing_on_the_last_line() {
    assert_eq!(after("a\nb|", A::JoinLines), "a\nb|");
}

#[test]
fn join_lines_with_a_selection_joins_all_selected_lines() {
    // spec 3.7: N-1 joins, caret to the selection end.
    assert_eq!(after("‹a\n b\n c›\nd", A::JoinLines), "a b c|\nd");
}

#[test]
fn join_lines_works_for_every_caret() {
    assert_eq!(after("a|\nb\nc|\nd", A::JoinLines), "a |b\nc |d");
}

#[test]
fn join_lines_does_not_add_a_space_before_an_empty_line() {
    assert_eq!(after("a|\n\nb", A::JoinLines), "a|\nb");
}

#[test]
fn toggle_case_uses_the_word_at_the_caret() {
    // spec 3.7: not all lowercase -> lowercase; lowercase -> uppercase.
    assert_eq!(after("Fo|o bar", A::ToggleCase), "‹foo› bar");
    assert_eq!(after("fo|o bar", A::ToggleCase), "‹FOO› bar");
}

#[test]
fn toggle_case_decides_across_all_carets() {
    assert_eq!(after("‹aB› ‹cd›", A::ToggleCase), "‹ab› ‹cd›");
    assert_eq!(after("‹ab› ‹cd›", A::ToggleCase), "‹AB› ‹CD›");
}

#[test]
fn toggle_case_handles_unicode_and_length_changes() {
    assert_eq!(after("‹straße›", A::ToggleCase), "‹STRASSE›");
    assert_eq!(after("‹ÉCOLE›", A::ToggleCase), "‹école›");
}

#[test]
fn toggle_case_leaves_escape_sequences_alone() {
    assert_eq!(after("‹a\\nb›", A::ToggleCase), "‹A\\nB›");
}

#[test]
fn toggle_case_without_a_word_does_nothing() {
    let mut e = editor("| ");
    let out = run(&mut e, A::ToggleCase);
    assert!(!out.text_changed);
}

#[test]
fn toggle_case_with_camel_humps_changes_one_hump() {
    let mut e = editor("fooB|arBaz");
    e.settings.camel_humps = true;
    run(&mut e, A::ToggleCase);
    assert_eq!(show(&e), "foo‹bar›Baz");
}

fn rust() -> Language {
    Language::c_like()
}

#[test]
fn comment_line_prefixes_at_column_zero_and_moves_down() {
    // spec 3.7 and 0.
    assert_eq!(
        after_lang("  a|b\ncd", A::CommentByLineComment, &rust()),
        "//  ab\ncd|"
    );
}

#[test]
fn comment_line_uncomments_when_every_line_is_commented() {
    // spec 3.7.
    assert_eq!(
        after_lang("//a|b\ncd", A::CommentByLineComment, &rust()),
        "ab\nc|d"
    );
    assert_eq!(
        after_lang("‹//a\n  //b›", A::CommentByLineComment, &rust()).replace(['‹', '›'], ""),
        "a\n  b"
    );
}

#[test]
fn comment_line_comments_all_when_one_line_is_not_commented() {
    assert_eq!(
        after_lang("‹//a\nb›", A::CommentByLineComment, &rust()),
        "‹////a\n//b›"
    );
}

#[test]
fn comment_line_skips_blank_lines_when_several_are_selected() {
    // spec 3.7.
    assert_eq!(
        after_lang("‹a\n\nb›", A::CommentByLineComment, &rust()),
        "‹//a\n\n//b›"
    );
}

#[test]
fn comment_line_does_not_move_the_caret_with_a_selection() {
    let mut e = editor("‹ab›\ncd");
    e.execute(&A::CommentByLineComment, &ActionContext::new(&rust()));
    assert!(e.carets.primary().has_selection());
    assert_eq!(e.document.text(), "//ab\ncd");
}

#[test]
fn comment_line_can_go_at_the_indent() {
    let mut e = editor("  a|\n    b");
    e.settings.line_comment_at_first_column = false;
    e.settings.comment_line_caret_down = false;
    e.execute(&A::CommentByLineComment, &ActionContext::new(&rust()));
    assert_eq!(e.document.text(), "  //a\n    b");
}

#[test]
fn comment_line_uses_the_callers_prefix_and_space_style() {
    let hash = Language::plain_text().with_line_comment("#");
    let mut e = editor("a|\nb");
    e.settings.comment_line_caret_down = false;
    e.execute(&A::CommentByLineComment, &ActionContext::new(&hash));
    assert_eq!(e.document.text(), "#a\nb");
    let mut spaced = hash.clone();
    spaced.line_comment_space = true;
    e.execute(&A::CommentByLineComment, &ActionContext::new(&spaced));
    assert_eq!(e.document.text(), "a\nb");
}

#[test]
fn comment_line_without_a_commenter_is_not_handled() {
    let mut e = editor("a|");
    let out = run(&mut e, A::CommentByLineComment);
    assert!(!out.handled);
    assert_eq!(e.document.text(), "a");
}

#[test]
fn comment_line_works_for_every_caret() {
    let mut e = editor("a|\nb|\nc");
    e.settings.comment_line_caret_down = false;
    e.execute(&A::CommentByLineComment, &ActionContext::new(&rust()));
    assert_eq!(e.document.text(), "//a\n//b\nc");
}

#[test]
fn comment_by_block_wraps_and_unwraps_the_selection() {
    assert_eq!(
        after_lang("a‹bc›d", A::CommentByBlockComment, &rust()),
        "a/*‹bc›*/d"
    );
    assert_eq!(
        after_lang("a‹/*bc*/›d", A::CommentByBlockComment, &rust()),
        "a‹bc›d"
    );
}

// ---- clipboard (spec 3.8) -----------------------------------------------------------------

#[test]
fn copy_with_a_selection_joins_the_carets_with_newlines() {
    // spec 1.5 / 3.8.
    let mut e = editor("‹ab› cd ‹ef›");
    let out = run(&mut e, A::Copy);
    assert_eq!(out.clipboard, Some(ClipboardContent::text("ab\nef")));
    assert_eq!(e.document.text(), "ab cd ef");
}

#[test]
fn copy_without_a_selection_copies_the_whole_line_and_selects_it() {
    // spec 3.8: marked as copied from an empty selection.
    let mut e = editor("a\nb|c\nd");
    let out = run(&mut e, A::Copy);
    assert_eq!(out.clipboard, Some(ClipboardContent::lines("bc\n")));
    assert_eq!(show(&e), "a\n‹bc\n›d");
}

#[test]
fn copy_without_a_selection_can_restore_the_caret() {
    let mut e = editor("a\nb|c\nd");
    e.settings.skip_selecting_line_after_copy = true;
    run(&mut e, A::Copy);
    assert_eq!(show(&e), "a\nb|c\nd");
}

#[test]
fn copy_of_the_last_line_adds_a_break() {
    let mut e = editor("a\nb|");
    let out = run(&mut e, A::Copy);
    assert_eq!(out.clipboard, Some(ClipboardContent::lines("b\n")));
}

#[test]
fn copy_without_a_selection_covers_every_caret_line() {
    let mut e = editor("a|\nb\nc|");
    let out = run(&mut e, A::Copy);
    assert_eq!(out.clipboard, Some(ClipboardContent::lines("a\nc\n")));
}

#[test]
fn copy_of_empty_selection_can_be_disabled() {
    let mut e = editor("a|");
    e.settings.copy_cut_empty_selection_line = false;
    let out = run(&mut e, A::Copy);
    assert!(out.clipboard.is_none());
}

#[test]
fn cut_removes_the_selection_or_the_line() {
    // spec 3.8.
    let mut e = editor("a‹bc›d");
    let out = run(&mut e, A::Cut);
    assert_eq!(out.clipboard, Some(ClipboardContent::text("bc")));
    assert_eq!(show(&e), "a|d");
    let mut e = editor("a\nb|c\nd");
    let out = run(&mut e, A::Cut);
    assert_eq!(out.clipboard, Some(ClipboardContent::lines("bc\n")));
    assert_eq!(e.document.text(), "a\nd");
}

#[test]
fn cut_with_several_carets_joins_the_texts() {
    let mut e = editor("‹a›b‹c›");
    let out = run(&mut e, A::Cut);
    assert_eq!(out.clipboard, Some(ClipboardContent::text("a\nc")));
    assert_eq!(e.document.text(), "b");
}

#[test]
fn paste_of_a_copied_line_goes_above_the_caret_line() {
    // spec 3.8: the caret keeps its logical position on its shifted line.
    assert_eq!(
        paste("a\nb|c", ClipboardContent::lines("x\n"), A::Paste),
        "a\nx\nb|c"
    );
    assert_eq!(
        paste("|a", ClipboardContent::lines("x\n"), A::Paste),
        "x\n|a"
    );
}

#[test]
fn paste_of_a_copied_line_replaces_a_selection_normally() {
    assert_eq!(
        paste("a‹b›c", ClipboardContent::lines("x\n"), A::Paste),
        "ax\n|c"
    );
}

#[test]
fn paste_of_copied_lines_distributes_them_over_carets() {
    // spec 1.5 / 3.8.
    assert_eq!(
        paste("a|\nb|", ClipboardContent::lines("1\n2\n"), A::Paste),
        "1\na|\n2\nb|"
    );
}

#[test]
fn paste_replaces_the_selection_and_puts_the_caret_after() {
    assert_eq!(
        paste("a‹bc›d", ClipboardContent::text("XY"), A::Paste),
        "aXY|d"
    );
}

#[test]
fn paste_normalizes_line_separators() {
    let mut e = editor("|");
    let clip = ClipboardContent::text("a\r\nb\rc");
    let lang = Language::plain_text();
    e.execute(&A::Paste, &ActionContext::new(&lang).with_clipboard(&clip));
    assert_eq!(e.document.text(), "a\nb\nc");
}

#[test]
fn paste_of_n_lines_into_n_carets_gives_one_line_to_each() {
    // spec 1.5.
    assert_eq!(
        paste("|a |b", ClipboardContent::text("1\n2"), A::Paste),
        "1|a 2|b"
    );
}

#[test]
fn paste_of_other_line_counts_pastes_the_whole_text_at_each_caret() {
    // The second caret is at column 2, so its later lines are shifted (indent-block paste).
    assert_eq!(
        paste("|a |b", ClipboardContent::text("1\n2\n3"), A::Paste),
        "1\n2\n3|a 1\n  2\n  3|b"
    );
}

#[test]
fn paste_shifts_later_lines_by_the_caret_column() {
    // spec 3.8: the indent-block fallback (spaces only, first line untouched).
    assert_eq!(
        paste("    x|", ClipboardContent::text("a\nb\nc"), A::Paste),
        "    xa\n     b\n     c|"
    );
}

#[test]
fn paste_indent_is_skipped_at_column_zero_for_blank_first_lines_and_single_lines() {
    // spec 3.8.
    assert_eq!(
        paste("|", ClipboardContent::text("a\nb"), A::Paste),
        "a\nb|"
    );
    assert_eq!(
        paste("  |", ClipboardContent::text("\na\nb"), A::Paste),
        "  \na\nb|"
    );
    assert_eq!(
        paste("  |", ClipboardContent::text("ab"), A::Paste),
        "  ab|"
    );
}

#[test]
fn paste_simple_and_the_none_setting_paste_verbatim() {
    // spec 3.8: Paste simple skips all processing.
    assert_eq!(
        paste("  |", ClipboardContent::text("a\nb"), A::PasteSimple),
        "  a\nb|"
    );
    let mut e = editor("  |");
    e.settings.reformat_on_paste = ReformatOnPaste::None;
    let clip = ClipboardContent::text("a\nb");
    let lang = Language::plain_text();
    e.execute(&A::Paste, &ActionContext::new(&lang).with_clipboard(&clip));
    assert_eq!(e.document.text(), "  a\nb");
}

#[test]
fn paste_without_clipboard_content_is_not_handled() {
    let mut e = editor("a|");
    let out = run(&mut e, A::Paste);
    assert!(!out.handled);
}

#[test]
fn cut_line_end_kills_to_the_line_end() {
    assert_eq!(after("a|bc\nd", A::CutLineEnd), "a|\nd");
    assert_eq!(after("abc|\nd", A::CutLineEnd), "abc|d");
    let mut e = editor("a|bc");
    let out = run(&mut e, A::CutLineEnd);
    assert_eq!(out.clipboard, Some(ClipboardContent::text("bc")));
}

// ---- undo (spec 3.12) -----------------------------------------------------------------------

#[test]
fn a_typing_run_is_one_undo_step() {
    // spec 3.12: however long the pauses.
    let mut e = editor("|");
    type_str(&mut e, "hello", &Language::plain_text());
    assert_eq!(e.undo.undo_depth(), 1);
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "|");
    run(&mut e, A::Redo);
    assert_eq!(show(&e), "hello|");
}

#[test]
fn enter_is_its_own_step_and_breaks_the_typing_run() {
    // spec 3.12.
    let mut e = editor("|");
    type_str(&mut e, "ab", &Language::plain_text());
    run(&mut e, A::Enter);
    type_str(&mut e, "cd", &Language::plain_text());
    assert_eq!(e.undo.undo_depth(), 3);
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "ab\n|");
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "ab|");
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "|");
}

#[test]
fn consecutive_backspaces_merge_but_one_across_a_line_break_does_not() {
    // spec 3.12.
    let mut e = editor("ab\ncde|");
    for _ in 0..3 {
        run(&mut e, A::Backspace);
    }
    assert_eq!(e.undo.undo_depth(), 1);
    run(&mut e, A::Backspace);
    assert_eq!(show(&e), "ab|");
    assert_eq!(e.undo.undo_depth(), 2, "the join is a step of its own");
    run(&mut e, A::Backspace);
    run(&mut e, A::Backspace);
    assert_eq!(e.undo.undo_depth(), 3, "and ends the run");
}

#[test]
fn consecutive_tabs_merge() {
    // spec 3.12.
    let mut e = editor("|");
    run(&mut e, A::Tab);
    run(&mut e, A::Tab);
    assert_eq!(e.undo.undo_depth(), 1);
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "|");
}

#[test]
fn typing_after_backspace_starts_a_new_step() {
    let mut e = editor("ab|");
    run(&mut e, A::Backspace);
    type_str(&mut e, "x", &Language::plain_text());
    assert_eq!(e.undo.undo_depth(), 2);
}

#[test]
fn paste_duplicate_and_line_actions_are_one_step_each() {
    // spec 3.12.
    let mut e = editor("a|\nb\nc\nd\ne");
    let clip = ClipboardContent::text("X");
    let lang = Language::plain_text();
    e.execute(&A::Paste, &ActionContext::new(&lang).with_clipboard(&clip));
    assert_eq!(e.undo.undo_depth(), 1);
    run(&mut e, A::Duplicate);
    run(&mut e, A::MoveLineDown);
    run(&mut e, A::DeleteLine);
    run(&mut e, A::JoinLines);
    assert_eq!(e.undo.undo_depth(), 5);
    let end = show(&e);
    for _ in 0..5 {
        run(&mut e, A::Undo);
    }
    assert_eq!(show(&e), "a|\nb\nc\nd\ne");
    for _ in 0..5 {
        run(&mut e, A::Redo);
    }
    assert_eq!(show(&e), end);
}

#[test]
fn caret_movement_ends_a_typing_run() {
    let mut e = editor("|");
    type_str(&mut e, "ab", &Language::plain_text());
    run(&mut e, A::Left);
    type_str(&mut e, "x", &Language::plain_text());
    assert_eq!(e.undo.undo_depth(), 2);
}

#[test]
fn the_first_undo_after_caret_movement_only_restores_the_caret() {
    // spec 3.12: consumes the keypress, the text stays.
    let mut e = editor("|");
    type_str(&mut e, "abc", &Language::plain_text());
    run(&mut e, A::TextStart);
    assert_eq!(show(&e), "|abc");
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "abc|", "caret back where the edit left it");
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "|");
}

#[test]
fn caret_only_undo_also_restores_the_selection() {
    let mut e = editor("|");
    type_str(&mut e, "abc", &Language::plain_text());
    run(&mut e, A::SelectAll);
    run(&mut e, A::Left);
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "abc|");
}

#[test]
fn undo_restores_the_carets_before_the_step_and_redo_those_after() {
    // spec 3.12.
    let mut e = editor("a‹bc›d");
    run(&mut e, A::Backspace);
    assert_eq!(show(&e), "a|d");
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "a‹bc›d");
    run(&mut e, A::Redo);
    assert_eq!(show(&e), "a|d");
}

#[test]
fn undo_restores_all_carets() {
    let mut e = editor("a|b\nc|d");
    type_str(&mut e, "x", &Language::plain_text());
    assert_eq!(show(&e), "ax|b\ncx|d");
    run(&mut e, A::Undo);
    assert_eq!(show(&e), "a|b\nc|d");
}

#[test]
fn a_new_edit_clears_the_redo_stack() {
    let mut e = editor("|");
    type_str(&mut e, "a", &Language::plain_text());
    run(&mut e, A::Undo);
    assert!(e.undo.can_redo());
    type_str(&mut e, "b", &Language::plain_text());
    assert!(!e.undo.can_redo());
}

#[test]
fn undo_and_redo_with_nothing_to_do_are_not_handled() {
    let mut e = editor("a|");
    assert!(!run(&mut e, A::Undo).handled);
    assert!(!run(&mut e, A::Redo).handled);
}

#[test]
fn undo_keeps_100_steps_per_document() {
    // spec 0.
    let mut e = editor("|");
    for _ in 0..150 {
        run(&mut e, A::Enter);
    }
    assert_eq!(e.undo.undo_depth(), 100);
}

#[test]
fn auto_inserted_closers_belong_to_the_step_of_the_typed_key() {
    // spec 3.12.
    let c = Language::c_like();
    let mut e = editor("|");
    type_str(&mut e, "(", &c);
    assert_eq!(e.undo.undo_depth(), 1);
    e.execute(&A::Undo, &ActionContext::new(&c));
    assert_eq!(e.document.text(), "");
}

#[test]
fn replace_range_by_the_application_is_its_own_step() {
    let mut e = editor("ab|c");
    e.replace_range(0, 1, "XY");
    assert_eq!(e.document.text(), "XYbc");
    assert_eq!(
        e.carets.primary().offset,
        3,
        "carets keep their text position"
    );
    run(&mut e, A::Undo);
    assert_eq!(e.document.text(), "abc");
}

// ---- keymap (spec 12) ---------------------------------------------------------------------

fn key(k: Key, mods: KeyModifiers, os: Os) -> Option<EditorAction> {
    editor_action_for(k, mods, os)
}

const CTRL: KeyModifiers = KeyModifiers::CTRL;
const CMD: KeyModifiers = KeyModifiers::CMD;
const ALT: KeyModifiers = KeyModifiers::ALT;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const NONE: KeyModifiers = KeyModifiers::NONE;
const WL: Os = Os::WindowsLinux;
const MAC: Os = Os::MacOs;

#[test]
fn keymap_windows_linux_movement() {
    assert_eq!(key(Key::ArrowLeft, NONE, WL), Some(A::Left));
    assert_eq!(key(Key::ArrowRight, SHIFT, WL), Some(A::RightWithSelection));
    assert_eq!(key(Key::ArrowRight, CTRL, WL), Some(A::NextWord));
    assert_eq!(
        key(Key::ArrowLeft, CTRL.plus_shift(), WL),
        Some(A::PreviousWordWithSelection)
    );
    assert_eq!(key(Key::Home, NONE, WL), Some(A::LineStart));
    assert_eq!(key(Key::End, SHIFT, WL), Some(A::LineEndWithSelection));
    assert_eq!(key(Key::Home, CTRL, WL), Some(A::TextStart));
    assert_eq!(key(Key::PageDown, NONE, WL), Some(A::PageDown));
    assert_eq!(key(Key::PageDown, CTRL, WL), Some(A::MoveToPageBottom));
    assert_eq!(key(Key::ArrowUp, CTRL, WL), Some(A::ScrollUp));
    assert_eq!(key(Key::M, CTRL, WL), Some(A::ScrollToCenter));
    assert_eq!(key(Key::OpenBracket, CTRL, WL), Some(A::CodeBlockStart));
}

#[test]
fn keymap_macos_movement() {
    assert_eq!(key(Key::ArrowRight, ALT, MAC), Some(A::NextWord));
    assert_eq!(
        key(Key::ArrowLeft, ALT.plus_shift(), MAC),
        Some(A::PreviousWordWithSelection)
    );
    assert_eq!(key(Key::ArrowLeft, CMD, MAC), Some(A::LineStart));
    assert_eq!(
        key(Key::ArrowRight, CMD.plus_shift(), MAC),
        Some(A::LineEndWithSelection)
    );
    assert_eq!(key(Key::Home, CMD, MAC), Some(A::TextStart));
    assert_eq!(
        key(Key::ArrowUp, CMD, MAC),
        None,
        "Cmd+Up is not caret movement"
    );
    assert_eq!(
        key(Key::ArrowUp, CTRL, MAC),
        None,
        "Ctrl+Up is Mission Control, unbound"
    );
    assert_eq!(key(Key::B, CTRL, MAC), Some(A::Left));
    assert_eq!(key(Key::N, CTRL, MAC), Some(A::Down));
    assert_eq!(key(Key::A, CTRL, MAC), Some(A::LineStart));
    assert_eq!(key(Key::E, CTRL, MAC), Some(A::LineEnd));
    assert_eq!(key(Key::L, CTRL, MAC), Some(A::ScrollToCenter));
}

#[test]
fn keymap_selection_actions_differ_per_platform() {
    // spec 12: the three swaps that matter.
    assert_eq!(key(Key::W, CTRL, WL), Some(A::SelectWord));
    assert_eq!(key(Key::ArrowUp, ALT, MAC), Some(A::SelectWord));
    assert_eq!(key(Key::W, CTRL.plus_shift(), WL), Some(A::UnSelectWord));
    assert_eq!(key(Key::ArrowDown, ALT, MAC), Some(A::UnSelectWord));
    assert_eq!(key(Key::J, ALT, WL), Some(A::SelectNextOccurrence));
    assert_eq!(key(Key::G, CTRL, MAC), Some(A::SelectNextOccurrence));
    assert_eq!(
        key(Key::J, ALT.plus_shift(), WL),
        Some(A::UnselectPreviousOccurrence)
    );
    assert_eq!(
        key(Key::G, CTRL.plus_shift(), MAC),
        Some(A::UnselectPreviousOccurrence)
    );
    assert_eq!(
        key(Key::J, CTRL.plus_alt().plus_shift(), WL),
        Some(A::SelectAllOccurrences)
    );
    assert_eq!(
        key(Key::G, CTRL.plus_cmd(), MAC),
        Some(A::SelectAllOccurrences)
    );
    assert_eq!(
        key(Key::G, ALT.plus_shift(), WL),
        Some(A::AddCaretPerSelectedLine)
    );
    assert_eq!(
        key(Key::Insert, ALT.plus_shift(), WL),
        Some(A::ToggleColumnMode)
    );
    assert_eq!(
        key(Key::Num8, CMD.plus_shift(), MAC),
        Some(A::ToggleColumnMode)
    );
    assert_eq!(key(Key::A, CTRL, WL), Some(A::SelectAll));
    assert_eq!(key(Key::A, CMD, MAC), Some(A::SelectAll));
}

#[test]
fn keymap_editing_actions() {
    assert_eq!(key(Key::D, CTRL, WL), Some(A::Duplicate));
    assert_eq!(key(Key::D, CMD, MAC), Some(A::Duplicate));
    assert_eq!(key(Key::Y, CTRL, WL), Some(A::DeleteLine));
    assert_eq!(key(Key::Backspace, CMD, MAC), Some(A::DeleteLine));
    assert_eq!(key(Key::Y, CTRL, MAC), None);
    assert_eq!(key(Key::Y, CMD, MAC), None);
    assert_eq!(key(Key::Backspace, CTRL, WL), Some(A::DeleteToWordStart));
    assert_eq!(key(Key::Backspace, ALT, MAC), Some(A::DeleteToWordStart));
    assert_eq!(key(Key::Delete, ALT, MAC), Some(A::DeleteToWordEnd));
    assert_eq!(key(Key::Enter, SHIFT, WL), Some(A::StartNewLine));
    assert_eq!(
        key(Key::Enter, CTRL.plus_alt(), WL),
        Some(A::StartNewLineBefore)
    );
    assert_eq!(
        key(Key::Enter, CMD.plus_alt(), MAC),
        Some(A::StartNewLineBefore)
    );
    assert_eq!(key(Key::Enter, CTRL, WL), Some(A::SplitLine));
    assert_eq!(key(Key::Enter, CMD, MAC), Some(A::SplitLine));
    assert_eq!(
        key(Key::Enter, CTRL.plus_shift(), WL),
        Some(A::CompleteStatement)
    );
    assert_eq!(key(Key::Tab, NONE, WL), Some(A::Tab));
    assert_eq!(key(Key::Tab, SHIFT, MAC), Some(A::UnindentSelection));
    assert_eq!(key(Key::Backspace, SHIFT, WL), Some(A::Backspace));
    assert_eq!(key(Key::J, CTRL.plus_shift(), WL), Some(A::JoinLines));
    assert_eq!(key(Key::J, CTRL.plus_shift(), MAC), Some(A::JoinLines));
    assert_eq!(key(Key::U, CTRL.plus_shift(), WL), Some(A::ToggleCase));
    assert_eq!(key(Key::U, CMD.plus_shift(), MAC), Some(A::ToggleCase));
    assert_eq!(key(Key::ArrowUp, ALT.plus_shift(), WL), Some(A::MoveLineUp));
    assert_eq!(
        key(Key::ArrowDown, ALT.plus_shift(), MAC),
        Some(A::MoveLineDown)
    );
    assert_eq!(
        key(Key::ArrowUp, CTRL.plus_shift(), WL),
        Some(A::MoveStatementUp)
    );
    assert_eq!(
        key(Key::ArrowUp, CMD.plus_shift(), MAC),
        Some(A::MoveStatementUp)
    );
    assert_eq!(key(Key::Slash, CTRL, WL), Some(A::CommentByLineComment));
    assert_eq!(key(Key::Slash, CMD, MAC), Some(A::CommentByLineComment));
    assert_eq!(
        key(Key::Slash, CTRL.plus_shift(), WL),
        Some(A::CommentByBlockComment)
    );
    assert_eq!(key(Key::K, CTRL, MAC), Some(A::CutLineEnd));
    assert_eq!(key(Key::Escape, NONE, MAC), Some(A::Escape));
    assert_eq!(key(Key::Insert, NONE, WL), Some(A::ToggleInsertState));
}

#[test]
fn keymap_clipboard_undo_redo() {
    assert_eq!(key(Key::C, CTRL, WL), Some(A::Copy));
    assert_eq!(key(Key::C, CMD, MAC), Some(A::Copy));
    assert_eq!(key(Key::Insert, CTRL, WL), Some(A::Copy));
    assert_eq!(key(Key::Delete, SHIFT, WL), Some(A::Cut));
    assert_eq!(key(Key::Insert, SHIFT, MAC), Some(A::Paste));
    assert_eq!(
        key(Key::V, CTRL.plus_alt().plus_shift(), WL),
        Some(A::PasteSimple)
    );
    assert_eq!(
        key(Key::V, CMD.plus_alt().plus_shift(), MAC),
        Some(A::PasteSimple)
    );
    assert_eq!(key(Key::Z, CTRL, WL), Some(A::Undo));
    assert_eq!(
        key(Key::Backspace, ALT, WL),
        Some(A::Undo),
        "legacy Alt+Backspace"
    );
    assert_eq!(
        key(Key::Backspace, ALT, MAC),
        Some(A::DeleteToWordStart),
        "no Alt+Backspace undo on macOS"
    );
    assert_eq!(key(Key::Z, CMD, MAC), Some(A::Undo));
    assert_eq!(key(Key::Z, CTRL.plus_shift(), WL), Some(A::Redo));
    assert_eq!(key(Key::Z, CMD.plus_shift(), MAC), Some(A::Redo));
    assert_eq!(key(Key::Backspace, ALT.plus_shift(), WL), Some(A::Redo));
    assert_eq!(key(Key::Backspace, ALT.plus_shift(), MAC), Some(A::Redo));
    assert_eq!(
        key(Key::Y, CTRL, WL),
        Some(A::DeleteLine),
        "Ctrl+Y is not redo"
    );
}

#[test]
fn keymap_folding() {
    assert_eq!(key(Key::Plus, CTRL, WL), Some(A::ExpandRegion));
    assert_eq!(key(Key::Equals, CMD, MAC), Some(A::ExpandRegion));
    assert_eq!(key(Key::Minus, CTRL, WL), Some(A::CollapseRegion));
    assert_eq!(
        key(Key::Minus, CTRL.plus_alt(), WL),
        Some(A::CollapseRegionRecursively)
    );
    assert_eq!(
        key(Key::Minus, CMD.plus_alt(), MAC),
        Some(A::CollapseRegionRecursively)
    );
    assert_eq!(
        key(Key::Minus, CTRL.plus_shift(), WL),
        Some(A::CollapseAllRegions)
    );
    assert_eq!(
        key(Key::Equals, CMD.plus_shift(), MAC),
        Some(A::ExpandAllRegions)
    );
}

#[test]
fn keymap_unbound_chords_return_none() {
    assert_eq!(key(Key::Q, NONE, WL), None);
    assert_eq!(key(Key::ArrowLeft, ALT, WL), None);
}

#[test]
fn keymap_has_no_duplicate_chords_per_platform() {
    for os in [WL, MAC] {
        let bindings = default_bindings(os);
        for (i, a) in bindings.iter().enumerate() {
            for b in &bindings[i + 1..] {
                assert!(
                    !(a.key == b.key && a.modifiers == b.modifiers),
                    "{os:?}: {:?} {:?} bound twice",
                    a.key,
                    a.modifiers
                );
            }
        }
    }
}

#[test]
fn keymap_modifiers_convert_from_egui() {
    let mods = egui::Modifiers {
        alt: true,
        ctrl: false,
        shift: true,
        mac_cmd: true,
        command: true,
    };
    assert_eq!(KeyModifiers::from(mods), CMD.plus_alt().plus_shift());
}

#[test]
fn action_ids_are_the_jetbrains_ids() {
    assert_eq!(A::Left.id(), "EditorLeft");
    assert_eq!(A::RightWithSelection.id(), "EditorRightWithSelection");
    assert_eq!(A::LineStart.id(), "EditorLineStart");
    assert_eq!(A::SelectWord.id(), "EditorSelectWord");
    assert_eq!(A::Backspace.id(), "EditorBackSpace");
    assert_eq!(A::Undo.id(), "$Undo");
}

// ---- scrolling (spec 4.1) -----------------------------------------------------------------

fn viewport(first: f64, lines: f64) -> Viewport {
    Viewport {
        first_line: first,
        visible_lines: lines,
        first_column: 0.0,
        visible_columns: 80.0,
    }
}

const CONTENT: ContentSize = ContentSize {
    lines: 100,
    columns: 120,
};

#[test]
fn relative_scroll_keeps_a_one_line_margin_below() {
    // spec 4.1: band bottom = caret + 1 margin line + the caret line.
    let s = EditorSettings::default();
    assert_eq!(
        vertical_target(&viewport(0.0, 10.0), 9.0, ScrollType::Relative, &s),
        1.0
    );
    assert_eq!(
        vertical_target(&viewport(0.0, 10.0), 8.0, ScrollType::Relative, &s),
        0.0
    );
}

#[test]
fn relative_scroll_keeps_a_one_line_margin_above() {
    let s = EditorSettings::default();
    assert_eq!(
        vertical_target(&viewport(10.0, 10.0), 10.0, ScrollType::Relative, &s),
        9.0
    );
    assert_eq!(
        vertical_target(&viewport(10.0, 10.0), 11.0, ScrollType::Relative, &s),
        10.0
    );
}

#[test]
fn relative_scroll_centers_when_the_band_does_not_fit() {
    let s = EditorSettings::default();
    let target = vertical_target(&viewport(0.0, 2.0), 50.0, ScrollType::Relative, &s);
    assert_eq!(target, 49.5);
}

#[test]
fn make_visible_centers_a_caret_outside_the_margins() {
    // spec 4.1: one third from the top when the viewport is tall enough.
    let s = EditorSettings::default();
    let target = vertical_target(&viewport(0.0, 30.0), 60.0, ScrollType::MakeVisible, &s);
    assert_eq!(target, 50.0);
    let unchanged = vertical_target(&viewport(0.0, 30.0), 10.0, ScrollType::MakeVisible, &s);
    assert_eq!(unchanged, 0.0);
}

#[test]
fn center_always_centers_and_center_up_down_only_when_needed() {
    let s = EditorSettings::default();
    let v = viewport(0.0, 30.0);
    assert_eq!(vertical_target(&v, 10.0, ScrollType::Center, &s), 0.0);
    assert_eq!(vertical_target(&v, 40.0, ScrollType::CenterDown, &s), 30.0);
    assert_eq!(vertical_target(&v, 40.0, ScrollType::CenterUp, &s), 0.0);
    let v = viewport(50.0, 30.0);
    assert_eq!(vertical_target(&v, 40.0, ScrollType::CenterUp, &s), 30.0);
    assert_eq!(vertical_target(&v, 60.0, ScrollType::CenterUp, &s), 50.0);
}

#[test]
fn scroll_jump_adds_extra_lines() {
    let mut s = EditorSettings::default();
    s.vertical_scroll_jump = 3;
    assert_eq!(
        vertical_target(&viewport(0.0, 10.0), 9.0, ScrollType::Relative, &s),
        4.0
    );
}

#[test]
fn horizontal_scroll_keeps_a_three_column_margin() {
    // spec 4.1.
    let s = EditorSettings::default();
    let v = viewport(0.0, 10.0);
    assert_eq!(horizontal_target(&v, 79.0, ScrollType::Relative, &s), 2.0);
    assert_eq!(horizontal_target(&v, 70.0, ScrollType::Relative, &s), 0.0);
    let v = Viewport {
        first_column: 40.0,
        ..v
    };
    assert_eq!(horizontal_target(&v, 41.0, ScrollType::Relative, &s), 38.0);
    assert_eq!(
        horizontal_target(&v, 41.0, ScrollType::MakeVisible, &s),
        0.0,
        "fits in the first screen"
    );
    assert_eq!(
        horizontal_target(
            &Viewport {
                first_column: 5.0,
                ..v
            },
            6.0,
            ScrollType::MakeVisible,
            &s
        ),
        0.0
    );
}

#[test]
fn scroll_target_is_clamped_with_scroll_past_end() {
    // spec 4.1: 5 empty lines below the last line, 3 columns on the right.
    let s = EditorSettings::default();
    assert_eq!(max_first_line(CONTENT, 10.0, &s), 95.0);
    assert_eq!(max_first_column(CONTENT, 80.0, &s), 43.0);
    let t = scroll_to_caret(
        &viewport(80.0, 10.0),
        CONTENT,
        99.0,
        0.0,
        ScrollType::Center,
        false,
        &s,
    );
    assert_eq!(t.first_line, 95.0);
    let mut page = EditorSettings::default();
    page.additional_page_at_bottom = true;
    assert_eq!(max_first_line(CONTENT, 10.0, &page), 99.0);
    assert_eq!(clamp_first_line(500.0, CONTENT, 10.0, &s), 95.0);
    assert_eq!(clamp_first_line(-4.0, CONTENT, 10.0, &s), 0.0);
}

#[test]
fn scroll_animation_depends_on_the_jump_distance() {
    // spec 4.1: no animation for one line or less, the full 100 ms from 11 lines.
    let s = EditorSettings::default();
    let go = |target: f64, animate: bool| {
        scroll_to_caret(
            &viewport(0.0, 10.0),
            CONTENT,
            target,
            0.0,
            ScrollType::Relative,
            animate,
            &s,
        )
        .animation_ms
    };
    assert_eq!(go(9.0, true), 0.0, "scrolled by one line");
    assert!(go(15.0, true) > 0.0 && go(15.0, true) < MAX_ANIMATION_MS);
    assert_eq!(go(40.0, true), MAX_ANIMATION_MS);
    assert_eq!(go(40.0, false), 0.0, "typing and Home/End do not animate");
}

#[test]
fn scrolling_never_panics_on_degenerate_viewports() {
    let s = EditorSettings::default();
    let v = viewport(0.0, 0.0);
    let t = scroll_to_caret(
        &v,
        ContentSize {
            lines: 0,
            columns: 0,
        },
        5.0,
        5.0,
        ScrollType::MakeVisible,
        true,
        &s,
    );
    assert!(t.first_line.is_finite() && t.first_column.is_finite());
}

// ---- folding (spec 6) -------------------------------------------------------------------

fn folded() -> Editor {
    let mut e = Editor::new("fn a() {\n    one;\n    two;\n}\nafter");
    let end = e.document.line_end_offset(3);
    e.folding.add_region(&e.document, 7, end, "{...}", false);
    e
}

#[test]
fn folds_map_offsets_to_visual_lines() {
    let e = folded();
    assert_eq!(e.document.line_count(), 5);
    assert_eq!(e.folding.visual_line_count(&e.document), 2);
    assert_eq!(e.folding.logical_to_visual_line(&e.document, 0), 0);
    assert_eq!(
        e.folding.logical_to_visual_line(&e.document, 2),
        0,
        "hidden lines map to the placeholder line"
    );
    assert_eq!(e.folding.logical_to_visual_line(&e.document, 4), 1);
    assert_eq!(e.folding.visual_to_logical_line(&e.document, 1), 4);
    assert_eq!(e.folding.visual_line_text(&e.document, 0), "fn a() {...}");
    assert_eq!(e.folding.visual_line_text(&e.document, 1), "after");
}

#[test]
fn folds_give_visual_columns_with_placeholders() {
    let e = folded();
    let end = e.document.line_end_offset(3);
    assert_eq!(
        e.folding.visual_position(&e.document, end, 4),
        VisualPosition::new(0, 12)
    );
    assert_eq!(
        e.folding.visual_position(&e.document, 10, 4),
        VisualPosition::new(0, 7),
        "hidden offsets map to the fold start"
    );
    assert_eq!(
        e.folding.visual_position(&e.document, e.document.len(), 4),
        VisualPosition::new(1, 5)
    );
}

#[test]
fn expanded_folds_hide_nothing() {
    let mut e = folded();
    e.folding.expand_all();
    assert_eq!(e.folding.visual_line_count(&e.document), 5);
    assert_eq!(e.folding.visual_line_text(&e.document, 1), "    one;");
}

#[test]
fn a_caret_inside_a_collapsed_fold_expands_it() {
    // spec 6: the caret cannot be inside a collapsed region.
    let mut e = folded();
    e.carets.set_caret(&e.document, 12);
    assert!(e.folding.expand_around_carets(&e.carets));
    assert!(e.folding.regions()[0].expanded);
}

#[test]
fn a_caret_at_a_fold_boundary_does_not_expand_it() {
    let mut e = folded();
    let end = e.document.line_end_offset(3);
    e.carets.set_caret(&e.document, end);
    assert!(!e.folding.expand_around_carets(&e.carets));
    e.carets.set_caret(&e.document, 7);
    assert!(!e.folding.expand_around_carets(&e.carets));
}

#[test]
fn typing_at_a_fold_boundary_does_not_unfold_but_inside_does() {
    // spec 6.
    let mut e = folded();
    let end = e.document.line_end_offset(3);
    e.carets.set_caret(&e.document, end);
    type_str(&mut e, "x", &Language::plain_text());
    assert!(!e.folding.regions()[0].expanded);
    assert_eq!(e.folding.regions()[0].end, end);
    e.replace_range(12, 13, "Z");
    assert!(e.folding.regions()[0].expanded);
}

#[test]
fn vertical_motion_skips_hidden_lines() {
    let mut e = folded();
    e.carets.set_caret(&e.document, 3);
    run(&mut e, A::Down);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 4);
    run(&mut e, A::Up);
    assert_eq!(e.document.line_number(e.carets.primary().offset), 0);
}

#[test]
fn collapse_and_expand_actions_act_at_the_caret() {
    // spec 6: collapse the innermost expanded region, expand the collapsed one at the caret.
    let mut e = folded();
    e.folding.expand_all();
    e.carets.set_caret(&e.document, 12);
    run(&mut e, A::CollapseRegion);
    assert!(!e.folding.regions()[0].expanded);
    assert_eq!(
        e.carets.primary().offset,
        7,
        "the caret moves to the fold start"
    );
    run(&mut e, A::ExpandRegion);
    assert!(e.folding.regions()[0].expanded);
    run(&mut e, A::CollapseAllRegions);
    assert!(!e.folding.regions()[0].expanded);
    run(&mut e, A::ExpandAllRegions);
    assert!(e.folding.regions()[0].expanded);
}

#[test]
fn recursive_fold_actions_include_nested_regions() {
    let mut e = Editor::new("a {\n b {\n c\n }\n}");
    let d = &e.document;
    e.folding.add_region(d, 2, d.len(), "{...}", true);
    e.folding.add_region(d, 7, 14, "{...}", true);
    e.carets.set_caret(&e.document, 3);
    run(&mut e, A::CollapseRegionRecursively);
    assert!(e.folding.regions().iter().all(|r| !r.expanded));
    e.carets.set_caret(&e.document, 2);
    run(&mut e, A::ExpandRegionRecursively);
    assert!(e.folding.regions().iter().all(|r| r.expanded));
}

#[test]
fn fold_regions_follow_edits_and_reject_overlaps() {
    let mut e = Editor::new("0123456789");
    assert!(
        e.folding
            .add_region(&e.document, 2, 6, "...", false)
            .is_some()
    );
    assert!(
        e.folding
            .add_region(&e.document, 4, 8, "...", false)
            .is_none(),
        "partial overlap"
    );
    assert!(
        e.folding
            .add_region(&e.document, 3, 3, "...", false)
            .is_none(),
        "empty"
    );
    assert!(
        e.folding
            .add_region(&e.document, 3, 5, "...", true)
            .is_some(),
        "nested"
    );
    e.replace_range(0, 0, "ab");
    assert_eq!(
        (e.folding.regions()[0].start, e.folding.regions()[0].end),
        (4, 8)
    );
}

#[test]
fn deleting_a_fold_removes_the_region() {
    let mut e = Editor::new("0123456789");
    e.folding.add_region(&e.document, 2, 6, "...", false);
    e.replace_range(0, 10, "");
    assert!(e.folding.regions().is_empty());
}

// ---- robustness (no panics) ---------------------------------------------------------------

fn all_actions() -> Vec<EditorAction> {
    vec![
        A::Left,
        A::LeftWithSelection,
        A::Right,
        A::RightWithSelection,
        A::Up,
        A::UpWithSelection,
        A::Down,
        A::DownWithSelection,
        A::PreviousWord,
        A::PreviousWordWithSelection,
        A::NextWord,
        A::NextWordWithSelection,
        A::LineStart,
        A::LineStartWithSelection,
        A::LineEnd,
        A::LineEndWithSelection,
        A::PageUp,
        A::PageUpWithSelection,
        A::PageDown,
        A::PageDownWithSelection,
        A::TextStart,
        A::TextStartWithSelection,
        A::TextEnd,
        A::TextEndWithSelection,
        A::MoveToPageTop,
        A::MoveToPageTopWithSelection,
        A::MoveToPageBottom,
        A::MoveToPageBottomWithSelection,
        A::ScrollUp,
        A::ScrollDown,
        A::ScrollToCenter,
        A::CodeBlockStart,
        A::CodeBlockStartWithSelection,
        A::CodeBlockEnd,
        A::CodeBlockEndWithSelection,
        A::MatchBrace,
        A::SelectAll,
        A::SelectWord,
        A::UnSelectWord,
        A::SelectNextOccurrence,
        A::UnselectPreviousOccurrence,
        A::SelectAllOccurrences,
        A::CloneCaretAbove,
        A::CloneCaretBelow,
        A::AddCaretPerSelectedLine,
        A::ToggleColumnMode,
        A::ToggleInsertState,
        A::Escape,
        A::Type('a'),
        A::Type('('),
        A::Type(')'),
        A::Type('"'),
        A::Type('é'),
        A::Type('\n'),
        A::Type('😀'),
        A::Enter,
        A::StartNewLine,
        A::StartNewLineBefore,
        A::CompleteStatement,
        A::SplitLine,
        A::Tab,
        A::UnindentSelection,
        A::Backspace,
        A::Delete,
        A::DeleteToWordStart,
        A::DeleteToWordEnd,
        A::DeleteLine,
        A::CutLineEnd,
        A::Duplicate,
        A::JoinLines,
        A::ToggleCase,
        A::MoveLineUp,
        A::MoveLineDown,
        A::MoveStatementUp,
        A::MoveStatementDown,
        A::CommentByLineComment,
        A::CommentByBlockComment,
        A::Cut,
        A::Copy,
        A::Paste,
        A::PasteSimple,
        A::Undo,
        A::Redo,
        A::ExpandRegion,
        A::CollapseRegion,
        A::ExpandRegionRecursively,
        A::CollapseRegionRecursively,
        A::ExpandAllRegions,
        A::CollapseAllRegions,
    ]
}

#[test]
fn no_action_panics_on_any_document_or_caret_state() {
    let texts = [
        "",
        "\n",
        "a",
        "é😀\n日本\t語\n\n  {(\"x\")}\n// c\n",
        "   \n\t\n  ",
        "foo bar\nbaz\n",
        "((((",
        "\"\"\"'''",
    ];
    let language = Language::c_like();
    let clip = ClipboardContent::text("p\nq\r\nr");
    let line_clip = ClipboardContent::lines("line\n");
    let mut seed = 99u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let actions = all_actions();
    for text in texts {
        for round in 0..400 {
            let mut e = Editor::new(text);
            let len = e.document.len();
            // Carets anywhere, including out of range and inside characters.
            for _ in 0..(next() % 4 + 1) {
                let a = (next() % (len as u64 + 20)) as usize;
                let b = (next() % (len as u64 + 20)) as usize;
                e.carets
                    .carets_mut()
                    .push(Caret::at(a, 100 + next() % 1000));
                let last = e.carets.carets_mut().len() - 1;
                e.carets.carets_mut()[last].anchor = b;
            }
            if round % 3 == 0 {
                e.folding.add_region(&e.document, 0, len, "...", false);
            }
            for _ in 0..12 {
                let action = &actions[(next() % actions.len() as u64) as usize];
                let c = if next() % 2 == 0 { &clip } else { &line_clip };
                let ctx = ActionContext::new(&language)
                    .with_viewport((next() % 5) as usize, (next() % 12) as usize)
                    .with_clipboard(c);
                e.execute(action, &ctx);
                // Invariants.
                assert!(e.carets.caret_count() >= 1);
                for caret in e.carets.carets() {
                    assert!(
                        caret.offset <= e.document.len(),
                        "{action:?} on {text:?}: {caret:?}"
                    );
                    assert!(
                        e.document.text().is_char_boundary(caret.offset),
                        "{action:?} on {text:?}"
                    );
                    assert!(
                        e.document.text().is_char_boundary(caret.anchor),
                        "{action:?} on {text:?}"
                    );
                }
                let rebuilt = Document::new(e.document.text());
                assert_eq!(
                    rebuilt.line_count(),
                    e.document.line_count(),
                    "{action:?} on {text:?}"
                );
            }
        }
    }
}

#[test]
fn execute_clamps_stale_carets() {
    let mut doc = Document::new("héllo");
    let mut carets = CaretModel::new();
    carets.carets_mut()[0].offset = 500;
    carets.carets_mut()[0].anchor = 2;
    let settings = EditorSettings::default();
    execute(&mut doc, &mut carets, &settings, &A::Type('x'), &plain());
    assert_eq!(doc.text(), "hx");
    assert!(doc.text().is_char_boundary(carets.primary().offset));
}

#[test]
fn everything_works_on_an_empty_document() {
    for action in all_actions() {
        let mut e = Editor::new("");
        e.execute(&action, &plain());
        assert!(e.document.text().len() < 10);
    }
}

#[test]
fn scrolling_and_folding_accept_out_of_range_input() {
    let doc = Document::new("a\nb");
    let mut folds = FoldingModel::new();
    assert!(folds.add_region(&doc, 100, 200, "x", false).is_none());
    assert_eq!(folds.logical_to_visual_line(&doc, 99), 99);
    assert_eq!(folds.visual_to_logical_line(&doc, 99), 1);
    assert_eq!(
        folds.visual_position(&doc, 999, 4),
        VisualPosition::new(1, 1)
    );
}
