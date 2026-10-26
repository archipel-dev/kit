//! Event-driven tests of the layer stack, dialogs, balloons, the status bar and which-key.

#![allow(clippy::unnecessary_mut_passed)]

use archipel_kit::notification::{
    Notification, NotificationAction, NotificationEventKind, NotificationGroup,
};
use archipel_kit::openapi::ui::{ExitCode, ValidationInfo};
use archipel_kit::ui::Status;
use archipel_kit::ui::popup::BaseListPopupStep;
use archipel_kit_tui::balloon::Balloons;
use archipel_kit_tui::components::{Diagnostics, Pane, PaneKind, StatusBar, StatusSlot};
use archipel_kit_tui::controls::{TextFieldControl, TextFieldState};
use archipel_kit_tui::dialog::{Dialog, DialogContent, Messages};
use archipel_kit_tui::input::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use archipel_kit_tui::overlay::{Handled, Layers, Overlay, PopupOverlay, draw_which_key};
use archipel_kit_tui::popup::{ListPopup, PopupState};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::statusbar::{StatusBarControl, StatusEvent};
use archipel_kit_tui::testing::{FnApp, Harness, app_fn, snapshot};
use archipel_kit_tui::tooltip::Tooltip;
use archipel_kit_tui::{Event, Id, Look, Runtime};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Exit(ExitCode),
    Chosen(&'static str),
    PopupCancelled,
}

struct Shared(Rc<RefCell<Balloons>>);

impl Overlay<Msg> for Shared {
    fn modal(&self) -> bool {
        false
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<Msg> {
        <Balloons as Overlay<Msg>>::event(&mut self.0.borrow_mut(), rt, event)
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        <Balloons as Overlay<Msg>>::draw(&mut self.0.borrow_mut(), rt, area, buf);
    }

    fn tick(&mut self, rt: &mut Runtime) -> Vec<Msg> {
        <Balloons as Overlay<Msg>>::tick(&mut self.0.borrow_mut(), rt)
    }
}

struct Screen {
    layers: Layers<Msg>,
    log: Vec<Msg>,
    app_keys: u32,
    app_clicks: u32,
    status: StatusBarControl,
    status_events: Vec<StatusEvent>,
    balloons: Rc<RefCell<Balloons>>,
}

fn screen() -> Screen {
    let balloons = Rc::new(RefCell::new(Balloons::new()));
    let mut layers = Layers::new();
    layers.push_ambient(Shared(balloons.clone()));
    Screen {
        layers,
        log: Vec::new(),
        app_keys: 0,
        app_clicks: 0,
        status: StatusBarControl::new(Id::new("status")).tooltip(
            StatusSlot::Cursor,
            Tooltip::new("Go to Line").shortcut("Ctrl+G"),
        ),
        status_events: Vec::new(),
        balloons,
    }
}

fn event(s: &mut Screen, rt: &mut Runtime, e: &Event) {
    let (consumed, messages) = s.layers.event(rt, e);
    s.log.extend(messages);
    if consumed {
        return;
    }
    match e {
        Event::Key(_) => s.app_keys += 1,
        Event::Mouse(_)
            if rt.pointer().is_some_and(|p| {
                matches!(p.kind, archipel_kit_tui::pointer::PointerKind::Down(_))
            }) =>
        {
            s.app_clicks += 1;
        }
        _ => {}
    }
    let r = s.status.handle(rt, e);
    if r != StatusEvent::None {
        s.status_events.push(r);
    }
}

fn draw(s: &mut Screen, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    Pane::new(&rt.look).kind(PaneKind::Editor).render(area, buf);
    let bar = Rect::new(0, area.bottom().saturating_sub(1), area.width, 1);
    let look = rt.look.clone();
    let bar_widget = StatusBar::new(&look)
        .mode("NORMAL", look.palette.nord_blue)
        .diagnostics(Diagnostics::default())
        .cursor("Ln 1, Col 1");
    s.status.show(rt, bar_widget, bar, buf);
    for m in s.layers.tick(rt) {
        s.log.push(m);
    }
    s.layers.draw(rt, area, buf);
    draw_which_key(rt, area, buf, &|id| id.to_owned());
}

type EventFn = fn(&mut Screen, &mut Runtime, &Event);
type DrawFn = fn(&mut Screen, &mut Runtime, Rect, &mut Buffer);
type ScreenApp = FnApp<Screen, EventFn, DrawFn>;

fn harness() -> Harness<ScreenApp> {
    Harness::new(app_fn(screen(), event as EventFn, draw as DrawFn), 100, 30)
}

macro_rules! screen {
    ($h:expr) => {
        &mut $h.app.state
    };
}

struct NameContent {
    field: TextFieldState,
}

impl DialogContent for NameContent {
    fn height(&self, _width: u16) -> u16 {
        1
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        TextFieldControl::new(Id::new("name"))
            .label("Name")
            .show(rt, &self.field, area, buf);
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> bool {
        use archipel_kit_tui::controls::FieldEvent;
        matches!(
            TextFieldControl::new(Id::new("name")).handle(rt, &mut self.field, event),
            FieldEvent::Changed | FieldEvent::Consumed | FieldEvent::Cleared
        )
    }

    fn revision(&self) -> u64 {
        self.field.revision
    }

    fn validate(&self) -> Vec<ValidationInfo> {
        let text = self.field.text();
        if text.is_empty() {
            vec![ValidationInfo::error("Name must not be empty")]
        } else if text.contains(' ') {
            vec![ValidationInfo::warning("Spaces are not recommended")]
        } else {
            Vec::new()
        }
    }

    fn preferred_focus(&self) -> Option<Id> {
        Some(Id::new("name"))
    }
}

fn push_dialog(h: &mut Harness<ScreenApp>, dialog: Dialog<Msg>) {
    let rt = &h.rt;
    screen!(h).layers.push(rt, dialog);
    h.draw();
}

fn ok_cancel() -> Dialog<Msg> {
    Messages::ok_cancel_dialog("Delete", "Delete the file?").on_exit(|c| vec![Msg::Exit(c)])
}

fn exits(h: &mut Harness<ScreenApp>) -> Vec<Msg> {
    screen!(h).log.clone()
}

// ----- layers -----

fn push_popup(h: &mut Harness<ScreenApp>) {
    let step = BaseListPopupStep::new("Pick", vec!["one", "two"]);
    let mut host = ListPopup::list(step);
    host.open(&h.rt, true);
    let overlay = PopupOverlay::new(host, |source, state| match state {
        PopupState::Chosen => source.take_chosen().map(Msg::Chosen).into_iter().collect(),
        PopupState::Cancelled => vec![Msg::PopupCancelled],
        PopupState::Open => Vec::new(),
    });
    let rt = &h.rt;
    screen!(h).layers.push(rt, overlay);
    h.draw();
}

#[test]
fn escape_unwinds_the_top_layer_first() {
    let mut h = harness();
    push_popup(&mut h);
    push_dialog(&mut h, ok_cancel());
    assert_eq!(screen!(h).layers.len(), 2);
    h.press(KeyCode::Esc);
    assert_eq!(screen!(h).layers.len(), 1, "the dialog went first");
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Dismissed)]);
    h.press(KeyCode::Esc);
    assert_eq!(screen!(h).layers.len(), 0);
    assert_eq!(screen!(h).app_keys, 0, "the app never saw the Esc keys");
}

#[test]
fn a_modal_layer_swallows_keys_and_clicks_meant_for_the_app() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.press(KeyCode::Char('x'));
    h.click(2, 2);
    assert_eq!(screen!(h).app_keys, 0);
    assert_eq!(screen!(h).app_clicks, 0);
}

#[test]
fn nothing_below_a_modal_layer_is_hit_or_focused() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    let (x, y) = (3, 29);
    assert_ne!(
        h.rt.hits.hit(x, y),
        Some(Id::new("status")),
        "the status bar is blocked"
    );
}

#[test]
fn the_layer_is_gone_and_the_app_gets_keys_again() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.press(KeyCode::Enter);
    assert_eq!(screen!(h).layers.len(), 0);
    h.press(KeyCode::Char('x'));
    assert_eq!(screen!(h).app_keys, 1);
}

// ----- dialog -----

#[test]
fn enter_presses_the_default_button() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.press(KeyCode::Enter);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Primary)]);
}

#[test]
fn escape_cancels_a_dialog() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.press(KeyCode::Esc);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Dismissed)]);
}

#[test]
fn arrows_move_between_buttons_when_a_button_has_focus_and_enter_presses_the_focused_one() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    // The primary button has the initial focus; Left goes to Cancel.
    h.press(KeyCode::Left);
    h.press(KeyCode::Enter);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Secondary)]);
}

#[test]
fn y_and_n_answer_a_yes_no_dialog() {
    for (key, code) in [('y', ExitCode::Primary), ('N', ExitCode::Secondary)] {
        let mut h = harness();
        push_dialog(
            &mut h,
            Messages::yes_no_dialog("Save", "Save changes?").on_exit(|c| vec![Msg::Exit(c)]),
        );
        h.press(KeyCode::Char(key));
        assert_eq!(exits(&mut h), [Msg::Exit(code)]);
    }
}

#[test]
fn y_does_nothing_in_a_dialog_without_yes_no_keys() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.press(KeyCode::Char('y'));
    assert!(exits(&mut h).is_empty());
}

#[test]
fn clicking_a_button_presses_it_and_clicking_outside_dismisses() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    let (x, y) = h.find("Cancel").expect("button");
    h.click(x + 1, y);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Secondary)]);
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.click(1, 1);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Dismissed)]);
}

#[test]
fn alt_plus_the_mnemonic_presses_a_button() {
    let mut h = harness();
    push_dialog(&mut h, ok_cancel());
    h.alt('c');
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Secondary)]);
}

#[test]
fn tab_cycles_through_the_content_and_the_buttons() {
    let mut h = harness();
    let dialog = Dialog::new("Rename")
        .primary("OK")
        .secondary("Cancel")
        .content(NameContent {
            field: TextFieldState::with_text("a"),
        })
        .on_exit(|c| vec![Msg::Exit(c)]);
    push_dialog(&mut h, dialog);
    assert!(
        h.rt.focused(Id::new("name")),
        "the field has the initial focus"
    );
    h.press(KeyCode::Tab);
    h.press(KeyCode::Tab);
    h.press(KeyCode::Enter);
    assert_eq!(
        exits(&mut h),
        [Msg::Exit(ExitCode::Primary)],
        "field, Cancel, OK"
    );
}

#[test]
fn typing_goes_to_the_content_and_enter_submits_the_dialog() {
    let mut h = harness();
    let dialog = Dialog::new("Rename")
        .primary("OK")
        .secondary("Cancel")
        .content(NameContent {
            field: TextFieldState::new(),
        })
        .on_exit(|c| vec![Msg::Exit(c)]);
    push_dialog(&mut h, dialog);
    h.type_text("main");
    assert!(h.text().contains("main"));
    h.advance(0.4);
    h.press(KeyCode::Enter);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Primary)]);
}

#[test]
fn validation_runs_300_ms_after_the_last_change_and_blocks_the_primary_button() {
    let mut h = harness();
    let dialog = Dialog::new("Rename")
        .primary("OK")
        .secondary("Cancel")
        .content(NameContent {
            field: TextFieldState::with_text("x"),
        })
        .on_exit(|c| vec![Msg::Exit(c)]);
    push_dialog(&mut h, dialog);
    h.advance(0.01);
    h.press(KeyCode::Backspace);
    h.advance(0.2);
    assert!(!h.text().contains("must not be empty"), "not yet");
    h.advance(0.15);
    assert!(h.text().contains("Name must not be empty"));
    h.press(KeyCode::Enter);
    assert!(exits(&mut h).is_empty(), "OK is blocked by the error");
    h.type_text("ok");
    h.advance(0.4);
    assert!(!h.text().contains("must not be empty"));
    h.press(KeyCode::Enter);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Primary)]);
}

#[test]
fn a_warning_does_not_block_the_primary_button() {
    let mut h = harness();
    let dialog = Dialog::new("Rename")
        .primary("OK")
        .content(NameContent {
            field: TextFieldState::with_text("a b"),
        })
        .on_exit(|c| vec![Msg::Exit(c)]);
    push_dialog(&mut h, dialog);
    h.advance(0.4);
    assert!(h.text().contains("Spaces are not recommended"));
    h.press(KeyCode::Enter);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Primary)]);
}

#[test]
fn the_tertiary_button_is_at_the_left_and_the_primary_at_the_right() {
    let mut h = harness();
    let dialog = Dialog::new("Settings")
        .primary("Apply")
        .secondary("Cancel")
        .tertiary("Reset")
        .on_exit(|c| vec![Msg::Exit(c)]);
    push_dialog(&mut h, dialog);
    let reset = h.find("Reset").expect("tertiary").0;
    let cancel = h.find("Cancel").expect("secondary").0;
    let apply = h.find("Apply").expect("primary").0;
    assert!(reset < cancel && cancel < apply);
    h.click(reset + 1, h.find("Reset").expect("tertiary").1);
    assert_eq!(exits(&mut h), [Msg::Exit(ExitCode::Tertiary)]);
}

#[test]
fn message_dialogs_show_their_title_icon_and_message() {
    let mut h = harness();
    push_dialog(
        &mut h,
        Messages::error_dialog("Build failed", "The build failed with 3 errors."),
    );
    let text = h.text();
    assert!(text.contains("Build failed") && text.contains("3 errors") && text.contains("OK"));
}

// ----- balloons -----

fn notify(h: &mut Harness<ScreenApp>, title: &str, sticky: bool) {
    let group = if sticky {
        NotificationGroup::sticky_balloon_group("g")
    } else {
        NotificationGroup::balloon_group("g")
    };
    let n = Notification::new(Status::Info, title)
        .group(group)
        .action("Open")
        .notification_action(NotificationAction::new("Keep").keep_notification(true));
    screen!(h)
        .balloons
        .borrow_mut()
        .notify(n, Some("Something happened."));
    h.draw();
}

fn balloon_count(h: &Harness<ScreenApp>) -> usize {
    h.app.state.balloons.borrow().manager.visible().len()
}

#[test]
fn a_balloon_appears_in_the_top_right_corner() {
    let mut h = harness();
    notify(&mut h, "Indexing finished", false);
    let (x, y) = h.find("Indexing finished").expect("balloon");
    assert!(x > 50 && y <= 3, "{x},{y}");
    assert!(h.text().contains("Something happened."));
}

#[test]
fn a_balloon_hides_after_ten_seconds_with_a_200_ms_fade() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    h.advance(9.9);
    assert_eq!(balloon_count(&h), 1);
    h.advance(0.2);
    assert_eq!(balloon_count(&h), 1, "fading");
    h.advance(0.25);
    assert_eq!(balloon_count(&h), 0);
    let kinds: Vec<_> = screen!(h)
        .balloons
        .borrow_mut()
        .take_events()
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(kinds, [NotificationEventKind::Hidden]);
}

#[test]
fn a_sticky_balloon_stays_for_five_minutes() {
    let mut h = harness();
    notify(&mut h, "Update available", true);
    h.advance(120.0);
    h.advance(120.0);
    assert_eq!(balloon_count(&h), 1);
    h.advance(61.0);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 0);
}

#[test]
fn the_timer_pauses_while_the_window_is_not_focused() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    h.advance(5.0);
    h.send(Event::FocusLost);
    h.advance(30.0);
    assert_eq!(balloon_count(&h), 1);
    h.send(Event::FocusGained);
    h.advance(4.0);
    assert_eq!(balloon_count(&h), 1);
    h.advance(1.5);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 0);
}

#[test]
fn hovering_a_balloon_stops_the_timer_and_leaving_restarts_the_full_time() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    let (x, y) = h.find("Done").expect("balloon");
    h.advance(8.0);
    h.hover(x, y);
    h.advance(1.0);
    h.advance(30.0);
    assert_eq!(balloon_count(&h), 1);
    h.hover(2, 15);
    h.advance(1.0);
    h.advance(8.5);
    assert_eq!(balloon_count(&h), 1, "the full 10 s start again");
    h.advance(1.6);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 0);
}

#[test]
fn only_two_balloons_show_and_the_rest_wait() {
    let mut h = harness();
    for title in ["First", "Second", "Third"] {
        notify(&mut h, title, false);
    }
    assert_eq!(balloon_count(&h), 2);
    assert!(h.text().contains("+1 more"));
    h.advance(10.1);
    h.advance(0.3);
    assert!(h.text().contains("Third"));
}

#[test]
fn the_close_icon_dismisses_a_balloon() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    let (x, y) = h.find("Done").expect("balloon");
    // The close icon is at the right edge of the title row.
    h.click(x + 46 - 8, y);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 0);
    let kinds: Vec<_> = screen!(h)
        .balloons
        .borrow_mut()
        .take_events()
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(kinds, [NotificationEventKind::Dismissed]);
}

#[test]
fn an_action_link_runs_and_expires_the_notification_unless_it_keeps_it() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    let (x, y) = h.find("Keep").expect("action");
    h.click(x, y);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 1, "a keeping action leaves it open");
    h.advance(1.0);
    let (x, y) = h.find("Open").expect("action");
    h.click(x, y);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 0);
    let kinds: Vec<_> = screen!(h)
        .balloons
        .borrow_mut()
        .take_events()
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert!(
        kinds.contains(&NotificationEventKind::Action(1))
            && kinds.contains(&NotificationEventKind::Action(0))
    );
}

#[test]
fn a_press_outside_hides_regular_balloons_but_not_sticky_ones() {
    let mut h = harness();
    notify(&mut h, "Regular", false);
    notify(&mut h, "Sticky", true);
    h.click(3, 15);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 1);
    assert!(h.text().contains("Sticky"));
}

#[test]
fn escape_closes_a_balloon_that_has_focus_but_otherwise_goes_to_the_app() {
    let mut h = harness();
    notify(&mut h, "Sticky", true);
    h.press(KeyCode::Esc);
    assert_eq!(screen!(h).app_keys, 1);
    let (x, y) = h.find("Something").expect("body");
    h.click(x, y);
    h.press(KeyCode::Esc);
    h.advance(0.3);
    assert_eq!(balloon_count(&h), 0);
}

#[test]
fn a_click_on_a_balloon_is_not_given_to_the_app() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    let (x, y) = h.find("Something").expect("body");
    h.click(x, y);
    assert_eq!(screen!(h).app_clicks, 0);
}

#[test]
fn balloons_are_nvim_notify_floats_with_a_colored_border() {
    let mut h = harness();
    notify(&mut h, "Done", false);
    let (x, y) = h.find("Done").expect("balloon");
    let corner = &h.buffer()[(x - 4, y - 1)];
    assert_eq!(corner.symbol(), "╭");
    assert_eq!(corner.fg, h.rt.look.palette.blue);
}

#[test]
fn the_keep_flag_of_a_notification_action_is_readable() {
    // Balloons read the flag from the Debug output: this pins the format.
    assert!(
        format!("{:?}", NotificationAction::new("x").keep_notification(true))
            .contains("keep: true")
    );
    assert!(format!("{:?}", NotificationAction::new("x")).contains("keep: false"));
}

// ----- status bar -----

#[test]
fn clicking_a_status_slot_reports_it_and_hovering_shows_the_tooltip() {
    let mut h = harness();
    let (x, y) = h.find("Ln 1, Col 1").expect("slot");
    h.hover(x, y);
    h.advance(0.6);
    assert_eq!(h.rt.tooltip.showing_title(), Some("Go to Line"));
    h.click(x, y);
    assert!(matches!(
        screen!(h).status_events.last(),
        Some(StatusEvent::Clicked(StatusSlot::Cursor, _))
    ));
}

#[test]
fn a_right_click_on_the_status_bar_asks_for_the_widget_menu() {
    let mut h = harness();
    h.right_click(40, 29);
    assert!(matches!(
        screen!(h).status_events.last(),
        Some(StatusEvent::ContextMenu(None, 40, 29))
    ));
}

#[test]
fn hovering_a_status_slot_lightens_it() {
    let mut h = harness();
    let (x, y) = h.find("Ln 1, Col 1").expect("slot");
    let before = h.buffer()[(x - 3, y)].bg;
    h.hover(x, y);
    assert_ne!(h.buffer()[(x - 3, y)].bg, before);
}

// ----- which-key -----

#[test]
fn a_pending_chord_shows_the_keys_that_complete_it_and_forgets_after_two_seconds() {
    let mut h = harness();
    h.ctrl('w');
    let text = h.text();
    assert!(text.contains("Ctrl+W"), "{text}");
    assert!(text.contains("ActivateToolWindow1"), "{text}");
    h.advance(2.1);
    assert!(!h.text().contains("ActivateToolWindow1"));
}

#[test]
fn overlays_survive_tiny_screens() {
    for (w, hgt) in [(1u16, 1u16), (6, 3), (30, 8)] {
        let mut h = Harness::new(app_fn(screen(), event as EventFn, draw as DrawFn), w, hgt);
        push_popup(&mut h);
        push_dialog(&mut h, ok_cancel());
        notify(&mut h, "Done", false);
        h.ctrl('w');
        h.hover(0, 0);
        h.advance(1.0);
        h.click(w - 1, hgt - 1);
        for code in [
            KeyCode::Tab,
            KeyCode::Left,
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Esc,
        ] {
            h.press(code);
        }
    }
}

#[test]
fn overlays_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let mut h = Harness::with_runtime(
            app_fn(screen(), event as EventFn, draw as DrawFn),
            Runtime::new(look),
            100,
            26,
        );
        notify(&mut h, "Indexing finished", false);
        let n = Notification::new(Status::Error, "Build failed")
            .group(NotificationGroup::sticky_balloon_group("b"))
            .action("Show log");
        screen!(h)
            .balloons
            .borrow_mut()
            .notify(n, Some("3 errors in archipel-kit-tui."));
        let dialog = Dialog::new("Rename")
            .primary("Rename")
            .secondary("Cancel")
            .message("Enter a new name for main.rs")
            .content(NameContent {
                field: TextFieldState::with_text("lib.rs"),
            })
            .on_exit(|c| vec![Msg::Exit(c)]);
        push_dialog(&mut h, dialog);
        h.advance(0.5);
        snapshot(&format!("overlays.{name}"), h.buffer());
    }
    let _ = (
        KeyModifiers::NONE,
        MouseButton::Left,
        MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        },
    );
}
