//! `ActionButton`: the IntelliJ state machine (spec §1.1 to §1.4) as tests.

mod common;

use archipel_kit::action_system::*;
use archipel_kit::testing::{self, Harness, NodeExt, Queryable};
use archipel_kit::{Glyph, Theme};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2, vec2};
use std::cell::Cell;
use std::rc::Rc;

fn press(harness: &Harness<'_, Counts>, pos: Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton {
        pos,
        button,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
}

fn release(harness: &Harness<'_, Counts>, pos: Pos2, button: PointerButton) {
    harness.event(Event::PointerButton {
        pos,
        button,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
}

#[derive(Default)]
struct Counts {
    a: u32,
    b: u32,
}

/// Two buttons side by side, "A" and "B".
fn two() -> Harness<'static, Counts> {
    let mut harness = testing::harness_state(Theme::dark(), Counts::default(), |ui, counts| {
        ui.horizontal(|ui| {
            if ui.add(ActionButton::new(Glyph::Run, "A")).clicked() {
                counts.a += 1;
            }
            if ui.add(ActionButton::new(Glyph::Stop, "B")).clicked() {
                counts.b += 1;
            }
        });
    });
    harness.run();
    harness
}

fn center(harness: &Harness<'_, Counts>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

/// Whether a `fill` rect is painted exactly over the button named `label`.
fn filled_with(harness: &Harness<'_, Counts>, label: &str, fill: egui::Color32) -> bool {
    let rect = harness.get_by_label(label).rect();
    harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Rect(r) => r.fill == fill && r.rect.expand(0.6).contains_rect(rect),
            _ => false,
        })
}

#[test]
fn fires_on_release_inside_not_on_press() {
    let mut harness = two();
    let a = center(&harness, "A");
    press(&harness, a, PointerButton::Primary);
    harness.run();
    assert_eq!(harness.state().a, 0, "not on press");
    release(&harness, a, PointerButton::Primary);
    harness.run();
    assert_eq!(harness.state().a, 1, "on release");
}

#[test]
fn release_outside_after_a_press_inside_does_nothing() {
    let mut harness = two();
    let a = center(&harness, "A");
    press(&harness, a, PointerButton::Primary);
    harness.run();
    let away = a + vec2(0.0, 60.0);
    harness.event(Event::PointerMoved(away));
    harness.run();
    release(&harness, away, PointerButton::Primary);
    harness.run();
    assert_eq!(harness.state().a, 0);
}

#[test]
fn a_press_outside_released_inside_does_nothing() {
    let mut harness = two();
    let a = center(&harness, "A");
    press(&harness, a + vec2(0.0, 60.0), PointerButton::Primary);
    harness.run();
    harness.event(Event::PointerMoved(a));
    harness.run();
    release(&harness, a, PointerButton::Primary);
    harness.run();
    assert_eq!(harness.state().a, 0);
}

#[test]
fn only_the_primary_button_fires() {
    let mut harness = two();
    let a = center(&harness, "A");
    for button in [PointerButton::Secondary, PointerButton::Middle] {
        press(&harness, a, button);
        harness.run();
        release(&harness, a, button);
        harness.run();
    }
    assert_eq!(harness.state().a, 0);
    assert!(
        !filled_with(&harness, "A", Theme::dark().colors.pressed),
        "a secondary press changes no state"
    );
}

#[test]
fn hover_fills_and_press_fills_more() {
    let mut harness = two();
    let colors = Theme::dark().colors;
    let a = center(&harness, "A");
    harness.hover_at(a);
    harness.run();
    assert!(filled_with(&harness, "A", colors.hover));
    press(&harness, a, PointerButton::Primary);
    harness.run();
    assert!(filled_with(&harness, "A", colors.pressed));
    assert!(!filled_with(&harness, "A", colors.hover));
}

#[test]
fn dragging_off_removes_the_pressed_look() {
    let mut harness = two();
    let colors = Theme::dark().colors;
    let a = center(&harness, "A");
    press(&harness, a, PointerButton::Primary);
    harness.run();
    assert!(filled_with(&harness, "A", colors.pressed));
    harness.event(Event::PointerMoved(a + vec2(0.0, 60.0)));
    harness.run();
    assert!(!filled_with(&harness, "A", colors.pressed));
    assert!(!filled_with(&harness, "A", colors.hover));
}

#[test]
fn hover_is_suppressed_on_other_buttons_while_a_press_started_elsewhere() {
    let mut harness = two();
    let colors = Theme::dark().colors;
    let (a, b) = (center(&harness, "A"), center(&harness, "B"));
    press(&harness, a, PointerButton::Primary);
    harness.run();
    harness.event(Event::PointerMoved(b));
    harness.run();
    harness.run();
    assert!(
        !filled_with(&harness, "B", colors.hover),
        "B does not hover"
    );
    assert!(!filled_with(&harness, "B", colors.pressed));
    release(&harness, b, PointerButton::Primary);
    harness.run();
    assert_eq!(harness.state().b, 0, "B was not clicked either");
    harness.event(Event::PointerMoved(b + vec2(1.0, 0.0)));
    harness.run();
    assert!(
        filled_with(&harness, "B", colors.hover),
        "hover returns after release"
    );
}

#[test]
fn a_press_that_started_on_a_non_button_does_not_suppress_hover_on_release() {
    let mut harness = two();
    let colors = Theme::dark().colors;
    let b = center(&harness, "B");
    harness.hover_at(b);
    harness.run();
    assert!(filled_with(&harness, "B", colors.hover));
}

#[test]
fn a_toggled_on_button_keeps_the_pressed_look() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(ActionButton::new(Glyph::Pin, "Pin").selected(true));
        ui.add(ActionButton::new(Glyph::Pin, "Off").selected(false));
    });
    harness.run();
    let colors = Theme::dark().colors;
    let rect = harness.get_by_label("Pin").rect();
    let painted = |fill| {
        harness.output().shapes.iter().any(|c| match &c.shape {
            egui::Shape::Rect(r) => r.fill == fill && r.rect.expand(0.6).contains_rect(rect),
            _ => false,
        })
    };
    assert!(painted(colors.pressed));
    assert!(harness.get_by_label("Pin").is_checked());
    assert!(!harness.get_by_label("Off").is_checked());
}

#[test]
fn a_disabled_button_ignores_everything() {
    let mut harness = testing::harness_state(Theme::dark(), 0u32, |ui, n| {
        if ui
            .add(ActionButton::new(Glyph::Run, "Off").enabled(false))
            .clicked()
        {
            *n += 1;
        }
    });
    harness.run();
    let p = harness.get_by_label("Off").rect().center();
    harness.hover_at(p);
    harness.run();
    harness.get_by_label("Off").click();
    harness.run();
    assert_eq!(*harness.state(), 0);
    assert!(harness.get_by_label("Off").is_disabled());
}

#[test]
fn keyboard_focus_shows_the_selected_look_and_ring() {
    let mut harness = two();
    let colors = Theme::dark().colors;
    harness.get_by_label("A").focus();
    harness.run();
    assert!(filled_with(&harness, "A", colors.hover));
}

#[test]
fn space_activates_a_focused_button_and_enter_does_not() {
    let mut harness = testing::harness_state(Theme::dark(), 0u32, |ui, n| {
        if ActionButton::new(Glyph::Run, "Go").show(ui).activated {
            *n += 1;
        }
    });
    harness.run();
    harness.get_by_label("Go").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 0, "Enter does not activate");
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(*harness.state(), 1, "Space does");
}

#[test]
fn down_activates_only_a_button_with_a_drop_down_arrow() {
    let mut harness = testing::harness_state(Theme::dark(), (0u32, 0u32), |ui, n| {
        if ActionButton::new(Glyph::Run, "Plain").show(ui).activated {
            n.0 += 1;
        }
        if ActionButton::new(Glyph::Run, "Menu")
            .dropdown(true)
            .show(ui)
            .activated
        {
            n.1 += 1;
        }
    });
    harness.run();
    harness.get_by_label("Plain").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().0, 0);
    harness.get_by_label("Menu").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().1, 1);
}

/// Not focusable at all unless a screen reader is active.
#[test]
fn it_is_not_in_the_tab_order_without_a_screen_reader() {
    let ctx = egui::Context::default();
    archipel_kit::install(&ctx, Theme::dark());
    let focusable = Cell::new(true);
    let frame = |ctx: &egui::Context| {
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = ui.add(ActionButton::new(Glyph::Run, "Run"));
                focusable.set(response.sense.is_focusable());
            });
        });
    };
    frame(&ctx);
    frame(&ctx);
    frame(&ctx);
    assert!(!focusable.get(), "no screen reader: not a Tab stop");
    // With a screen reader (AccessKit on) it is reachable.
    ctx.enable_accesskit();
    for _ in 0..4 {
        frame(&ctx);
    }
    assert!(focusable.get(), "screen reader on: a Tab stop");
}

#[test]
fn focusable_can_be_forced_either_way() {
    let ctx = egui::Context::default();
    archipel_kit::install(&ctx, Theme::dark());
    let senses = Cell::new((false, true));
    for _ in 0..3 {
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let yes = ui.add(ActionButton::new(Glyph::Run, "Yes").focusable(true));
                let no = ui.add(ActionButton::new(Glyph::Run, "No").focusable(false));
                senses.set((yes.sense.is_focusable(), no.sense.is_focusable()));
            });
        });
    }
    assert_eq!(senses.get(), (true, false));
}

#[test]
fn sizes_are_22_default_20_small_30_large_and_40_in_the_title_pane() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.horizontal(|ui| {
            ui.add(ActionButton::new(Glyph::Run, "Default"));
            ui.add(ActionButton::new(Glyph::Run, "Small").small(true));
            ui.add(ActionButton::new(Glyph::Run, "Large").large(true));
            ui.add(ActionButton::new(Glyph::Run, "Title").title_pane(true));
        });
    });
    let size = |label: &str| harness.get_by_label(label).rect().size();
    assert_eq!(size("Default"), vec2(22.0, 22.0));
    assert_eq!(size("Small"), vec2(20.0, 20.0));
    assert_eq!(size("Large"), vec2(30.0, 30.0));
    assert_eq!(size("Title"), vec2(40.0, 40.0));
}

#[test]
fn a_drop_down_arrow_widens_the_button() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(ActionButton::new(Glyph::Run, "Menu").dropdown(true));
    });
    let rect = harness.get_by_label("Menu").rect();
    assert_eq!(rect.height(), 22.0);
    assert_eq!(
        rect.width(),
        22.0 + archipel_kit::tokens::action_toolbar::ARROW_WIDTH
    );
}

struct Counter {
    runs: Rc<Cell<u32>>,
    on: Cell<bool>,
    enabled: bool,
}

impl AnAction for Counter {
    fn id(&self) -> &str {
        "Test.Counter"
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Count")
            .icon(Glyph::Run)
            .description("Counts runs")
            .shortcut_text("Ctrl+R")
    }

    fn update(&self, e: &mut AnActionEvent<'_>) {
        e.presentation.enabled = self.enabled;
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.runs.set(self.runs.get() + 1);
        self.on.set(!self.on.get());
    }
}

#[test]
fn for_action_renders_the_presentation_and_performs_on_release() {
    let runs = Rc::new(Cell::new(0));
    let action = Counter {
        runs: runs.clone(),
        on: Cell::new(false),
        enabled: true,
    };
    let context = DataContext::empty();
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, performed| {
        *performed |= ActionButton::for_action(&action, &context, &ActionPlace::TOOLBAR)
            .show(ui)
            .performed;
    });
    harness.run();
    assert!(!harness.get_by_label("Count").is_disabled());
    harness.get_by_label("Count").click();
    harness.run();
    assert!(*harness.state());
    assert_eq!(runs.get(), 1);
}

#[test]
fn for_action_does_not_perform_a_disabled_action() {
    let runs = Rc::new(Cell::new(0));
    let action = Counter {
        runs: runs.clone(),
        on: Cell::new(false),
        enabled: false,
    };
    let context = DataContext::empty();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ActionButton::for_action(&action, &context, &ActionPlace::TOOLBAR).show(ui);
    });
    harness.run();
    assert!(harness.get_by_label("Count").is_disabled());
    harness.get_by_label("Count").click();
    harness.run();
    assert_eq!(runs.get(), 0);
}

#[test]
fn a_toggle_action_renders_its_state() {
    struct Wrap(Cell<bool>);
    impl ToggleAction for Wrap {
        fn id(&self) -> &str {
            "Wrap"
        }
        fn template_presentation(&self) -> Presentation {
            Presentation::new("Soft-Wrap").icon(Glyph::SoftWrap)
        }
        fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool {
            self.0.get()
        }
        fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) {
            self.0.set(selected);
        }
    }
    let action = Toggle::new(Wrap(Cell::new(false)));
    let context = DataContext::empty();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ActionButton::for_action(&action, &context, &ActionPlace::TOOLBAR).show(ui);
    });
    harness.run();
    assert!(!harness.get_by_label("Soft-Wrap").is_checked());
    harness.get_by_label("Soft-Wrap").click();
    harness.run();
    harness.run();
    assert!(harness.get_by_label("Soft-Wrap").is_checked());
}

#[test]
fn the_tooltip_has_the_text_and_the_shortcut_after_the_delay() {
    let action = Counter {
        runs: Rc::new(Cell::new(0)),
        on: Cell::new(false),
        enabled: true,
    };
    let context = DataContext::empty();
    let mut harness =
        testing::harness_state_sized(Theme::dark(), vec2(300.0, 200.0), (), |ui, _| {
            ActionButton::for_action(&action, &context, &ActionPlace::TOOLBAR).show(ui);
        });
    harness.run();
    let shows_shortcut = |harness: &Harness<'_, ()>| {
        harness.output().shapes.iter().any(|c| match &c.shape {
            egui::Shape::Text(t) => t.galley.text() == "Ctrl+R",
            _ => false,
        })
    };
    harness.hover_at(harness.get_by_label("Count").rect().center());
    harness.run_steps(10);
    assert!(!shows_shortcut(&harness), "not before 0.5 s");
    harness.run_steps(40);
    assert!(shows_shortcut(&harness), "shortcut in the tooltip");
}

#[test]
fn hovering_publishes_the_action_description_for_the_status_bar() {
    let action = Counter {
        runs: Rc::new(Cell::new(0)),
        on: Cell::new(false),
        enabled: true,
    };
    let context = DataContext::empty();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ActionButton::for_action(&action, &context, &ActionPlace::TOOLBAR).show(ui);
    });
    harness.run();
    assert_eq!(ActionButton::hovered_description(&harness.ctx), None);
    harness.hover_at(harness.get_by_label("Count").rect().center());
    harness.run();
    assert_eq!(
        ActionButton::hovered_description(&harness.ctx).as_deref(),
        Some("Counts runs")
    );
    harness.hover_at(pos2(-20.0, -20.0));
    harness.run();
    harness.run();
    harness.run();
    assert_eq!(ActionButton::hovered_description(&harness.ctx), None);
}

#[test]
fn an_action_without_an_icon_shows_the_unknown_icon_not_nothing() {
    let presentation = Presentation::new("Mystery");
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(ActionButton::from_presentation(&presentation));
    });
    assert_eq!(
        harness.get_by_label("Mystery").rect().size(),
        vec2(22.0, 22.0)
    );
}

#[test]
fn action_button_states_snapshot() {
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let mut harness = testing::harness_state(theme, Counts::default(), |ui, _| {
            ui.horizontal(|ui| {
                ui.add(ActionButton::new(Glyph::Refresh, "Normal"));
                ui.add(ActionButton::new(Glyph::Pin, "Toggled on").selected(true));
                ui.add(ActionButton::new(Glyph::Refresh, "Focused").focusable(true));
                ui.add(ActionButton::new(Glyph::Refresh, "Disabled").enabled(false));
                ui.add(ActionButton::new(Glyph::Branch, "Menu").dropdown(true));
                ui.add(ActionButton::new(Glyph::Search, "Large").large(true));
            });
        });
        harness.run();
        harness.get_by_label("Focused").focus();
        harness.run();
        harness
            .try_snapshot_options(
                format!("action_button_states.{suffix}"),
                &testing::SnapshotOptions::new().output_path(common::IMG),
            )
            .expect("snapshot matches");
    }
}
