//! Event-driven tests of the workbench: tool window keys, title lines, splitters and the
//! opt-in stripes.

#![allow(clippy::unnecessary_mut_passed)]

use archipel_kit::wm::{Content, ToolWindow, ToolWindowAnchor};
use archipel_kit_tui::input::{KeyCode, KeyModifiers, key_with};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::testing::{FnApp, Harness, app_fn, snapshot};
use archipel_kit_tui::workbench::*;
use archipel_kit_tui::{Event, Glyph, Look, Runtime};

struct Screen {
    wb: Workbench,
    events: Vec<WorkbenchEvent>,
    areas: WorkbenchAreas,
}

fn windows() -> Vec<ToolWindow> {
    let mut run =
        ToolWindow::new("run", "Run", Glyph::Run, ToolWindowAnchor::Bottom).activate_number(4);
    run.contents.add_content(Content::new("a", "cargo run"));
    run.contents.add_content(Content::new("b", "tests"));
    vec![
        ToolWindow::new("project", "Project", Glyph::Project, ToolWindowAnchor::Left)
            .activate_number(1),
        ToolWindow::new(
            "structure",
            "Structure",
            Glyph::Structure,
            ToolWindowAnchor::Left,
        )
        .activate_number(2),
        ToolWindow::new(
            "notifications",
            "Notifications",
            Glyph::Bell,
            ToolWindowAnchor::Right,
        )
        .activate_number(3),
        run,
    ]
}

fn screen() -> Screen {
    Screen {
        wb: Workbench::new(windows()),
        events: Vec::new(),
        areas: WorkbenchAreas::default(),
    }
}

fn event(s: &mut Screen, rt: &mut Runtime, e: &Event) {
    let ids: Vec<String> = rt.actions().to_vec();
    for id in ids {
        if let Some(events) = s.wb.perform(&id) {
            s.events.extend(events);
            return;
        }
    }
    let events = s.wb.handle(rt, e);
    s.events.extend(events);
}

fn draw(s: &mut Screen, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    s.areas = s.wb.draw(rt, area, buf);
    let p = rt.look.palette;
    for (id, rect) in &s.areas.tools {
        archipel_kit_tui::testing::put_text(
            buf,
            *rect,
            rect.x + 1,
            rect.y,
            &format!("[{id}]"),
            p.white,
            p.darker_black,
        );
    }
    archipel_kit_tui::testing::put_text(
        buf,
        s.areas.editor,
        s.areas.editor.x + 1,
        s.areas.editor.y,
        "[editor]",
        p.white,
        p.black,
    );
}

type EventFn = fn(&mut Screen, &mut Runtime, &Event);
type DrawFn = fn(&mut Screen, &mut Runtime, Rect, &mut Buffer);
type ScreenApp = FnApp<Screen, EventFn, DrawFn>;

fn harness_sized(w: u16, h: u16) -> Harness<ScreenApp> {
    let mut h = Harness::new(app_fn(screen(), event as EventFn, draw as DrawFn), w, h);
    h.draw();
    h
}

fn harness() -> Harness<ScreenApp> {
    harness_sized(100, 30)
}

macro_rules! wb {
    ($h:expr) => {
        &mut $h.app.state.wb
    };
}

macro_rules! events {
    ($h:expr) => {
        &mut $h.app.state.events
    };
}

/// Turns the opt-in stripes on.
fn with_stripes(h: &mut Harness<ScreenApp>) {
    let wb = wb!(h).clone().stripes(true);
    *wb!(h) = wb;
    h.draw();
}

fn alt(h: &mut Harness<ScreenApp>, digit: char) {
    h.alt(digit);
}

#[test]
fn alt_digit_shows_and_focuses_a_hidden_tool_window() {
    let mut h = harness();
    wb!(h).state.hide_all();
    h.draw();
    alt(&mut h, '1');
    assert!(wb!(h).is_visible("project"));
    assert_eq!(wb!(h).focused_tool(), Some("project"));
    assert!(h.text().contains("[project]"));
}

#[test]
fn alt_digit_focuses_a_visible_tool_window_that_has_no_focus_and_hides_a_focused_one() {
    let mut h = harness();
    alt(&mut h, '1');
    assert_eq!(wb!(h).focused_tool(), Some("project"));
    alt(&mut h, '1');
    assert!(!wb!(h).is_visible("project"), "a focused window hides");
    assert_eq!(wb!(h).focused_tool(), None);
    alt(&mut h, '1');
    alt(&mut h, '4');
    assert_eq!(wb!(h).focused_tool(), Some("run"));
    alt(&mut h, '1');
    assert_eq!(
        wb!(h).focused_tool(),
        Some("project"),
        "visible but unfocused: focus it"
    );
}

#[test]
fn alt_digit_of_a_second_window_at_the_same_anchor_replaces_the_first() {
    let mut h = harness();
    alt(&mut h, '1');
    alt(&mut h, '2');
    assert!(wb!(h).is_visible("structure"));
    assert!(!wb!(h).is_visible("project"));
}

#[test]
fn the_terminal_fallback_chord_ctrl_w_then_digit_activates_too() {
    let mut h = harness();
    h.ctrl('w');
    assert!(h.rt.awaiting_second_stroke());
    h.press(KeyCode::Char('3'));
    assert_eq!(wb!(h).focused_tool(), Some("notifications"));
}

#[test]
fn escape_returns_focus_to_the_editor_and_keeps_the_window_open() {
    let mut h = harness();
    alt(&mut h, '1');
    h.press(KeyCode::Esc);
    assert_eq!(wb!(h).focus, WorkbenchFocus::Editor);
    assert!(wb!(h).is_visible("project"));
    assert!(events!(h).contains(&WorkbenchEvent::FocusEditor));
}

#[test]
fn shift_escape_hides_the_active_window_or_the_last_one_from_the_editor() {
    let mut h = harness();
    alt(&mut h, '1');
    h.press_with(KeyCode::Esc, KeyModifiers::SHIFT);
    assert!(!wb!(h).is_visible("project"));
    alt(&mut h, '4');
    h.press(KeyCode::Esc);
    h.press_with(KeyCode::Esc, KeyModifiers::SHIFT);
    assert!(
        !wb!(h).is_visible("run"),
        "hides the last active window from the editor"
    );
}

#[test]
fn ctrl_shift_f12_hides_all_windows_and_a_second_press_restores_them() {
    let mut h = harness();
    alt(&mut h, '1');
    alt(&mut h, '4');
    h.press_with(KeyCode::F(12), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert!(!wb!(h).is_visible("project") && !wb!(h).is_visible("run"));
    assert_eq!(wb!(h).focus, WorkbenchFocus::Editor);
    h.press_with(KeyCode::F(12), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert!(wb!(h).is_visible("project") && wb!(h).is_visible("run"));
}

#[test]
fn f12_jumps_to_the_last_tool_window_and_back_to_the_editor() {
    let mut h = harness();
    alt(&mut h, '4');
    h.press(KeyCode::Esc);
    h.press(KeyCode::F(12));
    assert_eq!(wb!(h).focused_tool(), Some("run"));
    h.press(KeyCode::F(12));
    assert_eq!(wb!(h).focus, WorkbenchFocus::Editor);
}

#[test]
fn the_maximize_action_gives_the_window_the_whole_area_and_restores() {
    let mut h = harness();
    alt(&mut h, '4');
    h.press_with(
        KeyCode::Char('\''),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(wb!(h).state.maximized, Some(ToolWindowAnchor::Bottom));
    let area = h.app.state.areas.tool("run").expect("body");
    assert!(area.width >= 90 && area.height >= 25, "{area:?}");
    h.press_with(
        KeyCode::Char('\''),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(wb!(h).state.maximized, None);
}

#[test]
fn resize_keys_move_a_separator_by_two_cells() {
    let mut h = harness();
    alt(&mut h, '1');
    let before = h.app.state.areas.tool("project").expect("body").width;
    h.send(key_with(
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    let after = h.app.state.areas.tool("project").expect("body").width;
    assert!(after > before && after - before <= 3, "{before} -> {after}");
    h.send(key_with(
        KeyCode::Left,
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    h.send(key_with(
        KeyCode::Left,
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    assert!(h.app.state.areas.tool("project").expect("body").width < before);
}

#[test]
fn there_is_no_stripe_column_by_default() {
    let mut h = harness();
    assert!(!wb!(h).has_stripes());
    let layout = wb!(h).layout(Rect::new(0, 0, 100, 30));
    assert!(layout.left_stripe.is_empty() && layout.right_stripe.is_empty());
    alt(&mut h, '1');
    alt(&mut h, '3');
    alt(&mut h, '4');
    let project = h.app.state.areas.tool("project").expect("body");
    assert_eq!(project.x, 0, "the Project pane starts at the screen edge");
    let right = layout_right_edge(&mut h);
    assert_eq!(right, 100, "the right pane ends at the screen edge");
}

fn layout_right_edge(h: &mut Harness<ScreenApp>) -> u16 {
    let body = h.app.state.areas.tool("notifications").expect("body");
    body.right()
}

#[test]
fn the_title_line_is_a_plain_bar_without_buttons() {
    let mut h = harness();
    alt(&mut h, '1');
    let body = h.app.state.areas.tool("project").expect("body");
    // The title row is above the body: icon and name, nothing at the right end.
    let row: String = (0..body.width)
        .map(|x| h.buffer()[(x, body.y - 1)].symbol().to_owned())
        .collect();
    assert!(row.contains("Project"), "{row:?}");
    assert!(row.trim_end().ends_with("Project"), "{row:?}");
}

#[test]
fn clicking_a_pane_title_focuses_the_window_and_a_right_click_asks_for_its_menu() {
    let mut h = harness();
    alt(&mut h, '4');
    let body = h.app.state.areas.tool("project").expect("body");
    h.click(body.x + 3, body.y - 1);
    assert_eq!(wb!(h).focused_tool(), Some("project"));
    let run = h.app.state.areas.tool("run").expect("body");
    h.click(run.x + 3, run.y - 1);
    assert_eq!(wb!(h).focused_tool(), Some("run"));
    h.right_click(run.x + 3, run.y - 1);
    assert!(events!(h).contains(&WorkbenchEvent::HeaderMenu(
        "run".into(),
        run.x + 3,
        run.y - 1
    )));
}

#[test]
fn a_stripe_button_toggles_its_window() {
    let mut h = harness();
    with_stripes(&mut h);
    // The Project button is the first left stripe row (y = 1), x = 1.
    h.click(1, 1);
    assert_eq!(wb!(h).focused_tool(), Some("project"));
    h.click(1, 1);
    assert!(!wb!(h).is_visible("project"));
}

#[test]
fn middle_click_or_shift_click_on_a_stripe_button_hides_the_window() {
    let mut h = harness();
    with_stripes(&mut h);
    alt(&mut h, '1');
    h.middle_click(1, 1);
    assert!(!wb!(h).is_visible("project"));
    h.click(1, 1);
    h.click_with(1, 1, KeyModifiers::SHIFT);
    assert!(!wb!(h).is_visible("project"));
}

#[test]
fn a_right_click_on_a_stripe_button_asks_for_its_menu() {
    let mut h = harness();
    with_stripes(&mut h);
    h.right_click(1, 1);
    assert!(events!(h).contains(&WorkbenchEvent::StripeMenu("project".into(), 1, 1)));
}

#[test]
fn a_stripe_button_shows_its_tooltip_with_the_shortcut() {
    let mut h = harness();
    with_stripes(&mut h);
    h.hover(1, 1);
    h.advance(0.6);
    assert_eq!(h.rt.tooltip.showing_title(), Some("Project"));
    assert!(h.text().contains("Alt+1"), "{}", h.text());
}

#[test]
fn clicking_a_pane_focuses_it_and_clicking_the_editor_focuses_the_editor() {
    let mut h = harness();
    let editor = h.app.state.areas.editor;
    let project = h.app.state.areas.tool("project").expect("body");
    h.click(project.x + 2, project.y + 3);
    assert_eq!(wb!(h).focused_tool(), Some("project"));
    h.click(editor.x + 3, editor.y + 3);
    assert_eq!(wb!(h).focus, WorkbenchFocus::Editor);
}

#[test]
fn a_double_click_on_a_header_maximizes_and_a_middle_click_hides() {
    let mut h = harness();
    let body = h.app.state.areas.tool("project").expect("body");
    let header_y = body.y - 1;
    h.double_click(body.x + 15, header_y);
    assert_eq!(wb!(h).state.maximized, Some(ToolWindowAnchor::Left));
    // Maximized: the header is at the top of the area.
    let body = h.app.state.areas.tool("project").expect("body");
    h.advance(1.0);
    h.double_click(body.x + 15, body.y - 1);
    assert_eq!(wb!(h).state.maximized, None);
    let body = h.app.state.areas.tool("project").expect("body");
    h.middle_click(body.x + 15, body.y - 1);
    assert!(!wb!(h).is_visible("project"));
}

#[test]
fn a_shift_click_on_a_title_hides_the_window() {
    let mut h = harness();
    let body = h.app.state.areas.tool("project").expect("body");
    h.click_with(body.x + 3, body.y - 1, KeyModifiers::SHIFT);
    assert!(!wb!(h).is_visible("project"));
}

#[test]
fn dragging_the_left_separator_resizes_the_window_and_a_double_click_resets_it() {
    let mut h = harness();
    let body = h.app.state.areas.tool("project").expect("body");
    let sep_x = body.right();
    h.drag((sep_x, 10), (sep_x + 12, 10));
    let wider = h.app.state.areas.tool("project").expect("body").width;
    assert!(wider >= body.width + 11, "{wider} vs {}", body.width);
    let sep_x = h.app.state.areas.tool("project").expect("body").right();
    h.advance(1.0);
    h.double_click(sep_x, 10);
    assert_eq!(
        h.app.state.areas.tool("project").expect("body").width,
        body.width
    );
}

#[test]
fn dragging_the_bottom_separator_resizes_the_bottom_window() {
    let mut h = harness();
    let run = h.app.state.areas.tool("run").expect("body");
    let sep_y = run.y - 2;
    h.drag((run.x + 5, sep_y), (run.x + 5, sep_y - 5));
    let taller = h.app.state.areas.tool("run").expect("body").height;
    assert!(taller >= run.height + 4, "{taller} vs {}", run.height);
}

#[test]
fn separators_keep_the_editor_and_windows_at_their_minimum_sizes() {
    let mut h = harness();
    let body = h.app.state.areas.tool("project").expect("body");
    h.drag((body.right(), 10), (95, 10));
    let editor = h.app.state.areas.editor;
    assert!(editor.width >= 16, "{editor:?}");
    let body = h.app.state.areas.tool("project").expect("body");
    h.drag((body.right(), 10), (0, 10));
    assert!(h.app.state.areas.tool("project").expect("body").width >= 12);
}

#[test]
fn a_content_tab_click_selects_it_and_next_tab_cycles_while_the_window_is_focused() {
    let mut h = harness();
    alt(&mut h, '4');
    let (x, y) = h.find("tests").expect("content tab");
    h.click(x, y);
    assert_eq!(
        wb!(h).window("run").expect("run").contents.selected_index(),
        Some(1)
    );
    h.alt_arrow(KeyCode::Right);
    assert_eq!(
        wb!(h).window("run").expect("run").contents.selected_index(),
        Some(0),
        "wraps"
    );
    h.alt_arrow(KeyCode::Left);
    assert_eq!(
        wb!(h).window("run").expect("run").contents.selected_index(),
        Some(1)
    );
}

#[test]
fn close_content_asks_to_close_the_selected_content() {
    let mut h = harness();
    alt(&mut h, '4');
    h.press_with(KeyCode::F(4), KeyModifiers::CONTROL);
    assert!(events!(h).contains(&WorkbenchEvent::ContentCloseRequested("run".into(), 0)));
}

#[test]
fn the_layout_never_overlaps_or_leaves_the_screen_at_any_size() {
    for (w, hgt, stripes) in (1..=24u16).chain((25..=70).step_by(5)).flat_map(|w| {
        (1..=14u16)
            .chain([20, 30])
            .flat_map(move |h| [(w, h, false), (w, h, true)])
    }) {
        {
            let mut h = harness_sized(w, hgt);
            if stripes {
                with_stripes(&mut h);
            }
            alt(&mut h, '1');
            alt(&mut h, '3');
            alt(&mut h, '4');
            let area = Rect::new(0, 0, w, hgt);
            let layout = wb!(h).layout(area);
            let mut rects: Vec<Rect> = [
                layout.left,
                layout.right,
                layout.bottom,
                Some(layout.editor),
            ]
            .into_iter()
            .flatten()
            .filter(|r| !r.is_empty())
            .collect();
            if let Some(m) = layout.maximized {
                rects.push(m);
            }
            for r in &rects {
                assert!(r.right() <= w && r.bottom() <= hgt, "{r:?} at {w}x{hgt}");
            }
            for (i, a) in rects.iter().enumerate() {
                for b in &rects[i + 1..] {
                    assert!(a.intersection(*b).is_empty(), "{a:?} {b:?} at {w}x{hgt}");
                }
            }
            h.click(w / 2, hgt / 2);
            h.drag((0, 0), (w - 1, hgt - 1));
            h.press_with(
                KeyCode::Char('\''),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            );
        }
    }
}

#[test]
fn the_workbench_with_opt_in_stripes_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let mut h = Harness::with_runtime(
            app_fn(screen(), event as EventFn, draw as DrawFn),
            Runtime::new(look),
            100,
            28,
        );
        with_stripes(&mut h);
        h.alt('1');
        h.alt('4');
        h.draw();
        snapshot(&format!("workbench-stripes.{name}"), h.buffer());
    }
}

#[test]
fn the_workbench_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let mut h = Harness::with_runtime(
            app_fn(screen(), event as EventFn, draw as DrawFn),
            Runtime::new(look),
            100,
            28,
        );
        h.alt('1');
        h.alt('4');
        h.alt('3');
        h.draw();
        snapshot(&format!("workbench.{name}"), h.buffer());
    }
}
