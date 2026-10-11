//! `SplitPane` tests: dragging, keyboard, clamping, accessibility and snapshots.

mod common;

use archipel_kit::components::{Side, SplitPane};
use archipel_kit::cursor;
use archipel_kit::egui::{self, CursorIcon, Key, pos2, vec2};
use archipel_kit::testing::{self, Harness, NodeExt, NodeT, Queryable, Role, SnapshotOptions};
use archipel_kit::{Theme, tokens::size};

const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 140.0;
/// Space the two panes share: the width without the divider.
const AVAIL: f32 = WIDTH - size::SPLIT_DIVIDER;

struct State {
    ratio: f32,
    changed: usize,
    cursor: CursorIcon,
}

fn state(ratio: f32) -> State {
    State {
        ratio,
        changed: 0,
        cursor: CursorIcon::Default,
    }
}

fn side_by_side(ui: &mut egui::Ui, state: &mut State) {
    ui.allocate_ui(vec2(WIDTH, HEIGHT), |ui| {
        let out = SplitPane::horizontal("split", &mut state.ratio)
            .default_ratio(0.25)
            .min_size(80.0)
            .label("Resize sidebar")
            .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"));
        state.changed += usize::from(out.divider.changed());
        state.cursor = cursor::current(ui.ctx());
    });
}

fn harness(ratio: f32) -> Harness<'static, State> {
    let mut harness = testing::harness_state(Theme::dark(), state(ratio), side_by_side);
    harness.run();
    harness
}

/// Width of the first pane: from its left edge (where its label starts) to the divider.
fn first_width(harness: &Harness<'_, State>) -> f32 {
    harness.get_by_label("Resize sidebar").rect().center().x
        - size::SPLIT_DIVIDER / 2.0
        - harness.get_by_label("Sidebar").rect().left()
}

fn divider_center(harness: &Harness<'_, State>) -> egui::Pos2 {
    harness.get_by_label("Resize sidebar").rect().center()
}

fn drag_to(harness: &mut Harness<'_, State>, x: f32) {
    let from = divider_center(harness);
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
fn panes_share_the_space_by_ratio() {
    let harness = harness(0.25);
    let sidebar = harness.get_by_label("Sidebar").rect();
    let request = harness.get_by_label("Request").rect();
    assert!(sidebar.left() < request.left());
    let divider = harness.get_by_label("Resize sidebar").rect();
    assert!(
        (divider.center().x - (sidebar.left() + AVAIL * 0.25 + size::SPLIT_DIVIDER / 2.0)).abs()
            < 1.5,
        "divider at the ratio: {divider:?}"
    );
    assert_eq!(harness.state().changed, 0, "drawing alone changes nothing");
}

#[test]
fn dragging_the_divider_moves_it_and_writes_the_ratio() {
    let mut harness = harness(0.25);
    let before = harness.state().ratio;
    let target = divider_center(&harness).x + 100.0;
    drag_to(&mut harness, target);
    let ratio = harness.state().ratio;
    assert!(ratio > before + 0.2, "ratio grew: {before} -> {ratio}");
    assert!(harness.state().changed > 0);
    // The divider ends where the pointer let go (it keeps its grab offset).
    let end = divider_center(&harness).x;
    assert!(
        (end - target).abs() < 3.0,
        "divider at {end}, pointer at {target}"
    );
}

#[test]
fn dragging_stops_at_the_minimum_sizes() {
    let mut harness = harness(0.5);
    drag_to(&mut harness, 5000.0);
    let max = harness.state().ratio;
    assert!((max - (AVAIL - 80.0) / AVAIL).abs() < 1e-3, "max {max}");
    drag_to(&mut harness, -5000.0);
    let min = harness.state().ratio;
    assert!((min - 80.0 / AVAIL).abs() < 1e-3, "min {min}");
    assert!(first_width(&harness) >= 79.0);
}

#[test]
fn double_click_resets_to_the_default_ratio() {
    let mut harness = harness(0.7);
    let center = divider_center(&harness);
    harness.hover_at(center);
    harness.run();
    for _ in 0..2 {
        harness.drag_at(center);
        harness.run();
        harness.drop_at(center);
        harness.run();
    }
    assert!((harness.state().ratio - 0.25).abs() < 1e-3);
}

#[test]
fn arrows_move_by_a_step_and_home_end_jump_to_the_limits() {
    let mut harness = harness(0.5);
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    let start = harness.state().ratio;
    harness.key_press(Key::ArrowRight);
    harness.run();
    let step = size::SPLIT_KEY_STEP / AVAIL;
    assert!((harness.state().ratio - (start + step)).abs() < 2e-3);
    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!((harness.state().ratio - (start - step)).abs() < 2e-3);
    harness.key_press(Key::End);
    harness.run();
    assert!((harness.state().ratio - (AVAIL - 80.0) / AVAIL).abs() < 1e-3);
    harness.key_press(Key::Home);
    harness.run();
    assert!((harness.state().ratio - 80.0 / AVAIL).abs() < 1e-3);
    assert!(
        harness
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .is_focused(),
        "arrow keys move the divider, not the focus"
    );
}

#[test]
fn vertical_split_uses_up_and_down() {
    let mut harness = testing::harness_state(Theme::dark(), 0.5f32, |ui, ratio| {
        ui.allocate_ui(vec2(300.0, 200.0), |ui| {
            SplitPane::vertical("stack", ratio).min_size(40.0).show(
                ui,
                |ui| ui.label("Request"),
                |ui| ui.label("Response"),
            );
        });
    });
    harness.run();
    harness.get_by_label("Resize panes").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    let down = *harness.state();
    assert!(down > 0.5);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(
        *harness.state(),
        down,
        "left and right don't move a stacked split"
    );
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(*harness.state() < 0.5);
    assert!(
        harness.get_by_label("Request").rect().top()
            < harness.get_by_label("Response").rect().top()
    );
}

#[test]
fn cursor_is_the_resize_cursor_over_the_divider_only() {
    let mut harness = harness(0.5);
    harness.hover_at(divider_center(&harness));
    harness.run();
    assert_eq!(harness.state().cursor, CursorIcon::ResizeHorizontal);
    harness.hover_at(harness.get_by_label("Sidebar").rect().center());
    harness.run();
    assert_eq!(harness.state().cursor, CursorIcon::Default);

    let mut stacked = testing::harness_state(Theme::dark(), CursorIcon::Default, |ui, seen| {
        ui.allocate_ui(vec2(300.0, 200.0), |ui| {
            let mut ratio = 0.5;
            SplitPane::vertical("stack", &mut ratio).show(ui, |_| {}, |_| {});
            *seen = cursor::current(ui.ctx());
        });
    });
    stacked.run();
    stacked.hover_at(stacked.get_by_label("Resize panes").rect().center());
    stacked.run();
    assert_eq!(*stacked.state(), CursorIcon::ResizeVertical);
}

#[test]
fn the_divider_is_a_named_splitter_with_a_value() {
    let harness = harness(0.25);
    let divider = harness.get_by_label("Resize sidebar");
    assert_eq!(divider.role(), Role::Splitter);
    assert!(!divider.is_disabled());
    let value = divider.numeric_value().expect("a value");
    assert!((value - 25.0).abs() < 0.5, "percent of the space: {value}");
}

#[test]
fn a_locked_split_ignores_the_user() {
    let mut harness = testing::harness_state(Theme::dark(), 0.4f32, |ui, ratio| {
        ui.allocate_ui(vec2(WIDTH, HEIGHT), |ui| {
            SplitPane::horizontal("locked", ratio).enabled(false).show(
                ui,
                |ui| ui.label("Sidebar"),
                |ui| ui.label("Request"),
            );
        });
    });
    harness.run();
    assert!(harness.get_by_label("Resize panes").is_disabled());
    let center = harness.get_by_label("Resize panes").rect().center();
    harness.hover_at(center);
    harness.run();
    harness.drag_at(center);
    harness.run();
    harness.hover_at(center + vec2(80.0, 0.0));
    harness.run();
    harness.drop_at(center + vec2(80.0, 0.0));
    harness.run();
    assert_eq!(*harness.state(), 0.4);
}

#[test]
fn shrinking_clamps_the_layout_but_keeps_the_ratio() {
    // State: (window width, ratio).
    let mut harness =
        testing::harness_state(Theme::dark(), (WIDTH, 0.3f32), |ui, (width, ratio)| {
            ui.allocate_ui(vec2(*width, HEIGHT), |ui| {
                SplitPane::horizontal("resize", ratio).min_size(100.0).show(
                    ui,
                    |ui| ui.label("Sidebar"),
                    |ui| ui.label("Request"),
                );
            });
        });
    harness.run();
    let second_offset = |harness: &Harness<'_, (f32, f32)>| {
        harness.get_by_label("Request").rect().left()
            - harness.get_by_label("Sidebar").rect().left()
    };
    let roomy = second_offset(&harness);
    assert!((roomy - (0.3 * (WIDTH - size::SPLIT_DIVIDER) + size::SPLIT_DIVIDER)).abs() < 1.5);

    // Too narrow for two 100-point panes: they share the room, the second keeps its share.
    harness.state_mut().0 = 150.0;
    harness.run();
    let narrow = second_offset(&harness);
    assert!(narrow < 150.0 - 40.0 && narrow > 40.0, "{narrow}");

    // Narrow, but room for the minimums: the first pane is pushed to its minimum.
    harness.state_mut().0 = 220.0;
    harness.run();
    assert!((second_offset(&harness) - (100.0 + size::SPLIT_DIVIDER)).abs() < 1.5);

    // Growing back restores the caller's split: the ratio was never rewritten.
    harness.state_mut().0 = WIDTH;
    harness.run();
    assert!((second_offset(&harness) - roomy).abs() < 1.5);
    assert_eq!(harness.state().1, 0.3);
}

#[test]
fn bad_input_does_not_panic() {
    for ratio in [f32::NAN, f32::INFINITY, -3.0, 7.0] {
        for width in [0.0, 5.0, 60.0, 400.0] {
            let mut ratio = ratio;
            let mut harness = testing::harness(Theme::dark(), |ui| {
                ui.allocate_ui(vec2(width, 80.0), |ui| {
                    SplitPane::horizontal("bad", &mut ratio)
                        .min_sizes(f32::NAN, 120.0)
                        .default_ratio(f32::NAN)
                        .show(ui, |ui| ui.label("A"), |ui| ui.label("B"));
                });
            });
            harness.run();
        }
    }
}

#[test]
fn splits_nest_into_three_columns() {
    let mut harness =
        testing::harness_state(Theme::dark(), (0.2f32, 0.5f32), |ui, (outer, inner)| {
            ui.allocate_ui(vec2(600.0, HEIGHT), |ui| {
                SplitPane::horizontal("outer", outer)
                    .min_size(60.0)
                    .label("Resize sidebar")
                    .show(
                        ui,
                        |ui| ui.label("Sidebar"),
                        |ui| {
                            SplitPane::horizontal("inner", inner)
                                .min_size(60.0)
                                .label("Resize response")
                                .show(ui, |ui| ui.label("Request"), |ui| ui.label("Response"));
                        },
                    );
            });
        });
    harness.run();
    let xs: Vec<f32> = ["Sidebar", "Request", "Response"]
        .iter()
        .map(|name| harness.get_by_label(name).rect().left())
        .collect();
    assert!(xs[0] < xs[1] && xs[1] < xs[2], "{xs:?}");
    // Moving the inner divider doesn't touch the outer one.
    harness.get_by_label("Resize response").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    let (outer, inner) = *harness.state();
    assert_eq!(outer, 0.2);
    assert!(inner > 0.5);
}

#[test]
fn split_screen_passes_the_audit() {
    let mut harness =
        testing::harness_state(Theme::dark(), (0.2f32, 0.5f32, 0.6f32), |ui, (a, b, c)| {
            ui.allocate_ui(vec2(600.0, 300.0), |ui| {
                SplitPane::horizontal("outer", a)
                    .min_size(60.0)
                    .label("Resize sidebar")
                    .show(
                        ui,
                        |ui| ui.label("Collections"),
                        |ui| {
                            SplitPane::vertical("inner", b)
                                .min_size(60.0)
                                .label("Resize request")
                                .show(
                                    ui,
                                    |ui| ui.label("Request"),
                                    |ui| {
                                        SplitPane::horizontal("deep", c)
                                            .min_size(60.0)
                                            .label("Resize response")
                                            .show(
                                                ui,
                                                |ui| ui.label("Body"),
                                                |ui| ui.label("Headers"),
                                            );
                                    },
                                );
                        },
                    );
            });
        });
    harness.run();
    testing::audit(&mut harness).assert_clean();
}

// Snapshots.

/// A canvas like a window: frame color around two islands with the divider between them.
fn canvas(salt: &str, ratio: &mut f32, vertical: bool, ui: &mut egui::Ui) {
    let colors = Theme::colors(ui.ctx());
    ui.allocate_ui(vec2(300.0, 132.0), |ui| {
        let pane = |ui: &mut egui::Ui, text: &str| {
            let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            ui.painter().rect_filled(rect, 6.0, colors.field);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(13.0),
                colors.text,
            );
        };
        let split = if vertical {
            SplitPane::vertical(salt, ratio)
        } else {
            SplitPane::horizontal(salt, ratio)
        };
        split
            .min_size(60.0)
            .show(ui, |ui| pane(ui, "Request"), |ui| pane(ui, "Response"));
    });
}

#[test]
fn snapshot() {
    testing::snapshot_themes("split_pane", common::IMG, |ui| {
        canvas("side", &mut 0.4, false, ui);
        ui.add_space(8.0);
        canvas("stack", &mut 0.5, true, ui);
    });
}

/// Snapshots one interaction state at 2×, in both themes.
fn snapshot_state(name: &str, act: impl Fn(&mut Harness<'_, f32>, egui::Pos2)) {
    let mut results = testing::SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut pending = Some(theme);
        let mut harness = testing::builder::<f32>()
            .with_pixels_per_point(2.0)
            .with_size(vec2(340.0, 160.0))
            .build_ui_state(
                move |ui, ratio| {
                    if let Some(theme) = pending.take() {
                        archipel_kit::install(ui.ctx(), theme);
                        ui.ctx().request_repaint();
                        return;
                    }
                    let colors = Theme::colors(ui.ctx());
                    ui.painter().rect_filled(ui.max_rect(), 0.0, colors.frame);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        canvas("state", ratio, false, ui);
                    });
                },
                0.4,
            );
        harness.run_ok();
        // AccessKit bounds are in physical pixels, pointer positions in points.
        let divider = harness.get_by_label("Resize panes").rect().center() / 2.0;
        act(&mut harness, divider);
        results.add(harness.try_snapshot_options(
            format!("{name}.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

#[test]
fn snapshot_hover() {
    snapshot_state("split_pane_hover", |harness, divider| {
        harness.hover_at(divider);
        harness.run_ok();
    });
}

#[test]
fn snapshot_dragging() {
    snapshot_state("split_pane_dragging", |harness, divider| {
        harness.hover_at(divider);
        harness.run_ok();
        harness.drag_at(divider);
        harness.run_ok();
        harness.hover_at(divider + vec2(30.0, 0.0));
        harness.run_ok();
        harness.hover_at(divider + vec2(30.0, 0.0));
        harness.run_ok();
    });
}

#[test]
fn snapshot_focused() {
    snapshot_state("split_pane_focus", |harness, _| {
        harness.get_by_label("Resize panes").focus();
        harness.run_ok();
    });
}

#[test]
fn tab_reaches_the_divider() {
    let mut harness = harness(0.5);
    harness.key_press(Key::Tab);
    harness.run();
    assert!(
        harness
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .is_focused(),
        "Tab focuses the divider"
    );
}

#[test]
fn tab_visits_an_outer_divider_before_the_nested_ones() {
    let mut harness =
        testing::harness_state(Theme::dark(), (0.2f32, 0.5f32), |ui, (outer, inner)| {
            ui.allocate_ui(vec2(600.0, HEIGHT), |ui| {
                SplitPane::horizontal("outer", outer)
                    .min_size(60.0)
                    .label("Resize sidebar")
                    .show(
                        ui,
                        |ui| ui.label("Sidebar"),
                        |ui| {
                            SplitPane::horizontal("inner", inner)
                                .min_size(60.0)
                                .label("Resize response")
                                .show(ui, |ui| ui.label("Request"), |ui| ui.label("Response"));
                        },
                    );
            });
        });
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(
        harness
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .is_focused()
    );
    harness.key_press(Key::Tab);
    harness.run();
    assert!(
        harness
            .get_by_label("Resize response")
            .accesskit_node()
            .is_focused()
    );
}

#[test]
fn quick_key_repeats_each_move_the_divider() {
    let mut harness = harness(0.5);
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    for _ in 0..3 {
        harness.key_press(Key::ArrowRight);
    }
    harness.run();
    let moved = (harness.state().ratio - 0.5) * AVAIL;
    assert!(
        (moved - 3.0 * size::SPLIT_KEY_STEP).abs() < 1.0,
        "moved {moved} points"
    );
}

#[test]
fn enter_does_nothing_and_escape_leaves_the_divider() {
    let mut harness = harness(0.5);
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().ratio, 0.5);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(
        !harness
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .is_focused()
    );
    assert_eq!(harness.state().ratio, 0.5);
}

// Collapse.

struct Collapsible {
    ratio: f32,
    collapsed: bool,
    side: Side,
    changed: usize,
}

fn collapsible(side: Side, ratio: f32, collapsed: bool) -> Harness<'static, Collapsible> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        Collapsible {
            ratio,
            collapsed,
            side,
            changed: 0,
        },
        |ui, state| {
            ui.allocate_ui(vec2(WIDTH, HEIGHT), |ui| {
                let out = SplitPane::horizontal("collapse", &mut state.ratio)
                    .min_size(80.0)
                    .label("Resize sidebar")
                    .collapsible(state.side, &mut state.collapsed)
                    .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"));
                state.changed += usize::from(out.divider.changed());
            });
        },
    );
    harness.run();
    harness
}

fn divider_x(harness: &Harness<'_, Collapsible>) -> f32 {
    harness.get_by_label("Resize sidebar").rect().center().x
}

/// Where the split starts: the divider of an open one is at `origin + share + gap / 2`.
fn origin() -> f32 {
    divider_x(&collapsible(Side::First, 0.5, false)) - 0.5 * AVAIL - size::SPLIT_DIVIDER / 2.0
}

fn drag_divider_to(harness: &mut Harness<'_, Collapsible>, x: f32) {
    let from = harness.get_by_label("Resize sidebar").rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=10 {
        let t = step as f32 / 10.0;
        harness.hover_at(pos2(from.x + (x - from.x) * t, from.y));
        harness.run();
    }
    harness.drop_at(pos2(x, from.y));
    harness.run();
}

#[test]
fn the_caller_collapses_and_expands_and_the_ratio_survives() {
    let mut harness = collapsible(Side::First, 0.4, false);
    let open = divider_x(&harness);
    harness.state_mut().collapsed = true;
    harness.run();
    let edge = divider_x(&harness);
    assert!(
        edge < open - 100.0 && edge < origin() + size::SPLIT_DIVIDER + 1.0,
        "divider at the edge: {edge}, was {open}"
    );
    assert_eq!(harness.state().ratio, 0.4, "collapsing keeps the ratio");
    assert_eq!(
        harness.state().changed,
        0,
        "the caller's change isn't the user's"
    );
    harness.state_mut().collapsed = false;
    harness.run();
    assert!((divider_x(&harness) - open).abs() < 1.0, "ratio restored");
}

#[test]
fn collapsed_state_starts_collapsed_with_a_grabbable_edge() {
    let harness = collapsible(Side::First, 0.4, true);
    let divider = harness.get_by_label("Resize sidebar").rect();
    assert!(
        divider.width() >= 8.0 && divider.left() < origin() + 1.0,
        "{divider:?}"
    );
    assert!(
        harness.get_by_label("Request").rect().left() < origin() + 8.0,
        "the second pane takes the room"
    );
}

#[test]
fn dragging_past_the_minimum_collapses() {
    // Just past the minimum (80) but within the threshold: still open, at the minimum.
    let mut harness = collapsible(Side::First, 0.4, false);
    let start = divider_x(&harness);
    drag_divider_to(&mut harness, start - (0.4 * AVAIL - 80.0) - 20.0);
    assert!(!harness.state().collapsed);
    assert!((harness.state().ratio - 80.0 / AVAIL).abs() < 1e-3);

    // Far past it: collapsed, and the ratio is the one from before the drag.
    let mut harness = collapsible(Side::First, 0.4, false);
    drag_divider_to(&mut harness, start - 200.0);
    assert!(harness.state().collapsed);
    assert_eq!(harness.state().ratio, 0.4);
    assert!(harness.state().changed > 0);
    assert!(divider_x(&harness) < origin() + 6.0);
}

#[test]
fn dragging_out_expands() {
    let mut harness = collapsible(Side::First, 0.4, true);
    drag_divider_to(&mut harness, 200.0);
    assert!(!harness.state().collapsed);
    let x = divider_x(&harness);
    assert!((x - 200.0).abs() < 4.0, "follows the pointer: {x}");
    assert!((harness.state().ratio - (200.0 - origin()) / AVAIL).abs() < 0.03);
}

#[test]
fn a_short_drag_from_the_edge_stays_collapsed() {
    let mut harness = collapsible(Side::First, 0.4, true);
    drag_divider_to(&mut harness, 30.0);
    assert!(harness.state().collapsed);
    assert_eq!(harness.state().ratio, 0.4);
}

#[test]
fn collapse_and_expand_are_stable_while_dragging_back_and_forth() {
    let mut harness = collapsible(Side::First, 0.4, false);
    let from = harness.get_by_label("Resize sidebar").rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    let mut flips = 0;
    let mut last = false;
    for x in (0..=100).rev().step_by(2).chain((0..=100).step_by(2)) {
        harness.hover_at(pos2(x as f32 + 4.0, from.y));
        harness.run();
        flips += usize::from(harness.state().collapsed != last);
        last = harness.state().collapsed;
    }
    // Down to collapsed once, back up to expanded once: no flicker around the boundary.
    assert_eq!(flips, 2, "collapsed state flips");
    harness.drop_at(pos2(104.0, from.y));
    harness.run();
}

#[test]
fn enter_toggles_the_focused_divider_and_restores_the_ratio() {
    let mut harness = collapsible(Side::First, 0.4, false);
    let open = divider_x(&harness);
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().collapsed);
    assert!(harness.state().changed > 0);
    assert_eq!(harness.state().ratio, 0.4);
    assert!(
        harness
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .is_focused(),
        "focus stays on the divider"
    );
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!harness.state().collapsed);
    assert_eq!(harness.state().ratio, 0.4);
    assert!((divider_x(&harness) - open).abs() < 1.0);
    harness.key_press(Key::Space);
    harness.run();
    assert!(harness.state().collapsed, "Space toggles too");
}

#[test]
fn arrow_toward_the_content_expands_a_collapsed_pane() {
    let mut harness = collapsible(Side::First, 0.4, true);
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::Home);
    harness.run();
    assert!(
        harness.state().collapsed,
        "nothing else moves a collapsed divider"
    );
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(!harness.state().collapsed);
    assert_eq!(harness.state().ratio, 0.4);
}

#[test]
fn double_click_expands_a_collapsed_pane_to_its_ratio() {
    let mut harness = collapsible(Side::First, 0.6, true);
    let center = harness.get_by_label("Resize sidebar").rect().center();
    harness.hover_at(center);
    harness.run();
    for _ in 0..2 {
        harness.drag_at(center);
        harness.run();
        harness.drop_at(center);
        harness.run();
    }
    assert!(!harness.state().collapsed);
    assert_eq!(harness.state().ratio, 0.6);
}

#[test]
fn the_second_side_collapses_to_the_far_edge() {
    let mut harness = collapsible(Side::Second, 0.5, false);
    let open = divider_x(&harness);
    drag_divider_to(&mut harness, open + 260.0);
    assert!(harness.state().collapsed);
    assert!(
        divider_x(&harness) > WIDTH - 12.0,
        "{}",
        divider_x(&harness)
    );
    assert_eq!(harness.state().ratio, 0.5, "from before the drag");
    drag_divider_to(&mut harness, 250.0);
    assert!(!harness.state().collapsed);
    // Keys: ArrowLeft (toward the content) expands, Enter collapses.
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().collapsed);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(
        harness.state().collapsed,
        "the arrow away from the content does nothing"
    );
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().collapsed);
}

#[test]
fn the_divider_reports_expanded_and_collapsed() {
    let mut harness = collapsible(Side::First, 0.4, false);
    let expanded = |harness: &Harness<'_, Collapsible>| {
        harness
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .data()
            .is_expanded()
    };
    assert_eq!(expanded(&harness), Some(true));
    harness.state_mut().collapsed = true;
    harness.run();
    assert_eq!(expanded(&harness), Some(false));
    assert_eq!(
        harness.get_by_label("Resize sidebar").role(),
        Role::Splitter
    );
    // A split that can't collapse says nothing about it.
    let plain = harness_plain();
    assert_eq!(
        plain
            .get_by_label("Resize sidebar")
            .accesskit_node()
            .data()
            .is_expanded(),
        None
    );
}

fn harness_plain() -> Harness<'static, State> {
    harness(0.5)
}

#[test]
fn a_locked_collapsible_split_ignores_the_keyboard() {
    let mut harness = testing::harness_state(Theme::dark(), (0.4f32, false), |ui, (r, c)| {
        ui.allocate_ui(vec2(WIDTH, HEIGHT), |ui| {
            SplitPane::horizontal("locked", r)
                .enabled(false)
                .collapsible(Side::First, c)
                .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"));
        });
    });
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!harness.state().1);
}

#[test]
fn a_collapsed_pane_is_hidden_from_the_user() {
    let harness = collapsible(Side::First, 0.4, true);
    // Its content is laid out but neither visible nor reachable.
    let sidebar = harness.query_by_label("Sidebar");
    assert!(
        sidebar.is_none_or(|node| node.accesskit_node().data().is_hidden() || node.is_disabled())
    );
}

#[test]
fn collapsed_splits_survive_bad_input() {
    for side in [Side::First, Side::Second] {
        for width in [0.0, 5.0, 60.0, 400.0] {
            for ratio in [f32::NAN, 7.0] {
                let (mut ratio, mut collapsed) = (ratio, true);
                let mut harness = testing::harness(Theme::dark(), |ui| {
                    ui.allocate_ui(vec2(width, 80.0), |ui| {
                        SplitPane::vertical("bad", &mut ratio)
                            .min_sizes(f32::NAN, 120.0)
                            .collapsible(side, &mut collapsed)
                            .show(ui, |ui| ui.label("A"), |ui| ui.label("B"));
                    });
                });
                harness.run();
            }
        }
    }
}

#[test]
fn collapsed_split_screen_passes_the_audit() {
    for side in [Side::First, Side::Second] {
        let mut harness = collapsible(side, 0.4, true);
        testing::audit(&mut harness).assert_clean();
        harness.state_mut().collapsed = false;
        harness.run();
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn collapsed_snapshot() {
    let mut results = testing::SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut pending = Some(theme);
        let mut harness = testing::builder::<f32>()
            .with_pixels_per_point(2.0)
            .with_size(vec2(340.0, 160.0))
            .build_ui_state(
                move |ui, ratio| {
                    if let Some(theme) = pending.take() {
                        archipel_kit::install(ui.ctx(), theme);
                        ui.ctx().request_repaint();
                        return;
                    }
                    let colors = Theme::colors(ui.ctx());
                    ui.painter().rect_filled(ui.max_rect(), 0.0, colors.frame);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        let mut collapsed = true;
                        ui.allocate_ui(vec2(300.0, 132.0), |ui| {
                            let pane = |ui: &mut egui::Ui, text: &str| {
                                let (rect, _) = ui
                                    .allocate_exact_size(ui.available_size(), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 6.0, colors.field);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    egui::FontId::proportional(13.0),
                                    colors.text,
                                );
                            };
                            SplitPane::horizontal("collapsed", ratio)
                                .min_size(60.0)
                                .collapsible(Side::First, &mut collapsed)
                                .show(ui, |ui| pane(ui, "Sidebar"), |ui| pane(ui, "Request"));
                        });
                    });
                },
                0.4,
            );
        harness.run_ok();
        results.add(harness.try_snapshot_options(
            format!("split_pane_collapsed.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

#[test]
fn collapsing_by_dragging_restores_the_size_from_before_the_drag() {
    let mut harness = collapsible(Side::First, 0.4, false);
    let start = divider_x(&harness);
    drag_divider_to(&mut harness, start - 250.0);
    assert!(harness.state().collapsed);
    assert_eq!(
        harness.state().ratio,
        0.4,
        "not the minimum it was dragged to"
    );
    // Expanding by the keyboard, by code or by a double click brings back that size.
    harness.get_by_label("Resize sidebar").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!harness.state().collapsed);
    assert!((divider_x(&harness) - start).abs() < 1.0, "back at {start}");
    assert_eq!(harness.state().ratio, 0.4);
}

#[test]
fn double_click_resets_an_open_pane_but_only_expands_a_collapsed_one() {
    let mut harness = collapsible(Side::First, 0.6, false);
    let center = harness.get_by_label("Resize sidebar").rect().center();
    harness.hover_at(center);
    harness.run();
    for _ in 0..2 {
        harness.drag_at(center);
        harness.run();
        harness.drop_at(center);
        harness.run();
    }
    assert!(!harness.state().collapsed);
    assert!((harness.state().ratio - 0.5).abs() < 1e-3, "default ratio");
}

/// Snapshots the collapsed edge in one interaction state, at 2×, in both themes.
fn snapshot_collapsed_state(name: &str, act: impl Fn(&mut Harness<'_, f32>, egui::Pos2)) {
    let mut results = testing::SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut pending = Some(theme);
        let mut harness = testing::builder::<f32>()
            .with_pixels_per_point(2.0)
            .with_size(vec2(340.0, 160.0))
            .build_ui_state(
                move |ui, ratio| {
                    if let Some(theme) = pending.take() {
                        archipel_kit::install(ui.ctx(), theme);
                        ui.ctx().request_repaint();
                        return;
                    }
                    let colors = Theme::colors(ui.ctx());
                    ui.painter().rect_filled(ui.max_rect(), 0.0, colors.frame);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        let mut collapsed = true;
                        ui.allocate_ui(vec2(300.0, 132.0), |ui| {
                            let pane = |ui: &mut egui::Ui, text: &str| {
                                let (rect, _) = ui
                                    .allocate_exact_size(ui.available_size(), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 6.0, colors.field);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    egui::FontId::proportional(13.0),
                                    colors.text,
                                );
                            };
                            SplitPane::horizontal("collapsed", ratio)
                                .min_size(60.0)
                                .collapsible(Side::First, &mut collapsed)
                                .show(ui, |ui| pane(ui, "Sidebar"), |ui| pane(ui, "Request"));
                        });
                    });
                },
                0.4,
            );
        harness.run_ok();
        let divider = harness.get_by_label("Resize panes").rect().center() / 2.0;
        act(&mut harness, divider);
        results.add(harness.try_snapshot_options(
            format!("{name}.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

#[test]
fn snapshot_collapsed_hover() {
    snapshot_collapsed_state("split_pane_collapsed_hover", |harness, divider| {
        harness.hover_at(divider);
        harness.run_ok();
    });
}

#[test]
fn snapshot_collapsed_focus() {
    snapshot_collapsed_state("split_pane_collapsed_focus", |harness, _| {
        harness.get_by_label("Resize panes").focus();
        harness.run_ok();
    });
}
