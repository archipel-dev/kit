//! `Workbench` tests: the islands look by construction, dividers, state and snapshots.

mod common;

use archipel_kit::components::{
    Anchor, IconButton, StatusBarItem, Workbench, WorkbenchResponse, WorkbenchState,
};
use archipel_kit::egui::{self, Key, vec2};
use archipel_kit::testing::{self, Harness, Queryable};
use archipel_kit::tokens::{radius, size};
use archipel_kit::{Glyph, Theme};

const WINDOW: egui::Vec2 = vec2(1280.0, 800.0);

/// Which tool windows the test layout has.
#[derive(Clone, Copy)]
struct Layout {
    left: bool,
    right: bool,
    bottom: bool,
    bars: bool,
}

const THREE: Layout = Layout {
    left: true,
    right: false,
    bottom: true,
    bars: true,
};
const FOUR: Layout = Layout {
    left: true,
    right: true,
    bottom: true,
    bars: true,
};

/// The state is `#[non_exhaustive]`: start from the default and set fields.
fn state_with(set: impl FnOnce(&mut WorkbenchState)) -> WorkbenchState {
    let mut state = WorkbenchState::default();
    set(&mut state);
    state
}

#[derive(Default)]
struct App {
    state: WorkbenchState,
    hidden: Vec<Anchor>,
    resized: Vec<Anchor>,
}

fn workbench(
    ui: &mut egui::Ui,
    state: &mut WorkbenchState,
    layout: Layout,
) -> WorkbenchResponse<()> {
    let mut wb = Workbench::new("window", state);
    if layout.bars {
        wb = wb
            .toolbar(|ui| {
                ui.add(IconButton::new(Glyph::Run, "Run"));
                ui.label("archipel");
            })
            .status_bar(|ui| {
                ui.add(StatusBarItem::new("main"));
                ui.add(StatusBarItem::new("UTF-8"));
            });
    }
    if layout.left {
        wb = wb
            .left("Project", |ui| {
                ui.label("src/");
            })
            .actions(Anchor::Left, |ui| {
                ui.add(IconButton::new(Glyph::CollapseAll, "Collapse all"));
            });
    }
    if layout.right {
        wb = wb.right("Outline", |ui| {
            ui.label("fn main");
        });
    }
    if layout.bottom {
        wb = wb.bottom("Terminal", |ui| {
            ui.label("$ cargo run");
        });
    }
    wb.show(ui, |ui| {
        ui.label("Editor");
    })
}

fn harness(theme: Theme, layout: Layout, state: WorkbenchState) -> Harness<'static, App> {
    let app = App {
        state,
        ..App::default()
    };
    let mut harness = testing::window_harness_state(theme, WINDOW, app, move |ui, app| {
        let out = workbench(ui, &mut app.state, layout);
        app.hidden.extend(out.hidden);
        app.resized.extend(out.resized);
    });
    harness.run();
    harness
}

fn left_edge(harness: &Harness<'_, App>, label: &str) -> f32 {
    harness.get_by_label(label).rect().left()
}

#[test]
fn three_and_four_island_layouts_pass_the_audit() {
    for layout in [THREE, FOUR] {
        for theme in [Theme::dark(), Theme::light()] {
            let mut harness = harness(theme, layout, WorkbenchState::default());
            // Everything is there and reachable: the audit presses Tab through it.
            for label in ["Hide Project", "Resize Project", "Resize Terminal", "Run"] {
                assert!(harness.query_by_label(label).is_some(), "{label} is shown");
            }
            assert_eq!(
                harness.query_by_label("Resize Outline").is_some(),
                layout.right
            );
            testing::audit(&mut harness).assert_clean();
        }
    }
}

#[test]
fn the_minimum_window_audits_clean_with_oversized_tool_windows() {
    let state = state_with(|s| {
        s.left = 900.0;
        s.right = 900.0;
        s.bottom = 900.0;
    });
    let mut harness =
        testing::window_harness_state(Theme::dark(), vec2(800.0, 500.0), state, |ui, state| {
            workbench(ui, state, FOUR);
        });
    harness.run();
    let editor = harness.get_by_label("Editor").rect();
    assert!(editor.width() > 1.0, "the center keeps room: {editor:?}");
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn dividers_conform() {
    for (label, anchor) in [
        ("Resize Project", Anchor::Left),
        ("Resize Outline", Anchor::Right),
        ("Resize Terminal", Anchor::Bottom),
    ] {
        let mut state = WorkbenchState::default();
        testing::conformance(label, move |ui, enabled| {
            ui.allocate_ui(vec2(900.0, 520.0), |ui| {
                let mut wb = Workbench::new("conformance", &mut state)
                    .enabled(enabled)
                    .left("Project", |_| {})
                    .right("Outline", |_| {})
                    .bottom("Terminal", |_| {});
                // Keep the other dividers out of the way of the one under test.
                let _ = &mut wb;
                let out = wb.show(ui, |_| {});
                out.dividers
                    .into_iter()
                    .find_map(|(a, response)| (a == anchor).then_some(response))
                    .expect("the divider is drawn")
            })
            .inner
        })
        .run();
    }
}

#[test]
fn the_hide_button_collapses_and_the_divider_restores() {
    let mut harness = harness(Theme::dark(), FOUR, WorkbenchState::default());
    let open = left_edge(&harness, "Editor");
    harness.get_by_label("Hide Project").click();
    harness.run();
    assert!(harness.state().state.left_collapsed);
    assert_eq!(
        harness.state().hidden,
        [Anchor::Left],
        "the hide click is reported"
    );
    assert!(
        left_edge(&harness, "Editor") < open - 200.0,
        "the center takes the room"
    );
    assert_eq!(
        harness.state().state.left,
        size::TOOL_WINDOW_DEFAULT,
        "the size is kept"
    );

    // Enter on the focused divider expands it again, at the size from before.
    harness.get_by_label("Resize Project").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!harness.state().state.left_collapsed);
    assert!(harness.state().resized.contains(&Anchor::Left));
    assert!((left_edge(&harness, "Editor") - open).abs() < 1.0);
}

#[test]
fn the_caller_collapses_and_expands_from_code() {
    let mut harness = harness(Theme::dark(), FOUR, WorkbenchState::default());
    let open = left_edge(&harness, "Editor");
    for anchor in [Anchor::Left, Anchor::Right, Anchor::Bottom] {
        harness.state_mut().state.toggle(anchor);
    }
    harness.run();
    assert!(left_edge(&harness, "Editor") < open - 200.0);
    assert!(harness.state().resized.is_empty() && harness.state().hidden.is_empty());
    for anchor in [Anchor::Left, Anchor::Right, Anchor::Bottom] {
        harness.state_mut().state.toggle(anchor);
    }
    harness.run();
    assert!((left_edge(&harness, "Editor") - open).abs() < 1.0);
}

#[test]
fn dragging_a_divider_writes_the_size_in_points() {
    let mut harness = harness(Theme::dark(), FOUR, WorkbenchState::default());
    let divider = harness.get_by_label("Resize Project").rect().center();
    harness.hover_at(divider);
    harness.run();
    harness.drag_at(divider);
    harness.run();
    for step in 1..=6 {
        harness.hover_at(divider + vec2(40.0 * step as f32 / 6.0, 0.0));
        harness.run();
    }
    harness.drop_at(divider + vec2(40.0, 0.0));
    harness.run();
    let state = &harness.state().state;
    assert!(
        (state.left - (size::TOOL_WINDOW_DEFAULT + 40.0)).abs() < 3.0,
        "{}",
        state.left
    );
    assert_eq!(
        state.right,
        size::TOOL_WINDOW_DEFAULT,
        "the others are untouched"
    );
    assert!(harness.state().resized.contains(&Anchor::Left));
}

#[test]
fn a_window_resize_gives_the_slack_to_the_center_only() {
    let mut harness = harness(Theme::dark(), FOUR, WorkbenchState::default());
    let before = left_edge(&harness, "Editor");
    harness.set_size(vec2(1000.0, 800.0));
    harness.run();
    assert_eq!(left_edge(&harness, "Editor"), before);
    assert_eq!(harness.state().state, WorkbenchState::default());
    assert!(
        harness.state().resized.is_empty(),
        "a window resize is not the user's"
    );
}

#[test]
fn state_survives_a_round_trip_and_older_files() {
    let state = state_with(|s| {
        s.left = 300.0;
        s.bottom = 150.0;
        s.right_collapsed = true;
    });
    let json = serde_json::to_string(&state).expect("serialize");
    let back: WorkbenchState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, state);
    let old: WorkbenchState = serde_json::from_str(r#"{"left": 310.0}"#).expect("older file");
    assert_eq!(old, state_with(|s| s.left = 310.0));

    // The restored state draws the same layout.
    let a = harness(Theme::dark(), FOUR, state);
    let b = harness(Theme::dark(), FOUR, back);
    assert_eq!(left_edge(&a, "Editor"), left_edge(&b, "Editor"));
    assert_eq!(
        left_edge(&a, "Hide Terminal"),
        left_edge(&b, "Hide Terminal")
    );
}

#[test]
fn bad_input_does_not_panic() {
    for (width, height) in [(0.0, 0.0), (30.0, 20.0), (200.0, 120.0)] {
        let state = state_with(|s| {
            s.left = f32::NAN;
            s.right = f32::INFINITY;
            s.bottom = -5.0;
        });
        let mut harness = testing::window_harness_state(
            Theme::dark(),
            vec2(width, height),
            state,
            |ui, state| {
                workbench(ui, state, FOUR);
            },
        );
        harness.run();
    }
    // Only a center, and an installed-less context is covered by the kit's fallbacks.
    let mut state = WorkbenchState::default();
    let mut harness = testing::window_harness_state(Theme::light(), WINDOW, (), |ui, _| {
        Workbench::new("bare", &mut state).show(ui, |ui| {
            ui.label("Editor");
        });
    });
    harness.run();
    assert!(harness.query_by_label("Editor").is_some());
}

/// The first and last pixel of every run of `color` along a row or column of the image.
fn runs(pixels: impl Iterator<Item = [u8; 4]>, color: egui::Color32) -> Vec<(usize, usize)> {
    let near = |p: [u8; 4]| {
        [
            p[0] as i32 - color.r() as i32,
            p[1] as i32 - color.g() as i32,
            p[2] as i32 - color.b() as i32,
        ]
        .iter()
        .all(|d| d.abs() <= 2)
    };
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    let mut last = 0;
    for (i, p) in pixels.enumerate() {
        match (near(p), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                runs.push((s, i - 1));
                start = None;
            }
            _ => {}
        }
        last = i;
    }
    if let Some(s) = start {
        runs.push((s, last));
    }
    runs
}

#[test]
fn islands_are_one_gap_apart_with_the_island_radius() {
    let layout = Layout {
        bars: false,
        ..FOUR
    };
    let gap = size::ISLAND_GAP as usize;
    for theme in [Theme::dark(), Theme::light()] {
        let colors = theme.colors;
        let mut harness = testing::window_harness_state(
            theme,
            vec2(900.0, 600.0),
            WorkbenchState::default(),
            move |ui, state| {
                workbench(ui, state, layout);
            },
        );
        harness.run();
        let image = harness.render().expect("render");
        let (w, h) = (image.width() as usize, image.height() as usize);

        // A row through the left, center and right islands (above the bottom window).
        let y = 200;
        let row = runs(
            (0..w).map(|x| image.get_pixel(x as u32, y).0),
            colors.island,
        );
        assert_eq!(row.len(), 3, "left, center and right island: {row:?}");
        assert_eq!(row[0].0, gap, "the gap at the window's edge");
        assert_eq!(row[1].0 - row[0].1 - 1, gap, "left to center");
        assert_eq!(row[2].0 - row[1].1 - 1, gap, "center to right");
        assert_eq!(w - 1 - row[2].1, gap, "the gap at the other edge");

        // A column through the center: the center island over the bottom window.
        let x = row[1].0 + 100;
        let col = runs(
            (0..h).map(|y| image.get_pixel(x as u32, y as u32).0),
            colors.island,
        );
        assert_eq!(col.len(), 2, "center and bottom island: {col:?}");
        assert_eq!(col[0].0, gap, "the gap at the top");
        assert_eq!(col[1].0 - col[0].1 - 1, gap, "center to bottom");
        assert_eq!(h - 1 - col[1].1, gap, "the gap at the bottom");

        // The corner is round: the first row of the island starts about a radius in.
        let top = runs(
            (0..w).map(|x| image.get_pixel(x as u32, gap as u32).0),
            colors.island,
        );
        let inset = top[0].0 - gap;
        let r = radius::ISLAND as usize;
        assert!(
            inset * 2 >= r && inset <= r,
            "island radius {r}: the top row starts {inset} px in"
        );
        assert_eq!(
            image.get_pixel(gap as u32, gap as u32).0[..3],
            [colors.frame.r(), colors.frame.g(), colors.frame.b()],
            "the corner shows the frame"
        );
    }
}

/// Fills the window with a layout, like an app would, and checks it.
fn snapshot(name: &str, layout: Layout) {
    let mut state = WorkbenchState::default();
    testing::snapshot_themes_window(name, common::IMG, WINDOW, move |ui| {
        let mut wb = Workbench::new("window", &mut state)
            .toolbar(|ui| {
                ui.add(IconButton::new(Glyph::Project, "Project"));
                ui.label("archipel-kit");
                ui.add(IconButton::new(Glyph::Run, "Run"));
            })
            .status_bar(|ui| {
                ui.add(StatusBarItem::new("main").icon(Glyph::Branch));
                ui.add(StatusBarItem::new("UTF-8"));
            })
            .left("Project", |ui| {
                ui.label("  src/");
                ui.label("  tests/");
            })
            .actions(Anchor::Left, |ui| {
                ui.add(IconButton::new(Glyph::CollapseAll, "Collapse all"));
            })
            .bottom("Terminal", |ui| {
                ui.label("  $ cargo test");
            });
        if layout.right {
            wb = wb.right("Outline", |ui| {
                ui.label("  fn main");
            });
        }
        wb.show(ui, |ui| {
            ui.add_space(8.0);
            ui.label("   fn main() {}");
        });
    });
}

#[test]
fn snapshot_three_islands() {
    snapshot("workbench_three_islands", THREE);
}

#[test]
fn snapshot_four_islands() {
    snapshot("workbench_four_islands", FOUR);
}
