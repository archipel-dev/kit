//! The scroll bar of collections: hover fade timing, dragging and paging (IntelliJ spec
//! section 13.1).

mod common;

use archipel_kit::Theme;
use archipel_kit::parts::ScrollBar;
use archipel_kit::testing;
use egui::{Event, PointerButton, Pos2, Rect, ScrollArea};

struct State {
    /// (time, hover factor) of every frame.
    log: Vec<(f64, f32)>,
    offset: f32,
    viewport: Rect,
}

impl Default for State {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            offset: 0.0,
            viewport: Rect::NOTHING,
        }
    }
}

fn harness() -> testing::Harness<'static, State> {
    let mut harness = testing::harness_state(Theme::dark(), State::default(), |ui, state| {
        let bar = ScrollBar::new("log");
        let area = bar.prepare(ui, ScrollArea::vertical().id_salt("log").max_height(100.0));
        let out = area.show(ui, |ui| {
            for line in 0..100 {
                ui.label(format!("line {line}"));
            }
        });
        bar.show(ui, &out);
        state.offset = out.state.offset.y;
        state.viewport = out.inner_rect;
        let time = ui.input(|i| i.time);
        state
            .log
            .push((time, ScrollBar::hover_factor(ui.ctx(), bar.id())));
    });
    harness.run_ok();
    harness
}

fn strip_point(harness: &testing::Harness<'_, State>, y_from_top: f32) -> Pos2 {
    let view = harness.state().viewport;
    Pos2::new(view.right() - 5.0, view.top() + y_from_top)
}

/// The time of the first logged frame at or after `from` whose factor satisfies `test`.
fn first(log: &[(f64, f32)], from: usize, test: impl Fn(f32) -> bool) -> Option<f64> {
    log[from..]
        .iter()
        .find(|(_, factor)| test(*factor))
        .map(|(time, _)| *time)
}

#[test]
fn the_thumb_waits_150_ms_then_brightens_over_125_ms() {
    let mut harness = harness();
    assert_eq!(harness.state().log.last().map(|l| l.1), Some(0.0));
    let start = harness.state().log.len();
    let pos = strip_point(&harness, 50.0);
    harness.hover_at(pos);
    harness.run_steps(40);
    let log = &harness.state().log;
    // The pointer is seen on the first frame after the event.
    let entered = log[start].0;
    let began = first(log, start, |f| f > 0.0).expect("it brightens");
    let done = first(log, start, |f| f >= 0.999).expect("it gets there");
    let frame = 1.0 / 60.0;
    assert!(
        (began - entered - 0.150).abs() < 2.0 * frame,
        "starts after 150 ms, not {:.0} ms",
        (began - entered) * 1000.0
    );
    assert!(
        (done - began - 0.125).abs() < 3.0 * frame,
        "takes 125 ms, not {:.0} ms",
        (done - began) * 1000.0
    );
}

#[test]
fn the_thumb_waits_300_ms_after_the_pointer_left_then_dims_over_125_ms() {
    let mut harness = harness();
    let pos = strip_point(&harness, 50.0);
    harness.hover_at(pos);
    harness.run_steps(40);
    assert!(harness.state().log.last().is_some_and(|l| l.1 >= 0.999));
    let start = harness.state().log.len();
    harness.hover_at(Pos2::new(10.0, 10.0));
    harness.run_steps(50);
    let log = &harness.state().log;
    let left = log[start].0;
    let began = first(log, start, |f| f < 1.0).expect("it dims");
    let done = first(log, start, |f| f <= 0.001).expect("it gets there");
    let frame = 1.0 / 60.0;
    assert!(
        (began - left - 0.300).abs() < 2.0 * frame,
        "starts after 300 ms, not {:.0} ms",
        (began - left) * 1000.0
    );
    assert!(
        (done - began - 0.125).abs() < 3.0 * frame,
        "takes 125 ms, not {:.0} ms",
        (done - began) * 1000.0
    );
}

#[test]
fn leaving_before_the_delay_never_brightens_it() {
    let mut harness = harness();
    let start = harness.state().log.len();
    harness.hover_at(strip_point(&harness, 50.0));
    harness.run_steps(5);
    harness.hover_at(Pos2::new(10.0, 10.0));
    harness.run_steps(40);
    let log = &harness.state().log;
    assert!(log[start..].iter().all(|(_, factor)| *factor == 0.0));
}

fn press(harness: &testing::Harness<'_, State>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

#[test]
fn dragging_the_thumb_scrolls_the_content() {
    let mut harness = harness();
    assert_eq!(harness.state().offset, 0.0);
    let from = strip_point(&harness, 4.0);
    harness.hover_at(from);
    harness.run_steps(2);
    press(&harness, from, true);
    harness.run_steps(2);
    for step in 1..=8 {
        harness.hover_at(Pos2::new(from.x, from.y + 5.0 * step as f32));
        harness.run_steps(2);
    }
    harness.run_steps(2);
    let dragged = harness.state().offset;
    assert!(dragged > 100.0, "the content followed the thumb: {dragged}");
    press(&harness, Pos2::new(from.x, from.y + 40.0), false);
    harness.run_steps(2);
    let released = harness.state().offset;
    harness.run_steps(2);
    assert_eq!(harness.state().offset, released, "released: it stays");
}

#[test]
fn clicking_the_track_scrolls_one_page_toward_the_pointer() {
    let mut harness = harness();
    let pos = strip_point(&harness, 90.0);
    harness.hover_at(pos);
    harness.run();
    press(&harness, pos, true);
    press(&harness, pos, false);
    harness.run();
    harness.run();
    let offset = harness.state().offset;
    let page = harness.state().viewport.height();
    assert!(
        (offset - page).abs() < 1.0,
        "one page ({page}) down, got {offset}"
    );
}

#[test]
fn a_short_content_has_no_bar() {
    let mut harness = testing::harness_state(Theme::dark(), 0.0_f32, |ui, factor| {
        let bar = ScrollBar::new("short");
        let area = bar.prepare(
            ui,
            ScrollArea::vertical().id_salt("short").max_height(100.0),
        );
        let out = area.show(ui, |ui| {
            ui.label("only line");
        });
        bar.show(ui, &out);
        *factor = ScrollBar::hover_factor(ui.ctx(), bar.id());
    });
    harness.run_ok();
    harness.run_steps(30);
    assert_eq!(*harness.state(), 0.0);
}
