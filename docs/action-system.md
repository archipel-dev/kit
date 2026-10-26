# Action system

The action system is the model behind toolbars, menus, popup lists and shortcuts, the way
IntelliJ's `AnAction` is: one object per command, shown in many places, each asking it "are
you available, and how do you look?" every frame. It has no painting. Components read the
[`Presentation`](../crates/archipel-kit/src/action_system/presentation.rs)s it computes.

| IntelliJ | Kit (`archipel_kit::action_system`) |
|---|---|
| `AnAction`, `ToggleAction` | `AnAction`, `ToggleAction` + `Toggle` adapter |
| `Presentation` | `Presentation` (a value, not an observable) |
| `AnActionEvent`, `ActionUtil` | `AnActionEvent`, `update_presentation`, `perform` |
| `DataContext`, `DataKey`, `CommonDataKeys` | `DataContext`, `DataKey`, `CommonDataKeys` |
| `ActionGroup`, `DefaultActionGroup`, `Separator` | same names, plus `ActionItem`, `flatten` |
| `ActionPlaces` | `ActionPlace` constants |
| `Keymap`, `KeyboardShortcut`, `KeymapUtil` | same names |
| `ActionManager`, `KeymapManager`, `PropertiesComponent` | traits in `archipel_kit::host` |

## Define an action

An action has an id, a template presentation, an `update` that adjusts the presentation for
this frame and place, and an `action_performed`. Methods take `&self`: keep state in the
caller (`Cell`, `RefCell`, a channel).

```rust
use std::cell::RefCell;
use std::rc::Rc;
use archipel_kit::action_system::*;

const SELECTION: DataKey<Vec<String>> = DataKey::new("example.selection");

struct Delete {
    deleted: Rc<RefCell<Vec<String>>>,
}

impl AnAction for Delete {
    fn id(&self) -> &str { "Example.Delete" }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Delete").description("Delete the selected rows")
    }

    fn update(&self, e: &mut AnActionEvent<'_>) {
        // Enabled only when something is selected; hidden in the toolbar when not.
        let has_selection = e.data(SELECTION).is_some_and(|items| !items.is_empty());
        e.presentation.enabled = has_selection;
        if e.place == ActionPlace::MAIN_TOOLBAR {
            e.presentation.visible = has_selection;
        }
    }

    fn action_performed(&self, e: &AnActionEvent<'_>) {
        if let Some(items) = e.data(SELECTION) {
            self.deleted.borrow_mut().extend(items.iter().cloned());
        }
    }
}

let deleted = Rc::new(RefCell::new(Vec::new()));
let action = Delete { deleted: deleted.clone() };

// Every frame: build the context from your state, then update and draw.
let context = DataContext::builder()
    .with(SELECTION, vec!["a.txt".to_owned()])
    .build();
let presentation = update_presentation(&action, &context, &ActionPlace::POPUP);
assert!(presentation.enabled);

// On click: perform re-runs update and only runs a enabled, visible action.
assert!(perform(&action, &context, &ActionPlace::POPUP, InputKind::Mouse));
assert_eq!(*deleted.borrow(), ["a.txt"]);
assert!(!perform(&action, &DataContext::empty(), &ActionPlace::POPUP, InputKind::Mouse));
```

### Toggles

Implement `ToggleAction` (`is_selected`, `set_selected`) and wrap it in `Toggle`. The wrapper
copies the state into `presentation.selected` on every update and flips it on perform.
Renderers draw `selected: Some(_)` as a pressed button, a checkmark or a checkbox row.

```rust
use std::cell::Cell;
use archipel_kit::action_system::*;

#[derive(Default)]
struct SoftWrap(Cell<bool>);

impl ToggleAction for SoftWrap {
    fn id(&self) -> &str { "Example.SoftWrap" }
    fn template_presentation(&self) -> Presentation { Presentation::new("Soft-Wrap") }
    fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool { self.0.get() }
    fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) { self.0.set(selected) }
}

let action = Toggle::new(SoftWrap::default());
let context = DataContext::empty();
perform(&action, &context, &ActionPlace::TOOLBAR, InputKind::Mouse);
assert_eq!(update_presentation(&action, &context, &ActionPlace::TOOLBAR).selected, Some(true));
```

`DumbAwareAction` has no counterpart: it marks actions usable while IntelliJ indexes, and the
kit has no indexing.

## Data context

`DataContext` is a typed map. IntelliJ fills it by walking the focused component's parents;
here you build it and chain contexts with `DataContext::with_parent`, so the list can add
its selection on top of the window's tool window id. Keys are `const DataKey<T>` values:
there is no global registry. `CommonDataKeys` has the few keys that need no project model
(selected items, text range, file path, tool window id, speed search text, context menu
point).

```rust
use std::rc::Rc;
use archipel_kit::action_system::*;

let window = Rc::new(
    DataContext::builder()
        .with(CommonDataKeys::TOOL_WINDOW_ID, "Run".to_owned())
        .build(),
);
let list = DataContext::with_parent(window)
    .with(CommonDataKeys::SELECTED_ITEMS, vec!["row 3".to_owned()])
    .build();
assert_eq!(list.get(CommonDataKeys::TOOL_WINDOW_ID).map(String::as_str), Some("Run"));
```

## Groups and rows

`DefaultActionGroup` is an ordered list of actions, separators and nested groups. A flat
group is inlined into its parent; a popup group (`DefaultActionGroup::popup`) stays one row
that opens a submenu. `flatten` updates every child for a place and returns the rows to draw:
invisible ones are dropped, separators never lead, trail or repeat, and an empty popup group
is hidden or disabled by its presentation flags.

```rust
use archipel_kit::action_system::*;

struct Command(&'static str);
impl AnAction for Command {
    fn id(&self) -> &str { self.0 }
    fn template_presentation(&self) -> Presentation { Presentation::new(self.0) }
    fn action_performed(&self, _e: &AnActionEvent<'_>) {}
}

let mut menu = DefaultActionGroup::new("Edit").with_id("EditMenu");
menu.add(Command("Cut")).add(Command("Copy")).add_separator().add(Command("Paste"));

let keymap = Keymap::default_for(Os::Linux);
let rows = flatten_with_shortcuts(&menu, &DataContext::empty(), &ActionPlace::EDITOR_POPUP, &keymap, false);
assert_eq!(rows.len(), 4);
```

Register actions and groups with a host `ActionManager` (`SimpleActionManager` is the
in-memory one) when you need to look them up by id.

## Keymaps

A `Keymap` maps action ids to `Shortcut`s. `Keymap::default_for(Os)` has the IntelliJ default
shortcuts of the actions the kit handles (clipboard, undo, find, occurrences, escape, tab
switching). Add or remove your own, or inherit with `Keymap::with_parent`. `KeyStroke::parse`
reads IntelliJ's `"ctrl shift A"` form, and `KeyboardShortcut::parse("ctrl K > ctrl C")` reads a
chord.

Feed the keys you receive to `Keymap::feed` with a `pending` slot you keep between frames. A
first stroke of a chord returns `AwaitSecond`; the second must arrive within
`SECOND_KEYSTROKE_TIMEOUT` (2000 ms, IntelliJ's `actionSystem.secondKeystrokeTimeout`).

```rust
use archipel_kit::action_system::*;
use archipel_kit::egui::{Key, Modifiers};

let mut keymap = Keymap::new("Mine");
keymap.add_shortcut("Example.Comment", KeyboardShortcut::parse("ctrl K > ctrl C").expect("valid"));

let mut pending = None;
let ctrl = Modifiers::CTRL;
assert_eq!(keymap.feed(&mut pending, KeyStroke::new(Key::K, ctrl), 10.0), KeyResolution::AwaitSecond);
assert_eq!(
    keymap.feed(&mut pending, KeyStroke::new(Key::C, ctrl), 11.0),
    KeyResolution::Matched(vec!["Example.Comment".to_owned()]),
);
```

`KeymapUtil::shortcut_text(&shortcut, mac)` gives the text for menus and tooltips:
`Ctrl+Shift+A` and `Ctrl+K, Ctrl+C` on Windows and Linux, `⇧⌘A` on macOS (modifiers in the
order `⌃⌥⇧⌘`).

## Renderers

Four widgets draw the model, and all of them perform the action themselves (with `perform`, so
a disabled or hidden action never runs) and report what they did as data:

| Widget | Draws | Reports |
|---|---|---|
| [`ActionButton::for_action`](components/action-button.md) | One action as an icon button | `ActionButtonResponse::performed` |
| [`ActionToolbar`](components/action-toolbar.md) | A group as a toolbar, drop-downs, overflow | `ActionToolbarResponse::invoked` (the id) |
| [`ActionPopupMenu`](components/action-popup-menu.md) | A group as a popup menu with submenus | `ActionPopupMenuResponse::performed` (the id) |
| `PopupHandler::for_group` | The same menu as a context menu on any widget | performs only |

Menus and [list popups](components/list-popup.md) share one engine (keys, pointer rules and
timings of IntelliJ's `ListPopup`); the caller owns the open state (`PopupChain`).

## Differences from IntelliJ

Immediate mode changes four things, all for the same reason: the kit keeps no state.

- **`Presentation` is a value.** `update_presentation` computes a new one per frame from the
  template and `update`; there are no listeners. Keep `update` cheap, or cache in the host
  and invalidate when your data changes.
- **No update thread, no timer.** IntelliJ updates on a background thread and re-updates on a
  timer. Here `update` runs on the UI thread, in the frame that draws the action.
- **The context is explicit.** No parent-chain walk from the focused component: you build the
  `DataContext` (and chain it) from your own state.
- **No global services.** `ActionManager`, `KeymapManager`, `PropertiesComponent` and `Clock`
  are traits in `archipel_kit::host` that you pass in; the kit ships in-memory versions for
  tests and small apps.

Pending chord strokes, popup open flags and the like are likewise yours to keep: the kit
returns what happened (`KeyResolution`, `perform`'s `bool`) and you act on it.
