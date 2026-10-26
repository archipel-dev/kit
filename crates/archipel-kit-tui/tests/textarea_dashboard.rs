//! Tests of the read-only text area and the dashboard.

#![allow(clippy::unnecessary_mut_passed)]

use archipel_kit_tui::components::{Pane, PaneKind};
use archipel_kit_tui::dashboard::{Dashboard, DashboardButton};
use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{FnApp, Harness, app_fn};
use archipel_kit_tui::textarea::{TextArea, TextAreaState, TextLanguage};
use archipel_kit_tui::{Event, Glyph, Id, Runtime};

const AREA: Id = Id::new("area");
const DASH: Id = Id::new("dash");

struct Screen {
    state: TextAreaState,
    lines: Vec<String>,
    language: TextLanguage,
    dash: Dashboard,
    pressed: Option<usize>,
}

fn screen(language: TextLanguage) -> Screen {
    Screen {
        state: TextAreaState::new(),
        lines: (0..60)
            .map(|i| match i {
                0 => "fn main() { let x = 1; }".to_owned(),
                1 => "# A heading with for and in".to_owned(),
                _ => format!("    line {i}"),
            })
            .collect(),
        language,
        dash: Dashboard::new(
            DASH,
            vec!["HEADER".into()],
            vec![
                DashboardButton::new(Glyph::Search, "Search", "Shift Shift", 'e'),
                DashboardButton::new(Glyph::Close, "Quit", "Ctrl K Q", 'q'),
            ],
        ),
        pressed: None,
    }
}

fn event(s: &mut Screen, rt: &mut Runtime, e: &Event) {
    TextArea::new(AREA).handle(rt, &mut s.state, &s.lines, e);
    if let Some(i) = s.dash.handle(rt, e) {
        s.pressed = Some(i);
    }
}

fn draw(s: &mut Screen, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    Pane::new(&rt.look).kind(PaneKind::Editor).render(area, buf);
    let look = rt.look.clone();
    let left = Rect::new(0, 0, 40, area.height);
    TextArea::new(AREA)
        .language(s.language)
        .show(rt, &mut s.state, &s.lines, left, buf);
    s.dash
        .show(rt, &look, Rect::new(40, 0, 40, area.height), buf);
}

type EventFn = fn(&mut Screen, &mut Runtime, &Event);
type DrawFn = fn(&mut Screen, &mut Runtime, Rect, &mut Buffer);
type ScreenApp = FnApp<Screen, EventFn, DrawFn>;

fn harness(language: TextLanguage) -> Harness<ScreenApp> {
    let mut h = Harness::new(
        app_fn(screen(language), event as EventFn, draw as DrawFn),
        80,
        12,
    );
    h.rt.focus.focus(AREA);
    h.draw();
    h
}

macro_rules! s {
    ($h:expr) => {
        &mut $h.app.state
    };
}

#[test]
fn arrows_move_the_cursor_and_wrap_between_lines() {
    let mut h = harness(TextLanguage::Code);
    h.press(KeyCode::Down);
    assert_eq!(s!(h).state.line, 1);
    h.press(KeyCode::End);
    h.press(KeyCode::Right);
    assert_eq!((s!(h).state.line, s!(h).state.col), (2, 0));
    h.press(KeyCode::Left);
    assert_eq!(s!(h).state.line, 1);
}

#[test]
fn page_keys_and_ctrl_home_end_jump_and_the_view_follows() {
    let mut h = harness(TextLanguage::Code);
    h.press(KeyCode::PageDown);
    assert_eq!(s!(h).state.line, 11);
    h.press_with(KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(s!(h).state.line, 59);
    assert!(h.text().contains("line 59"));
    h.press_with(KeyCode::Home, KeyModifiers::CONTROL);
    assert_eq!(s!(h).state.top(), 0);
}

#[test]
fn a_click_sets_the_cursor_and_the_wheel_scrolls() {
    let mut h = harness(TextLanguage::Code);
    h.click(12, 3);
    assert_eq!(s!(h).state.line, 3);
    h.scroll(10, 3, 2);
    assert_eq!(s!(h).state.top(), 6);
}

#[test]
fn the_current_line_number_is_bright_and_the_cursor_line_is_highlighted() {
    let mut h = harness(TextLanguage::Code);
    h.press(KeyCode::Down);
    let p = h.rt.look.palette;
    assert_eq!(h.buffer()[(2, 1)].fg, p.white);
    assert_eq!(h.buffer()[(30, 1)].bg, p.black2);
    assert_eq!(h.buffer()[(2, 5)].fg, p.grey_fg);
}

#[test]
fn code_is_tinted_with_keywords_but_markdown_is_not() {
    let mut code = harness(TextLanguage::Code);
    let purple = code.rt.look.palette.purple;
    let (x, y) = code.find("fn main").expect("line");
    assert_eq!(code.buffer()[(x + 1, y)].fg, purple);
    let (x, y) = code.find("let x").expect("line");
    assert_eq!(code.buffer()[(x + 1, y)].fg, purple, "keywords in code");
    let mut md = harness(TextLanguage::Markdown);
    let blue = md.rt.look.palette.blue;
    let (x, y) = md.find("# A heading").expect("line");
    assert_eq!(md.buffer()[(x, y)].fg, blue, "a heading");
    let (x, y) = md.find("for and in").expect("line");
    assert_ne!(
        md.buffer()[(x, y)].fg,
        purple,
        "no keyword tint in markdown"
    );
    let _ = (&mut code, &mut md);
}

#[test]
fn language_follows_the_file_extension() {
    assert_eq!(TextLanguage::of_file("README.md"), TextLanguage::Markdown);
    assert_eq!(TextLanguage::of_file("main.rs"), TextLanguage::Code);
    assert_eq!(TextLanguage::of_file("Cargo.lock"), TextLanguage::Plain);
}

#[test]
fn indent_guides_are_drawn_in_the_line_color() {
    let h = harness(TextLanguage::Code);
    let p = h.rt.look.palette;
    let mut found = false;
    for x in 0..40 {
        let c = &h.buffer()[(x, 4)];
        if c.symbol() == "│" && c.fg == p.line {
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn a_dashboard_button_is_pressed_by_click_or_its_key() {
    let mut h = harness(TextLanguage::Code);
    let (x, y) = h.find("Quit").expect("button");
    h.click(x, y);
    assert_eq!(s!(h).pressed, Some(1));
    s!(h).pressed = None;
    h.rt.focus.clear();
    h.press(KeyCode::Char('e'));
    assert_eq!(s!(h).pressed, Some(0));
}

#[test]
fn both_survive_tiny_screens() {
    for (w, hgt) in [(1u16, 1u16), (3, 2), (10, 4)] {
        let mut h = Harness::new(
            app_fn(
                screen(TextLanguage::Markdown),
                event as EventFn,
                draw as DrawFn,
            ),
            w,
            hgt,
        );
        h.rt.focus.focus(AREA);
        for code in [
            KeyCode::Down,
            KeyCode::End,
            KeyCode::PageDown,
            KeyCode::Left,
        ] {
            h.press(code);
        }
        h.click(0, 0);
        h.scroll(0, 0, 3);
        h.type_text("eq");
    }
}
