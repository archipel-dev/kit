//! Composable island headers, hover actions and keyboard accessibility.
use archipel_kit::{
    Glyph, Theme,
    components::{Button, IconButton, Panel, Separator},
    egui,
    testing::{self, Harness, Queryable, SnapshotOptions, SnapshotResults},
};

fn harness(theme: Theme, gpu: bool) -> Harness<'static, usize> {
    let mut installed = false;
    let builder = Harness::builder()
        .with_size(egui::vec2(460.0, 200.0))
        .with_max_steps(60);
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
            egui::CentralPanel::default().show(ctx, |ui| {
                Panel::new("Inspector")
                    .header(200.0, |ui| {
                        ui.label("Inspector");
                        ui.add(Separator::length(16.0));
                        ui.label("JSON");
                    })
                    .padded(true)
                    .header_separator(true)
                    .hover_actions(true)
                    .actions(|ui| {
                        if ui
                            .add(IconButton::new(Glyph::Refresh, "Reload inspector"))
                            .clicked()
                        {
                            *count += 1;
                        }
                        ui.add(Separator::length(16.0));
                        ui.add(IconButton::new(Glyph::Search, "Find in inspector"));
                    })
                    .show(ui, |ui| {
                        ui.add(Button::secondary("Body action"));
                        ui.label("Padded content");
                    });
            });
        },
        0,
    );
    h.run();
    h
}

#[test]
fn fading_actions_keep_keyboard_access_and_body_hover() {
    let mut h = harness(Theme::dark(), false);
    h.get_by_label("Reload inspector").focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert_eq!(*h.state(), 1);
    let body = h.get_by_label("Body action").rect().center();
    h.hover_at(body);
    h.run();
    testing::audit(&mut h).assert_clean();
    h.get_by_label("Find in inspector").click();
    h.run();
    assert_eq!(*h.state(), 1);
}

#[test]
fn snapshot() {
    let mut results = SnapshotResults::new();
    for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        let mut h = harness(theme, true);
        let body = h.get_by_label("Body action").rect().center();
        h.hover_at(body);
        h.run();
        results.add(h.try_snapshot_options(
            format!("island_header.{name}"),
            &SnapshotOptions::new().output_path("../../docs/components/img"),
        ));
    }
    results.unwrap();
}
