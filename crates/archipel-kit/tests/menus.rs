//! `ComboBox`, `ActionMenuItem` and `ActionLink` tests.

mod common;

use archipel_kit::parts::Surface;
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme};
use archipel_kit::{
    action_system::ActionMenuItem, openapi::ui::ComboBox, ui::ActionLink, ui::popup::ListSeparator,
};

#[test]
fn menu_snapshot() {
    testing::snapshot_themes("action_menu_item", common::IMG, |ui| {
        Surface::Popup.frame(ui.ctx()).show(ui, |ui| {
            ui.set_width(260.0);
            ui.add(
                ActionMenuItem::new("Copy")
                    .icon(Glyph::Copy)
                    .shortcut("Ctrl+C")
                    .underlined('C'),
            );
            ui.add(
                ActionMenuItem::new("Find in Files…")
                    .icon(Glyph::FindInFiles)
                    .shortcut("Ctrl+Shift+F")
                    .underlined('d'),
            );
            ui.add(ListSeparator::new());
            ui.add(ActionMenuItem::new("Soft-Wrap").checked(true));
            ui.add(ActionMenuItem::new("Run").icon(Glyph::Run).enabled(false));
            ui.add(ListSeparator::new());
            ui.add(ListSeparator::with_text("Open Projects"));
            ui.add(
                ActionMenuItem::new("archipel")
                    .icon(Glyph::Folder)
                    .description("~/code/archipel"),
            );
        });
    });
}

#[test]
fn action_menu_item_clicks_and_reports_check_state() {
    let mut harness = testing::harness_state(Theme::dark(), (0, true), |ui, (copies, wrap)| {
        if ui
            .add(ActionMenuItem::new("Copy").shortcut("Ctrl+C"))
            .clicked()
        {
            *copies += 1;
        }
        if ui
            .add(ActionMenuItem::new("Soft-Wrap").checked(*wrap))
            .clicked()
        {
            *wrap = !*wrap;
        }
    });
    harness.get_by_label("Copy").click();
    harness.run();
    assert_eq!(harness.state().0, 1);
    assert!(harness.get_by_label("Soft-Wrap").is_checked());
    harness.get_by_label("Soft-Wrap").click();
    harness.run();
    assert!(!harness.state().1);
}

#[test]
fn an_underlined_mnemonic_keeps_the_label_and_clicks() {
    let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
        if ui
            .add(ActionMenuItem::new("Paste").underlined('p'))
            .clicked()
        {
            *clicks += 1;
        }
        // A letter the label lacks draws no underline and changes nothing.
        ui.add(ActionMenuItem::new("Cut").underlined('z'));
    });
    harness.get_by_label("Paste").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
    harness.get_by_label("Cut");
}

#[test]
fn combo_box_snapshot() {
    testing::snapshot_themes("combo_box", common::IMG, |ui| {
        ui.add(ComboBox::new("method", &mut "GET", ["GET", "POST"]).width(120.0));
        ui.add(
            ComboBox::new("off", &mut "Disabled", ["Disabled"])
                .width(120.0)
                .enabled(false),
        );
    });
}

#[test]
fn combo_box_picks_an_option() {
    let mut harness = testing::harness_state(Theme::dark(), "GET", |ui, method| {
        ui.add(ComboBox::new("method", method, ["GET", "POST", "PUT"]));
    });
    // Make room for the popup below the button.
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 240.0));
    harness.run();
    let combo = harness.get_by_role(Role::ComboBox);
    assert_eq!(combo.accessible_label().as_deref(), Some("GET"));
    combo.click();
    harness.run();
    harness.get_by_label("PUT").click();
    harness.run();
    assert_eq!(*harness.state(), "PUT");
}

#[test]
fn link_snapshot() {
    testing::snapshot_themes("action_link", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.label("Gradle JVM is not set.");
            ui.add(ActionLink::new("Configure…"));
        });
    });
}

#[test]
fn link_has_link_role() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, clicked| {
        *clicked |= ui.add(ActionLink::new("Open docs")).clicked();
    });
    let node = harness.get_by_label("Open docs");
    assert_eq!(node.role(), Role::Link);
    node.click();
    harness.run();
    assert!(*harness.state());
}

mod submenu {
    use archipel_kit::behaviour::PopupHandler;
    use archipel_kit::testing::{self, NodeExt, NodeT, Queryable};
    use archipel_kit::{Glyph, Theme};
    use archipel_kit::{action_system::ActionMenuItem, ui::ColoredListCellRenderer};
    use egui::Key;

    /// A row whose context menu has "Open" and a "New" submenu with two items; the state
    /// counts what was clicked.
    fn harness() -> testing::Harness<'static, Vec<&'static str>> {
        let mut harness = testing::harness_state(Theme::dark(), Vec::new(), |ui, clicked| {
            let cell = std::cell::RefCell::new(Vec::new());
            ColoredListCellRenderer::new("src")
                .with(PopupHandler::new(|ui| {
                    if ui
                        .add(ActionMenuItem::new("Open").icon(Glyph::Folder))
                        .clicked()
                    {
                        cell.borrow_mut().push("open");
                    }
                    ui.add(ActionMenuItem::new("New").submenu(|ui| {
                        if ui.add(ActionMenuItem::new("File…")).clicked() {
                            cell.borrow_mut().push("file");
                        }
                        ui.add(ActionMenuItem::new("Directory…"));
                    }));
                    ui.add(
                        ActionMenuItem::new("Locked")
                            .submenu(|ui| {
                                ui.add(ActionMenuItem::new("Never shown"));
                            })
                            .enabled(false),
                    );
                }))
                .show(ui);
            clicked.extend(cell.into_inner());
        });
        harness.set_size(egui::vec2(400.0, 300.0));
        harness.run();
        harness.get_by_label("src").click_secondary();
        harness.run();
        harness
    }

    #[test]
    fn opens_on_hover_and_names_its_state() {
        let mut harness = harness();
        assert!(harness.query_by_label("File…").is_none());
        harness.get_by_label("New").hover();
        harness.run();
        assert!(harness.query_by_label("File…").is_some());
        assert!(harness.query_by_label("Directory…").is_some());
        assert_eq!(
            harness
                .get_by_label("New")
                .accesskit_node()
                .data()
                .is_expanded(),
            Some(true)
        );
    }

    #[test]
    fn click_on_a_submenu_item_acts_and_closes_every_menu() {
        let mut harness = harness();
        harness.get_by_label("New").hover();
        harness.run();
        harness.get_by_label("File…").click();
        harness.run();
        harness.run();
        assert_eq!(*harness.state(), ["file"]);
        assert!(harness.query_by_label("File…").is_none());
        assert!(
            harness.query_by_label("New").is_none(),
            "the parent menu closed too"
        );
    }

    #[test]
    fn clicking_the_parent_item_keeps_the_menu_open() {
        let mut harness = harness();
        harness.get_by_label("New").click();
        harness.run();
        assert!(harness.query_by_label("File…").is_some());
        assert!(harness.query_by_label("Open").is_some());
    }

    #[test]
    fn a_plain_item_closes_the_menu() {
        let mut harness = harness();
        harness.get_by_label("Open").click();
        harness.run();
        harness.run();
        assert_eq!(*harness.state(), ["open"]);
        assert!(harness.query_by_label("New").is_none());
    }

    #[test]
    fn escape_closes_only_the_submenu() {
        let mut harness = harness();
        harness.get_by_label("New").hover();
        harness.run();
        harness.key_press(Key::Escape);
        harness.run();
        assert!(harness.query_by_label("File…").is_none());
        assert!(harness.query_by_label("New").is_some(), "the menu stays");
    }

    #[test]
    fn left_closes_and_right_opens() {
        let mut harness = harness();
        harness.get_by_label("New").hover();
        harness.run();
        assert!(harness.query_by_label("File…").is_some());
        harness.key_press(Key::ArrowLeft);
        harness.run();
        harness.run();
        assert!(harness.query_by_label("File…").is_none());
        assert!(harness.query_by_label("New").is_some());
        harness.key_press(Key::ArrowRight);
        harness.run();
        harness.run();
        assert!(harness.query_by_label("File…").is_some());
    }

    #[test]
    fn moving_to_another_item_closes_it() {
        let mut harness = harness();
        harness.get_by_label("New").hover();
        harness.run();
        harness.get_by_label("Open").hover();
        harness.run();
        harness.run();
        assert!(harness.query_by_label("File…").is_none());
    }

    #[test]
    fn disabled_parent_does_not_open() {
        let mut harness = harness();
        harness.get_by_label("Locked").hover();
        harness.run();
        harness.run();
        assert!(harness.query_by_label("Never shown").is_none());
        assert!(harness.get_by_label("Locked").is_disabled());
    }

    #[test]
    fn submenu_item_conforms() {
        testing::conformance("New", |ui, enabled| {
            ui.add(
                ActionMenuItem::new("New")
                    .submenu(|ui| {
                        ui.add(ActionMenuItem::new("File…"));
                    })
                    .enabled(enabled),
            )
        })
        .run();
    }

    /// The open submenu, in both themes (the docs image).
    #[test]
    fn open_submenu_snapshot() {
        for theme in [Theme::dark(), Theme::light()] {
            let suffix = if theme.dark { "dark" } else { "light" };
            let mut harness = testing::harness(theme, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(200.0);
                    ui.add(
                        ActionMenuItem::new("Open")
                            .icon(Glyph::Folder)
                            .shortcut("Enter"),
                    );
                    ui.add(ActionMenuItem::new("New").submenu(|ui| {
                        ui.add(ActionMenuItem::new("File…").icon(Glyph::File));
                        ui.add(ActionMenuItem::new("Directory…").icon(Glyph::Folder));
                    }));
                    ui.add(ActionMenuItem::new("Delete").shortcut("Del"));
                });
            });
            harness.set_size(egui::vec2(460.0, 150.0));
            harness.run();
            let rect = harness.get_by_label("New").rect();
            harness.hover_at(rect.center() - egui::vec2(20.0, 0.0));
            harness.run();
            harness.hover_at(rect.center());
            harness.run();
            harness.run();
            assert!(harness.query_by_label("File…").is_some());
            harness
                .try_snapshot_options(
                    format!("action_menu_item_submenu.{suffix}"),
                    &testing::SnapshotOptions::new().output_path(super::common::IMG),
                )
                .expect("snapshot matches");
        }
    }
}

/// Whether the item labeled `label` is painted highlighted: its row has the selection pill.
fn highlighted(harness: &egui_kittest::Harness<'_>, label: &str) -> bool {
    let rect = harness.get_by_label(label).rect();
    let selection = Theme::dark().colors.selection;
    harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Rect(r) => r.fill == selection && r.rect.center().y == rect.center().y,
            _ => false,
        })
}

fn three_items() -> egui_kittest::Harness<'static> {
    testing::harness(Theme::dark(), |ui| {
        ui.add(ActionMenuItem::new("New Project…"));
        ui.add(ActionMenuItem::new("Open…"));
        ui.add(ActionMenuItem::new("Clone Repository…"));
    })
}

#[test]
fn menu_rows_touch_whatever_the_parent_spacing() {
    let harness = three_items();
    let first = harness.get_by_label("New Project…").rect();
    let second = harness.get_by_label("Open…").rect();
    assert_eq!(second.top(), first.bottom());
}

#[test]
fn first_pointer_move_after_opening_is_ignored() {
    // As in Android Studio: a menu that opens under the pointer highlights nothing until the
    // pointer really moves over it.
    let mut harness = three_items();
    let open = harness.get_by_label("Open…").rect().center();
    harness.hover_at(open);
    harness.run();
    assert!(!highlighted(&harness, "Open…"));
    harness.hover_at(open + egui::vec2(1.0, 0.0));
    harness.run();
    assert!(highlighted(&harness, "Open…"));
}

#[test]
fn highlight_stays_when_the_pointer_leaves_and_keys_continue_from_it() {
    let mut harness = three_items();
    let open = harness.get_by_label("Open…").rect().center();
    harness.hover_at(open);
    harness.run();
    harness.hover_at(open + egui::vec2(1.0, 0.0));
    harness.run();
    harness.hover_at(egui::pos2(-50.0, -50.0));
    harness.run();
    assert!(
        highlighted(&harness, "Open…"),
        "stays on the last hovered item"
    );
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert!(highlighted(&harness, "Clone Repository…"));
    assert!(!highlighted(&harness, "Open…"), "one highlight per menu");
}
