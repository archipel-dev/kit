//! `Form` tests: the Settings-page rhythm measured on Android Studio, comments, groups.

mod common;

use archipel_kit::components::{Checkbox, Dropdown, FieldLabel, Form, Radio, TextField};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::{Theme, egui, tokens};
use egui_kittest::kittest::NodeT;

fn settings_page(ui: &mut egui::Ui) {
    ui.set_width(560.0);
    let (mut lighter, mut readers, mut contrast, mut custom) = (false, true, false, false);
    let mut zoom = String::from("100%");
    let (mut font, mut size, mut open) = ("Inter", "13", 2);
    Form::new().show(ui, |form| {
        form.add(
            Checkbox::new(&mut lighter, "Different tool window background")
                .comment("Use a lighter color in the dark theme and a darker one in the light theme"),
        );
        form.group("Accessibility", |form| {
            form.row("Zoom:", |ui| ui.add(TextField::new(&mut zoom).name("Zoom").width(60.0)));
            form.space();
            form.fields(|ui| {
                ui.add(Checkbox::new(&mut custom, "Use custom font:"));
                ui.add(Dropdown::new("font", &mut font, ["Inter"]).width(200.0).enabled(custom));
                ui.add_space(tokens::size::FORM_CONTROL_SPACE);
                ui.add(FieldLabel::new("Size:").enabled(custom));
                ui.add(Dropdown::new("size", &mut size, ["13"]).width(70.0).enabled(custom));
            });
            form.add(
                Checkbox::new(&mut readers, "Support screen readers")
                    .inline_comment("Requires restart")
                    .comment("Ctrl+Tab and Ctrl+Shift+Tab will navigate UI controls in dialogs and will not be available for switching editor tabs."),
            );
            form.add(Checkbox::new(&mut contrast, "Use contrast scrollbars"));
        });
        form.group("Project", |form| {
            form.radios("Open project in", |ui| {
                ui.add(Radio::new(&mut open, 0, "New window"));
                ui.add(Radio::new(&mut open, 1, "Current window"));
                ui.add(Radio::new(&mut open, 2, "Ask"));
            });
        });
    });
}

#[test]
fn snapshot() {
    testing::snapshot_themes("form", common::IMG, settings_page);
}

#[test]
fn rows_follow_the_settings_rhythm() {
    let harness = testing::harness(Theme::dark(), settings_page);
    let top = |label: &str| harness.get_by_label(label).rect().top();
    // Checkbox rows 29 pt apart; a comment adds 7 pt plus 16 per line (measured in
    // Android Studio: 23 for one line, 38 for two).
    assert_eq!(
        top("Use contrast scrollbars") - top("Support screen readers"),
        29.0 + 39.0
    );
    // A group title sits 20 pt further down than a plain row would.
    let lighter = top("Different tool window background");
    assert_eq!(top("Accessibility") - lighter, 29.0 + 23.0 + 20.0);
    // Rows in a group are indented 24 pt under the title.
    let indent = harness.get_by_label("Zoom:").rect().left()
        - harness.get_by_label("Accessibility").rect().left();
    assert_eq!(indent, tokens::size::FORM_GROUP_INDENT);
}

#[test]
fn first_group_has_no_space_above() {
    let harness = testing::harness(Theme::dark(), |ui| {
        Form::new().show(ui, |form| {
            form.group("General", |form| {
                form.add(Checkbox::new(&mut false, "Show tips"));
            });
        });
    });
    let title = harness.get_by_label("General").rect();
    let row = harness.get_by_label("Show tips").rect();
    assert_eq!(
        row.top() - title.top(),
        tokens::size::ROW_HEIGHT + tokens::size::FORM_ROW_GAP
    );
}

#[test]
fn comment_describes_the_checkbox_and_does_not_toggle_it() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(
            Checkbox::new(checked, "Support screen readers")
                .inline_comment("Requires restart")
                .comment("Tooltips on mouse hover will be disabled."),
        );
    });
    let node = harness.get_by_label("Support screen readers");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("Tooltips on mouse hover will be disabled.")
    );
    // The comment below is explanation, not part of the click target.
    let row = node.rect();
    let pos = egui::pos2(row.left() + 40.0, row.bottom() + 10.0);
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
    assert!(!*harness.state());
    harness.get_by_label("Support screen readers").click();
    harness.run();
    assert!(*harness.state());
}

fn advanced(ui: &mut egui::Ui, open: &mut bool) {
    ui.set_width(360.0);
    let mut timeout = String::from("30");
    let mut retry = true;
    Form::new().show(ui, |form| {
        form.collapsible("Advanced", open, |form| {
            form.row("Timeout:", |ui| {
                ui.add(TextField::new(&mut timeout).name("Timeout").width(60.0))
            });
            form.add(Checkbox::new(&mut retry, "Retry reads"));
        });
    });
}

#[test]
fn a_collapsible_group_shows_its_rows_only_while_open() {
    let mut harness = testing::harness_state(Theme::dark(), false, advanced);
    let header = harness.get_by_label("Advanced");
    assert_eq!(header.accesskit_node().data().is_expanded(), Some(false));
    assert!(harness.query_by_label("Timeout").is_none());
    harness.get_by_label("Advanced").click();
    harness.run();
    assert!(*harness.state());
    assert_eq!(
        harness
            .get_by_label("Advanced")
            .accesskit_node()
            .data()
            .is_expanded(),
        Some(true)
    );
    assert!(harness.query_by_label("Timeout").is_some());
    // The keyboard toggles it too.
    harness.get_by_label("Advanced").focus();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(!*harness.state());
}

#[test]
fn collapsible_snapshot() {
    testing::snapshot_themes("form_collapsible", common::IMG, |ui| {
        let mut open = true;
        advanced(ui, &mut open);
        let mut closed = false;
        Form::new().show(ui, |form| {
            form.collapsible("Proxy", &mut closed, |_| {});
        });
    });
}
