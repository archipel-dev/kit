//! Drag and drop: a row with a `DragSource` behaviour onto a `DropZone`.

mod common;

use archipel_kit::Theme;
use archipel_kit::behaviour::DragSource;
use archipel_kit::components::{DropZone, ListRow};
use archipel_kit::egui::{self, Pos2, pos2};
use archipel_kit::testing::{self, Harness, Queryable};

#[derive(Debug, Clone, PartialEq)]
struct Field(&'static str);

#[derive(Debug, Clone, PartialEq)]
struct Other;

#[derive(Default)]
struct State {
    filter: Vec<&'static str>,
    sort: Vec<&'static str>,
    hovered: bool,
    clicked: Option<&'static str>,
}

fn harness() -> Harness<'static, State> {
    testing::harness_state_sized(
        Theme::dark(),
        egui::vec2(360.0, 260.0),
        State::default(),
        |ui, state| {
            for name in ["name", "price"] {
                let row = ListRow::new(name).with(DragSource(Field(name))).show(ui);
                if row.response.clicked() {
                    state.clicked = Some(name);
                }
            }
            ListRow::new("other").with(DragSource(Other)).show(ui);
            let filter = DropZone::new("filter")
                .hint("Drop fields here to filter")
                .show_hint(state.filter.is_empty())
                .show::<Field, _>(ui, |ui| {
                    for name in &state.filter {
                        ui.label(*name);
                    }
                });
            state.hovered = filter.hovered;
            if let Some(field) = filter.dropped {
                state.filter.push(field.0);
            }
            let sort = DropZone::new("sort")
                .hint("Drop fields here to sort")
                .show::<Field, _>(ui, |_| {});
            if let Some(field) = sort.dropped {
                state.sort.push(field.0);
            }
        },
    )
}

fn center(harness: &Harness<'_, State>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

/// Presses at `from`, moves to `to` in steps, and checks `during` before letting go.
fn drag(harness: &mut Harness<'_, State>, from: Pos2, to: Pos2, during: impl Fn(&State)) {
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        harness.hover_at(from + (to - from) * t);
        harness.run();
    }
    during(harness.state());
    harness.drop_at(to);
    harness.run();
    harness.run();
}

#[test]
fn a_row_dropped_on_a_zone_lands_there() {
    let mut harness = harness();
    let from = center(&harness, "price");
    let to = center(&harness, "Drop fields here to filter");
    drag(&mut harness, from, to, |state| {
        assert!(state.hovered, "the zone shows it would take the field");
    });
    assert_eq!(harness.state().filter, ["price"]);
    assert!(
        harness.state().sort.is_empty(),
        "only the zone under the pointer"
    );
    assert!(!harness.state().hovered, "the drag is over");
}

#[test]
fn other_payloads_and_misses_drop_nothing() {
    let mut harness = harness();
    let from = center(&harness, "other");
    let to = center(&harness, "Drop fields here to filter");
    drag(&mut harness, from, to, |state| assert!(!state.hovered));
    assert!(harness.state().filter.is_empty(), "another payload type");
    let from = center(&harness, "name");
    drag(&mut harness, from, pos2(from.x, 4.0), |_| {});
    assert!(harness.state().filter.is_empty() && harness.state().sort.is_empty());
}

#[test]
fn a_click_still_reaches_the_row() {
    let mut harness = harness();
    harness.get_by_label("name").click();
    harness.run();
    assert_eq!(harness.state().clicked, Some("name"));
    assert!(harness.state().filter.is_empty());
}

#[test]
fn snapshot_while_dragging() {
    testing::snapshot_themes_after(
        "drag_and_drop",
        common::IMG,
        |ui| {
            ui.set_width(280.0);
            for name in ["name", "price"] {
                ListRow::new(name)
                    .icon(archipel_kit::Glyph::Key)
                    .with(DragSource(Field(name)))
                    .show(ui);
            }
            ui.add_space(8.0);
            let _ = DropZone::new("filter")
                .hint("Drop fields here to filter")
                .show::<Field, _>(ui, |_| {});
            let _ = DropZone::new("other")
                .hint("Takes something else")
                .show::<Other, _>(ui, |_| {});
        },
        |h| {
            // Accessibility bounds are in pixels, pointer events in points.
            let ppp = h.ctx.pixels_per_point();
            let point = |p: Pos2| pos2(p.x / ppp, p.y / ppp);
            let from = point(h.get_by_label("price").rect().center());
            let to = point(h.get_by_label("Drop fields here to filter").rect().center());
            h.hover_at(from);
            h.run();
            h.drag_at(from);
            h.run();
            for step in 1..=8 {
                let t = step as f32 / 8.0;
                h.hover_at(from + (to - from) * t);
                h.run();
            }
        },
    );
}
