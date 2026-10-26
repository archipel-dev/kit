//! `StatusBarWidget` and `Surface::Chrome` tests.

mod common;

use archipel_kit::parts::Surface;
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme, Tone};
use archipel_kit::{action_system::ActionButton, ide::HelpTooltip, wm::StatusBarWidget};

#[test]
fn status_bar_snapshot() {
    testing::snapshot_themes("status_bar_widget", common::IMG, |ui| {
        ui.set_width(420.0);
        Surface::Chrome.frame(ui.ctx()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(ActionButton::new(Glyph::Run, "Run").tone(Tone::Run));
                ui.add(
                    StatusBarWidget::new("app")
                        .icon(Glyph::Folder)
                        .chevron(true),
                );
            });
        });
        Surface::Chrome.frame(ui.ctx()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    StatusBarWidget::new("main")
                        .icon(Glyph::Branch)
                        .chevron(true),
                );
                ui.add(StatusBarWidget::new("UTF-8"));
                ui.add(StatusBarWidget::new("2 errors").tone(Tone::Error));
                ui.add(StatusBarWidget::new("Offline").enabled(false));
                ui.add(
                    StatusBarWidget::new("A very long branch name that cannot fit here")
                        .max_width(90.0),
                );
            });
        });
    });
}

#[test]
fn item_is_a_named_button_that_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui
            .add(
                StatusBarWidget::new("main")
                    .icon(Glyph::Branch)
                    .chevron(true),
            )
            .clicked()
        {
            *clicks += 1;
        }
    });
    let node = harness.get_by_label("main");
    assert_eq!(node.role(), Role::Button);
    node.click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn disabled_item_ignores_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui
            .add(StatusBarWidget::new("Offline").enabled(false))
            .clicked()
        {
            *clicks += 1;
        }
    });
    assert!(harness.get_by_label("Offline").is_disabled());
    harness.get_by_label("Offline").click();
    harness.run();
    assert_eq!(*harness.state(), 0);
}

#[test]
fn long_text_is_elided_and_shown_in_full_as_the_tooltip() {
    let long = "A very long branch name that cannot fit here";
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(StatusBarWidget::new(long).max_width(90.0));
    });
    harness.run();
    let width = harness.get_by_label(long).rect().width();
    assert!(width <= 91.0, "width {width}");
    harness.get_by_label(long).hover();
    harness.run_steps(40);
    assert_eq!(
        harness.query_all_by_label(long).count(),
        2,
        "name + tooltip"
    );
}

#[test]
fn caller_tooltip_replaces_the_elision_tooltip() {
    let long = "A very long branch name that cannot fit here";
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(
            StatusBarWidget::new(long)
                .max_width(90.0)
                .with(HelpTooltip::new("Git branch")),
        );
    });
    harness.run();
    harness.get_by_label(long).hover();
    harness.run_steps(40);
    assert!(harness.query_by_label("Git branch").is_some());
    assert_eq!(
        harness.query_all_by_label(long).count(),
        1,
        "no second tooltip"
    );
}

#[test]
fn caller_tooltip_is_accepted() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(StatusBarWidget::new("main").with(HelpTooltip::new("Git branch: main")));
    });
    harness.get_by_label("main").hover();
    harness.run_steps(40);
    assert!(harness.query_by_label("Git branch: main").is_some());
}

#[test]
fn item_conforms_and_passes_the_audit() {
    testing::conformance("main", |ui, enabled| {
        ui.add(
            StatusBarWidget::new("main")
                .icon(Glyph::Branch)
                .chevron(true)
                .enabled(enabled),
        )
    })
    .run();
    let mut harness = testing::harness(Theme::light(), |ui| {
        Surface::Chrome.frame(ui.ctx()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    StatusBarWidget::new("main")
                        .icon(Glyph::Branch)
                        .chevron(true),
                );
                ui.add(StatusBarWidget::new("UTF-8"));
            });
        });
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
}
