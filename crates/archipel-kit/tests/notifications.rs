//! `NotificationsManager` behavior from IntelliJ spec section 10.1 and the notification
//! model, the editor notification panel and the inline banner of section 10.2.

mod common;

use archipel_kit::Theme;
use archipel_kit::notification::{
    Notification, NotificationAction, NotificationDisplayType, NotificationEvent,
    NotificationEventKind, NotificationGroup, NotificationType, NotificationsManager,
};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::ui::{EditorNotificationPanel, InlineBanner, Status};
use egui::Key;

/// The manager and everything it reported.
#[derive(Default)]
struct World {
    manager: NotificationsManager,
    events: Vec<NotificationEvent>,
}

fn world(setup: impl FnOnce(&mut NotificationsManager)) -> testing::Harness<'static, World> {
    let mut state = World::default();
    setup(&mut state.manager);
    let mut harness = testing::harness_state(Theme::dark(), state, |ui, world| {
        ui.label("Editor behind the balloons");
        let events = world.manager.show(ui.ctx());
        world.events.extend(events);
    });
    harness.set_size(egui::vec2(520.0, 400.0));
    harness.run_steps(3);
    harness
}

fn info(title: &str) -> Notification {
    Notification::new(Status::Info, title)
}

fn sticky(title: &str) -> Notification {
    info(title).group(NotificationGroup::sticky_balloon_group("Sticky"))
}

/// A primary click at `pos`, one frame per event.
fn click_at<S>(harness: &mut testing::Harness<'_, S>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.drop_at(pos);
    harness.step();
}

fn kinds(harness: &testing::Harness<'_, World>) -> Vec<NotificationEventKind> {
    harness.state().events.iter().map(|e| e.kind).collect()
}

#[test]
fn a_balloon_is_360_wide() {
    // The stack sits 16 pt from the window's right edge; the title starts after the card's
    // 12 pt padding, the 16 pt icon and the 8 pt gap.
    let harness = world(|m| {
        m.notify(info("Build finished"));
    });
    let title = harness.get_by_label("Build finished").rect();
    let card_left = 520.0 - 16.0 - 360.0;
    assert!(
        (title.left() - (card_left + 12.0 + 16.0 + 8.0)).abs() <= 2.0,
        "{title:?}"
    );
}

#[test]
fn notification_types_map_to_statuses() {
    assert_eq!(Status::from(NotificationType::Information), Status::Info);
    assert_eq!(Status::from(NotificationType::Warning), Status::Warning);
    assert_eq!(Status::from(NotificationType::Error), Status::Error);
    assert_eq!(Status::from(NotificationType::IdeUpdate), Status::Promo);
}

#[test]
fn groups_know_their_display_id_and_type() {
    let group = NotificationGroup::tool_window_group("Build", "Run");
    assert_eq!(group.display_id(), "Build");
    assert_eq!(group.display_type(), NotificationDisplayType::ToolWindow);
    assert_eq!(group.tool_window_id(), Some("Run"));
    assert_eq!(
        NotificationGroup::log_only_group("Log").display_type(),
        NotificationDisplayType::None
    );
}

#[test]
fn at_most_two_balloons_are_visible_and_the_rest_wait() {
    let harness = world(|m| {
        for title in ["First", "Second", "Third"] {
            m.notify(info(title));
        }
    });
    assert!(harness.query_by_label("First").is_some());
    assert!(harness.query_by_label("Second").is_some());
    assert!(harness.query_by_label("Third").is_none(), "queued");
    assert_eq!(harness.state().manager.visible().len(), 2);
    assert_eq!(harness.state().manager.queued().len(), 1);
}

#[test]
fn the_newest_balloon_is_at_the_bottom() {
    let harness = world(|m| {
        m.notify(info("First"));
        m.notify(info("Second"));
    });
    let first = harness.get_by_label("First").rect();
    let second = harness.get_by_label("Second").rect();
    assert!(second.top() > first.bottom(), "{first:?} {second:?}");
    assert!(
        first.left() == second.left(),
        "both are in the same column at the right edge"
    );
}

#[test]
fn a_balloon_hides_after_10_seconds() {
    let mut harness = world(|m| {
        m.notify(info("Build finished"));
    });
    common::advance_by(&mut harness, 9.5);
    assert!(harness.query_by_label("Build finished").is_some());
    assert!(kinds(&harness).is_empty());
    common::advance_by(&mut harness, 1.0);
    assert!(harness.query_by_label("Build finished").is_none());
    assert_eq!(kinds(&harness), [NotificationEventKind::Hidden]);
    assert_eq!(harness.state().manager.history().len(), 1);
}

#[test]
fn a_balloon_fades_out_for_200_ms_before_it_is_gone() {
    let mut harness = world(|m| {
        m.notify(info("Build finished"));
    });
    common::advance_by(&mut harness, 10.1);
    assert!(
        kinds(&harness).is_empty(),
        "the fade-out is under way, the balloon is still there"
    );
    common::advance_by(&mut harness, 0.3);
    assert_eq!(kinds(&harness), [NotificationEventKind::Hidden]);
}

#[test]
fn a_sticky_balloon_stays_300_seconds() {
    let mut harness = world(|m| {
        m.notify(sticky("Needs you"));
    });
    // Big steps keep the test short: the timer counts the time that passed.
    for _ in 0..28 {
        common::advance(&mut harness, 10.0);
    }
    assert!(
        harness.query_by_label("Needs you").is_some(),
        "still there after 280 s"
    );
    for _ in 0..3 {
        common::advance(&mut harness, 10.0);
    }
    common::advance(&mut harness, 0.3);
    assert!(harness.query_by_label("Needs you").is_none());
    assert_eq!(kinds(&harness), [NotificationEventKind::Hidden]);
}

#[test]
fn the_timer_pauses_while_the_pointer_is_over_the_balloon() {
    let mut harness = world(|m| {
        m.notify(info("Build finished"));
    });
    harness.get_by_label("Build finished").hover();
    harness.run_steps(3);
    common::advance_by(&mut harness, 30.0);
    assert!(
        harness.query_by_label("Build finished").is_some(),
        "30 s of hovering do not hide it"
    );
    // The pointer leaves: the full 10 s start over.
    harness.hover_at(egui::pos2(5.0, 5.0));
    harness.run_steps(2);
    common::advance_by(&mut harness, 9.0);
    assert!(harness.query_by_label("Build finished").is_some());
    common::advance_by(&mut harness, 1.5);
    assert!(harness.query_by_label("Build finished").is_none());
}

#[test]
fn the_timer_pauses_while_the_window_is_not_active() {
    let mut harness = world(|m| {
        m.notify(info("Build finished"));
    });
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .focused = Some(false);
    harness.step();
    common::advance_by(&mut harness, 60.0);
    assert!(harness.query_by_label("Build finished").is_some());
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .focused = Some(true);
    harness.step();
    common::advance_by(&mut harness, 10.5);
    assert!(harness.query_by_label("Build finished").is_none());
}

#[test]
fn a_queued_notification_starts_its_timer_when_it_gets_a_place() {
    let mut harness = world(|m| {
        m.notify(info("First"));
        m.notify(info("Second"));
        m.notify(info("Third"));
    });
    common::advance_by(&mut harness, 8.0);
    assert!(harness.query_by_label("Third").is_none());
    common::advance_by(&mut harness, 2.6);
    assert!(
        harness.query_by_label("Third").is_some(),
        "it has a place now"
    );
    common::advance_by(&mut harness, 8.0);
    assert!(
        harness.query_by_label("Third").is_some(),
        "its 10 s began when it showed"
    );
    common::advance_by(&mut harness, 2.5);
    assert!(harness.query_by_label("Third").is_none());
}

#[test]
fn the_close_button_dismisses_the_notification() {
    let mut harness = world(|m| {
        m.notify(info("Build finished"));
    });
    harness.get_by_label("Build finished").hover();
    harness.run_steps(2);
    harness.get_by_label("Dismiss").click();
    harness.run_steps(3);
    assert_eq!(kinds(&harness), [NotificationEventKind::Dismissed]);
    assert!(harness.query_by_label("Build finished").is_none());
}

#[test]
fn escape_closes_the_balloon_that_has_the_focus() {
    let mut harness = world(|m| {
        m.notify(info("Build finished").action("Open log"));
    });
    harness.get_by_label("Open log").focus();
    harness.run_steps(2);
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert_eq!(kinds(&harness), [NotificationEventKind::Dismissed]);
}

#[test]
fn escape_without_focus_in_a_balloon_leaves_it_alone() {
    let mut harness = world(|m| {
        m.notify(info("Build finished").action("Open log"));
    });
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(kinds(&harness).is_empty());
}

#[test]
fn performing_an_action_expires_the_notification() {
    let mut harness = world(|m| {
        m.notify(info("Update available").action("Restart"));
    });
    harness.get_by_label("Restart").click();
    harness.run_steps(3);
    assert_eq!(kinds(&harness), [NotificationEventKind::Action(0)]);
    assert!(harness.query_by_label("Update available").is_none());
}

#[test]
fn an_action_that_keeps_the_notification_leaves_it_open() {
    let mut harness = world(|m| {
        m.notify(
            info("Update available")
                .notification_action(NotificationAction::new("Remind me").keep_notification(true)),
        );
    });
    harness.get_by_label("Remind me").click();
    harness.run_steps(3);
    assert_eq!(kinds(&harness), [NotificationEventKind::Action(0)]);
    assert!(harness.query_by_label("Update available").is_some());
}

#[test]
fn a_click_outside_hides_a_regular_balloon_but_not_a_sticky_one() {
    let mut harness = world(|m| {
        m.notify(info("Regular"));
        m.notify(sticky("Sticky"));
    });
    harness.hover_at(egui::pos2(10.0, 10.0));
    harness.run_steps(2);
    click_at(&mut harness, egui::pos2(10.0, 10.0));
    common::advance_by(&mut harness, 0.5);
    assert!(harness.query_by_label("Regular").is_none());
    assert!(harness.query_by_label("Sticky").is_some());
    assert_eq!(kinds(&harness), [NotificationEventKind::Hidden]);
}

#[test]
fn a_click_inside_a_balloon_hides_nothing() {
    let mut harness = world(|m| {
        m.notify(info("Regular"));
    });
    harness.get_by_label("Regular").click();
    common::advance_by(&mut harness, 0.5);
    assert!(harness.query_by_label("Regular").is_some());
}

#[test]
fn hide_on_click_outside_can_be_turned_off() {
    let mut harness = world(|m| {
        *m = std::mem::take(m).hide_on_click_outside(false);
        m.notify(info("Regular"));
    });
    harness.hover_at(egui::pos2(10.0, 10.0));
    harness.run_steps(2);
    click_at(&mut harness, egui::pos2(10.0, 10.0));
    common::advance_by(&mut harness, 0.5);
    assert!(harness.query_by_label("Regular").is_some());
}

#[test]
fn a_log_only_group_is_never_shown() {
    let harness = world(|m| {
        m.notify(info("Quiet").group(NotificationGroup::log_only_group("Log")));
    });
    assert!(harness.query_by_label("Quiet").is_none());
    assert_eq!(harness.state().manager.history().len(), 1);
    assert!(harness.state().manager.visible().is_empty());
}

#[test]
fn expire_removes_a_notification_without_a_trace() {
    let mut harness = world(|m| {
        m.notify(info("Gone soon"));
    });
    let id = harness.state().manager.visible()[0];
    assert!(harness.state_mut().manager.expire(id));
    harness.run_steps(2);
    assert!(harness.query_by_label("Gone soon").is_none());
    assert!(harness.state().manager.history().is_empty());
    assert!(!harness.state_mut().manager.expire(id));
}

#[test]
fn a_long_balloon_scrolls_instead_of_growing_past_75_percent() {
    let long = "A very long line of explanation. ".repeat(200);
    let harness = world(|m| {
        m.notify(info("Long").body(long));
        m.notify(info("Another"));
    });
    let title = harness.get_by_label("Long").rect();
    let another = harness.get_by_label("Another").rect();
    assert!(title.top() >= 0.0, "{title:?}");
    assert!(another.bottom() <= 400.0, "{another:?}");
}

#[test]
fn balloons_pass_the_audit() {
    for theme in [Theme::dark(), Theme::light()] {
        let mut world = World::default();
        world.manager.notify(
            Notification::new(Status::Success, "Build finished")
                .body("3 modules compiled")
                .action("Open log"),
        );
        let mut harness = testing::harness_state(theme, world, |ui, world| {
            let events = world.manager.show(ui.ctx());
            world.events.extend(events);
        });
        harness.set_size(egui::vec2(520.0, 400.0));
        harness.run_steps(3);
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn balloons_snapshot() {
    testing::snapshot_themes_sized(
        "notification_balloons",
        common::IMG,
        egui::vec2(460.0, 300.0),
        |ui| {
            ui.label("Editor content behind the balloons");
            let mut manager = NotificationsManager::new();
            manager.notify(
                Notification::new(NotificationType::Warning, "Indexing paused")
                    .body("Indexing was paused while the project builds.")
                    .action("Resume"),
            );
            manager.notify(
                Notification::new(NotificationType::IdeUpdate, "Update available")
                    .body("Version 2026.2.4 is ready to install.")
                    .action("Update")
                    .action("Later"),
            );
            let _ = manager.show(ui.ctx());
        },
    );
}

#[test]
fn promo_status_uses_the_info_tint_and_the_sparkle_icon() {
    testing::snapshot_themes("inline_banner_promo", common::IMG, |ui| {
        InlineBanner::new(Status::Promo, "Try the new debugger tool window")
            .action("Show me")
            .gear_action("Banner settings")
            .dismissible(true)
            .show(ui);
        InlineBanner::new(Status::Info, "Indexing finished").show(ui);
    });
}

#[test]
fn editor_notification_panel_snapshot() {
    testing::snapshot_themes("editor_notification_panel", common::IMG, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        EditorNotificationPanel::new("JDK is not defined")
            .status(Status::Warning)
            .action("Setup JDK")
            .dismissible(true)
            .show(ui);
        EditorNotificationPanel::new("Changed on disk")
            .action("Reload")
            .action("Compare")
            .show(ui);
        EditorNotificationPanel::new("Try the debugger")
            .status(Status::Promo)
            .action("Show me")
            .dismissible(true)
            .show(ui);
    });
}

#[test]
fn editor_notification_panel_reports_actions_and_dismissal() {
    let mut harness =
        testing::harness_state(Theme::dark(), (None, false), |ui, (action, dismissed)| {
            let out = EditorNotificationPanel::new("File is read-only")
                .status(Status::Error)
                .action("Make writable")
                .action("Open anyway")
                .dismissible(true)
                .show(ui);
            if out.action.is_some() {
                *action = out.action;
            }
            *dismissed |= out.dismissed;
        });
    harness.get_by_label("Open anyway").click();
    harness.run();
    assert_eq!(harness.state().0, Some(1));
    harness.get_by_label("Dismiss").click();
    harness.run();
    assert!(harness.state().1);
}

#[test]
fn editor_notification_panel_spans_the_width_and_has_the_measured_insets() {
    let harness = testing::harness(Theme::dark(), |ui| {
        EditorNotificationPanel::new("File is read-only")
            .status(Status::Error)
            .action("Make writable")
            .show(ui);
    });
    let text = harness.get_by_label("File is read-only").rect();
    let icon_gap = text.left();
    assert!(
        icon_gap >= 12.0 + 16.0 + 8.0 - 1.0,
        "12 inset, 16 icon, 8 gap: {icon_gap}"
    );
    let link = harness.get_by_label("Make writable").rect();
    assert!(text.right() <= link.left() - 19.0, "20 pt before the links");
}

#[test]
fn the_gear_action_is_reported() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, gear| {
        *gear |= InlineBanner::new(Status::Info, "Tip")
            .gear_action("Settings")
            .show(ui)
            .gear;
    });
    harness.get_by_label("Settings").click();
    harness.run();
    assert!(*harness.state());
}
