//! Regression coverage for keyboard operation of picker popups.
use archipel_kit::{
    Theme,
    openapi::ui::ComboBox,
    testing::{self, Queryable, Role},
};

#[test]
fn arrows_preview_enter_commits_and_escape_cancels() {
    let mut h = testing::harness_state(Theme::dark(), "GET", |ui, value| {
        ui.add(ComboBox::new("method", value, ["GET", "POST", "PUT"]));
    });
    h.set_size(egui::vec2(400.0, 300.0));
    h.get_by_role(Role::ComboBox).click();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(*h.state(), "GET");
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(*h.state(), "POST");
    assert!(h.query_by_label("PUT").is_none());
    assert!(h.get_by_role(Role::ComboBox).is_focused());
    h.get_by_role(Role::ComboBox).click();
    h.run();
    h.key_press(egui::Key::End);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert_eq!(*h.state(), "POST");
}

#[test]
fn disabling_an_open_picker_cancels_keyboard_selection() {
    let mut h = testing::harness_state(Theme::dark(), ("GET", true), |ui, (value, enabled)| {
        ui.add(ComboBox::new("method", value, ["GET", "POST"]).enabled(*enabled));
    });
    h.set_size(egui::vec2(400.0, 260.0));
    h.get_by_role(Role::ComboBox).click();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.state_mut().1 = false;
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(h.state().0, "GET");
    assert!(h.query_by_label("POST").is_none());
}

#[test]
fn enter_picks_an_option_without_confirming_the_surrounding_dialog() {
    let mut h = testing::harness_state(Theme::dark(), ("GET", None), |ui, (value, action)| {
        if let Some(done) = archipel_kit::openapi::ui::DialogWrapper::new("Request")
            .content(|ui| {
                ui.add(ComboBox::new("method", value, ["GET", "POST"]));
            })
            .primary("Save")
            .show(ui.ctx())
        {
            *action = Some(done);
        }
    });
    h.set_size(egui::vec2(520.0, 400.0));
    h.run_steps(3);
    h.get_by_role(Role::ComboBox).click();
    h.run_steps(3);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(3);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(h.state().0, "POST");
    assert_eq!(h.state().1, None);
}

#[test]
fn changing_options_while_open_never_submits_the_dialog() {
    for remaining in [0, 1] {
        let mut h = testing::harness_state(
            Theme::dark(),
            ("GET", 3, None),
            |ui, (value, len, action)| {
                if let Some(done) = archipel_kit::openapi::ui::DialogWrapper::new("Request")
                    .content(|ui| {
                        ui.add(ComboBox::new(
                            "method",
                            value,
                            ["GET", "POST", "PUT"].into_iter().take(*len),
                        ));
                    })
                    .primary("Save")
                    .show(ui.ctx())
                {
                    *action = Some(done);
                }
            },
        );
        h.set_size(egui::vec2(520.0, 400.0));
        h.run_steps(3);
        h.get_by_role(Role::ComboBox).click();
        h.run_steps(3);
        h.key_press(egui::Key::End);
        h.run_steps(3);
        h.state_mut().1 = remaining;
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
        assert_eq!(h.state().0, "GET");
        assert_eq!(h.state().2, None);
    }
}
