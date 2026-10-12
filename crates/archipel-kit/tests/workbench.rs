//! Docked shell: state, visibility, resizing, narrow windows and both themes.
use archipel_kit::{
    Glyph, Theme,
    components::{Button, ListRow, Workbench, WorkbenchRegion, WorkbenchState},
    egui,
    testing::{self, Harness, Queryable, Role, SnapshotOptions, SnapshotResults},
};

fn harness(theme: Theme, width: f32, right: bool, gpu: bool) -> Harness<'static, WorkbenchState> {
    let mut installed = false;
    let builder = Harness::builder().with_size(egui::vec2(width, 520.0));
    let builder = if gpu {
        builder.with_pixels_per_point(2.0).wgpu()
    } else {
        builder
    };
    let mut h = builder.build_state(
        move |ctx, state| {
            if !installed {
                archipel_kit::install(ctx, theme.clone());
                installed = true;
            }
            let mut shell = Workbench::new("test", state)
                .left("Browser", |ui| {
                    ListRow::new("Collections").icon(Glyph::Folder).show(ui);
                    ListRow::new("Environments").icon(Glyph::Folder).show(ui);
                })
                .bottom("Messages", |ui| {
                    ui.label("Ready");
                })
                .padded(WorkbenchRegion::Bottom, true)
                .icon(WorkbenchRegion::Left, Glyph::Folder)
                .icon(WorkbenchRegion::Bottom, Glyph::Terminal);
            if right {
                shell = shell
                    .right("Details", |ui| {
                        ui.label("Name: example");
                    })
                    .padded(WorkbenchRegion::Right, true)
                    .icon(WorkbenchRegion::Right, Glyph::Structure);
            }
            shell.show(ctx, |ui| {
                ui.add(Button::primary("Run request"));
            });
        },
        WorkbenchState::default(),
    );
    h.run();
    h
}

#[test]
fn headers_hide_maximize_and_restore_without_losing_layout() {
    let mut h = harness(Theme::dark(), 920.0, true, false);
    let before = h.state().left_ratio;
    h.get_by_label("Maximize Browser").click();
    h.run();
    assert_eq!(h.state().maximized, Some(WorkbenchRegion::Left));
    assert!(h.query_by_label("Run request").is_none());
    h.get_by_label("Restore Browser").click();
    h.run();
    assert_eq!(h.state().maximized, None);
    assert!(h.query_by_label("Run request").is_some());
    h.get_by_label("Hide Browser").click();
    h.run();
    assert!(h.state().left_hidden);
    assert!(h.query_by_label("Collections").is_none());
    h.state_mut().left_hidden = false;
    h.run();
    assert!(h.query_by_label("Collections").is_some());
    assert_eq!(h.state().left_ratio, before);
}

#[test]
fn small_windows_keep_the_center_and_preserve_preferred_state() {
    let mut h = harness(Theme::light(), 280.0, true, false);
    assert!(h.query_by_label("Run request").is_some());
    assert!(h.query_by_label("Collections").is_none());
    assert!(!h.state().left_hidden && !h.state().right_hidden);
    assert_eq!(h.state().left_ratio, WorkbenchState::default().left_ratio);
    h.state_mut().maximized = Some(WorkbenchRegion::Left);
    h.state_mut().left_hidden = true;
    h.run();
    assert!(h.query_by_label("Run request").is_some());
    assert_eq!(h.state().maximized, None);
}

#[test]
fn splitters_are_keyboard_reachable_and_adjust_state() {
    let mut h = harness(Theme::dark(), 920.0, true, false);
    h.get_by_label("Resize left tool window").focus();
    h.run();
    let before = h.state().left_ratio;
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert!(h.state().left_ratio > before);
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().left_hidden);
}

#[test]
fn layout_serializes_without_context_and_passes_both_theme_audits() {
    let state = WorkbenchState {
        left_hidden: true,
        maximized: Some(WorkbenchRegion::Right),
        ..Default::default()
    };
    let text = serde_json::to_string(&state).expect("serialize");
    let restored: WorkbenchState = serde_json::from_str(&text).expect("deserialize");
    assert!(restored.left_hidden);
    assert_eq!(restored.maximized, state.maximized);
    for theme in [Theme::dark(), Theme::light()] {
        let mut h = harness(theme, 920.0, true, false);
        testing::audit(&mut h).assert_clean();
    }
}

#[test]
fn snapshot() {
    let mut results = SnapshotResults::new();
    for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        for (count, right) in [(3, false), (4, true)] {
            let mut h = harness(theme.clone(), 920.0, right, true);
            results.add(h.try_snapshot_options(
                format!("workbench_{count}.{name}"),
                &SnapshotOptions::new().output_path("../../docs/components/img"),
            ));
        }
    }
    results.unwrap();
}

#[test]
fn padded_regions_inset_their_body_and_gaps_have_no_line() {
    let h = harness(Theme::dark(), 920.0, true, false);
    let details = h.get_by_label("Name: example").rect();
    let messages = h.get_by_label("Ready").rect();
    let rows = h.get_by_label("Collections").rect();
    // Padded bodies start at the content inset, like the island title above them.
    let inset = archipel_kit::tokens::size::CONTENT_INSET;
    // The islands start after the left stripe and the island gap: 42 pt, as in Android Studio.
    let islands = archipel_kit::tokens::stripe::WIDTH + archipel_kit::tokens::size::ISLAND_GAP;
    assert!(messages.left() >= islands + inset - 0.5, "{messages:?}");
    // Edge-to-edge regions leave the inset to their rows.
    assert!(rows.left() <= islands + 0.5, "{rows:?}");
    assert!(details.left() > rows.right());
}

#[test]
fn stripe_buttons_open_and_close_tool_windows() {
    let mut h = harness(Theme::dark(), 920.0, true, false);
    // As in Android Studio: the left stripe holds the left and bottom tool windows.
    h.get_by_role_and_label(Role::Button, "Messages").click();
    h.run();
    assert!(h.state().bottom_hidden);
    assert!(h.query_by_label("Ready").is_none());
    h.get_by_role_and_label(Role::Button, "Messages").click();
    h.run();
    assert!(!h.state().bottom_hidden);
    assert!(h.query_by_label("Ready").is_some());
    // A hidden tool window comes back from its stripe button, not only from the host.
    h.get_by_label("Hide Details").click();
    h.run();
    assert!(h.state().right_hidden);
    h.get_by_role_and_label(Role::Button, "Details").click();
    h.run();
    assert!(!h.state().right_hidden);
    assert!(h.query_by_label("Name: example").is_some());
}

/// A whole IDE-like window built only from kit components, laid out like Android Studio's
/// Islands UI: toolbar with project widget and project color, stripes, a project tree and an
/// editor with tabs.
fn ide(theme: Theme, gpu: bool) -> Harness<'static, WorkbenchState> {
    use archipel_kit::components::{
        Avatar, Breadcrumbs, IconButton, StatusBarItem, Tab, TabBar, ToolbarButton, WindowChrome,
    };
    use archipel_kit::parts::Slot;
    let mut installed = false;
    let builder = Harness::builder().with_size(egui::vec2(1000.0, 560.0));
    let builder = if gpu {
        builder.with_pixels_per_point(2.0).wgpu()
    } else {
        builder
    };
    let tabs = [
        Tab::new("lib.rs")
            .icon(archipel_kit::Icon::for_file("lib.rs"))
            .closable(true),
        Tab::new("theme.rs")
            .icon(archipel_kit::Icon::for_file("theme.rs"))
            .closable(true),
        Tab::new("Cargo.toml")
            .icon(archipel_kit::Icon::for_file("Cargo.toml"))
            .closable(true),
    ];
    let mut h = builder.build_state(
        move |ctx, state| {
            if !installed {
                archipel_kit::install(ctx, theme.clone());
                installed = true;
            }
            let project = egui::Color32::from_rgb(0x4C, 0xA0, 0x74);
            Workbench::new("ide", state)
                .project_color(project)
                .toolbar(WindowChrome::Native, |ui| {
                    ui.add(IconButton::new(Glyph::Menu, "Main menu").large(true));
                    let _ = ToolbarButton::new("archipel")
                        .leading(Slot::ui_width(20.0, |ui| {
                            ui.add(Avatar::new("archipel").size(20.0));
                        }))
                        .dropdown(true)
                        .show(ui);
                    let _ = ToolbarButton::new("main")
                        .icon(Glyph::Branch)
                        .dropdown(true)
                        .show(ui);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(IconButton::new(Glyph::Settings, "Settings").large(true));
                        ui.add(IconButton::new(Glyph::Search, "Search everywhere").large(true));
                        ui.add(IconButton::new(Glyph::Run, "Run").large(true));
                    });
                })
                .left("Project", |ui| {
                    ListRow::new("archipel")
                        .icon(Glyph::Folder)
                        .strong(true)
                        .expanded(Some(true))
                        .show(ui);
                    ListRow::new("src")
                        .icon(Glyph::Folder)
                        .depth(1)
                        .expanded(Some(true))
                        .show(ui);
                    ListRow::new("lib.rs")
                        .icon(archipel_kit::Icon::for_file("lib.rs"))
                        .depth(2)
                        .selected(true)
                        .show(ui);
                    ListRow::new("theme.rs")
                        .icon(archipel_kit::Icon::for_file("theme.rs"))
                        .depth(2)
                        .show(ui);
                    ListRow::new("Cargo.toml")
                        .icon(archipel_kit::Icon::for_file("Cargo.toml"))
                        .depth(1)
                        .show(ui);
                })
                .icon(WorkbenchRegion::Left, Glyph::Folder)
                .bottom("Terminal", |ui| {
                    ui.label("$ cargo test");
                })
                .padded(WorkbenchRegion::Bottom, true)
                .icon(WorkbenchRegion::Bottom, Glyph::Terminal)
                .center_padded(false)
                .status(|ui| {
                    let _ = Breadcrumbs::new()
                        .crumb("archipel", Some(Glyph::Folder.into()))
                        .crumb("lib.rs", Some(archipel_kit::Icon::for_file("lib.rs")))
                        .show(ui);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(StatusBarItem::new("4 spaces"));
                        ui.add(StatusBarItem::new("UTF-8"));
                        ui.add(StatusBarItem::new("LF"));
                        ui.add(StatusBarItem::new("23:1"));
                    });
                })
                .show(ctx, |ui| {
                    TabBar::new(&tabs, 0).show(ui);
                    ui.horizontal(|ui| {
                        ui.add_space(archipel_kit::tokens::size::CONTENT_INSET);
                        ui.label("pub fn install(ctx: &egui::Context, theme: Theme) { … }");
                    });
                });
        },
        WorkbenchState {
            top_ratio: 0.72,
            ..WorkbenchState::default()
        },
    );
    h.run();
    h
}

#[test]
fn ide_window_passes_both_theme_audits_and_snapshot() {
    for theme in [Theme::dark(), Theme::light()] {
        let mut h = ide(theme, false);
        testing::audit(&mut h).assert_clean();
    }
    let mut results = SnapshotResults::new();
    for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        let mut h = ide(theme, true);
        results.add(h.try_snapshot_options(
            format!("workbench_ide.{name}"),
            &SnapshotOptions::new().output_path("../../docs/components/img"),
        ));
    }
    results.unwrap();
}

#[test]
fn stripe_tooltips_show_at_once() {
    let count = |h: &Harness<'_, WorkbenchState>| {
        h.output()
            .shapes
            .iter()
            .filter(|c| match &c.shape {
                egui::Shape::Text(t) => t.galley.text() == "Messages",
                _ => false,
            })
            .count()
    };
    let mut h = harness(Theme::dark(), 920.0, true, false);
    let before = count(&h);
    let button = h
        .get_by_role_and_label(Role::Button, "Messages")
        .rect()
        .center();
    h.hover_at(button);
    // Two frames, far less than the usual tooltip delay: Android Studio's stripes name
    // their buttons at once.
    h.step();
    h.step();
    assert_eq!(
        count(&h),
        before + 1,
        "the tooltip shows next to the panel title"
    );
}

#[test]
fn the_focused_tool_window_has_an_accent_stripe_button() {
    let accent = Theme::dark().colors.accent;
    let filled = |h: &Harness<'_, WorkbenchState>, label: &str| {
        let rect = h.get_by_role_and_label(Role::Button, label).rect();
        h.output().shapes.iter().any(|c| match &c.shape {
            egui::Shape::Rect(r) => r.fill == accent && r.rect.center() == rect.center(),
            _ => false,
        })
    };
    let mut h = harness(Theme::dark(), 920.0, true, false);
    assert!(!filled(&h, "Browser"));
    // Focus inside the Browser tool window makes it the active one, as in Android Studio.
    h.get_by_label("Collections").focus();
    h.run();
    assert!(filled(&h, "Browser"));
    assert!(!filled(&h, "Messages"));
}

#[test]
fn show_with_fills_every_place_from_one_state() {
    use archipel_kit::components::{StatusBarItem, WorkbenchPlace};
    #[derive(Default)]
    struct App {
        layout: WorkbenchState,
        drawn: Vec<WorkbenchPlace>,
        clicks: u32,
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 520.0))
        .build_state(
            |ctx, app: &mut App| {
                app.drawn.clear();
                let App {
                    layout,
                    drawn,
                    clicks,
                } = app;
                let mut state = (drawn, clicks);
                Workbench::new("with", layout)
                    .tool(WorkbenchRegion::Left, "Collections", Glyph::Folder)
                    .tool(WorkbenchRegion::Right, "Response", Glyph::Structure)
                    .with_status()
                    .show_with(ctx, &mut state, |place, ui, (drawn, clicks)| {
                        drawn.push(place);
                        match place {
                            WorkbenchPlace::Center => {
                                if ui.add(Button::primary("Send")).clicked() {
                                    **clicks += 1;
                                }
                            }
                            WorkbenchPlace::Status => {
                                ui.add(StatusBarItem::new(format!("{clicks} sent")));
                            }
                            WorkbenchPlace::ToolActions(WorkbenchRegion::Left) => {
                                ui.add(archipel_kit::components::IconButton::new(
                                    Glyph::Plus,
                                    "New request",
                                ));
                            }
                            _ => {
                                ui.label(format!("{clicks} in tool"));
                            }
                        }
                    });
            },
            App::default(),
        );
    h.run();
    for place in [
        WorkbenchPlace::Center,
        WorkbenchPlace::Status,
        WorkbenchPlace::Tool(WorkbenchRegion::Left),
        WorkbenchPlace::Tool(WorkbenchRegion::Right),
    ] {
        assert!(h.state().drawn.contains(&place), "{place:?} was not drawn");
    }
    assert!(h.query_by_label("Collections").is_some());
    assert!(
        h.query_by_label("New request").is_some(),
        "tool window actions"
    );
    h.get_by_label("Send").click();
    h.run();
    h.run();
    assert_eq!(h.state().clicks, 1);
    assert!(h.query_by_label("1 sent").is_some());
}

#[test]
fn tool_tabs_select_and_remember_the_content() {
    use archipel_kit::components::WorkbenchPlace;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 520.0))
        .build_state(
            |ctx, layout: &mut WorkbenchState| {
                let tab = layout.tool_tab(WorkbenchRegion::Right);
                Workbench::new("tabs", layout)
                    .tool(WorkbenchRegion::Right, "Response", Glyph::Structure)
                    .tool_tabs(
                        WorkbenchRegion::Right,
                        vec!["Preview".into(), "Raw".into(), "Headers".into()],
                    )
                    .show_with(ctx, &mut (), |place, ui, _| {
                        if place == WorkbenchPlace::Tool(WorkbenchRegion::Right) {
                            ui.label(["Preview body", "Raw body", "Header list"][tab]);
                        }
                    });
            },
            WorkbenchState {
                center_ratio: 0.45,
                ..WorkbenchState::default()
            },
        );
    h.run();
    assert!(h.query_by_label("Preview body").is_some());
    h.get_by_label("Raw").click();
    h.run();
    h.run();
    assert_eq!(h.state().tool_tab(WorkbenchRegion::Right), 1);
    assert!(h.query_by_label("Raw body").is_some());
}

/// Whether a run of `show_inside` paints a rectangle in the frame color over the whole area.
fn paints_frame(frame_background: bool) -> bool {
    let ctx = egui::Context::default();
    archipel_kit::install(&ctx, Theme::dark());
    let mut state = WorkbenchState::default();
    let mut area = egui::Rect::NOTHING;
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            area = ui.available_rect_before_wrap();
            Workbench::new("embedded", &mut state)
                .left("Browser", |ui| {
                    ui.label("Collections");
                })
                .frame_background(frame_background)
                .show_inside(ui, |ui| {
                    ui.label("Center");
                });
        });
    });
    let frame = Theme::colors(&ctx).frame;
    output.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(rect) => rect.fill == frame && rect.rect == area,
        _ => false,
    })
}

#[test]
fn an_embedded_workbench_can_leave_the_host_frame_and_its_glow_visible() {
    assert!(
        paints_frame(true),
        "by default the workbench paints its frame"
    );
    assert!(
        !paints_frame(false),
        "with frame_background(false) the host's frame and glow stay one surface"
    );
}
