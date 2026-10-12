//! `SegmentedControl`, `TabBar` and `ListRow` tests: behavior, accessibility and snapshots.

mod common;

use archipel_kit::components::{ListRow, MenuItem, SegmentedControl, Tab, TabBar};
use archipel_kit::testing::{self, NodeExt, NodeT, Queryable, Role};
use archipel_kit::{Glyph, Icon, Theme, Tone};

#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Pretty,
    Raw,
    Preview,
}

const VIEWS: [(View, &str); 3] = [
    (View::Pretty, "Pretty"),
    (View::Raw, "Raw"),
    (View::Preview, "Preview"),
];

/// Draws `add` at the top of the test island, flush with its edges, where an editor's tabs
/// are: the island's content padding is for the content below the header. `width` is the
/// width of the island's content.
fn at_island_top(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let inset = archipel_kit::tokens::size::CONTENT_INSET;
    let header = archipel_kit::tokens::size::HEADER_HEIGHT;
    let rect = egui::Rect::from_min_size(
        ui.max_rect().min - egui::vec2(inset, inset),
        egui::vec2(width + 2.0 * inset, header),
    );
    let mut top = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    add(&mut top);
    ui.allocate_space(egui::vec2(width, header - inset));
}

/// Wide enough for the three tabs of [`tabs`].
const WIDE: f32 = 420.0;

fn tabs() -> Vec<Tab> {
    vec![
        Tab::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .closable(true),
        Tab::new("Cargo.toml")
            .icon(Icon::for_file("Cargo.toml"))
            .closable(true)
            .modified(true),
        Tab::new("README.md")
            .icon(Icon::for_file("README.md"))
            .closable(true),
    ]
}

#[test]
fn segmented_snapshot() {
    testing::snapshot_themes("segmented_control", common::IMG, |ui| {
        ui.add(SegmentedControl::new(&mut View::Pretty, VIEWS));
        ui.add(SegmentedControl::new(&mut View::Raw, VIEWS).enabled(false));
    });
}

#[test]
fn segmented_selects_clicked_option() {
    let mut harness = testing::harness_state(Theme::dark(), View::Pretty, |ui, view| {
        ui.add(SegmentedControl::new(view, VIEWS));
    });
    assert!(harness.get_by_label("Pretty").is_checked());
    harness.get_by_label("Raw").click();
    harness.run();
    assert_eq!(*harness.state(), View::Raw);
    assert!(harness.get_by_label("Raw").is_checked());
    assert!(!harness.get_by_label("Pretty").is_checked());
}

#[test]
fn tab_bar_snapshot() {
    testing::snapshot_themes("tab_bar", common::IMG, |ui| {
        at_island_top(ui, WIDE, |ui| {
            TabBar::new(&tabs(), 0).show(ui);
        });
    });
}

#[test]
fn tab_bar_reports_selection_and_close() {
    let tabs = tabs();
    let mut harness =
        testing::harness_state(Theme::dark(), (0usize, None), |ui, (current, closed)| {
            ui.set_width(WIDE);
            let out = TabBar::new(&tabs, *current).show(ui);
            if let Some(index) = out.selected {
                *current = index;
            }
            if out.closed.is_some() {
                *closed = out.closed;
            }
        });
    harness.get_by_label("README.md").click();
    harness.run();
    assert_eq!(harness.state().0, 2);
    assert!(harness.get_by_label("README.md").is_checked());
    harness.get_by_label("Close README.md").click();
    harness.run();
    assert_eq!(harness.state().1, Some(2));
}

/// More tabs than fit in the 360 pt test island: they overflow.
fn many_tabs() -> Vec<Tab> {
    MANY.into_iter()
        .map(|name| Tab::new(name).icon(Icon::for_file(name)).closable(true))
        .collect()
}

const MANY: [&str; 8] = [
    "main.rs",
    "lib.rs",
    "theme.rs",
    "tokens.rs",
    "Cargo.toml",
    "README.md",
    "segmented.rs",
    "navigation.rs",
];

fn many_tabs_labels() -> [&'static str; 8] {
    MANY
}

/// The width the tab bars of these tests are drawn in.
const STRIP_WIDTH: f32 = 340.0;

/// The tab node (not a hidden-tabs menu item) labelled `label`.
fn tab_node<'h>(
    harness: &'h egui_kittest::Harness<'_, TabsState>,
    label: &'h str,
) -> egui_kittest::Node<'h> {
    harness
        .query_all_by_label(label)
        .find(|node| in_tab_list(node))
        .unwrap_or_else(|| panic!("no tab {label:?}"))
}

/// Whether `node` is a tab of a tab bar (not an item of its hidden tabs list).
fn in_tab_list(node: &egui_kittest::Node<'_>) -> bool {
    node.parent()
        .is_some_and(|parent| parent.accesskit_node().role() == Role::TabList)
}

#[derive(Default)]
struct TabsState {
    current: usize,
    more_clicked: bool,
}

fn overflowing(theme: Theme) -> egui_kittest::Harness<'static, TabsState> {
    let tabs = many_tabs();
    let mut harness = testing::harness_state(theme, TabsState::default(), move |ui, state| {
        ui.set_width(STRIP_WIDTH);
        let out = TabBar::new(&tabs, state.current)
            .more_menu(|ui| {
                if ui.add(MenuItem::new("Close All Tabs")).clicked() {
                    state.more_clicked = true;
                }
            })
            .show(ui);
        if let Some(index) = out.selected {
            state.current = index;
        }
    });
    // Room below the strip for its popups.
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 320.0));
    harness.run();
    harness
}

fn wheel(harness: &mut egui_kittest::Harness<'_, TabsState>, delta: egui::Vec2, shift: bool) {
    // Over the tabs, just before the hidden tabs button.
    let caret = harness.get_by_label("Show hidden tabs").rect();
    let pos = egui::pos2(caret.left() - 40.0, caret.center().y);
    harness.hover_at(pos);
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta,
        modifiers: if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        },
    });
    harness.run();
}

#[test]
fn tab_bar_starts_inset_from_the_island_edge() {
    let tabs = tabs();
    let harness = testing::harness_state(Theme::dark(), 0.0_f32, |ui, left| {
        ui.set_width(WIDE);
        *left = ui.max_rect().left();
        TabBar::new(&tabs, 0).show(ui);
    });
    let first = harness.get_by_label("main.rs").rect();
    assert_eq!(
        first.left() - *harness.state(),
        archipel_kit::tokens::tab::STRIP_INSET,
        "the first pill starts 6 pt from the edge"
    );
    assert!(harness.query_by_label("Show hidden tabs").is_none());
}

#[test]
fn wheel_scrolls_overflowing_tabs_sideways() {
    for (delta, shift) in [
        (egui::vec2(0.0, -120.0), false),
        (egui::vec2(-120.0, 0.0), false),
        (egui::vec2(0.0, -120.0), true),
    ] {
        let mut harness = overflowing(Theme::dark());
        let before = tab_node(&harness, "lib.rs").rect().left();
        wheel(&mut harness, delta, shift);
        let after = tab_node(&harness, "lib.rs").rect().left();
        assert!(
            after < before - 50.0,
            "{delta:?} (shift {shift}) moved the tabs from {before} to {after}"
        );
        // And back: wheel up scrolls toward the first tab, which stops at the inset.
        wheel(&mut harness, -delta * 10.0, shift);
        assert_eq!(tab_node(&harness, "lib.rs").rect().left(), before);
    }
}

#[test]
fn tabs_that_fit_do_not_scroll() {
    let tabs = tabs();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.set_width(WIDE);
        TabBar::new(&tabs, 0).show(ui);
    });
    let before = harness.get_by_label("README.md").rect();
    harness.hover_at(harness.get_by_label("main.rs").rect().center());
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    assert_eq!(harness.get_by_label("README.md").rect(), before);
}

#[test]
fn selecting_a_hidden_tab_scrolls_it_into_view() {
    let mut harness = overflowing(Theme::dark());
    let caret = harness.get_by_label("Show hidden tabs").rect();
    assert!(
        tab_node(&harness, "navigation.rs").rect().left() > caret.left(),
        "the last tab starts out of view"
    );
    // The app selects it (a keyboard shortcut, a new tab…).
    harness.state_mut().current = 7;
    harness.run();
    let last = tab_node(&harness, "navigation.rs").rect();
    assert!(
        last.right() <= caret.left() && last.left() >= 0.0,
        "the selected tab is in view: {last:?}, caret at {caret:?}"
    );
    // And back to the first one.
    harness.state_mut().current = 0;
    harness.run();
    let first = tab_node(&harness, "main.rs").rect();
    let lib = tab_node(&harness, "lib.rs").rect();
    assert_eq!(
        lib.left() - first.right(),
        archipel_kit::tokens::tab::GAP,
        "tabs keep their spacing"
    );
    assert!(first.left() > 0.0 && first.right() < caret.left());
}

#[test]
fn dragging_the_thumb_scrolls_the_tabs() {
    let mut harness = overflowing(Theme::dark());
    let first = tab_node(&harness, "main.rs").rect();
    let before = tab_node(&harness, "lib.rs").rect().left();
    // The thumb lies along the strip's top edge, from its left end.
    let grab = egui::pos2(first.left() + 20.0, first.top() + 2.0);
    harness.hover_at(grab);
    harness.run();
    harness.event(egui::Event::PointerButton {
        pos: grab,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    for step in 1..=4 {
        harness.hover_at(grab + egui::vec2(15.0 * step as f32, 0.0));
        harness.run();
    }
    harness.event(egui::Event::PointerButton {
        pos: grab + egui::vec2(60.0, 0.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    let after = tab_node(&harness, "lib.rs").rect().left();
    assert!(
        after < before - 60.0,
        "the drag moved the tabs from {before} to {after}"
    );
    assert_eq!(
        harness.state().current,
        0,
        "dragging the thumb selects nothing"
    );
}

#[test]
fn hidden_tabs_list_selects_and_reveals_a_tab() {
    let mut harness = overflowing(Theme::dark());
    harness.get_by_label("Show hidden tabs").click();
    harness.run();
    // Only the tabs out of view are listed, with their icons and names.
    let listed = |harness: &egui_kittest::Harness<'_, TabsState>, label: &str| {
        harness
            .query_all_by_label(label)
            .any(|node| !in_tab_list(&node))
    };
    assert!(listed(&harness, "navigation.rs"));
    assert!(!listed(&harness, "main.rs"), "visible tabs are not listed");
    harness
        .query_all_by_label("navigation.rs")
        .find(|node| !in_tab_list(node))
        .expect("listed")
        .click();
    harness.run();
    assert_eq!(harness.state().current, 7);
    assert!(!listed(&harness, "navigation.rs"), "the list closed");
    let caret = harness.get_by_label("Show hidden tabs").rect();
    let last = tab_node(&harness, "navigation.rs").rect();
    assert!(last.right() <= caret.left(), "the picked tab is in view");
}

#[test]
fn hidden_tabs_list_works_from_the_keyboard() {
    let mut harness = overflowing(Theme::dark());
    harness.get_by_label("Show hidden tabs").focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let menu_items = |harness: &egui_kittest::Harness<'_, TabsState>| {
        many_tabs_labels()
            .iter()
            .flat_map(|label| harness.query_all_by_label(label).collect::<Vec<_>>())
            .filter(|node| !in_tab_list(node))
            .count()
    };
    assert!(menu_items(&harness) > 0, "Enter opens the list");
    // The first row has the focus; Down moves to the next, Enter picks it.
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_ne!(harness.state().current, 0, "a hidden tab was picked");
    assert_eq!(menu_items(&harness), 0, "picking closes the list");
    // Escape closes it and gives the focus back to the button.
    harness.get_by_label("Show hidden tabs").focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(menu_items(&harness) > 0);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(menu_items(&harness), 0, "Escape closes the list");
    assert!(
        harness
            .get_by_label("Show hidden tabs")
            .accesskit_node()
            .is_focused()
    );
}

#[test]
fn more_actions_menu_runs_the_apps_actions() {
    let mut harness = overflowing(Theme::dark());
    harness.get_by_label("More actions").click();
    harness.run();
    harness.get_by_label("Close All Tabs").click();
    harness.run();
    assert!(harness.state().more_clicked);
    assert!(
        harness.query_by_label("Close All Tabs").is_none(),
        "a click closes the menu"
    );
    harness.get_by_label("More actions").click();
    harness.run();
    assert!(harness.query_by_label("Close All Tabs").is_some());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        harness.query_by_label("Close All Tabs").is_none(),
        "Escape closes the menu"
    );
}

#[test]
fn tab_bar_overflow_snapshot() {
    let tabs = many_tabs();
    testing::snapshot_themes_after(
        "tab_bar_overflow",
        common::IMG,
        |ui| {
            at_island_top(ui, STRIP_WIDTH, |ui| {
                TabBar::new(&tabs, 2)
                    .more_menu(|ui| {
                        ui.add(MenuItem::new("Close All Tabs"));
                    })
                    .show(ui);
            });
        },
        // The pointer over the strip shows the scroll bar at its top edge.
        |harness| {
            let rect = harness.get_by_label("lib.rs").rect();
            let center = rect.center() / testing::SNAPSHOT_SCALE;
            harness.hover_at(center);
            harness.run();
        },
    );
}

#[test]
fn tab_bar_hidden_tabs_snapshot() {
    let tabs = many_tabs();
    testing::snapshot_themes_after(
        "tab_bar_hidden_tabs",
        common::IMG,
        |ui| {
            at_island_top(ui, STRIP_WIDTH, |ui| {
                TabBar::new(&tabs, 2)
                    .more_menu(|ui| {
                        ui.add(MenuItem::new("Close All Tabs"));
                    })
                    .show(ui);
            });
            // Room for the list below the strip.
            ui.add_space(200.0);
        },
        // Opened from the keyboard: the list starts on its first row.
        |harness| {
            harness.get_by_label("Show hidden tabs").focus();
            harness.run();
            harness.key_press(egui::Key::Enter);
            harness.run();
        },
    );
}

#[test]
fn list_row_snapshot() {
    testing::snapshot_themes("list_row", common::IMG, |ui| {
        ListRow::new("archipel")
            .icon(Glyph::Project)
            .expanded(Some(true))
            .show(ui);
        ListRow::new("src")
            .icon(Glyph::Folder)
            .depth(1)
            .expanded(Some(true))
            .show(ui);
        ListRow::new("main.rs")
            .icon(Icon::for_file("main.rs"))
            .depth(2)
            .selected(true)
            .show(ui);
        ListRow::new("theme.rs")
            .icon(Icon::for_file("theme.rs"))
            .depth(2)
            .label_tone(Tone::Modified)
            .show(ui);
        ListRow::new("target")
            .icon(Glyph::Folder)
            .depth(1)
            .expanded(Some(false))
            .secondary("excluded")
            .show(ui);
        ListRow::new("Cargo.toml")
            .icon(Icon::for_file("Cargo.toml"))
            .depth(1)
            .secondary("1 KB")
            .show(ui);
        ListRow::new("Disabled")
            .icon(Glyph::FileText)
            .depth(1)
            .enabled(false)
            .show(ui);
    });
}

#[derive(Default)]
struct TreeState {
    expanded: bool,
    selected: Option<&'static str>,
    opened: Option<&'static str>,
}

fn tree(ui: &mut egui::Ui, state: &mut TreeState) {
    let row = ListRow::new("src")
        .icon(Glyph::Folder)
        .expanded(Some(state.expanded))
        .show(ui);
    if row.toggled {
        state.expanded = !state.expanded;
    }
    if row.response.clicked() {
        state.selected = Some("src");
    }
    if state.expanded {
        let row = ListRow::new("main.rs")
            .depth(1)
            .icon(Glyph::FileText)
            .selected(state.selected == Some("main.rs"))
            .show(ui);
        if row.response.clicked() {
            state.selected = Some("main.rs");
        }
        if row.response.double_clicked() {
            state.opened = Some("main.rs");
        }
    }
}

#[test]
fn list_row_click_selects_and_chevron_expands() {
    let mut harness = testing::harness_state(Theme::dark(), TreeState::default(), tree);
    assert!(harness.query_by_label("main.rs").is_none());
    // The chevron sits at the start of the row.
    let row = harness.get_by_label("src").rect();
    harness.hover_at(egui::pos2(row.left() + 14.0, row.center().y));
    harness.event(egui::Event::PointerButton {
        pos: egui::pos2(row.left() + 14.0, row.center().y),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos: egui::pos2(row.left() + 14.0, row.center().y),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    harness.run();
    assert!(harness.state().expanded);
    harness.get_by_label("main.rs").click();
    harness.run();
    assert_eq!(harness.state().selected, Some("main.rs"));
    assert!(harness.get_by_label("main.rs").is_checked());
}

#[test]
fn arrow_keys_expand_and_collapse_a_focused_row() {
    let mut harness = testing::harness_state(Theme::dark(), TreeState::default(), tree);
    harness.get_by_label("src").focus();
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert!(harness.state().expanded);
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().expanded);
}

#[test]
fn double_click_toggles_a_node() {
    let mut harness = testing::harness_state(Theme::dark(), TreeState::default(), tree);
    let center = harness.get_by_label("src").rect().center();
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
        }
    }
    harness.run();
    assert!(harness.state().expanded);
}
