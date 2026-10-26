//! UI looks (`docs/looks.md`): one composite IDE page (toolbar, tool windows, tabs, editor,
//! status bar, a tree with a selection, a popup menu) in Islands, New UI and Classic, normal
//! and compact; the metric and structural rules of each look; switching at runtime.

mod common;

use archipel_kit::action_system::{
    ActionMenuItem, AnAction, AnActionEvent, DefaultActionGroup, Presentation,
};
use archipel_kit::editor::model::Editor;
use archipel_kit::editor::{EditorComponent, JavaScript};
use archipel_kit::look::{self, LookSettings, Metrics, SelectionShape, TabStyle, UiLook, ViewMode};
use archipel_kit::testing::{self, Harness, NodeExt, Queryable, SnapshotOptions, SnapshotResults};
use archipel_kit::ui::popup::JBPopup;
use archipel_kit::ui::tabs::{JBTabs, TabInfo};
use archipel_kit::ui::{BreadcrumbsComponent, ColoredListCellRenderer};
use archipel_kit::wm::{
    IdeFrameDecorator, StatusBarWidget, ToolWindowAnchor, ToolWindowPane, ToolWindowPaneState,
    ToolbarComboButton,
};
use archipel_kit::{Glyph, Icon, Theme, egui, tokens};

const CODE: &str = r#"// Load the open requests
const limit = 25;

async function load(name) {
  const res = await fetch(`/api/${name}?limit=${limit}`);
  if (!res.ok) {
    throw new Error("HTTP " + res.status);
  }
  return res.json();
}"#;

/// A menu command that does nothing.
struct Cmd(&'static str);

impl AnAction for Cmd {
    fn id(&self) -> &str {
        self.0
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.0)
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {}
}

/// The main menu: File, Edit, View, Navigate, Code, Help.
fn main_menu() -> DefaultActionGroup {
    let mut menu = DefaultActionGroup::new("MainMenu");
    for (name, items) in [
        ("File", ["New", "Open…", "Save All"]),
        ("Edit", ["Undo", "Redo", "Find"]),
        ("View", ["Tool Windows", "Appearance", "Quick Definition"]),
        ("Navigate", ["Back", "Forward", "Class…"]),
        (
            "Code",
            ["Reformat Code", "Optimize Imports", "Comment Line"],
        ),
        ("Help", ["Find Action…", "Keymap Reference", "About"]),
    ] {
        let mut group = DefaultActionGroup::popup(name);
        for item in items {
            group.add(Cmd(item));
        }
        menu.add_group(group);
    }
    menu
}

/// What the page keeps between frames.
struct Page {
    layout: ToolWindowPaneState,
    editor: Editor,
    menu: DefaultActionGroup,
    context: archipel_kit::action_system::DataContext,
    popup: bool,
}

impl Page {
    fn new(popup: bool) -> Self {
        Self {
            layout: ToolWindowPaneState {
                left_ratio: 0.24,
                top_ratio: 0.68,
                tool_tabs: [0, 0, 0],
                ..ToolWindowPaneState::default()
            },
            editor: Editor::new(CODE),
            menu: main_menu(),
            context: archipel_kit::action_system::DataContext::empty(),
            popup,
        }
    }
}

/// The composite IDE page.
fn page(ctx: &egui::Context, page: &mut Page) {
    let tabs = [
        TabInfo::new("load.js")
            .icon(Icon::for_file("load.js"))
            .closable(true),
        TabInfo::new("theme.rs")
            .icon(Icon::for_file("theme.rs"))
            .closable(true)
            .modified(true),
        TabInfo::new("Cargo.toml")
            .icon(Icon::for_file("Cargo.toml"))
            .closable(true),
    ];
    let Page {
        layout,
        editor,
        menu,
        context,
        popup,
    } = page;
    let project = egui::Color32::from_rgb(0x4C, 0xA0, 0x74);
    ToolWindowPane::new("ide", layout)
        .project_color(project)
        .main_menu(&*menu, &*context)
        .toolbar(IdeFrameDecorator::Native, |ui| {
            let _ = ToolbarComboButton::new("archipel")
                .icon(Glyph::Project)
                .dropdown(true)
                .show(ui);
            let _ = ToolbarComboButton::new("main")
                .icon(Glyph::Branch)
                .dropdown(true)
                .show(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    archipel_kit::action_system::ActionButton::new(Glyph::Settings, "Settings")
                        .large(true),
                );
                ui.add(
                    archipel_kit::action_system::ActionButton::new(
                        Glyph::Search,
                        "Search everywhere",
                    )
                    .large(true),
                );
                ui.add(
                    archipel_kit::action_system::ActionButton::new(Glyph::Run, "Run").large(true),
                );
            });
        })
        .left("Project", |ui| {
            let rows: [(&str, Icon, usize, Option<bool>, bool); 6] = [
                ("archipel", Glyph::Project.into(), 0, Some(true), false),
                ("src", Glyph::Folder.into(), 1, Some(true), false),
                ("load.js", Icon::for_file("load.js"), 2, None, true),
                ("theme.rs", Icon::for_file("theme.rs"), 2, None, false),
                ("tests", Glyph::Folder.into(), 1, Some(false), false),
                ("Cargo.toml", Icon::for_file("Cargo.toml"), 1, None, false),
            ];
            for (name, icon, depth, expanded, selected) in rows {
                ColoredListCellRenderer::new(name)
                    .icon(icon)
                    .depth(depth)
                    .expanded(expanded)
                    .selected(selected)
                    .show(ui);
            }
        })
        .icon(ToolWindowAnchor::Left, Glyph::Folder)
        .activate_with(ToolWindowAnchor::Left, 1)
        .right("Notifications", |ui| {
            ui.label("No new notifications");
        })
        .padded(ToolWindowAnchor::Right, true)
        .icon(ToolWindowAnchor::Right, Glyph::Bell)
        .bottom("Terminal", |ui| {
            ui.label("$ cargo test --workspace");
        })
        .padded(ToolWindowAnchor::Bottom, true)
        .icon(ToolWindowAnchor::Bottom, Glyph::Terminal)
        .tool_tabs(
            ToolWindowAnchor::Bottom,
            vec!["Local".to_owned(), "Build".to_owned()],
        )
        .center_padded(false)
        .status(|ui| {
            let _ = BreadcrumbsComponent::new()
                .crumb("archipel", Some(Glyph::Folder.into()))
                .crumb("load.js", Some(Icon::for_file("load.js")))
                .show(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(StatusBarWidget::new("4 spaces"));
                ui.add(StatusBarWidget::new("UTF-8"));
                ui.add(StatusBarWidget::new("LF"));
                ui.add(StatusBarWidget::new("5:12"));
            });
        })
        .show(ctx, |ui| {
            let area = ui.max_rect();
            JBTabs::new(&tabs, 0).show(ui);
            EditorComponent::new(editor)
                .label("Editor")
                .highlighter(&JavaScript)
                .line_numbers(true)
                .folding(true)
                .bracket_folding(true)
                .show(ui);
            if *popup {
                let anchor = egui::Rect::from_min_size(
                    egui::pos2(area.right() - 220.0, area.top() + 120.0),
                    egui::vec2(1.0, 1.0),
                );
                let mut open = true;
                JBPopup::new("context-menu", anchor)
                    .width(200.0)
                    .show(ui.ctx(), &mut open, |ui| {
                        ui.add(
                            ActionMenuItem::new("Copy")
                                .icon(Glyph::Copy)
                                .shortcut("Ctrl+C"),
                        );
                        ui.add(
                            ActionMenuItem::new("Paste")
                                .icon(Glyph::Paste)
                                .shortcut("Ctrl+V")
                                .highlighted(true),
                        );
                        ui.add(archipel_kit::ui::popup::ListSeparator::new());
                        ui.add(ActionMenuItem::new("Reformat Code").shortcut("Ctrl+Alt+L"));
                        ui.add(ActionMenuItem::new("Refactor").shortcut("Ctrl+Alt+Shift+T"));
                    });
            }
        });
}

/// A harness that shows the page in `theme` with `settings`.
fn harness(
    theme: Theme,
    settings: LookSettings,
    size: egui::Vec2,
    gpu: bool,
    popup: bool,
) -> Harness<'static, Page> {
    let mut pending = Some(theme);
    let builder = Harness::builder().with_size(size).with_max_steps(30);
    let builder = if gpu {
        builder
            .with_pixels_per_point(testing::SNAPSHOT_SCALE)
            .wgpu()
    } else {
        builder
    };
    let mut h = builder.build_state(
        move |ctx, state: &mut Page| {
            // Fonts installed during a frame take effect on the next one.
            if let Some(theme) = pending.take() {
                archipel_kit::install_with_look(ctx, theme, settings);
                ctx.request_repaint();
                return;
            }
            page(ctx, state);
        },
        Page::new(popup),
    );
    h.run_ok();
    h
}

/// The looks of the docs page: look, compact, theme id, theme.
fn cases() -> Vec<(UiLook, bool, &'static str, Theme)> {
    let mut out = Vec::new();
    for compact in [false, true] {
        out.push((
            UiLook::Islands,
            compact,
            "islands_dark",
            Theme::islands_dark(),
        ));
        out.push((UiLook::NewUi, compact, "new_ui_dark", Theme::new_ui_dark()));
        out.push((UiLook::Classic, compact, "darcula", Theme::darcula()));
    }
    out.push((
        UiLook::Classic,
        false,
        "intellij_light",
        Theme::intellij_light(),
    ));
    out
}

fn name(look: UiLook, compact: bool, theme: &str) -> String {
    format!(
        "look_{}{}_{theme}",
        look.id(),
        if compact { "_compact" } else { "" }
    )
}

#[test]
fn every_look_renders_the_composite_page() {
    let mut results = SnapshotResults::new();
    for (look, compact, theme_id, theme) in cases() {
        // Classic has no compact mode: its compact image would equal the normal one.
        if look == UiLook::Classic && compact {
            continue;
        }
        let settings = LookSettings::new(look).compact(compact);
        let mut h = harness(theme, settings, egui::vec2(1100.0, 640.0), true, true);
        h.run_steps(3);
        results.add(h.try_snapshot_options(
            name(look, compact, theme_id),
            &SnapshotOptions::new().output_path(common::IMG),
        ));
    }
    results.unwrap();
}

#[test]
fn every_theme_renders_in_every_look_without_panics() {
    for info in Theme::all() {
        for look in UiLook::ALL {
            for compact in [false, true] {
                let settings = LookSettings::new(look).compact(compact);
                let mut h = harness(
                    (info.constructor)(),
                    settings,
                    egui::vec2(900.0, 520.0),
                    false,
                    true,
                );
                h.run_steps(2);
            }
        }
    }
}

#[test]
fn switching_looks_at_runtime_relays_out_without_panics() {
    let mut h = harness(
        Theme::dark(),
        LookSettings::default(),
        egui::vec2(900.0, 520.0),
        false,
        false,
    );
    for look in [
        UiLook::Classic,
        UiLook::NewUi,
        UiLook::Islands,
        UiLook::Classic,
    ] {
        look::set_look(&h.ctx, look);
        h.run_steps(2);
        look::set_compact(&h.ctx, true);
        h.run_steps(2);
        look::set_view_mode(&h.ctx, ViewMode::distraction_free());
        h.run_steps(2);
        look::set_view_mode(&h.ctx, ViewMode::NORMAL);
        look::set_compact(&h.ctx, false);
        h.run_steps(2);
        assert_eq!(look::look(&h.ctx), look);
    }
    // The page is still there and operable.
    h.get_by_label("Project").role();
}

#[test]
fn compact_rows_are_22_and_classic_rows_20() {
    for (look, compact, row) in [
        (UiLook::Islands, false, 24.0),
        (UiLook::Islands, true, 22.0),
        (UiLook::NewUi, true, 22.0),
        (UiLook::Classic, false, 20.0),
        (UiLook::Classic, true, 20.0),
    ] {
        let mut h = testing::harness(Theme::dark(), |ui| {
            ColoredListCellRenderer::new("Row").show(ui);
        });
        look::set_settings(&h.ctx, LookSettings::new(look).compact(compact));
        h.run_steps(2);
        let rect = h.get_by_label("Row").rect();
        assert_eq!(rect.height(), row, "{look:?} compact={compact}");
    }
}

#[test]
fn compact_mode_values_follow_the_spec() {
    let m = Metrics::resolve(UiLook::Islands, true);
    assert_eq!(m.row_height, 22.0);
    assert_eq!(m.header_height, 30.0);
    assert_eq!(m.control_height, 24.0);
    assert_eq!(m.island_radius, 8);
    assert_eq!(m.window_toolbar_height, 32.0);
    assert_eq!(m.tab_height, 24.0);
}

#[test]
fn classic_tabs_always_show_their_close_buttons() {
    let tabs = [
        TabInfo::new("a.rs").closable(true),
        TabInfo::new("b.rs").closable(true),
        TabInfo::new("c.rs").closable(true),
    ];
    for (look, closes) in [
        (UiLook::Classic, 3),
        (UiLook::NewUi, 1),
        (UiLook::Islands, 1),
    ] {
        let mut h = testing::harness(Theme::dark(), |ui| {
            JBTabs::new(&tabs, 0).show(ui);
        });
        look::set_look(&h.ctx, look);
        // Away from the tabs: only the selected tab's close button in New UI and Islands.
        h.hover_at(egui::pos2(1.0, 300.0));
        h.run_steps(2);
        let count = h
            .query_all_by(|node| {
                node.label()
                    .is_some_and(|label| label.starts_with("Close "))
            })
            .count();
        assert_eq!(count, closes, "{look:?}");
    }
}

#[test]
fn tab_styles_and_underlines_per_look() {
    let islands = Metrics::resolve(UiLook::Islands, false);
    assert_eq!(islands.tab_style, TabStyle::Pill);
    assert_eq!(islands.tab_underline, 0.0);
    let new_ui = Metrics::resolve(UiLook::NewUi, false);
    assert_eq!(new_ui.tab_style, TabStyle::Underline);
    assert_eq!(
        (new_ui.tab_underline, new_ui.tab_underline_radius),
        (4.0, 2)
    );
    let classic = Metrics::resolve(UiLook::Classic, false);
    assert_eq!(
        (classic.tab_underline, classic.tab_underline_radius),
        (3.0, 0)
    );
    assert!(classic.tab_close_always && !new_ui.tab_close_always);
}

#[test]
fn status_bar_has_a_top_line_in_new_ui_and_classic_only() {
    assert_eq!(
        Metrics::resolve(UiLook::Islands, false).status_top_line,
        0.0
    );
    assert_eq!(Metrics::resolve(UiLook::NewUi, false).status_top_line, 1.0);
    assert_eq!(
        Metrics::resolve(UiLook::Classic, false).status_top_line,
        1.0
    );
    // The bar is drawn with the line's room: 31 in New UI, 30 in Islands.
    for (look, height) in [
        (UiLook::Islands, 30.0),
        (UiLook::NewUi, 31.0),
        (UiLook::Classic, 23.0),
    ] {
        let h = harness(
            Theme::dark(),
            LookSettings::new(look),
            egui::vec2(900.0, 520.0),
            false,
            false,
        );
        let widget = h.get_by_label("UTF-8").rect();
        let content = h.ctx.content_rect();
        assert!(
            content.bottom() - widget.center().y <= height / 2.0 + 1.0,
            "{look:?}: status widget at {widget:?}"
        );
    }
}

#[test]
fn selection_is_a_pill_except_in_classic() {
    assert_eq!(
        Metrics::resolve(UiLook::Islands, false).selection,
        SelectionShape::Pill
    );
    assert_eq!(
        Metrics::resolve(UiLook::NewUi, true).selection,
        SelectionShape::Pill
    );
    let classic = Metrics::resolve(UiLook::Classic, false);
    assert_eq!(classic.selection, SelectionShape::FullRow);
    assert_eq!(
        (
            classic.row_pill_inset,
            classic.row_radius,
            classic.row_inset
        ),
        (0.0, 0, 0.0)
    );
}

#[test]
fn flush_looks_have_no_island_gaps_and_one_pixel_dividers() {
    let islands = Metrics::resolve(UiLook::Islands, false);
    assert_eq!((islands.island_gap, islands.split_divider), (4.0, 4.0));
    for look in [UiLook::NewUi, UiLook::Classic] {
        let m = Metrics::resolve(look, false);
        assert_eq!(
            (m.island_gap, m.island_radius, m.split_divider),
            (0.0, 0, 1.0)
        );
    }
}

#[test]
fn classic_shows_header_actions_always_and_text_stripes() {
    let mut h = harness(
        Theme::dark(),
        LookSettings::new(UiLook::Classic),
        egui::vec2(900.0, 520.0),
        false,
        false,
    );
    // The pointer is nowhere near the tool windows: the header buttons still show.
    h.hover_at(egui::pos2(450.0, 5.0));
    common::advance_by(&mut h, 1.0);
    assert_eq!(h.get_by_label("Hide Project").role(), testing::Role::Button);
    let classic = tokens::metrics(&h.ctx);
    assert!(classic.header_actions_always);
    // The classic stripe buttons are text buttons named after their tool windows.
    let stripe_button = h.get_by_label("Terminal");
    assert!(stripe_button.rect().width() > stripe_button.rect().height());
    // The menu bar has the main menu's groups.
    h.get_by_label("File").click();
    h.run_steps(3);
    h.get_by_label("Save All");
}

#[test]
fn new_ui_and_islands_open_the_main_menu_from_a_hamburger() {
    for look in [UiLook::NewUi, UiLook::Islands] {
        let mut h = harness(
            Theme::dark(),
            LookSettings::new(look),
            egui::vec2(900.0, 520.0),
            false,
            false,
        );
        assert!(h.query_by_label("File").is_none());
        h.get_by_label("Main Menu").click();
        h.run_steps(3);
        h.get_by_label("Edit");
    }
}

#[test]
fn presentation_mode_hides_stripes_status_bar_and_tabs() {
    let h = harness(
        Theme::dark(),
        LookSettings::new(UiLook::Islands).view(ViewMode::presentation()),
        egui::vec2(1100.0, 700.0),
        false,
        false,
    );
    assert!(h.query_by_label("UTF-8").is_none(), "status bar");
    assert!(h.query_by_label("Close load.js").is_none(), "editor tabs");
    assert!(h.query_by_label("Hide Project").is_none(), "tool windows");
    assert!((h.ctx.zoom_factor() - 1.75).abs() < 1e-3);
}

#[test]
fn distraction_free_mode_hides_chrome_and_centers_the_editor() {
    let h = harness(
        Theme::dark(),
        LookSettings::new(UiLook::Islands).view(ViewMode::distraction_free()),
        egui::vec2(1400.0, 600.0),
        false,
        false,
    );
    assert!(h.query_by_label("UTF-8").is_none(), "status bar");
    assert!(h.query_by_label("Close load.js").is_none(), "editor tabs");
    assert_eq!(tokens::metrics(&h.ctx).window_toolbar_height, 30.0);
    // 120 columns of text centered: the margin is half of what is left.
    let margin = archipel_kit::editor::distraction_free_margin(1000.0, 40.0, 8.0, 120);
    assert_eq!(margin, (1000.0 - 40.0 - 960.0) / 2.0);
    assert_eq!(
        archipel_kit::editor::distraction_free_margin(500.0, 40.0, 8.0, 120),
        0.0
    );
}

#[test]
fn themes_suggest_their_look() {
    assert_eq!(Theme::darcula().suggested_look(), UiLook::Classic);
    assert_eq!(Theme::intellij_light().suggested_look(), UiLook::Classic);
    assert_eq!(Theme::new_ui_light().suggested_look(), UiLook::NewUi);
    assert_eq!(Theme::islands_dark().suggested_look(), UiLook::Islands);
    assert_eq!(Theme::dark().suggested_look(), UiLook::Islands);
    // A suggestion only: installing the theme keeps the default look.
    let ctx = egui::Context::default();
    archipel_kit::install(&ctx, Theme::darcula());
    assert_eq!(look::look(&ctx), UiLook::Islands);
}

#[test]
fn selected_rows_use_the_themes_selection_foreground() {
    // IntelliJ Light: white text on the dark blue selection, not black.
    let colors = Theme::intellij_light().colors;
    let state = archipel_kit::parts::State {
        enabled: true,
        selected: true,
        ..Default::default()
    };
    let text = state.text(archipel_kit::Tone::Default, &colors);
    assert_eq!(text, colors.selection_text);
    assert_eq!(text, egui::Color32::WHITE);
    // Without focus the quieter selection keeps the dark text.
    let inactive = state
        .inactive(true)
        .text(archipel_kit::Tone::Default, &colors);
    assert_eq!(inactive, colors.selection_text_inactive);
    // Islands: the selection foreground is the strong text color, as before.
    let islands = Theme::dark().colors;
    assert_eq!(
        state.text(archipel_kit::Tone::Default, &islands),
        islands.text_strong
    );
}

#[test]
fn stripe_tooltips_never_cover_the_pointer_in_small_windows() {
    let mut h = harness(
        Theme::dark(),
        LookSettings::default(),
        egui::vec2(420.0, 300.0),
        false,
        false,
    );
    let stripe_button = |h: &Harness<'static, Page>| {
        h.query_all_by_label("Notifications")
            .find(|node| node.role() == testing::Role::Button)
            .map(|node| node.rect())
            .expect("the stripe button")
    };
    let button = stripe_button(&h);
    h.hover_at(button.center());
    h.run_steps(3);
    let pointer = button.center();
    let covering = h
        .query_all_by(|node| node.label().is_some_and(|l| l == "Notifications"))
        .filter(|node| node.rect() != button)
        .any(|node| node.rect().contains(pointer));
    assert!(!covering, "the tooltip covers the pointer");
    // The button under the pointer still takes the click.
    h.drag_at(pointer);
    h.run();
    h.drop_at(pointer);
    h.run_steps(2);
    assert!(!h.state().layout.is_visible(ToolWindowAnchor::Right));
}
