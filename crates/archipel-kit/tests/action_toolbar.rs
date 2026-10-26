//! `ActionToolbar`: an action group as buttons, drop-downs and overflow (spec §1.4, §1.5).

mod common;

use archipel_kit::action_system::*;
use archipel_kit::testing::{self, Harness, NodeExt, NodeT, Queryable};
use archipel_kit::{Glyph, Theme};
use egui::{Key, pos2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Log = Rc<RefCell<Vec<String>>>;

struct Cmd {
    id: &'static str,
    glyph: Glyph,
    log: Log,
    on: Option<Rc<Cell<bool>>>,
}

impl AnAction for Cmd {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        let mut p = Presentation::new(self.id).icon(self.glyph);
        if let Some(on) = &self.on {
            p = p.selected(on.get());
        }
        p
    }

    fn update(&self, e: &mut AnActionEvent<'_>) {
        if let Some(on) = &self.on {
            e.presentation.selected = Some(on.get());
        }
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.log.borrow_mut().push(self.id.to_owned());
        if let Some(on) = &self.on {
            on.set(!on.get());
        }
    }
}

/// A popup group with a "+" icon.
struct IconGroup(DefaultActionGroup);

impl ActionGroup for IconGroup {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn children(&self, e: Option<&AnActionEvent<'_>>) -> Vec<ActionItem> {
        self.0.children(e)
    }

    fn is_popup(&self) -> bool {
        true
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.0.name())
            .icon(Glyph::Plus)
            .popup_group(true)
    }
}

struct Fixture {
    log: Log,
    wrap: Rc<Cell<bool>>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            log: Log::default(),
            wrap: Rc::new(Cell::new(false)),
        }
    }

    fn cmd(&self, id: &'static str, glyph: Glyph) -> Cmd {
        Cmd {
            id,
            glyph,
            log: self.log.clone(),
            on: None,
        }
    }

    /// Reload, Run, a separator, Soft-Wrap (toggle), and a popup group "Add" with two items.
    fn group(&self) -> DefaultActionGroup {
        let mut add = DefaultActionGroup::popup("Add").with_id("Add");
        add.add(self.cmd("New File", Glyph::NewFile))
            .add(self.cmd("New Folder", Glyph::NewFolder));
        let mut group = DefaultActionGroup::new("Tools");
        group
            .add(self.cmd("Reload", Glyph::Refresh))
            .add(self.cmd("Run", Glyph::Run))
            .add_separator()
            .add(Cmd {
                on: Some(self.wrap.clone()),
                ..self.cmd("Soft-Wrap", Glyph::SoftWrap)
            })
            .add_group(IconGroup(add));
        group
    }
}

#[derive(Default)]
struct State {
    invoked: Vec<String>,
    overflowed: usize,
    open: bool,
}

fn harness(fixture: &Fixture, width: f32, vertical: bool) -> Harness<'static, State> {
    let (log, wrap) = (fixture.log.clone(), fixture.wrap.clone());
    let mut harness = testing::harness_state_sized(
        Theme::dark(),
        vec2(width.max(200.0), 300.0),
        State::default(),
        move |ui, s| {
            let fixture = Fixture {
                log: log.clone(),
                wrap: wrap.clone(),
            };
            let group = fixture.group();
            let context = DataContext::empty();
            let mut secondary = DefaultActionGroup::popup("Options");
            secondary.add(fixture.cmd("Hide", Glyph::EyeOff));
            ui.set_max_width(width);
            let mut toolbar = ActionToolbar::new("tools", &group, &context, ActionPlace::TOOLBAR);
            if vertical {
                toolbar = toolbar.orientation(ToolbarOrientation::Vertical);
            }
            let out = toolbar.show(ui);
            s.overflowed = out.overflowed;
            s.open = out.popup_open;
            if let Some(id) = out.invoked {
                s.invoked.push(id);
            }
        },
    );
    harness.run();
    harness
}

#[test]
fn it_shows_one_button_per_action_and_a_dropdown_for_a_popup_group() {
    let harness = harness(&Fixture::new(), 400.0, false);
    for label in ["Reload", "Run", "Soft-Wrap", "Add"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    assert_eq!(
        harness.get_by_label("Reload").rect().size(),
        vec2(22.0, 22.0)
    );
    let add = harness.get_by_label("Add").rect();
    assert_eq!(
        add.width(),
        22.0 + archipel_kit::tokens::action_toolbar::ARROW_WIDTH,
        "a popup group has a drop-down arrow"
    );
}

#[test]
fn buttons_are_laid_out_left_to_right_4_pt_apart_with_a_separator_gap() {
    let harness = harness(&Fixture::new(), 400.0, false);
    let reload = harness.get_by_label("Reload").rect();
    let run = harness.get_by_label("Run").rect();
    let wrap = harness.get_by_label("Soft-Wrap").rect();
    assert_eq!(run.left() - reload.right(), 4.0);
    assert!(run.top() == reload.top());
    // 4 + (4 + 1 + 4) separator + 4.
    assert_eq!(wrap.left() - run.right(), 4.0 + 9.0 + 4.0);
}

#[test]
fn vertical_toolbars_stack_their_buttons() {
    let harness = harness(&Fixture::new(), 400.0, true);
    let reload = harness.get_by_label("Reload").rect();
    let run = harness.get_by_label("Run").rect();
    assert_eq!(run.left(), reload.left());
    assert_eq!(run.top() - reload.bottom(), 4.0);
}

#[test]
fn clicking_a_button_performs_the_action_and_reports_it() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, 400.0, false);
    harness.get_by_label("Run").click();
    harness.run();
    assert_eq!(*fixture.log.borrow(), ["Run"]);
    assert_eq!(harness.state().invoked, ["Run"]);
}

#[test]
fn a_toggle_action_shows_and_flips_its_state() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, 400.0, false);
    assert!(!harness.get_by_label("Soft-Wrap").is_checked());
    harness.get_by_label("Soft-Wrap").click();
    harness.run();
    harness.run();
    assert!(harness.get_by_label("Soft-Wrap").is_checked());
    assert!(fixture.wrap.get());
}

#[test]
fn a_popup_group_opens_a_menu_with_the_first_row_selected() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, 400.0, false);
    harness.get_by_label("Add").click();
    harness.run();
    harness.run();
    assert!(harness.state().open);
    assert!(harness.query_by_label("New File").is_some());
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        *fixture.log.borrow(),
        ["New File"],
        "Enter chose the first row"
    );
    assert_eq!(harness.state().invoked, ["New File"]);
    assert!(!harness.state().open);
}

#[test]
fn the_menu_sits_below_its_button() {
    let mut harness = harness(&Fixture::new(), 400.0, false);
    harness.get_by_label("Add").click();
    harness.run();
    harness.run();
    let button = harness.get_by_label("Add").rect();
    let row = harness.get_by_label("New File").rect();
    assert!(row.top() > button.bottom());
    assert!(row.left() >= button.left() - 1.0);
}

#[test]
fn clicking_the_button_again_closes_the_menu_and_does_not_reopen_it() {
    let mut harness = harness(&Fixture::new(), 400.0, false);
    harness.get_by_label("Add").click();
    harness.run();
    harness.run();
    assert!(harness.state().open);
    harness.get_by_label("Add").click();
    harness.run();
    harness.run();
    assert!(!harness.state().open, "the second click closes it");
    assert!(harness.query_by_label("New File").is_none());
}

#[test]
fn escape_closes_the_menu() {
    let mut harness = harness(&Fixture::new(), 400.0, false);
    harness.get_by_label("Add").click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().open);
}

#[test]
fn rows_that_do_not_fit_move_into_a_more_actions_menu() {
    let fixture = Fixture::new();
    // Room for about three buttons.
    let mut harness = harness(&fixture, 100.0, false);
    assert!(harness.state().overflowed > 0);
    assert!(harness.query_by_label("More actions").is_some());
    assert!(
        harness.query_by_label("Add").is_none(),
        "the last row overflowed"
    );
    harness.get_by_label("More actions").click();
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("Add").is_some(),
        "its menu lists the rest"
    );
    harness.get_by_label("Add").click();
    harness.run();
}

#[test]
fn the_overflow_menu_performs_its_actions() {
    let fixture = Fixture::new();
    let mut harness = harness(&fixture, 60.0, false);
    harness.get_by_label("More actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Run").click();
    harness.run();
    assert_eq!(harness.state().invoked, ["Run"]);
}

#[test]
fn nothing_overflows_when_there_is_room() {
    let harness = harness(&Fixture::new(), 400.0, false);
    assert_eq!(harness.state().overflowed, 0);
    assert!(harness.query_by_label("More actions").is_none());
}

#[test]
fn a_toolbar_has_no_leading_or_trailing_separator() {
    let fixture = Fixture::new();
    let group = {
        let mut g = DefaultActionGroup::new("t");
        g.add_separator()
            .add(fixture.cmd("Only", Glyph::Run))
            .add_separator();
        g
    };
    let context = DataContext::empty();
    let mut harness = testing::harness_state(Theme::dark(), 0.0_f32, |ui, width| {
        *width = ActionToolbar::new("t", &group, &context, ActionPlace::TOOLBAR)
            .show(ui)
            .response
            .rect
            .width();
    });
    harness.run();
    // A plain toolbar is inset 2 pt: its width is the one button and the border.
    assert_eq!(*harness.state(), 22.0 + 2.0 * 2.0);
}

#[test]
fn a_decorated_toolbar_has_the_new_ui_insets() {
    let fixture = Fixture::new();
    let mut group = DefaultActionGroup::new("t");
    group.add(fixture.cmd("Only", Glyph::Run));
    let context = DataContext::empty();
    let mut harness = testing::harness_state(Theme::dark(), vec2(0.0, 0.0), |ui, size| {
        *size = ActionToolbar::new("t", &group, &context, ActionPlace::TOOLBAR)
            .decorated(true)
            .show(ui)
            .response
            .rect
            .size();
    });
    harness.run();
    assert_eq!(
        *harness.state(),
        vec2(22.0 + 14.0, 22.0 + 10.0),
        "insets (5, 7)"
    );
}

#[test]
fn a_secondary_group_is_a_drop_down_at_the_far_end() {
    let fixture = Fixture::new();
    let log = fixture.log.clone();
    let mut harness =
        testing::harness_state_sized(Theme::dark(), vec2(400.0, 200.0), (), move |ui, _| {
            let fixture = Fixture {
                log: log.clone(),
                wrap: Rc::new(Cell::new(false)),
            };
            let group = fixture.group();
            let mut secondary = DefaultActionGroup::popup("Options");
            secondary.add(fixture.cmd("Hide", Glyph::EyeOff));
            let context = DataContext::empty();
            ActionToolbar::new("t", &group, &context, ActionPlace::TOOLBAR)
                .secondary(&secondary)
                .show(ui);
        });
    harness.run();
    let options = harness.get_by_label("Options").rect();
    let add = harness.get_by_label("Add").rect();
    assert!(options.left() > add.right());
    harness.get_by_label("Options").click();
    harness.run();
    harness.run();
    harness.get_by_label("Hide").click();
    harness.run();
    assert_eq!(*fixture.log.borrow(), ["Hide"]);
}

#[test]
fn toolbar_buttons_are_not_tab_stops_when_not_focusable() {
    let fixture = Fixture::new();
    let (log, wrap) = (fixture.log.clone(), fixture.wrap.clone());
    let mut harness = testing::harness(Theme::dark(), move |ui| {
        let fixture = Fixture {
            log: log.clone(),
            wrap: wrap.clone(),
        };
        let group = fixture.group();
        let context = DataContext::empty();
        ActionToolbar::new("t", &group, &context, ActionPlace::TOOLBAR)
            .focusable(false)
            .show(ui);
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(
        !harness.get_by_label("Reload").accesskit_node().is_focused(),
        "Tab never lands on a toolbar button"
    );
}

#[test]
fn the_toolbar_passes_the_audit() {
    let fixture = Fixture::new();
    let (log, wrap) = (fixture.log.clone(), fixture.wrap.clone());
    let mut harness = testing::harness(Theme::dark(), move |ui| {
        let fixture = Fixture {
            log: log.clone(),
            wrap: wrap.clone(),
        };
        let group = fixture.group();
        let context = DataContext::empty();
        ActionToolbar::new("t", &group, &context, ActionPlace::TOOLBAR).show(ui);
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn action_toolbar_snapshot() {
    for theme in [Theme::dark(), Theme::light()] {
        let suffix = if theme.dark { "dark" } else { "light" };
        let fixture = Fixture::new();
        fixture.wrap.set(true);
        let (log, wrap) = (fixture.log.clone(), fixture.wrap.clone());
        let mut harness =
            testing::harness_state_sized(theme, vec2(360.0, 180.0), (), move |ui, _| {
                common::backdrop(ui.ctx());
                let fixture = Fixture {
                    log: log.clone(),
                    wrap: wrap.clone(),
                };
                let group = fixture.group();
                let context = DataContext::empty();
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ActionToolbar::new("snap", &group, &context, ActionPlace::TOOLBAR).show(ui);
                });
            });
        harness.run();
        harness.get_by_label("Add").click();
        harness.run();
        harness.run();
        harness.hover_at(pos2(900.0, 900.0));
        harness
            .try_snapshot_options(
                format!("action_toolbar.{suffix}"),
                &testing::SnapshotOptions::new().output_path(common::IMG),
            )
            .expect("snapshot matches");
    }
}
