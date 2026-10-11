//! `Histogram` and `Sparkline` tests.

mod common;

use archipel_kit::components::{Bucket, ChartColor, Histogram, Sparkline};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Theme, Tone, cursor, egui};
use egui::{CursorIcon, Key};

fn price() -> Vec<Bucket> {
    vec![
        Bucket::range(3.0, 20.0, 31),
        Bucket::range(20.0, 40.0, 148),
        Bucket::range(40.0, 60.0, 96),
        Bucket::range(60.0, 980.0, 12),
    ]
}

fn top_n() -> Vec<Bucket> {
    vec![
        Bucket::new("Berlin", 482),
        Bucket::new("Hamburg", 301),
        Bucket::new("Munich", 255),
        Bucket::new("A very long city name that has to be truncated", 40),
        Bucket::new("Köln", 0),
    ]
}

const PRICE: &str = "Histogram of price: 4 buckets, min 3, max 980, mode 20–40";

#[test]
fn histogram_conforms() {
    let buckets = price();
    testing::conformance(PRICE, |ui, enabled| {
        Histogram::new("price", &buckets)
            .selectable(true)
            .enabled(enabled)
            .show(ui)
            .response
    })
    .run();
}

#[test]
fn horizontal_histogram_conforms() {
    let buckets = top_n();
    let label = "Histogram of city: 5 buckets, mode Berlin";
    testing::conformance(label, |ui, enabled| {
        Histogram::new("city", &buckets)
            .horizontal(true)
            .selectable(true)
            .enabled(enabled)
            .show(ui)
            .response
    })
    .keys(&[Key::Enter, Key::Space])
    .run();
}

#[test]
fn charts_pass_the_audit() {
    let (buckets, cities) = (price(), top_n());
    let values = [3.0, 4.5, 4.0, 9.0, 7.5];
    for theme in [Theme::dark(), Theme::light()] {
        let mut harness = testing::harness(theme, |ui| {
            Histogram::new("price", &buckets).selectable(true).show(ui);
            Histogram::new("city", &cities).horizontal(true).show(ui);
            Histogram::new("none", &[]).show(ui);
            ui.horizontal(|ui| {
                ui.label("latency");
                ui.add(
                    Sparkline::new("latency", &values)
                        .last_marker(true)
                        .tooltip(true),
                );
                ui.add(Sparkline::new("empty", &[]));
            });
        });
        harness.run();
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn histogram_summary_is_the_accessible_name() {
    let buckets = price();
    let harness = testing::harness(Theme::dark(), |ui| {
        Histogram::new("price", &buckets).show(ui);
        Histogram::new("empty", &[]).show(ui);
        Histogram::new("city", &top_n()).show(ui);
    });
    assert!(harness.query_by_label(PRICE).is_some());
    assert!(
        harness
            .query_by_label("Histogram of empty: no data")
            .is_some()
    );
    assert!(
        harness
            .query_by_label("Histogram of city: 5 buckets, mode Berlin")
            .is_some()
    );
}

#[test]
fn sparkline_summary_is_the_accessible_name() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.add(Sparkline::new("query time", &[3.2, 18.0, 7.1]));
        ui.add(Sparkline::new("one", &[5.0]));
        ui.add(Sparkline::new("gaps", &[f32::NAN, 2.0, f32::INFINITY, 4.0]));
        ui.add(Sparkline::new("none", &[]));
    });
    harness.get_by_label("Sparkline of query time: 3 points, min 3.2, max 18, last 7.1");
    harness.get_by_label("Sparkline of one: 1 point, min 5, max 5, last 5");
    harness.get_by_label("Sparkline of gaps: 4 points, min 2, max 4, last 4");
    harness.get_by_label("Sparkline of none: no data");
}

#[derive(Default)]
struct Chart {
    selected: Option<usize>,
    clicked: Option<usize>,
    cursor: CursorIcon,
}

fn selectable_harness() -> testing::Harness<'static, Chart> {
    let buckets: &'static [Bucket] = Box::leak(price().into_boxed_slice());
    testing::harness_state(Theme::dark(), Chart::default(), move |ui, chart| {
        let out = Histogram::new("price", buckets)
            .selectable(true)
            .selected(chart.selected)
            .show(ui);
        if out.selected.is_some() {
            chart.selected = out.selected;
        }
        if out.clicked.is_some() {
            chart.clicked = out.clicked;
        }
        chart.cursor = cursor::current(ui.ctx());
    })
}

#[test]
fn click_selects_the_bucket_under_the_pointer() {
    let mut harness = selectable_harness();
    let rect = harness.get_by_label_contains("Histogram of price").rect();
    // The second of four equal slots.
    let pos = egui::pos2(rect.left() + rect.width() * 0.37, rect.top() + 20.0);
    harness.hover_at(pos);
    harness.run();
    assert_eq!(harness.state().cursor, CursorIcon::PointingHand);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
    harness.run();
    assert_eq!(harness.state().selected, Some(1));
    assert_eq!(harness.state().clicked, Some(1));
    // The click took the keyboard focus: arrows continue from the clicked bucket.
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selected, Some(2));
    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness.get_by_label(&format!("{PRICE}; selected 20–40: 148 documents"));
}

#[test]
fn arrow_keys_move_the_selection_and_enter_activates() {
    let mut harness = selectable_harness();
    harness.get_by_label(PRICE).focus();
    harness.run();
    for (key, expected) in [
        (Key::ArrowRight, 0),
        (Key::ArrowRight, 1),
        (Key::ArrowRight, 2),
        (Key::ArrowLeft, 1),
        (Key::End, 3),
        (Key::ArrowRight, 3),
        (Key::Home, 0),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().selected, Some(expected), "{key:?}");
    }
    assert_eq!(harness.state().clicked, None, "arrows only select");
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().clicked, Some(1));
}

#[test]
fn data_keeps_the_arrow_unless_selectable() {
    let buckets = price();
    let mut harness = testing::harness_state(Theme::dark(), CursorIcon::Default, |ui, c| {
        Histogram::new("price", &buckets).show(ui);
        *c = cursor::current(ui.ctx());
    });
    let rect = harness.get_by_label(PRICE).rect();
    harness.hover_at(rect.center());
    harness.run();
    assert_eq!(*harness.state(), CursorIcon::Default);
}

#[test]
fn hovering_a_bar_shows_its_range_and_count() {
    let buckets = price();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        Histogram::new("price", &buckets).show(ui);
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 260.0));
    assert!(harness.query_by_label("20–40").is_none());
    let rect = harness.get_by_label(PRICE).rect();
    harness.hover_at(egui::pos2(
        rect.left() + rect.width() * 0.37,
        rect.top() + 30.0,
    ));
    harness.run_steps(60);
    assert!(harness.query_by_label("20–40").is_some());
    assert!(
        harness
            .query_by_label_contains("148 documents · 51.6%")
            .is_some()
    );
}

#[test]
fn a_tooltip_attached_by_the_caller_replaces_the_automatic_one() {
    let buckets = price();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        Histogram::new("price", &buckets)
            .with(archipel_kit::components::Tooltip::new("Filter by price"))
            .show(ui);
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 260.0));
    let rect = harness.get_by_label(PRICE).rect();
    harness.hover_at(egui::pos2(
        rect.left() + rect.width() * 0.37,
        rect.top() + 30.0,
    ));
    harness.run_steps(60);
    assert!(harness.query_by_label("Filter by price").is_some());
    assert!(harness.query_by_label("20–40").is_none());
}

#[test]
fn sparkline_tooltip_is_opt_in() {
    let values = [1.0, 5.0, 3.0];
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(Sparkline::new("rtt", &values).tooltip(true));
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 160.0));
    let rect = harness.get_by_label_contains("Sparkline of rtt").rect();
    harness.hover_at(egui::pos2(rect.center().x, rect.center().y));
    harness.run_steps(60);
    assert!(harness.query_by_label("5").is_some());
}

#[test]
fn degenerate_inputs_do_not_panic() {
    let zeros = [Bucket::new("a", 0), Bucket::new("b", 0)];
    let one = [Bucket::new("only", 7)];
    let many: Vec<Bucket> = (0..500).map(|i| Bucket::new(format!("b{i}"), i)).collect();
    let nan = [f32::NAN; 3];
    let mut harness = testing::harness(Theme::dark(), |ui| {
        for buckets in [&zeros[..], &one[..], &many[..], &[][..]] {
            Histogram::new("x", buckets)
                .selectable(true)
                .selected(Some(99))
                .show(ui);
            Histogram::new("x", buckets)
                .horizontal(true)
                .ramp(true)
                .show(ui);
        }
        for values in [&nan[..], &[1.0][..], &[2.0, 2.0][..], &[][..]] {
            ui.add(
                Sparkline::new("x", values)
                    .last_marker(true)
                    .min_max_band(true),
            );
        }
    });
    harness.run();
}

#[test]
fn charts_snapshot() {
    let (buckets, cities) = (price(), top_n());
    let wide: Vec<Bucket> = [3, 8, 14, 22, 35, 41, 38, 27, 19, 11, 6, 2]
        .into_iter()
        .enumerate()
        .map(|(i, count)| Bucket::range(i as f64 * 10.0, i as f64 * 10.0 + 10.0, count))
        .collect();
    let trend = [
        12.0, 14.5, 11.0, 13.0, 30.0, 18.0, 16.5, 15.0, 14.0, 17.0, 12.5,
    ];
    testing::snapshot_themes("chart", common::IMG, |ui| {
        ui.set_max_width(360.0);
        Histogram::new("price", &buckets).height(96.0).show(ui);
        ui.add_space(8.0);
        Histogram::new("age", &wide)
            .height(96.0)
            .ramp(true)
            .selected(Some(5))
            .show(ui);
        ui.add_space(8.0);
        Histogram::new("city", &cities)
            .horizontal(true)
            .selectable(true)
            .selected(Some(1))
            .show(ui);
        ui.add_space(8.0);
        Histogram::new("price", &buckets)
            .height(64.0)
            .enabled(false)
            .show(ui);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(Sparkline::new("latency", &trend));
            ui.add(
                Sparkline::new("latency", &trend)
                    .last_marker(true)
                    .min_max_band(true),
            );
            ui.add(
                Sparkline::new("errors", &trend)
                    .color(ChartColor::Tone(Tone::Error))
                    .last_marker(true),
            );
            ui.add(Sparkline::new("single", &[4.0]).last_marker(true));
            ui.add(Sparkline::new("none", &[]));
        });
    });
}
