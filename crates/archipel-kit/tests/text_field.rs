//! `text_field` component tests: behavior, accessibility and dark/light snapshots.

mod common;

use archipel_kit::Theme;
use archipel_kit::components::TextField;
use archipel_kit::testing::{self, NodeExt, Queryable, Role};

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
