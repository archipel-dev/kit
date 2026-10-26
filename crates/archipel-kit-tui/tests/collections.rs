//! Event-driven tests of lists, trees, tables and speed search.

use archipel_kit::ui::SortOrder;
use archipel_kit_tui::collections::*;
use archipel_kit_tui::components::{Pane, PaneKind, RowIcon};
use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{FnApp, Harness, app_fn, snapshot};
use archipel_kit_tui::{Event, Glyph, Id, Language, Look, Runtime};

const LIST: Id = Id::new("list");
const TREE: Id = Id::new("tree");
const TABLE: Id = Id::new("table");

#[derive(Default)]
struct Screen {
    list: JBList,
    rows: Vec<RowData>,
    events: Vec<CollectionEvent>,
    tree: JBTree,
    nodes: Vec<TreeNode>,
    tree_events: Vec<TreeEvent>,
    table: JBTable,
    cells: Vec<Vec<String>>,
    table_events: Vec<TableEvent>,
}

fn names() -> Vec<&'static str> {
    vec![
        "NullPointerException",
        "NumberFormatException",
        "README.md",
        "Cargo.toml",
        "main.rs",
        "lib.rs",
        "theme.py",
        "tests",
        "docs",
        "src",
        "target",
        "build.gradle",
        "settings.json",
        "index.html",
        "style.css",
        "app.js",
    ]
}

fn tree_nodes() -> Vec<TreeNode> {
    vec![TreeNode::new("kit").icon(Glyph::Folder).children(vec![
        TreeNode::new("crates").icon(Glyph::Folder).children(vec![
            TreeNode::new("archipel-kit").icon(Glyph::Folder).children(vec![
                TreeNode::new("lib.rs").icon(Language::Rust),
            ]),
            TreeNode::new("archipel-kit-tui").icon(Glyph::Folder).children(vec![
                TreeNode::new("components.rs").icon(Language::Rust),
                TreeNode::new("controls.rs").icon(Language::Rust),
            ]),
        ]),
        TreeNode::new("Cargo.toml").icon(Language::Toml),
        TreeNode::new("README.md").icon(Language::Markdown),
    ])]
}

fn new_screen(filter: bool) -> Screen {
    let mut s = Screen {
        rows: names().into_iter().map(RowData::new).collect(),
        nodes: tree_nodes(),
        cells: (0..12)
            .map(|i| vec![format!("item {i}"), format!("{}", 100 - i * 7), "ok".into()])
            .collect(),
        table: JBTable::new(vec![
            Column::new("Name", 14),
            Column::new("Size", 8).right(),
            Column::new("State", 8),
        ]),
        ..Screen::default()
    };
    s.list.state.filter = filter;
    s
}

fn event(s: &mut Screen, rt: &mut Runtime, e: &Event) {
    let r = s.list.handle(rt, LIST, e, &s.rows);
    if r != CollectionEvent::None {
        s.events.push(r);
    }
    let r = s.tree.handle(rt, TREE, e);
    if r != TreeEvent::None {
        s.tree_events.push(r);
    }
    let r = s.table.handle(rt, TABLE, e, &s.cells);
    if r != TableEvent::None {
        s.table_events.push(r);
    }
}

fn draw(s: &mut Screen, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
    Pane::new(&rt.look)
        .kind(PaneKind::Sidebar)
        .render(area, buf);
    let bg = rt.look.palette.darker_black;
    s.list
        .show(rt, LIST, Rect::new(0, 0, 24, 8), &s.rows, bg, buf);
    s.tree
        .show(rt, TREE, Rect::new(26, 0, 30, 8), &s.nodes, bg, buf);
    s.table
        .show(rt, TABLE, Rect::new(0, 10, 40, 6), &s.cells, bg, buf);
}

type EventFn = fn(&mut Screen, &mut Runtime, &Event);
type DrawFn = fn(&mut Screen, &mut Runtime, Rect, &mut Buffer);
type ScreenApp = FnApp<Screen, EventFn, DrawFn>;

fn harness_with(screen: Screen) -> Harness<ScreenApp> {
    let mut h = Harness::new(app_fn(screen, event as EventFn, draw as DrawFn), 60, 17);
    h.rt.focus.focus(LIST);
    h.draw();
    h
}

fn harness() -> Harness<ScreenApp> {
    harness_with(new_screen(false))
}

macro_rules! screen {
    ($h:expr) => {
        &mut $h.app.state
    };
}

fn selected(h: &mut Harness<ScreenApp>) -> Vec<usize> {
    screen!(h).list.state.selection.iter().collect()
}

#[test]
fn down_and_up_move_the_selection_without_wrapping() {
    let mut h = harness();
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h), [0]);
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h), [1]);
    h.press(KeyCode::Up);
    h.press(KeyCode::Up);
    assert_eq!(selected(&mut h), [0], "Up at the first row stays");
    h.press(KeyCode::End);
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h), [15], "Down at the last row stays");
}

#[test]
fn home_and_end_select_the_first_and_last_row() {
    let mut h = harness();
    h.press(KeyCode::End);
    assert_eq!(selected(&mut h), [15]);
    h.press(KeyCode::Home);
    assert_eq!(selected(&mut h), [0]);
}

#[test]
fn shift_down_extends_the_selection_from_the_anchor() {
    let mut h = harness();
    h.press(KeyCode::Down);
    h.press(KeyCode::Down);
    h.press_with(KeyCode::Down, KeyModifiers::SHIFT);
    h.press_with(KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(selected(&mut h), [1, 2, 3]);
    h.press_with(KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(selected(&mut h), [1, 2]);
}

#[test]
fn ctrl_down_moves_the_lead_and_ctrl_space_toggles_it() {
    let mut h = harness();
    h.press(KeyCode::Down);
    h.press_with(KeyCode::Down, KeyModifiers::CONTROL);
    h.press_with(KeyCode::Down, KeyModifiers::CONTROL);
    assert_eq!(selected(&mut h), [0], "the selection did not move");
    h.press_with(KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert_eq!(selected(&mut h), [0, 2]);
    h.press_with(KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert_eq!(selected(&mut h), [0]);
}

#[test]
fn page_down_and_page_up_move_by_a_page() {
    let mut h = harness();
    h.press(KeyCode::Down);
    h.press(KeyCode::PageDown);
    // The list is 8 rows high: a page is 7 rows.
    assert_eq!(selected(&mut h), [7]);
    h.press(KeyCode::PageDown);
    assert_eq!(selected(&mut h), [14]);
    h.press(KeyCode::PageUp);
    assert_eq!(selected(&mut h), [7]);
}

#[test]
fn ctrl_a_selects_all_rows() {
    let mut h = harness();
    h.ctrl('a');
    assert_eq!(selected(&mut h).len(), 16);
}

#[test]
fn enter_activates_the_selected_row() {
    let mut h = harness();
    h.press(KeyCode::Down);
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(
        screen!(h).events.last(),
        Some(&CollectionEvent::Activated(1))
    );
}

#[test]
fn the_view_scrolls_to_keep_the_selection_visible() {
    let mut h = harness();
    h.press(KeyCode::End);
    assert_eq!(screen!(h).list.state.scroll(), 8);
    assert!(h.text().contains("app.js"));
    h.press(KeyCode::Home);
    assert_eq!(screen!(h).list.state.scroll(), 0);
}

#[test]
fn a_click_selects_a_row_and_focuses_the_list() {
    let mut h = harness();
    h.click(3, 2);
    assert_eq!(selected(&mut h), [2]);
    assert!(h.rt.focused(LIST));
}

#[test]
fn ctrl_click_toggles_and_shift_click_extends() {
    let mut h = harness();
    h.click(3, 1);
    h.click_with(3, 4, KeyModifiers::CONTROL);
    assert_eq!(selected(&mut h), [1, 4]);
    h.advance(1.0);
    h.click_with(3, 4, KeyModifiers::CONTROL);
    assert_eq!(selected(&mut h), [1]);
    // The toggle moved the anchor to row 4: Shift+click extends from there, as in Swing.
    h.click_with(3, 5, KeyModifiers::SHIFT);
    assert_eq!(selected(&mut h), [4, 5]);
    h.advance(1.0);
    h.click(3, 1);
    h.click_with(3, 3, KeyModifiers::SHIFT);
    assert_eq!(selected(&mut h), [1, 2, 3]);
}

#[test]
fn a_double_click_activates_the_row() {
    let mut h = harness();
    h.double_click(3, 3);
    assert_eq!(
        screen!(h).events.last(),
        Some(&CollectionEvent::Activated(3))
    );
}

#[test]
fn dragging_over_rows_extends_the_selection() {
    let mut h = harness();
    h.drag((3, 1), (3, 5));
    assert_eq!(selected(&mut h), [1, 2, 3, 4, 5]);
}

#[test]
fn the_wheel_scrolls_three_rows_and_stops_at_the_end() {
    let mut h = harness();
    h.scroll(3, 3, 1);
    assert_eq!(screen!(h).list.state.scroll(), 3);
    h.scroll(3, 3, 10);
    assert_eq!(screen!(h).list.state.scroll(), 8);
    h.scroll(3, 3, -10);
    assert_eq!(screen!(h).list.state.scroll(), 0);
}

#[test]
fn right_click_selects_the_row_under_the_pointer_and_asks_for_a_menu() {
    let mut h = harness();
    h.click(3, 0);
    h.right_click(3, 3);
    assert_eq!(selected(&mut h), [3]);
    assert_eq!(
        screen!(h).events.last(),
        Some(&CollectionEvent::ContextMenu {
            row: Some(3),
            x: 3,
            y: 3
        })
    );
}

#[test]
fn right_click_inside_a_multiple_selection_keeps_it() {
    let mut h = harness();
    h.click(3, 1);
    h.click_with(3, 3, KeyModifiers::SHIFT);
    h.right_click(3, 2);
    assert_eq!(selected(&mut h), [1, 2, 3]);
}

#[test]
fn hovering_a_row_highlights_it() {
    let mut h = harness();
    h.hover(3, 4);
    assert_eq!(screen!(h).list.state.hovered(), Some(4));
    let hovered = h.buffer()[(3, 4)].bg;
    let normal = h.buffer()[(3, 5)].bg;
    assert_ne!(hovered, normal);
}

// ----- speed search -----

#[test]
fn typing_starts_the_speed_search_and_selects_the_first_match() {
    let mut h = harness();
    h.type_text("mai");
    assert!(screen!(h).list.state.speed.is_active());
    assert_eq!(screen!(h).list.state.speed.text(), "mai");
    assert_eq!(selected(&mut h), [4], "main.rs");
}

#[test]
fn speed_search_matches_camel_humps() {
    let mut h = harness();
    h.type_text("nPE");
    assert_eq!(selected(&mut h), [0]);
    h.press(KeyCode::Backspace);
    h.press(KeyCode::Backspace);
    h.type_text("FE");
    assert_eq!(selected(&mut h), [1], "NumberFormatException");
}

#[test]
fn speed_search_highlights_the_matches() {
    let mut h = harness();
    h.type_text("rm");
    // README.md: R and M are highlighted (bold + underline).
    let (x, y) = h.find("README.md").expect("row");
    let cell = |dx: u16| h.buffer()[(x + dx, y)].modifier;
    use archipel_kit_tui::ratatui::style::Modifier;
    assert!(cell(0).contains(Modifier::UNDERLINED));
    assert!(!cell(1).contains(Modifier::UNDERLINED));
    assert!(cell(7).contains(Modifier::UNDERLINED));
}

#[test]
fn up_and_down_move_between_matches_while_searching() {
    let mut h = harness();
    h.type_text("rs");
    assert_eq!(selected(&mut h), [4], "main.rs");
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h), [5], "lib.rs");
    h.press(KeyCode::Down);
    assert_eq!(selected(&mut h), [5], "no more matches below");
}

#[test]
fn backspace_edits_and_ctrl_backspace_clears_the_search() {
    let mut h = harness();
    h.type_text("cargo");
    h.press(KeyCode::Backspace);
    assert_eq!(screen!(h).list.state.speed.text(), "carg");
    h.press_with(KeyCode::Backspace, KeyModifiers::CONTROL);
    assert_eq!(screen!(h).list.state.speed.text(), "");
    assert!(
        screen!(h).list.state.speed.is_active(),
        "stays open when emptied"
    );
    h.press(KeyCode::Backspace);
    assert!(screen!(h).list.state.speed.is_active());
}

#[test]
fn escape_closes_the_speed_search_and_keeps_the_selection() {
    let mut h = harness();
    h.type_text("lib");
    h.press(KeyCode::Esc);
    assert!(!screen!(h).list.state.speed.is_active());
    assert_eq!(selected(&mut h), [5]);
}

#[test]
fn enter_closes_the_speed_search_and_activates() {
    let mut h = harness();
    h.type_text("lib");
    h.press(KeyCode::Enter);
    assert!(!screen!(h).list.state.speed.is_active());
    assert_eq!(
        screen!(h).events.last(),
        Some(&CollectionEvent::Activated(5))
    );
}

#[test]
fn page_down_closes_the_speed_search_and_pages() {
    let mut h = harness();
    h.type_text("nu");
    h.press(KeyCode::PageDown);
    assert!(!screen!(h).list.state.speed.is_active());
}

#[test]
fn alt_keys_and_a_leading_space_do_not_start_a_speed_search() {
    let mut h = harness();
    h.alt('m');
    h.press(KeyCode::Char(' '));
    assert!(!screen!(h).list.state.speed.is_active());
    h.press(KeyCode::Char('.'));
    assert!(
        screen!(h).list.state.speed.is_active(),
        "punctuation starts it"
    );
}

#[test]
fn a_space_continues_a_running_search() {
    let mut h = harness();
    h.type_text("t");
    h.press(KeyCode::Char(' '));
    assert_eq!(screen!(h).list.state.speed.text(), "t ");
}

#[test]
fn a_search_without_match_turns_the_text_red() {
    let mut h = harness();
    h.type_text("zzz");
    let (x, y) = h.find("zzz").expect("search box");
    let red = h.rt.look.palette.red;
    assert_eq!(h.buffer()[(x, y)].fg, red);
}

#[test]
fn a_filtering_speed_search_hides_the_rows_that_do_not_match() {
    let mut h = harness_with(new_screen(true));
    h.type_text("rs");
    let text = h.text();
    assert!(text.contains("main.rs") && text.contains("lib.rs"));
    assert!(!text.contains("Cargo.toml"));
    h.press(KeyCode::Esc);
    assert!(h.text().contains("Cargo.toml"));
}

#[test]
fn a_list_without_focus_ignores_keys() {
    let mut h = harness();
    h.rt.focus.focus(TREE);
    h.press(KeyCode::Down);
    assert!(selected(&mut h).is_empty());
}

// ----- tree -----

fn tree_focus(h: &mut Harness<ScreenApp>) {
    h.rt.focus.focus(TREE);
    h.draw();
}

fn tree_text(h: &Harness<ScreenApp>) -> String {
    h.screen()
        .iter()
        .take(8)
        .map(|l| l.chars().skip(26).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn right_expands_a_collapsed_node_then_selects_its_first_child() {
    let mut h = harness();
    tree_focus(&mut h);
    h.press(KeyCode::Down);
    assert!(!tree_text(&h).contains("crates"));
    h.press(KeyCode::Right);
    assert!(tree_text(&h).contains("crates"), "{}", tree_text(&h));
    assert_eq!(screen!(h).tree.selected_path(), Some("kit"));
    h.press(KeyCode::Right);
    assert_eq!(screen!(h).tree.selected_path(), Some("kit/crates"));
}

#[test]
fn left_collapses_an_expanded_node_then_selects_the_parent() {
    let mut h = harness();
    tree_focus(&mut h);
    h.press(KeyCode::Down);
    h.press(KeyCode::Right);
    h.press(KeyCode::Right);
    h.press(KeyCode::Right);
    h.press(KeyCode::Right);
    assert_eq!(
        screen!(h).tree.selected_path(),
        Some("kit/crates/archipel-kit")
    );
    h.press(KeyCode::Left);
    assert_eq!(
        screen!(h).tree.selected_path(),
        Some("kit/crates"),
        "collapsed node: to parent"
    );
    h.press(KeyCode::Left);
    assert!(
        !tree_text(&h).contains("archipel-kit"),
        "an expanded node collapses"
    );
    h.press(KeyCode::Left);
    assert_eq!(screen!(h).tree.selected_path(), Some("kit"));
}

#[test]
fn enter_toggles_a_parent_and_activates_a_leaf() {
    let mut h = harness();
    tree_focus(&mut h);
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert!(tree_text(&h).contains("Cargo.toml"));
    h.press(KeyCode::Down);
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(
        screen!(h).tree_events.last(),
        Some(&TreeEvent::Activated("kit/Cargo.toml".into()))
    );
}

#[test]
fn clicking_the_chevron_toggles_without_a_double_click() {
    let mut h = harness();
    h.click(27, 0);
    assert!(tree_text(&h).contains("crates"));
    h.click(27, 0);
    assert!(!tree_text(&h).contains("crates"));
}

#[test]
fn a_double_click_on_a_parent_toggles_it() {
    let mut h = harness();
    h.double_click(32, 0);
    assert!(tree_text(&h).contains("crates"));
}

#[test]
fn select_path_expands_the_ancestors() {
    let mut h = harness();
    let nodes = tree_nodes();
    assert!(
        screen!(h)
            .tree
            .select_path(&nodes, "kit/crates/archipel-kit-tui/controls.rs")
    );
    h.draw();
    assert!(tree_text(&h).contains("controls.rs"));
    assert_eq!(
        screen!(h).tree.selected_path(),
        Some("kit/crates/archipel-kit-tui/controls.rs")
    );
}

#[test]
fn the_tree_speed_search_finds_collapsed_rows_only_when_visible() {
    let mut h = harness();
    tree_focus(&mut h);
    screen!(h).tree.expand_path("kit/crates/archipel-kit-tui");
    h.draw();
    h.type_text("cont");
    assert_eq!(
        screen!(h).tree.selected_path(),
        Some("kit/crates/archipel-kit-tui/controls.rs")
    );
}

#[test]
fn a_right_click_in_the_tree_asks_for_the_context_menu_of_the_node() {
    let mut h = harness();
    h.right_click(30, 0);
    assert_eq!(
        screen!(h).tree_events.last(),
        Some(&TreeEvent::ContextMenu {
            path: Some("kit".into()),
            x: 30,
            y: 0
        })
    );
}

// ----- table -----

#[test]
fn clicking_a_header_sorts_ascending_then_descending() {
    let mut h = harness();
    h.click(3, 10);
    assert_eq!(
        screen!(h).table_events.last(),
        Some(&TableEvent::Sort(0, SortOrder::Ascending))
    );
    h.click(3, 10);
    assert_eq!(
        screen!(h).table_events.last(),
        Some(&TableEvent::Sort(0, SortOrder::Descending))
    );
    assert!(h.text().contains("Name ▼"));
}

#[test]
fn dragging_a_column_divider_resizes_the_column() {
    let mut h = harness();
    // The Name column is 14 wide: its divider is at x = 13.
    h.drag((13, 10), (18, 10));
    assert_eq!(screen!(h).table.columns[0].width, 19);
    assert!(screen!(h).table_events.contains(&TableEvent::Resized(0)));
}

#[test]
fn table_rows_select_and_activate_like_list_rows() {
    let mut h = harness();
    h.click(3, 12);
    assert_eq!(screen!(h).table.state.selection.first(), Some(1));
    h.rt.focus.focus(TABLE);
    h.press(KeyCode::Down);
    assert_eq!(screen!(h).table.state.selection.first(), Some(2));
    h.press(KeyCode::Enter);
    assert_eq!(
        screen!(h).table_events.last(),
        Some(&TableEvent::Activated(2))
    );
}

#[test]
fn table_cells_can_be_right_aligned() {
    let h = harness();
    let line = &h.screen()[11];
    assert!(line.contains("item 0"), "{line}");
    assert!(line.contains("100"), "{line}");
}

#[test]
fn collections_survive_tiny_screens_and_empty_models() {
    for (w, hgt) in [(1u16, 1u16), (3, 2), (12, 4), (60, 3)] {
        let mut s = new_screen(false);
        s.rows.clear();
        s.cells.clear();
        s.nodes.clear();
        let mut h = Harness::new(app_fn(s, event as EventFn, draw as DrawFn), w, hgt);
        for id in [LIST, TREE, TABLE] {
            h.rt.focus.focus(id);
            for code in [
                KeyCode::Down,
                KeyCode::Up,
                KeyCode::PageDown,
                KeyCode::Enter,
                KeyCode::Right,
                KeyCode::Left,
                KeyCode::Home,
                KeyCode::End,
            ] {
                h.press(code);
            }
            h.type_text("abc");
            h.click(0, 0);
            h.right_click(w - 1, hgt - 1);
            h.scroll(0, 0, 3);
            h.drag((0, 0), (w - 1, hgt - 1));
        }
        let mut s = new_screen(true);
        s.list.state.speed.activate("x");
        let mut h = Harness::new(app_fn(s, event as EventFn, draw as DrawFn), w, hgt);
        h.press(KeyCode::Down);
        h.ctrl('a');
    }
}

#[test]
fn collections_in_both_themes() {
    for (name, look) in [("dark", Look::dark()), ("light", Look::light())] {
        let mut s = new_screen(false);
        s.list.state.speed.activate("rs");
        s.list.state.selection.select(4);
        s.tree.expand_path("kit/crates/archipel-kit-tui");
        let mut h = Harness::with_runtime(
            app_fn(s, event as EventFn, draw as DrawFn),
            Runtime::new(look),
            60,
            17,
        );
        let nodes = tree_nodes();
        screen!(h)
            .tree
            .select_path(&nodes, "kit/crates/archipel-kit-tui/controls.rs");
        h.rt.focus.focus(TREE);
        screen!(h).table.sort = Some((1, SortOrder::Descending));
        screen!(h).table.state.selection.select(2);
        h.draw();
        snapshot(&format!("collections.{name}"), h.buffer());
    }
    let _ = RowIcon::Glyph(Glyph::File);
}
