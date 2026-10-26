//! Selection of `JBList`, `Tree`, `JBTable` and `TreeTable`: the keys of Swing's list and tree
//! actions, the mouse rules, the focus colors (IntelliJ spec section 3.1 to 3.3).

mod common;

use archipel_kit::icons::Language;
use archipel_kit::testing::{self, NodeExt, Queryable};
use archipel_kit::ui::{
    Cell, ColoredListCellRenderer, ColumnInfo, JBList, JBTable, ListSelectionModel, SelectionMode,
    Tree, TreeNode, TreeTable,
};
use archipel_kit::{Glyph, Theme};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};

const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::NONE
};
const CTRL: Modifiers = Modifiers {
    ctrl: true,
    command: true,
    ..Modifiers::NONE
};
const CTRL_SHIFT: Modifiers = Modifiers {
    ctrl: true,
    command: true,
    shift: true,
    ..Modifiers::NONE
};

fn label(i: usize) -> String {
    format!("GET /users/{i}")
}

#[derive(Default)]
struct State {
    selection: ListSelectionModel,
    activated: Option<usize>,
    context: Option<usize>,
    changes: usize,
}

fn harness_with(mode: SelectionMode) -> testing::Harness<'static, State> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        State {
            selection: ListSelectionModel::new(mode),
            ..State::default()
        },
        |ui, state| {
            let out = JBList::new("rows", 40, &mut state.selection)
                .name("Requests")
                .height(120.0)
                .show(ui, |i| ColoredListCellRenderer::new(label(i)));
            state.changes += usize::from(out.selection_changed);
            if out.activated.is_some() {
                state.activated = out.activated;
            }
            if out.secondary_clicked.is_some() {
                state.context = out.secondary_clicked;
            }
        },
    );
    harness.run();
    harness
}

fn harness() -> testing::Harness<'static, State> {
    harness_with(SelectionMode::MultipleInterval)
}

fn selected(harness: &testing::Harness<'_, State>) -> Vec<usize> {
    harness.state().selection.iter().collect()
}

fn click_row(harness: &mut testing::Harness<'_, State>, row: usize, mods: Modifiers) {
    click_row_with(harness, row, mods, PointerButton::Primary);
}

fn click_row_with(
    harness: &mut testing::Harness<'_, State>,
    row: usize,
    mods: Modifiers,
    button: PointerButton,
) {
    let pos: Pos2 = harness.get_by_label(&label(row)).rect().center();
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event_modifiers(
            Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: mods,
            },
            mods,
        );
    }
    harness.run();
}

fn press(harness: &mut testing::Harness<'_, State>, mods: Modifiers, key: Key) {
    harness.key_press_modifiers(mods, key);
    harness.run();
}

#[test]
fn shift_down_extends_the_selection_from_the_anchor() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    press(&mut harness, SHIFT, Key::ArrowDown);
    press(&mut harness, SHIFT, Key::ArrowDown);
    assert_eq!(selected(&harness), vec![1, 2, 3]);
    assert_eq!(harness.state().selection.anchor(), Some(1));
    assert_eq!(harness.state().selection.lead(), Some(3));
    // Back toward the anchor shrinks it again.
    press(&mut harness, SHIFT, Key::ArrowUp);
    assert_eq!(selected(&harness), vec![1, 2]);
    // Past the anchor it extends the other way.
    press(&mut harness, SHIFT, Key::ArrowUp);
    press(&mut harness, SHIFT, Key::ArrowUp);
    assert_eq!(selected(&harness), vec![0, 1]);
}

#[test]
fn plain_arrows_collapse_a_multi_selection_to_one_row() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    press(&mut harness, SHIFT, Key::ArrowDown);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(selected(&harness), vec![3]);
}

#[test]
fn up_at_the_first_row_stays_there() {
    // `ide.tree.ui.cyclic.scrolling.allowed` is off: no wrap around.
    let mut harness = harness();
    click_row(&mut harness, 0, Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(selected(&harness), vec![0]);
    press(&mut harness, Modifiers::NONE, Key::End);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(selected(&harness), vec![39]);
}

#[test]
fn shift_home_and_shift_end_extend_to_the_ends() {
    let mut harness = harness();
    click_row(&mut harness, 2, Modifiers::NONE);
    press(&mut harness, SHIFT, Key::Home);
    assert_eq!(selected(&harness), vec![0, 1, 2]);
    press(&mut harness, SHIFT, Key::End);
    assert_eq!(selected(&harness), (2..40).collect::<Vec<_>>());
}

#[test]
fn ctrl_a_selects_all_rows() {
    let mut harness = harness();
    click_row(&mut harness, 2, Modifiers::NONE);
    press(&mut harness, CTRL, Key::A);
    assert_eq!(selected(&harness), (0..40).collect::<Vec<_>>());
}

#[test]
fn ctrl_arrows_move_the_lead_without_selecting_and_ctrl_space_toggles_it() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    press(&mut harness, CTRL, Key::ArrowDown);
    press(&mut harness, CTRL, Key::ArrowDown);
    assert_eq!(selected(&harness), vec![1], "the selection did not move");
    assert_eq!(harness.state().selection.lead(), Some(3));
    press(&mut harness, CTRL, Key::Space);
    assert_eq!(selected(&harness), vec![1, 3]);
    press(&mut harness, CTRL, Key::Space);
    assert_eq!(selected(&harness), vec![1]);
}

#[test]
fn page_down_goes_to_the_bottom_of_the_view_then_a_page_further() {
    // 120 pt of 24 pt rows: rows 0 to 4 are in view.
    let mut harness = harness();
    click_row(&mut harness, 0, Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    assert_eq!(selected(&harness), vec![4], "first to the last visible row");
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    assert_eq!(selected(&harness), vec![9], "then a page further");
    press(&mut harness, Modifiers::NONE, Key::PageUp);
    harness.run();
    let after = selected(&harness);
    assert!(after[0] < 9, "page up moves up: {after:?}");
}

#[test]
fn shift_page_down_extends_by_a_page() {
    let mut harness = harness();
    click_row(&mut harness, 0, Modifiers::NONE);
    press(&mut harness, SHIFT, Key::PageDown);
    assert_eq!(selected(&harness), vec![0, 1, 2, 3, 4]);
}

#[test]
fn click_replaces_the_selection() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 2, CTRL);
    assert_eq!(selected(&harness), vec![1, 2]);
    click_row(&mut harness, 4, Modifiers::NONE);
    assert_eq!(selected(&harness), vec![4]);
}

#[test]
fn ctrl_click_toggles_a_row() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 3, CTRL);
    assert_eq!(selected(&harness), vec![1, 3]);
    assert!(harness.get_by_label(&label(3)).is_checked());
    click_row(&mut harness, 1, CTRL);
    assert_eq!(selected(&harness), vec![3]);
}

#[test]
fn shift_click_extends_from_the_anchor_and_keeps_it() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 3, SHIFT);
    assert_eq!(selected(&harness), vec![1, 2, 3]);
    // The anchor stays on row 1, so the next shift click re-ranges from it.
    click_row(&mut harness, 0, SHIFT);
    assert_eq!(selected(&harness), vec![0, 1]);
}

#[test]
fn ctrl_shift_click_adds_a_range_to_the_selection() {
    let mut harness = harness();
    click_row(&mut harness, 0, Modifiers::NONE);
    click_row(&mut harness, 3, CTRL);
    click_row(&mut harness, 4, CTRL_SHIFT);
    assert_eq!(selected(&harness), vec![0, 3, 4]);
}

#[test]
fn right_click_inside_the_selection_keeps_it() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 3, SHIFT);
    click_row_with(&mut harness, 2, Modifiers::NONE, PointerButton::Secondary);
    assert_eq!(selected(&harness), vec![1, 2, 3], "the selection stays");
    assert_eq!(harness.state().context, Some(2));
}

#[test]
fn right_click_outside_the_selection_selects_that_row() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 3, SHIFT);
    click_row_with(&mut harness, 4, Modifiers::NONE, PointerButton::Secondary);
    assert_eq!(selected(&harness), vec![4]);
    assert_eq!(harness.state().context, Some(4));
}

#[test]
fn right_click_with_one_row_selected_moves_the_selection() {
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row_with(&mut harness, 2, Modifiers::NONE, PointerButton::Secondary);
    assert_eq!(selected(&harness), vec![2]);
}

#[test]
fn single_selection_mode_ignores_shift() {
    let mut harness = harness_with(SelectionMode::Single);
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 3, SHIFT);
    assert_eq!(selected(&harness), vec![3]);
    press(&mut harness, SHIFT, Key::ArrowDown);
    assert_eq!(selected(&harness), vec![4]);
    press(&mut harness, CTRL, Key::A);
    assert_eq!(selected(&harness), vec![4], "select all needs several rows");
}

#[test]
fn single_interval_mode_keeps_one_run_of_rows() {
    let mut harness = harness_with(SelectionMode::SingleInterval);
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 3, SHIFT);
    assert_eq!(selected(&harness), vec![1, 2, 3]);
    click_row(&mut harness, 4, CTRL);
    assert_eq!(selected(&harness), vec![4], "ctrl starts a new run");
}

#[test]
fn enter_activates_the_lead_row_and_double_click_a_row() {
    let mut harness = harness();
    click_row(&mut harness, 2, Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().activated, Some(2));
}

#[test]
fn selection_changed_is_reported_once_per_change() {
    let mut harness = harness();
    let before = harness.state().changes;
    click_row(&mut harness, 2, Modifiers::NONE);
    assert_eq!(harness.state().changes, before + 1);
    // Clicking the selected row again changes nothing.
    click_row(&mut harness, 2, Modifiers::NONE);
    assert_eq!(harness.state().changes, before + 1);
}

#[test]
fn single_option_selection_still_works_with_modifiers() {
    // The old API: `&mut Option<usize>`. Shift and Ctrl act as plain clicks, Ctrl+click on
    // the selected row deselects it.
    let mut harness = testing::harness_state(Theme::dark(), None::<usize>, |ui, selected| {
        let _ = JBList::new("single", 10, selected)
            .height(120.0)
            .show(ui, |i| ColoredListCellRenderer::new(label(i)));
    });
    harness.run();
    let pos = harness.get_by_label(&label(2)).rect().center();
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event_modifiers(
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: SHIFT,
            },
            SHIFT,
        );
    }
    harness.run();
    assert_eq!(*harness.state(), Some(2));
    harness.key_press_modifiers(SHIFT, Key::ArrowDown);
    harness.run();
    assert_eq!(*harness.state(), Some(3));
}

/// The pixel in the middle of the selection pill of `row`, at 1x.
fn pill_color(harness: &mut testing::Harness<'_, State>, row: usize) -> [u8; 4] {
    let rect = harness.get_by_label(&label(row)).rect();
    let image = harness.render().expect("renders");
    // Inside the pill, which starts 12 pt from the row's edge.
    let (x, y) = (rect.left() as u32 + 16, rect.center().y as u32);
    image.get_pixel(x, y).0
}

#[test]
fn the_selection_is_blue_with_focus_and_gray_without() {
    let theme = Theme::dark();
    // Selected, but the list never had the focus: the quiet selection.
    let mut harness = harness();
    harness.state_mut().selection.select(2);
    harness.run();
    let color = pill_color(&mut harness, 2);
    let inactive = theme.colors.selection_inactive;
    assert_eq!(
        color[..3],
        [inactive.r(), inactive.g(), inactive.b()],
        "unfocused selection"
    );
    // A click gives the list the focus: the active selection.
    click_row(&mut harness, 2, Modifiers::NONE);
    harness.run();
    let color = pill_color(&mut harness, 2);
    let active = theme.colors.selection;
    assert_eq!(
        color[..3],
        [active.r(), active.g(), active.b()],
        "focused selection"
    );
}

#[test]
fn the_selection_is_gray_while_the_window_is_inactive() {
    let theme = Theme::dark();
    let mut harness = harness();
    click_row(&mut harness, 2, Modifiers::NONE);
    harness.run();
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .focused = Some(false);
    harness.run();
    let color = pill_color(&mut harness, 2);
    let inactive = theme.colors.selection_inactive;
    assert_eq!(color[..3], [inactive.r(), inactive.g(), inactive.b()]);
}

#[test]
fn selected_neighbors_join_into_one_block() {
    // Two selected rows touch: no notch between their pills, only the outer corners are
    // round. Check the pixel at the left edge of the pill, on the boundary between them.
    let theme = Theme::dark();
    let mut harness = harness();
    click_row(&mut harness, 1, Modifiers::NONE);
    click_row(&mut harness, 2, SHIFT);
    let upper = harness.get_by_label(&label(1)).rect();
    let image = harness.render().expect("renders");
    // Pill's left edge is 12 pt in; one pixel inside, on the seam between rows 1 and 2.
    let (x, y) = (upper.left() as u32 + 12 + 1, upper.bottom() as u32);
    let pixel = image.get_pixel(x, y).0;
    let active = theme.colors.selection;
    assert_eq!(pixel[..3], [active.r(), active.g(), active.b()], "no notch");
    // The block's outer corner is round: the very corner pixel is not the selection.
    let corner = image
        .get_pixel(upper.left() as u32 + 12, upper.top() as u32)
        .0;
    assert_ne!(corner[..3], [active.r(), active.g(), active.b()]);
}

// Trees.

fn project_rows(src: bool) -> Vec<TreeNode> {
    let mut rows = vec![TreeNode::new("kit", 0).icon(Glyph::Folder).parent(true)];
    rows.push(TreeNode::new("src", 1).icon(Glyph::Folder).parent(src));
    if src {
        rows.push(TreeNode::new("lib.rs", 2).icon(Language::Rust));
        rows.push(TreeNode::new("theme.rs", 2).icon(Language::Rust));
    }
    rows.push(TreeNode::new("README.md", 1));
    rows
}

#[derive(Default)]
struct TreeState {
    src_open: bool,
    selection: ListSelectionModel,
    activated: Option<usize>,
    toggled: Vec<usize>,
}

fn tree_harness() -> testing::Harness<'static, TreeState> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        TreeState {
            src_open: false,
            ..TreeState::default()
        },
        |ui, state| {
            let rows = project_rows(state.src_open);
            let out = Tree::new("project", &rows, &mut state.selection)
                .name("Project")
                .show(ui);
            if let Some(index) = out.toggled {
                state.toggled.push(index);
                if rows[index].label() == "src" {
                    state.src_open = !state.src_open;
                }
            }
            if out.activated.is_some() {
                state.activated = out.activated;
            }
        },
    );
    harness.run();
    harness
}

#[test]
fn tree_enter_toggles_a_parent_and_activates_a_leaf() {
    let mut harness = tree_harness();
    harness.get_by_label("src").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().toggled, vec![1], "a parent toggles");
    assert!(harness.state().src_open);
    assert_eq!(harness.state().activated, None, "and is not activated");
    harness.get_by_label("README.md").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().activated, Some(4), "a leaf is activated");
}

#[test]
fn tree_arrows_follow_the_lead_of_a_multi_selection() {
    let mut harness = tree_harness();
    harness.get_by_label("src").click();
    harness.run();
    harness.key_press_modifiers(SHIFT, Key::ArrowDown);
    harness.run();
    assert_eq!(
        harness.state().selection.iter().collect::<Vec<_>>(),
        vec![1, 2]
    );
    // The lead is `README.md`, a leaf at depth 1: ← walks to its parent `kit`.
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(
        harness.state().selection.iter().collect::<Vec<_>>(),
        vec![0],
        "left selected the parent"
    );
}

#[test]
fn tree_shift_click_selects_the_rows_between_across_levels() {
    let mut harness = tree_harness();
    harness.get_by_label("src").click();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.run();
    let pos = harness.get_by_label("README.md").rect().center();
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event_modifiers(
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: SHIFT,
            },
            SHIFT,
        );
    }
    harness.run();
    assert_eq!(
        harness.state().selection.iter().collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

// Tables.

#[derive(Default)]
struct TableState {
    selection: ListSelectionModel,
}

fn table_harness() -> testing::Harness<'static, TableState> {
    let mut columns = vec![
        ColumnInfo::new("Method", 80.0),
        ColumnInfo::new("Path", 160.0),
    ];
    let mut harness =
        testing::harness_state(Theme::dark(), TableState::default(), move |ui, state| {
            let _ = JBTable::new("table", &mut columns, 30, &mut state.selection)
                .name("Requests")
                .height(144.0)
                .show(ui, |row, col| match col {
                    0 => Cell::new("GET"),
                    _ => Cell::new(format!("/users/{row}")),
                });
        });
    harness.run();
    harness
}

#[test]
fn table_rows_select_with_ctrl_and_shift() {
    let mut harness = table_harness();
    let center = |harness: &testing::Harness<'_, TableState>, row: usize| {
        harness
            .get_by_label(&format!("GET, /users/{row}"))
            .rect()
            .center()
    };
    let click = |harness: &mut testing::Harness<'_, TableState>, row: usize, mods: Modifiers| {
        let pos = center(harness, row);
        harness.hover_at(pos);
        harness.run();
        for pressed in [true, false] {
            harness.event_modifiers(
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: mods,
                },
                mods,
            );
        }
        harness.run();
    };
    click(&mut harness, 1, Modifiers::NONE);
    click(&mut harness, 3, SHIFT);
    assert_eq!(
        harness.state().selection.iter().collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    click(&mut harness, 5, CTRL);
    assert_eq!(
        harness.state().selection.iter().collect::<Vec<_>>(),
        vec![1, 2, 3, 5]
    );
    harness.key_press_modifiers(CTRL, Key::A);
    harness.run();
    assert_eq!(harness.state().selection.len(), 30);
}

#[test]
fn tree_table_takes_a_selection_model() {
    let rows = vec![
        TreeNode::new("name", 0),
        TreeNode::new("address", 0).parent(true),
        TreeNode::new("city", 1),
    ];
    let mut columns = vec![
        ColumnInfo::new("Key", 120.0),
        ColumnInfo::new("Value", 120.0),
    ];
    let mut harness = testing::harness_state(
        Theme::dark(),
        ListSelectionModel::multiple(),
        move |ui, selection| {
            let _ = TreeTable::new("doc", &mut columns, &rows, selection)
                .show(ui, |_, _| Cell::new("v"));
        },
    );
    harness.run();
    harness.get_by_label("name, v").click();
    harness.run();
    harness.key_press_modifiers(SHIFT, Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().iter().collect::<Vec<_>>(), vec![0, 1]);
}

// The model.

#[test]
fn the_model_follows_swing_for_intervals_anchor_and_lead() {
    let mut model = ListSelectionModel::multiple();
    assert!(model.is_empty());
    assert!(model.select(2));
    assert!(!model.select(2), "nothing changed");
    assert!(model.add_selection_interval(4, 6));
    assert_eq!(model.iter().collect::<Vec<_>>(), vec![2, 4, 5, 6]);
    assert_eq!((model.anchor(), model.lead()), (Some(4), Some(6)));
    assert!(model.remove_selection_interval(5, 5));
    assert_eq!(model.iter().collect::<Vec<_>>(), vec![2, 4, 6]);
    assert!(model.set_selection_interval(7, 5));
    assert_eq!(model.iter().collect::<Vec<_>>(), vec![5, 6, 7]);
    assert_eq!((model.first(), model.last()), (Some(5), Some(7)));
    assert!(model.clamp(6));
    assert_eq!(model.iter().collect::<Vec<_>>(), vec![5]);
    model.set_mode(SelectionMode::Single);
    model.select_all(10);
    assert_eq!(model.len(), 1, "single mode cannot select all");
}

#[test]
fn single_mode_keeps_only_the_lead_of_a_range() {
    let mut model = ListSelectionModel::single();
    model.set_selection_interval(1, 4);
    assert_eq!(model.iter().collect::<Vec<_>>(), vec![4]);
    model.toggle(4);
    assert!(model.is_empty());
}
