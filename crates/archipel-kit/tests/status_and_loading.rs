//! `StatusText` (empty text with links), the empty state of collections, `JBLoadingPanel` and
//! `CollapsiblePanel` (IntelliJ spec sections 3.3 and 13.7).

mod common;

use archipel_kit::Theme;
use archipel_kit::parts::SimpleTextAttributes;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::ui::{
    Button, CollapsiblePanel, ColoredListCellRenderer, ColumnInfo, JBList, JBLoadingPanel, JBTable,
    StatusText, Tree,
};

#[test]
fn an_empty_list_says_nothing_to_show() {
    let mut harness = testing::harness_state(Theme::dark(), None::<usize>, |ui, selected| {
        let _ = JBList::new("empty", 0, selected)
            .height(120.0)
            .show(ui, |_| ColoredListCellRenderer::new("never"));
    });
    harness.run();
    harness.get_by_label("Nothing to show");
}

#[test]
fn an_empty_tree_and_table_say_it_too() {
    let mut harness = testing::harness_state(Theme::dark(), None::<usize>, |ui, selected| {
        let _ = Tree::new("tree", &[], &mut *selected).height(60.0).show(ui);
        let mut columns = vec![ColumnInfo::new("A", 80.0)];
        let _ = JBTable::new("table", &mut columns, 0, selected)
            .height(60.0)
            .show(ui, |_, _| "x".into());
    });
    harness.run();
    assert_eq!(harness.get_all_by_label("Nothing to show").count(), 2);
}

#[derive(Default)]
struct Links {
    clicked: Option<usize>,
}

#[test]
fn a_link_of_the_empty_text_is_clickable_and_reported() {
    let mut harness = testing::harness_state(Theme::dark(), Links::default(), |ui, state| {
        let mut selected = None;
        let out = JBList::new("links", 0, &mut selected)
            .height(120.0)
            .empty_text(
                StatusText::text("No requests yet. ")
                    .link("Create one")
                    .append(" or ")
                    .link("import"),
            )
            .show(ui, |_| ColoredListCellRenderer::new("never"));
        if out.empty_text_link.is_some() {
            state.clicked = out.empty_text_link;
        }
    });
    harness.run();
    harness.get_by_label("No requests yet. Create one or import");
    harness.get_by_label("import").click();
    harness.run();
    assert_eq!(harness.state().clicked, Some(1));
    harness.get_by_label("Create one").click();
    harness.run();
    assert_eq!(harness.state().clicked, Some(0));
}

#[test]
fn status_text_lines_are_grayed_and_centered_and_links_pass_the_audit() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = StatusText::text("Nothing is selected")
            .line("Select a request to see it here")
            .link("Learn more")
            .show_with_links(ui);
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn status_text_link_reports_through_show_with_links() {
    let mut harness = testing::harness_state(Theme::dark(), None::<usize>, |ui, state| {
        let out = StatusText::text("Empty. ")
            .link("Fill it")
            .show_with_links(ui);
        if out.link.is_some() {
            *state = out.link;
        }
    });
    harness.run();
    harness.get_by_label("Fill it").click();
    harness.run();
    assert_eq!(*harness.state(), Some(0));
}

#[test]
fn text_attributes_have_the_ide_presets() {
    assert!(SimpleTextAttributes::REGULAR_BOLD_ATTRIBUTES.is_bold());
    assert!(SimpleTextAttributes::GRAYED_ITALIC_ATTRIBUTES.is_italic());
    assert!(SimpleTextAttributes::LINK_ATTRIBUTES.is_underline());
    assert!(!SimpleTextAttributes::LINK_PLAIN_ATTRIBUTES.is_underline());
    assert!(SimpleTextAttributes::GRAYED_SMALL_ATTRIBUTES.is_smaller());
    assert!(SimpleTextAttributes::SEARCH_MATCH_ATTRIBUTES.is_search_match());
    assert_eq!(
        SimpleTextAttributes::GRAY_ATTRIBUTES,
        SimpleTextAttributes::GRAYED_ATTRIBUTES
    );
}

#[test]
fn appended_fragments_are_part_of_the_row_name() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ColoredListCellRenderer::new("main.rs")
            .append(" src/bin", SimpleTextAttributes::GRAYED_ATTRIBUTES)
            .append(" 2 errors", SimpleTextAttributes::ERROR_ATTRIBUTES)
            .show(ui);
    });
    harness.run();
    harness.get_by_label("main.rs src/bin 2 errors");
}

// JBLoadingPanel.

struct Loading {
    loading: bool,
    delay: u32,
    clicks: usize,
}

fn loading_harness(delay: u32) -> testing::Harness<'static, Loading> {
    let mut harness = testing::harness_state(
        Theme::dark(),
        Loading {
            loading: false,
            delay,
            clicks: 0,
        },
        |ui, state| {
            JBLoadingPanel::new("panel", state.loading)
                .start_delay_ms(state.delay)
                .text("Loading results")
                .show(ui, |ui| {
                    if ui.add(Button::secondary("Reload")).clicked() {
                        state.clicks += 1;
                    }
                    ui.add_space(40.0);
                });
        },
    );
    harness.run_ok();
    harness
}

fn veil(harness: &testing::Harness<'_, Loading>) -> bool {
    harness.query_all_by_label("Loading results").count() > 0
}

#[test]
fn the_veil_shows_after_the_start_delay_not_before() {
    let mut harness = loading_harness(300);
    assert!(!veil(&harness));
    harness.state_mut().loading = true;
    harness.run_steps(10); // 167 ms
    assert!(!veil(&harness), "not yet at 167 ms");
    harness.run_steps(15); // 417 ms
    assert!(veil(&harness), "shown after 300 ms");
}

#[test]
fn a_load_that_ends_before_the_delay_never_shows_the_veil() {
    let mut harness = loading_harness(300);
    harness.state_mut().loading = true;
    harness.run_steps(8);
    harness.state_mut().loading = false;
    harness.run_steps(60);
    assert!(!veil(&harness));
}

#[test]
fn without_a_delay_the_veil_shows_at_once() {
    let mut harness = loading_harness(0);
    harness.state_mut().loading = true;
    harness.run_steps(3);
    assert!(veil(&harness));
}

#[test]
fn the_veil_blocks_the_content_and_fades_out_when_loading_ends() {
    let mut harness = loading_harness(0);
    harness.state_mut().loading = true;
    harness.run_steps(5);
    harness.get_by_label("Reload").click();
    harness.run_steps(3);
    assert_eq!(harness.state().clicks, 0, "the veil took the click");
    harness.state_mut().loading = false;
    harness.run_steps(3);
    assert!(veil(&harness), "still fading out");
    harness.run_steps(30);
    assert!(!veil(&harness), "gone after the fade");
    harness.get_by_label("Reload").click();
    harness.run_steps(3);
    assert_eq!(harness.state().clicks, 1, "the content works again");
}

// CollapsiblePanel.

#[test]
fn the_panel_shows_its_content_only_while_expanded() {
    let mut harness = testing::harness_state(Theme::dark(), true, |ui, collapsed| {
        CollapsiblePanel::new("Advanced", collapsed).show(ui, |ui| {
            ui.label("Hidden settings");
        });
    });
    harness.run();
    assert!(harness.query_by_label("Hidden settings").is_none());
    harness.get_by_label("Advanced").click();
    harness.run();
    assert!(!*harness.state());
    harness.get_by_label("Hidden settings");
    harness.get_by_label("Advanced").click();
    harness.run();
    assert!(harness.query_by_label("Hidden settings").is_none());
}

#[test]
fn the_panel_header_toggles_with_enter_and_space_when_focused() {
    for key in [egui::Key::Enter, egui::Key::Space] {
        let mut harness = testing::harness_state(Theme::dark(), true, |ui, collapsed| {
            CollapsiblePanel::new("Advanced", collapsed).show(ui, |ui| {
                ui.label("Hidden settings");
            });
        });
        harness.run();
        harness.get_by_label("Advanced").focus();
        harness.run();
        harness.key_press(key);
        harness.run();
        assert!(!*harness.state(), "{key:?} expanded it");
    }
}

#[test]
fn the_chevron_can_sit_at_the_end() {
    let mut harness = testing::harness_state(Theme::dark(), false, |ui, collapsed| {
        CollapsiblePanel::new("Advanced", collapsed)
            .collapse_button_at_left(false)
            .show(ui, |ui| {
                ui.label("Body");
            });
    });
    harness.run();
    harness.get_by_label("Body");
    harness.get_by_label("Advanced").click();
    harness.run();
    assert!(*harness.state());
}
