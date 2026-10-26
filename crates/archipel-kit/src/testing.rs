//! Test helpers built on [egui_kittest]: the same tools the kit's own tests use, for your
//! panels and plugins.
//!
//! Tests find widgets the way a user does, by role and visible label (the AccessKit tree),
//! not by coordinates, and snapshot tests render the real pixels through wgpu in both
//! themes:
//!
//! ```no_run
//! use archipel_kit::ui::Button;
//! use archipel_kit::testing::{self, Queryable};
//! use archipel_kit::Theme;
//!
//! let mut clicks = 0;
//! let mut harness = testing::harness(Theme::dark(), |ui| {
//!     if ui.add(Button::primary("Run")).clicked() {
//!         clicks += 1;
//!     }
//! });
//! harness.get_by_label("Run").click();
//! harness.run();
//! drop(harness);
//! assert_eq!(clicks, 1);
//!
//! // Writes/compares `tests/snapshots/run_button.dark.png` and `.light.png`.
//! testing::snapshot_themes("run_button", "tests/snapshots", |ui| {
//!     ui.add(Button::primary("Run"));
//! });
//! ```
//!
//! Snapshots are compared with a small tolerance. Run `UPDATE_SNAPSHOTS=1 cargo test` to
//! accept new images. Rendering needs a GPU or a software Vulkan driver (on Linux CI:
//! `mesa-vulkan-drivers`).

use crate::theme::Theme;
use std::path::Path;

mod audit;
mod conformance;

pub use audit::{Audit, Finding, MIN_TARGET, Problem, audit};
pub use conformance::{Conformance, conformance};

pub use egui::accesskit::Role;
pub use egui_kittest::kittest::{By, NodeT, Queryable};
pub use egui_kittest::{Harness, HarnessBuilder, Node, SnapshotOptions, SnapshotResults};

/// Width of the canvas content is laid out in before the harness shrinks to fit it.
pub const DEFAULT_WIDTH: f32 = 360.0;

/// A harness that renders `app` on an island of `theme`, sized to fit its content.
pub fn harness<'a>(theme: Theme, mut app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    harness_state(theme, (), move |ui, _| app(ui))
}

/// Like [`harness`], with state the test can inspect afterwards through
/// [`Harness::state`].
pub fn harness_state<'a, State>(
    theme: Theme,
    state: State,
    app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
) -> Harness<'a, State> {
    // Interaction tests run at 1×: kittest clicks at AccessKit bounds, which are in
    // physical pixels.
    build(theme, state, app, 1.0)
}

/// Like [`harness_state`], in a window of `size` from the first frame on, instead of one
/// fitted to the content. Use it for screens and dialogs: a window resized after the first
/// frame lays them out twice, and clicks can land on where things were.
pub fn harness_state_sized<'a, State>(
    theme: Theme,
    size: egui::Vec2,
    state: State,
    app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
) -> Harness<'a, State> {
    build_with(theme, state, app, 1.0, Some(size))
}

fn build<'a, State>(
    theme: Theme,
    state: State,
    app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
    pixels_per_point: f32,
) -> Harness<'a, State> {
    build_with(theme, state, app, pixels_per_point, None)
}

fn build_with<'a, State>(
    theme: Theme,
    state: State,
    mut app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
    pixels_per_point: f32,
    size: Option<egui::Vec2>,
) -> Harness<'a, State> {
    let egui_theme = if theme.dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let mut pending = Some(theme);
    let mut builder = builder();
    if let Some(size) = size {
        builder = builder.with_size(size);
    }
    let mut harness = builder
        .with_theme(egui_theme)
        .with_pixels_per_point(pixels_per_point)
        .build_ui_state(
            move |ui, state| {
                // Fonts installed during a frame take effect on the next one, so the first
                // frame only installs the kit.
                if let Some(theme) = pending.take() {
                    crate::install(ui.ctx(), theme);
                    ui.ctx().request_repaint();
                    return;
                }
                island(ui, |ui| app(ui, state));
            },
            state,
        );
    // Animated widgets (spinners) never settle, so wait with a step limit.
    harness.run_ok();
    if size.is_none() {
        harness.fit_contents();
    }
    harness
}

/// The builder [`harness`] starts from: fixed width, 60 fps steps and the wgpu renderer.
pub fn builder<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder()
        .with_size(egui::vec2(DEFAULT_WIDTH, 600.0))
        .with_step_dt(1.0 / 60.0)
        .with_max_steps(30)
        .wgpu()
}

/// Scale of snapshot images: 2× stays crisp in docs on high-DPI screens.
pub const SNAPSHOT_SCALE: f32 = 2.0;

/// Renders `app` in dark and light and compares each with `{dir}/{name}.{dark,light}.png`.
///
/// # Panics
/// If an image differs from its snapshot (or none exists and `UPDATE_SNAPSHOTS` is unset).
#[track_caller]
pub fn snapshot_themes(name: &str, dir: impl AsRef<Path>, mut app: impl FnMut(&mut egui::Ui)) {
    let mut results = SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut harness = build(theme, (), |ui, _| app(ui), SNAPSHOT_SCALE);
        results.add(harness.try_snapshot_options(
            format!("{name}.{suffix}"),
            &SnapshotOptions::new().output_path(dir.as_ref()),
        ));
    }
    results.unwrap();
}

/// Like [`snapshot_themes`], with `act` run on each harness before the image is taken: press
/// keys, focus a widget, type, for states a still image can't be built from (an open find
/// bar, a selection). The canvas re-fits its content after `act`.
///
/// # Panics
/// If an image differs from its snapshot (or none exists and `UPDATE_SNAPSHOTS` is unset).
#[track_caller]
pub fn snapshot_themes_after(
    name: &str,
    dir: impl AsRef<Path>,
    mut app: impl FnMut(&mut egui::Ui),
    mut act: impl FnMut(&mut Harness<'_, ()>),
) {
    let mut results = SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut harness = build(theme, (), |ui, _| app(ui), SNAPSHOT_SCALE);
        act(&mut harness);
        harness.run_ok();
        harness.fit_contents();
        results.add(harness.try_snapshot_options(
            format!("{name}.{suffix}"),
            &SnapshotOptions::new().output_path(dir.as_ref()),
        ));
    }
    results.unwrap();
}

/// Like [`snapshot_themes`] on a canvas of fixed `size` (in points) instead of one fitted
/// to the content: for overlays such as dialogs, which float above the island.
///
/// # Panics
/// If an image differs from its snapshot (or none exists and `UPDATE_SNAPSHOTS` is unset).
#[track_caller]
pub fn snapshot_themes_sized(
    name: &str,
    dir: impl AsRef<Path>,
    size: egui::Vec2,
    mut app: impl FnMut(&mut egui::Ui),
) {
    let mut results = SnapshotResults::new();
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let draw = |ui: &mut egui::Ui, _: &mut ()| {
            // The whole canvas is the window's frame color, like the area around islands.
            let colors = Theme::colors(ui.ctx());
            ui.ctx()
                .layer_painter(egui::LayerId::background())
                .rect_filled(ui.ctx().content_rect(), 0.0, colors.frame);
            app(ui);
        };
        let mut harness = build(theme, (), draw, SNAPSHOT_SCALE);
        harness.set_size(size);
        harness.run_ok();
        // Centered overlays learn their size in the first frame and move in the second.
        harness.run_steps(3);
        results.add(harness.try_snapshot_options(
            format!("{name}.{suffix}"),
            &SnapshotOptions::new().output_path(dir.as_ref()),
        ));
    }
    results.unwrap();
}

/// Draws `add` on a rounded island with content padding, like a panel in an app.
fn island(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    crate::parts::Surface::Island.frame(ui.ctx()).show(ui, add);
}

/// Accessible state of a node, for assertions: what a screen reader would announce.
pub trait NodeExt {
    /// Checkboxes, radio buttons, toggles, selected tabs and rows: whether it is on.
    fn is_checked(&self) -> bool;
    /// Whether the widget is disabled.
    fn is_disabled(&self) -> bool;
    /// The accessible name.
    fn accessible_label(&self) -> Option<String>;
    /// Numeric value (progress bars report percent).
    fn numeric_value(&self) -> Option<f64>;
    /// Role (button, checkbox, text input, ...).
    fn role(&self) -> Role;
}

impl NodeExt for Node<'_> {
    fn is_checked(&self) -> bool {
        self.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    }

    fn is_disabled(&self) -> bool {
        self.accesskit_node().is_disabled()
    }

    fn accessible_label(&self) -> Option<String> {
        self.accesskit_node().label()
    }

    fn numeric_value(&self) -> Option<f64> {
        self.accesskit_node().numeric_value()
    }

    fn role(&self) -> Role {
        self.accesskit_node().role()
    }
}
