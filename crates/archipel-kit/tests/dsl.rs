//! The UI DSL (`ui::dsl`): the Settings-page rhythm measured on Android Studio, comments,
//! groups, gaps, the label column, enabled/visible and validation.

mod common;

use archipel_kit::openapi::ui::ValidationInfo;
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::ui::dsl::{CellAlign, Gap, RadioGroup, RightGap, RowLayout, panel};
use archipel_kit::ui::{JBCheckBox, JBTextField, ThreeStateCheckBox};
use archipel_kit::{Theme, egui, tokens};
use egui_kittest::kittest::NodeT;

fn settings_page(ui: &mut egui::Ui) {
    ui.set_width(560.0);
    let (mut lighter, mut readers, mut contrast, mut custom) = (false, true, false, false);
    let mut zoom = String::from("100%");
    let (mut font, mut size, mut open) = ("Inter", "13", 2);
    panel(ui, |p| {
        p.unlabeled_row(|r| {
            r.cell(JBCheckBox::new(
                &mut lighter,
                "Different tool window background",
            ))
            .comment_indent(24.0)
            .comment("Use a lighter color in the dark theme and a darker one in the light theme");
        });
        p.group("Accessibility", |p| {
            p.row("Zoom:", |r| {
                r.text_field(&mut zoom).width(60.0);
            });
            p.gap(Gap::Small);
            // A cell borrows its value until the row is laid out: read what `enabled_if`
            // needs first.
            let custom_font = custom;
            p.unlabeled_row(|r| {
                r.check_box(&mut custom, "Use custom font:").gap(RightGap::Small);
                r.combo_box("font", &mut font, ["Inter"])
                    .width(200.0)
                    .enabled_if(custom_font);
                r.label("Size:").enabled_if(custom_font);
                r.combo_box("size", &mut size, ["13"])
                    .width(70.0)
                    .enabled_if(custom_font);
            });
            p.unlabeled_row(|r| {
                r.cell(JBCheckBox::new(&mut readers, "Support screen readers").inline_comment("Requires restart"))
                    .comment_indent(24.0)
                    .comment("Ctrl+Tab and Ctrl+Shift+Tab will navigate UI controls in dialogs and will not be available for switching editor tabs.");
            });
            p.unlabeled_row(|r| {
                r.check_box(&mut contrast, "Use contrast scrollbars");
            });
        });
        p.group("Project", |p| {
            let group = RadioGroup::new(&mut open);
            p.row("Open project in", |r| {
                r.radio_button(&group, 0, "New window");
                r.radio_button(&group, 1, "Current window");
                r.radio_button(&group, 2, "Ask");
            });
        });
    });
}

#[test]
fn snapshot() {
    testing::snapshot_themes("dsl", common::IMG, settings_page);
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
        let mut tips = false;
        panel(ui, |p| {
            p.group("General", |p| {
                p.unlabeled_row(|r| {
                    r.check_box(&mut tips, "Show tips");
                });
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
fn comment_describes_the_check_box_and_does_not_toggle_it() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, checked| {
        ui.add(
            JBCheckBox::new(checked, "Support screen readers")
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
    panel(ui, |p| {
        p.collapsible_group("Advanced", open, |p| {
            p.row("Timeout:", |r| {
                r.text_field(&mut timeout)
                    .width(60.0)
                    .comment("Seconds before a query is cancelled");
            });
            p.unlabeled_row(|r| {
                r.check_box(&mut retry, "Retry reads");
            });
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
    testing::snapshot_themes("dsl_collapsible", common::IMG, |ui| {
        let mut open = true;
        advanced(ui, &mut open);
        let mut closed = false;
        panel(ui, |p| {
            p.collapsible_group("Proxy", &mut closed, |_| {});
        });
    });
}

#[test]
fn labelled_rows_start_their_fields_in_one_column() {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (String::from("Local"), String::from("mongodb://localhost")),
        |ui, (name, uri)| {
            ui.set_width(420.0);
            panel(ui, |p| {
                p.row("Name:", |r| {
                    r.text_field(name);
                });
                p.row("Connection string:", |r| {
                    r.text_field(uri);
                });
            });
        },
    );
    // The first frame already shows the column: the panel lays out again when it moves.
    let name = harness.get_by_label("Name").rect();
    let uri = harness.get_by_label("Connection string").rect();
    assert!((name.left() - uri.left()).abs() < 0.5, "{name:?} {uri:?}");
    harness.run();
    let again = harness.get_by_label("Name").rect();
    assert_eq!(name, again, "stable after the first frame");
}

#[test]
fn a_comment_starts_under_the_fields_of_its_row() {
    let harness = testing::harness_state(Theme::dark(), true, advanced);
    let field = harness.get_by_label("Timeout").rect();
    let comment = harness
        .get_by_label("Seconds before a query is cancelled")
        .rect();
    assert!(comment.top() >= field.bottom(), "below the field");
    // The field's text sits a padding inside its box; the comment starts at the box.
    let padding = tokens::size::CODE_PADDING;
    assert!(
        (comment.left() + padding - field.left()).abs() <= padding,
        "under the field column: {comment:?} {field:?}"
    );
}

#[test]
fn the_comment_sits_three_points_under_the_control() {
    let harness = testing::harness(Theme::dark(), |ui| {
        panel(ui, |p| {
            let mut zoom = String::new();
            p.row("Zoom:", |r| {
                r.text_field(&mut zoom).comment("A note");
            });
        });
    });
    let field = harness.get_by_label("Zoom").rect();
    let note = harness.get_by_label("A note").rect();
    assert!(
        (note.top() - field.bottom() - tokens::size::COMMENT_GAP).abs() <= 6.0,
        "{note:?} {field:?}"
    );
}

#[test]
fn gaps_between_cells_follow_the_right_gap() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.set_width(600.0);
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.label("A:");
                r.label("B:");
                r.label("C:").gap(RightGap::Default);
                r.label("D:");
                r.label("E:").gap(RightGap::Columns);
                r.label("F:");
            });
        });
    });
    let right = |l: &str| harness.get_by_label(l).rect().right();
    let left = |l: &str| harness.get_by_label(l).rect().left();
    assert!((left("B:") - right("A:") - tokens::size::FORM_LABEL_GAP).abs() < 0.6);
    assert!((left("D:") - right("C:") - tokens::size::FORM_CONTROL_GAP).abs() < 0.6);
    assert!((left("F:") - right("E:") - tokens::size::DSL_COLUMNS_GAP).abs() < 0.6);
}

#[test]
fn top_and_bottom_gaps_add_vertical_space() {
    let harness = testing::harness(Theme::dark(), |ui| {
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.label("One");
            });
            p.unlabeled_row(|r| {
                r.top_gap(Gap::Small);
                r.label("Two");
            });
            p.unlabeled_row(|r| {
                r.top_gap(Gap::Medium);
                r.label("Three");
            });
        });
    });
    let top = |l: &str| harness.get_by_label(l).rect().top();
    let pitch = tokens::size::ROW_HEIGHT + tokens::size::FORM_ROW_GAP;
    assert!((top("Two") - top("One") - pitch - tokens::size::DSL_GAP_SMALL).abs() < 0.6);
    assert!((top("Three") - top("Two") - pitch - tokens::size::DSL_GAP_MEDIUM).abs() < 0.6);
}

#[test]
fn enabled_if_disables_a_cell_and_visible_if_hides_it() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, on| {
        let on_now = *on;
        panel(ui, |p| {
            let mut host = String::new();
            p.unlabeled_row(|r| {
                r.check_box(on, "Use proxy");
                r.text_field(&mut host).enabled_if(on_now);
                r.button("Hidden").visible_if(on_now);
            });
        });
    });
    harness.run();
    assert!(harness.query_by_label("Hidden").is_none());
    assert!(
        harness
            .get_all_by_role(testing::Role::TextInput)
            .next()
            .expect("the field")
            .is_disabled()
    );
    harness.get_by_label("Use proxy").click();
    harness.run();
    assert!(harness.query_by_label("Hidden").is_some());
    assert!(
        !harness
            .get_all_by_role(testing::Role::TextInput)
            .next()
            .expect("the field")
            .is_disabled()
    );
}

#[test]
fn a_hidden_row_leaves_no_space() {
    let harness = testing::harness(Theme::dark(), |ui| {
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.label("One");
            });
            p.unlabeled_row(|r| {
                r.visible_if(false);
                r.label("Two");
            });
            p.unlabeled_row(|r| {
                r.label("Three");
            });
        });
    });
    assert!(harness.query_by_label("Two").is_none());
    let pitch = tokens::size::ROW_HEIGHT + tokens::size::FORM_ROW_GAP;
    let top = |l: &str| harness.get_by_label(l).rect().top();
    assert!((top("Three") - top("One") - pitch).abs() < 0.6);
}

#[test]
fn a_disabled_row_disables_all_its_cells() {
    let harness = testing::harness(Theme::dark(), |ui| {
        panel(ui, |p| {
            let mut locked = false;
            p.enabled_if(false, |p| {
                p.unlabeled_row(|r| {
                    r.check_box(&mut locked, "Locked");
                });
            });
        });
    });
    assert!(harness.get_by_label("Locked").is_disabled());
}

#[test]
fn a_validated_cell_shows_its_message_and_a_field_its_outline() {
    let harness = testing::harness(Theme::dark(), |ui| {
        let (mut port, mut host) = (String::from("80a"), "a");
        panel(ui, |p| {
            p.row("Port:", |r| {
                r.text_field(&mut port)
                    .width(80.0)
                    .validation(Some(&ValidationInfo::error(
                        "Enter a number from 0 to 65535",
                    )));
            });
            p.row("Host:", |r| {
                r.combo_box("host", &mut host, ["a", "b"])
                    .validation(Some(&ValidationInfo::warning("Resolves slowly")));
            });
        });
    });
    assert!(
        harness
            .query_by_label("Enter a number from 0 to 65535")
            .is_some()
    );
    assert!(harness.query_by_label("Resolves slowly").is_some());
}

#[test]
fn validation_snapshot() {
    testing::snapshot_themes("dsl_validation", common::IMG, |ui| {
        ui.set_width(420.0);
        let mut port = String::from("80a");
        let mut host = String::from("localhost");
        let mut method = "GET";
        panel(ui, |p| {
            p.row("Host:", |r| {
                r.text_field(&mut host)
                    .validation(Some(&ValidationInfo::warning("Host is not reachable")));
            });
            p.row("Port:", |r| {
                r.text_field(&mut port)
                    .width(120.0)
                    .validation(Some(&ValidationInfo::error(
                        "Enter a number from 0 to 65535",
                    )));
            });
            p.row("Method:", |r| {
                r.combo_box("m", &mut method, ["GET", "POST"])
                    .width(120.0)
                    .validation(Some(&ValidationInfo::error("POST is not allowed")))
                    .context_help("The HTTP method of the request");
            });
        });
    });
}

#[test]
fn parent_grid_rows_share_their_columns() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.set_width(600.0);
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.layout(RowLayout::ParentGrid);
                r.label("Short");
                r.label("x");
            });
            p.unlabeled_row(|r| {
                r.layout(RowLayout::ParentGrid);
                r.label("A much longer label");
                r.label("y");
            });
        });
    });
    let x = harness.get_by_label("x").rect().left();
    let y = harness.get_by_label("y").rect().left();
    assert!((x - y).abs() < 0.6, "{x} {y}");
}

#[test]
fn cells_can_align_right_and_fill() {
    let harness = testing::harness(Theme::dark(), |ui| {
        ui.set_width(400.0);
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.label("Left");
                r.label("Right").align(CellAlign::Right);
            });
        });
    });
    // Right alignment needs a frame of measuring; the harness runs it.
    let right = harness.get_by_label("Right").rect().right();
    let left = harness.get_by_label("Left").rect().left();
    assert!(right - left > 300.0, "{left} {right}");
}

#[test]
fn on_response_reports_a_change() {
    let mut harness = testing::harness_state(Theme::dark(), (false, 0), |ui, (on, changes)| {
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.check_box(on, "Option").on_response(|response| {
                    if response.changed() {
                        *changes += 1;
                    }
                });
            });
        });
    });
    harness.get_by_label("Option").click();
    harness.run();
    assert_eq!(harness.state().1, 1);
}

#[test]
fn a_three_state_checkbox_cycles_in_a_row() {
    use archipel_kit::ui::ThreeState;
    let mut harness = testing::harness_state(Theme::dark(), ThreeState::Unchecked, |ui, state| {
        panel(ui, |p| {
            p.unlabeled_row(|r| {
                r.cell(ThreeStateCheckBox::new(state, "All"));
            });
        });
    });
    for expected in [
        ThreeState::Checked,
        ThreeState::Indeterminate,
        ThreeState::Unchecked,
    ] {
        harness.get_by_label("All").click();
        harness.run();
        assert_eq!(*harness.state(), expected);
    }
}

#[test]
fn fields_take_the_label_of_their_row_as_name() {
    let harness = testing::harness(Theme::dark(), |ui| {
        let (mut zoom, mut font) = (String::new(), "Inter");
        panel(ui, |p| {
            p.row("Zoom:", |r| {
                r.text_field(&mut zoom);
            });
            p.row("Font:", |r| {
                r.combo_box("font", &mut font, ["Inter"]);
            });
        });
    });
    assert!(harness.query_by_label("Zoom").is_some());
    assert!(harness.query_by_label("Font").is_some());
}

#[test]
fn buttons_groups_indent_under_their_title() {
    let harness = testing::harness(Theme::dark(), |ui| {
        let mut open = 0;
        let group = RadioGroup::new(&mut open);
        panel(ui, |p| {
            p.buttons_group(Some("Open in"), |p| {
                p.unlabeled_row(|r| {
                    r.radio_button(&group, 0, "New window");
                });
            });
        });
    });
    let title = harness.get_by_label("Open in").rect();
    let radio = harness.get_by_label("New window").rect();
    assert_eq!(radio.left() - title.left(), tokens::size::FORM_GROUP_INDENT);
}

#[test]
fn text_field_cells_do_not_break_on_a_narrow_ui() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.set_max_width(40.0);
        let (mut text, mut other, mut choice) = (String::new(), String::new(), "x");
        panel(ui, |p| {
            p.row("A long label:", |r| {
                r.text_field(&mut text);
                r.cell(JBTextField::new(&mut other));
                r.combo_box("c", &mut choice, ["x"]);
            });
        });
    });
    harness.run();
}
