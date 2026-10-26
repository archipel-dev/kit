//! `HelpTooltip` timing and layout (IntelliJ spec section 11.1), `GotItTooltip` (section
//! 11.2) and the progress indicators (section 13.8).

mod common;

use archipel_kit::host::{InMemoryProperties, PropertiesComponent};
use archipel_kit::ide::{
    GotItDismissal, GotItPosition, GotItResponse, GotItTooltip, HelpTooltip, HelpTooltipExt,
    HelpTooltipResponse,
};
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::ui::{AsyncProcessIcon, Button, ProgressBar, ProgressState};
use archipel_kit::{Theme, tokens};
use egui::{Key, pos2, vec2};

/// A button with `tooltip` on it, in a window with room for the tooltip.
fn owner(tooltip: HelpTooltip) -> testing::Harness<'static, HelpTooltipResponse> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        HelpTooltipResponse::default(),
        move |ui, state| {
            ui.add_space(40.0);
            let (_, out) = ui
                .add(Button::secondary("Owner"))
                .tooltip_with_response(tooltip.clone());
            let clicked = state.link_clicked | out.link_clicked;
            *state = out;
            state.link_clicked = clicked;
        },
    );
    harness.set_size(vec2(420.0, 300.0));
    harness.run_steps(3);
    harness
}

fn visible(harness: &testing::Harness<'_, HelpTooltipResponse>) -> bool {
    harness.state().visible
}

#[test]
fn a_help_tooltip_shows_500_ms_after_the_pointer_rests_on_the_owner() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'"));
    harness.get_by_label("Owner").hover();
    harness.step();
    common::advance_by(&mut harness, 0.4);
    assert!(!visible(&harness), "not after 400 ms");
    common::advance_by(&mut harness, 0.15);
    assert!(visible(&harness), "after 550 ms");
    assert!(harness.query_by_label("Run 'gallery'").is_some());
}

#[test]
fn moving_the_pointer_starts_the_help_tooltip_delay_over() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'"));
    let owner_rect = harness.get_by_label("Owner").rect();
    harness.hover_at(owner_rect.center());
    harness.step();
    common::advance_by(&mut harness, 0.4);
    harness.hover_at(owner_rect.center() + vec2(3.0, 0.0));
    harness.step();
    common::advance_by(&mut harness, 0.4);
    assert!(!visible(&harness), "the move restarted the wait");
    common::advance_by(&mut harness, 0.2);
    assert!(visible(&harness));
}

#[test]
fn an_immediate_help_tooltip_shows_without_the_delay() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'").immediate(true));
    harness.get_by_label("Owner").hover();
    harness.step();
    harness.step();
    assert!(visible(&harness));
}

#[test]
fn a_help_tooltip_hides_150_ms_after_the_pointer_leaves() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'"));
    harness.get_by_label("Owner").hover();
    harness.step();
    common::advance_by(&mut harness, 0.6);
    assert!(visible(&harness));
    harness.hover_at(pos2(400.0, 280.0));
    harness.step();
    common::advance_by(&mut harness, 0.1);
    assert!(visible(&harness), "still there after 100 ms");
    common::advance_by(&mut harness, 0.1);
    assert!(!visible(&harness), "gone after 200 ms");
}

#[test]
fn a_help_tooltip_stays_while_the_pointer_is_over_it() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'").hint("Runs the gallery app."));
    harness.get_by_label("Owner").hover();
    harness.step();
    common::advance_by(&mut harness, 0.6);
    assert!(visible(&harness));
    let tip = harness.get_by_label("Run 'gallery'").rect();
    harness.hover_at(tip.center());
    harness.step();
    common::advance_by(&mut harness, 2.0);
    assert!(visible(&harness), "so its link can be reached");
    harness.hover_at(pos2(400.0, 280.0));
    harness.step();
    common::advance_by(&mut harness, 0.3);
    assert!(
        !visible(&harness),
        "and goes once the pointer leaves it too"
    );
}

#[test]
fn a_one_line_help_tooltip_hides_itself_after_10_seconds() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'"));
    harness.get_by_label("Owner").hover();
    harness.step();
    common::advance_by(&mut harness, 0.6);
    common::advance_by(&mut harness, 9.0);
    assert!(visible(&harness), "9.6 s");
    common::advance_by(&mut harness, 1.0);
    assert!(!visible(&harness), "10.6 s");
}

#[test]
fn a_help_tooltip_with_a_description_hides_itself_after_30_seconds() {
    let mut harness = owner(HelpTooltip::new("Run").hint("Runs the selected configuration."));
    harness.get_by_label("Owner").hover();
    harness.step();
    common::advance_by(&mut harness, 0.6);
    common::advance_by(&mut harness, 25.0);
    assert!(visible(&harness), "25.6 s");
    common::advance_by(&mut harness, 5.5);
    assert!(!visible(&harness), "31 s");
}

#[test]
fn never_hide_on_timeout_keeps_the_help_tooltip() {
    let mut harness = owner(HelpTooltip::new("Run").never_hide_on_timeout(true));
    harness.get_by_label("Owner").hover();
    harness.step();
    common::advance_by(&mut harness, 0.6);
    common::advance_by(&mut harness, 60.0);
    assert!(visible(&harness));
}

#[test]
fn the_dismiss_delay_follows_the_content() {
    assert_eq!(HelpTooltip::new("A").dismiss_delay(), Some(10.0));
    assert_eq!(HelpTooltip::new("A").hint("B").dismiss_delay(), Some(30.0));
    assert_eq!(
        HelpTooltip::new("A").link("B", false).dismiss_delay(),
        Some(30.0)
    );
    assert_eq!(
        HelpTooltip::new("A")
            .never_hide_on_timeout(true)
            .dismiss_delay(),
        None
    );
}

#[test]
fn pressing_the_owner_hides_the_help_tooltip_until_the_pointer_returns() {
    let mut harness = owner(HelpTooltip::new("Run 'gallery'"));
    let center = harness.get_by_label("Owner").rect().center();
    harness.hover_at(center);
    harness.step();
    common::advance_by(&mut harness, 0.6);
    assert!(visible(&harness));
    harness.drag_at(center);
    harness.step();
    assert!(!visible(&harness));
    harness.drop_at(center);
    harness.step();
    common::advance_by(&mut harness, 2.0);
    assert!(
        !visible(&harness),
        "it does not come back while the pointer stays"
    );
    harness.hover_at(pos2(400.0, 280.0));
    harness.step();
    common::advance_by(&mut harness, 0.5);
    harness.hover_at(center);
    harness.step();
    common::advance_by(&mut harness, 0.6);
    assert!(visible(&harness), "after leaving and coming back");
}

#[test]
fn a_help_tooltip_explains_a_disabled_owner_after_the_delay() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add_space(40.0);
        ui.add(Button::primary("Deploy").enabled(false))
            .tooltip("Connect a cluster first");
    });
    harness.set_size(vec2(420.0, 300.0));
    harness.run_steps(3);
    harness.get_by_label("Deploy").hover();
    harness.step();
    common::advance_by(&mut harness, 0.3);
    assert!(harness.query_by_label("Connect a cluster first").is_none());
    common::advance_by(&mut harness, 0.4);
    assert!(harness.query_by_label("Connect a cluster first").is_some());
}

#[test]
fn help_tooltip_text_is_at_most_250_wide() {
    let long = "A very long description that has to wrap onto several lines. ".repeat(4);
    let mut harness = testing::harness(Theme::dark(), move |ui| {
        let tooltip = HelpTooltip::new("Title").hint(long.clone());
        HelpTooltip::frame(ui.ctx()).show(ui, |ui| tooltip.content(ui));
    });
    harness.run();
    let hint = harness.get_by_label_contains("A very long").rect();
    assert!(hint.width() <= 250.5, "{hint:?}");
    assert!(hint.height() > 30.0, "wrapped: {hint:?}");
}

#[test]
fn a_help_tooltip_link_is_reported_and_stays_clickable() {
    let mut harness = owner(
        HelpTooltip::new("Run")
            .hint("Runs it.")
            .link("Learn more", true),
    );
    let owner_rect = harness.get_by_label("Owner").rect();
    harness.hover_at(owner_rect.center());
    harness.step();
    common::advance_by(&mut harness, 0.6);
    let link = harness.get_by_label("Learn more");
    assert_eq!(link.role(), Role::Link);
    // The pointer moves from the owner to the link, across the gap between them.
    harness.hover_at(link.rect().center());
    harness.step();
    common::advance_by(&mut harness, 0.05);
    harness.get_by_label("Learn more").click();
    harness.step();
    harness.step();
    assert!(harness.state().link_clicked);
}

#[test]
fn help_tooltip_layout_snapshot() {
    testing::snapshot_themes("help_tooltip_link", common::IMG, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        for tooltip in [
            HelpTooltip::new("Pause Indexing").shortcut("Ctrl+Alt+P"),
            HelpTooltip::new("Hot Reload")
                .shortcut("Ctrl+F10")
                .hint("Applies code changes to the running application without a restart.")
                .link("Learn more", true),
        ] {
            HelpTooltip::frame(ui.ctx()).show(ui, |ui| tooltip.content(ui));
        }
    });
}

// ---- GotItTooltip ----

#[derive(Default)]
struct Tip {
    response: GotItResponse,
    shown: u32,
    dismissed: Option<GotItDismissal>,
    link_clicked: bool,
    secondary_clicked: bool,
}

/// A target button with got-it tooltips pointing at it. `should_show` is the state of
/// the test.
fn got_it(tips: Vec<GotItTooltip>, should_show: bool) -> testing::Harness<'static, (Tip, bool)> {
    let mut harness = testing::harness_state_sized(
        Theme::dark(),
        vec2(720.0, 360.0),
        (Tip::default(), should_show),
        move |ui, (tip, should_show)| {
            ui.add_space(120.0);
            ui.horizontal(|ui| {
                ui.add_space(180.0);
                let target = ui.add(Button::secondary("Run configuration")).rect;
                for tooltip in &tips {
                    let response = tooltip.show(ui.ctx(), target, *should_show);
                    if response.newly_shown {
                        tip.shown += 1;
                    }
                    if response.dismissed.is_some() {
                        tip.dismissed = response.dismissed;
                        *should_show = false;
                    }
                    tip.link_clicked |= response.link_clicked;
                    tip.secondary_clicked |= response.secondary_clicked;
                    tip.response = response;
                }
            });
        },
    );
    harness.run_steps(4);
    harness
}

fn tip() -> GotItTooltip {
    GotItTooltip::new("run.config", "Pick the configuration to run here.")
        .header("Run configurations")
}

#[test]
fn a_got_it_tooltip_shows_while_should_show_is_set() {
    let mut harness = got_it(vec![tip()], true);
    assert!(harness.query_by_label("Run configurations").is_some());
    assert!(
        harness
            .query_by_label("Pick the configuration to run here.")
            .is_some()
    );
    assert!(harness.query_by_label("Got it").is_some());
    harness.state_mut().1 = false;
    harness.run_steps(3);
    assert!(harness.query_by_label("Got it").is_none());
}

#[test]
fn a_got_it_tooltip_reports_that_it_appeared_once() {
    let harness = got_it(vec![tip()], true);
    assert_eq!(harness.state().0.shown, 1);
}

#[test]
fn the_got_it_button_dismisses_the_tooltip() {
    let mut harness = got_it(vec![tip()], true);
    harness.get_by_label("Got it").click();
    harness.run_steps(3);
    assert_eq!(harness.state().0.dismissed, Some(GotItDismissal::Button));
    assert!(harness.query_by_label("Got it").is_none());
}

#[test]
fn escape_closes_the_got_it_tooltip() {
    let mut harness = got_it(vec![tip()], true);
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert_eq!(harness.state().0.dismissed, Some(GotItDismissal::Escape));
}

#[test]
fn a_click_elsewhere_does_not_close_the_got_it_tooltip() {
    let mut harness = got_it(vec![tip()], true);
    let at = pos2(20.0, 340.0);
    harness.hover_at(at);
    harness.step();
    harness.drag_at(at);
    harness.step();
    harness.drop_at(at);
    harness.run_steps(3);
    assert_eq!(harness.state().0.dismissed, None);
    assert!(harness.query_by_label("Got it").is_some());
}

#[test]
fn a_got_it_tooltip_with_a_timeout_has_no_button_and_closes_after_5_seconds() {
    let mut harness = got_it(vec![tip().with_timeout()], true);
    assert!(harness.query_by_label("Got it").is_none(), "no button");
    common::advance_by(&mut harness, 4.5);
    assert_eq!(harness.state().0.dismissed, None);
    common::advance_by(&mut harness, 1.0);
    assert_eq!(harness.state().0.dismissed, Some(GotItDismissal::Timeout));
}

#[test]
fn a_got_it_tooltip_shows_its_step_and_link() {
    let mut harness = got_it(vec![tip().step(2).link("Learn more")], true);
    assert!(harness.query_by_label("02").is_some(), "the step counter");
    harness.get_by_label("Learn more").click();
    harness.run_steps(3);
    assert!(harness.state().0.link_clicked);
    assert!(
        harness.query_by_label("Got it").is_some(),
        "the link leaves the tooltip open"
    );
}

#[test]
fn a_secondary_button_is_reported() {
    let mut harness = got_it(vec![tip().secondary_button("Skip tour")], true);
    harness.get_by_label("Skip tour").click();
    harness.run_steps(3);
    assert!(harness.state().0.secondary_clicked);
}

#[test]
fn the_got_it_button_can_take_the_focus() {
    let harness = got_it(vec![tip().focus(true)], true);
    assert!(harness.get_by_label("Got it").is_focused());
}

#[test]
fn only_one_got_it_tooltip_shows_at_a_time() {
    let second = GotItTooltip::new("second", "Second tip").header("Second header");
    let harness = got_it(vec![tip(), second], true);
    assert!(harness.query_by_label("Run configurations").is_some());
    assert!(
        harness.query_by_label("Second header").is_none(),
        "it waits for the first"
    );
}

#[test]
fn got_it_positions_put_the_bubble_around_the_target() {
    let target = |harness: &testing::Harness<'_, (Tip, bool)>| {
        harness.get_by_label("Run configuration").rect()
    };
    let cases = [
        (GotItPosition::TopMiddle, "above"),
        (GotItPosition::BottomMiddle, "below"),
        (GotItPosition::LeftMiddle, "left"),
        (GotItPosition::RightMiddle, "right"),
        (GotItPosition::BottomLeft, "below"),
    ];
    for (position, side) in cases {
        let harness = got_it(vec![tip().position(position)], true);
        let target = target(&harness);
        let body = harness.get_by_label("Run configurations").rect();
        match side {
            "above" => assert!(body.bottom() < target.top(), "{position:?}"),
            "below" => assert!(body.top() > target.bottom(), "{position:?}"),
            "left" => assert!(body.right() < target.left(), "{position:?}"),
            _ => assert!(body.left() > target.right(), "{position:?}"),
        }
    }
}

#[test]
fn got_it_text_is_at_most_280_wide() {
    let long = "A got-it text that is long enough to need several lines to fit. ".repeat(4);
    let harness = got_it(vec![GotItTooltip::new("long", long)], true);
    let text = harness.get_by_label_contains("A got-it text").rect();
    assert!(text.width() <= 280.5, "{text:?}");
}

#[test]
fn got_it_state_lives_in_the_properties_component() {
    let tooltip = tip();
    let mut props = InMemoryProperties::new();
    assert!(tooltip.can_show(&props));
    tooltip.record_shown(&mut props);
    assert!(
        !tooltip.can_show(&props),
        "shown once, one show allowed by default"
    );
    tooltip.reset(&mut props);
    assert!(tooltip.can_show(&props));
    tooltip.got_it(&mut props);
    assert!(!tooltip.can_show(&props));
    assert!(props.get_bool("gotit.run.config.done", false));
}

#[test]
fn show_count_allows_several_showings_without_a_dismissal() {
    let tooltip = tip().show_count(3);
    let mut props = InMemoryProperties::new();
    for _ in 0..3 {
        assert!(tooltip.can_show(&props));
        tooltip.record_shown(&mut props);
    }
    assert!(!tooltip.can_show(&props));
}

#[test]
fn show_with_counts_the_showing_and_remembers_the_dismissal() {
    let mut harness = testing::harness_state_sized(
        Theme::dark(),
        vec2(520.0, 360.0),
        InMemoryProperties::new(),
        |ui, props| {
            ui.add_space(120.0);
            let target = ui.add(Button::secondary("Run configuration")).rect;
            tip().show_with(ui.ctx(), target, props);
        },
    );
    harness.run_steps(4);
    assert_eq!(harness.state().get_int("gotit.run.config.shown", 0), 1);
    assert!(harness.query_by_label("Got it").is_some());
    harness.get_by_label("Got it").click();
    harness.run_steps(3);
    assert!(harness.state().get_bool("gotit.run.config.done", false));
    assert!(harness.query_by_label("Got it").is_none(), "never again");
}

#[test]
fn got_it_snapshot() {
    testing::snapshot_themes_sized("got_it_tooltip", common::IMG, vec2(440.0, 250.0), |ui| {
        ui.add_space(130.0);
        ui.horizontal(|ui| {
            ui.add_space(150.0);
            let target = ui.add(Button::secondary("Run configuration")).rect;
            GotItTooltip::new(
                "snapshot",
                "Pick the configuration to run, or edit it here.",
            )
            .header("Run configurations")
            .step(1)
            .link("Learn more")
            .position(GotItPosition::TopMiddle)
            .show(ui.ctx(), target, true);
        });
    });
}

// ---- Progress ----

#[test]
fn the_async_process_icon_advances_one_frame_every_125_ms() {
    assert_eq!(AsyncProcessIcon::frame_at(0.0), 0);
    assert_eq!(AsyncProcessIcon::frame_at(0.124), 0);
    assert_eq!(AsyncProcessIcon::frame_at(0.125), 1);
    assert_eq!(AsyncProcessIcon::frame_at(0.9), 7);
    assert_eq!(AsyncProcessIcon::frame_at(1.0), 0, "eight frames per turn");
    assert_eq!(tokens::motion::ASYNC_PROCESS_FRAMES, 8);
}

#[test]
fn an_indeterminate_segment_slides_across_the_track() {
    let period = f64::from(tokens::motion::PROGRESS_INDETERMINATE_PERIOD);
    let (start, end) = ProgressBar::indeterminate_segment(100.0, 0.0);
    assert_eq!((start, end), (0.0, 0.0), "it enters from the left edge");
    let (start, end) = ProgressBar::indeterminate_segment(100.0, period * 0.5);
    assert!(start > 0.0 && end < 100.0 && (end - start - 30.0).abs() < 0.01);
    let (start, end) = ProgressBar::indeterminate_segment(100.0, period * 0.999);
    assert!(
        start > 90.0 && (end - 100.0).abs() < 0.01,
        "it leaves on the right"
    );
    assert_eq!(
        ProgressBar::indeterminate_segment(100.0, period * 3.25),
        ProgressBar::indeterminate_segment(100.0, period * 0.25),
        "it repeats every period"
    );
}

#[test]
fn progress_states_snapshot() {
    testing::snapshot_themes("progress_states", common::IMG, |ui| {
        ui.add(ProgressBar::new(Some(0.6)));
        ui.add(ProgressBar::new(Some(1.0)).state(ProgressState::Passed));
        ui.add(ProgressBar::new(Some(0.7)).state(ProgressState::Warning));
        ui.add(ProgressBar::new(Some(0.3)).state(ProgressState::Failed));
    });
}

#[test]
fn a_progress_bar_is_8_high_with_a_4_high_track() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(ProgressBar::new(Some(0.5)).label("Download"));
    });
    let bar = harness.get_by_label("Download").rect();
    assert_eq!(bar.height(), 8.0);
    assert_eq!(tokens::size::PROGRESS_BAR_HEIGHT, 4.0);
}
