//! `InlineBanner`, `Notification`, `AsyncProcessIcon`, `ProgressBar` and `StatusText` tests.

mod common;

use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme};
use archipel_kit::{
    notification::Notification,
    ui::{AsyncProcessIcon, InlineBanner, ProgressBar, Status, StatusText},
};

#[test]
fn inline_banner_snapshot() {
    testing::snapshot_themes("inline_banner", common::IMG, |ui| {
        InlineBanner::new(Status::Info, "Indexing finished: 1,204 files")
            .dismissible(true)
            .show(ui);
        InlineBanner::new(Status::Success, "Gradle sync succeeded").show(ui);
        InlineBanner::new(Status::Warning, "Project JDK is not defined")
            .action("Set up JDK")
            .show(ui);
        InlineBanner::new(Status::Error, "Build failed with 3 errors")
            .action("Show")
            .action("Retry")
            .show(ui);
    });
}

#[test]
fn inline_banner_reports_actions_and_dismissal() {
    let mut harness =
        testing::harness_state(Theme::dark(), (None, false), |ui, (action, dismissed)| {
            let out = InlineBanner::new(Status::Error, "Build failed")
                .action("Show")
                .action("Retry")
                .dismissible(true)
                .show(ui);
            if out.action.is_some() {
                *action = out.action;
            }
            *dismissed |= out.dismissed;
        });
    harness.get_by_label("Retry").click();
    harness.run();
    assert_eq!(harness.state().0, Some(1));
    harness.get_by_label("Dismiss").click();
    harness.run();
    assert!(harness.state().1);
}

#[test]
fn long_inline_banner_text_wraps() {
    let harness = testing::harness(Theme::dark(), |ui| {
        InlineBanner::new(
            Status::Warning,
            "The Gradle wrapper version is outdated and does not support the selected JDK; update it to continue.",
        )
        .action("Update")
        .show(ui);
    });
    let text = harness.get_by_label_contains("Gradle wrapper").rect();
    assert!(text.height() > 20.0, "wrapped onto several lines: {text:?}");
    assert!(text.right() < harness.get_by_label("Update").rect().left());
}

#[test]
fn notification_snapshot() {
    testing::snapshot_themes("notification", common::IMG, |ui| {
        Notification::new(Status::Success, "Build finished")
            .body("3 modules compiled in 4.2 s")
            .action("Open log")
            .action("Run again")
            .show(ui);
        Notification::new(Status::Error, "Connection refused")
            .body("mongodb://localhost:27017")
            .show(ui);
    });
}

#[test]
fn notification_reports_actions() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, action| {
        let out = Notification::new(Status::Info, "Update available")
            .action("Restart")
            .show(ui);
        if out.action.is_some() {
            *action = out.action;
        }
    });
    let link = harness.get_by_label("Restart");
    assert_eq!(link.role(), Role::Link);
    link.click();
    harness.run();
    assert_eq!(*harness.state(), Some(0));
}

#[test]
fn progress_snapshot() {
    testing::snapshot_themes("progress", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(AsyncProcessIcon::new());
            ui.label("Indexing…");
        });
        ui.add(ProgressBar::new(Some(0.0)));
        ui.add(ProgressBar::new(Some(0.4)));
        ui.add(ProgressBar::new(Some(1.0)));
    });
}

#[test]
fn progress_reports_percent() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(ProgressBar::new(Some(0.42)).label("Downloading JDK"));
        ui.add(ProgressBar::new(None).label("Resolving"));
        ui.add(AsyncProcessIcon::new().label("Indexing"));
    });
    assert_eq!(
        harness.get_by_label("Downloading JDK").numeric_value(),
        Some(42.0)
    );
    assert_eq!(harness.get_by_label("Resolving").numeric_value(), None);
    assert_eq!(
        harness.get_by_label("Indexing").role(),
        Role::ProgressIndicator
    );
}

#[test]
fn status_text_snapshot() {
    testing::snapshot_themes("status_text", common::IMG, |ui| {
        StatusText::new("No requests yet")
            .icon(Glyph::Send)
            .hint("Create a request to start testing your API.")
            .action("New request")
            .show(ui);
    });
}

#[test]
fn status_text_action_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, clicked| {
        *clicked |= StatusText::new("Nothing here")
            .action("Create")
            .show(ui)
            .clicked();
    });
    harness.get_by_label("Create").click();
    harness.run();
    assert!(*harness.state());
}

#[test]
fn messages_and_status_texts_pass_the_audit() {
    for theme in [Theme::dark(), Theme::light()] {
        let mut harness = testing::harness(theme, |ui| {
            let _ = InlineBanner::new(Status::Warning, "Project JDK is not defined")
                .action("Configure")
                .dismissible(true)
                .show(ui);
            let _ = Notification::new(Status::Success, "Build finished")
                .body("3 modules compiled")
                .action("Open log")
                .show(ui);
            let _ = StatusText::new("No requests yet")
                .hint("Send one to see it here")
                .action("New request")
                .show(ui);
        });
        testing::audit(&mut harness).assert_clean();
    }
}
