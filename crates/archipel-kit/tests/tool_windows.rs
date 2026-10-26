//! Tool windows (IntelliJ spec section 8): header mouse and actions, content tabs, stripe
//! buttons, keyboard and the events the pane reports.

use archipel_kit::action_system::{
    ActionGroup, AnAction, AnActionEvent, DataContext, DefaultActionGroup, Presentation,
};
use archipel_kit::egui::{self, Event, Key, Modifiers, PointerButton, pos2};
use archipel_kit::testing::{Harness, Queryable};
use archipel_kit::ui::{Button, JBTextField};
use archipel_kit::wm::{
    Content, ToolWindow, ToolWindowAnchor, ToolWindowEvent, ToolWindowPane, ToolWindowPaneState,
    ToolWindowPlace,
};
use archipel_kit::{Glyph, Theme};
use std::cell::RefCell;
use std::rc::Rc;

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

struct App {
    pane: ToolWindowPaneState,
    problems: ToolWindow,
    title_group: DefaultActionGroup,
    options: DefaultActionGroup,
    context: DataContext,
    log: Rc<RefCell<Vec<String>>>,
    search: String,
    events: Vec<ToolWindowEvent>,
}

fn app_state() -> App {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut title_group = DefaultActionGroup::new("Title");
    title_group.add(Recorder {
        id: "Reload",
        log: log.clone(),
    });
    let mut options = DefaultActionGroup::popup("Options");
    options.add(Recorder {
        id: "Pin Tab",
        log: log.clone(),
    });
    let mut problems = ToolWindow::new(
        "problems",
        "Problems",
        Glyph::Warning,
        ToolWindowAnchor::Bottom,
    )
    .activate_number(6);
    problems.contents.add_content(Content::new("file", "File"));
    problems
        .contents
        .add_content(Content::new("project", "Project Errors"));
    App {
        pane: ToolWindowPaneState::default(),
        problems,
        title_group,
        options,
        context: DataContext::empty(),
        log,
        search: String::new(),
        events: Vec::new(),
    }
}

fn harness() -> Harness<'static, App> {
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 520.0))
        .with_step_dt(1.0 / 60.0)
        .build_state(
            move |ctx, app| {
                if !installed {
                    archipel_kit::install(ctx, Theme::dark());
                    installed = true;
                }
                let App {
                    pane,
                    problems,
                    title_group,
                    options,
                    context,
                    search,
                    ..
                } = app;
                ToolWindowPane::new("tw", pane)
                    .tool(ToolWindowAnchor::Left, "Project", Glyph::Folder)
                    .activate_with(ToolWindowAnchor::Left, 1)
                    .header_actions(
                        ToolWindowAnchor::Left,
                        title_group as &dyn ActionGroup,
                        context,
                    )
                    .options_menu(ToolWindowAnchor::Left, options as &dyn ActionGroup)
                    .tool(ToolWindowAnchor::Right, "Notifications", Glyph::Bell)
                    .activate_with(ToolWindowAnchor::Right, 2)
                    .tool_window(problems)
                    .show_with(ctx, search, |place, ui, search| match place {
                        ToolWindowPlace::Center => {
                            ui.label("Editor");
                        }
                        ToolWindowPlace::Tool(ToolWindowAnchor::Left) => {
                            ui.add(Button::secondary("Inside"));
                            ui.add(JBTextField::new(search).label("Search"));
                        }
                        ToolWindowPlace::Tool(ToolWindowAnchor::Right) => {
                            ui.label("No notifications");
                        }
                        ToolWindowPlace::Tool(ToolWindowAnchor::Bottom) => {
                            ui.label("No problems");
                        }
                        _ => {}
                    });
                let taken = pane.take_events();
                app.events.extend(taken);
            },
            app_state(),
        );
    harness.run();
    harness
}

fn events(harness: &mut Harness<'_, App>) -> Vec<ToolWindowEvent> {
    std::mem::take(&mut harness.state_mut().events)
}

/// A point on the left tool window's header, away from its buttons.
fn header_point(harness: &Harness<'_, App>) -> egui::Pos2 {
    // The left island starts at 42; its buttons take the right half of the header.
    let hide = harness.get_by_label("Hide Project").rect();
    pos2(70.0, hide.center().y)
}

fn press(harness: &mut Harness<'_, App>, pos: egui::Pos2, button: PointerButton, mods: Modifiers) {
    harness.hover_at(pos);
    harness.step();
    for pressed in [true, false] {
        harness.event_modifiers(
            Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: mods,
            },
            mods,
        );
        harness.step();
    }
    harness.run();
}

fn wait(harness: &mut Harness<'_, App>) {
    for _ in 0..40 {
        harness.step();
    }
}

#[test]
fn a_click_on_the_header_activates_the_tool_window() {
    let mut harness = harness();
    let pos = header_point(&harness);
    press(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    assert!(events(&mut harness).contains(&ToolWindowEvent::Activated(ToolWindowAnchor::Left)));
}

#[test]
fn a_double_click_on_the_header_toggles_maximize() {
    let mut harness = harness();
    let pos = header_point(&harness);
    press(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    press(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.state().pane.maximized, Some(ToolWindowAnchor::Left));
    assert!(
        events(&mut harness).contains(&ToolWindowEvent::MaximizeToggled(
            ToolWindowAnchor::Left,
            true
        ))
    );
}

#[test]
fn a_middle_click_or_shift_click_on_the_header_hides_the_tool_window() {
    let mut harness = harness();
    let pos = header_point(&harness);
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Left));
    assert!(events(&mut harness).contains(&ToolWindowEvent::Hidden(ToolWindowAnchor::Left)));

    let mut harness = self::harness();
    let pos = header_point(&harness);
    press(&mut harness, pos, PointerButton::Primary, Modifiers::SHIFT);
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Left));
    let seen = events(&mut harness);
    assert!(seen.contains(&ToolWindowEvent::Hidden(ToolWindowAnchor::Left)));
    assert!(
        !seen.contains(&ToolWindowEvent::Activated(ToolWindowAnchor::Left)),
        "a close click does not activate"
    );
}

#[test]
fn header_actions_come_from_the_action_group_in_the_title_place() {
    let mut harness = harness();
    let pos = header_point(&harness);
    harness.hover_at(pos);
    harness.run();
    harness.get_by_label("Reload").click();
    harness.run();
    assert_eq!(
        *harness.state().log.borrow(),
        ["Reload from ToolwindowTitle"]
    );
}

#[test]
fn the_gear_opens_the_options_menu_and_runs_its_action_in_the_popup_place() {
    let mut harness = harness();
    harness.hover_at(header_point(&harness));
    harness.run();
    harness.get_by_label("Options").click();
    harness.run();
    harness.get_by_label("Pin Tab").click();
    harness.run();
    assert_eq!(
        *harness.state().log.borrow(),
        ["Pin Tab from ToolwindowPopup"]
    );
    assert!(
        harness.query_by_label("Pin Tab").is_none(),
        "the menu closes"
    );
}

#[test]
fn a_right_click_on_the_header_opens_the_options_menu() {
    let mut harness = harness();
    let pos = header_point(&harness);
    press(&mut harness, pos, PointerButton::Secondary, Modifiers::NONE);
    harness.run();
    assert!(harness.query_by_label("Pin Tab").is_some());
}

#[test]
fn esc_in_a_tool_window_asks_to_return_to_the_editor() {
    let mut harness = harness();
    harness.get_by_label("Inside").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(events(&mut harness).contains(&ToolWindowEvent::FocusEditor(ToolWindowAnchor::Left)));
}

#[test]
fn esc_in_a_text_field_is_the_fields_and_does_not_leave_the_tool_window() {
    let mut harness = harness();
    harness.get_by_label("Search").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(
        !events(&mut harness)
            .iter()
            .any(|event| matches!(event, ToolWindowEvent::FocusEditor(_)))
    );
}

#[test]
fn shift_esc_hides_the_active_tool_window() {
    let mut harness = harness();
    harness.get_by_label("Inside").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Escape);
    harness.run();
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Left));
    let seen = events(&mut harness);
    assert!(seen.contains(&ToolWindowEvent::Hidden(ToolWindowAnchor::Left)));
    assert!(
        !seen
            .iter()
            .any(|event| matches!(event, ToolWindowEvent::FocusEditor(_))),
        "Shift+Esc hides, it does not also return to the editor"
    );
}

#[test]
fn ctrl_shift_f12_hides_every_tool_window() {
    let mut harness = harness();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::F12);
    harness.run();
    let pane = &harness.state().pane;
    assert!(
        !pane.is_visible(ToolWindowAnchor::Left)
            && !pane.is_visible(ToolWindowAnchor::Right)
            && !pane.is_visible(ToolWindowAnchor::Bottom)
    );
    assert!(events(&mut harness).contains(&ToolWindowEvent::AllHidden));
}

#[test]
fn alt_digit_shows_a_hidden_window_hides_the_active_one_and_activates_a_visible_one() {
    let mut harness = harness();
    harness.state_mut().pane.hide(ToolWindowAnchor::Left);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Num1);
    harness.run();
    assert!(harness.state().pane.is_visible(ToolWindowAnchor::Left));
    assert!(events(&mut harness).contains(&ToolWindowEvent::Activated(ToolWindowAnchor::Left)));

    // Visible, focus elsewhere: activates.
    harness.key_press_modifiers(Modifiers::ALT, Key::Num1);
    harness.run();
    assert!(harness.state().pane.is_visible(ToolWindowAnchor::Left));
    assert!(events(&mut harness).contains(&ToolWindowEvent::Activated(ToolWindowAnchor::Left)));

    // Visible and active: hides.
    harness.get_by_label("Inside").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Num1);
    harness.run();
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Left));
    assert!(events(&mut harness).contains(&ToolWindowEvent::Hidden(ToolWindowAnchor::Left)));
}

#[test]
fn ctrl_shift_quote_maximizes_the_active_tool_window() {
    let mut harness = harness();
    harness.get_by_label("Inside").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Quote);
    harness.run();
    assert_eq!(harness.state().pane.maximized, Some(ToolWindowAnchor::Left));
}

#[test]
fn the_stripe_tooltip_shows_the_activation_shortcut() {
    let mut harness = harness();
    let button = harness.get_by_label("Project").rect().center();
    harness.hover_at(button);
    harness.step();
    harness.step();
    let shown = harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => text.galley.text() == "Alt+1",
            _ => false,
        });
    assert!(shown, "the tooltip names Alt+1");
}

#[test]
fn a_middle_click_or_shift_click_on_a_stripe_button_hides_without_toggling_back() {
    let mut harness = harness();
    let pos = harness.get_by_label("Notifications").rect().center();
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Right));
    // Closed windows stay closed for a close click.
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Right));
    harness.state_mut().pane.show(ToolWindowAnchor::Right);
    harness.run();
    wait(&mut harness);
    press(&mut harness, pos, PointerButton::Primary, Modifiers::SHIFT);
    assert!(!harness.state().pane.is_visible(ToolWindowAnchor::Right));
}

#[test]
fn a_click_on_a_stripe_button_toggles_and_reports_activation() {
    let mut harness = harness();
    harness.state_mut().pane.hide(ToolWindowAnchor::Right);
    harness.run();
    let pos = harness.get_by_label("Notifications").rect().center();
    press(&mut harness, pos, PointerButton::Primary, Modifiers::NONE);
    assert!(harness.state().pane.is_visible(ToolWindowAnchor::Right));
    assert!(events(&mut harness).contains(&ToolWindowEvent::Activated(ToolWindowAnchor::Right)));
}

#[test]
fn a_right_click_on_a_stripe_button_reports_its_menu() {
    let mut harness = harness();
    let pos = harness.get_by_label("Project").rect().center();
    press(&mut harness, pos, PointerButton::Secondary, Modifiers::NONE);
    assert!(events(&mut harness).iter().any(|event| matches!(
        event,
        ToolWindowEvent::StripeMenuRequested(ToolWindowAnchor::Left, _)
    )));
}

fn drag(harness: &mut Harness<'_, App>, from: egui::Pos2, to: egui::Pos2, release: bool) {
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
        harness.hover_at(pos2(
            from.x + (to.x - from.x) * t,
            from.y + (to.y - from.y) * t,
        ));
        harness.step();
    }
    if release {
        harness.event(Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        harness.run();
    }
}

#[test]
fn dragging_a_stripe_button_to_the_other_group_reports_the_move() {
    let mut harness = harness();
    let from = harness.get_by_label("Project").rect().center();
    // The bottom half of the left stripe is the bottom tool window's group.
    let to = pos2(from.x, 480.0);
    drag(&mut harness, from, to, true);
    assert!(
        events(&mut harness).contains(&ToolWindowEvent::MovedToAnchor {
            window: ToolWindowAnchor::Left,
            to: ToolWindowAnchor::Bottom,
        })
    );
    assert!(
        harness.state().pane.is_visible(ToolWindowAnchor::Left),
        "a drag does not toggle"
    );
}

#[test]
fn a_stripe_drag_shorter_than_7_points_is_not_a_drag() {
    let mut harness = harness();
    let from = harness.get_by_label("Project").rect().center();
    drag(&mut harness, from, pos2(from.x, from.y + 5.0), true);
    assert!(
        !events(&mut harness)
            .iter()
            .any(|event| matches!(event, ToolWindowEvent::MovedToAnchor { .. }))
    );
}

#[test]
fn esc_cancels_a_stripe_drag() {
    let mut harness = harness();
    let from = harness.get_by_label("Project").rect().center();
    let to = pos2(from.x, 480.0);
    drag(&mut harness, from, to, false);
    harness.key_press(Key::Escape);
    harness.step();
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.run();
    assert!(
        !events(&mut harness)
            .iter()
            .any(|event| matches!(event, ToolWindowEvent::MovedToAnchor { .. }))
    );
}

#[test]
fn content_tabs_come_from_the_content_manager_and_write_the_selection_back() {
    let mut harness = harness();
    assert_eq!(harness.state().problems.contents.selected_index(), Some(0));
    harness.get_by_label("Project Errors").click();
    harness.run();
    assert_eq!(harness.state().problems.contents.selected_index(), Some(1));
}

#[test]
fn a_middle_click_on_a_closeable_content_tab_removes_the_content_and_reports_it() {
    let mut harness = harness();
    let pos = harness.get_by_label("File").rect().center();
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(harness.state().problems.contents.len(), 1);
    assert_eq!(
        harness.state().problems.contents.contents()[0].display_name,
        "Project Errors"
    );
    assert!(
        events(&mut harness).contains(&ToolWindowEvent::ContentCloseRequested(
            ToolWindowAnchor::Bottom,
            0
        ))
    );
}

#[test]
fn a_pinned_content_tab_cannot_be_closed() {
    let mut harness = harness();
    harness.state_mut().problems.contents = {
        let mut contents = archipel_kit::wm::ContentManager::new();
        contents.add_content(Content::new("file", "File").pinned(true));
        contents.add_content(Content::new("project", "Project Errors"));
        contents
    };
    harness.run();
    let pos = harness.get_by_label("File").rect().center();
    press(&mut harness, pos, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(harness.state().problems.contents.len(), 2);
}
