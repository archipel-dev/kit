//! The main toolbar's widgets and zones (IntelliJ spec §14).

mod common;

use archipel_kit::action_system::*;
use archipel_kit::testing::{self, Harness, NodeT, Queryable};
use archipel_kit::wm::{IdeFrameDecorator, MainToolbar, ToolbarComboButton, ToolbarSplitButton};
use archipel_kit::{Glyph, Theme, egui};
use egui::{Key, Modifiers, pos2, vec2};
use std::cell::RefCell;
use std::rc::Rc;

fn button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let glyph = match label {
        "Main menu" => Glyph::Menu,
        "Settings" => Glyph::Settings,
        _ => Glyph::Search,
    };
    ui.add(ActionButton::new(glyph, label).large(true).focusable(true))
}

#[derive(Default)]
struct Seen {
    main: u32,
    arrow: u32,
}

fn zoned(theme: Theme, chrome: IdeFrameDecorator) -> Harness<'static, Seen> {
    let log: Rc<RefCell<Vec<String>>> = Rc::default();
    struct Customize(Rc<RefCell<Vec<String>>>);
    impl AnAction for Customize {
        fn id(&self) -> &str {
            "CustomizeToolbar"
        }
        fn template_presentation(&self) -> Presentation {
            Presentation::new("Customize Toolbar…")
        }
        fn action_performed(&self, _e: &AnActionEvent<'_>) {
            self.0.borrow_mut().push("customize".into());
        }
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 48.0))
        .build_state(
            move |ctx, seen: &mut Seen| {
                archipel_kit::install(ctx, theme.clone());
                let mut menu = DefaultActionGroup::new("Toolbar");
                menu.add(Customize(log.clone()));
                let context = DataContext::empty();
                MainToolbar::new("tb")
                    .chrome(chrome)
                    .context_menu(&menu, &context)
                    .show_zones(
                        ctx,
                        |ui| {
                            button(ui, "Main menu");
                            ToolbarComboButton::new("archipel")
                                .icon(Glyph::Folder)
                                .dropdown(true)
                                .show(ui);
                        },
                        |ui| {
                            let out = ToolbarSplitButton::new("app").icon(Glyph::Run).show(ui);
                            seen.main += u32::from(out.main.clicked());
                            seen.arrow += u32::from(out.arrow.clicked());
                        },
                        |ui| {
                            button(ui, "Search everywhere");
                            button(ui, "Settings");
                        },
                    );
                egui::CentralPanel::default().show(ctx, |_| {});
            },
            Seen::default(),
        );
    harness.run_steps(4);
    harness
}

#[test]
fn zones_left_center_right() {
    let harness = zoned(Theme::dark(), IdeFrameDecorator::Native);
    let window = harness.ctx.content_rect();
    let menu = harness.get_by_label("Main menu").rect();
    let project = harness.get_by_label("archipel").rect();
    let run = harness.get_by_label("app").rect();
    let arrow = harness.get_by_label("app options").rect();
    let search = harness.get_by_label("Search everywhere").rect();
    let settings = harness.get_by_label("Settings").rect();
    assert_eq!(menu.left(), window.left() + 6.0, "6 pt window padding");
    assert!(
        project.left() > menu.right(),
        "project widget after the menu button"
    );
    let center = (run.left() + arrow.right()) / 2.0;
    assert!(
        (center - window.center().x).abs() <= 1.0,
        "run widget centered: {center}"
    );
    assert!(search.left() > arrow.right(), "search is in the right zone");
    assert!(settings.left() > search.right());
    assert!(
        (window.right() - 6.0 - settings.right()).abs() <= 1.0,
        "the right zone ends at the window padding"
    );
}

#[test]
fn main_toolbar_buttons_are_30_with_a_20_icon() {
    let harness = zoned(Theme::dark(), IdeFrameDecorator::Native);
    assert_eq!(
        harness.get_by_label("Settings").rect().size(),
        vec2(30.0, 30.0)
    );
    assert_eq!(harness.get_by_label("archipel").rect().height(), 30.0);
}

#[test]
fn the_split_button_has_two_parts_with_their_own_clicks() {
    let mut harness = zoned(Theme::dark(), IdeFrameDecorator::Native);
    harness.get_by_label("app").click();
    harness.run_steps(2);
    assert_eq!((harness.state().main, harness.state().arrow), (1, 0));
    harness.get_by_label("app options").click();
    harness.run_steps(2);
    assert_eq!((harness.state().main, harness.state().arrow), (1, 1));
}

#[test]
fn the_split_button_measures_arrow_26_and_separator_margins_3() {
    let harness = zoned(Theme::dark(), IdeFrameDecorator::Native);
    let main = harness.get_by_label("app").rect();
    let arrow = harness.get_by_label("app options").rect();
    assert_eq!(arrow.width(), 26.0);
    assert_eq!(arrow.height(), 30.0);
    assert_eq!(arrow.left() - main.right(), 7.0, "3 + 1 pt line + 3");
}

#[test]
fn a_drop_down_button_elides_text_beyond_350_pt() {
    let long = "a-very-long-project-name-".repeat(12);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ToolbarComboButton::new(long.as_str())
            .dropdown(true)
            .show(ui);
    });
    harness.run();
    let rect = harness.get_by_label(long.as_str()).rect();
    assert!(rect.width() <= 350.0 + 0.5, "{}", rect.width());
    assert!(rect.width() >= 340.0, "it grows to the maximum");
}

#[test]
fn right_clicking_the_empty_background_opens_the_customize_menu() {
    let mut harness = zoned(Theme::dark(), IdeFrameDecorator::Native);
    assert!(harness.query_by_label("Customize Toolbar…").is_none());
    let window = harness.ctx.content_rect();
    // Between the left and the center zone: empty.
    harness.event(egui::Event::PointerMoved(pos2(
        window.center().x - 150.0,
        20.0,
    )));
    harness.event(egui::Event::PointerButton {
        pos: pos2(window.center().x - 150.0, 20.0),
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(egui::Event::PointerButton {
        pos: pos2(window.center().x - 150.0, 20.0),
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(3);
    assert!(harness.query_by_label("Customize Toolbar…").is_some());
}

#[test]
fn left_and_right_move_focus_between_toolbar_controls() {
    let mut harness = zoned(Theme::dark(), IdeFrameDecorator::Native);
    harness.get_by_label("Main menu").focus();
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert!(
        !harness
            .get_by_label("Main menu")
            .accesskit_node()
            .is_focused(),
        "focus left the first control"
    );
    harness.key_press(Key::ArrowLeft);
    harness.run_steps(2);
    assert!(
        harness
            .get_by_label("Main menu")
            .accesskit_node()
            .is_focused()
    );
}

#[test]
fn title_pane_buttons_are_40_pt_squares_with_a_square_hover() {
    let harness = zoned(Theme::dark(), IdeFrameDecorator::Borderless);
    for label in ["Minimize window", "Maximize window", "Close window"] {
        assert_eq!(
            harness.get_by_label(label).rect().size(),
            vec2(40.0, 40.0),
            "{label}"
        );
    }
    let close = harness.get_by_label("Close window").rect();
    assert_eq!(close.right(), harness.ctx.content_rect().right());
}

#[test]
fn main_toolbar_snapshot() {
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut harness = zoned(theme, IdeFrameDecorator::Native);
        harness.hover_at(harness.get_by_label("Search everywhere").rect().center());
        harness.run_steps(2);
        harness
            .try_snapshot_options(
                format!("main_toolbar_zones.{suffix}"),
                &testing::SnapshotOptions::new().output_path(common::IMG),
            )
            .expect("snapshot matches");
    }
}
