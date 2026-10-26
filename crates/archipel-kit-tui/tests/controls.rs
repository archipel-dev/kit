//! Event-driven tests of the basic controls: keys and the mouse against an in-memory screen.

#![allow(clippy::unnecessary_mut_passed)]

use archipel_kit::ui::Status;
use archipel_kit_tui::components::{Pane, PaneKind};
use archipel_kit_tui::controls::*;
use archipel_kit_tui::input::{KeyCode, KeyModifiers, key, key_with};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{FnApp, Harness, app_fn, snapshot};
use archipel_kit_tui::{Event, Id, Look, Runtime};

const OK: Id = Id::new("ok");
const CHECK: Id = Id::new("check");
const RADIO: Id = Id::new("radio");
const SWITCH: Id = Id::new("switch");
const SEG: Id = Id::new("seg");
const NAME: Id = Id::new("name");
const SEARCH: Id = Id::new("search");
const BANNER: Id = Id::new("banner");

#[derive(Default)]
struct Form {
    button: ButtonState,
    clicks: u32,
    check: CheckState,
    tri: bool,
    radio: usize,
    switch: bool,
    seg: usize,
    name: TextFieldState,
    search: SearchFieldState,
    last: Option<FieldEvent>,
    banner_action: Option<usize>,
}

const RADIOS: [&str; 3] = ["Spaces", "Tabs", "Mixed"];
const SEGS: [&str; 3] = ["Tree", "List", "Graph"];

fn event(f: &mut Form, rt: &mut Runtime, e: &Event) {
    if ButtonControl::new(OK, "Apply")
        .primary()
        .key('a')
        .handle(rt, &mut f.button, e)
    {
        f.clicks += 1;
    }
    CheckBoxControl::new(CHECK, "Format on save")
        .tri_state(f.tri)
        .handle(rt, &mut f.check, e);
    RadioGroupControl::new(RADIO, &RADIOS).handle(rt, &mut f.radio, e);
    if OnOffControl::new(SWITCH, "Wrap").handle(rt, e) {
        f.switch = !f.switch;
    }
    SegmentedControl::new(SEG, &SEGS).handle(rt, &mut f.seg, e);
    let r = TextFieldControl::new(NAME)
        .label("Name")
        .handle(rt, &mut f.name, e);
    if r != FieldEvent::Ignored {
        f.last = Some(r);
    }
    let r = TextFieldControl::search(SEARCH).handle(rt, &mut f.search.field, e);
    if r != FieldEvent::Ignored {
        f.last = Some(r);
    }
    f.banner_action = BannerControl::new(
        BANNER,
        Status::Warning,
        "Low disk space",
        &["Clean", "Hide"],
    )
    .handle(rt, e)
    .or(f.banner_action);
    if let Event::Key(k) = e
        && k.code == KeyCode::Tab
    {
        if k.modifiers.contains(KeyModifiers::SHIFT) {
            rt.focus.previous();
        } else {
            rt.focus.next();
        }
    }
    if let Event::Key(k) = e
        && k.code == KeyCode::BackTab
    {
        rt.focus.previous();
    }
}

fn draw(f: &mut Form, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    Pane::new(&rt.look).kind(PaneKind::Editor).render(area, buf);
    ButtonControl::new(OK, "Apply")
        .primary()
        .key('a')
        .show(rt, &f.button, 1, 0, buf);
    CheckBoxControl::new(CHECK, "Format on save")
        .tri_state(f.tri)
        .show(rt, f.check, 1, 2, buf);
    RadioGroupControl::new(RADIO, &RADIOS).show(rt, f.radio, 1, 4, buf);
    OnOffControl::new(SWITCH, "Wrap").show(rt, f.switch, 1, 8, buf);
    SegmentedControl::new(SEG, &SEGS).show(rt, f.seg, 1, 10, buf);
    TextFieldControl::new(NAME)
        .label("Name")
        .placeholder("project name")
        .show(rt, &f.name, Rect::new(1, 12, 40, 1), buf);
    TextFieldControl::search(SEARCH).placeholder("Search").show(
        rt,
        &f.search.field,
        Rect::new(1, 14, 40, 1),
        buf,
    );
    BannerControl::new(
        BANNER,
        Status::Warning,
        "Low disk space",
        &["Clean", "Hide"],
    )
    .show(rt, Rect::new(0, 16, 60, 1), buf);
}

type EventFn = fn(&mut Form, &mut Runtime, &Event);
type DrawFn = fn(&mut Form, &mut Runtime, Rect, &mut Buffer);
type FormApp = FnApp<Form, EventFn, DrawFn>;

fn form_app() -> FormApp {
    app_fn(Form::default(), event as EventFn, draw as DrawFn)
}

fn harness() -> Harness<FormApp> {
    Harness::new(form_app(), 60, 18)
}

// The harness owns the app; tests read the form through this accessor.
macro_rules! form {
    ($h:expr) => {
        &mut $h.app.state
    };
}

#[test]
fn enter_presses_the_focused_button() {
    let mut h = harness();
    h.rt.focus.focus(OK);
    h.draw();
    h.press(KeyCode::Enter);
    assert_eq!(form!(h).clicks, 1);
}

#[test]
fn space_presses_the_focused_button_but_not_an_unfocused_one() {
    let mut h = harness();
    h.press(KeyCode::Char(' '));
    assert_eq!(form!(h).clicks, 0);
    h.rt.focus.focus(OK);
    h.press(KeyCode::Char(' '));
    assert_eq!(form!(h).clicks, 1);
}

#[test]
fn alt_plus_the_mnemonic_presses_a_button() {
    let mut h = harness();
    h.alt('a');
    assert_eq!(form!(h).clicks, 1);
}

#[test]
fn a_click_on_a_button_completes_on_release_inside() {
    let mut h = harness();
    h.click(3, 0);
    assert_eq!(form!(h).clicks, 1);
    assert!(h.rt.focused(OK), "focus follows the click");
}

#[test]
fn pressing_a_button_and_releasing_outside_does_nothing() {
    let mut h = harness();
    h.drag((3, 0), (30, 6));
    assert_eq!(form!(h).clicks, 0);
    assert!(!form!(h).button.pressed);
}

#[test]
fn the_button_shows_pressed_while_the_mouse_is_down_on_it() {
    let mut h = harness();
    let before = h.buffer()[(3, 0)].bg;
    h.send(Event::Mouse(archipel_kit_tui::input::MouseEvent {
        kind: archipel_kit_tui::input::MouseEventKind::Down(
            archipel_kit_tui::input::MouseButton::Left,
        ),
        column: 3,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }));
    assert!(form!(h).button.pressed);
    assert_ne!(h.buffer()[(3, 0)].bg, before, "a darker fill while pressed");
}

#[test]
fn space_toggles_a_focused_check_box() {
    let mut h = harness();
    h.rt.focus.focus(CHECK);
    h.press(KeyCode::Char(' '));
    assert_eq!(form!(h).check, CheckState::Checked);
    h.press(KeyCode::Char(' '));
    assert_eq!(form!(h).check, CheckState::Unchecked);
}

#[test]
fn a_tri_state_check_box_cycles_through_mixed() {
    let mut h = harness();
    form!(h).tri = true;
    h.rt.focus.focus(CHECK);
    for expected in [
        CheckState::Checked,
        CheckState::Mixed,
        CheckState::Unchecked,
    ] {
        h.press(KeyCode::Char(' '));
        assert_eq!(form!(h).check, expected);
    }
}

#[test]
fn clicking_the_label_of_a_check_box_toggles_it() {
    let mut h = harness();
    h.click(8, 2);
    assert_eq!(form!(h).check, CheckState::Checked);
}

#[test]
fn arrow_keys_move_the_radio_selection_and_wrap() {
    let mut h = harness();
    h.rt.focus.focus(RADIO);
    h.press(KeyCode::Down);
    assert_eq!(form!(h).radio, 1);
    h.press(KeyCode::Down);
    h.press(KeyCode::Down);
    assert_eq!(form!(h).radio, 0, "wraps");
    h.press(KeyCode::Up);
    assert_eq!(form!(h).radio, 2);
}

#[test]
fn clicking_a_radio_button_selects_it() {
    let mut h = harness();
    h.click(6, 5);
    assert_eq!(form!(h).radio, 1);
    assert!(h.rt.focused(RADIO));
}

#[test]
fn clicking_a_switch_toggles_it() {
    let mut h = harness();
    h.click(5, 8);
    assert!(form!(h).switch);
}

#[test]
fn left_and_right_move_the_segmented_selection_without_wrapping() {
    let mut h = harness();
    h.rt.focus.focus(SEG);
    h.press(KeyCode::Right);
    h.press(KeyCode::Right);
    h.press(KeyCode::Right);
    assert_eq!(form!(h).seg, 2);
    h.press(KeyCode::Home);
    assert_eq!(form!(h).seg, 0);
    h.press(KeyCode::Left);
    assert_eq!(form!(h).seg, 0);
}

#[test]
fn clicking_a_segment_selects_it() {
    let mut h = harness();
    let (x, y) = h.find("Graph").expect("segment");
    h.click(x + 1, y);
    assert_eq!(form!(h).seg, 2);
}

#[test]
fn tab_and_shift_tab_walk_the_focus_order() {
    let mut h = harness();
    h.press(KeyCode::Tab);
    assert!(h.rt.focused(OK));
    h.press(KeyCode::Tab);
    assert!(h.rt.focused(CHECK));
    h.press(KeyCode::BackTab);
    assert!(h.rt.focused(OK));
    h.press(KeyCode::BackTab);
    assert!(h.rt.focused(BANNER.with(1)), "wraps to the last widget");
}

#[test]
fn typing_inserts_text_at_the_caret() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("archipel");
    assert_eq!(form!(h).name.text(), "archipel");
    h.press(KeyCode::Left);
    h.press(KeyCode::Left);
    h.type_text("-");
    assert_eq!(form!(h).name.text(), "archip-el");
}

#[test]
fn unfocused_fields_ignore_typing() {
    let mut h = harness();
    h.type_text("abc");
    assert_eq!(form!(h).name.text(), "");
}

#[test]
fn backspace_and_delete_remove_one_character() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("abcd");
    h.press(KeyCode::Backspace);
    assert_eq!(form!(h).name.text(), "abc");
    h.press(KeyCode::Home);
    h.press(KeyCode::Delete);
    assert_eq!(form!(h).name.text(), "bc");
}

#[test]
fn ctrl_backspace_deletes_a_word() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("one two");
    h.press_with(KeyCode::Backspace, KeyModifiers::CONTROL);
    assert_eq!(form!(h).name.text(), "one ");
}

#[test]
fn shift_arrows_select_and_typing_replaces_the_selection() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("hello");
    for _ in 0..3 {
        h.press_with(KeyCode::Left, KeyModifiers::SHIFT);
    }
    assert_eq!(form!(h).name.selected_text(), "llo");
    h.type_text("p");
    assert_eq!(form!(h).name.text(), "hep");
}

#[test]
fn ctrl_a_selects_all_and_ctrl_c_ctrl_v_copy_and_paste() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("copy me");
    h.ctrl('a');
    assert_eq!(form!(h).name.selected_text(), "copy me");
    h.ctrl('c');
    assert_eq!(h.rt.clipboard.get(), "copy me");
    assert!(h.rt.clipboard.take_sequence().is_some(), "OSC 52 is queued");
    h.press(KeyCode::End);
    h.ctrl('v');
    assert_eq!(form!(h).name.text(), "copy mecopy me");
}

#[test]
fn ctrl_x_cuts_the_selection() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("cut this");
    h.ctrl('a');
    h.ctrl('x');
    assert_eq!(form!(h).name.text(), "");
    assert_eq!(h.rt.clipboard.get(), "cut this");
}

#[test]
fn ctrl_z_undoes_and_redo_restores() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("ab");
    h.ctrl('z');
    h.ctrl('z');
    assert_eq!(form!(h).name.text(), "");
    h.press_with(
        KeyCode::Char('Z'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(form!(h).name.text(), "a");
}

#[test]
fn ctrl_arrows_move_by_words() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("alpha beta gamma");
    h.press_with(KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(form!(h).name.caret(), 11);
    h.press_with(KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(form!(h).name.caret(), 6);
    h.press_with(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(form!(h).name.caret(), 10);
}

#[test]
fn a_paste_event_inserts_the_text_and_flattens_newlines() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.send(Event::Paste("a\nb".into()));
    assert_eq!(form!(h).name.text(), "a b");
}

#[test]
fn enter_submits_a_field() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.press(KeyCode::Enter);
    assert_eq!(form!(h).last, Some(FieldEvent::Submitted));
}

#[test]
fn escape_in_a_plain_field_is_left_to_the_caller() {
    let mut h = harness();
    h.rt.focus.focus(NAME);
    h.type_text("x");
    h.press(KeyCode::Esc);
    assert_eq!(form!(h).last, Some(FieldEvent::Cancelled));
    assert_eq!(form!(h).name.text(), "x");
}

#[test]
fn escape_clears_a_search_field_first() {
    let mut h = harness();
    h.rt.focus.focus(SEARCH);
    h.type_text("query");
    h.press(KeyCode::Esc);
    assert_eq!(form!(h).search.field.text(), "");
    assert_eq!(form!(h).last, Some(FieldEvent::Cleared));
    h.press(KeyCode::Esc);
    assert_eq!(form!(h).last, Some(FieldEvent::Cancelled));
}

#[test]
fn the_clear_button_empties_a_search_field() {
    let mut h = harness();
    h.rt.focus.focus(SEARCH);
    h.type_text("query");
    let (x, y) = (39, 14);
    h.click(x, y);
    assert_eq!(form!(h).search.field.text(), "");
    assert_eq!(form!(h).last, Some(FieldEvent::Cleared));
}

#[test]
fn clicking_a_field_focuses_it_and_places_the_caret() {
    let mut h = harness();
    form!(h).name.set_text("abcdef");
    h.draw();
    // "Name" block starts at x=2: text starts at 1 + 6 + 1 + 2.
    h.click(11, 12);
    assert!(h.rt.focused(NAME));
    assert_eq!(form!(h).name.caret(), 2);
}

#[test]
fn double_click_selects_a_word_and_triple_click_everything() {
    let mut h = harness();
    form!(h).name.set_text("alpha beta");
    h.draw();
    h.double_click(11, 12);
    assert_eq!(form!(h).name.selected_text(), "alpha");
    h.click(11, 12);
    // Third press at the same cell (the tracker counts without time passing).
    assert_eq!(form!(h).name.selected_text(), "alpha beta");
}

#[test]
fn dragging_in_a_field_selects_text() {
    let mut h = harness();
    form!(h).name.set_text("abcdefgh");
    h.draw();
    h.drag((11, 12), (14, 12));
    assert_eq!(form!(h).name.selected_text(), "cde");
}

#[test]
fn search_history_keeps_five_unique_entries_newest_first_and_cycles() {
    let mut s = SearchFieldState::new();
    for q in ["a", "b", "c", "d", "e", "f", "c"] {
        s.field.set_text(q);
        s.commit();
    }
    assert_eq!(s.history(), ["c", "f", "e", "d", "b"]);
    s.cycle(true);
    assert_eq!(s.field.text(), "c");
    s.cycle(true);
    assert_eq!(s.field.text(), "f");
    s.cycle(false);
    s.cycle(false);
    assert_eq!(s.field.text(), "b", "wraps");
}

#[test]
fn clicking_a_banner_action_reports_its_index() {
    let mut h = harness();
    let (x, y) = h.find("Hide").expect("action");
    h.click(x, y);
    assert_eq!(form!(h).banner_action, Some(1));
}

#[test]
fn enter_on_a_focused_banner_action_performs_it() {
    let mut h = harness();
    h.rt.focus.focus(BANNER.with(0));
    h.press(KeyCode::Enter);
    assert_eq!(form!(h).banner_action, Some(0));
}

#[test]
fn the_form_survives_tiny_screens() {
    for (w, hgt) in [(1, 1), (3, 2), (10, 5), (40, 3)] {
        let mut h = Harness::new(form_app(), w, hgt);
        h.press(KeyCode::Tab);
        h.type_text("x");
        h.click(0, 0);
        h.drag((0, 0), (w - 1, hgt - 1));
        h.press_with(KeyCode::Char('a'), KeyModifiers::ALT);
        h.resize(w.max(2), hgt.max(2));
    }
    let _ = key(KeyCode::Esc);
    let _ = key_with(KeyCode::Esc, KeyModifiers::NONE);
}

#[test]
fn the_form_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let mut h = Harness::with_runtime(form_app(), Runtime::new(look), 60, 18);
        form!(h).name.set_text("archipel");
        form!(h).check = CheckState::Checked;
        form!(h).radio = 1;
        form!(h).switch = true;
        form!(h).seg = 1;
        form!(h).search.field.set_text("find me");
        h.rt.focus.focus(NAME);
        h.draw();
        snapshot(&format!("controls.{name}"), h.buffer());
    }
}
