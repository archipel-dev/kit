//! Splitter family (IntelliJ spec section 12): divider widths and hit zone, the one pixel
//! splitter, Ctrl-click to flip the orientation, lack-of-space strategies, proportion limits
//! and persistence by key.

use archipel_kit::cursor;
use archipel_kit::egui::{self, CursorIcon, Event, Modifiers, PointerButton, pos2, vec2};
use archipel_kit::host::{InMemoryProperties, PropertiesComponent};
use archipel_kit::openapi::ui::{
    DividerStyle, JBSplitter, LackOfSpaceStrategy, OnePixelSplitter, Splitter,
};
use archipel_kit::testing::{self, Harness, Queryable};
use archipel_kit::{Theme, tokens::size};

const WIDTH: f32 = 400.0;

struct State {
    ratio: f32,
    flips: usize,
    cursor: CursorIcon,
    properties: InMemoryProperties,
}

fn state(ratio: f32) -> State {
    State {
        ratio,
        flips: 0,
        cursor: CursorIcon::Default,
        properties: InMemoryProperties::new(),
    }
}

fn harness_of(
    state: State,
    width: f32,
    build: impl for<'a> Fn(&'a mut f32, &'a mut InMemoryProperties) -> Splitter<'a> + 'static,
) -> Harness<'static, State> {
    let mut harness = testing::harness_state(Theme::dark(), state, move |ui, state| {
        let State {
            ratio,
            flips,
            cursor: cursor_out,
            properties,
        } = state;
        ui.allocate_ui(vec2(width, 120.0), |ui| {
            let out = build(ratio, properties).label("Resize sidebar").show(
                ui,
                |ui| ui.label("Sidebar"),
                |ui| ui.label("Request"),
            );
            *flips += usize::from(out.flip_orientation);
            *cursor_out = cursor::current(ui.ctx());
        });
    });
    harness.run();
    harness
}

fn divider(harness: &Harness<'_, State>) -> egui::Rect {
    harness.get_by_label("Resize sidebar").rect()
}

fn click_with(harness: &mut Harness<'_, State>, pos: egui::Pos2, mods: Modifiers) {
    harness.hover_at(pos);
    harness.step();
    for pressed in [true, false] {
        harness.event_modifiers(
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: mods,
            },
            mods,
        );
        harness.step();
    }
}

fn drag_to(harness: &mut Harness<'_, State>, x: f32) {
    let from = divider(harness).center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=6 {
        let t = step as f32 / 6.0;
        harness.hover_at(pos2(from.x + (x - from.x) * t, from.y));
        harness.run();
    }
    harness.drop_at(pos2(x, from.y));
    harness.run();
}

#[test]
fn the_classic_divider_is_7_points_wide() {
    let harness = harness_of(state(0.5), WIDTH, |ratio, _| {
        Splitter::horizontal("split", ratio).divider_style(DividerStyle::Classic)
    });
    let rect = divider(&harness);
    assert!(
        (rect.width() - size::SPLIT_CLASSIC_DIVIDER).abs() < 0.5,
        "{rect:?}"
    );
}

#[test]
fn the_one_pixel_divider_has_a_6_point_hit_zone_around_a_1_point_line() {
    let mut harness = harness_of(state(0.5), WIDTH, |ratio, _| {
        OnePixelSplitter::horizontal("split", ratio)
    });
    let rect = divider(&harness);
    assert!(
        (rect.width() - size::SPLIT_MOUSE_ZONE).abs() < 0.5,
        "{rect:?}"
    );
    // The panes share the width minus the 1 pt line.
    let sidebar = harness.get_by_label("Sidebar").rect();
    let request = harness.get_by_label("Request").rect();
    let gap = request.left() - sidebar.left();
    assert!(gap > 0.0);
    // Inside the zone: the resize cursor; outside: not.
    let center = rect.center();
    harness.hover_at(pos2(center.x + 2.5, center.y));
    harness.run();
    assert_eq!(harness.state().cursor, CursorIcon::ResizeHorizontal);
    // (egui hits widgets within its 5 pt interact radius, so test well outside.)
    harness.hover_at(pos2(center.x + 14.0, center.y));
    harness.run();
    assert_ne!(harness.state().cursor, CursorIcon::ResizeHorizontal);
}

#[test]
fn a_one_pixel_divider_gives_the_panes_all_but_one_point() {
    let harness = harness_of(state(0.5), WIDTH, |ratio, _| {
        OnePixelSplitter::horizontal("split", ratio)
    });
    let rect = divider(&harness);
    // The line is centered in the zone, at half of the width without the line.
    let line_x = rect.center().x;
    let sidebar_left = harness.get_by_label("Sidebar").rect().left();
    let first = line_x - 0.5 - sidebar_left;
    assert!(
        (first - (WIDTH - 1.0) / 2.0).abs() < 2.0,
        "first pane {first}"
    );
}

#[test]
fn double_click_resets_a_one_pixel_splitter_to_one_half() {
    let mut harness = harness_of(state(0.8), WIDTH, |ratio, _| {
        OnePixelSplitter::horizontal("split", ratio)
    });
    let center = divider(&harness).center();
    harness.hover_at(center);
    harness.run();
    for _ in 0..2 {
        harness.drag_at(center);
        harness.run();
        harness.drop_at(center);
        harness.run();
    }
    assert!(
        (harness.state().ratio - 0.5).abs() < 1e-3,
        "{}",
        harness.state().ratio
    );
}

#[test]
fn ctrl_click_reports_a_flip_only_when_allowed() {
    let mut allowed = harness_of(state(0.5), WIDTH, |ratio, _| {
        Splitter::horizontal("split", ratio).allow_switch_orientation_by_mouse_click(true)
    });
    let pos = divider(&allowed).center();
    click_with(&mut allowed, pos, Modifiers::NONE);
    assert_eq!(allowed.state().flips, 0, "a plain click flips nothing");
    // Wait out the double click time, so the next click is a click of its own.
    for _ in 0..40 {
        allowed.step();
    }
    click_with(&mut allowed, pos, Modifiers::CTRL);
    assert_eq!(allowed.state().flips, 1);

    let mut locked = harness_of(state(0.5), WIDTH, |ratio, _| {
        Splitter::horizontal("split", ratio)
    });
    let pos = divider(&locked).center();
    click_with(&mut locked, pos, Modifiers::CTRL);
    assert_eq!(locked.state().flips, 0, "off by default");
}

#[test]
fn without_room_for_both_minimums_the_strategy_says_who_keeps_theirs() {
    // 120 points for two panes that want 100 each (minus the 4 pt divider).
    let first_width = |strategy| {
        let harness = harness_of(state(0.5), 120.0, move |ratio, _| {
            Splitter::horizontal("split", ratio)
                .min_size(100.0)
                .lack_of_space_strategy(strategy)
        });
        let center = divider(&harness).center().x;
        center - size::SPLIT_DIVIDER / 2.0 - harness.get_by_label("Sidebar").rect().left()
    };
    let shared = first_width(LackOfSpaceStrategy::SimpleRatio);
    let first = first_width(LackOfSpaceStrategy::HonorTheFirstMinSize);
    let second = first_width(LackOfSpaceStrategy::HonorTheSecondMinSize);
    let avail = 120.0 - size::SPLIT_DIVIDER;
    assert!((shared - avail / 2.0).abs() < 1.5, "shared {shared}");
    assert!(
        (first - 100.0).abs() < 1.5,
        "first keeps its minimum: {first}"
    );
    assert!(
        (second - (avail - 100.0)).abs() < 1.5,
        "second keeps its minimum: {second}"
    );
}

#[test]
fn proportion_limits_stop_the_drag() {
    let mut harness = harness_of(state(0.5), WIDTH, |ratio, _| {
        Splitter::horizontal("split", ratio)
            .min_size(10.0)
            .proportion_limits(0.3, 0.6)
    });
    drag_to(&mut harness, 5000.0);
    assert!((harness.state().ratio - 0.6).abs() < 1e-3);
    drag_to(&mut harness, -5000.0);
    assert!((harness.state().ratio - 0.3).abs() < 1e-3);
}

#[test]
fn a_jb_splitter_loads_its_proportion_by_key_and_stores_every_change() {
    let mut initial = state(0.5);
    initial.properties.set("test.splitter.proportion", "0.3");
    let mut harness = harness_of(initial, WIDTH, |ratio, properties| {
        JBSplitter::horizontal("split", ratio, "test.splitter.proportion", properties)
    });
    assert!(
        (harness.state().ratio - 0.3).abs() < 1e-3,
        "loaded: {}",
        harness.state().ratio
    );
    let target = divider(&harness).center().x + 80.0;
    drag_to(&mut harness, target);
    let ratio = harness.state().ratio;
    assert!(ratio > 0.4);
    let stored: f32 = harness
        .state()
        .properties
        .get("test.splitter.proportion")
        .expect("stored")
        .parse()
        .expect("a number");
    assert!(
        (stored - ratio).abs() < 1e-3,
        "stored {stored}, ratio {ratio}"
    );
}

#[test]
fn a_jb_splitter_without_a_stored_value_keeps_the_callers_ratio_and_stores_nothing() {
    let harness = harness_of(state(0.4), WIDTH, |ratio, properties| {
        JBSplitter::horizontal("split", ratio, "test.splitter.proportion", properties)
    });
    assert!((harness.state().ratio - 0.4).abs() < 1e-3);
    assert!(
        !harness
            .state()
            .properties
            .is_set("test.splitter.proportion")
    );
}

#[test]
fn a_loaded_proportion_is_not_loaded_again_after_the_user_moved_the_divider() {
    let mut initial = state(0.5);
    initial.properties.set("k", "0.3");
    let mut harness = harness_of(initial, WIDTH, |ratio, properties| {
        JBSplitter::horizontal("split", ratio, "k", properties)
    });
    let target = divider(&harness).center().x + 80.0;
    drag_to(&mut harness, target);
    let moved = harness.state().ratio;
    harness.run();
    harness.run();
    assert!((harness.state().ratio - moved).abs() < 1e-6);
}
