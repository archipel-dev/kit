//! `Dropdown` keyboard tests: Escape closes the popup and returns focus to the button.

mod common;

use archipel_kit::components::{Dialog, DialogAction, Dropdown};
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::Theme;

fn open_dropdown() -> testing::Harness<'static, &'static str> {
    let mut harness = testing::harness_state(Theme::dark(), "GET", |ui, method| {
        ui.add(Dropdown::new("method", method, ["GET", "POST", "PUT"]));
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 240.0));
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    assert!(harness.query_by_label("PUT").is_some(), "the popup is open");
    harness
}

#[test]
fn escape_closes_the_popup_and_refocuses_the_button() {
    let mut harness = open_dropdown();
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    assert!(harness.query_by_label("PUT").is_none(), "the popup is closed");
    assert!(harness.get_by_role(Role::ComboBox).is_focused());
    assert_eq!(*harness.state(), "GET", "Escape picks nothing");
}

#[test]
fn escape_in_a_dropdown_does_not_close_the_dialog() {
    let mut harness = testing::harness_state(
        Theme::dark(),
        ("GET", None::<DialogAction>),
        |ui, (method, action)| {
            let shown = Dialog::new("Request")
                .content(|ui| {
                    ui.add(Dropdown::new("method", method, ["GET", "POST", "PUT"]));
                })
                .primary("Save")
                .show(ui.ctx());
            if let Some(done) = shown {
                action.get_or_insert(done);
            }
        },
    );
    harness.set_size(egui::vec2(520.0, 400.0));
    harness.run_steps(3);
    harness.get_by_role(Role::ComboBox).click();
    harness.run_steps(3);
    assert!(harness.query_by_label("PUT").is_some(), "the popup is open");
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    assert!(harness.query_by_label("PUT").is_none(), "the popup is closed");
    assert_eq!(harness.state().1, None, "the dialog stays open");
    assert!(harness.get_by_role(Role::ComboBox).is_focused());
    // A second Escape, with no popup open, dismisses the dialog.
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    assert_eq!(harness.state().1, Some(DialogAction::Dismissed));
}
