//! Multi-line and compact named field behavior and accessibility.
use archipel_kit::{
    Theme,
    testing::{self, NodeExt, Queryable, Role},
    ui::JBTextField,
};

#[test]
fn named_multiline_enter_inserts_a_line_and_tab_leaves() {
    let id = egui::Id::new("body");
    let mut h = testing::harness_state(
        Theme::dark(),
        (String::new(), false),
        |ui, (text, submitted)| {
            let out = JBTextField::new(text)
                .name("Request body")
                .multiline(3)
                .id(id)
                .show(ui);
            *submitted |= out.submitted;
            ui.add(archipel_kit::ui::Button::secondary("Send"));
        },
    );
    h.get_by_role_and_label(Role::MultilineTextInput, "Request body")
        .focus();
    h.event(egui::Event::Text("first".into()));
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Text("second".into()));
    h.run();
    assert_eq!(h.state().0, "first\nsecond");
    assert!(!h.state().1);
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(!h.ctx.memory(|m| m.has_focus(id)));
}

#[test]
fn name_overrides_placeholder_and_disabled_multiline_cannot_change() {
    let mut h = testing::harness_state(Theme::light(), "fixed".to_owned(), |ui, text| {
        ui.add(
            JBTextField::new(text)
                .name("Hosts")
                .placeholder("host:port")
                .multiline(3)
                .enabled(false),
        );
    });
    let input = h.get_by_label("Hosts");
    assert!(input.is_disabled());
    input.focus();
    input.type_text("changed");
    h.run();
    assert_eq!(h.state(), "fixed");
}

#[test]
fn multiline_snapshot_and_audit() {
    testing::snapshot_themes(
        "jb_text_field_multiline",
        "../../docs/components/img",
        |ui| {
            let mut body = "{\n  \"name\": \"archipel\"\n}".to_owned();
            ui.add(
                JBTextField::new(&mut body)
                    .label("Request body")
                    .multiline(4)
                    .monospace(true),
            );
            let mut hosts = String::new();
            ui.add(
                JBTextField::new(&mut hosts)
                    .name("Hosts")
                    .placeholder("host:port, one per line")
                    .multiline(2)
                    .enabled(false),
            );
        },
    );
}

#[test]
fn multiline_composition_passes_the_audit_in_both_themes() {
    for theme in [Theme::dark(), Theme::light()] {
        let mut h = testing::harness_state(theme, String::new(), |ui, text| {
            ui.add(
                JBTextField::new(text)
                    .name("Hosts")
                    .multiline(3)
                    .clearable(true),
            );
            ui.add(archipel_kit::ui::Button::primary("Connect"));
        });
        testing::audit(&mut h).assert_clean();
    }
}
