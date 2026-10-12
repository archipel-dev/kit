//! `Banner`, `Toast`, `Spinner`, `ProgressBar` and `EmptyState` tests.

mod common;

use archipel_kit::components::{Banner, EmptyState, ProgressBar, Severity, Spinner, Toast};
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme};

#[test]
fn banner_snapshot() {
    testing::snapshot_themes("banner", common::IMG, |ui| {
        Banner::new(Severity::Info, "Indexing finished: 1,204 files")
            .dismissible(true)
            .show(ui);
        Banner::new(Severity::Success, "Gradle sync succeeded").show(ui);
        Banner::new(Severity::Warning, "Project JDK is not defined")
            .action("Set up JDK")
            .show(ui);
        Banner::new(Severity::Error, "Build failed with 3 errors")
            .action("Show")
            .action("Retry")
            .show(ui);
    });
}

#[test]
fn banner_reports_actions_and_dismissal() {
    let mut harness =
        testing::harness_state(Theme::dark(), (None, false), |ui, (action, dismissed)| {
            let out = Banner::new(Severity::Error, "Build failed")
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
fn long_banner_text_wraps() {
    let harness = testing::harness(Theme::dark(), |ui| {
        Banner::new(
            Severity::Warning,
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
fn toast_snapshot() {
    testing::snapshot_themes("toast", common::IMG, |ui| {
        Toast::new(Severity::Success, "Build finished")
            .body("3 modules compiled in 4.2 s")
            .action("Open log")
            .action("Run again")
            .show(ui);
        Toast::new(Severity::Error, "Connection refused")
            .body("mongodb://localhost:27017")
            .show(ui);
    });
}

#[test]
fn toast_reports_actions() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, action| {
        let out = Toast::new(Severity::Info, "Update available")
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
            ui.add(Spinner::new());
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
        ui.add(Spinner::new().label("Indexing"));
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
fn empty_state_snapshot() {
    testing::snapshot_themes("empty_state", common::IMG, |ui| {
        EmptyState::new("No requests yet")
            .icon(Glyph::Send)
            .hint("Create a request to start testing your API.")
            .action("New request")
            .show(ui);
    });
}

#[test]
fn empty_state_action_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, clicked| {
        *clicked |= EmptyState::new("Nothing here")
            .action("Create")
            .show(ui)
            .clicked();
    });
    harness.get_by_label("Create").click();
    harness.run();
    assert!(*harness.state());
}

#[test]
fn messages_and_empty_states_pass_the_audit() {
    for theme in [Theme::dark(), Theme::light()] {
        let mut harness = testing::harness(theme, |ui| {
            let _ = Banner::new(Severity::Warning, "Project JDK is not defined")
                .action("Configure")
                .dismissible(true)
                .show(ui);
            let _ = Toast::new(Severity::Success, "Build finished")
                .body("3 modules compiled")
                .action("Open log")
                .show(ui);
            let _ = EmptyState::new("No requests yet")
                .hint("Send one to see it here")
                .action("New request")
                .show(ui);
        });
        testing::audit(&mut harness).assert_clean();
    }
}
