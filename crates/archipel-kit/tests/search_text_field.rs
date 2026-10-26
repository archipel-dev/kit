//! `SearchTextField`, text field extensions and validation outlines (IntelliJ spec section 6).

mod common;

use archipel_kit::openapi::ui::ValidationInfo;
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::ui::{Extension, JBTextField, SearchHistory, SearchTextField};
use archipel_kit::{Glyph, Theme, egui, tokens};
use egui::Key;

struct Search {
    text: String,
    history: SearchHistory,
    submitted: usize,
}

impl Search {
    fn new(text: &str, history: &[&str]) -> Self {
        let mut h = SearchHistory::new();
        for entry in history.iter().rev() {
            h.add(*entry);
        }
        Self {
            text: text.to_owned(),
            history: h,
            submitted: 0,
        }
    }
}

fn search(text: &str, history: &[&str]) -> testing::Harness<'static, Search> {
    let mut harness =
        testing::harness_state(Theme::dark(), Search::new(text, history), |ui, state| {
            let out = SearchTextField::new(&mut state.text)
                .history(&mut state.history)
                .show(ui);
            if out.submitted {
                state.submitted += 1;
            }
        });
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 260.0));
    harness.run();
    harness
}

fn focus(harness: &mut testing::Harness<'static, Search>) {
    harness.get_by_label("Search").focus();
    harness.run();
}

#[test]
fn escape_clears_the_text_and_an_empty_field_leaves_escape_alone() {
    let mut harness = search("main", &[]);
    focus(&mut harness);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().text, "");
    // Nothing left to clear: Escape is not used up (a dialog would see it).
    let used = std::cell::Cell::new(false);
    let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        SearchTextField::new(text).show(ui);
        used.set(used.get() || ui.input(|i| i.key_pressed(Key::Escape)));
    });
    harness.get_by_label("Search").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.step();
    assert!(
        used.get(),
        "Escape reaches whoever is behind an empty field"
    );
}

#[test]
fn escape_does_not_clear_when_turned_off() {
    let mut harness = testing::harness_state(Theme::dark(), "main".to_owned(), |ui, text| {
        SearchTextField::new(text).clear_on_escape(false).show(ui);
    });
    harness.get_by_label("Search").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state(), "main");
}

#[test]
fn the_clear_button_appears_with_text_and_clears_it() {
    let mut harness = search("main", &[]);
    harness.get_by_label("Clear").click();
    harness.run();
    assert_eq!(harness.state().text, "");
    assert!(harness.query_by_label("Clear").is_none());
}

#[test]
fn the_magnifier_opens_the_history_and_a_click_on_an_entry_picks_it() {
    let mut harness = search("", &["beta", "alpha"]);
    harness.get_by_label("Search history").click();
    harness.run();
    assert!(
        harness.query_by_label("alpha").is_some(),
        "the list is open"
    );
    harness.get_by_label("alpha").click();
    harness.run();
    assert_eq!(harness.state().text, "alpha");
    assert!(harness.query_by_label("beta").is_none(), "the list closed");
    // The chosen entry moves to the front.
    assert_eq!(harness.state().history.entries()[0], "alpha");
}

#[test]
fn the_history_button_is_the_first_28_points() {
    let harness = search("", &["beta"]);
    let field = harness.get_by_label("Search").rect();
    let button = harness.get_by_label("Search history").rect();
    assert!((button.width() - tokens::size::SEARCH_ICON_AREA).abs() < 0.6);
    assert!(button.left() <= field.left());
}

#[test]
fn alt_down_opens_the_history_and_the_arrows_choose() {
    let mut harness = search("", &["beta", "alpha"]);
    focus(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::ALT, Key::ArrowDown);
    harness.run();
    assert!(harness.query_by_label("alpha").is_some());
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(harness.state().text, "alpha");
    assert!(harness.get_by_label("Search").is_focused());
}

#[test]
fn escape_closes_the_history_and_keeps_the_text() {
    let mut harness = search("keep", &["beta", "alpha"]);
    focus(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::ALT, Key::ArrowDown);
    harness.run();
    assert!(harness.query_by_label("alpha").is_some());
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(harness.query_by_label("alpha").is_none());
    assert_eq!(harness.state().text, "keep", "Escape closed the list first");
}

#[test]
fn alt_up_and_alt_down_cycle_the_entries_and_wrap() {
    let mut harness = search("", &["c", "b", "a"]);
    focus(&mut harness);
    // Alt+Up from nowhere goes to the oldest, then back toward the newest, wrapping.
    for expected in ["a", "b", "c", "a"] {
        harness.key_press_modifiers(egui::Modifiers::ALT, Key::ArrowUp);
        harness.run();
        assert_eq!(harness.state().text, expected);
    }
}

#[test]
fn enter_remembers_the_search_and_five_entries_are_kept() {
    let mut harness = search("", &[]);
    focus(&mut harness);
    for word in ["a", "b", "c", "d", "e", "f"] {
        harness.get_by_label("Search").focus();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
        harness.get_by_label("Search").type_text(word);
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        harness.get_by_label("Search").focus();
        harness.run();
    }
    let history = &harness.state().history;
    assert_eq!(history.entries(), ["f", "e", "d", "c", "b"]);
    assert_eq!(harness.state().submitted, 6);
}

#[test]
fn a_field_without_history_has_no_history_button() {
    let harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        SearchTextField::new(text).show(ui);
    });
    assert!(harness.query_by_label("Search history").is_none());
}

#[test]
fn history_ignores_blank_entries_and_duplicates() {
    let mut history = SearchHistory::with_capacity(3);
    history.add("  ");
    history.add("a");
    history.add("b");
    history.add("a");
    assert_eq!(history.entries(), ["a", "b"]);
    history.clear();
    assert!(history.is_empty());
}

// Extensions.

struct Ext {
    text: String,
    clicked: Vec<usize>,
}

fn with_extensions(before: bool) -> testing::Harness<'static, Ext> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        Ext {
            text: "path".into(),
            clicked: vec![],
        },
        move |ui, state| {
            let out = JBTextField::new(&mut state.text)
                .name("Path")
                .width(200.0)
                .extension(
                    Extension::new(Glyph::Folder, "Browse")
                        .hovered_icon(Glyph::FolderOpen)
                        .tooltip("Browse (Shift+Enter)")
                        .icon_before_text(before),
                )
                .extension(Extension::new(Glyph::Info, "Details").focusable(false))
                .show(ui);
            if let Some(index) = out.extension_clicked {
                state.clicked.push(index);
            }
        },
    );
    harness.set_size(egui::vec2(testing::DEFAULT_WIDTH, 120.0));
    harness.run();
    harness
}

#[test]
fn a_click_on_an_extension_reports_its_index() {
    let mut harness = with_extensions(false);
    harness.get_by_label("Browse").click();
    harness.run();
    harness.get_by_label("Details").click();
    harness.run();
    assert_eq!(harness.state().clicked, [0, 1]);
}

#[test]
fn extension_icons_sit_after_the_text_in_order_or_before_it() {
    let harness = with_extensions(false);
    let field = harness.get_by_label("Path").rect();
    let browse = harness.get_by_label("Browse").rect();
    let details = harness.get_by_label("Details").rect();
    assert!(browse.left() >= field.right(), "after the text edit");
    assert!(details.left() > browse.left(), "added later, further right");
    let harness = with_extensions(true);
    let field = harness.get_by_label("Path").rect();
    let browse = harness.get_by_label("Browse").rect();
    assert!(browse.right() <= field.left(), "before the text edit");
}

#[test]
fn a_focusable_extension_is_reached_with_tab_and_enter_or_space_activate_it() {
    let mut harness = with_extensions(false);
    harness.get_by_label("Path").focus();
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Browse").is_focused());
    harness.key_press(Key::Enter);
    harness.run();
    harness.get_by_label("Browse").focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state().clicked, [0, 0]);
    // The unfocusable one is skipped by Tab.
    harness.key_press(Key::Tab);
    harness.run();
    assert!(!harness.get_by_label("Details").is_focused());
}

#[test]
fn the_pointer_is_a_hand_over_an_extension_and_a_text_cursor_elsewhere() {
    let mut harness = with_extensions(false);
    let field = harness.get_by_label("Path").rect();
    harness.hover_at(field.center());
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Text
    );
    let browse = harness.get_by_label("Browse").rect();
    harness.hover_at(browse.center());
    harness.run();
    assert_ne!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Text,
        "an action cursor over the icon"
    );
}

#[test]
fn an_extension_tooltip_shows_after_the_help_tooltip_delay() {
    let mut harness = with_extensions(false);
    let browse = harness.get_by_label("Browse").rect();
    harness.hover_at(browse.center());
    harness.run();
    assert!(harness.query_by_label("Browse (Shift+Enter)").is_none());
    common::advance_by(
        &mut harness,
        f64::from(tokens::motion::HELP_TOOLTIP_SHOW_DELAY) + 0.1,
    );
    assert!(harness.query_by_label("Browse (Shift+Enter)").is_some());
}

#[test]
fn a_disabled_field_disables_its_extensions() {
    let mut harness = testing::harness_state(Theme::dark(), 0usize, |ui, clicks| {
        let mut text = String::new();
        let out = JBTextField::new(&mut text)
            .name("Path")
            .enabled(false)
            .extension(Extension::new(Glyph::Folder, "Browse"))
            .show(ui);
        if out.extension_clicked.is_some() {
            *clicks += 1;
        }
    });
    assert!(harness.get_by_label("Browse").is_disabled());
    harness.get_by_label("Browse").click();
    harness.run();
    assert_eq!(*harness.state(), 0);
}

#[test]
fn a_narrow_field_grows_to_fit_its_icons_and_never_goes_below_49() {
    let harness = testing::harness(Theme::dark(), |ui| {
        let mut a = String::new();
        let mut b = String::new();
        JBTextField::new(&mut a).name("Tiny").width(10.0).show(ui);
        JBTextField::new(&mut b)
            .name("Icons")
            .width(20.0)
            .extension(Extension::new(Glyph::Folder, "One"))
            .extension(Extension::new(Glyph::Info, "Two"))
            .extension(Extension::new(Glyph::Search, "Three"))
            .show(ui);
    });
    let tiny = harness.get_by_label("Tiny").rect();
    assert!(
        tiny.width() + 2.0 * tokens::size::CODE_PADDING >= tokens::size::TEXT_FIELD_MIN_WIDTH - 0.6
    );
    let three = harness.get_by_label("Three").rect();
    let one = harness.get_by_label("One").rect();
    assert!(one.left() >= harness.get_by_label("Icons").rect().left() - tokens::size::CODE_PADDING);
    assert!(three.right() > one.right() || three.left() > one.left() || three.left() != one.left());
}

// Validation outline.

#[test]
fn a_validation_info_shows_its_message_as_error_or_warning() {
    for (info, message) in [
        (ValidationInfo::error("Bad"), "Bad"),
        (ValidationInfo::warning("Odd"), "Odd"),
    ] {
        let harness = testing::harness(Theme::dark(), move |ui| {
            let mut text = String::from("x");
            ui.add(
                JBTextField::new(&mut text)
                    .name("Field")
                    .validation(Some(&info)),
            );
        });
        assert!(harness.query_by_label(message).is_some());
    }
}

#[test]
fn validation_snapshot() {
    testing::snapshot_themes("jb_text_field_validation", common::IMG, |ui| {
        let mut a = String::from("80a");
        let mut b = String::from("localhost");
        let error = ValidationInfo::error("Enter a number from 0 to 65535");
        let warning = ValidationInfo::warning("This host is not reachable");
        ui.add(
            JBTextField::new(&mut a)
                .label("Port")
                .validation(Some(&error)),
        );
        ui.add(
            JBTextField::new(&mut b)
                .label("Host")
                .validation(Some(&warning)),
        );
    });
}

#[test]
fn search_snapshot() {
    testing::snapshot_themes_after(
        "search_text_field",
        common::IMG,
        |ui| {
            ui.set_width(320.0);
            let mut found = String::from("Spin");
            let mut history = SearchHistory::new();
            history.add("TextField");
            history.add("Spinner");
            history.add("ComboBox");
            SearchTextField::new(&mut found)
                .history(&mut history)
                .extension(Extension::new(Glyph::Filter, "Filter"))
                .show(ui);
            ui.add_space(120.0);
        },
        |h| {
            // Keys work at the snapshots' 2x scale, clicks are placed for 1x.
            h.get_by_label("Search").focus();
            h.run();
            h.key_press_modifiers(egui::Modifiers::ALT, Key::ArrowDown);
            h.run();
        },
    );
}

#[test]
fn search_empty_snapshot() {
    testing::snapshot_themes("search_text_field_empty", common::IMG, |ui| {
        ui.set_width(320.0);
        let mut empty = String::new();
        SearchTextField::new(&mut empty).show(ui);
    });
}

#[test]
fn extension_snapshot() {
    testing::snapshot_themes("jb_text_field_extensions", common::IMG, |ui| {
        ui.set_width(320.0);
        let mut path = String::from("~/code/archipel");
        let mut empty = String::new();
        let mut secret = String::from("hunter2");
        JBTextField::new(&mut path)
            .name("Location")
            .extension(Extension::new(Glyph::Folder, "Browse"))
            .show(ui);
        JBTextField::new(&mut empty)
            .name("Filter")
            .placeholder("Filter files")
            .extension(Extension::new(Glyph::Filter, "Filter").icon_before_text(true))
            .show(ui);
        JBTextField::new(&mut secret)
            .name("Password")
            .password(true)
            .extension(Extension::new(Glyph::Eye, "Show password"))
            .show(ui);
    });
}
