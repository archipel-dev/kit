//! `ActionPopupMenu` and `PopupHandler::for_group`: an action group as a popup menu (spec §5).

mod common;

use archipel_kit::action_system::*;
use archipel_kit::behaviour::PopupHandler;
use archipel_kit::testing::{self, Harness, NodeExt, Queryable};
use archipel_kit::ui::ColoredListCellRenderer;
use archipel_kit::ui::popup::PopupChain;
use archipel_kit::{Glyph, Theme};
use egui::{Key, Modifiers, pos2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Log = Rc<RefCell<Vec<String>>>;

/// A command that logs its id when performed.
struct Cmd {
    id: &'static str,
    enabled: Rc<Cell<bool>>,
    keep_open: bool,
    log: Log,
}

impl AnAction for Cmd {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.id).keep_popup_on_perform(self.keep_open)
    }

    fn update(&self, e: &mut AnActionEvent<'_>) {
        e.presentation.enabled = self.enabled.get();
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.log.borrow_mut().push(self.id.to_owned());
    }
}

struct Fixture {
    log: Log,
    beta_enabled: Rc<Cell<bool>>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            log: Log::default(),
            beta_enabled: Rc::new(Cell::new(true)),
        }
    }

    fn cmd(&self, id: &'static str) -> Cmd {
        Cmd {
            id,
            enabled: Rc::new(Cell::new(true)),
            keep_open: false,
            log: self.log.clone(),
        }
    }

    /// Alpha, separator, Beta (can be disabled), Gamma, a popup group "More" holding Inner A and
    /// Inner B, and Last.
    fn group(&self) -> DefaultActionGroup {
        let mut more = DefaultActionGroup::popup("More");
        more.add(self.cmd("Inner A")).add(self.cmd("Inner B"));
        let mut group = DefaultActionGroup::new("Menu");
        group
            .add(self.cmd("Alpha"))
            .add_separator()
            .add(Cmd {
                enabled: self.beta_enabled.clone(),
                ..self.cmd("Beta")
            })
            .add(self.cmd("Gamma"))
            .add_group(more)
            .add(self.cmd("Last"));
        group
    }
}

struct State {
    chain: PopupChain,
    performed: Vec<String>,
    cancelled: u32,
}

fn harness(fixture: &Fixture, numbering: bool) -> Harness<'static, State> {
    let fixture = Fixture {
        log: fixture.log.clone(),
        beta_enabled: fixture.beta_enabled.clone(),
    };
    let state = State {
        chain: PopupChain::default(),
        performed: Vec::new(),
        cancelled: 0,
    };
    let mut harness =
        testing::harness_state_sized(Theme::dark(), vec2(520.0, 400.0), state, move |ui, s| {
            let group = fixture.group();
            let context = DataContext::empty();
            let out = ActionPopupMenu::new("menu", &group, &context, ActionPlace::EDITOR_POPUP)
                .numbering(numbering)
                .show_at(ui.ctx(), &mut s.chain, pos2(20.0, 20.0));
            if let Some(id) = out.performed {
                s.performed.push(id);
            }
            if out.cancelled {
                s.cancelled += 1;
            }
        });
    harness.run();
    let now = harness.ctx.input(|i| i.time);
    harness.state_mut().chain.open(now, false);
    harness.run();
    harness
}

fn selected(harness: &Harness<'_, State>) -> Option<usize> {
    harness.state().chain.selected(0)
}

#[test]
fn rows_show_the_actions_and_the_separator_folds_into_the_row_after_it() {
    let harness = harness(&Fixture::new(), false);
    for label in ["Alpha", "Beta", "Gamma", "More", "Last"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    // 8 + Alpha 24 + separator 9 + 4 rows of 24: the rows are 24 pt and touch.
    let alpha = harness.get_by_label("Alpha").rect();
    let beta = harness.get_by_label("Beta").rect();
    assert_eq!(alpha.height(), 24.0);
    assert_eq!(beta.top() - alpha.bottom(), 9.0, "the separator row");
}

#[test]
fn a_menu_selects_nothing_until_a_key_or_the_pointer_moves() {
    let harness = harness(&Fixture::new(), false);
    assert_eq!(selected(&harness), None);
}

#[test]
fn down_selects_the_first_row_up_the_last_and_arrows_wrap() {
    let mut harness = harness(&Fixture::new(), false);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(selected(&harness), Some(0));
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(selected(&harness), Some(4), "wraps to Last");
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(selected(&harness), Some(0), "wraps to Alpha");
}

#[test]
fn up_from_nothing_selects_the_last_row() {
    let mut harness = harness(&Fixture::new(), false);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(selected(&harness), Some(4));
}

#[test]
fn disabled_rows_are_skipped_by_the_arrows_and_reflect_live_state() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(selected(&harness), Some(1), "Beta");
    fixture.beta_enabled.set(false);
    harness.run();
    assert!(harness.get_by_label("Beta").is_disabled());
    assert_eq!(
        selected(&harness),
        None,
        "a row that became disabled loses the highlight"
    );
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(
        selected(&harness),
        Some(2),
        "Beta is skipped: Alpha then Gamma"
    );
}

#[test]
fn enter_performs_the_action_closes_and_reports_it() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, false);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().performed, ["Alpha"]);
    assert_eq!(*fixture.log.borrow(), ["Alpha"]);
    assert!(!harness.state().chain.is_open());
    assert!(harness.query_by_label("Alpha").is_none());
}

#[test]
fn a_click_performs_on_release() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, false);
    harness.get_by_label("Gamma").click();
    harness.run();
    assert_eq!(*fixture.log.borrow(), ["Gamma"]);
    assert!(!harness.state().chain.is_open());
}

#[test]
fn a_disabled_row_cannot_be_clicked() {
    let fixture = Fixture::new();
    fixture.beta_enabled.set(false);
    let mut harness = harness(&fixture, false);
    harness.get_by_label("Beta").click();
    harness.run();
    assert!(fixture.log.borrow().is_empty());
    assert!(harness.state().chain.is_open());
}

#[test]
fn a_popup_group_is_a_submenu_that_opens_with_right_and_closes_with_left() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, false);
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(selected(&harness), Some(3), "More");
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.query_by_label("Inner A").is_some());
    assert_eq!(harness.state().chain.selected(1), Some(0));
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(harness.query_by_label("Inner A").is_none());
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*fixture.log.borrow(), ["Inner B"]);
    assert!(!harness.state().chain.is_open(), "the whole chain closes");
}

#[test]
fn escape_closes_the_submenu_first_and_shift_escape_closes_everything() {
    let mut harness = harness(&Fixture::new(), false);
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().chain.is_open());
    assert_eq!(harness.state().chain.depth(), 1);
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Escape);
    harness.run();
    assert!(!harness.state().chain.is_open());
    assert_eq!(harness.state().cancelled, 1);
}

#[test]
fn a_submenu_opens_250_ms_after_the_pointer_rests_on_its_row() {
    let mut harness = harness(&Fixture::new(), false);
    let more = harness.get_by_label("More").rect().center();
    harness.hover_at(more - vec2(10.0, 0.0));
    harness.run();
    harness.hover_at(more);
    harness.run_steps(6);
    assert!(harness.query_by_label("Inner A").is_none());
    harness.run_steps(14);
    assert!(harness.query_by_label("Inner A").is_some());
    assert_eq!(harness.state().chain.selected(0), Some(3));
}

#[test]
fn keep_popup_on_perform_leaves_the_menu_open() {
    let log = Log::default();
    let mut harness = testing::harness_state_sized(
        Theme::dark(),
        vec2(300.0, 200.0),
        PopupChain::default(),
        move |ui, chain| {
            let mut group = DefaultActionGroup::new("Menu");
            group.add(Cmd {
                id: "Sticky",
                enabled: Rc::new(Cell::new(true)),
                keep_open: true,
                log: log.clone(),
            });
            let context = DataContext::empty();
            ActionPopupMenu::new("menu", &group, &context, ActionPlace::POPUP).show_at(
                ui.ctx(),
                chain,
                pos2(10.0, 10.0),
            );
        },
    );
    harness.run();
    let now = harness.ctx.input(|i| i.time);
    harness.state_mut().open(now, false);
    harness.run();
    harness.get_by_label("Sticky").click();
    harness.run();
    assert!(harness.state().is_open());
}

#[test]
fn numbering_chooses_a_row_when_its_number_is_typed() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, true);
    // Alpha 1, Beta 2, Gamma 3.
    harness.event(egui::Event::Text("3".to_owned()));
    harness.run();
    assert_eq!(*fixture.log.borrow(), ["Gamma"]);
}

#[test]
fn it_cannot_reopen_within_200_ms_of_closing() {
    let mut harness = harness(&Fixture::new(), false);
    harness.key_press(Key::Escape);
    harness.run();
    let now = harness.ctx.input(|i| i.time);
    assert!(!harness.state_mut().chain.open(now, false));
    harness.run_steps(14);
    let now = harness.ctx.input(|i| i.time);
    assert!(harness.state_mut().chain.open(now, false));
}

#[test]
fn rows_are_named_and_the_audit_is_clean() {
    let mut harness = harness(&Fixture::new(), false);
    testing::audit(&mut harness).assert_clean();
}

// ---- PopupHandler::for_group -------------------------------------------------------------

fn handler_harness(fixture: &Fixture) -> Harness<'static, ()> {
    let fixture = Fixture {
        log: fixture.log.clone(),
        beta_enabled: fixture.beta_enabled.clone(),
    };
    let mut harness =
        testing::harness_state_sized(Theme::dark(), vec2(520.0, 400.0), (), move |ui, _| {
            let group = fixture.group();
            let context = DataContext::empty();
            ColoredListCellRenderer::new("src")
                .with(PopupHandler::for_group(
                    &group,
                    &context,
                    ActionPlace::PROJECT_VIEW_POPUP,
                ))
                .show(ui);
        });
    harness.run();
    harness
}

#[test]
fn a_context_menu_opens_at_the_pointer_on_right_click_and_selects_nothing() {
    let mut harness = handler_harness(&Fixture::new());
    assert!(harness.query_by_label("Gamma").is_none());
    harness.get_by_label("src").click_secondary();
    harness.run();
    assert!(harness.query_by_label("Gamma").is_some());
    let alpha = harness.get_by_label("Alpha").rect();
    let row = harness.get_by_label("src").rect();
    assert!(alpha.left() >= row.left(), "at the pointer, in the row");
    let highlighted = harness.output().shapes.iter().any(|c| match &c.shape {
        egui::Shape::Rect(r) => {
            r.fill == Theme::dark().colors.selection && alpha.contains(r.rect.center())
        }
        _ => false,
    });
    assert!(!highlighted, "no row is selected on open");
}

#[test]
fn a_context_menu_performs_the_chosen_action_and_closes() {
    let fixture = Fixture::new();
    let mut harness = handler_harness(&fixture);
    harness.get_by_label("src").click_secondary();
    harness.run();
    harness.get_by_label("Last").click();
    harness.run();
    assert_eq!(*fixture.log.borrow(), ["Last"]);
    assert!(harness.query_by_label("Last").is_none());
}

#[test]
fn escape_closes_a_context_menu() {
    let mut harness = handler_harness(&Fixture::new());
    harness.get_by_label("src").click_secondary();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("Gamma").is_none());
}

#[test]
fn a_press_outside_closes_a_context_menu() {
    let mut harness = handler_harness(&Fixture::new());
    harness.get_by_label("src").click_secondary();
    harness.run();
    harness.event(egui::Event::PointerMoved(pos2(500.0, 380.0)));
    harness.event(egui::Event::PointerButton {
        pos: pos2(500.0, 380.0),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(harness.query_by_label("Gamma").is_none());
}

#[test]
fn action_popup_menu_snapshot() {
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let state = PopupChain::default();
        let mut harness =
            testing::harness_state_sized(theme, vec2(470.0, 230.0), state, move |ui, chain| {
                common::backdrop(ui.ctx());
                struct Item(&'static str, Option<Glyph>, Option<bool>, &'static str);
                impl AnAction for Item {
                    fn id(&self) -> &str {
                        self.0
                    }
                    fn template_presentation(&self) -> Presentation {
                        let mut p = Presentation::new(self.0);
                        if let Some(icon) = self.1 {
                            p = p.icon(icon);
                        }
                        if let Some(on) = self.2 {
                            p = p.selected(on);
                        }
                        if !self.3.is_empty() {
                            p = p.shortcut_text(self.3);
                        }
                        p
                    }
                    fn action_performed(&self, _e: &AnActionEvent<'_>) {}
                }
                let mut new_menu = DefaultActionGroup::popup("New");
                new_menu
                    .add(Item("File…", Some(Glyph::NewFile), None, ""))
                    .add(Item("Directory…", Some(Glyph::NewFolder), None, ""));
                let mut group = DefaultActionGroup::new("Menu");
                group
                    .add(Item("Copy", Some(Glyph::Copy), None, "Ctrl+C"))
                    .add(Item("Paste", Some(Glyph::Paste), None, "Ctrl+V"))
                    .add_separator()
                    .add(Item("Soft-Wrap", None, Some(true), ""))
                    .add_separator_with_text("Open Projects")
                    .add_group(new_menu);
                let context = DataContext::empty();
                ActionPopupMenu::new("snap", &group, &context, ActionPlace::EDITOR_POPUP)
                    .show_submenu_on_hover(false)
                    .show_at(ui.ctx(), chain, pos2(10.0, 8.0));
            });
        harness.run();
        let now = harness.ctx.input(|i| i.time);
        harness.state_mut().open(now, false);
        harness.run();
        // Select "New" with the keyboard and open it.
        harness.key_press(Key::ArrowUp);
        harness.key_press(Key::ArrowRight);
        harness.run();
        harness
            .try_snapshot_options(
                format!("action_popup_menu.{suffix}"),
                &testing::SnapshotOptions::new().output_path(common::IMG),
            )
            .expect("snapshot matches");
    }
}
