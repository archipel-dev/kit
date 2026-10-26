//! `ActionLink` (IntelliJ spec §13.6).

mod common;

use archipel_kit::testing::{self, Harness, NodeExt, Queryable, Role};
use archipel_kit::ui::ActionLink;
use archipel_kit::{Theme, egui};
use egui::{Event, Key, Modifiers, PointerButton};

fn harness(link: impl Fn() -> ActionLink + 'static) -> Harness<'static, u32> {
    let mut harness = testing::harness_state(Theme::dark(), 0, move |ui, clicks| {
        if ui.add(link()).clicked() {
            *clicks += 1;
        }
    });
    harness.run();
    harness
}

fn underlined(harness: &Harness<'_, u32>, label: &str) -> bool {
    let rect = harness.get_by_label(label).rect();
    harness.output().shapes.iter().any(|c| match &c.shape {
        egui::Shape::LineSegment { points, .. } => {
            points[0].y == points[1].y
                && (points[0].y - rect.bottom()).abs() <= 3.0
                && points[0].x >= rect.left() - 1.0
        }
        _ => false,
    })
}

#[test]
fn it_is_a_link_named_by_its_text() {
    let harness = harness(|| ActionLink::new("Open docs"));
    assert_eq!(harness.get_by_label("Open docs").role(), Role::Link);
}

#[test]
fn it_fires_on_release_not_on_press() {
    let mut harness = harness(|| ActionLink::new("Open docs"));
    let p = harness.get_by_label("Open docs").rect().center();
    harness.event(Event::PointerMoved(p));
    harness.event(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(*harness.state(), 0);
    harness.event(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(*harness.state(), 1);
}

#[test]
fn it_is_underlined_on_hover_only() {
    let mut harness = harness(|| ActionLink::new("Open docs"));
    assert!(!underlined(&harness, "Open docs"));
    harness.hover_at(harness.get_by_label("Open docs").rect().center());
    harness.run();
    assert!(underlined(&harness, "Open docs"));
}

#[test]
fn it_shows_the_hand_cursor_on_hover() {
    let mut harness = harness(|| ActionLink::new("Open docs"));
    harness.hover_at(harness.get_by_label("Open docs").rect().center());
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::PointingHand
    );
}

#[test]
fn enter_and_space_fire_it_when_focused() {
    let mut harness = harness(|| ActionLink::new("Open docs"));
    harness.get_by_label("Open docs").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(*harness.state(), 2);
}

#[test]
fn a_focused_link_shows_the_focus_ring() {
    let mut harness = harness(|| ActionLink::new("Open docs"));
    let ring = Theme::dark().colors.focus_ring;
    let drawn = |harness: &Harness<'_, u32>| {
        harness.output().shapes.iter().any(|c| match &c.shape {
            egui::Shape::Rect(r) => r.stroke.color == ring,
            _ => false,
        })
    };
    assert!(!drawn(&harness));
    harness.get_by_label("Open docs").focus();
    harness.run();
    assert!(drawn(&harness));
}

#[test]
fn a_disabled_link_is_inert_dimmed_and_has_no_hand_or_underline() {
    let mut harness = harness(|| ActionLink::new("Open docs").enabled(false));
    assert!(harness.get_by_label("Open docs").is_disabled());
    harness.hover_at(harness.get_by_label("Open docs").rect().center());
    harness.run();
    harness.get_by_label("Open docs").click();
    harness.run();
    assert_eq!(*harness.state(), 0);
    assert!(!underlined(&harness, "Open docs"));
    assert_ne!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::PointingHand
    );
}

#[test]
fn the_external_link_and_drop_down_icons_widen_the_link() {
    let plain = harness(|| ActionLink::new("Docs"))
        .get_by_label("Docs")
        .rect()
        .width();
    let external = harness(|| ActionLink::new("Docs").external(true))
        .get_by_label("Docs")
        .rect()
        .width();
    let dropdown = harness(|| ActionLink::new("Docs").dropdown(true))
        .get_by_label("Docs")
        .rect()
        .width();
    assert_eq!(external, plain + 16.0);
    assert_eq!(dropdown, plain + 16.0);
}

#[test]
fn action_link_variants_snapshot() {
    testing::snapshot_themes("action_link_variants", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(ActionLink::new("Configure…"));
            ui.add(ActionLink::new("Documentation").external(true));
            ui.add(ActionLink::new("Sort by").dropdown(true));
            ui.add(ActionLink::new("Disabled").enabled(false));
        });
    });
}
