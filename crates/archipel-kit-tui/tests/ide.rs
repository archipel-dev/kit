//! The IDE example, driven with synthetic events: the same code that runs in a terminal.

#![allow(clippy::unnecessary_mut_passed, dead_code, unused_imports)]

#[path = "../examples/ide.rs"]
mod ide;

use archipel_kit_tui::input::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use archipel_kit_tui::ratatui::crossterm::event::ModifierKeyCode;
use archipel_kit_tui::testing::{Harness, snapshot};
use archipel_kit_tui::{Event, Runtime};
use ide::IdeApp;

fn harness_with(theme: usize, w: u16, h: u16) -> Harness<IdeApp> {
    let look = IdeApp::look_for(theme);
    let rt = Runtime::with(
        look,
        IdeApp::keymap(archipel_kit::action_system::Os::Linux),
        archipel_kit_tui::Caps::kitty(),
    );
    let mut h = Harness::with_runtime(IdeApp::new(theme), rt, w, h);
    h.draw();
    h
}

fn harness() -> Harness<IdeApp> {
    harness_with(0, 130, 36)
}

#[test]
fn it_starts_with_the_readme_open_the_project_tree_and_a_status_line() {
    let h = harness();
    let text = h.text();
    assert!(text.contains("README.md"), "{text}");
    assert!(text.contains("Project"));
    assert!(text.contains("Ln 1, Col 1"));
}

#[test]
fn alt_1_focuses_the_project_window_and_escape_returns_to_the_editor() {
    let mut h = harness();
    h.alt('1');
    assert_eq!(h.app.workbench_focus_tool(), Some("project".to_owned()));
    h.press(KeyCode::Esc);
    assert_eq!(h.app.workbench_focus_tool(), None);
}

#[test]
fn enter_in_the_tree_opens_the_file_in_a_tab() {
    let mut h = harness();
    h.alt('1');
    h.type_text("lib.rs");
    h.press(KeyCode::Enter);
    assert!(h.text().contains("lib.rs"), "{}", h.text());
    assert_eq!(h.app.open_files().len(), 2);
}

#[test]
fn ctrl_p_opens_search_everywhere_and_choosing_an_action_runs_it() {
    let mut h = harness();
    h.ctrl('p');
    assert!(h.text().contains("Search Everywhere"));
    h.type_text("run");
    h.press(KeyCode::Enter);
    assert!(h.app.is_running());
    h.advance(3.0);
    assert!(!h.app.is_running());
    assert!(h.text().contains("Build succeeded"), "{}", h.text());
}

#[test]
fn double_shift_opens_search_everywhere() {
    let mut h = harness();
    let shift = |up: bool| {
        let key = KeyEvent::new_with_kind(
            KeyCode::Modifier(ModifierKeyCode::LeftShift),
            KeyModifiers::SHIFT,
            if up {
                KeyEventKind::Release
            } else {
                KeyEventKind::Press
            },
        );
        if up {
            Event::KeyUp(key)
        } else {
            Event::Key(key)
        }
    };
    h.send(shift(false));
    h.send(shift(true));
    h.advance(0.2);
    h.send(shift(false));
    h.send(shift(true));
    assert!(h.text().contains("Search Everywhere"));
}

#[test]
fn ctrl_t_switches_the_theme_through_all_of_them() {
    let mut h = harness();
    let before = h.rt.look.palette.black;
    h.ctrl('t');
    assert_ne!(h.rt.look.palette.black, before);
    for _ in 0..archipel_kit_tui::Theme::all().len() {
        h.ctrl('t');
    }
}

#[test]
fn the_new_file_dialog_validates_and_creates_the_file() {
    let mut h = harness();
    h.ctrl('n');
    assert!(h.text().contains("New File"));
    h.advance(0.4);
    assert!(h.text().contains("Enter a file name"));
    h.type_text("tool.rs");
    h.advance(0.4);
    h.press(KeyCode::Enter);
    assert!(h.app.open_files().iter().any(|p| p.ends_with("tool.rs")));
}

#[test]
fn quit_asks_first() {
    let mut h = harness();
    h.ctrl('k');
    h.press(KeyCode::Char('q'));
    assert!(h.text().contains("Leave the IDE?"));
    h.press(KeyCode::Char('n'));
    assert!(!h.rt.quit);
    h.ctrl('k');
    h.press(KeyCode::Char('q'));
    h.press(KeyCode::Char('y'));
    assert!(h.rt.quit);
}

#[test]
fn right_click_in_the_tree_opens_a_context_menu_and_delete_asks_to_confirm() {
    let mut h = harness();
    let (x, y) = h.find("palette.rs").expect("tree row");
    h.right_click(x + 1, y);
    assert!(h.text().contains("Delete"), "{}", h.text());
    let (dx, dy) = h.find("Delete").expect("menu row");
    h.click(dx, dy);
    assert!(h.text().contains("palette.rs?"), "{}", h.text());
    h.press(KeyCode::Char('y'));
    assert!(!h.app.has_file("crates/archipel-kit-tui/src/palette.rs"));
}

#[test]
fn escape_unwinds_menus_before_anything_else() {
    let mut h = harness();
    h.ctrl('p');
    h.press(KeyCode::Esc);
    assert!(!h.text().contains("Search Everywhere"));
}

#[test]
fn the_ide_survives_any_small_size_and_random_input() {
    let keys = [
        KeyCode::Tab,
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::Down,
        KeyCode::Right,
        KeyCode::Left,
        KeyCode::PageDown,
    ];
    for (w, hgt) in [(1u16, 1u16), (5, 3), (20, 4), (40, 10), (60, 14)] {
        let mut h = harness_with(1, w, hgt);
        for code in keys {
            h.press(code);
        }
        h.alt('1');
        h.alt('4');
        h.ctrl('p');
        h.type_text("run");
        h.press(KeyCode::Enter);
        h.advance(3.0);
        h.click(w / 2, hgt / 2);
        h.right_click(w / 2, hgt / 2);
        h.drag((0, 0), (w - 1, hgt - 1));
        h.scroll(1, 1, 3);
        h.ctrl('t');
        h.ctrl('n');
        h.press(KeyCode::Esc);
        h.resize(w.max(2) + 3, hgt.max(2) + 1);
    }
}

#[test]
fn the_ide_in_both_themes() {
    for (name, theme) in [("dark", 0usize), ("light", 1usize)] {
        let mut h = harness_with(theme, 130, 36);
        h.alt('1');
        h.press_with(KeyCode::F(10), KeyModifiers::SHIFT);
        h.advance(1.1);
        snapshot(&format!("ide.{name}"), h.buffer());
    }
    let mut h = harness_with(0, 130, 36);
    h.ctrl('p');
    h.type_text("th");
    snapshot("ide-search.dark", h.buffer());
}

#[test]
fn the_editor_edits_shows_the_position_and_the_modified_dot_until_saved() {
    let mut h = harness();
    h.press(KeyCode::Down);
    h.press(KeyCode::End);
    assert!(h.text().contains("Ln 2, Col 1"), "{}", h.text());
    assert!(h.text().contains("EDIT"));
    h.type_text("hi");
    assert!(h.text().contains("Ln 2, Col 3"), "{}", h.text());
    assert!(h.app.is_modified(0));
    h.ctrl('z');
    assert!(!h.app.is_modified(0), "undo back to the saved text");
    h.type_text("x");
    h.ctrl('s');
    assert!(!h.app.is_modified(0));
    // Find works inside the IDE.
    h.ctrl('f');
    h.type_text("NvChad");
    assert!(h.text().contains("Find"), "{}", h.text());
    h.press(KeyCode::Esc);
}

#[test]
fn the_editor_in_the_ide_with_a_selection_and_the_find_bar() {
    for (name, theme) in [("dark", 0usize), ("light", 1usize)] {
        let mut h = harness_with(theme, 130, 36);
        h.alt('1');
        h.type_text("lib.rs");
        h.press(KeyCode::Enter);
        h.press(KeyCode::Esc);
        for _ in 0..12 {
            h.press(KeyCode::Down);
        }
        h.ctrl('f');
        h.type_text("look");
        h.snapshot(&format!("ide-editor.{name}"));
    }
}
