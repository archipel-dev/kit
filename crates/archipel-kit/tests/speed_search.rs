//! Speed search of lists, trees and tables (IntelliJ spec section 3.4).

mod common;

use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::ui::{
    Cell, ColoredListCellRenderer, ColumnInfo, JBList, JBTable, SpeedSearch, SpeedSearchComparator,
    SpeedSearchState, Tree, TreeNode,
};
use archipel_kit::{Glyph, Theme};
use egui::{Event, Key, Modifiers};

const NAMES: [&str; 8] = [
    "Cargo.toml",
    "README.md",
    "NullPointerException",
    "src",
    "tests",
    "docs",
    "readme.txt",
    "9lives",
];

#[derive(Default)]
struct State {
    selected: Option<usize>,
    search: SpeedSearchState,
    activated: Option<usize>,
}

fn harness(filter: bool) -> testing::Harness<'static, State> {
    let mut harness = testing::harness_state(Theme::light(), State::default(), move |ui, state| {
        let out = JBList::new("files", NAMES.len(), &mut state.selected)
            .name("Files")
            .height(240.0)
            .speed_search(SpeedSearch::new(&mut state.search).filter(filter))
            .show(ui, |i| ColoredListCellRenderer::new(NAMES[i]));
        if out.activated.is_some() {
            state.activated = out.activated;
        }
    });
    harness.run();
    harness.get_by_label("Cargo.toml").click();
    harness.run();
    harness
}

fn type_text(harness: &mut testing::Harness<'_, State>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

fn press(harness: &mut testing::Harness<'_, State>, key: Key) {
    harness.key_press(key);
    harness.run();
}

#[test]
fn a_typed_letter_opens_the_popup_and_selects_the_first_match() {
    let mut harness = harness(false);
    type_text(&mut harness, "t");
    assert!(harness.state().search.is_active());
    assert_eq!(harness.state().search.text(), "t");
    // "t" starts "Cargo.toml" (after the dot) at its word start; Cargo.toml is current.
    assert_eq!(harness.state().selected, Some(0));
    type_text(&mut harness, "e");
    type_text(&mut harness, "s");
    assert_eq!(harness.state().selected, Some(4), "tests");
    harness.get_by_label("Speed search");
}

#[test]
fn digits_and_punctuation_open_it_but_a_space_alone_does_not() {
    for (text, opens) in [("9", true), (".", true), ("_", true), (" ", false)] {
        let mut harness = harness(false);
        type_text(&mut harness, text);
        assert_eq!(harness.state().search.is_active(), opens, "typing {text:?}");
    }
}

#[test]
fn a_space_inside_the_text_is_part_of_it() {
    let mut harness = harness(false);
    type_text(&mut harness, "r");
    type_text(&mut harness, " ");
    assert_eq!(harness.state().search.text(), "r ");
}

#[test]
fn backspace_edits_and_keeps_the_popup_open_when_emptied() {
    let mut harness = harness(false);
    type_text(&mut harness, "do");
    assert_eq!(harness.state().selected, Some(5));
    press(&mut harness, Key::Backspace);
    assert_eq!(harness.state().search.text(), "d");
    press(&mut harness, Key::Backspace);
    assert_eq!(harness.state().search.text(), "");
    assert!(harness.state().search.is_active(), "stays open when empty");
    press(&mut harness, Key::Backspace);
    assert!(harness.state().search.is_active(), "the key is swallowed");
}

#[test]
fn ctrl_backspace_clears_the_whole_text() {
    let mut harness = harness(false);
    type_text(&mut harness, "tes");
    harness.key_press_modifiers(
        Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        },
        Key::Backspace,
    );
    harness.run();
    assert_eq!(harness.state().search.text(), "");
    assert!(harness.state().search.is_active());
}

#[test]
fn escape_closes_the_popup_and_is_consumed() {
    let mut harness = harness(false);
    type_text(&mut harness, "d");
    press(&mut harness, Key::Escape);
    assert!(!harness.state().search.is_active());
    assert_eq!(harness.state().search.text(), "");
    // The list kept the focus: arrows still work.
    press(&mut harness, Key::ArrowDown);
    assert!(harness.state().selected.is_some());
}

#[test]
fn up_down_home_end_jump_between_matches() {
    let mut harness = harness(false);
    // "r" matches README.md (1), NullPointerException (2: the hump P is not r, but
    // "Exception" has no word starting with r; "Pointer" has an r inside) and readme.txt (6).
    type_text(&mut harness, "re");
    let matches: Vec<usize> = (0..NAMES.len())
        .filter(|i| SpeedSearchComparator::new("re").matches(NAMES[*i]))
        .collect();
    assert!(matches.len() >= 2, "{matches:?}");
    let first = matches[0];
    assert_eq!(harness.state().selected, Some(first));
    press(&mut harness, Key::ArrowDown);
    assert_eq!(harness.state().selected, Some(matches[1]), "next match");
    press(&mut harness, Key::ArrowUp);
    assert_eq!(harness.state().selected, Some(first), "previous match");
    press(&mut harness, Key::End);
    assert_eq!(harness.state().selected, matches.last().copied(), "last");
    press(&mut harness, Key::Home);
    assert_eq!(harness.state().selected, Some(first), "first");
    assert!(harness.state().search.is_active(), "still open");
}

#[test]
fn enter_closes_the_popup_and_activates_the_match() {
    let mut harness = harness(false);
    type_text(&mut harness, "do");
    press(&mut harness, Key::Enter);
    assert!(!harness.state().search.is_active());
    assert_eq!(harness.state().activated, Some(5));
}

#[test]
fn page_keys_and_horizontal_arrows_close_the_popup() {
    for key in [Key::PageDown, Key::PageUp, Key::ArrowLeft, Key::ArrowRight] {
        let mut harness = harness(false);
        type_text(&mut harness, "d");
        press(&mut harness, key);
        assert!(!harness.state().search.is_active(), "{key:?} closes it");
    }
}

#[test]
fn losing_focus_closes_the_popup() {
    let mut harness = harness(false);
    type_text(&mut harness, "d");
    harness.get_by_label("Speed search");
    harness.key_press(Key::Tab);
    harness.run();
    harness.run();
    assert!(!harness.state().search.is_active());
}

#[test]
fn filtering_hides_the_rows_that_do_not_match() {
    let mut harness = harness(true);
    type_text(&mut harness, "re");
    assert!(harness.query_by_label("README.md").is_some());
    assert!(harness.query_by_label("readme.txt").is_some());
    assert!(harness.query_by_label("Cargo.toml").is_none());
    assert!(harness.query_by_label("src").is_none());
    // The selection is on the first visible match.
    assert!(harness.get_by_label("README.md").is_checked());
    // "re" also matches NullPointerException: r of "Pointer", then the hump "Exception".
    assert!(harness.query_by_label("NullPointerException").is_some());
    press(&mut harness, Key::ArrowDown);
    assert!(harness.get_by_label("NullPointerException").is_checked());
}

#[test]
fn filtering_without_matches_says_so() {
    let mut harness = harness(true);
    type_text(&mut harness, "zzz");
    assert!(harness.query_by_label("Cargo.toml").is_none());
    harness.get_by_label("No matches");
    // Deleting the text brings every row back.
    harness.key_press_modifiers(
        Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        },
        Key::Backspace,
    );
    harness.run();
    assert!(harness.query_by_label("Cargo.toml").is_some());
}

#[test]
fn matched_fragments_are_highlighted_in_the_rows() {
    let theme = Theme::light();
    let mut harness = harness(false);
    let count = |harness: &mut testing::Harness<'_, State>| {
        let image = harness.render().expect("renders");
        let m = theme.colors.search_match;
        image
            .pixels()
            .filter(|p| p.0[..3] == [m.r(), m.g(), m.b()])
            .count()
    };
    assert_eq!(count(&mut harness), 0, "no highlight without a search");
    type_text(&mut harness, "do");
    assert!(count(&mut harness) > 20, "the match has a background");
}

#[test]
fn the_popup_text_is_red_when_nothing_matches() {
    let theme = Theme::light();
    let mut harness = harness(false);
    let red = |harness: &mut testing::Harness<'_, State>| {
        let image = harness.render().expect("renders");
        let e = theme.colors.error;
        image
            .pixels()
            .filter(|p| {
                let d = |a: u8, b: u8| i32::from(a).abs_diff(i32::from(b));
                d(p.0[0], e.r()) + d(p.0[1], e.g()) + d(p.0[2], e.b()) < 40
            })
            .count()
    };
    type_text(&mut harness, "d");
    assert_eq!(red(&mut harness), 0);
    type_text(&mut harness, "qqq");
    assert!(red(&mut harness) > 0, "error color text");
}

#[test]
fn a_closed_speed_search_does_nothing_to_the_selection() {
    let mut harness = harness(false);
    assert_eq!(harness.state().selected, Some(0));
    press(&mut harness, Key::Backspace);
    assert!(!harness.state().search.is_active());
    assert_eq!(harness.state().selected, Some(0));
}

#[test]
fn trees_search_their_visible_rows() {
    let rows = vec![
        TreeNode::new("kit", 0).icon(Glyph::Folder).parent(true),
        TreeNode::new("src", 1),
        TreeNode::new("theme.rs", 1),
    ];
    let mut harness = testing::harness_state(Theme::dark(), State::default(), move |ui, state| {
        let _ = Tree::new("tree", &rows, &mut state.selected)
            .speed_search(SpeedSearch::new(&mut state.search))
            .show(ui);
    });
    harness.run();
    harness.get_by_label("kit").click();
    harness.run();
    type_text(&mut harness, "th");
    assert_eq!(harness.state().selected, Some(2));
}

#[test]
fn tables_match_any_cell_of_a_row() {
    let mut columns = vec![
        ColumnInfo::new("Method", 80.0),
        ColumnInfo::new("Path", 160.0),
    ];
    let mut harness = testing::harness_state(Theme::dark(), State::default(), move |ui, state| {
        let _ = JBTable::new("table", &mut columns, 6, &mut state.selected)
            .speed_search(SpeedSearch::new(&mut state.search))
            .show(ui, |row, col| match col {
                0 => Cell::new(if row % 2 == 0 { "GET" } else { "POST" }),
                _ => Cell::new(format!("/orders/{row}")),
            });
    });
    harness.run();
    harness.get_by_label("GET, /orders/0").click();
    harness.run();
    type_text(&mut harness, "po");
    assert_eq!(
        harness.state().selected,
        Some(1),
        "POST in the first column"
    );
}

#[test]
fn the_comparator_matches_words_and_humps_like_the_ide() {
    let m = |pattern: &str, text: &str| SpeedSearchComparator::new(pattern).matches(text);
    // Prefix, case-insensitive.
    assert!(m("car", "Cargo.toml"));
    // Camel humps.
    assert!(m("npe", "NullPointerException"));
    assert!(m("NPE", "NullPointerException"));
    assert!(
        !m("NPE", "nullpointerexception"),
        "uppercase needs uppercase"
    );
    // Words after separators.
    assert!(m("rm", "README.md"));
    assert!(m("tt", "theme.toml"));
    // Anywhere for the first letter, then continue or start a word.
    assert!(m("oml", "Cargo.toml"));
    assert!(!m("zz", "Cargo.toml"));
    // `*` matches anything; a space starts a new word.
    assert!(m("c*ml", "Cargo.toml"));
    assert!(m("cargo toml", "Cargo.toml"));
    assert!(!m("cargo oml", "Cargo.toml"));
    // An empty pattern matches everything.
    assert!(m("", "anything"));
}

#[test]
fn the_comparator_reports_the_matched_fragments_as_character_ranges() {
    let c = SpeedSearchComparator::new("rm");
    assert_eq!(c.matching_fragments("README.md"), Some(vec![0..1, 7..8]));
    let c = SpeedSearchComparator::new("read");
    assert_eq!(c.matching_fragments("README.md"), Some(one(0, 4)));
    let c = SpeedSearchComparator::new("é");
    assert_eq!(c.matching_fragments("café"), Some(one(3, 4)));
    assert_eq!(
        SpeedSearchComparator::new("x").matching_fragments("abc"),
        None
    );
}

#[test]
fn a_pathological_pattern_gives_up_instead_of_hanging() {
    let text = "a".repeat(200);
    let pattern = format!("{}b", "a".repeat(60));
    assert!(!SpeedSearchComparator::new(&pattern).matches(&text));
}

/// One matched fragment.
fn one(start: usize, end: usize) -> Vec<std::ops::Range<usize>> {
    std::iter::once(start..end).collect()
}
