//! Event-driven tests of the action toolbar, the combo box and the tab strip.

use archipel_kit::action_system::{
    ActionPlace, AnAction, AnActionEvent, DataContext, DefaultActionGroup, Presentation, Toggle,
    ToggleAction,
};
use archipel_kit_tui::combo::*;
use archipel_kit_tui::components::{Pane, PaneKind, TabInfo};
use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use archipel_kit_tui::popup::PopupState;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::tabs::*;
use archipel_kit_tui::testing::{FnApp, Harness, app_fn, snapshot};
use archipel_kit_tui::toolbar::*;
use archipel_kit_tui::{Event, Glyph, Id, Language, Look, Runtime};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const BAR: Id = Id::new("bar");
const TABS: Id = Id::new("tabs");
const COMBO: Id = Id::new("combo");

struct Act {
    id: &'static str,
    enabled: bool,
    runs: Rc<RefCell<Vec<String>>>,
}

impl AnAction for Act {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.id)
            .icon(match self.id {
                "Run" => Glyph::Run,
                "Debug" => Glyph::Debug,
                "Stop" => Glyph::Stop,
                _ => Glyph::Settings,
            })
            .description(format!("{} the configuration", self.id))
            .enabled(self.enabled)
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.runs.borrow_mut().push(self.id.to_owned());
    }
}

struct Wrap(Rc<Cell<bool>>);

impl ToggleAction for Wrap {
    fn id(&self) -> &str {
        "WordWrap"
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Soft-Wrap").icon(Glyph::SoftWrap)
    }

    fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool {
        self.0.get()
    }

    fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) {
        self.0.set(selected);
    }
}

struct Screen {
    bar: ActionToolbar,
    bar_state: ToolbarState,
    bar_events: Vec<String>,
    menu: Option<archipel_kit_tui::popup::ActionPopupMenu>,
    runs: Rc<RefCell<Vec<String>>>,
    wrap: Rc<Cell<bool>>,
    tabs: Vec<TabInfo>,
    tab_state: TabsState,
    tab_control: TabsControl,
    tab_events: Vec<TabsEvent>,
    combo_state: ComboBoxState,
    options: Vec<String>,
    list: Option<archipel_kit_tui::popup::ListPopup<usize>>,
    combo_events: Vec<ComboEvent>,
}

fn screen(width_limited: bool) -> Screen {
    let runs = Rc::new(RefCell::new(Vec::new()));
    let wrap = Rc::new(Cell::new(false));
    let act = |id, enabled| Act {
        id,
        enabled,
        runs: runs.clone(),
    };
    let mut g = DefaultActionGroup::new("Main");
    g.add(act("Run", true));
    g.add(act("Debug", false));
    g.add(act("Stop", true));
    g.add_separator();
    g.add(Toggle::new(Wrap(wrap.clone())));
    let mut more = DefaultActionGroup::popup("More");
    more.add(act("Profile", true));
    g.add_group(more);
    if width_limited {
        for name in ["Alpha", "Beta", "Gamma", "Delta"] {
            g.add(act(name, true));
        }
    }
    Screen {
        bar: ActionToolbar::new(
            BAR,
            Rc::new(g),
            Rc::new(DataContext::empty()),
            ActionPlace::MAIN_TOOLBAR,
        ),
        bar_state: ToolbarState::default(),
        bar_events: Vec::new(),
        menu: None,
        runs,
        wrap,
        tabs: vec![
            TabInfo::new("main.rs").icon(Language::Rust),
            TabInfo::new("Cargo.toml")
                .icon(Language::Toml)
                .modified(true),
            TabInfo::new("README.md").icon(Glyph::FileText),
            TabInfo::new("pinned.rs").icon(Language::Rust).pinned(true),
        ],
        tab_state: TabsState::new(0),
        tab_control: TabsControl::new(TABS),
        tab_events: Vec::new(),
        combo_state: ComboBoxState::new(1),
        options: vec!["Dark".into(), "Light".into(), "High contrast".into()],
        list: None,
        combo_events: Vec::new(),
    }
}

fn event(s: &mut Screen, rt: &mut Runtime, e: &Event) {
    if let Some(menu) = &mut s.menu {
        if menu.event(rt, e) != PopupState::Open {
            s.menu = None;
        }
        return;
    }
    if let Some(list) = &mut s.list {
        match list.event(rt, e) {
            PopupState::Chosen => {
                if let Some(i) = list.take_chosen() {
                    s.combo_state.choose(i, rt.now);
                }
                s.list = None;
            }
            PopupState::Cancelled => {
                s.combo_state.list_closed(rt.now);
                s.list = None;
            }
            PopupState::Open => {}
        }
        return;
    }
    match s.bar.handle(rt, &mut s.bar_state, e) {
        ToolbarEvent::Performed(id) => s.bar_events.push(id),
        ToolbarEvent::OpenGroup { index, anchor } => {
            if let Some(mut menu) = s.bar.popup_for(rt, index, anchor) {
                menu.open(rt, false);
                s.menu = Some(menu);
            }
        }
        ToolbarEvent::OpenOverflow { anchor } => {
            if let Some(mut menu) = s.bar.overflow_menu(rt, anchor) {
                menu.open(rt, false);
                s.menu = Some(menu);
            }
        }
        _ => {}
    }
    let r = s.tab_control.handle(rt, &mut s.tab_state, &mut s.tabs, e);
    if r != TabsEvent::None {
        s.tab_events.push(r);
    }
    let combo = ComboBoxControl::new(COMBO, &s.options).width(20);
    let r = combo.handle(rt, &mut s.combo_state, e);
    if r == ComboEvent::OpenList {
        let anchor = rt.hits.rect_of(COMBO).unwrap_or_default();
        let mut list = combo.popup(&s.combo_state, anchor);
        list.open(rt, true);
        s.list = Some(list);
    }
    if r != ComboEvent::None {
        s.combo_events.push(r);
    }
}

fn draw(s: &mut Screen, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    Pane::new(&rt.look).kind(PaneKind::Editor).render(area, buf);
    let bar_area = Rect::new(2, 1, 30, 1);
    s.bar.show(rt, &s.bar_state, bar_area, buf);
    let fill = rt.look.palette.darker_black;
    s.tab_control.show(
        rt,
        &mut s.tab_state,
        &s.tabs,
        Rect::new(0, 3, area.width.min(70), 1),
        fill,
        buf,
    );
    let combo = ComboBoxControl::new(COMBO, &s.options).width(20);
    combo.show(rt, &s.combo_state, 2, 6, buf);
    if let Some(menu) = &mut s.menu {
        menu.tick(rt);
        menu.draw(rt, buf);
    }
    if let Some(list) = &mut s.list {
        list.tick(rt);
        list.draw(rt, buf);
    }
}

type EventFn = fn(&mut Screen, &mut Runtime, &Event);
type DrawFn = fn(&mut Screen, &mut Runtime, Rect, &mut Buffer);
type ScreenApp = FnApp<Screen, EventFn, DrawFn>;

fn harness_with(s: Screen) -> Harness<ScreenApp> {
    Harness::new(app_fn(s, event as EventFn, draw as DrawFn), 80, 14)
}

fn harness() -> Harness<ScreenApp> {
    harness_with(screen(false))
}

macro_rules! screen {
    ($h:expr) => {
        &mut $h.app.state
    };
}

fn button_x(h: &Harness<ScreenApp>, id: &str) -> u16 {
    // Buttons are 3 cells: the rectangle starts at the bar's x plus the widths before it.
    let order = ["Run", "Debug", "Stop"];
    2 + 3 * order.iter().position(|n| *n == id).expect("button") as u16 + 1 + h.buffer().area.x
}

// ----- toolbar -----

#[test]
fn a_click_performs_the_action_on_release_inside_the_button() {
    let mut h = harness();
    h.click(button_x(&h, "Run"), 1);
    assert_eq!(*screen!(h).runs.borrow(), ["Run"]);
    assert_eq!(screen!(h).bar_events, ["Run"]);
}

#[test]
fn pressing_a_button_and_releasing_elsewhere_performs_nothing() {
    let mut h = harness();
    h.drag((button_x(&h, "Run"), 1), (30, 9));
    assert!(screen!(h).runs.borrow().is_empty());
}

#[test]
fn a_disabled_action_is_not_performed() {
    let mut h = harness();
    h.click(button_x(&h, "Debug"), 1);
    assert!(screen!(h).runs.borrow().is_empty());
}

#[test]
fn a_toggle_action_shows_pressed_and_flips_on_click() {
    let mut h = harness();
    // The separator is at x = 11; the toggle button follows it.
    let before = h.buffer()[(13, 1)].bg;
    h.click(13, 1);
    assert!(screen!(h).wrap.get());
    assert_ne!(
        h.buffer()[(13, 1)].bg,
        before,
        "toggled buttons keep the pressed look"
    );
}

#[test]
fn right_and_middle_clicks_do_not_perform_actions() {
    let mut h = harness();
    h.right_click(button_x(&h, "Run"), 1);
    h.middle_click(button_x(&h, "Run"), 1);
    assert!(screen!(h).runs.borrow().is_empty());
}

#[test]
fn a_drop_down_group_opens_an_action_popup_menu_below_the_button() {
    let mut h = harness();
    h.click(17, 1);
    assert!(screen!(h).menu.is_some());
    assert!(h.text().contains("Profile"));
    let (x, y) = h.find("Profile").expect("row");
    h.click(x, y);
    assert_eq!(*screen!(h).runs.borrow(), ["Profile"]);
}

#[test]
fn hovering_a_button_shows_its_tooltip_with_the_shortcut_after_500_ms() {
    let mut h = harness();
    let x = button_x(&h, "Run");
    h.hover(x, 1);
    h.advance(0.3);
    assert!(!h.rt.tooltip.is_showing());
    h.advance(0.25);
    assert_eq!(h.rt.tooltip.showing_title(), Some("Run"));
    h.hover(40, 10);
    h.advance(0.2);
    assert!(
        !h.rt.tooltip.is_showing(),
        "hides 150 ms after the pointer left"
    );
}

#[test]
fn hovering_a_button_publishes_the_description_for_the_status_bar() {
    let mut h = harness();
    h.hover(button_x(&h, "Stop"), 1);
    assert_eq!(h.rt.hint.as_deref(), Some("Stop the configuration"));
    h.hover(40, 10);
    assert_eq!(h.rt.hint, None);
}

#[test]
fn the_toolbar_is_not_in_the_focus_order() {
    let mut h = harness();
    for _ in 0..4 {
        h.press(KeyCode::Tab);
        if let Some(id) = h.rt.focus.current() {
            assert!(!h.rt.focus.order().is_empty() && id != BAR);
        }
    }
    assert!(h.rt.focus.order().iter().all(|id| *id != BAR));
}

#[test]
fn actions_that_do_not_fit_move_to_an_overflow_menu() {
    let mut h = harness_with(screen(true));
    // The bar is 30 cells wide: the last actions do not fit.
    let more = (2..32).find(|x| {
        h.buffer()[(*x, 1)].symbol() == archipel_kit_tui::glyphs::nerd_font(Glyph::More).to_string()
    });
    let x = more.expect("overflow button");
    h.click(x, 1);
    assert!(screen!(h).menu.is_some());
    assert!(h.text().contains("Delta"), "{}", h.text());
}

// ----- combo box -----

fn focus_combo(h: &mut Harness<ScreenApp>) {
    h.rt.focus.focus(COMBO);
    h.draw();
}

#[test]
fn up_and_down_change_the_selection_of_a_closed_combo_without_wrapping() {
    let mut h = harness();
    focus_combo(&mut h);
    h.press(KeyCode::Down);
    assert_eq!(screen!(h).combo_state.selected, 2);
    h.press(KeyCode::Down);
    assert_eq!(screen!(h).combo_state.selected, 2);
    h.press(KeyCode::Up);
    h.press(KeyCode::Up);
    h.press(KeyCode::Up);
    assert_eq!(screen!(h).combo_state.selected, 0);
}

#[test]
fn alt_down_f4_enter_and_space_open_the_list() {
    for (code, mods) in [
        (KeyCode::Down, KeyModifiers::ALT),
        (KeyCode::F(4), KeyModifiers::NONE),
        (KeyCode::Enter, KeyModifiers::NONE),
        (KeyCode::Char(' '), KeyModifiers::NONE),
    ] {
        let mut h = harness();
        focus_combo(&mut h);
        h.press_with(code, mods);
        assert!(screen!(h).list.is_some(), "{code:?}");
    }
}

#[test]
fn the_list_is_as_wide_as_the_field_and_opens_on_the_current_option() {
    let mut h = harness();
    focus_combo(&mut h);
    h.press(KeyCode::Enter);
    let rect = screen!(h)
        .list
        .as_ref()
        .expect("list")
        .chain
        .rect(0)
        .expect("rect");
    assert!(rect.width >= 20, "{rect:?}");
    assert_eq!(
        screen!(h).list.as_ref().expect("list").chain.selected(0),
        Some(1)
    );
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(screen!(h).combo_state.selected, 2);
    assert!(screen!(h).list.is_none());
}

#[test]
fn escape_closes_the_list_without_changing_the_selection() {
    let mut h = harness();
    focus_combo(&mut h);
    h.press(KeyCode::Enter);
    h.press(KeyCode::Down);
    h.press(KeyCode::Esc);
    assert_eq!(screen!(h).combo_state.selected, 1);
    assert!(screen!(h).list.is_none());
}

#[test]
fn a_click_on_the_field_opens_the_list_but_not_right_after_it_closed() {
    let mut h = harness();
    h.click(5, 6);
    assert!(screen!(h).list.is_some());
    h.press(KeyCode::Esc);
    h.click(5, 6);
    assert!(screen!(h).list.is_none(), "the 200 ms reopen guard");
    h.advance(0.3);
    h.click(5, 6);
    assert!(screen!(h).list.is_some());
}

// ----- tabs -----

#[test]
fn a_press_on_a_tab_selects_it_at_once() {
    let mut h = harness();
    h.send(Event::Mouse(archipel_kit_tui::input::MouseEvent {
        kind: archipel_kit_tui::input::MouseEventKind::Down(
            archipel_kit_tui::input::MouseButton::Left,
        ),
        column: 15,
        row: 3,
        modifiers: KeyModifiers::NONE,
    }));
    assert_eq!(screen!(h).tab_state.selected, 1);
    assert_eq!(screen!(h).tab_events, [TabsEvent::Selected(1)]);
}

#[test]
fn a_middle_click_closes_a_tab_without_selecting_it() {
    let mut h = harness();
    h.middle_click(15, 3);
    assert_eq!(screen!(h).tab_events, [TabsEvent::CloseRequested(1)]);
    assert_eq!(screen!(h).tab_state.selected, 0);
}

#[test]
fn shift_click_closes_a_tab_without_selecting_it() {
    let mut h = harness();
    h.click_with(15, 3, KeyModifiers::SHIFT);
    assert_eq!(screen!(h).tab_events, [TabsEvent::CloseRequested(1)]);
    assert_eq!(screen!(h).tab_state.selected, 0);
}

#[test]
fn a_pinned_tab_cannot_be_closed() {
    let mut h = harness();
    let (x, y) = h.find("pinned.rs").expect("tab");
    h.middle_click(x, y);
    h.click_with(x, y, KeyModifiers::SHIFT);
    assert!(screen!(h).tab_events.is_empty());
}

#[test]
fn the_close_icon_closes_after_the_pointer_rested_150_ms() {
    let mut h = harness();
    let layout_close = {
        let (x, y) = h.find("main.rs").expect("tab");
        (x + 9, y)
    };
    h.hover(layout_close.0, layout_close.1);
    h.advance(0.2);
    h.click(layout_close.0, layout_close.1);
    assert_eq!(screen!(h).tab_events, [TabsEvent::CloseRequested(0)]);
}

#[test]
fn a_click_on_the_close_icon_right_after_the_pointer_landed_is_ignored() {
    let mut h = harness();
    let (x, y) = h.find("main.rs").expect("tab");
    h.click(x + 9, y);
    assert!(
        screen!(h).tab_events.is_empty(),
        "{:?}",
        screen!(h).tab_events
    );
}

#[test]
fn a_right_press_asks_for_the_tab_menu() {
    let mut h = harness();
    h.right_click(15, 3);
    assert_eq!(
        screen!(h).tab_events,
        [TabsEvent::ContextMenu {
            index: 1,
            x: 15,
            y: 3
        }]
    );
}

#[test]
fn dragging_a_tab_past_its_neighbor_swaps_them_and_keeps_the_selection_on_it() {
    let mut h = harness();
    let (x, y) = h.find("main.rs").expect("tab");
    h.drag((x + 2, y), (x + 25, y));
    assert_eq!(screen!(h).tabs[0].label, "Cargo.toml");
    assert_eq!(screen!(h).tabs[1].label, "main.rs");
    assert_eq!(
        screen!(h).tab_state.selected,
        1,
        "the selected tab moved with the drag"
    );
    assert!(
        screen!(h)
            .tab_events
            .iter()
            .any(|e| matches!(e, TabsEvent::Reordered { from: 0, to: 1 }))
    );
}

#[test]
fn a_tiny_drag_inside_the_dead_zone_does_not_reorder() {
    let mut h = harness();
    let (x, y) = h.find("main.rs").expect("tab");
    h.drag((x + 2, y), (x + 3, y));
    assert_eq!(screen!(h).tabs[0].label, "main.rs");
}

#[test]
fn left_and_right_select_the_previous_and_next_tab_when_focused() {
    let mut h = harness();
    h.rt.focus.focus(TABS);
    h.press(KeyCode::Right);
    h.press(KeyCode::Right);
    assert_eq!(screen!(h).tab_state.selected, 2);
    h.press(KeyCode::Left);
    assert_eq!(screen!(h).tab_state.selected, 1);
    h.press(KeyCode::Left);
    h.press(KeyCode::Left);
    assert_eq!(screen!(h).tab_state.selected, 0, "no wrapping");
}

#[test]
fn next_and_previous_tab_actions_wrap() {
    let mut state = TabsState::new(3);
    assert_eq!(state.select_relative(4, 1), Some(0));
    assert_eq!(state.select_relative(4, -1), Some(3));
    assert_eq!(state.select_relative(0, 1), None);
}

#[test]
fn removing_a_tab_keeps_a_valid_selection() {
    let mut state = TabsState::new(2);
    state.removed(0, 3);
    assert_eq!(state.selected, 1);
    state.removed(1, 2);
    assert_eq!(state.selected, 1, "the next tab slides into the place");
    state.removed(1, 1);
    assert_eq!(state.selected, 0);
    state.removed(0, 0);
    assert_eq!(state.selected, 0);
}

#[test]
fn the_wheel_scrolls_a_strip_wider_than_its_area() {
    let mut s = screen(false);
    s.tabs = (0..12)
        .map(|i| TabInfo::new(format!("file{i}.rs")).icon(Language::Rust))
        .collect();
    let mut h = harness_with(s);
    assert!(!h.text().contains("file11.rs"));
    h.scroll(10, 3, 9);
    assert!(screen!(h).tab_state.offset() >= 5);
    assert!(h.text().contains("file11.rs"));
}

#[test]
fn the_strip_scrolls_to_keep_the_selected_tab_visible() {
    let mut s = screen(false);
    s.tabs = (0..12)
        .map(|i| TabInfo::new(format!("file{i}.rs")).icon(Language::Rust))
        .collect();
    s.tab_state = TabsState::new(11);
    let h = harness_with(s);
    assert!(h.text().contains("file11.rs"));
}

#[test]
fn toolbar_combo_and_tabs_survive_tiny_screens() {
    for (w, hgt) in [(1u16, 1u16), (4, 2), (20, 5)] {
        let mut h = Harness::new(
            app_fn(screen(true), event as EventFn, draw as DrawFn),
            w,
            hgt,
        );
        h.rt.focus.focus(COMBO);
        for code in [KeyCode::Down, KeyCode::Enter, KeyCode::Esc, KeyCode::Left] {
            h.press(code);
        }
        h.click(0, 0);
        h.right_click(w - 1, hgt - 1);
        h.drag((0, 0), (w - 1, hgt - 1));
        h.scroll(0, 0, 4);
        h.hover(w - 1, 0);
        h.advance(1.0);
    }
}

#[test]
fn toolbar_combo_and_tabs_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let mut h = Harness::with_runtime(
            app_fn(screen(false), event as EventFn, draw as DrawFn),
            Runtime::new(look),
            80,
            14,
        );
        h.hover(button_x(&h, "Run"), 1);
        h.advance(0.6);
        h.rt.focus.focus(COMBO);
        h.draw();
        snapshot(&format!("toolbar.{name}"), h.buffer());
    }
}
