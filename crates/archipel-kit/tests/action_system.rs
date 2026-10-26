//! Behavior of the action system model: update/perform gating, toggles, data context
//! lookup, group flattening, keymaps, shortcut text and the host services.

use std::cell::Cell;
use std::rc::Rc;

use archipel_kit::action_system::*;
use archipel_kit::egui::{Key, Modifiers, PointerButton};
use archipel_kit::host::{
    ActionManager, Clock, InMemoryProperties, KeymapManager, ManualClock, PropertiesComponent,
    SimpleActionManager, SimpleKeymapManager,
};

const FLAG: DataKey<bool> = DataKey::new("test.flag");

/// An action whose availability follows `FLAG` and that counts its runs.
struct Probe {
    id: &'static str,
    runs: Rc<Cell<u32>>,
    hidden_in_toolbar: bool,
}

impl Probe {
    fn new(id: &'static str) -> Self {
        Self {
            id,
            runs: Rc::default(),
            hidden_in_toolbar: false,
        }
    }
}

impl AnAction for Probe {
    fn id(&self) -> &str {
        self.id
    }
    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.id)
    }
    fn update(&self, e: &mut AnActionEvent<'_>) {
        e.presentation.enabled = e.data(FLAG).copied().unwrap_or(false);
        if self.hidden_in_toolbar && e.place.is_toolbar_place() {
            e.presentation.visible = false;
        }
    }
    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.runs.set(self.runs.get() + 1);
    }
}

fn with_flag(value: bool) -> DataContext {
    DataContext::builder().with(FLAG, value).build()
}

#[test]
fn perform_runs_only_when_enabled_and_visible() {
    let action = Probe::new("A");
    let runs = action.runs.clone();
    assert!(!perform(
        &action,
        &with_flag(false),
        &ActionPlace::POPUP,
        InputKind::Mouse
    ));
    assert_eq!(runs.get(), 0);
    assert!(perform(
        &action,
        &with_flag(true),
        &ActionPlace::POPUP,
        InputKind::Keyboard
    ));
    assert_eq!(runs.get(), 1);

    let hidden = Probe {
        hidden_in_toolbar: true,
        ..Probe::new("B")
    };
    let context = with_flag(true);
    assert!(!perform(
        &hidden,
        &context,
        &ActionPlace::MAIN_TOOLBAR,
        InputKind::None
    ));
    assert!(perform(
        &hidden,
        &context,
        &ActionPlace::POPUP,
        InputKind::None
    ));
}

#[test]
fn update_starts_from_the_template_and_reports_the_place() {
    struct PlaceName;
    impl AnAction for PlaceName {
        fn id(&self) -> &str {
            "P"
        }
        fn template_presentation(&self) -> Presentation {
            Presentation::new("template").description("d")
        }
        fn update(&self, e: &mut AnActionEvent<'_>) {
            e.presentation.text = format!("{}@{}", e.presentation.text, e.place);
        }
        fn action_performed(&self, _e: &AnActionEvent<'_>) {}
    }
    let presentation =
        update_presentation(&PlaceName, &DataContext::empty(), &ActionPlace::EDITOR_TAB);
    assert_eq!(presentation.text, "template@EditorTab");
    assert_eq!(presentation.description, "d");
}

#[test]
fn toggle_reports_and_flips_its_state() {
    #[derive(Default)]
    struct Wrap(Cell<bool>);
    impl ToggleAction for Wrap {
        fn id(&self) -> &str {
            "Wrap"
        }
        fn template_presentation(&self) -> Presentation {
            Presentation::new("Wrap")
        }
        fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool {
            self.0.get()
        }
        fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) {
            self.0.set(selected);
        }
    }
    let toggle = Toggle::new(Wrap::default());
    let context = DataContext::empty();
    assert_eq!(toggle.template_presentation().selected, Some(false));
    for expected in [true, false, true] {
        assert!(perform(
            &toggle,
            &context,
            &ActionPlace::TOOLBAR,
            InputKind::Mouse
        ));
        let presentation = update_presentation(&toggle, &context, &ActionPlace::TOOLBAR);
        assert_eq!(presentation.selected, Some(expected));
        assert_eq!(presentation.is_selected(), expected);
    }
}

#[test]
fn data_context_looks_up_parents_and_separates_types() {
    const TEXT: DataKey<String> = DataKey::new("test.shared");
    const NUMBER: DataKey<u32> = DataKey::new("test.shared");

    let root = Rc::new(
        DataContext::builder()
            .with(CommonDataKeys::TOOL_WINDOW_ID, "Run".to_owned())
            .with(TEXT, "outer".to_owned())
            .build(),
    );
    let child = DataContext::with_parent(root.clone())
        .with(TEXT, "inner".to_owned())
        .with(NUMBER, 7)
        .build();
    assert_eq!(
        child
            .get(CommonDataKeys::TOOL_WINDOW_ID)
            .map(String::as_str),
        Some("Run")
    );
    assert_eq!(child.get(TEXT).map(String::as_str), Some("inner"));
    assert_eq!(root.get(TEXT).map(String::as_str), Some("outer"));
    assert_eq!(child.get(NUMBER), Some(&7));
    assert_eq!(root.get(NUMBER), None);
    assert!(!child.contains(CommonDataKeys::SPEED_SEARCH_TEXT));
    assert!(child.parent().is_some());
}

fn names(rows: &[ActionRow]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            ActionRow::Action { presentation, .. } | ActionRow::Group { presentation, .. } => {
                presentation.text.clone()
            }
            ActionRow::Separator(separator) => match &separator.text {
                Some(text) => format!("--{text}--"),
                None => "-".to_owned(),
            },
            _ => "?".to_owned(),
        })
        .collect()
}

#[test]
fn flatten_hides_invisible_and_collapses_separators() {
    let mut group = DefaultActionGroup::new("g");
    group
        .add_separator()
        .add(Probe::new("a"))
        .add_separator()
        .add_separator()
        .add(Probe {
            hidden_in_toolbar: true,
            ..Probe::new("hidden")
        })
        .add_separator()
        .add(Probe::new("b"))
        .add_separator();
    let context = with_flag(true);
    let rows = flatten(&group, &context, &ActionPlace::MAIN_TOOLBAR);
    assert_eq!(names(&rows), ["a", "-", "b"]);
    let rows = flatten(&group, &context, &ActionPlace::POPUP);
    assert_eq!(names(&rows), ["a", "-", "hidden", "-", "b"]);
    // Disabled actions stay in the list.
    let rows = flatten(&group, &with_flag(false), &ActionPlace::POPUP);
    assert_eq!(rows.len(), 5);
    assert!(
        rows.iter()
            .filter_map(ActionRow::presentation)
            .all(|p| !p.enabled)
    );
}

#[test]
fn titled_separators_may_lead_and_win_over_plain_ones() {
    let mut group = DefaultActionGroup::new("g");
    group
        .add_separator_with_text("Recent")
        .add(Probe::new("a"))
        .add_separator()
        .add_separator_with_text("Other")
        .add(Probe::new("b"))
        .add_separator_with_text("Empty");
    let rows = flatten(&group, &with_flag(true), &ActionPlace::POPUP);
    assert_eq!(names(&rows), ["--Recent--", "a", "--Other--", "b"]);
}

#[test]
fn flat_groups_inline_and_popup_groups_stay_rows() {
    let mut inner = DefaultActionGroup::new("inner");
    inner.add(Probe::new("x")).add(Probe::new("y"));
    let mut submenu = DefaultActionGroup::popup("More");
    submenu.add(Probe::new("z"));
    let mut group = DefaultActionGroup::new("g");
    group
        .add(Probe::new("a"))
        .add_separator()
        .add_group(inner)
        .add_group(submenu);
    let rows = flatten(&group, &with_flag(true), &ActionPlace::POPUP);
    assert_eq!(names(&rows), ["a", "-", "x", "y", "More"]);
    let more = rows
        .last()
        .and_then(ActionRow::presentation)
        .expect("popup row");
    assert!(more.popup_group);
}

#[test]
fn empty_popup_groups_follow_their_flags() {
    struct Empty {
        hide: bool,
        disable: bool,
    }
    impl ActionGroup for Empty {
        fn name(&self) -> &str {
            "Empty"
        }
        fn is_popup(&self) -> bool {
            true
        }
        fn children(&self, _e: Option<&AnActionEvent<'_>>) -> Vec<ActionItem> {
            vec![ActionItem::separator()]
        }
        fn template_presentation(&self) -> Presentation {
            Presentation::new("Empty")
                .popup_group(true)
                .hide_group_if_empty(self.hide)
                .disable_group_if_empty(self.disable)
        }
    }
    let place = ActionPlace::POPUP;
    let context = DataContext::empty();
    let mut hidden = DefaultActionGroup::new("g");
    hidden.add_group(Empty {
        hide: true,
        disable: false,
    });
    assert!(flatten(&hidden, &context, &place).is_empty());
    let mut disabled = DefaultActionGroup::new("g");
    disabled.add_group(Empty {
        hide: false,
        disable: true,
    });
    let rows = flatten(&disabled, &context, &place);
    assert!(!rows[0].presentation().expect("row").enabled);
}

#[test]
fn a_group_containing_itself_does_not_recurse_forever() {
    struct Loop;
    impl ActionGroup for Loop {
        fn children(&self, _e: Option<&AnActionEvent<'_>>) -> Vec<ActionItem> {
            vec![ActionItem::group(Loop), ActionItem::action(Probe::new("a"))]
        }
    }
    let rows = flatten(&Loop, &with_flag(true), &ActionPlace::POPUP);
    assert!(rows.len() <= MAX_GROUP_DEPTH + 2);
}

#[test]
fn flatten_with_shortcuts_fills_the_text() {
    let mut group = DefaultActionGroup::new("g");
    group
        .add(Probe::new(IdeActions::ACTION_COPY))
        .add(Probe::new("NoKey"));
    let keymap = Keymap::default_for(Os::Linux);
    let rows = flatten_with_shortcuts(
        &group,
        &with_flag(true),
        &ActionPlace::POPUP,
        &keymap,
        false,
    );
    let texts: Vec<_> = rows
        .iter()
        .filter_map(ActionRow::presentation)
        .map(|p| p.shortcut_text.clone())
        .collect();
    assert_eq!(texts, [Some("Ctrl+C".to_owned()), None]);
}

#[test]
fn places_know_popups_and_toolbars() {
    for place in [
        ActionPlace::POPUP,
        ActionPlace::EDITOR_POPUP,
        ActionPlace::PROJECT_VIEW_POPUP,
        ActionPlace::TOOLWINDOW_POPUP,
    ] {
        assert!(place.is_popup_place(), "{place}");
    }
    for place in [
        ActionPlace::MAIN_TOOLBAR,
        ActionPlace::EDITOR_TAB,
        ActionPlace::STATUS_BAR_PLACE,
        ActionPlace::TOOLWINDOW_TITLE,
    ] {
        assert!(!place.is_popup_place(), "{place}");
    }
    assert!(ActionPlace::MAIN_TOOLBAR.is_toolbar_place());
    assert!(ActionPlace::new("MyToolbar").is_toolbar_place());
    assert_eq!(ActionPlace::new("EditorPopup"), ActionPlace::EDITOR_POPUP);
    assert_eq!(ActionPlace::default(), ActionPlace::UNKNOWN);
    let context = DataContext::empty();
    let event = AnActionEvent::new(&context, ActionPlace::EDITOR_POPUP);
    assert!(event.is_from_context_menu() && !event.is_from_action_toolbar());
}

fn stroke(text: &str) -> KeyStroke {
    KeyStroke::parse(text).expect("valid stroke")
}

#[test]
fn keymap_lookup_and_inheritance() {
    let keymap = Keymap::default_for(Os::Windows);
    assert_eq!(
        keymap.resolve(&stroke("ctrl shift Z")),
        KeyResolution::Matched(vec![IdeActions::ACTION_REDO.to_owned()])
    );
    assert_eq!(
        keymap.resolve(&stroke("ctrl alt shift F9")),
        KeyResolution::NoMatch
    );
    // Esc is both the editor's escape and "focus editor": a conflict lists both.
    assert_eq!(
        keymap.resolve(&stroke("Esc")),
        KeyResolution::Matched(vec!["EditorEscape".to_owned(), "FocusEditor".to_owned()])
    );

    let mut child = Keymap::with_parent("child", Rc::new(keymap));
    assert_eq!(child.shortcuts(IdeActions::ACTION_COPY).len(), 2);
    child.set_shortcuts(IdeActions::ACTION_COPY, vec![stroke("ctrl alt C").into()]);
    assert_eq!(child.shortcuts(IdeActions::ACTION_COPY).len(), 1);
    assert_eq!(child.resolve(&stroke("ctrl C")), KeyResolution::NoMatch);
    assert!(matches!(
        child.resolve(&stroke("ctrl alt C")),
        KeyResolution::Matched(_)
    ));
    // Inherited entries are untouched.
    assert!(matches!(
        child.resolve(&stroke("ctrl V")),
        KeyResolution::Matched(_)
    ));
    child.remove_shortcut(IdeActions::ACTION_PASTE, &stroke("ctrl V").into());
    assert_eq!(child.shortcuts(IdeActions::ACTION_PASTE).len(), 1);
    assert!(
        child
            .action_ids()
            .contains(&IdeActions::ACTION_PASTE.to_owned())
    );
}

#[test]
fn strokes_ignore_egui_command_flag() {
    let from_event = KeyStroke::new(Key::C, Modifiers::CTRL); // command is true on Linux
    assert_eq!(from_event, stroke("ctrl C"));
    assert_ne!(from_event, stroke("ctrl shift C"));
    assert_ne!(stroke("cmd C"), stroke("ctrl C"));
}

#[test]
fn every_default_shortcut_parses() {
    for os in [Os::Windows, Os::Linux, Os::MacOs] {
        let keymap = Keymap::default_for(os);
        assert!(keymap.action_ids().len() > 30, "{os:?}");
    }
    // Spot checks against the notes.
    let mac = Keymap::default_for(Os::MacOs);
    assert_eq!(
        KeymapUtil::first_keyboard_shortcut_text(&mac, IdeActions::ACTION_REDO, true).as_deref(),
        Some("⇧⌘Z")
    );
    assert!(mac.shortcuts("Switcher").is_empty());
    let linux = Keymap::default_for(Os::Linux);
    assert_eq!(
        KeymapUtil::first_keyboard_shortcut_text(
            &linux,
            IdeActions::ACTION_SELECT_ALL_OCCURRENCES,
            false
        )
        .as_deref(),
        Some("Ctrl+Alt+Shift+J")
    );
}

#[test]
fn two_stroke_shortcuts_respect_the_timeout() {
    let mut keymap = Keymap::new("k");
    keymap.add_shortcut(
        "Comment",
        KeyboardShortcut::parse("ctrl K > ctrl C").expect("valid"),
    );
    keymap.add_shortcut("Solo", stroke("ctrl K"));
    // A one-stroke match wins over the start of a chord.
    assert!(matches!(
        keymap.resolve(&stroke("ctrl K")),
        KeyResolution::Matched(_)
    ));

    let mut keymap = Keymap::new("k");
    keymap.add_shortcut(
        "Comment",
        KeyboardShortcut::parse("ctrl K > ctrl C").expect("valid"),
    );
    let mut pending = None;
    assert_eq!(
        keymap.feed(&mut pending, stroke("ctrl K"), 0.0),
        KeyResolution::AwaitSecond
    );
    assert!(pending.is_some());
    let second = SECOND_KEYSTROKE_TIMEOUT.as_secs_f64();
    assert_eq!(
        keymap.feed(&mut pending, stroke("ctrl C"), second),
        KeyResolution::Matched(vec!["Comment".to_owned()])
    );
    assert!(pending.is_none());

    // Too late: the second stroke is just a stroke bound to nothing.
    keymap.feed(&mut pending, stroke("ctrl K"), 0.0);
    assert_eq!(
        keymap.feed(&mut pending, stroke("ctrl C"), second + 0.1),
        KeyResolution::NoMatch
    );
    // A wrong second stroke clears the pending one.
    keymap.feed(&mut pending, stroke("ctrl K"), 0.0);
    assert_eq!(
        keymap.feed(&mut pending, stroke("ctrl X"), 0.5),
        KeyResolution::NoMatch
    );
    assert!(pending.is_none());
}

#[test]
fn shortcut_text_in_both_styles() {
    let text = |source: &str, mac: bool| {
        KeymapUtil::shortcut_text(&KeyboardShortcut::parse(source).expect("valid"), mac)
    };
    assert_eq!(text("ctrl C", false), "Ctrl+C");
    assert_eq!(text("ctrl alt shift J", false), "Ctrl+Alt+Shift+J");
    assert_eq!(text("alt shift Up", false), "Alt+Shift+Up");
    assert_eq!(text("shift PgDn", false), "Shift+PgDn");
    assert_eq!(text("Esc", false), "Esc");
    assert_eq!(text("ctrl F4", false), "Ctrl+F4");
    assert_eq!(text("ctrl K > ctrl C", false), "Ctrl+K, Ctrl+C");
    assert_eq!(text("ctrl /", false), "Ctrl+/");

    assert_eq!(text("cmd C", true), "⌘C");
    assert_eq!(text("ctrl alt shift cmd J", true), "⌃⌥⇧⌘J");
    assert_eq!(text("alt Left", true), "⌥←");
    assert_eq!(text("cmd Backspace", true), "⌘⌫");
    assert_eq!(text("Esc", true), "⎋");
    assert_eq!(text("shift Enter", true), "⇧↩");
    assert_eq!(text("cmd K > cmd C", true), "⌘K ⌘C");

    let click = Shortcut::Mouse(MouseShortcut::new(
        PointerButton::Primary,
        2,
        Modifiers::ALT,
    ));
    assert_eq!(KeymapUtil::text(&click, false), "Alt+Double Click");
    assert!(!click.is_keyboard());
}

#[test]
fn parse_rejects_garbage_without_panicking() {
    for bad in [
        "",
        "ctrl",
        "hyper A",
        "ctrl A B",
        "ctrl K > ",
        ">",
        "ctrl shift",
    ] {
        assert!(KeyboardShortcut::parse(bad).is_none(), "{bad:?}");
    }
}

#[test]
fn action_manager_registers_and_unregisters() {
    let mut manager = SimpleActionManager::new();
    manager.register_action(Rc::new(Probe::new("Edit.Copy")));
    manager.register_action(Rc::new(Probe::new("Edit.Cut")));
    manager.register_group("Edit.Menu", Rc::new(DefaultActionGroup::popup("Edit")));
    assert!(manager.action("Edit.Copy").is_some());
    assert!(manager.group("Edit.Menu").is_some());
    assert!(manager.is_group("Edit.Menu") && !manager.is_group("Edit.Copy"));
    assert_eq!(
        manager.action_ids("Edit."),
        ["Edit.Copy", "Edit.Cut", "Edit.Menu"]
    );
    assert!(manager.unregister("Edit.Copy"));
    assert!(!manager.unregister("Edit.Copy"));
    assert!(manager.action("Edit.Copy").is_none());
}

#[test]
fn keymap_manager_and_properties_and_clock() {
    let manager = SimpleKeymapManager::new(Os::MacOs);
    assert!(manager.is_mac());
    assert_eq!(manager.active_keymap().name(), "macOS");

    let mut properties = InMemoryProperties::new();
    assert!(!properties.is_set("splitter"));
    properties.set("splitter", "0.25");
    properties.set_bool("tip.shown", true);
    properties.set_int("count", 3);
    properties.set("bad", "x");
    assert_eq!(properties.get("splitter").as_deref(), Some("0.25"));
    assert!(properties.get_bool("tip.shown", false));
    assert!(properties.get_bool("missing", true));
    assert_eq!(properties.get_int("count", 0), 3);
    assert_eq!(properties.get_int("bad", 9), 9);
    properties.unset("count");
    assert!(!properties.is_set("count"));
    assert_eq!(properties.entries().count(), 3);

    let clock = ManualClock::new();
    clock.advance(1.5);
    clock.advance(-3.0);
    assert!((clock.now() - 1.5).abs() < f64::EPSILON);
}

#[test]
fn presentation_builders_compose() {
    let presentation = Presentation::new("Run")
        .description("Run it")
        .selected(true)
        .popup_group(true)
        .perform_group(true)
        .shortcut_text("Ctrl+R")
        .disabled_reason("No target")
        .enabled(false);
    assert!(presentation.has_text() && presentation.is_selected());
    assert!(!presentation.is_actionable());
    assert_eq!(presentation.disabled_reason.as_deref(), Some("No target"));
    let hidden = Presentation::default().enabled_and_visible(false);
    assert!(!hidden.enabled && !hidden.visible);
}
