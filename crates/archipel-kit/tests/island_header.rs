//! Composable island headers, hover actions and keyboard accessibility.
use archipel_kit::{
    Glyph, Theme,
    action_system::ActionButton,
    egui,
    testing::{self, Harness, Queryable, SnapshotOptions, SnapshotResults},
    ui::{Button, SeparatorComponent},
    wm::SimpleToolWindowPanel,
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
                SimpleToolWindowPanel::new("Inspector")
                    .header(200.0, |ui| {
                        ui.label("Inspector");
                        ui.add(SeparatorComponent::group());
                        ui.label("JSON");
                    })
                    .padded(true)
                    .header_separator(true)
                    .hover_actions(true)
                    .actions(|ui| {
                        if ui
                            .add(ActionButton::new(Glyph::Refresh, "Reload inspector"))
                            .clicked()
                        {
                            *count += 1;
                        }
                        ui.add(SeparatorComponent::group());
                        ui.add(ActionButton::new(Glyph::Search, "Find in inspector"));
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
        // AccessKit bounds are physical pixels; the pointer is in points (2× here).
        let body = h.get_by_label("Body action").rect().center() / 2.0;
        h.hover_at(body);
        h.run();
        results.add(h.try_snapshot_options(
            format!("island_header.{name}"),
            &SnapshotOptions::new().output_path("../../docs/components/img"),
        ));
    }
    results.unwrap();
}

/// Whether the last frame painted `text`.
fn painted(h: &Harness<'_, usize>, text: &str) -> bool {
    h.output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Text(t) => t.galley.text() == text,
            _ => false,
        })
}

/// A narrow tool window, as in Android Studio with the Project island dragged thin:
/// its title and its actions do not fit side by side.
fn narrow(theme: Theme) -> Harness<'static, usize> {
    let mut installed = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(170.0, 160.0))
        .with_step_dt(0.05)
        .build_state(
            move |ctx, _| {
                if !installed {
                    archipel_kit::install(ctx, theme.clone());
                    installed = true;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    SimpleToolWindowPanel::new("Project")
                        .hover_actions(true)
                        .on_hide(|| {})
                        .actions(|ui| {
                            ui.add(ActionButton::new(Glyph::More, "Options"));
                            ui.add(ActionButton::new(Glyph::CollapseAll, "Collapse all"));
                            ui.add(ActionButton::new(Glyph::Search, "Find"));
                        })
                        .show(ui, |ui| {
                            ui.label("src/");
                        });
                });
            },
            0,
        );
    h.run();
    h
}

#[test]
fn shown_actions_take_a_narrow_header_and_hide_after_a_delay() {
    let mut h = narrow(Theme::dark());
    // At rest the actions are hidden and leave the whole header to the title.
    assert!(painted(&h, "Project"));
    let body = h.get_by_label("src/").rect().center();
    h.hover_at(body);
    h.step();
    h.step();
    // They appear at once and, not fitting beside it, push the title out whole.
    assert!(!painted(&h, "Project"));
    h.hover_at(egui::pos2(-10.0, -10.0));
    h.step();
    h.step();
    // 0.1 s after leaving: still shown (the hide delay is 0.2 s).
    assert!(!painted(&h, "Project"));
    for _ in 0..8 {
        h.step();
    }
    assert!(painted(&h, "Project"));
    // Keyboard focus inside the island reveals them too.
    h.get_by_label("Find").focus();
    h.step();
    h.step();
    assert!(!painted(&h, "Project"));
}

#[test]
fn content_tabs_select_and_turn_blue_while_the_tool_window_is_active() {
    let colors = Theme::dark().colors;
    let selected_fill = |h: &Harness<'_, usize>| {
        let rect = h.get_by_label("File").rect();
        h.output().shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.rect.center() == rect.center() => Some(r.fill),
            _ => None,
        })
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(520.0, 160.0))
        .build_state(
            |ctx, selected| {
                archipel_kit::install(ctx, Theme::dark());
                egui::CentralPanel::default().show(ctx, |ui| {
                    let tabs = ["File".to_owned(), "Project Errors".to_owned()];
                    SimpleToolWindowPanel::new("Problems")
                        .tabs(&tabs, selected)
                        .show(ui, |ui| {
                            ui.add(Button::secondary("Inside"));
                        });
                });
            },
            0usize,
        );
    h.run();
    // Inactive tool window: the selected tab is outlined gray, as in Android Studio.
    assert_eq!(selected_fill(&h), Some(colors.frame));
    h.get_by_label("Inside").focus();
    h.run();
    assert_eq!(selected_fill(&h), Some(colors.tab_selected));
    h.get_by_label("Project Errors").click();
    h.run();
    assert_eq!(*h.state(), 1);
}

#[test]
fn the_title_switches_views_like_project_in_android_studio() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(420.0, 360.0))
        .build_state(
            |ctx, selected| {
                archipel_kit::install(ctx, Theme::dark());
                egui::CentralPanel::default().show(ctx, |ui| {
                    let views = [
                        "Project".to_owned(),
                        "Packages".to_owned(),
                        "Tests".to_owned(),
                    ];
                    SimpleToolWindowPanel::new("Project")
                        .views(&views, selected)
                        .show(ui, |ui| {
                            ui.label("tree");
                        });
                });
            },
            0usize,
        );
    h.run();
    assert!(h.query_by_label("Packages").is_none());
    h.get_by_label("Project").click();
    h.run();
    h.get_by_label("Packages").click();
    h.run();
    assert_eq!(*h.state(), 1);
    // The title now reads the chosen view and the list is closed.
    assert!(h.query_by_label("Packages").is_some());
    assert!(h.query_by_label("Tests").is_none());
    // Enter on the focused title opens it again.
    h.get_by_label("Packages").focus();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.query_by_label("Tests").is_some());
}

#[test]
fn tabs_that_do_not_fit_go_behind_a_chevron() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(320.0, 200.0))
        .build_state(
            |ctx, selected| {
                archipel_kit::install(ctx, Theme::dark());
                egui::CentralPanel::default().show(ctx, |ui| {
                    let tabs: Vec<String> =
                        ["Params", "Body", "Auth", "Headers", "Tests", "Settings"]
                            .map(String::from)
                            .to_vec();
                    SimpleToolWindowPanel::new("Request")
                        .tabs(&tabs, selected)
                        .actions(|ui| {
                            ui.add(archipel_kit::action_system::ActionButton::new(
                                archipel_kit::Glyph::Code,
                                "Generate code",
                            ));
                        })
                        .show(ui, |ui| {
                            ui.label("body");
                        });
                });
            },
            0usize,
        );
    h.run();
    assert!(
        h.query_by_label("Settings").is_none(),
        "Settings does not fit"
    );
    let audit = testing::audit(&mut h);
    let overlaps: Vec<_> = audit
        .findings
        .iter()
        .filter(|f| f.problem == testing::Problem::Overlap)
        .collect();
    assert!(overlaps.is_empty(), "{overlaps:?}");
    h.get_by_label("More tabs").click();
    h.run();
    h.get_by_label("Settings").click();
    h.run();
    h.run();
    assert_eq!(*h.state(), 5);
    // The selected tab is shown in the header now.
    assert!(h.query_by_label("Settings").is_some());
}
