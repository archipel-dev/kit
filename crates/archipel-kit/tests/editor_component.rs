//! The rules of the IntelliJ editor, one test per rule: keys through the keymap, text and
//! clipboard input, the mouse, scrolling, the gutter, folding, highlights and painting. The
//! assertions look at the model (carets, text, folds) the component drives, or at what the
//! component reports.

mod common;

use archipel_kit::editor::model::{Editor, Os};
use archipel_kit::editor::{
    ChangeKind, ChangeMarker, EditorComponent, GutterArea, GutterClick, GutterIcon,
    HighlightSeverity, Json, NavigationKind, NavigationRequest, RangeHighlight,
};
use archipel_kit::testing::{self, Harness, NodeT, Queryable};
use archipel_kit::{Glyph, Theme, Tone, egui};
use egui::{Event, Key, Modifiers, OutputCommand, PointerButton, Pos2, pos2};

/// One cell of JetBrains Mono 13 and the padding left of column 0.
const CELL: f32 = 7.8;
const PAD: f32 = 8.0;
const ROW: f32 = 20.0;

type H<'a> = Harness<'a, Editor>;

thread_local! {
    /// The texts the component copied to the clipboard, in order.
    static COPIES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Call at the end of the app closure: records the copies of the frame.
fn record_copies(ctx: &egui::Context) {
    ctx.output(|o| {
        for command in &o.commands {
            if let OutputCommand::CopyText(text) = command {
                COPIES.with(|c| c.borrow_mut().push(text.clone()));
            }
        }
    });
}

fn harness_with<'a>(text: &str, configure: impl FnOnce(&mut Editor), height: f32) -> H<'a> {
    let mut editor = Editor::new(text);
    configure(&mut editor);
    testing::harness_state(Theme::dark(), editor, move |ui, editor| {
        EditorComponent::new(editor)
            .label("Ed")
            .height(height)
            .show(ui);
        record_copies(ui.ctx());
    })
}

fn harness<'a>(text: &str) -> H<'a> {
    harness_with(text, |_| {}, 200.0)
}

fn focused<'a>(text: &str) -> H<'a> {
    let mut h = harness(text);
    h.get_by_label("Ed").focus();
    h.run_ok();
    h
}

fn text(h: &H<'_>) -> String {
    h.state().document.text().to_owned()
}

fn caret(h: &H<'_>) -> usize {
    h.state().carets.primary().offset
}

fn set_caret(h: &mut H<'_>, offset: usize) {
    let editor = h.state_mut();
    let doc = editor.document.clone();
    editor.carets.set_caret(&doc, offset);
    h.run_ok();
}

fn key(h: &mut H<'_>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run_ok();
}

const CTRL: Modifiers = Modifiers::COMMAND;

/// The screen position of column `col` on row `row` of the text area.
fn at(h: &H<'_>, row: usize, col: f32) -> Pos2 {
    let rect = h.get_by_label("Ed").rect();
    pos2(
        rect.left() + PAD + col * CELL,
        rect.top() + 4.0 + ROW * row as f32 + ROW / 2.0,
    )
}

fn press(h: &mut H<'_>, pos: Pos2, button: PointerButton, modifiers: Modifiers) {
    h.hover_at(pos);
    h.event(Event::PointerButton {
        pos,
        button,
        pressed: true,
        modifiers,
    });
    h.run_ok();
}

fn release(h: &mut H<'_>, pos: Pos2, button: PointerButton, modifiers: Modifiers) {
    h.event(Event::PointerButton {
        pos,
        button,
        pressed: false,
        modifiers,
    });
    h.run_ok();
}

fn click(h: &mut H<'_>, pos: Pos2, modifiers: Modifiers) {
    press(h, pos, PointerButton::Primary, modifiers);
    release(h, pos, PointerButton::Primary, modifiers);
}

fn drag(h: &mut H<'_>, from: Pos2, to: Pos2, modifiers: Modifiers, button: PointerButton) {
    press(h, from, button, modifiers);
    h.event(Event::PointerMoved(to));
    h.run_ok();
    release(h, to, button, modifiers);
}

fn selected(h: &H<'_>) -> Vec<String> {
    let editor = h.state();
    editor
        .carets
        .selected_texts(&editor.document)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn copied(_h: &mut H<'_>) -> Option<String> {
    COPIES.with(|c| c.borrow().last().cloned())
}

fn copy_count() -> usize {
    COPIES.with(|c| c.borrow().len())
}

// ---- keyboard -------------------------------------------------------------------------

#[test]
fn ctrl_right_stops_at_the_word_end_not_the_next_word() {
    let mut h = focused("foo bar");
    key(&mut h, CTRL, Key::ArrowRight);
    assert_eq!(caret(&h), 3);
    key(&mut h, CTRL, Key::ArrowRight);
    assert_eq!(caret(&h), 7);
}

#[test]
fn ctrl_left_stops_at_the_word_start() {
    let mut h = focused("foo bar");
    set_caret(&mut h, 7);
    key(&mut h, CTRL, Key::ArrowLeft);
    assert_eq!(caret(&h), 4);
}

#[test]
fn home_goes_to_the_indent_first_then_to_column_zero() {
    let mut h = focused("    foo");
    set_caret(&mut h, 7);
    key(&mut h, Modifiers::NONE, Key::Home);
    assert_eq!(caret(&h), 4, "smart home: the first non-blank");
    key(&mut h, Modifiers::NONE, Key::Home);
    assert_eq!(caret(&h), 0);
}

#[test]
fn left_with_a_selection_collapses_it_to_its_start() {
    let mut h = focused("hello world");
    key(&mut h, Modifiers::SHIFT, Key::ArrowRight);
    key(&mut h, Modifiers::SHIFT, Key::ArrowRight);
    assert_eq!(selected(&h), ["he"]);
    key(&mut h, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(caret(&h), 0);
    assert!(selected(&h).is_empty());
}

#[test]
fn select_all_and_typing_replaces_the_text() {
    let mut h = focused("abc\ndef");
    key(&mut h, CTRL, Key::A);
    assert_eq!(selected(&h), ["abc\ndef"]);
    h.get_by_label("Ed").type_text("x");
    h.run_ok();
    assert_eq!(text(&h), "x");
}

#[test]
fn duplicate_line_with_ctrl_d() {
    let mut h = focused("one\ntwo");
    key(&mut h, CTRL, Key::D);
    assert_eq!(text(&h), "one\none\ntwo");
}

#[test]
fn delete_line_with_ctrl_y_not_redo() {
    let mut h = focused("one\ntwo\nthree");
    set_caret(&mut h, 5);
    key(&mut h, CTRL, Key::Y);
    assert_eq!(text(&h), "one\nthree");
}

#[test]
fn a_run_of_typing_is_one_undo_step_and_redo_is_ctrl_shift_z() {
    let mut h = focused("");
    h.get_by_label("Ed").type_text("abc");
    h.run_ok();
    h.get_by_label("Ed").type_text("def");
    h.run_ok();
    assert_eq!(text(&h), "abcdef");
    key(&mut h, CTRL, Key::Z);
    assert_eq!(text(&h), "", "typing merges into one step");
    key(&mut h, CTRL.plus(Modifiers::SHIFT), Key::Z);
    assert_eq!(text(&h), "abcdef");
}

#[test]
fn move_line_down_with_alt_shift_down() {
    let mut h = focused("one\ntwo");
    key(&mut h, Modifiers::ALT | Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(text(&h), "two\none");
}

#[test]
fn copy_without_a_selection_copies_the_whole_line_and_paste_goes_above() {
    let mut h = focused("one\ntwo");
    set_caret(&mut h, 5);
    key(&mut h, CTRL, Key::C);
    assert_eq!(copied(&mut h).as_deref(), Some("two\n"));
    set_caret(&mut h, 1);
    h.event(Event::Paste("two\n".into()));
    h.run_ok();
    assert_eq!(
        text(&h),
        "two\none\ntwo",
        "a copied line pastes above the caret line"
    );
}

#[test]
fn paste_inserts_the_text_at_the_caret() {
    let mut h = focused("ab");
    set_caret(&mut h, 1);
    h.event(Event::Paste("XY".into()));
    h.run_ok();
    assert_eq!(text(&h), "aXYb");
    assert_eq!(caret(&h), 3);
}

#[test]
fn cut_removes_the_selection_and_copies_it() {
    let mut h = focused("hello world");
    key(&mut h, Modifiers::SHIFT, Key::ArrowRight);
    key(&mut h, Modifiers::SHIFT, Key::ArrowRight);
    key(&mut h, CTRL, Key::X);
    assert_eq!(text(&h), "llo world");
    assert_eq!(copied(&mut h).as_deref(), Some("he"));
}

#[test]
fn the_egui_copy_event_copies_once_even_with_the_key() {
    let mut h = focused("hello");
    key(&mut h, CTRL, Key::A);
    let before = copy_count();
    // Both in one frame, as a backend delivers Ctrl+C.
    h.input_mut().events.push(Event::Key {
        key: Key::C,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: CTRL,
    });
    h.input_mut().events.push(Event::Copy);
    h.step();
    assert_eq!(copy_count() - before, 1);
}

#[test]
fn the_egui_cut_event_cuts() {
    let mut h = focused("hello");
    key(&mut h, CTRL, Key::A);
    h.event(Event::Cut);
    h.run_ok();
    assert_eq!(text(&h), "");
}

#[test]
fn alt_j_selects_the_next_occurrence_with_a_second_caret() {
    let mut h = focused("foo bar foo");
    key(&mut h, Modifiers::ALT, Key::J);
    assert_eq!(selected(&h), ["foo"]);
    key(&mut h, Modifiers::ALT, Key::J);
    assert_eq!(h.state().carets.caret_count(), 2);
    h.get_by_label("Ed").type_text("x");
    h.run_ok();
    assert_eq!(text(&h), "x bar x", "typing goes to every caret");
}

#[test]
fn escape_clears_the_selection_and_the_extra_carets() {
    let mut h = focused("foo foo");
    key(&mut h, Modifiers::ALT, Key::J);
    key(&mut h, Modifiers::ALT, Key::J);
    assert_eq!(h.state().carets.caret_count(), 2);
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert_eq!(h.state().carets.caret_count(), 1);
    assert!(selected(&h).is_empty());
    assert!(
        h.get_by_label("Ed").accesskit_node().is_focused(),
        "the Escape that cleared the carets does not leave the editor"
    );
}

#[test]
fn insert_toggles_overwrite_mode() {
    let mut h = focused("abc");
    key(&mut h, Modifiers::NONE, Key::Insert);
    assert!(h.state().carets.overwrite);
    h.get_by_label("Ed").type_text("X");
    h.run_ok();
    assert_eq!(text(&h), "Xbc");
}

#[test]
fn ime_commit_types_the_text() {
    let mut h = focused("");
    h.event(Event::Ime(egui::ImeEvent::Commit("héllo".into())));
    h.run_ok();
    assert_eq!(text(&h), "héllo");
}

#[test]
fn the_macos_keymap_uses_cmd_for_line_start() {
    let mut editor = Editor::new("    foo bar");
    let mut h = testing::harness_state(Theme::dark(), editor.clone(), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .keymap_os(Os::MacOs)
            .height(100.0)
            .show(ui);
    });
    editor.carets.set_caret(&editor.document, 9);
    *h.state_mut() = editor;
    h.get_by_label("Ed").focus();
    h.run_ok();
    h.key_press_modifiers(Modifiers::MAC_CMD | Modifiers::COMMAND, Key::ArrowLeft);
    h.run_ok();
    assert_eq!(
        h.state().carets.primary().offset,
        4,
        "Cmd+Left is smart Home"
    );
}

#[test]
fn tab_stays_in_the_editor_and_indents() {
    let mut h = focused("a");
    key(&mut h, Modifiers::NONE, Key::Tab);
    assert_eq!(text(&h), "    a");
    assert!(h.get_by_label("Ed").accesskit_node().is_focused());
}

#[test]
fn a_viewer_ignores_editing_keys_but_moves_and_copies() {
    let mut h = testing::harness_state(Theme::dark(), Editor::new("hello"), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .viewer(true)
            .height(100.0)
            .show(ui);
        record_copies(ui.ctx());
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    h.get_by_label("Ed").type_text("x");
    h.run_ok();
    key(&mut h, Modifiers::NONE, Key::Backspace);
    key(&mut h, CTRL, Key::D);
    assert_eq!(h.state().document.text(), "hello");
    key(&mut h, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(h.state().carets.primary().offset, 1, "carets work");
    key(&mut h, CTRL, Key::A);
    key(&mut h, CTRL, Key::C);
    assert_eq!(copied(&mut h).as_deref(), Some("hello"));
    h.event(Event::Paste("zz".into()));
    h.run_ok();
    assert_eq!(h.state().document.text(), "hello", "paste does nothing");
}

#[test]
fn a_viewer_supports_multiple_carets() {
    let mut h = testing::harness_state(Theme::dark(), Editor::new("foo foo"), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .viewer(true)
            .height(100.0)
            .show(ui);
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, Modifiers::ALT, Key::J);
    key(&mut h, Modifiers::ALT, Key::J);
    assert_eq!(h.state().carets.caret_count(), 2);
}

#[test]
fn tab_leaves_a_viewer() {
    let mut h = testing::harness_state(Theme::dark(), Editor::new("hello"), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .viewer(true)
            .height(100.0)
            .show(ui);
        ui.add(archipel_kit::ui::Button::primary("Next"));
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, Modifiers::NONE, Key::Tab);
    assert!(
        !h.get_by_label("Ed").accesskit_node().is_focused(),
        "Tab moves on from a viewer"
    );
}

#[test]
fn tab_moves_focus_when_tab_indents_is_off() {
    let mut h = testing::harness_state(Theme::dark(), Editor::new("a"), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .tab_indents(false)
            .height(100.0)
            .show(ui);
        ui.add(archipel_kit::ui::Button::primary("Next"));
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, Modifiers::NONE, Key::Tab);
    assert_eq!(h.state().document.text(), "a");
    assert!(!h.get_by_label("Ed").accesskit_node().is_focused());
}

#[test]
fn a_disabled_component_does_not_react() {
    let mut h = testing::harness_state(Theme::dark(), Editor::new("hello"), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .enabled(false)
            .height(100.0)
            .show(ui);
    });
    h.get_by_label("Ed").type_text("x");
    h.run_ok();
    assert_eq!(h.state().document.text(), "hello");
}

#[test]
fn the_response_reports_text_changes_and_caret_moves() {
    let report = std::cell::Cell::new((false, false));
    let mut h = testing::harness_state(Theme::dark(), Editor::new("ab"), |ui, e| {
        let out = EditorComponent::new(e).label("Ed").height(100.0).show(ui);
        let (text, moved) = report.get();
        report.set((text | out.text_changed, moved | out.carets_moved));
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    report.set((false, false));
    h.step();
    assert_eq!(report.get(), (false, false), "idle");
    h.get_by_label("Ed").type_text("x");
    h.run_ok();
    assert_eq!(report.get(), (true, true));
    report.set((false, false));
    h.key_press(Key::ArrowLeft);
    h.run_ok();
    assert_eq!(report.get(), (false, true), "a caret move is reported");
}

// ---- mouse ----------------------------------------------------------------------------

#[test]
fn a_click_places_the_caret_at_the_nearest_boundary() {
    let mut h = focused("hello world");
    let pos = at(&h, 0, 5.2);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(caret(&h), 5);
    let pos = at(&h, 0, 8.7);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(caret(&h), 9);
}

#[test]
fn a_click_past_the_line_end_goes_to_the_line_end() {
    let mut h = focused("ab\ncd");
    let pos = at(&h, 0, 30.0);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(caret(&h), 2);
}

#[test]
fn a_click_below_the_last_line_goes_to_the_last_line() {
    let mut h = focused("ab\ncd");
    let pos = at(&h, 6, 1.0);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(caret(&h), 4);
}

#[test]
fn double_click_selects_the_word_and_triple_click_the_line() {
    let mut h = focused("foo bar\nbaz");
    let pos = at(&h, 0, 5.0);
    click(&mut h, pos, Modifiers::NONE);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(selected(&h), ["bar"]);
    click(&mut h, pos, Modifiers::NONE);
    click(&mut h, pos, Modifiers::NONE);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(selected(&h), ["foo bar\n"], "the line with its break");
    assert_eq!(caret(&h), 8, "the caret ends at the start of the next line");
}

#[test]
fn double_click_on_blanks_selects_the_line_text() {
    let mut h = focused("a   b");
    let pos = at(&h, 0, 2.0);
    click(&mut h, pos, Modifiers::NONE);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(selected(&h), ["a   b"]);
}

#[test]
fn dragging_after_a_double_click_extends_by_words() {
    let mut h = focused("foo bar baz");
    let word = at(&h, 0, 5.0);
    click(&mut h, word, Modifiers::NONE);
    press(&mut h, word, PointerButton::Primary, Modifiers::NONE);
    let to = at(&h, 0, 9.0);
    h.event(Event::PointerMoved(to));
    h.run_ok();
    release(&mut h, to, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(selected(&h), ["bar baz"]);
}

#[test]
fn shift_click_extends_the_selection_from_the_anchor() {
    let mut h = focused("hello world");
    let a = at(&h, 0, 2.0);
    click(&mut h, a, Modifiers::NONE);
    let b = at(&h, 0, 8.0);
    click(&mut h, b, Modifiers::SHIFT);
    assert_eq!(selected(&h), ["llo wo"]);
}

#[test]
fn drag_selects_from_the_press_to_the_pointer() {
    let mut h = focused("hello world");
    let from = at(&h, 0, 1.0);
    let to = at(&h, 0, 4.0);
    drag(&mut h, from, to, Modifiers::NONE, PointerButton::Primary);
    assert_eq!(selected(&h), ["ell"]);
}

#[test]
fn drag_upwards_keeps_the_anchor() {
    let mut h = focused("one\ntwo\nthree");
    let from = at(&h, 2, 2.0);
    let to = at(&h, 0, 1.0);
    drag(&mut h, from, to, Modifiers::NONE, PointerButton::Primary);
    assert_eq!(selected(&h), ["ne\ntwo\nth"]);
}

#[test]
fn a_press_inside_the_selection_collapses_it_on_release_without_a_drag() {
    let mut h = focused("hello world");
    key(&mut h, CTRL, Key::A);
    let pos = at(&h, 0, 3.0);
    press(&mut h, pos, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(selected(&h), ["hello world"], "kept until release");
    release(&mut h, pos, PointerButton::Primary, Modifiers::NONE);
    assert!(selected(&h).is_empty());
    assert_eq!(caret(&h), 3);
}

#[test]
fn alt_shift_click_adds_a_caret_and_clicking_it_again_removes_it() {
    let mut h = focused("one\ntwo\nthree");
    let second = at(&h, 1, 1.0);
    click(&mut h, second, Modifiers::ALT | Modifiers::SHIFT);
    assert_eq!(h.state().carets.caret_count(), 2);
    let third = at(&h, 2, 2.0);
    click(&mut h, third, Modifiers::ALT | Modifiers::SHIFT);
    assert_eq!(h.state().carets.caret_count(), 3);
    click(&mut h, second, Modifiers::ALT | Modifiers::SHIFT);
    assert_eq!(h.state().carets.caret_count(), 2, "removed");
    h.get_by_label("Ed").type_text("!");
    h.run_ok();
    assert_eq!(text(&h), "!one\ntwo\nth!ree");
}

#[test]
fn alt_drag_makes_a_column_selection() {
    let mut h = focused("abcd\nabcd\nabcd");
    let from = at(&h, 0, 1.0);
    let to = at(&h, 2, 3.0);
    drag(&mut h, from, to, Modifiers::ALT, PointerButton::Primary);
    assert_eq!(h.state().carets.caret_count(), 3);
    assert_eq!(selected(&h), ["bc", "bc", "bc"]);
}

#[test]
fn middle_drag_makes_a_column_selection_too() {
    let mut h = focused("abcd\nabcd");
    let from = at(&h, 0, 1.0);
    let to = at(&h, 1, 3.0);
    drag(&mut h, from, to, Modifiers::NONE, PointerButton::Middle);
    assert_eq!(selected(&h), ["bc", "bc"]);
}

#[test]
fn a_column_selection_on_a_short_line_leaves_a_caret_at_its_end() {
    let mut h = focused("abcd\na\nabcd");
    let from = at(&h, 0, 2.0);
    let to = at(&h, 2, 4.0);
    drag(&mut h, from, to, Modifiers::ALT, PointerButton::Primary);
    assert_eq!(h.state().carets.caret_count(), 3);
    assert_eq!(selected(&h), ["cd", "cd"]);
}

#[test]
fn ctrl_click_reports_a_navigation_request() {
    let mut h = testing::harness_state(
        Theme::dark(),
        (Editor::new("foo bar"), None),
        |ui, (e, nav): &mut (Editor, Option<NavigationRequest>)| {
            if let Some(n) = EditorComponent::new(e)
                .label("Ed")
                .height(100.0)
                .show(ui)
                .navigation
            {
                *nav = Some(n);
            }
        },
    );
    h.get_by_label("Ed").focus();
    h.run_ok();
    let rect = h.get_by_label("Ed").rect();
    let pos = pos2(rect.left() + PAD + 5.0 * CELL, rect.top() + 14.0);
    h.hover_at(pos);
    h.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::CTRL,
    });
    h.run_ok();
    assert_eq!(
        h.state().1,
        Some(NavigationRequest {
            offset: 5,
            kind: NavigationKind::Declaration
        })
    );
}

#[test]
fn right_click_moves_the_caret_unless_inside_the_selection() {
    let mut h = focused("hello world");
    let pos = at(&h, 0, 3.0);
    click(&mut h, pos, Modifiers::NONE);
    key(&mut h, CTRL, Key::A);
    press(&mut h, pos, PointerButton::Secondary, Modifiers::NONE);
    release(&mut h, pos, PointerButton::Secondary, Modifiers::NONE);
    assert_eq!(selected(&h), ["hello world"], "kept: the click was inside");
    set_caret(&mut h, 0);
    let pos = at(&h, 0, 8.0);
    press(&mut h, pos, PointerButton::Secondary, Modifiers::NONE);
    release(&mut h, pos, PointerButton::Secondary, Modifiers::NONE);
    assert_eq!(caret(&h), 8);
}

#[test]
fn a_click_gives_the_component_the_focus() {
    let mut h = harness("hello");
    let pos = at(&h, 0, 1.0);
    click(&mut h, pos, Modifiers::NONE);
    assert!(h.get_by_label("Ed").accesskit_node().is_focused());
}

#[test]
fn dragging_below_the_viewport_autoscrolls_and_extends_the_selection() {
    let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
    let mut h = harness_with(&lines.join("\n"), |_| {}, 100.0);
    h.get_by_label("Ed").focus();
    h.run_ok();
    let rect = h.get_by_label("Ed").rect();
    let from = at(&h, 0, 1.0);
    press(&mut h, from, PointerButton::Primary, Modifiers::NONE);
    // 40 px below the bottom edge: three lines every tick of 100 ms.
    let below = pos2(from.x, rect.bottom() + 40.0);
    h.event(Event::PointerMoved(below));
    for _ in 0..30 {
        h.step();
    }
    let end = caret(&h);
    release(&mut h, below, PointerButton::Primary, Modifiers::NONE);
    let line = h.state().document.line_number(end);
    assert!(line > 12, "autoscroll moved the head down to line {line}");
    assert!(h.state().carets.primary().has_selection());
}

#[test]
fn the_wheel_scrolls_vertically_and_shift_wheel_horizontally() {
    let lines: Vec<String> = (0..100)
        .map(|i| format!("line {i} {}", "x".repeat(200)))
        .collect();
    let mut h = testing::harness_state(
        Theme::dark(),
        (Editor::new(&lines.join("\n")), (0usize, 0.0f32)),
        |ui, (e, report)| {
            let out = EditorComponent::new(e)
                .label("Ed")
                .viewer(true)
                .line_numbers(true)
                .height(100.0)
                .show(ui);
            *report = (out.visible_lines.start, 0.0);
        },
    );
    let rect = h.get_by_label("Ed").rect();
    h.hover_at(rect.center());
    h.run_ok();
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -3.0),
        modifiers: Modifiers::NONE,
    });
    h.run_ok();
    h.run_ok();
    assert!(
        h.state().1.0 >= 2,
        "scrolled down: first line {}",
        h.state().1.0
    );
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -3.0),
        modifiers: Modifiers::SHIFT,
    });
    h.run_ok();
    h.run_ok();
    // The horizontal scroll hides the start of the line: its number column stays.
    assert!(h.state().1.0 >= 2);
}

// ---- scrolling ------------------------------------------------------------------------

fn numbered(n: usize) -> String {
    (0..n)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn visible_of(h: &H<'_>, slot: &std::cell::Cell<(usize, usize)>) -> (usize, usize) {
    let _ = h;
    slot.get()
}

#[test]
fn ctrl_end_scrolls_the_caret_into_view_and_typing_keeps_it_visible() {
    let slot = std::cell::Cell::new((0usize, 0usize));
    let mut editor = Editor::new(&numbered(100));
    editor.settings.scroll_past_end_lines = 5;
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        let out = EditorComponent::new(e).label("Ed").height(100.0).show(ui);
        slot.set((out.visible_lines.start, out.visible_lines.end));
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, CTRL, Key::End);
    for _ in 0..10 {
        h.step();
    }
    let (first, last) = visible_of(&h, &slot);
    assert!(last >= 100, "the last line is visible: {first}..{last}");
}

#[test]
fn the_content_scrolls_five_lines_past_the_end() {
    let slot = std::cell::Cell::new((0usize, 0usize));
    let mut h = testing::harness_state(Theme::dark(), Editor::new(&numbered(100)), |ui, e| {
        let out = EditorComponent::new(e)
            .label("Ed")
            .viewer(true)
            .height(300.0)
            .show(ui);
        slot.set((out.visible_lines.start, out.visible_lines.end));
    });
    let rect = h.get_by_label("Ed").rect();
    h.hover_at(rect.center());
    for _ in 0..40 {
        h.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -10.0),
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_ok();
    let (first, last) = slot.get();
    // About 14 lines fit in 300 px: at the very end the last line sits five lines above the
    // bottom, so the view shows 14 - 5 = 9 or 10 lines of text.
    assert_eq!(last, 100, "the last line is the last visible one");
    assert!((89..=91).contains(&first), "first visible line {first}");
}

#[test]
fn a_jump_scrolls_with_an_ease_out_of_at_most_100_ms() {
    let slot = std::cell::Cell::new(0usize);
    let mut h = testing::harness_state(Theme::dark(), Editor::new(&numbered(200)), |ui, e| {
        let out = EditorComponent::new(e).label("Ed").height(100.0).show(ui);
        slot.set(out.visible_lines.start);
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, CTRL, Key::End);
    // Mid-animation the view has not arrived yet; 100 ms (6 steps of 1/60 s) later it has.
    let early = slot.get();
    for _ in 0..12 {
        h.step();
    }
    let late = slot.get();
    assert!(late >= 190, "arrived at the end: {late} (was {early})");
}

// ---- gutter ---------------------------------------------------------------------------

fn gutter_harness<'a>(
    text: &str,
    configure: impl Fn(EditorComponent<'_>) -> EditorComponent<'_> + 'a,
) -> H<'a> {
    testing::harness_state(Theme::dark(), Editor::new(text), move |ui, e| {
        configure(
            EditorComponent::new(e)
                .label("Ed")
                .line_numbers(true)
                .height(160.0),
        )
        .show(ui);
    })
}

fn gutter_pos(h: &H<'_>, row: usize, x: f32) -> Pos2 {
    let rect = h.get_by_label("Ed").rect();
    pos2(
        rect.left() - x,
        rect.top() + 4.0 + ROW * row as f32 + ROW / 2.0,
    )
}

#[test]
fn dragging_in_the_line_number_column_selects_whole_lines() {
    let mut h = gutter_harness("one\ntwo\nthree\nfour", |c| c);
    let from = gutter_pos(&h, 0, 12.0);
    let to = gutter_pos(&h, 2, 12.0);
    drag(&mut h, from, to, Modifiers::NONE, PointerButton::Primary);
    assert_eq!(selected(&h), ["one\ntwo\nthree\n"]);
}

#[test]
fn a_click_on_a_line_number_is_only_reported_in_the_new_ui() {
    let slot = std::cell::RefCell::new(Vec::<GutterClick>::new());
    let mut h = testing::harness_state(Theme::dark(), Editor::new("one\ntwo"), |ui, e| {
        let out = EditorComponent::new(e)
            .label("Ed")
            .line_numbers(true)
            .height(160.0)
            .show(ui);
        slot.borrow_mut().extend(out.gutter_clicks);
    });
    let pos = gutter_pos(&h, 1, 12.0);
    click(&mut h, pos, Modifiers::NONE);
    assert!(selected(&h).is_empty(), "the click does not select");
    let clicks = slot.borrow();
    assert_eq!(clicks.len(), 1);
    assert_eq!(
        (clicks[0].line, clicks[0].area),
        (1, GutterArea::LineNumber)
    );
}

#[test]
fn the_classic_gutter_selects_the_line_on_click() {
    let mut h = gutter_harness("one\ntwo\nthree", |c| c.gutter_click_selects_line(true));
    let pos = gutter_pos(&h, 1, 12.0);
    click(&mut h, pos, Modifiers::NONE);
    assert_eq!(selected(&h), ["two\n"]);
}

#[test]
fn shift_click_on_a_line_number_extends_by_lines() {
    let mut h = gutter_harness("one\ntwo\nthree\nfour", |c| {
        c.gutter_click_selects_line(true)
    });
    let first = gutter_pos(&h, 0, 12.0);
    click(&mut h, first, Modifiers::NONE);
    let third = gutter_pos(&h, 2, 12.0);
    click(&mut h, third, Modifiers::SHIFT);
    assert_eq!(selected(&h), ["one\ntwo\nthree\n"]);
}

#[test]
fn gutter_icons_are_buttons_and_report_clicks() {
    let icons = vec![
        GutterIcon::new(1, Glyph::Breakpoint, "Breakpoint on line 2").tone(Tone::Error),
        GutterIcon::new(2, Glyph::Bookmark, "Bookmark on line 3").lane(1),
    ];
    let slot = std::cell::RefCell::new(Vec::<GutterClick>::new());
    let mut h = testing::harness_state(Theme::dark(), Editor::new("a\nb\nc\nd"), |ui, e| {
        let out = EditorComponent::new(e)
            .label("Ed")
            .line_numbers(true)
            .gutter_icons(&icons)
            .height(160.0)
            .show(ui);
        slot.borrow_mut().extend(out.gutter_clicks);
    });
    h.get_by_label("Breakpoint on line 2").click();
    h.run_ok();
    h.get_by_label("Bookmark on line 3").click();
    h.run_ok();
    let clicks = slot.borrow();
    assert_eq!(clicks.len(), 2);
    assert_eq!((clicks[0].line, clicks[0].area), (1, GutterArea::Icon(0)));
    assert_eq!((clicks[1].line, clicks[1].area), (2, GutterArea::Icon(1)));
}

fn fold_text() -> String {
    "fn a() {\n    one;\n    two;\n}\nafter".to_owned()
}

fn folding_editor() -> Editor {
    let mut editor = Editor::new(&fold_text());
    editor
        .folding
        .add_region(&editor.document, 7, 28, "{...}", true);
    editor
}

#[test]
fn clicking_a_fold_chevron_collapses_and_expands_the_region() {
    let mut h = testing::harness_state(Theme::dark(), folding_editor(), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .line_numbers(true)
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.get_by_label("Fold lines 1 to 4").click();
    h.run_ok();
    assert!(!h.state().folding.regions()[0].expanded);
    assert!(h.query_by_label("Unfold lines 1 to 4").is_some());
    h.get_by_label("Unfold lines 1 to 4").click();
    h.run_ok();
    assert!(h.state().folding.regions()[0].expanded);
}

#[test]
fn a_caret_inside_a_region_moves_to_its_start_when_it_collapses() {
    let mut editor = folding_editor();
    editor.carets.set_caret(&editor.document, 15);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.get_by_label("Fold lines 1 to 4").click();
    h.run_ok();
    assert_eq!(h.state().carets.primary().offset, 7);
}

#[test]
fn clicking_a_collapsed_placeholder_expands_it() {
    let mut editor = folding_editor();
    editor.folding.set_expanded(0, false);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    // "fn a() {...}": the placeholder starts at column 7.
    let pos = at(&h, 0, 9.0);
    click(&mut h, pos, Modifiers::NONE);
    assert!(h.state().folding.regions()[0].expanded);
}

#[test]
fn collapsed_regions_hide_their_lines_and_keep_the_following_text() {
    let mut editor = folding_editor();
    editor.folding.set_expanded(0, false);
    let slot = std::cell::Cell::new(0..0);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        slot.set(
            EditorComponent::new(e)
                .label("Ed")
                .line_numbers(true)
                .folding(true)
                .height(160.0)
                .show(ui)
                .visible_lines,
        );
    });
    h.run_ok();
    // Lines 0 and 4 are drawn; 1..=3 hide behind the placeholder.
    assert_eq!(slot.take(), 0..5);
    assert!(h.query_by_label("Unfold lines 1 to 4").is_some());
}

#[test]
fn the_right_arrow_jumps_over_a_collapsed_region() {
    let mut editor = folding_editor();
    editor.folding.set_expanded(0, false);
    editor.carets.set_caret(&editor.document, 7);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(h.state().carets.primary().offset, 28);
}

// ---- highlights -----------------------------------------------------------------------

#[test]
fn the_stripe_reports_the_mark_under_the_pointer_and_a_click_moves_the_caret() {
    let highlights = vec![
        RangeHighlight::new(2..4, HighlightSeverity::Error).message("boom"),
        RangeHighlight::new(30..32, HighlightSeverity::Warning),
    ];
    let slot = std::cell::Cell::new(None);
    let mut h = testing::harness_state(Theme::dark(), Editor::new(&numbered(40)), |ui, e| {
        let out = EditorComponent::new(e)
            .label("Ed")
            .highlights(&highlights)
            .height(200.0)
            .show(ui);
        slot.set(out.stripe_hover);
    });
    h.run_ok();
    let rect = h.get_by_label("Ed").rect();
    // The first mark sits at the very top of the stripe.
    let pos = pos2(rect.right() + 7.0, rect.top() + 1.0 + 1.0);
    h.hover_at(pos);
    h.run_ok();
    assert_eq!(slot.get(), Some(0));
    click(&mut h, pos, Modifiers::NONE);
    h.run_ok();
    assert_eq!(h.state().carets.primary().offset, 2);
}

// ---- hover ----------------------------------------------------------------------------

#[test]
fn resting_the_pointer_reports_a_hover_after_500_ms() {
    let slot = std::cell::RefCell::new(None);
    let highlights = vec![RangeHighlight::new(0..3, HighlightSeverity::Error).message("bad")];
    let mut h = testing::harness_state(Theme::dark(), Editor::new("foo bar"), |ui, e| {
        let out = EditorComponent::new(e)
            .label("Ed")
            .highlights(&highlights)
            .height(100.0)
            .show(ui);
        *slot.borrow_mut() = out.hover;
    });
    let pos = at(&h, 0, 1.2);
    h.hover_at(pos);
    for _ in 0..10 {
        h.step();
    }
    assert!(slot.borrow().is_none(), "not yet after about 170 ms");
    for _ in 0..30 {
        h.step();
    }
    let hover = slot.borrow().clone().expect("a hover after 500 ms");
    assert_eq!(hover.offset, 1);
    assert_eq!(hover.highlight, Some(0));
}

// ---- painting inputs ------------------------------------------------------------------

#[test]
fn change_markers_and_stripe_render_without_panicking() {
    let markers = vec![
        ChangeMarker::new(0..2, ChangeKind::Added),
        ChangeMarker::new(3..4, ChangeKind::Modified),
        ChangeMarker::new(5..5, ChangeKind::Deleted),
    ];
    let mut h = testing::harness_state(Theme::light(), Editor::new(&numbered(10)), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .highlighter(&Json)
            .line_numbers(true)
            .change_markers(&markers)
            .show_whitespace(true)
            .height(100.0)
            .show(ui);
    });
    h.run_ok();
    testing::audit(&mut h).assert_clean();
}

// ---- snapshots ------------------------------------------------------------------------

/// Snapshot harnesses run at 2x: node rectangles are in physical pixels, events in points.
fn points(rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        pos2(rect.min.x / 2.0, rect.min.y / 2.0),
        pos2(rect.max.x / 2.0, rect.max.y / 2.0),
    )
}

/// The snapshot helpers run the same app in both themes: each theme gets its own editor.
fn theme_index(ui: &egui::Ui) -> usize {
    usize::from(!ui.visuals().dark_mode)
}

const SAMPLE: &str = "function total(items) {\n  let sum = 0;\n  for (const item of items) {\n    if (item.price > 0) {\n      sum += item.price * item.qty;\n    }\n  }\n  // TODO: tax\n  return sum;\n}\n";

fn show_sample<'a>(editor: &'a mut Editor) -> EditorComponent<'a> {
    EditorComponent::new(editor)
        .label("Ed")
        .highlighter(&archipel_kit::editor::JavaScript)
        .line_numbers(true)
        .height(236.0)
}

#[test]
fn multi_caret_snapshot() {
    let mut editors = [Editor::new(SAMPLE), Editor::new(SAMPLE)];
    testing::snapshot_themes_after(
        "editor_multi_caret",
        common::IMG,
        |ui| {
            show_sample(&mut editors[theme_index(ui)]).show(ui);
        },
        |h| {
            h.get_by_label("Ed").focus();
            h.run_ok();
            let rect = points(h.get_by_label("Ed").rect());
            // Double-click `item` on line 3, then Alt+J twice adds the next two occurrences.
            let pos = pos2(
                rect.left() + PAD + 15.0 * CELL,
                rect.top() + 4.0 + ROW * 2.5,
            );
            h.hover_at(pos);
            for _ in 0..2 {
                h.event(Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                });
                h.event(Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                });
            }
            h.run_ok();
            h.key_press_modifiers(Modifiers::ALT, Key::J);
            h.key_press_modifiers(Modifiers::ALT, Key::J);
            h.run_ok();
        },
    );
}

#[test]
fn column_selection_snapshot() {
    let mut editors = [Editor::new(SAMPLE), Editor::new(SAMPLE)];
    testing::snapshot_themes_after(
        "editor_column_selection",
        common::IMG,
        |ui| {
            show_sample(&mut editors[theme_index(ui)]).show(ui);
        },
        |h| {
            h.get_by_label("Ed").focus();
            h.run_ok();
            let rect = points(h.get_by_label("Ed").rect());
            let from = pos2(rect.left() + PAD + 2.0 * CELL, rect.top() + 4.0 + ROW * 1.5);
            let to = pos2(rect.left() + PAD + 9.0 * CELL, rect.top() + 4.0 + ROW * 4.5);
            h.hover_at(from);
            h.event(Event::PointerButton {
                pos: from,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::ALT,
            });
            h.run_ok();
            h.event(Event::PointerMoved(to));
            h.run_ok();
            h.event(Event::PointerButton {
                pos: to,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::ALT,
            });
            h.run_ok();
        },
    );
}

#[test]
fn selection_and_unfocused_snapshot() {
    let mut editor = Editor::new(SAMPLE);
    let doc = editor.document.clone();
    editor.carets.set_selection(&doc, 40, 120);
    testing::snapshot_themes("editor_selection_inactive", common::IMG, |ui| {
        show_sample(&mut editor).show(ui);
    });
}

#[test]
fn fold_and_gutter_snapshot() {
    let text = "fn total(items) {\n  let sum = 0;\n  for item in items {\n    sum += item;\n  }\n  return sum;\n}\n";
    let mut editor = Editor::new(text);
    let icons = vec![
        GutterIcon::new(1, Glyph::Breakpoint, "Breakpoint on line 2").tone(Tone::Error),
        GutterIcon::new(5, Glyph::Bookmark, "Bookmark on line 6").lane(1),
    ];
    let markers = vec![
        ChangeMarker::new(1..2, ChangeKind::Added),
        ChangeMarker::new(5..6, ChangeKind::Modified),
        ChangeMarker::new(6..6, ChangeKind::Deleted),
    ];
    // The function body and the `for` block (collapsed).
    let doc = editor.document.clone();
    let outer_open = text.find('{').expect("a brace");
    let outer_close = text.rfind('}').expect("a brace") + 1;
    let inner_open = text[outer_open + 1..].find('{').expect("a brace") + outer_open + 1;
    let inner_close = text.find("  }\n").expect("a brace") + 3;
    editor
        .folding
        .add_region(&doc, outer_open, outer_close, "{...}", true);
    editor
        .folding
        .add_region(&doc, inner_open, inner_close, "{...}", false);
    testing::snapshot_themes_after(
        "editor_gutter",
        common::IMG,
        |ui| {
            EditorComponent::new(&mut editor)
                .label("Ed")
                .highlighter(&archipel_kit::editor::JavaScript)
                .line_numbers(true)
                .folding(true)
                .gutter_icons(&icons)
                .change_markers(&markers)
                .height(180.0)
                .show(ui);
        },
        |h| {
            let rect = points(h.get_by_label("Ed").rect());
            // The pointer over the gutter shows the chevrons of the expanded regions.
            h.hover_at(pos2(rect.left() - 20.0, rect.top() + 100.0));
            for _ in 0..12 {
                h.step();
            }
        },
    );
}

#[test]
fn error_stripe_and_squiggles_snapshot() {
    let mut editor = Editor::new(SAMPLE);
    let highlights = vec![
        RangeHighlight::new(33..36, HighlightSeverity::Error).message("Unexpected token"),
        RangeHighlight::new(100..104, HighlightSeverity::Warning).message("Unused"),
        RangeHighlight::new(150..156, HighlightSeverity::WeakWarning),
        RangeHighlight::new(180..185, HighlightSeverity::TextSearch),
    ];
    testing::snapshot_themes("editor_error_stripe", common::IMG, |ui| {
        show_sample(&mut editor).highlights(&highlights).show(ui);
    });
}

#[test]
fn guides_whitespace_and_margin_snapshot() {
    let text = "fn main() {\n\tif x {\n\t\tdo_it();   \n\n\t}\n    return;\n}\n";
    let mut editors = [Editor::new(text), Editor::new(text)];
    for editor in &mut editors {
        editor.settings.right_margin = 24;
    }
    testing::snapshot_themes_after(
        "editor_guides",
        common::IMG,
        |ui| {
            EditorComponent::new(&mut editors[theme_index(ui)])
                .label("Ed")
                .highlighter(&archipel_kit::editor::JavaScript)
                .show_whitespace(true)
                .height(160.0)
                .show(ui);
        },
        |h| {
            h.get_by_label("Ed").focus();
            h.run_ok();
            h.key_press(Key::ArrowDown);
            h.key_press(Key::ArrowDown);
            h.run_ok();
        },
    );
}

// ---- painting, checked on rendered pixels -----------------------------------------------

fn pixel(h: &mut H<'_>, pos: Pos2) -> egui::Color32 {
    let image = h.render().expect("the harness renders");
    let p = image.get_pixel(pos.x as u32, pos.y as u32).0;
    egui::Color32::from_rgb(p[0], p[1], p[2])
}

fn near(a: egui::Color32, b: egui::Color32) -> bool {
    let d = |x: u8, y: u8| x.abs_diff(y);
    d(a.r(), b.r()) <= 3 && d(a.g(), b.g()) <= 3 && d(a.b(), b.b()) <= 3
}

#[track_caller]
fn assert_color(actual: egui::Color32, expected: egui::Color32, what: &str) {
    assert!(
        near(actual, expected),
        "{what}: {actual:?}, expected {expected:?}"
    );
}

fn colors() -> archipel_kit::theme::EditorColors {
    Theme::dark().editor
}

/// How many pixels of the 16 px left of the text area, around the first row, differ from the
/// editor background.
fn chevron_pixels(h: &mut H<'_>, row: usize) -> usize {
    let rect = h.get_by_label("Ed").rect();
    let image = h.render().expect("renders");
    let y = (rect.top() + 4.0 + ROW * row as f32 + 10.0) as u32;
    let mut differing = 0;
    for yy in y - 6..y + 6 {
        for xx in rect.left() as u32 - 16..rect.left() as u32 {
            let p = image.get_pixel(xx, yy).0;
            if !near(
                egui::Color32::from_rgb(p[0], p[1], p[2]),
                colors().background,
            ) {
                differing += 1;
            }
        }
    }
    differing
}

#[test]
fn a_selected_line_break_is_drawn_as_a_one_cell_tail() {
    let mut h = focused("ab\ncd");
    key(&mut h, CTRL, Key::A);
    let c = colors();
    let top = at(&h, 0, 0.0).y - 6.0;
    let (tail, past) = (at(&h, 0, 2.5).x, at(&h, 0, 4.5).x);
    // `ab` is two cells; the tail is a third.
    assert_color(
        pixel(&mut h, pos2(tail, top)),
        c.selection,
        "the tail of line 1",
    );
    assert_color(
        pixel(&mut h, pos2(past, top)),
        c.background,
        "past the tail",
    );
    // The last line has no break to show.
    let last = pos2(tail, at(&h, 1, 0.0).y - 6.0);
    assert_color(
        pixel(&mut h, last),
        c.caret_row,
        "no tail after the last line",
    );
}

#[test]
fn the_selection_is_dimmer_without_the_focus() {
    let mut h = harness("hello world");
    let doc = h.state().document.clone();
    h.state_mut().carets.set_selection(&doc, 0, 5);
    h.run_ok();
    let c = colors();
    let inside = pos2(at(&h, 0, 2.0).x + 3.0, at(&h, 0, 0.0).y - 9.0);
    assert_color(
        pixel(&mut h, inside),
        c.selection_inactive,
        "inactive selection",
    );
    h.get_by_label("Ed").focus();
    h.run_ok();
    assert_color(pixel(&mut h, inside), c.selection, "active selection");
}

#[test]
fn the_caret_row_is_highlighted_without_the_focus() {
    let mut h = harness("one\ntwo\nthree");
    let c = colors();
    let other = pos2(at(&h, 1, 20.0).x, at(&h, 1, 0.0).y);
    assert_color(pixel(&mut h, other), c.background, "another row");
    let caret_row = pos2(at(&h, 0, 20.0).x, at(&h, 0, 0.0).y);
    assert_color(pixel(&mut h, caret_row), c.caret_row, "the caret row");
}

#[test]
fn the_caret_is_a_two_pixel_bar_that_blinks_and_restarts_on_activity() {
    let mut h = focused("abc");
    key(&mut h, Modifiers::NONE, Key::ArrowRight);
    let c = colors();
    // The bar starts at the cell boundary and is 2 px wide.
    let y = at(&h, 0, 0.0).y;
    let bar = |h: &mut H<'_>, col: f32, dx: f32| pixel(h, pos2(at(h, 0, col).x + dx, y));
    assert_color(bar(&mut h, 1.0, 0.5), c.caret, "inside the bar");
    assert!(
        !near(bar(&mut h, 1.0, 3.5), c.caret),
        "the bar is narrower than 3 px"
    );
    // After 500 ms without activity it is hidden, after 1 s visible again.
    for _ in 0..35 {
        h.step();
    }
    assert!(
        !near(bar(&mut h, 1.0, 0.5), c.caret),
        "hidden in the second half"
    );
    for _ in 0..30 {
        h.step();
    }
    assert_color(bar(&mut h, 1.0, 0.5), c.caret, "visible again");
    // Typing shows it at once, whatever the phase.
    for _ in 0..35 {
        h.step();
    }
    h.get_by_label("Ed").type_text("x");
    h.step();
    h.step();
    assert_color(bar(&mut h, 2.0, 0.5), c.caret, "restarted by typing");
}

#[test]
fn overwrite_mode_draws_a_block_caret() {
    let mut h = focused("abc");
    key(&mut h, Modifiers::NONE, Key::Insert);
    let c = colors();
    let probe = pos2(at(&h, 0, 0.0).x + 3.0, at(&h, 0, 0.0).y - 9.0);
    assert_color(pixel(&mut h, probe), c.caret, "inside the block");
}

#[test]
fn a_matched_brace_pair_is_highlighted_when_the_editor_has_the_focus() {
    let mut h = focused("(a)");
    set_caret(&mut h, 1);
    let c = colors();
    // The left edge of the first and the last cell, above the glyphs.
    let y = at(&h, 0, 0.0).y - 9.0;
    let open = pos2(at(&h, 0, 0.0).x + 3.0, y);
    let close = pos2(at(&h, 0, 2.0).x + 3.0, y);
    let middle = pos2(at(&h, 0, 1.0).x + 3.0, y);
    assert_color(pixel(&mut h, open), c.matched_brace, "the opener");
    assert_color(pixel(&mut h, close), c.matched_brace, "the closer");
    assert_color(pixel(&mut h, middle), c.caret_row, "between them");
}

#[test]
fn other_occurrences_of_the_selected_text_are_highlighted() {
    let mut h = focused("foo bar foo");
    let doc = h.state().document.clone();
    h.state_mut().carets.set_selection(&doc, 0, 3);
    h.run_ok();
    let c = colors();
    let y = at(&h, 0, 0.0).y - 9.0;
    let second = pos2(at(&h, 0, 8.0).x + 3.0, y);
    assert_color(pixel(&mut h, second), c.rename_occurrence, "the second foo");
    let bar = pos2(at(&h, 0, 4.0).x + 3.0, y);
    assert_color(pixel(&mut h, bar), c.caret_row, "bar");
}

#[test]
fn a_collapsed_region_is_a_pill_with_a_two_pixel_border() {
    let text = "a {\n  b\n}\nc";
    let mut editor = Editor::new(text);
    let doc = editor.document.clone();
    editor.folding.add_region(&doc, 2, 9, "{...}", false);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .folding(true)
            .height(100.0)
            .show(ui);
    });
    let c = colors();
    let top = at(&h, 0, 0.0).y - ROW / 2.0 + 0.5;
    let left = at(&h, 0, 2.0).x;
    assert_color(
        pixel(&mut h, pos2(left + 0.5, top)),
        c.folded_border,
        "the ring",
    );
    assert_color(
        pixel(&mut h, pos2(left + 3.0, top + 3.0)),
        c.folded_background,
        "inside the ring",
    );
}

#[test]
fn error_stripe_marks_use_the_severity_colors() {
    let highlights = vec![RangeHighlight::new(0..2, HighlightSeverity::Error)];
    let mut h = testing::harness_state(Theme::dark(), Editor::new(&numbered(30)), |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .highlights(&highlights)
            .height(200.0)
            .show(ui);
    });
    let rect = h.get_by_label("Ed").rect();
    let mark = pos2(rect.right() + 7.0, rect.top() + 1.5);
    assert_color(
        pixel(&mut h, mark),
        colors().error_stripe,
        "the stripe mark",
    );
}

#[test]
fn the_right_margin_and_indent_guides_are_one_pixel_lines() {
    let mut h = harness_with("a\n    b\n    c", |e| e.settings.right_margin = 10, 100.0);
    let c = colors();
    let y = at(&h, 3, 0.0).y;
    let margin_x = at(&h, 0, 10.0).x;
    let found = (-1..=1).any(|d| near(pixel(&mut h, pos2(margin_x + d as f32, y)), c.right_margin));
    assert!(found, "the margin line near x = {margin_x}");
    // A guide at column 0 on the indented lines.
    let guide = at(&h, 1, 0.0);
    // The caret is on the header of the block, so its guide is the stronger one.
    let found = (-1..=1).any(|d| {
        near(
            pixel(&mut h, pos2(guide.x + d as f32, guide.y)),
            c.indent_guide_selected,
        )
    });
    assert!(found, "the selected indent guide at column 0");
}

#[test]
fn fold_chevrons_show_on_gutter_hover_and_collapsed_ones_always() {
    let mut editor = folding_editor();
    let doc = editor.document.clone();
    editor.folding.add_region(&doc, 0, 28, "{...}", true);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .line_numbers(true)
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.run_ok();
    let rect = h.get_by_label("Ed").rect();
    assert_eq!(
        chevron_pixels(&mut h, 0),
        0,
        "hidden while the pointer is away"
    );
    h.hover_at(pos2(rect.left() - 30.0, rect.top() + 60.0));
    for _ in 0..12 {
        h.step();
    }
    assert!(
        chevron_pixels(&mut h, 0) > 3,
        "shown after the 130 ms fade-in"
    );
    h.hover_at(pos2(rect.center().x, rect.top() + 60.0));
    for _ in 0..30 {
        h.step();
    }
    assert_eq!(chevron_pixels(&mut h, 0), 0, "gone after the fade-out");
}

#[test]
fn a_collapsed_chevron_stays_visible_without_hover() {
    let mut editor = folding_editor();
    editor.folding.set_expanded(0, false);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .line_numbers(true)
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.run_ok();
    assert!(
        chevron_pixels(&mut h, 0) > 3,
        "the collapsed chevron is drawn"
    );
}

// ---- more rules -----------------------------------------------------------------------

#[test]
fn ctrl_minus_and_ctrl_equals_fold_and_unfold_the_region_at_the_caret() {
    let mut editor = folding_editor();
    editor.carets.set_caret(&editor.document, 15);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        EditorComponent::new(e)
            .label("Ed")
            .folding(true)
            .height(160.0)
            .show(ui);
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    key(&mut h, CTRL, Key::Minus);
    assert!(!h.state().folding.regions()[0].expanded);
    assert_eq!(
        h.state().carets.primary().offset,
        7,
        "the caret leaves the fold"
    );
    key(&mut h, CTRL, Key::Equals);
    assert!(h.state().folding.regions()[0].expanded);
}

#[test]
fn moving_the_caret_down_keeps_a_line_of_margin_below_it() {
    let slot = std::cell::Cell::new(0usize);
    let mut h = testing::harness_state(Theme::dark(), Editor::new(&numbered(50)), |ui, e| {
        slot.set(
            EditorComponent::new(e)
                .label("Ed")
                .height(100.0)
                .show(ui)
                .visible_lines
                .start,
        );
    });
    h.get_by_label("Ed").focus();
    h.run_ok();
    // Four lines and a bit fit: the caret on line 4 needs line 5 visible below it.
    for _ in 0..5 {
        key(&mut h, Modifiers::NONE, Key::ArrowDown);
    }
    for _ in 0..8 {
        h.step();
    }
    assert!(
        slot.get() >= 1,
        "scrolled for a margin: first line {}",
        slot.get()
    );
}

#[test]
fn a_middle_click_pastes_the_primary_selection_on_linux_and_navigates_on_macos() {
    let mut h = focused("hello world");
    let from = at(&h, 0, 0.0);
    let to = at(&h, 0, 5.0);
    drag(&mut h, from, to, Modifiers::NONE, PointerButton::Primary);
    assert_eq!(selected(&h), ["hello"]);
    let at_end = at(&h, 0, 11.0);
    press(&mut h, at_end, PointerButton::Middle, Modifiers::NONE);
    release(&mut h, at_end, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(text(&h), "hello worldhello");

    let slot = std::cell::Cell::new(None);
    let mut mac = testing::harness_state(Theme::dark(), Editor::new("hello world"), |ui, e| {
        slot.set(
            EditorComponent::new(e)
                .label("Ed")
                .keymap_os(Os::MacOs)
                .height(100.0)
                .show(ui)
                .navigation
                .or(slot.get()),
        );
    });
    mac.get_by_label("Ed").focus();
    mac.run_ok();
    let pos = at(&mac, 0, 3.0);
    press(&mut mac, pos, PointerButton::Middle, Modifiers::NONE);
    release(&mut mac, pos, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(
        slot.get(),
        Some(NavigationRequest {
            offset: 3,
            kind: NavigationKind::Declaration
        })
    );
}

#[test]
fn the_pointer_resting_on_a_fold_placeholder_reports_a_preview_after_300_ms() {
    let mut editor = folding_editor();
    editor.folding.set_expanded(0, false);
    let slot = std::cell::RefCell::new(None);
    let mut h = testing::harness_state(Theme::dark(), editor, |ui, e| {
        *slot.borrow_mut() = EditorComponent::new(e)
            .label("Ed")
            .folding(true)
            .height(160.0)
            .show(ui)
            .fold_preview;
    });
    let pos = at(&h, 0, 9.0);
    h.hover_at(pos);
    for _ in 0..8 {
        h.step();
    }
    assert!(slot.borrow().is_none(), "not after 130 ms");
    for _ in 0..20 {
        h.step();
    }
    assert_eq!(slot.borrow().clone(), Some(7..28));
}

#[test]
fn each_arrow_press_moves_the_caret_once() {
    let mut h = focused("a\nb\nc\nd\ne\nf");
    h.key_press(Key::ArrowDown);
    h.key_press(Key::ArrowDown);
    h.run_ok();
    assert_eq!(h.state().document.line_number(caret(&h)), 2);
}
