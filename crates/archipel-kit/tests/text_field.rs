//! `text_field` component tests: behavior, accessibility and dark/light snapshots.

mod common;

use archipel_kit::components::TextField;
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Theme, egui};

#[test]
fn snapshot() {
    testing::snapshot_themes("text_field", common::IMG, |ui| {
        let mut empty = String::new();
        ui.add(
            TextField::new(&mut empty)
                .label("Project name")
                .placeholder("my-app"),
        );
        let mut query = "Button".to_owned();
        ui.add(TextField::search(&mut query));
        let mut port = "80a".to_owned();
        ui.add(
            TextField::new(&mut port)
                .label("Port")
                .error(Some("Enter a number from 0 to 65535")),
        );
        let mut path = "~/code/archipel".to_owned();
        ui.add(
            TextField::new(&mut path)
                .label("Location")
                .monospace(true)
                .enabled(false),
        );
    });
}

#[test]
fn typing_updates_the_bound_string() {
    let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        ui.add(TextField::new(text).label("Name"));
    });
    let input = harness.get_by_label("Name");
    assert_eq!(input.role(), Role::TextInput);
    input.focus();
    input.type_text("archipel");
    harness.run();
    assert_eq!(harness.state(), "archipel");
}

#[test]
fn clear_button_empties_the_field() {
    let mut harness = testing::harness_state(Theme::dark(), "query".to_owned(), |ui, text| {
        ui.add(TextField::search(text));
    });
    harness.get_by_label("Clear").click();
    harness.run();
    assert_eq!(harness.state(), "");
    assert!(
        harness.query_by_label("Clear").is_none(),
        "no clear button on an empty field"
    );
}

#[test]
fn error_message_is_visible() {
    let harness = testing::harness(Theme::light(), |ui| {
        let mut text = "x".to_owned();
        ui.add(TextField::new(&mut text).error(Some("Required")));
    });
    harness.get_by_label("Required");
}

#[test]
fn disabled_field_is_read_only() {
    let mut harness = testing::harness_state(Theme::dark(), "fixed".to_owned(), |ui, text| {
        ui.add(TextField::new(text).label("Path").enabled(false));
    });
    let input = harness.get_by_label("Path");
    input.focus();
    input.type_text("changed");
    harness.run();
    assert_eq!(harness.state(), "fixed");
}

#[test]
fn id_is_the_text_edits_id_and_requests_focus() {
    let id = egui::Id::new("find-input");
    let mut harness =
        testing::harness_state(Theme::dark(), (String::new(), None), |ui, (text, seen)| {
            let response = ui.add(TextField::new(text).label("Find").id(id));
            *seen = Some(response.id);
        });
    harness.run();
    assert_eq!(
        harness.state().1,
        Some(id),
        "the response is the TextEdit's, with the given id"
    );
    assert!(!harness.ctx.memory(|m| m.has_focus(id)));
    harness.ctx.memory_mut(|m| m.request_focus(id));
    harness.run();
    assert!(harness.ctx.memory(|m| m.has_focus(id)));
    harness.key_press(egui::Key::A);
    harness.event(egui::Event::Text("a".into()));
    harness.run();
    assert_eq!(harness.state().0, "a");
}

#[test]
fn clear_button_sits_at_the_measured_place() {
    // Android Studio's search fields: a 16 pt round target centered 15 pt from the right edge.
    use archipel_kit::egui;
    use archipel_kit::tokens::size;
    let field = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(247.0, 28.0));
    let harness = testing::harness(Theme::dark(), move |ui| {
        let mut text = String::from("font");
        ui.put(field, TextField::search(&mut text).width(247.0));
    });
    let clear = harness.get_by_label("Clear").rect();
    assert_eq!(
        clear.size(),
        egui::vec2(size::CLEAR_BUTTON, size::CLEAR_BUTTON)
    );
    assert_eq!(clear.center().x, field.right() - size::CLEAR_BUTTON_INSET);
    assert_eq!(clear.center().y, field.center().y);
}

#[test]
fn a_sized_field_keeps_its_width_in_a_right_to_left_row() {
    let harness = testing::harness_state_sized(
        Theme::dark(),
        egui::vec2(400.0, 60.0),
        (String::from("1000"), 50_u32),
        |ui, (text, number)| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(TextField::new(text).name("Sample").width(80.0));
                ui.add(archipel_kit::components::NumberField::new(number).name("Limit"));
            });
        },
    );
    let sample = harness.get_by_label("Sample").rect();
    let limit = harness.get_by_label("Limit").rect();
    assert!(sample.right() > 360.0, "at the right edge: {sample:?}");
    assert!(
        limit.right() <= sample.left(),
        "left of it: {limit:?} {sample:?}"
    );
    assert!(
        limit.width() < 64.0 && sample.width() < 80.0,
        "their own widths, not the row: {limit:?} {sample:?}"
    );
}
