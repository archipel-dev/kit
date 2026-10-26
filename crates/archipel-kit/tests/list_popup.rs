//! `ListPopup` / `PopupStep`: the IntelliJ list popup rules (spec §5) as tests.

mod common;

use archipel_kit::testing::{self, Harness, NodeExt, Queryable};
use archipel_kit::ui::popup::{BaseListPopupStep, ListPopup, PopupStepResult};
use archipel_kit::{Glyph, Theme};
use egui::{Key, Modifiers, pos2, vec2};

struct State {
    popup: ListPopup<String>,
    chosen: Vec<String>,
    cancelled: u32,
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).to_owned()).collect()
}

fn harness_with(step: BaseListPopupStep<String>, hover: bool) -> Harness<'static, State> {
    let state = State {
        popup: ListPopup::new("list", step).show_submenu_on_hover(hover),
        chosen: Vec::new(),
        cancelled: 0,
    };
    let mut harness =
        testing::harness_state_sized(Theme::dark(), vec2(500.0, 400.0), state, |ui, state| {
            let anchor = egui::Rect::from_min_size(pos2(20.0, 20.0), vec2(80.0, 24.0));
            let response = state.popup.show(ui.ctx(), anchor);
            if let Some(value) = response.chosen {
                state.chosen.push(value);
            }
            if response.cancelled {
                state.cancelled += 1;
            }
        });
    harness.run();
    let ctx = harness.ctx.clone();
    harness.state_mut().popup.open(&ctx);
    harness.run();
    harness
}

fn plain() -> BaseListPopupStep<String> {
    BaseListPopupStep::new("", names(&["Alpha", "Beta", "Gamma", "Delta"]))
}

/// "Alpha", a separator, "Beta" (disabled), "Gamma".
fn with_gaps() -> BaseListPopupStep<String> {
    BaseListPopupStep::new("", names(&["Alpha", "Beta", "Gamma", "Delta"]))
        .separator_before(1)
        .unselectable(1)
}

fn selected(harness: &Harness<'_, State>, depth: usize) -> Option<usize> {
    harness.state().popup.chain().selected(depth)
}

/// A step with a sub-step on "More".
fn nested() -> BaseListPopupStep<String> {
    BaseListPopupStep::new("", names(&["One", "More", "Last"]))
        .has_substep(|v: &String| v == "More")
        .on_chosen(|value: &String, final_choice| {
            if value == "More" && !final_choice {
                PopupStepResult::Next(Box::new(BaseListPopupStep::new(
                    "",
                    names(&["Inner A", "Inner B"]),
                )))
            } else {
                PopupStepResult::FinalChoice
            }
        })
}

#[test]
fn opens_with_the_first_selectable_row_selected() {
    let harness = harness_with(plain(), false);
    assert_eq!(selected(&harness, 0), Some(0));
}

#[test]
fn default_option_index_is_selected_on_open() {
    let harness = harness_with(plain().default_index(2), false);
    assert_eq!(selected(&harness, 0), Some(2));
}

#[test]
fn arrows_wrap_around_and_skip_disabled_rows() {
    let mut harness = harness_with(with_gaps(), false);
    assert_eq!(selected(&harness, 0), Some(0));
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(2), "skips the disabled row");
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(0), "wraps from last to first");
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(3), "wraps from first to last");
}

#[test]
fn home_and_end_select_the_first_and_last_selectable_rows() {
    let mut harness = harness_with(with_gaps(), false);
    harness.key_press(Key::End);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(3));
    harness.key_press(Key::Home);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(0));
}

#[test]
fn page_keys_move_by_a_page_and_stop_at_the_ends() {
    let many: Vec<String> = (0..40).map(|i| format!("Row {i}")).collect();
    let mut harness = harness_with(BaseListPopupStep::new("", many), false);
    harness.key_press(Key::PageDown);
    harness.run();
    let after = selected(&harness, 0).expect("selected");
    assert!(after > 1, "a page is more than one row");
    harness.key_press(Key::PageUp);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(0));
    harness.key_press(Key::PageUp);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(0), "does not wrap");
}

#[test]
fn enter_chooses_the_selected_row_and_closes() {
    let mut harness = harness_with(plain(), false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().chosen, ["Beta"]);
    assert!(!harness.state().popup.is_open());
    assert!(harness.query_by_label("Alpha").is_none());
}

#[test]
fn enter_on_a_row_with_a_substep_opens_it_instead_of_finishing() {
    let mut harness = harness_with(nested(), false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().chosen.is_empty());
    assert!(harness.query_by_label("Inner A").is_some());
    assert_eq!(harness.state().popup.chain().depth(), 2);
    assert_eq!(
        selected(&harness, 1),
        Some(0),
        "the child selects its first row"
    );
}

#[test]
fn right_opens_the_substep_and_left_goes_back() {
    let mut harness = harness_with(nested(), false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.query_by_label("Inner B").is_some());
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(harness.query_by_label("Inner B").is_none());
    assert!(harness.query_by_label("More").is_some());
    assert_eq!(selected(&harness, 0), Some(1), "the parent keeps its row");
}

#[test]
fn right_on_a_plain_row_does_nothing() {
    let mut harness = harness_with(nested(), false);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().popup.chain().depth(), 1);
}

#[test]
fn escape_goes_back_one_level_then_closes() {
    let mut harness = harness_with(nested(), false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().popup.is_open());
    assert_eq!(harness.state().popup.chain().depth(), 1);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().popup.is_open());
    assert_eq!(harness.state().cancelled, 1);
}

#[test]
fn shift_escape_closes_the_whole_chain() {
    let mut harness = harness_with(nested(), false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Escape);
    harness.run();
    assert!(!harness.state().popup.is_open());
    assert_eq!(harness.state().cancelled, 1);
}

#[test]
fn first_pointer_move_after_opening_is_ignored() {
    let mut harness = harness_with(plain(), false);
    let beta = harness.get_by_label("Beta").rect().center();
    harness.hover_at(beta);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(0), "the first move is ignored");
    harness.hover_at(beta + vec2(2.0, 0.0));
    harness.run();
    assert_eq!(selected(&harness, 0), Some(1), "the next one selects");
}

#[test]
fn hover_selects_the_row_and_a_separator_clears_it() {
    let mut harness = harness_with(with_gaps(), false);
    let gamma = harness.get_by_label("Gamma").rect().center();
    harness.hover_at(gamma - vec2(30.0, 0.0));
    harness.run();
    harness.hover_at(gamma);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(2));
    // The disabled row is not selectable: hovering it clears the selection.
    let beta = harness.get_by_label("Beta").rect().center();
    harness.hover_at(beta);
    harness.run();
    assert_eq!(selected(&harness, 0), None);
}

#[test]
fn unchanged_pointer_position_does_not_move_the_selection() {
    let mut harness = harness_with(plain(), false);
    let beta = harness.get_by_label("Beta").rect().center();
    harness.hover_at(beta - vec2(5.0, 0.0));
    harness.hover_at(beta);
    harness.run();
    assert_eq!(selected(&harness, 0), Some(1));
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.run();
    assert_eq!(
        selected(&harness, 0),
        Some(2),
        "a still pointer does not take it back"
    );
}

#[test]
fn click_chooses_on_release() {
    let mut harness = harness_with(plain(), false);
    harness.get_by_label("Gamma").click();
    harness.run();
    assert_eq!(harness.state().chosen, ["Gamma"]);
    assert!(!harness.state().popup.is_open());
}

#[test]
fn press_outside_cancels() {
    let mut harness = harness_with(plain(), false);
    harness.event(egui::Event::PointerMoved(pos2(450.0, 350.0)));
    harness.event(egui::Event::PointerButton {
        pos: pos2(450.0, 350.0),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(!harness.state().popup.is_open());
    assert_eq!(harness.state().cancelled, 1);
}

#[test]
fn reopening_within_200_ms_of_closing_is_refused() {
    let mut harness = harness_with(plain(), false);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().popup.is_open());
    let ctx = harness.ctx.clone();
    assert!(!harness.state_mut().popup.open(&ctx), "guard: just closed");
    // 200 ms is 12 frames at 60 fps.
    harness.run_steps(14);
    assert!(
        harness.state_mut().popup.open(&ctx),
        "open again after the guard"
    );
}

#[test]
fn substep_opens_after_250_ms_of_hover_when_enabled() {
    let mut harness = harness_with(nested(), true);
    let more = harness.get_by_label("More").rect().center();
    harness.hover_at(more - vec2(10.0, 0.0));
    harness.run();
    harness.hover_at(more);
    harness.run();
    harness.run_steps(8); // about 133 ms
    assert!(harness.query_by_label("Inner A").is_none(), "not yet");
    harness.run_steps(10); // past 250 ms
    assert!(
        harness.query_by_label("Inner A").is_some(),
        "after the delay"
    );
}

#[test]
fn substep_does_not_open_on_hover_by_default() {
    let mut harness = harness_with(nested(), false);
    let more = harness.get_by_label("More").rect().center();
    harness.hover_at(more - vec2(10.0, 0.0));
    harness.run();
    harness.hover_at(more);
    harness.run();
    harness.run_steps(30);
    assert!(harness.query_by_label("Inner A").is_none());
}

#[test]
fn leaving_the_row_before_the_delay_cancels_the_submenu() {
    let mut harness = harness_with(nested(), true);
    let more = harness.get_by_label("More").rect().center();
    let one = harness.get_by_label("One").rect().center();
    harness.hover_at(more - vec2(10.0, 0.0));
    harness.run();
    harness.hover_at(more);
    harness.run_steps(6);
    harness.hover_at(one);
    harness.run_steps(30);
    assert!(harness.query_by_label("Inner A").is_none());
}

#[test]
fn moving_toward_an_open_submenu_does_not_retarget_the_selection() {
    let mut harness = harness_with(nested(), true);
    let more = harness.get_by_label("More").rect();
    harness.hover_at(more.center() - vec2(10.0, 0.0));
    harness.run();
    harness.hover_at(more.center());
    harness.run_steps(20);
    assert!(harness.query_by_label("Inner A").is_some());
    let child = harness.state().popup.chain().rect(1).expect("child rect");
    // Along the diagonal toward the child, across "Last".
    let last = harness.get_by_label("Last").rect();
    let from = more.center();
    let to = pos2(child.left() + 4.0, last.center().y);
    for step in 1..=4 {
        let t = step as f32 / 4.0;
        harness.hover_at(from + (to - from) * t);
        harness.run();
    }
    assert_eq!(selected(&harness, 0), Some(1), "still on the submenu's row");
    assert!(harness.query_by_label("Inner A").is_some());
}

#[test]
fn auto_selection_opens_the_selected_substep_after_500_ms_idle() {
    let mut harness = harness_with(nested().auto_selection(true), false);
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.run_steps(20); // about 333 ms
    assert!(harness.query_by_label("Inner A").is_none());
    harness.run_steps(20);
    assert!(harness.query_by_label("Inner A").is_some());
}

#[test]
fn moving_restarts_the_auto_selection_timer() {
    let mut harness = harness_with(nested().auto_selection(true), false);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(24); // 400 ms
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowUp);
    harness.run_steps(24);
    assert!(
        harness.query_by_label("Inner A").is_none(),
        "restarted by the keys"
    );
    harness.run_steps(12);
    assert!(harness.query_by_label("Inner A").is_some());
}

#[test]
fn chevron_press_opens_the_substep_without_choosing() {
    let mut harness = harness_with(nested(), false);
    let more = harness.get_by_label("More").rect();
    let p = pos2(more.right() - 14.0, more.center().y);
    harness.event(egui::Event::PointerMoved(p));
    harness.event(egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(
        harness.query_by_label("Inner A").is_some(),
        "opened on press"
    );
    assert!(harness.state().chosen.is_empty());
}

#[test]
fn mnemonics_choose_the_row_when_typed() {
    let mut harness = harness_with(plain().mnemonics(true), false);
    assert!(harness.query_by_label("Gamma").is_some());
    harness.event(egui::Event::Text("3".to_owned()));
    harness.run();
    assert_eq!(harness.state().chosen, ["Gamma"]);
}

#[test]
fn typing_filters_with_speed_search_and_escape_clears_it_first() {
    let mut harness = harness_with(plain().speed_search(true), false);
    harness.event(egui::Event::Text("ta".to_owned()));
    harness.run();
    assert!(harness.query_by_label("Beta").is_some());
    assert!(harness.query_by_label("Delta").is_some());
    assert!(harness.query_by_label("Alpha").is_none(), "filtered out");
    assert_eq!(harness.state().popup.chain().query(0), "ta");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(
        harness.state().popup.is_open(),
        "Esc clears the search first"
    );
    assert!(harness.query_by_label("Alpha").is_some());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().popup.is_open());
}

#[test]
fn rows_are_named_and_not_tab_stops() {
    let mut harness = harness_with(plain().icons(|_: &String| Some(Glyph::File.into())), false);
    assert!(!harness.get_by_label("Alpha").is_disabled());
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn popup_snapshot() {
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let step =
            BaseListPopupStep::new("Branches", names(&["main", "dev", "release/1.2", "More…"]))
                .icons(|_: &String| Some(Glyph::Branch.into()))
                .separator_before(3)
                .has_substep(|v: &String| v == "More…");
        let state = State {
            popup: ListPopup::new("list", step),
            chosen: Vec::new(),
            cancelled: 0,
        };
        let mut harness =
            testing::harness_state_sized(theme, vec2(260.0, 190.0), state, |ui, s| {
                common::backdrop(ui.ctx());
                s.popup.show_at(ui.ctx(), pos2(8.0, 8.0));
            });
        harness.run();
        let ctx = harness.ctx.clone();
        harness.state_mut().popup.open(&ctx);
        harness.run();
        harness
            .try_snapshot_options(
                format!("list_popup.{suffix}"),
                &testing::SnapshotOptions::new().output_path(common::IMG),
            )
            .expect("snapshot matches");
    }
}
