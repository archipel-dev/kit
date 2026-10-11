//! Window toolbar integration, command isolation and accessibility.
use archipel_kit::{
    Theme,
    components::{Button, WindowChrome, WindowToolbar},
    egui,
    testing::{self, Harness, Queryable, SnapshotOptions, SnapshotResults},
};

fn harness(theme: Theme, chrome: WindowChrome, gpu: bool) -> Harness<'static, usize> {
    let mut installed = false;
    let builder = Harness::builder().with_size(egui::vec2(480.0, 160.0));
    let builder = if gpu {
        builder.with_pixels_per_point(2.0).wgpu()
    } else {
        builder
    };
    let mut h = builder.build_state(
        move |ctx, count| {
            if !installed {
                archipel_kit::install(ctx, theme.clone());
                installed = true;
            }
            WindowToolbar::new("toolbar")
                .chrome(chrome)
                .show(ctx, |ui| {
                    ui.label("My application");
                    if ui.add(Button::secondary("Open")).clicked() {
                        *count += 1;
                    }
                });
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Content stays inside its window");
            });
        },
        0,
    );
    h.run();
    h
}

#[test]
fn viewport_preserves_native_controls_with_integrated_macos_content() {
    let viewport = WindowChrome::Integrated.viewport(egui::ViewportBuilder::default());
    assert_eq!(viewport.decorations, Some(true));
    if cfg!(target_os = "macos") {
        assert_eq!(viewport.fullsize_content_view, Some(true));
        assert_eq!(viewport.titlebar_shown, Some(false));
        assert_eq!(viewport.titlebar_buttons_shown, Some(true));
        assert_eq!(viewport.movable_by_window_background, Some(false));
    }
    assert_eq!(
        WindowChrome::Borderless
            .viewport(egui::ViewportBuilder::default())
            .decorations,
        Some(false)
    );
}

#[test]
fn toolbar_actions_do_not_trigger_window_drag_or_maximize() {
    let mut h = harness(Theme::dark(), WindowChrome::Borderless, false);
    h.get_by_label("Open").click();
    h.run_steps(1);
    assert_eq!(*h.state(), 1);
    assert!(
        !h.output()
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .any(|c| matches!(
                c,
                egui::ViewportCommand::StartDrag | egui::ViewportCommand::Maximized(_)
            ))
    );
    testing::audit(&mut h).assert_clean();
}

#[test]
fn window_controls_send_their_named_commands() {
    for (label, expected) in [
        ("Minimize window", 0),
        ("Maximize window", 1),
        ("Close window", 2),
    ] {
        let mut h = harness(Theme::light(), WindowChrome::Borderless, false);
        h.get_by_label(label).click();
        h.run_steps(1);
        assert!(
            h.output()
                .viewport_output
                .values()
                .flat_map(|v| &v.commands)
                .any(|c| match expected {
                    0 => matches!(c, egui::ViewportCommand::Minimized(true)),
                    1 => matches!(c, egui::ViewportCommand::Maximized(true)),
                    _ => matches!(c, egui::ViewportCommand::Close),
                })
        );
        assert_eq!(*h.state(), 0);
    }
}

#[test]
fn empty_toolbar_background_starts_native_drag() {
    let mut h = harness(Theme::dark(), WindowChrome::Borderless, false);
    let from = egui::pos2(300.0, 20.0);
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run_steps(1);
    let started_on_press = h
        .output()
        .viewport_output
        .values()
        .flat_map(|v| &v.commands)
        .any(|c| matches!(c, egui::ViewportCommand::StartDrag));
    h.hover_at(from + egui::vec2(30.0, 0.0));
    h.run_steps(1);
    assert!(
        started_on_press
            || h.output()
                .viewport_output
                .values()
                .flat_map(|v| &v.commands)
                .any(|c| matches!(c, egui::ViewportCommand::StartDrag))
    );
    assert_eq!(*h.state(), 0);
}

#[test]
fn snapshot() {
    let mut results = SnapshotResults::new();
    for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        let mut h = harness(theme, WindowChrome::Borderless, true);
        results.add(h.try_snapshot_options(
            format!("window_toolbar.{name}"),
            &SnapshotOptions::new().output_path("../../docs/components/img"),
        ));
    }
    results.unwrap();
}
