//! Snapshots of the window family's new looks: tab actions and pinned tabs, a tab being
//! dragged, the splitter variants, the status bar container and the tool window header.

mod common;

use archipel_kit::action_system::{
    ActionGroup, AnAction, AnActionEvent, DataContext, DefaultActionGroup, Presentation,
};
use archipel_kit::egui::{self, Event, Modifiers, PointerButton, pos2};
use archipel_kit::openapi::ui::{DividerStyle, Splitter};
use archipel_kit::testing::{self, Harness, Queryable, SnapshotOptions, SnapshotResults};
use archipel_kit::ui::tabs::{JBTabs, TabInfo};
use archipel_kit::wm::{
    IconPresentation, MultipleTextValuesPresentation, SimpleToolWindowPanel, StatusBar,
    StatusBarWidgetItem, TextPresentation, ToolWindowAnchor, ToolWindowPane, ToolWindowPaneState,
    ToolWindowPlace,
};
use archipel_kit::{Glyph, Icon, Theme, Tone};
use std::rc::Rc;

struct Command(&'static str, Glyph);

impl AnAction for Command {
    fn id(&self) -> &str {
        self.0
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.0).icon(self.1)
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {}
}

/// Draws `add` at the top of the test island, flush with its edges, where an editor's tabs
/// are.
fn at_island_top(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let inset = archipel_kit::tokens::size::CONTENT_INSET;
    let header = archipel_kit::tokens::size::HEADER_HEIGHT;
    let rect = egui::Rect::from_min_size(
        ui.max_rect().min - egui::vec2(inset, inset),
        egui::vec2(width + 2.0 * inset, header),
    );
    let mut top = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    add(&mut top);
    ui.allocate_space(egui::vec2(width, header - inset));
}

fn tabs_with_markers() -> Vec<TabInfo> {
    let mut actions = DefaultActionGroup::new("Tab actions");
    actions.add(Command("Reload", Glyph::Refresh));
    vec![
        TabInfo::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .closable(true)
            .pinned(true),
        TabInfo::new("Cargo.toml")
            .icon(Icon::for_file("Cargo.toml"))
            .closable(true)
            .modified(true)
            .actions(Rc::new(actions)),
        TabInfo::new(
            "a_very_long_file_name_that_goes_on_and_on_and_on_and_on_and_on_and_on_and_on.rs",
        )
        .icon(Icon::for_file("a.rs"))
        .closable(true),
        TabInfo::new("README.md")
            .icon(Icon::for_file("README.md"))
            .enabled(false),
    ]
}

#[test]
fn jb_tabs_pinned_actions_snapshot() {
    let tabs = tabs_with_markers();
    testing::snapshot_themes("jb_tabs_pinned_actions", common::IMG, |ui| {
        at_island_top(ui, 760.0, |ui| {
            JBTabs::new(&tabs, 1).show(ui);
        });
    });
}

#[test]
fn jb_tabs_dragging_snapshot() {
    let tabs: Vec<TabInfo> = ["main.rs", "lib.rs", "theme.rs", "tokens.rs"]
        .into_iter()
        .map(|name| TabInfo::new(name).icon(Icon::for_file(name)).closable(true))
        .collect();
    testing::snapshot_themes_after(
        "jb_tabs_dragging",
        common::IMG,
        |ui| {
            at_island_top(ui, 420.0, |ui| {
                JBTabs::new(&tabs, 0).show(ui);
            });
        },
        // main.rs is dragged past lib.rs: it follows the pointer, lib.rs moved aside.
        |harness| {
            harness.run();
            let from = harness.get_by_label("main.rs").rect().center() / testing::SNAPSHOT_SCALE;
            let to = pos2(from.x + 120.0, from.y);
            harness.hover_at(from);
            harness.step();
            harness.event(Event::PointerButton {
                pos: from,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            });
            harness.step();
            for step in 1..=8 {
                let t = step as f32 / 8.0;
                harness.hover_at(pos2(from.x + (to.x - from.x) * t, from.y));
                harness.step();
            }
        },
    );
}

#[test]
fn splitter_variants_snapshot() {
    testing::snapshot_themes("splitter_variants", common::IMG, |ui| {
        for (index, (name, style)) in [
            ("Gap", DividerStyle::Gap),
            ("Classic", DividerStyle::Classic),
            ("OnePixel", DividerStyle::OnePixel),
        ]
        .into_iter()
        .enumerate()
        {
            let mut ratio = 0.5;
            ui.allocate_ui(egui::vec2(340.0, 56.0), |ui| {
                Splitter::horizontal(("variants", index), &mut ratio)
                    .divider_style(style)
                    .min_size(20.0)
                    .show(ui, |ui| ui.label(name), |ui| ui.label("second pane"));
            });
        }
    });
}

fn gpu_harness<S: 'static>(
    theme: Theme,
    size: egui::Vec2,
    state: S,
    mut app: impl FnMut(&egui::Context, &mut S) + 'static,
) -> Harness<'static, S> {
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(2.0)
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_state(
            move |ctx, state| {
                if !installed {
                    archipel_kit::install(ctx, theme.clone());
                    installed = true;
                }
                app(ctx, state);
            },
            state,
        );
    harness.run();
    harness
}

fn status_widgets(encoding: usize) -> Vec<(bool, StatusBarWidgetItem)> {
    vec![
        (
            true,
            StatusBarWidgetItem::new("vcs", TextPresentation::new("main")),
        ),
        (
            false,
            StatusBarWidgetItem::new(
                "position",
                TextPresentation::new("12:8")
                    .max_value("9999:99")
                    .alignment(1.0),
            ),
        ),
        (
            false,
            StatusBarWidgetItem::new(
                "encoding",
                MultipleTextValuesPresentation::new(
                    vec!["UTF-8".into(), "UTF-16".into(), "ISO-8859-1".into()],
                    encoding,
                ),
            ),
        ),
        (
            false,
            StatusBarWidgetItem::new("problems", TextPresentation::new("2 errors"))
                .tone(Tone::Error),
        ),
        (
            false,
            StatusBarWidgetItem::new("bell", IconPresentation::new(Glyph::Bell, "Notifications")),
        ),
    ]
}

fn status_bar_harness(theme: Theme) -> Harness<'static, ()> {
    gpu_harness(theme, egui::vec2(520.0, 34.0), (), |ctx, _| {
        egui::CentralPanel::default().show(ctx, |_| {});
        egui::TopBottomPanel::bottom("status")
            .exact_height(archipel_kit::tokens::size::STATUS_BAR_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(Theme::colors(ctx).frame)
                    .inner_margin(egui::Margin::symmetric(8, 0)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                let mut bar = StatusBar::new("bar").info("Indexing finished");
                for (left, item) in status_widgets(0) {
                    bar = if left {
                        bar.left_widget(item)
                    } else {
                        bar.widget(item)
                    };
                }
                bar.show(ui);
            });
    })
}

#[test]
fn status_bar_snapshots() {
    let mut results = SnapshotResults::new();
    for (suffix, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        // Rest, with the pointer on the encoding widget (the hover fill, arc 4).
        let mut harness = status_bar_harness(theme.clone());
        let pos = harness.get_by_label("UTF-8").rect().center() / 2.0;
        harness.hover_at(pos);
        harness.run();
        results.add(harness.try_snapshot_options(
            format!("status_bar.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
        // The press opens the list of values above the widget (the canvas grows to hold it).
        harness.set_size(egui::vec2(520.0, 130.0));
        harness.run();
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        results.add(harness.try_snapshot_options(
            format!("status_bar_values.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

struct HeaderState {
    title: DefaultActionGroup,
    options: DefaultActionGroup,
    context: DataContext,
    selected: usize,
}

fn header_state() -> HeaderState {
    let mut title = DefaultActionGroup::new("Title");
    title.add(Command("Rerun", Glyph::Refresh));
    title.add(Command("Filter", Glyph::Filter));
    let mut options = DefaultActionGroup::popup("Options");
    options.add(Command("Group by File", Glyph::Folder));
    HeaderState {
        title,
        options,
        context: DataContext::empty(),
        selected: 0,
    }
}

#[test]
fn tool_window_header_snapshot() {
    let tabs = ["File".to_owned(), "Project Errors".to_owned()];
    let closable = [false, true];
    let mut results = SnapshotResults::new();
    for (suffix, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        let tabs = tabs.clone();
        let mut harness = gpu_harness(
            theme,
            egui::vec2(420.0, 170.0),
            header_state(),
            move |ctx, state| {
                let HeaderState {
                    title,
                    options,
                    context,
                    selected,
                } = state;
                egui::CentralPanel::default().show(ctx, |ui| {
                    SimpleToolWindowPanel::new("Problems")
                        .tabs(&tabs, selected)
                        .tab_closable(&closable)
                        .title_actions(title as &dyn ActionGroup)
                        .popup_group(options as &dyn ActionGroup)
                        .data_context(context)
                        .on_hide(|| {})
                        .show(ui, |ui| {
                            ui.label("No problems found");
                        });
                });
            },
        );
        results.add(harness.try_snapshot_options(
            format!("tool_window_header.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

#[test]
fn stripe_drag_snapshot() {
    let mut results = SnapshotResults::new();
    for (suffix, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
        let mut harness = gpu_harness(
            theme,
            egui::vec2(700.0, 400.0),
            ToolWindowPaneState::default(),
            |ctx, state| {
                ToolWindowPane::new("drag", state)
                    .tool(ToolWindowAnchor::Left, "Project", Glyph::Folder)
                    .tool(ToolWindowAnchor::Bottom, "Terminal", Glyph::Terminal)
                    .tool(ToolWindowAnchor::Right, "Notifications", Glyph::Bell)
                    .show_with(ctx, &mut (), |place, ui, _| match place {
                        ToolWindowPlace::Center => {
                            ui.label("Editor");
                        }
                        ToolWindowPlace::Tool(_) => {}
                        _ => {}
                    });
            },
        );
        // The Project button is dragged toward the bottom group: a ghost follows the
        // pointer and the stripe is the drop target.
        let from = harness.get_by_label("Project").rect().center() / 2.0;
        let to = pos2(from.x, from.y + 150.0);
        harness.hover_at(from);
        harness.step();
        harness.event(Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        for step in 1..=8 {
            let t = step as f32 / 8.0;
            harness.hover_at(pos2(from.x, from.y + (to.y - from.y) * t));
            harness.step();
        }
        harness.run_steps(2);
        results.add(harness.try_snapshot_options(
            format!("stripe_drag.{suffix}"),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}
