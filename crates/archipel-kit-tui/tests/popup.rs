//! Event-driven tests of list popups and action popup menus: keys, mouse, timers.

#![allow(clippy::unnecessary_mut_passed)]

use archipel_kit::action_system::{
    ActionPlace, AnAction, AnActionEvent, DataContext, DefaultActionGroup, Presentation, Toggle,
    ToggleAction,
};
use archipel_kit::ui::popup::{BaseListPopupStep, PopupStepResult};
use archipel_kit_tui::components::{Pane, PaneKind};
use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use archipel_kit_tui::popup::*;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{FnApp, Harness, app_fn, snapshot};
use archipel_kit_tui::{Caps, Event, Glyph, Look, Runtime};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct Screen {
    popup: Option<ListPopup<&'static str>>,
    menu: Option<ActionPopupMenu>,
    chosen: Vec<&'static str>,
    performed: Vec<String>,
    cancelled: u32,
}

fn event(s: &mut Screen, rt: &mut Runtime, e: &Event) {
    if let Some(popup) = &mut s.popup {
        match popup.event(rt, e) {
            PopupState::Chosen => {
                if let Some(v) = popup.take_chosen() {
                    s.chosen.push(v);
                }
                s.popup = None;
            }
            PopupState::Cancelled => {
                s.cancelled += 1;
                s.popup = None;
            }
            PopupState::Open => {}
        }
    }
    if let Some(menu) = &mut s.menu {
        match menu.event(rt, e) {
            PopupState::Chosen => {
                if let Some(id) = menu.take_performed() {
                    s.performed.push(id);
                }
                s.menu = None;
            }
            PopupState::Cancelled => {
                s.cancelled += 1;
                s.menu = None;
            }
            PopupState::Open => {}
        }
    }
}

fn draw(s: &mut Screen, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    Pane::new(&rt.look).kind(PaneKind::Editor).render(area, buf);
    if let Some(popup) = &mut s.popup {
        popup.tick(rt);
        popup.draw(rt, buf);
    }
    if let Some(menu) = &mut s.menu {
        menu.tick(rt);
        menu.draw(rt, buf);
    }
}

type EventFn = fn(&mut Screen, &mut Runtime, &Event);
type DrawFn = fn(&mut Screen, &mut Runtime, Rect, &mut Buffer);
type ScreenApp = FnApp<Screen, EventFn, DrawFn>;

fn harness(screen: Screen) -> Harness<ScreenApp> {
    Harness::new(app_fn(screen, event as EventFn, draw as DrawFn), 70, 24)
}

macro_rules! screen {
    ($h:expr) => {
        &mut $h.app.state
    };
}

fn open_popup(h: &mut Harness<ScreenApp>, popup: ListPopup<&'static str>) {
    let mut popup = popup;
    assert!(popup.open(&h.rt, true));
    screen!(h).popup = Some(popup);
    h.draw();
}

fn open_menu(h: &mut Harness<ScreenApp>, menu: ActionPopupMenu) {
    let mut menu = menu;
    assert!(menu.open(&h.rt, true));
    screen!(h).menu = Some(menu);
    h.draw();
}

fn selected(h: &mut Harness<ScreenApp>, depth: usize) -> Option<usize> {
    let s = screen!(h);
    if let Some(p) = &s.popup {
        return p.chain.selected(depth);
    }
    s.menu.as_ref().and_then(|m| m.chain.selected(depth))
}

const ITEMS: [&str; 6] = ["Run", "Debug", "Profile", "Coverage", "Stop", "Rerun"];

fn items_step() -> BaseListPopupStep<&'static str> {
    BaseListPopupStep::new("Run", ITEMS.to_vec())
        .separator_before(2)
        .unselectable(1)
}

fn menu_popup(step: BaseListPopupStep<&'static str>, anchor: Rect) -> ListPopup<&'static str> {
    ListPopup::menu_below(step, anchor)
}

#[test]
fn up_and_down_wrap_and_skip_disabled_rows() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    assert_eq!(
        selected(&mut h, 0),
        Some(0),
        "opened by keyboard: first row"
    );
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h, 0), Some(2), "row 1 is disabled");
    h.press(KeyCode::Up);
    h.press(KeyCode::Up);
    assert_eq!(selected(&mut h, 0), Some(5), "wraps to the last row");
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h, 0), Some(0));
}

#[test]
fn home_end_and_page_keys_jump() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::End);
    assert_eq!(selected(&mut h, 0), Some(5));
    h.press(KeyCode::Home);
    assert_eq!(selected(&mut h, 0), Some(0));
    h.press(KeyCode::PageDown);
    assert_eq!(selected(&mut h, 0), Some(5));
}

#[test]
fn enter_chooses_the_selected_row_and_closes() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(screen!(h).chosen, ["Profile"]);
    assert!(screen!(h).popup.is_none());
}

#[test]
fn escape_cancels_the_popup() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Esc);
    assert_eq!(screen!(h).cancelled, 1);
    assert!(screen!(h).popup.is_none());
}

fn nested_step() -> BaseListPopupStep<&'static str> {
    BaseListPopupStep::new("", vec!["Open", "Recent", "Close"])
        .has_substep(|v| *v == "Recent")
        .on_chosen(|v, final_choice| {
            if *v == "Recent" && !final_choice {
                return PopupStepResult::Next(Box::new(
                    BaseListPopupStep::new("", vec!["a.rs", "b.rs", "c.rs"]).has_substep(|_| false),
                ));
            }
            if *v == "Recent" {
                return PopupStepResult::Next(Box::new(BaseListPopupStep::new(
                    "",
                    vec!["a.rs", "b.rs", "c.rs"],
                )));
            }
            PopupStepResult::FinalChoice
        })
}

#[test]
fn right_opens_a_substep_left_goes_back_and_escape_goes_back_one_level() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(nested_step(), Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Down);
    h.press(KeyCode::Right);
    let p = screen!(h).popup.as_ref().expect("open");
    assert_eq!(p.chain.depth(), 2);
    assert_eq!(
        selected(&mut h, 1),
        Some(0),
        "opened by keyboard: first row"
    );
    assert!(h.text().contains("b.rs"));
    h.press(KeyCode::Left);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 1);
    h.press(KeyCode::Right);
    h.press(KeyCode::Esc);
    let p = screen!(h).popup.as_ref().expect("Esc only went back");
    assert_eq!(p.chain.depth(), 1);
}

#[test]
fn enter_on_a_row_with_a_substep_opens_it_instead_of_finishing() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(nested_step(), Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 2);
    h.press(KeyCode::Enter);
    assert_eq!(screen!(h).chosen, ["a.rs"]);
}

#[test]
fn shift_escape_closes_every_level() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(nested_step(), Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Down);
    h.press(KeyCode::Right);
    h.press_with(KeyCode::Esc, KeyModifiers::SHIFT);
    assert!(screen!(h).popup.is_none());
    assert_eq!(screen!(h).cancelled, 1);
}

#[test]
fn a_mnemonic_chooses_its_row() {
    let mut h = harness(Screen::default());
    let step = BaseListPopupStep::new("", ITEMS.to_vec()).mnemonics(true);
    open_popup(&mut h, menu_popup(step, Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Char('3'));
    assert_eq!(screen!(h).chosen, ["Profile"]);
}

#[test]
fn typing_filters_with_the_speed_search_and_escape_clears_it_first() {
    let mut h = harness(Screen::default());
    let step = BaseListPopupStep::new("", ITEMS.to_vec()).speed_search(true);
    open_popup(&mut h, menu_popup(step, Rect::new(4, 2, 10, 1)));
    h.type_text("re");
    let text = h.text();
    assert!(text.contains("Rerun") && !text.contains("Debug"), "{text}");
    assert_eq!(selected(&mut h, 0), Some(5), "the first match is selected");
    h.press(KeyCode::Backspace);
    h.press(KeyCode::Backspace);
    assert!(h.text().contains("Debug"));
    h.type_text("x");
    assert!(h.text().contains("No matches"));
    h.press(KeyCode::Esc);
    assert!(
        screen!(h).popup.is_some(),
        "the first Esc only clears the search"
    );
    h.press(KeyCode::Esc);
    assert!(screen!(h).popup.is_none());
}

#[test]
fn hovering_selects_the_row_under_the_pointer_but_not_on_the_first_move() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    let (x, y) = h.find("Coverage").expect("row");
    h.hover(x, y);
    assert_eq!(selected(&mut h, 0), Some(0), "the first move is ignored");
    h.hover(x + 1, y);
    assert_eq!(selected(&mut h, 0), Some(3));
}

#[test]
fn hovering_a_disabled_row_clears_the_selection() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    let (x, y) = h.find("Debug").expect("row");
    h.hover(x, y);
    h.hover(x + 1, y);
    assert_eq!(selected(&mut h, 0), None);
}

#[test]
fn a_click_chooses_on_release_over_the_same_row() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    let (x, y) = h.find("Stop").expect("row");
    h.click(x, y);
    assert_eq!(screen!(h).chosen, ["Stop"]);
}

#[test]
fn pressing_on_one_row_and_releasing_on_another_chooses_nothing() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    let (x, y) = h.find("Stop").expect("row");
    h.drag((x, y), (x, y + 1));
    assert!(screen!(h).chosen.is_empty());
    assert!(screen!(h).popup.is_some());
}

#[test]
fn a_press_outside_cancels_and_is_swallowed() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 2, 10, 1)));
    h.click(60, 20);
    assert_eq!(screen!(h).cancelled, 1);
    assert!(screen!(h).popup.is_none());
}

#[test]
fn pressing_the_chevron_opens_the_substep_without_choosing() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(nested_step(), Rect::new(4, 2, 10, 1)));
    let rect = screen!(h)
        .popup
        .as_ref()
        .expect("open")
        .chain
        .rect(0)
        .expect("rect");
    let (_, y) = h.find("Recent").expect("row");
    h.click(rect.right() - 2, y);
    let p = screen!(h).popup.as_ref().expect("open");
    assert_eq!(p.chain.depth(), 2);
    assert!(screen!(h).chosen.is_empty());
}

#[test]
fn a_submenu_opens_250_ms_after_the_pointer_rests_on_its_row_when_hover_is_on() {
    let mut h = harness(Screen::default());
    let popup = menu_popup(nested_step(), Rect::new(4, 2, 10, 1)).show_submenu_on_hover(true);
    open_popup(&mut h, popup);
    let (x, y) = h.find("Recent").expect("row");
    h.hover(x, y);
    h.hover(x + 1, y);
    h.advance(0.2);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 1);
    h.advance(0.06);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 2);
}

#[test]
fn a_submenu_does_not_open_on_hover_when_hover_is_off() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(nested_step(), Rect::new(4, 2, 10, 1)));
    let (x, y) = h.find("Recent").expect("row");
    h.hover(x, y);
    h.hover(x + 1, y);
    h.advance(1.0);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 1);
}

#[test]
fn leaving_the_row_before_250_ms_cancels_the_pending_submenu() {
    let mut h = harness(Screen::default());
    let popup = menu_popup(nested_step(), Rect::new(4, 2, 10, 1)).show_submenu_on_hover(true);
    open_popup(&mut h, popup);
    let (x, y) = h.find("Recent").expect("row");
    h.hover(x, y);
    h.hover(x + 1, y);
    h.advance(0.15);
    h.hover(x + 1, y - 1);
    h.advance(0.5);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 1);
}

#[test]
fn an_auto_selection_popup_opens_the_selected_substep_after_500_ms() {
    let mut h = harness(Screen::default());
    let step = nested_step().auto_selection(true);
    open_popup(&mut h, menu_popup(step, Rect::new(4, 2, 10, 1)));
    h.press(KeyCode::Down);
    h.advance(0.4);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 1);
    h.advance(0.15);
    assert_eq!(screen!(h).popup.as_ref().expect("open").chain.depth(), 2);
}

#[test]
fn a_popup_refuses_to_reopen_within_200_ms_of_closing() {
    let mut h = harness(Screen::default());
    let mut popup = menu_popup(items_step(), Rect::new(4, 2, 10, 1));
    assert!(popup.open(&h.rt, true));
    popup.chain.close(h.rt.now);
    assert!(!popup.open(&h.rt, true));
    h.rt.now += 0.25;
    assert!(popup.open(&h.rt, true));
}

#[test]
fn the_wheel_scrolls_a_long_list() {
    let mut h = harness(Screen::default());
    let values: Vec<&'static str> = (0..40)
        .map(|i| &*Box::leak(format!("Item {i}").into_boxed_str()))
        .collect();
    let step = BaseListPopupStep::new("", values);
    open_popup(&mut h, menu_popup(step, Rect::new(4, 2, 10, 1)));
    assert!(!h.text().contains("Item 39"));
    let (x, y) = h.find("Item 3").expect("row");
    h.scroll(x, y, 20);
    assert!(h.text().contains("Item 39"));
}

#[test]
fn the_popup_flips_above_when_there_is_no_room_below() {
    let mut h = harness(Screen::default());
    open_popup(&mut h, menu_popup(items_step(), Rect::new(4, 22, 10, 1)));
    let rect = screen!(h)
        .popup
        .as_ref()
        .expect("open")
        .chain
        .rect(0)
        .expect("rect");
    assert!(rect.bottom() <= 22, "{rect:?}");
}

#[test]
fn a_telescope_popup_is_centered_with_a_title_and_a_prompt() {
    let mut h = harness(Screen::default());
    let step = BaseListPopupStep::new("Find Files", ITEMS.to_vec()).speed_search(true);
    open_popup(&mut h, ListPopup::list(step));
    let rect = screen!(h)
        .popup
        .as_ref()
        .expect("open")
        .chain
        .rect(0)
        .expect("rect");
    assert!(rect.x > 10 && rect.right() < 60, "{rect:?}");
    assert!(h.text().contains(" Find Files "));
    h.type_text("deb");
    assert!(h.text().contains("deb"));
}

// ----- action popup menu -----

struct Act {
    id: &'static str,
    text: &'static str,
    enabled: bool,
    runs: Rc<RefCell<Vec<String>>>,
    keep: bool,
}

impl AnAction for Act {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.text)
            .enabled(self.enabled)
            .keep_popup_on_perform(self.keep)
            .icon(Glyph::Run)
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.runs.borrow_mut().push(self.id.to_owned());
    }
}

#[derive(Default)]
struct Wrap(Cell<bool>);

impl ToggleAction for Wrap {
    fn id(&self) -> &str {
        "WordWrap"
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Soft-Wrap")
    }

    fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool {
        self.0.get()
    }

    fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) {
        self.0.set(selected);
    }
}

fn group(runs: &Rc<RefCell<Vec<String>>>) -> DefaultActionGroup {
    let act = |id, text, enabled, keep| Act {
        id,
        text,
        enabled,
        runs: runs.clone(),
        keep,
    };
    let mut g = DefaultActionGroup::new("Edit");
    g.add(act("Run", "Run", true, false));
    g.add(act("Debug", "Debug", false, false));
    g.add_separator();
    g.add(Toggle::new(Wrap::default()));
    g.add(act("Keep", "Stay open", true, true));
    let mut more = DefaultActionGroup::popup("More Tools");
    more.add(act("Profile", "Profile", true, false));
    more.add(act("Coverage", "Coverage", true, false));
    g.add_group(more);
    g.add_group(DefaultActionGroup::popup("Hidden"));
    g
}

fn menu(runs: &Rc<RefCell<Vec<String>>>, placement: Placement) -> ActionPopupMenu {
    let place = ActionPlace::EDITOR_POPUP;
    let mut keymap =
        archipel_kit_tui::input::terminal_keymap(archipel_kit::action_system::Os::Linux);
    keymap.add_shortcut(
        "Run",
        archipel_kit::action_system::Shortcut::parse("ctrl R").expect("shortcut"),
    );
    ActionPopupMenu::actions(
        Rc::new(group(runs)),
        Rc::new(DataContext::empty()),
        place,
        keymap,
        Caps::legacy(),
        placement,
    )
}

#[test]
fn choosing_an_action_performs_it_once_and_closes_the_menu() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    h.press(KeyCode::Enter);
    assert_eq!(*runs.borrow(), ["Run"]);
    assert_eq!(screen!(h).performed, ["Run"]);
    assert!(screen!(h).menu.is_none());
}

#[test]
fn a_disabled_action_is_skipped_and_never_performed() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h, 0), Some(2), "skips Debug");
    let (x, y) = h.find("Debug").expect("row");
    h.click(x, y);
    assert!(runs.borrow().is_empty());
}

#[test]
fn a_toggle_action_shows_a_check_and_flips_on_choose() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    assert!(
        !h.text().contains(
            archipel_kit_tui::glyphs::nerd_font(Glyph::Check)
                .to_string()
                .as_str()
        )
    );
    let (x, y) = h.find("Soft-Wrap").expect("row");
    h.click(x, y);
    assert_eq!(screen!(h).performed, ["WordWrap"]);
}

#[test]
fn an_action_that_keeps_the_popup_open_does_not_close_it() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    let (x, y) = h.find("Stay open").expect("row");
    h.click(x, y);
    assert_eq!(*runs.borrow(), ["Keep"]);
    assert!(screen!(h).menu.is_some());
}

#[test]
fn a_popup_group_opens_as_a_submenu() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    let (x, y) = h.find("More Tools").expect("row");
    h.hover(x, y);
    h.hover(x + 1, y);
    h.advance(0.3);
    assert!(h.text().contains("Coverage"), "{}", h.text());
    // Keys go to the deepest level, which has no selection yet when opened by hover.
    h.press(KeyCode::Down);
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(*runs.borrow(), ["Coverage"]);
}

#[test]
fn a_submenu_opened_with_the_keyboard_selects_its_first_row() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    for _ in 0..3 {
        h.press(KeyCode::Down);
    }
    h.press(KeyCode::Right);
    assert_eq!(selected(&mut h, 1), Some(0));
    h.press(KeyCode::Enter);
    assert_eq!(*runs.borrow(), ["Profile"]);
}

#[test]
fn shortcuts_are_shown_grey_at_the_right_with_the_terminal_fallback() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)));
    let (_, y) = h.find("Run").expect("row");
    let line = &h.screen()[y as usize];
    assert!(line.contains("Ctrl+R"), "{line}");
}

#[test]
fn numbering_chooses_rows_by_their_number() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)).numbering(true));
    h.press(KeyCode::Char('4'));
    assert_eq!(*runs.borrow(), ["Keep"]);
}

#[test]
fn the_action_menu_can_filter_with_the_speed_search() {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let mut h = harness(Screen::default());
    open_menu(&mut h, menu(&runs, Placement::At(5, 3)).speed_search(true));
    h.type_text("stay");
    assert!(!h.text().contains("Soft-Wrap"));
    h.press(KeyCode::Enter);
    assert_eq!(*runs.borrow(), ["Keep"]);
}

#[test]
fn popups_survive_tiny_screens() {
    for (w, hgt) in [(1u16, 1u16), (4, 2), (12, 5), (30, 3)] {
        let runs = Rc::new(RefCell::new(Vec::new()));
        let mut h = Harness::new(
            app_fn(Screen::default(), event as EventFn, draw as DrawFn),
            w,
            hgt,
        );
        let mut m = menu(
            &runs,
            Placement::At(w.saturating_sub(1), hgt.saturating_sub(1)),
        );
        assert!(m.open(&h.rt, true));
        screen!(h).menu = Some(m);
        h.draw();
        for code in [
            KeyCode::Down,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Left,
            KeyCode::PageDown,
            KeyCode::End,
        ] {
            h.press(code);
        }
        h.hover(0, 0);
        h.hover(w - 1, hgt - 1);
        h.advance(1.0);
        h.scroll(0, 0, 3);
        let mut p = ListPopup::list(BaseListPopupStep::new("T", ITEMS.to_vec()).speed_search(true));
        assert!(p.open(&h.rt, true));
        screen!(h).popup = Some(p);
        screen!(h).menu = None;
        h.draw();
        h.type_text("run");
        h.press(KeyCode::Enter);
    }
}

#[test]
fn popups_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let runs = Rc::new(RefCell::new(Vec::new()));
        let mut h = Harness::with_runtime(
            app_fn(Screen::default(), event as EventFn, draw as DrawFn),
            Runtime::new(look),
            72,
            22,
        );
        // The menu with a submenu open.
        let mut m = menu(&runs, Placement::At(3, 2));
        assert!(m.open(&h.rt, true));
        screen!(h).menu = Some(m);
        h.draw();
        h.press(KeyCode::Down);
        h.press(KeyCode::Down);
        h.press(KeyCode::Down);
        h.press(KeyCode::Right);
        assert_eq!(selected(&mut h, 1), Some(0));
        // The telescope list below it.
        let step = BaseListPopupStep::new("Find Files", ITEMS.to_vec()).speed_search(true);
        let mut p = ListPopup::list(step)
            .at(Placement::At(26, 11))
            .style(PopupStyle::Telescope);
        assert!(p.open(&h.rt, true));
        screen!(h).popup = Some(p);
        h.draw();
        snapshot(&format!("popup.{name}"), h.buffer());
    }
}
