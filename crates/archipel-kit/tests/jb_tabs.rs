//! `JBTabs` behavior from the IntelliJ spec (section 2): press selects, close clicks, the
//! close button's dead zone, arrow keys, dragging, pinned tabs, tab actions and the popup
//! group.

use archipel_kit::action_system::{
    ActionGroup, AnAction, AnActionEvent, DefaultActionGroup, Presentation,
};
use archipel_kit::egui::{self, Event, Key, Modifiers, PointerButton, pos2};
use archipel_kit::testing::{self, Harness, NodeT, Queryable};
use archipel_kit::tokens::tab;
use archipel_kit::ui::tabs::{JBTabs, TabInfo};
use archipel_kit::{Glyph, Theme};
use std::cell::RefCell;
use std::rc::Rc;

const NAMES: [&str; 4] = ["main.rs", "lib.rs", "theme.rs", "tokens.rs"];

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    selected: usize,
    closed: Vec<usize>,
    moved: Vec<(usize, usize)>,
    pinned: Vec<usize>,
    popup: Vec<usize>,
    performed: Vec<(Option<usize>, String)>,
    popup_group: Option<DefaultActionGroup>,
}

struct Recorder {
    id: &'static str,
    log: Rc<RefCell<Vec<String>>>,
}

impl AnAction for Recorder {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.id).icon(Glyph::Refresh)
    }

    fn action_performed(&self, e: &AnActionEvent<'_>) {
        self.log
            .borrow_mut()
            .push(format!("{} from {}", self.id, e.place));
    }
}

fn plain_tabs() -> Vec<TabInfo> {
    NAMES
        .iter()
        .map(|name| TabInfo::new(*name).closable(true))
        .collect()
}

fn harness_with(state: State) -> Harness<'static, State> {
    let mut harness = testing::harness_state(Theme::dark(), state, |ui, state| {
        ui.set_width(520.0);
        let State {
            tabs,
            selected,
            closed,
            moved,
            pinned,
            popup,
            performed,
            popup_group,
        } = state;
        let mut strip = JBTabs::new(tabs, *selected);
        if let Some(group) = popup_group.as_ref() {
            strip = strip.popup_group(group as &dyn ActionGroup);
        }
        let out = strip.show(ui);
        if let Some(index) = out.selected {
            *selected = index;
        }
        closed.extend(out.closed);
        moved.extend(out.moved);
        pinned.extend(out.pin_toggled);
        popup.extend(out.popup_requested);
        performed.extend(out.performed);
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH + 200.0, 260.0));
    harness.run();
    harness
}

fn harness() -> Harness<'static, State> {
    harness_with(State {
        tabs: plain_tabs(),
        ..State::default()
    })
}

fn tab_center(harness: &Harness<'_, State>, label: &str) -> egui::Pos2 {
    harness.get_by_label(label).rect().center()
}

fn press(
    harness: &mut Harness<'_, State>,
    pos: egui::Pos2,
    button: PointerButton,
    mods: Modifiers,
) {
    harness.hover_at(pos);
    harness.step();
    harness.event_modifiers(
        Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: mods,
        },
        mods,
    );
    harness.step();
}

fn release(
    harness: &mut Harness<'_, State>,
    pos: egui::Pos2,
    button: PointerButton,
    mods: Modifiers,
) {
    harness.event_modifiers(
        Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: mods,
        },
        mods,
    );
    harness.step();
}

#[test]
fn a_press_selects_the_tab_before_the_release() {
    let mut harness = harness();
    let pos = tab_center(&harness, "theme.rs");
    press(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.state().selected, 2, "selected on press");
    release(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.state().selected, 2);
}

#[test]
fn a_press_with_ctrl_or_alt_does_not_select() {
    let mut harness = harness();
    let pos = tab_center(&harness, "theme.rs");
    press(&mut harness, pos, PointerButton::Primary, Modifiers::CTRL);
    release(&mut harness, pos, PointerButton::Primary, Modifiers::CTRL);
    press(&mut harness, pos, PointerButton::Primary, Modifiers::ALT);
    assert_eq!(harness.state().selected, 0);
}

#[test]
fn a_middle_click_closes_a_tab_without_selecting_it() {
    let mut harness = harness();
    let pos = tab_center(&harness, "lib.rs");
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(harness.state().closed, [1]);
    assert_eq!(
        harness.state().selected,
        0,
        "the close click does not select"
    );
}

#[test]
fn shift_click_closes_a_tab_and_does_not_select_it() {
    let mut harness = harness();
    let pos = tab_center(&harness, "theme.rs");
    press(&mut harness, pos, PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(harness.state().closed, [2]);
    assert_eq!(harness.state().selected, 0);
    release(&mut harness, pos, PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(
        harness.state().closed,
        [2],
        "the release closes nothing more"
    );
}

#[test]
fn a_tab_that_is_not_closable_ignores_the_close_click() {
    let mut harness = harness_with(State {
        tabs: NAMES.iter().map(|name| TabInfo::new(*name)).collect(),
        ..State::default()
    });
    let pos = tab_center(&harness, "lib.rs");
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert!(harness.state().closed.is_empty());
}

#[test]
fn the_close_button_of_a_hovered_tab_ignores_clicks_for_150_ms() {
    let mut harness = harness();
    let tab = tab_center(&harness, "lib.rs");
    harness.hover_at(tab);
    harness.step();
    let close = harness.get_by_label("Close lib.rs").rect().center();
    // The pointer lands on the freshly shown button: the click is inside the dead zone.
    harness.hover_at(close);
    harness.step();
    harness.event(Event::PointerButton {
        pos: close,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    release(&mut harness, close, PointerButton::Primary, Modifiers::NONE);
    assert!(
        harness.state().closed.is_empty(),
        "clicked inside the dead zone"
    );
    // 0.15 s later (steps are 1/60 s), the same click closes it.
    for _ in 0..12 {
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: close,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    release(&mut harness, close, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.state().closed, [1]);
}

#[test]
fn the_selected_tabs_close_button_has_no_dead_zone() {
    let mut harness = harness();
    harness.get_by_label("Close main.rs").click();
    harness.run();
    assert_eq!(harness.state().closed, [0]);
}

#[test]
fn arrow_keys_on_a_focused_tab_select_the_neighbor_and_move_the_focus() {
    let mut harness = harness();
    harness.get_by_label("main.rs").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selected, 1);
    assert!(harness.get_by_label("lib.rs").accesskit_node().is_focused());
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selected, 2);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selected, 1);
    assert!(harness.get_by_label("lib.rs").accesskit_node().is_focused());
}

#[test]
fn arrow_keys_stop_at_the_ends_and_skip_disabled_tabs() {
    let mut tabs = plain_tabs();
    tabs[1] = TabInfo::new("lib.rs").closable(true).enabled(false);
    let mut harness = harness_with(State {
        tabs,
        ..State::default()
    });
    harness.get_by_label("main.rs").focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selected, 0, "nothing before the first tab");
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selected, 2, "the disabled tab is skipped");
}

#[test]
fn a_disabled_tab_cannot_be_selected_with_the_pointer() {
    let mut tabs = plain_tabs();
    tabs[2] = TabInfo::new("theme.rs").closable(true).enabled(false);
    let mut harness = harness_with(State {
        tabs,
        ..State::default()
    });
    let pos = tab_center(&harness, "theme.rs");
    press(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.state().selected, 0);
}

#[test]
fn a_tab_is_at_most_300_points_wide_and_cuts_its_title() {
    let long = "a_very_long_file_name_that_goes_on_and_on_and_on_and_on_and_on_and_on_and_on_and_on_and_on_and_on_and_on.rs";
    let mut tabs = plain_tabs();
    tabs[0] = TabInfo::new(long).closable(true);
    let mut harness =
        testing::harness_state(Theme::dark(), (tabs, 0usize), |ui, (tabs, selected)| {
            ui.set_width(900.0);
            JBTabs::new(tabs, *selected).show(ui);
        });
    harness.set_size(egui::vec2(940.0, 120.0));
    harness.run();
    let first = harness.get_by_label(long).rect();
    let second = harness.get_by_label("lib.rs").rect();
    // The next pill starts one gap after the first one ends.
    let width = second.left() - first.left();
    assert!(
        width <= tab::MAX_WIDTH + tab::GAP + 40.0,
        "the first tab is {width} wide"
    );
    assert!(width >= tab::MAX_WIDTH - 40.0);
}

#[test]
fn dragging_a_tab_past_its_neighbors_reports_the_new_place_on_release() {
    let mut harness = harness();
    let from = tab_center(&harness, "main.rs");
    // The dragged tab's center ends over the third tab's left half, which is where the
    // third tab (in a strip without the dragged one) has its center.
    let to = pos2(tab_center(&harness, "theme.rs").x - 20.0, from.y);
    press(&mut harness, from, PointerButton::Primary, Modifiers::NONE);
    // Past the 7 px dead zone, in steps.
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        harness.hover_at(pos2(from.x + (to.x - from.x) * t, from.y));
        harness.step();
    }
    assert!(
        harness.state().moved.is_empty(),
        "nothing is reported until the release"
    );
    release(&mut harness, to, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.state().moved, [(0, 2)]);
}

#[test]
fn a_press_that_moves_less_than_7_points_is_not_a_drag() {
    let mut harness = harness();
    let from = tab_center(&harness, "main.rs");
    press(&mut harness, from, PointerButton::Primary, Modifiers::NONE);
    let near = pos2(from.x + 6.0, from.y);
    harness.hover_at(near);
    harness.step();
    release(&mut harness, near, PointerButton::Primary, Modifiers::NONE);
    assert!(harness.state().moved.is_empty());
}

#[test]
fn tabs_that_are_not_reorderable_do_not_drag() {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (plain_tabs(), 0usize, Vec::<(usize, usize)>::new()),
        |ui, (tabs, selected, moved)| {
            ui.set_width(520.0);
            let out = JBTabs::new(tabs, *selected).reorderable(false).show(ui);
            moved.extend(out.moved);
        },
    );
    harness.set_size(egui::vec2(560.0, 120.0));
    harness.run();
    let from = harness.get_by_label("main.rs").rect().center();
    let to = pos2(
        harness.get_by_label("theme.rs").rect().center().x + 20.0,
        from.y,
    );
    harness.hover_at(from);
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        harness.hover_at(pos2(from.x + (to.x - from.x) * t, from.y));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    assert!(harness.state().2.is_empty());
}

#[test]
fn a_pinned_tab_shows_its_pin_always_and_the_close_click_leaves_it_open() {
    let mut tabs = plain_tabs();
    tabs[1] = TabInfo::new("lib.rs").closable(true).pinned(true);
    let mut harness = harness_with(State {
        tabs,
        ..State::default()
    });
    // Not selected, not hovered: the pin is there.
    assert!(harness.query_by_label("Unpin lib.rs").is_some());
    let pos = tab_center(&harness, "lib.rs");
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert!(harness.state().closed.is_empty());
    harness.get_by_label("Unpin lib.rs").hover();
    for _ in 0..12 {
        harness.step();
    }
    harness.get_by_label("Unpin lib.rs").click();
    harness.run();
    assert_eq!(harness.state().pinned, [1]);
}

#[test]
fn tab_actions_run_from_the_action_group_in_the_editor_tab_place() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut group = DefaultActionGroup::new("Tab actions");
    group.add(Recorder {
        id: "Reload",
        log: log.clone(),
    });
    let mut tabs = plain_tabs();
    tabs[0] = TabInfo::new("main.rs")
        .closable(true)
        .actions(Rc::new(group));
    let mut harness = harness_with(State {
        tabs,
        ..State::default()
    });
    // The selected tab shows its actions; the button is named by the action's text.
    harness.get_by_label("Reload").click();
    harness.run();
    assert_eq!(*log.borrow(), ["Reload from EditorTab"]);
    assert_eq!(harness.state().performed, [(Some(0), "Reload".to_owned())]);
    assert_eq!(
        harness.state().selected,
        0,
        "the press on a button does not select"
    );
}

#[test]
fn a_right_click_opens_the_popup_group_and_runs_its_action() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut group = DefaultActionGroup::popup("Tab popup");
    group.add(Recorder {
        id: "Close Others",
        log: log.clone(),
    });
    let mut harness = harness_with(State {
        tabs: plain_tabs(),
        popup_group: Some(group),
        ..State::default()
    });
    let pos = tab_center(&harness, "lib.rs");
    press(&mut harness, pos, PointerButton::Secondary, Modifiers::NONE);
    release(&mut harness, pos, PointerButton::Secondary, Modifiers::NONE);
    harness.run();
    assert_eq!(harness.state().popup, [1]);
    assert_eq!(harness.state().selected, 0, "a right click does not select");
    harness.get_by_label("Close Others").click();
    harness.run();
    assert_eq!(*log.borrow(), ["Close Others from EditorTabPopup"]);
    assert_eq!(
        harness.state().performed,
        [(None, "Close Others".to_owned())]
    );
    assert!(
        harness.query_by_label("Close Others").is_none(),
        "the menu closes"
    );
}
