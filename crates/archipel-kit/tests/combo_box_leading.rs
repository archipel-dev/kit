//! ComboBox slots reuse the list row's content and preserve picker accessibility.
use archipel_kit::{
    Glyph, Theme,
    openapi::ui::ComboBox,
    parts::Slot,
    testing::{self, Queryable, Role},
};
#[test]
fn slots_do_not_replace_option_names_or_keyboard_selection() {
    let mut h = testing::harness_state(Theme::dark(), "Local", |ui, value| {
        ui.add(
            ComboBox::new("environment", value, ["Local", "Production"])
                .label("Environment")
                .leading(|_| Slot::icon(Glyph::Database)),
        );
    });
    h.set_size(egui::vec2(400.0, 260.0));
    h.get_by_role(Role::ComboBox).click();
    h.run();
    h.get_by_label("Production");
    h.key_press(egui::Key::ArrowDown);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(*h.state(), "Production");
}
#[test]
fn snapshots() {
    testing::snapshot_themes("combo_box_leading", "../../docs/components/img", |ui| {
        let mut value = "Local";
        ui.add(
            ComboBox::new("env", &mut value, ["Local", "Production"])
                .label("Environment")
                .leading(|_| Slot::icon(Glyph::Database)),
        );
        ui.add(
            ComboBox::new("disabled", &mut value, ["Local", "Production"])
                .label("Disabled environment")
                .leading(|_| Slot::icon(Glyph::Database))
                .enabled(false),
        );
    });
}
