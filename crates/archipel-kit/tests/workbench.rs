//! Docked shell: state, visibility, resizing, narrow windows and both themes.
use archipel_kit::{
    Glyph, Theme,
    components::{Button, ListRow, Workbench, WorkbenchRegion, WorkbenchState},
    egui,
    testing::{self, Harness, Queryable, SnapshotOptions, SnapshotResults},
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
                .padded(WorkbenchRegion::Bottom, true);
            if right {
                shell = shell
                    .right("Details", |ui| {
                        ui.label("Name: example");
                    })
                    .padded(WorkbenchRegion::Right, true);
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
    let gap = archipel_kit::tokens::size::ISLAND_GAP;
    assert!(messages.left() >= gap + inset - 0.5, "{messages:?}");
    // Edge-to-edge regions leave the inset to their rows.
    assert!(rows.left() <= gap + 0.5, "{rows:?}");
    assert!(details.left() > rows.right());
}
