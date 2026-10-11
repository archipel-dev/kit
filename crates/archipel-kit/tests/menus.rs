//! `Dropdown`, `MenuItem` and `Link` tests.

mod common;

use archipel_kit::components::{Dropdown, Link, MenuItem, MenuSeparator};
use archipel_kit::parts::Slot;
use archipel_kit::testing::{self, NodeExt, Queryable, Role};
use archipel_kit::{Glyph, Theme};

#[test]
fn menu_snapshot() {
    testing::snapshot_themes("menu_item", common::IMG, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(240.0);
            ui.add(MenuItem::new("Copy").icon(Glyph::Copy).shortcut("Ctrl+C"));
            ui.add(
                MenuItem::new("Find in Files…")
                    .icon(Glyph::FindInFiles)
                    .shortcut("Ctrl+Shift+F"),
            );
            ui.add(MenuSeparator);
            ui.add(MenuItem::new("Soft-Wrap").checked(true));
            ui.add(MenuItem::new("Run").icon(Glyph::Run).enabled(false));
        });
    });
}

#[test]
fn menu_item_clicks_and_reports_check_state() {
    let mut harness = testing::harness_state(Theme::dark(), (0, true), |ui, (copies, wrap)| {
        if ui.add(MenuItem::new("Copy").shortcut("Ctrl+C")).clicked() {
            *copies += 1;
        }
        if ui.add(MenuItem::new("Soft-Wrap").checked(*wrap)).clicked() {
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
fn dropdown_snapshot() {
    testing::snapshot_themes("dropdown", common::IMG, |ui| {
        ui.add(Dropdown::new("method", &mut "GET", ["GET", "POST"]).width(120.0));
        ui.add(
            Dropdown::new("off", &mut "Disabled", ["Disabled"])
                .width(120.0)
                .enabled(false),
        );
    });
}

#[test]
fn dropdown_picks_an_option() {
    let mut harness = testing::harness_state(Theme::dark(), "GET", |ui, method| {
        ui.add(Dropdown::new("method", method, ["GET", "POST", "PUT"]));
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

/// The color of an environment, from the theme.
fn env_color(colors: &archipel_kit::theme::Colors, env: &str) -> egui::Color32 {
    if env == "Local" {
        colors.success
    } else {
        colors.warning
    }
}

fn env_dot_slot(color: egui::Color32) -> Slot<'static> {
    Slot::ui_width(16.0, move |ui| {
        ui.painter()
            .circle_filled(ui.max_rect().center(), 4.0, color);
    })
}

#[test]
fn dropdown_leading_snapshot() {
    testing::snapshot_themes("dropdown_leading", common::IMG, |ui| {
        let colors = Theme::colors(ui.ctx());
        ui.add(
            Dropdown::new("env", &mut "Local", ["Local", "Staging"])
                .width(160.0)
                .leading(move |env| env_dot_slot(env_color(&colors, env))),
        );
        ui.add(
            Dropdown::new("icon", &mut "Staging", ["Local", "Staging"])
                .width(160.0)
                .leading(|_| Slot::icon(Glyph::Folder)),
        );
        ui.add(
            Dropdown::new("off", &mut "Staging", ["Staging"])
                .width(160.0)
                .leading(|_| Slot::icon(Glyph::Folder))
                .enabled(false),
        );
    });
}

#[test]
fn dropdown_leading_shows_in_the_button_and_the_open_list() {
    let drawn = std::cell::Cell::new(Vec::<String>::new());
    let mut harness = testing::harness_state(Theme::dark(), "GET", |ui, method| {
        ui.add(
            Dropdown::new("method", method, ["GET", "POST"]).leading(|value| {
                let mut seen = drawn.take();
                seen.push(value.to_string());
                drawn.set(seen);
                Slot::icon(Glyph::Folder)
            }),
        );
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 240.0));
    harness.run();
    // Closed: only the selected value asks for its leading content.
    let closed = drawn.take();
    assert!(
        !closed.is_empty() && closed.iter().all(|v| v == "GET"),
        "{closed:?}"
    );
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let open = drawn.take();
    assert!(
        open.iter().any(|v| v == "POST"),
        "options are led: {open:?}"
    );
    harness.get_by_label("POST").click();
    harness.run();
    assert_eq!(*harness.state(), "POST");
    // The button now leads with the new selection.
    harness.run_steps(10);
    drawn.take();
    harness.run();
    assert!(drawn.take().iter().all(|v| v == "POST"));
}

#[test]
fn dropdown_leading_is_audited_and_keeps_its_name() {
    let mut harness = testing::harness_state(Theme::dark(), "Staging", |ui, env| {
        let colors = Theme::colors(ui.ctx());
        ui.add(
            Dropdown::new("env", env, ["Local", "Staging"])
                .label("Environment")
                .leading(move |env| env_dot_slot(env_color(&colors, env))),
        );
    });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 240.0));
    harness.run();
    testing::audit(&mut harness).assert_clean();
    let combo = harness.get_by_role(Role::ComboBox);
    assert_eq!(combo.accessible_label().as_deref(), Some("Environment"));
    // The open list: the options keep their names with the leading content. (The audit is
    // not run on it: egui's popup frame is an unnamed node, with or without a leading slot.)
    combo.click();
    harness.run();
    harness.get_by_label("Local");
    harness.get_by_label("Staging");
}

#[test]
fn link_snapshot() {
    testing::snapshot_themes("link", common::IMG, |ui| {
        ui.horizontal(|ui| {
            ui.label("Gradle JVM is not set.");
            ui.add(Link::new("Configure…"));
        });
    });
}

#[test]
fn link_has_link_role() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, clicked| {
        *clicked |= ui.add(Link::new("Open docs")).clicked();
    });
    let node = harness.get_by_label("Open docs");
    assert_eq!(node.role(), Role::Link);
    node.click();
    harness.run();
    assert!(*harness.state());
}

mod submenu {
    use archipel_kit::behaviour::ContextMenu;
    use archipel_kit::components::{ListRow, MenuItem};
    use archipel_kit::testing::{self, NodeExt, NodeT, Queryable};
    use archipel_kit::{Glyph, Theme};
    use egui::Key;

    /// A row whose context menu has "Open" and a "New" submenu with two items; the state
    /// counts what was clicked.
    fn harness() -> testing::Harness<'static, Vec<&'static str>> {
        let mut harness = testing::harness_state(Theme::dark(), Vec::new(), |ui, clicked| {
            let cell = std::cell::RefCell::new(Vec::new());
            ListRow::new("src")
                .with(ContextMenu::new(|ui| {
                    if ui.add(MenuItem::new("Open").icon(Glyph::Folder)).clicked() {
                        cell.borrow_mut().push("open");
                    }
                    ui.add(MenuItem::new("New").submenu(|ui| {
                        if ui.add(MenuItem::new("File…")).clicked() {
                            cell.borrow_mut().push("file");
                        }
                        ui.add(MenuItem::new("Directory…"));
                    }));
                    ui.add(
                        MenuItem::new("Locked")
                            .submenu(|ui| {
                                ui.add(MenuItem::new("Never shown"));
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
                MenuItem::new("New")
                    .submenu(|ui| {
                        ui.add(MenuItem::new("File…"));
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
                    ui.add(MenuItem::new("Open").icon(Glyph::Folder).shortcut("Enter"));
                    ui.add(MenuItem::new("New").submenu(|ui| {
                        ui.add(MenuItem::new("File…").icon(Glyph::File));
                        ui.add(MenuItem::new("Directory…").icon(Glyph::Folder));
                    }));
                    ui.add(MenuItem::new("Delete").shortcut("Del"));
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
                    format!("menu_item_submenu.{suffix}"),
                    &testing::SnapshotOptions::new().output_path(super::common::IMG),
                )
                .expect("snapshot matches");
        }
    }
}
