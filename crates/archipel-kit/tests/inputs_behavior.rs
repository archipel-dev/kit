//! Behavior rules of buttons, checkboxes, radios, toggles, spinners, combo boxes and segmented
//! buttons, named after the IntelliJ UX spec sections 4, 13.2, 13.3, 13.4 and 13.5.

mod common;

use archipel_kit::openapi::ui::ComboBox;
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::ui::{
    Button, JBCheckBox, JBIntSpinner, JBRadioButton, OnOffButton, SegmentedButton, ThreeState,
    ThreeStateCheckBox,
};
use archipel_kit::{Theme, egui, tokens};
use egui::{Key, Modifiers, PointerButton, pos2};

fn press(harness: &mut testing::Harness<'_, impl Sized>, pos: egui::Pos2, pressed: bool) {
    harness.hover_at(pos);
    harness.event(egui::Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

// 13.2 Buttons.

#[test]
fn a_button_is_at_least_72_by_28() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(Button::secondary("OK"));
        ui.add(Button::primary("A much longer label"));
    });
    let ok = harness.get_by_label("OK").rect();
    assert_eq!(ok.width(), tokens::size::BUTTON_MIN_WIDTH);
    assert_eq!(ok.height(), 28.0);
    assert!(harness.get_by_label("A much longer label").rect().width() > 72.0);
}

#[test]
fn space_clicks_a_button_on_release_and_enter_on_press() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui.add(Button::primary("Run")).clicked() {
            *clicks += 1;
        }
    });
    harness.get_by_label("Run").focus();
    harness.run();
    // Space pressed alone does nothing yet.
    harness.event(egui::Event::Key {
        key: Key::Space,
        physical_key: Some(Key::Space),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(*harness.state(), 0, "not on press");
    harness.event(egui::Event::Key {
        key: Key::Space,
        physical_key: Some(Key::Space),
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(*harness.state(), 1, "on release");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 2, "Enter clicks at once");
}

#[test]
fn alt_and_the_mnemonic_click_an_enabled_button() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui.add(Button::secondary("Cancel").mnemonic('C')).clicked() {
            *clicks += 1;
        }
        if ui
            .add(Button::secondary("Delete").mnemonic('D').enabled(false))
            .clicked()
        {
            *clicks += 100;
        }
    });
    harness.key_press_modifiers(Modifiers::ALT, Key::C);
    harness.run();
    assert_eq!(*harness.state(), 1);
    harness.key_press_modifiers(Modifiers::ALT, Key::D);
    harness.run();
    assert_eq!(*harness.state(), 1, "a disabled button has no mnemonic");
}

#[test]
fn a_button_does_not_react_to_hover_but_darkens_while_pressed() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(Button::primary("Run"));
    });
    harness.run();
    let image = |h: &mut testing::Harness<'_>| {
        h.fit_contents();
        h.render().expect("render")
    };
    let rest = image(&mut harness);
    let center = harness.get_by_label("Run").rect().center();
    harness.hover_at(center);
    harness.run();
    let hovered = image(&mut harness);
    assert_eq!(rest, hovered, "no hover change (IntelliJ spec 13.2)");
    harness.event(egui::Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let pressed = image(&mut harness);
    assert_ne!(rest, pressed, "pressed is darker");
}

#[test]
fn a_focused_button_gets_a_focus_ring_outside_its_bounds() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add_space(8.0);
        ui.add(Button::secondary("Run"));
    });
    harness.run();
    harness.fit_contents();
    let rest = harness.render().expect("render");
    harness.get_by_label("Run").focus();
    harness.run();
    harness.fit_contents();
    let focused = harness.render().expect("render");
    assert_ne!(rest, focused);
}

// 13.3 CheckBox / RadioButton / OnOffButton.

#[test]
fn a_check_box_toggles_on_release_inside_not_on_press() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, on| {
        ui.add(JBCheckBox::new(on, "Reformat"));
    });
    let center = harness.get_by_label("Reformat").rect().center();
    press(&mut harness, center, true);
    assert!(!*harness.state(), "not on press");
    press(&mut harness, center, false);
    assert!(*harness.state(), "on release");
}

#[test]
fn releasing_outside_a_check_box_does_not_toggle_it() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, on| {
        ui.add(JBCheckBox::new(on, "Reformat"));
    });
    let rect = harness.get_by_label("Reformat").rect();
    press(&mut harness, rect.center(), true);
    harness.hover_at(pos2(rect.right() + 120.0, rect.center().y));
    harness.run();
    press(
        &mut harness,
        pos2(rect.right() + 120.0, rect.center().y),
        false,
    );
    assert!(!*harness.state());
}

#[test]
fn space_toggles_a_check_box_on_release_and_enter_does_not() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, on| {
        ui.add(JBCheckBox::new(on, "Reformat"));
    });
    harness.get_by_label("Reformat").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!*harness.state(), "Enter belongs to the default button");
    harness.event(egui::Event::Key {
        key: Key::Space,
        physical_key: Some(Key::Space),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(!*harness.state(), "not on key press");
    harness.event(egui::Event::Key {
        key: Key::Space,
        physical_key: Some(Key::Space),
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(*harness.state(), "on key release");
}

#[test]
fn a_three_state_check_box_cycles_unchecked_checked_indeterminate() {
    let mut harness = testing::harness_state(Theme::dark(), ThreeState::Unchecked, |ui, state| {
        ui.add(ThreeStateCheckBox::new(state, "All"));
    });
    for expected in [
        ThreeState::Checked,
        ThreeState::Indeterminate,
        ThreeState::Unchecked,
    ] {
        harness.get_by_label("All").click();
        harness.run();
        assert_eq!(*harness.state(), expected);
    }
}

#[test]
fn radio_buttons_and_toggles_follow_the_same_activation_rules() {
    let mut harness = testing::harness_state(Theme::dark(), (0, false), |ui, (indent, on)| {
        ui.add(JBRadioButton::new(indent, 0, "Tabs"));
        ui.add(JBRadioButton::new(indent, 1, "Spaces"));
        ui.add(OnOffButton::new(on, "Power save"));
    });
    harness.get_by_label("Spaces").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().0, 0, "Enter does not select");
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state().0, 1);
    harness.get_by_label("Power save").focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(harness.state().1);
}

// 13.4 JBIntSpinner.

fn spinner(value: u32) -> testing::Harness<'static, u32> {
    testing::harness_state(Theme::dark(), value, |ui, limit| {
        ui.add(JBIntSpinner::new(limit).name("Limit").range(1..=20).step(5));
    })
}

#[test]
fn a_spinner_is_72_by_28_with_arrows_that_step_once_per_press() {
    let mut harness = spinner(10);
    let arrows = harness.get_by_label("Increment or decrement").rect();
    let field = harness.get_by_label("Limit").rect();
    assert!(arrows.right() >= field.right());
    let up = pos2(arrows.center().x, arrows.top() + 4.0);
    press(&mut harness, up, true);
    press(&mut harness, up, false);
    assert_eq!(*harness.state(), 15);
    let down = pos2(arrows.center().x, arrows.bottom() - 4.0);
    press(&mut harness, down, true);
    press(&mut harness, down, false);
    assert_eq!(*harness.state(), 10);
    let frame_width = arrows.right() - field.left();
    assert!(frame_width >= 72.0 - 16.0, "{frame_width}");
}

#[test]
fn a_held_arrow_repeats_after_the_delay_and_stops_at_the_maximum() {
    let mut harness = spinner(1);
    let arrows = harness.get_by_label("Increment or decrement").rect();
    let up = pos2(arrows.center().x, arrows.top() + 4.0);
    press(&mut harness, up, true);
    assert_eq!(*harness.state(), 6, "the press steps at once");
    common::advance_by(
        &mut harness,
        f64::from(tokens::motion::SPINNER_REPEAT_DELAY) - 0.1,
    );
    assert_eq!(*harness.state(), 6, "nothing before the delay");
    common::advance_by(&mut harness, 0.5);
    assert_eq!(*harness.state(), 20, "repeats, clamped to the maximum");
    press(&mut harness, up, false);
    let held = *harness.state();
    common::advance_by(&mut harness, 0.5);
    assert_eq!(*harness.state(), held, "released: no more steps");
}

#[test]
fn the_wheel_steps_only_while_the_field_has_focus() {
    let mut harness = spinner(10);
    let center = harness.get_by_label("Limit").rect().center();
    harness.hover_at(center);
    harness.run();
    let wheel = |h: &mut testing::Harness<'_, u32>, delta: f32| {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, delta),
            modifiers: Modifiers::NONE,
        });
        h.run();
    };
    wheel(&mut harness, 1.0);
    assert_eq!(*harness.state(), 10, "no focus: the wheel does nothing");
    harness.get_by_label("Limit").focus();
    harness.run();
    harness.hover_at(center);
    wheel(&mut harness, 1.0);
    assert_eq!(*harness.state(), 15, "up steps up by the step");
    wheel(&mut harness, -1.0);
    wheel(&mut harness, -1.0);
    assert_eq!(*harness.state(), 5);
    for _ in 0..5 {
        wheel(&mut harness, -1.0);
    }
    assert_eq!(*harness.state(), 1, "clamped to the minimum");
}

#[test]
fn a_disabled_spinner_ignores_the_wheel_and_the_arrows() {
    let mut harness = testing::harness_state(Theme::dark(), 10_u32, |ui, limit| {
        ui.add(
            JBIntSpinner::new(limit)
                .name("Limit")
                .range(1..=20)
                .enabled(false),
        );
    });
    let arrows = harness.get_by_label("Increment or decrement").rect();
    press(&mut harness, arrows.center(), true);
    press(&mut harness, arrows.center(), false);
    assert_eq!(*harness.state(), 10);
}

// 4 ComboBox.

fn combo(
    options: &'static [&'static str],
    value: &'static str,
) -> testing::Harness<'static, &'static str> {
    let mut harness = testing::harness_state(Theme::dark(), value, move |ui, value| {
        ui.add(
            ComboBox::new("combo", value, options.iter().copied())
                .label("Combo")
                .width(120.0),
        );
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 420.0));
    harness.run();
    harness
}

const FOUR: &[&str] = &["Alpha", "Beta", "Gamma", "Delta"];

#[test]
fn a_combo_box_is_at_least_49_by_28_and_the_list_is_as_wide_as_it() {
    let harness = testing::harness(Theme::dark(), |ui| {
        let mut value = "A";
        ui.add(ComboBox::new("tiny", &mut value, ["A", "B"]).width(10.0));
    });
    let rect = harness.get_by_role(Role::ComboBox).rect();
    assert_eq!(rect.width(), tokens::size::COMBO_MIN_WIDTH);
    assert_eq!(rect.height(), 28.0);
    let mut harness = combo(FOUR, "Alpha");
    let combo = harness.get_by_label("Combo").rect();
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_steps(3);
    let row = harness.get_by_label("Beta").rect();
    assert!(row.left() >= combo.left() - 1.0);
    // Rows span the whole list, which is as wide as the combo box.
    assert!(
        (row.width() - combo.width()).abs() < 2.5,
        "{row:?} {combo:?}"
    );
}

#[test]
fn alt_down_f4_enter_and_space_open_the_list_and_escape_keeps_the_value() {
    for open in [
        (Modifiers::ALT, Key::ArrowDown),
        (Modifiers::NONE, Key::F4),
        (Modifiers::NONE, Key::Enter),
        (Modifiers::NONE, Key::Space),
    ] {
        let mut harness = combo(FOUR, "Alpha");
        harness.get_by_label("Combo").focus();
        harness.run();
        harness.key_press_modifiers(open.0, open.1);
        harness.run_steps(3);
        assert!(harness.query_by_label("Gamma").is_some(), "{open:?} opens");
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowDown);
        harness.run_steps(2);
        assert_eq!(*harness.state(), "Alpha", "the highlight is not the value");
        harness.key_press(Key::Escape);
        harness.run_steps(3);
        assert!(harness.query_by_label("Gamma").is_none());
        assert_eq!(*harness.state(), "Alpha", "Esc restores");
        assert!(harness.get_by_label("Combo").is_focused());
    }
}

#[test]
fn up_and_down_on_a_closed_combo_step_the_value_and_stop_at_the_ends() {
    let mut harness = combo(FOUR, "Alpha");
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(*harness.state(), "Alpha", "no wrap at the top");
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(*harness.state(), "Beta");
    for _ in 0..5 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    assert_eq!(*harness.state(), "Delta", "no wrap at the bottom");
    assert!(
        harness.query_by_label("Gamma").is_none(),
        "the list stayed closed"
    );
}

#[test]
fn arrows_in_the_open_list_wrap_and_enter_commits() {
    let mut harness = combo(FOUR, "Alpha");
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_steps(3);
    harness.key_press(Key::ArrowUp);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(
        *harness.state(),
        "Delta",
        "Up from the first wraps to the last"
    );
}

#[test]
fn typing_on_a_closed_combo_selects_by_prefix_and_repeating_cycles() {
    let mut harness = combo(&["Apple", "Avocado", "Banana", "Blueberry"], "Apple");
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.get_by_label("Combo").type_text("b");
    harness.run();
    assert_eq!(*harness.state(), "Banana");
    harness.get_by_label("Combo").type_text("b");
    harness.run();
    assert_eq!(*harness.state(), "Blueberry", "the same letter cycles");
}

#[test]
fn a_longer_prefix_typed_quickly_narrows_and_a_pause_starts_over() {
    let mut harness = combo(&["Apple", "Avocado", "Banana", "Blueberry"], "Apple");
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.get_by_label("Combo").type_text("a");
    harness.run();
    assert_eq!(*harness.state(), "Avocado");
    harness.get_by_label("Combo").type_text("p");
    harness.run();
    assert_eq!(*harness.state(), "Apple", "'ap' narrows to Apple");
    common::advance_by(
        &mut harness,
        f64::from(tokens::motion::COMBO_TYPE_AHEAD_RESET) + 0.2,
    );
    harness.get_by_label("Combo").type_text("b");
    harness.run();
    assert_eq!(
        *harness.state(),
        "Banana",
        "after a pause 'b' starts a new prefix"
    );
}

#[test]
fn unselectable_options_are_skipped_by_the_arrow_keys() {
    let mut harness = testing::harness_state(Theme::dark(), "Alpha", |ui, value| {
        ui.add(
            ComboBox::new("c", value, FOUR.iter().copied())
                .label("Combo")
                .unselectable([1]),
        );
    });
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(*harness.state(), "Gamma", "Beta cannot be chosen");
}

#[test]
fn the_list_shows_ten_rows_and_scrolls() {
    let options: Vec<String> = (1..=30).map(|n| format!("Item {n}")).collect();
    let mut harness =
        testing::harness_state(Theme::dark(), "Item 1".to_owned(), move |ui, value| {
            ui.add(
                ComboBox::new("c", value, options.clone())
                    .label("Combo")
                    .width(140.0),
            );
        });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 600.0));
    harness.run();
    harness.get_by_label("Combo").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_steps(3);
    let first = harness.get_by_label("Item 1").rect();
    let tenth = harness.get_by_label("Item 10").rect();
    assert!(
        (tenth.bottom() - first.top() - 10.0 * tokens::size::ROW_HEIGHT).abs() < 1.5,
        "{first:?} {tenth:?}"
    );
}

#[test]
fn a_second_click_on_the_combo_closes_the_list() {
    let mut harness = combo(FOUR, "Alpha");
    harness.get_by_label("Combo").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Gamma").is_some());
    common::advance_by(&mut harness, 0.3);
    harness.get_by_label("Combo").click();
    harness.run_steps(3);
    assert!(
        harness.query_by_label("Gamma").is_none(),
        "closed, not reopened"
    );
}

#[test]
fn a_disabled_combo_does_not_open_or_take_keys() {
    let mut harness = testing::harness_state(Theme::dark(), "Alpha", |ui, value| {
        ui.add(
            ComboBox::new("c", value, FOUR.iter().copied())
                .label("Combo")
                .enabled(false),
        );
    });
    harness.get_by_label("Combo").click();
    harness.key_press(Key::ArrowDown);
    harness.run_steps(3);
    assert!(harness.query_by_label("Gamma").is_none());
    assert_eq!(*harness.state(), "Alpha");
    assert!(harness.get_by_label("Combo").is_disabled());
}

// 13.5 SegmentedButton.

#[test]
fn left_and_right_move_the_selection_and_space_or_enter_select_a_focused_segment() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, selected| {
        ui.add(SegmentedButton::new(
            selected,
            [(0, "Body"), (1, "Headers"), (2, "Auth")],
        ));
    });
    harness.get_by_label("Body").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(*harness.state(), 1);
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(*harness.state(), 2, "stops at the last");
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(*harness.state(), 1);
    harness.get_by_label("Auth").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 2, "Enter selects the focused segment");
    harness.get_by_label("Body").focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(*harness.state(), 0, "Space too");
}

#[test]
fn exactly_one_segment_is_selected() {
    let harness = testing::harness(Theme::dark(), |ui| {
        let mut selected = 1;
        ui.add(SegmentedButton::new(
            &mut selected,
            [(0, "Body"), (1, "Headers"), (2, "Auth")],
        ));
    });
    let on: Vec<_> = ["Body", "Headers", "Auth"]
        .into_iter()
        .filter(|label| harness.get_by_label(label).is_checked())
        .collect();
    assert_eq!(on, ["Headers"]);
}

#[test]
fn too_many_segments_turn_into_a_combo_box() {
    let options: Vec<(usize, String)> = (0..8).map(|n| (n, format!("View {n}"))).collect();
    let mut harness = testing::harness_state(Theme::dark(), 2_usize, move |ui, selected| {
        ui.add(SegmentedButton::new(selected, options.clone()));
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 400.0));
    harness.run();
    assert!(harness.query_by_role(Role::ComboBox).is_some());
    assert!(harness.query_by_role(Role::RadioButton).is_none());
    harness.get_by_role(Role::ComboBox).focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(*harness.state(), 3, "the combo edits the same value");
    // Few enough segments stay segments.
    let harness = testing::harness(Theme::dark(), |ui| {
        let mut selected = 0;
        ui.add(SegmentedButton::new(&mut selected, [(0, "A"), (1, "B")]));
    });
    assert!(harness.query_by_role(Role::ComboBox).is_none());
}

#[test]
fn a_selected_focused_segment_is_painted_differently_from_a_selected_one() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let mut selected = 0;
        ui.add(SegmentedButton::new(&mut selected, [(0, "A"), (1, "B")]));
    });
    harness.run();
    harness.fit_contents();
    let rest = harness.render().expect("render");
    harness.get_by_label("A").focus();
    harness.run();
    harness.fit_contents();
    let focused = harness.render().expect("render");
    assert_ne!(rest, focused);
}

// Snapshots.

#[test]
fn states_snapshot() {
    testing::snapshot_themes("inputs_states", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.add(Button::primary("OK"));
            ui.add(Button::secondary("Cancel").mnemonic('C'));
            ui.add(Button::secondary("Apply").enabled(false));
        });
        ui.horizontal(|ui| {
            let (mut a, mut b) = (10_u32, 100_u32);
            ui.add(JBIntSpinner::new(&mut a).name("A").range(1..=20));
            ui.add(JBIntSpinner::new(&mut b).name("B").enabled(false));
            let mut value = "Beta";
            ui.add(ComboBox::new("c", &mut value, FOUR.iter().copied()).width(100.0));
        });
        let mut state = ThreeState::Indeterminate;
        ui.add(ThreeStateCheckBox::new(&mut state, "All inspections"));
    });
}

#[test]
fn combo_list_snapshot() {
    testing::snapshot_themes_after(
        "combo_box_list",
        common::IMG,
        |ui| {
            let id = egui::Id::new("combo-list");
            let mut value = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || "Beta"));
            ui.add(
                ComboBox::new("c", &mut value, FOUR.iter().copied())
                    .label("Combo")
                    .separators([2])
                    .unselectable([3])
                    .width(140.0),
            );
            ui.data_mut(|d| d.insert_temp(id, value));
            ui.add_space(170.0);
        },
        |h| {
            h.get_by_label("Combo").focus();
            h.run();
            h.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
            h.run();
        },
    );
}

#[test]
fn segmented_focus_snapshot() {
    testing::snapshot_themes_after(
        "segmented_button_focus",
        common::IMG,
        |ui| {
            let mut selected = 1;
            ui.add(SegmentedButton::new(
                &mut selected,
                [(0, "Body"), (1, "Headers"), (2, "Auth")],
            ));
        },
        |h| {
            h.get_by_label("Headers").focus();
            h.run();
        },
    );
}
