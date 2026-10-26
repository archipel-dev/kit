# SimpleToolWindowPanel, TitledSeparator, SeparatorComponent, PropertyGrid

Layout building blocks. A **panel** is an island with a title bar: the unit of a workbench
layout (tool windows, sidebars, inspectors). **Section headers** group content inside it;
**property grids** show key/value details.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/simple_tool_window_panel.dark.png">
  <img alt="A Response panel with copy and more actions, a General section and a property grid" src="img/simple_tool_window_panel.light.png" width="360">
</picture>

## Usage

```rust
use archipel_kit::{action_system::ActionButton, wm::SimpleToolWindowPanel, ui::{PropertyGrid, TitledSeparator}};
use archipel_kit::{Glyph, egui};

fn response_panel(ui: &mut egui::Ui) {
    SimpleToolWindowPanel::new("Response")
        .actions(|ui| {
            // Laid out right to left: the first action is the rightmost.
            ui.add(ActionButton::new(Glyph::More, "More"));
            ui.add(ActionButton::new(Glyph::Copy, "Copy"));
        })
        .show(ui, |ui| {
            ui.add(TitledSeparator::new("General"));
            PropertyGrid::new("response")
                .row("Status", "200 OK")
                .mono_row("Content-Type", "application/json")
                .show(ui);
        });
}
```

### Tool window header

`.on_hide(|| …)` adds the tool window's hide button at the right end of the title bar (named
"Hide <title>", tooltip "Hide"). With `.actions(...)` the title bar is the IntelliJ-style
header: title, action icon buttons, hide. Panels without `on_hide` are unchanged.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/simple_tool_window_panel_tool_window.dark.png">
  <img alt="A Project tool window with find and collapse-all actions and a hide button" src="img/simple_tool_window_panel_tool_window.light.png" width="360">
</picture>

```rust
use archipel_kit::{action_system::ActionButton, wm::SimpleToolWindowPanel};
use archipel_kit::{Glyph, egui};

fn project_window(ui: &mut egui::Ui, visible: &mut bool) {
    SimpleToolWindowPanel::new("Project")
        .actions(|ui| {
            ui.add(ActionButton::new(Glyph::CollapseAll, "Collapse all"));
            ui.add(ActionButton::new(Glyph::Search, "Find"));
        })
        .on_hide(|| *visible = false)
        .show(ui, |ui| {
            ui.label("src/");
        });
}
```

The kit has one container with a title bar, so the header is part of `SimpleToolWindowPanel` rather than a
second component that would paint the same title and toolbar.

### Header actions from an `ActionGroup`, the options menu and the header's mouse

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/tool_window_header.dark.png">
  <img alt="A Problems tool window: bold title, File and Project Errors tabs, then rerun, filter, options and hide buttons" src="img/tool_window_header.light.png" width="420">
</picture>

```rust
# egui::__run_test_ui(|ui| {
use archipel_kit::action_system::{ActionGroup, DataContext, DefaultActionGroup};
use archipel_kit::wm::SimpleToolWindowPanel;

let title = DefaultActionGroup::new("Title actions"); // add your actions
let options = DefaultActionGroup::popup("Options");
let context = DataContext::empty();
SimpleToolWindowPanel::new("Problems")
    .title_actions(&title as &dyn ActionGroup)
    .popup_group(&options as &dyn ActionGroup)
    .data_context(&context)
    .on_activate(|| { /* move the focus into the tool window */ })
    .on_toggle_maximize(|| { /* maximize or restore */ })
    .on_hide(|| { /* hide it */ })
    .show(ui, |ui| { ui.label("No problems"); });
# });
```

| Builder | Effect (IntelliJ spec section 8.1) |
|---|---|
| `.title_actions(&dyn ActionGroup)` | The group's visible actions as buttons in the group's order (place `TOOLWINDOW_TITLE`), left of `.actions(...)`, under the same show-on-hover rules. A click performs the action. |
| `.popup_group(&dyn ActionGroup)` | An "Options" gear before Hide opens the group below it (place `TOOLWINDOW_POPUP`), and so does a right click on the header; the gear toggles it. |
| `.on_activate(f)` | A left click on the header (not on its buttons or tabs). |
| `.on_toggle_maximize(f)` | A double click on the header. |
| `.on_hide(f)` | The Hide button, a middle click, or Shift + left click on the header. |
| `.on_popup(f)` | A right click, when there is no `popup_group`: receives the pointer position. |
| `.tab_closable(&[bool])`, `.on_close_tab(f)` | Content tabs the user may close with a middle or Shift + left click; `f` gets the index. |
| `.data_context(&DataContext)` | What the group and the menu update and run with. |

The header's area under its buttons is not a Tab stop and has no accessible node; the
buttons and tabs are the keyboard's way in.

## API

| Component | API |
|---|---|
| `SimpleToolWindowPanel::new(title)` | `.actions(|ui| …)` toolbar on the right of the title (right to left), `.on_hide(\|\| …)` hide button, `.show(ui, |ui| body)`. |
| `TitledSeparator::new(title)` | Group title with a line to the right edge, 20 pt above it (none at the top of a `Ui`). In settings pages use [`Panel::group`](dsl.md). `.collapsible(&mut bool)` adds a chevron that shows and hides the group ([`Panel::collapsible_group`](dsl.md)). |
| `SeparatorComponent` | 1 pt line across the available width, or a vertical one in a horizontal layout. |
| `PropertyGrid::new(id)` | `.row(key, value)`, `.mono_row(key, value)`, `.show(ui)`. |

Panels sit on the frame color with a 4 pt gap between them (`tokens::size::ISLAND_GAP`); see
the [gallery](../../crates/archipel-kit-gallery/src/main.rs) for a full workbench layout.

Source: [`wm/simple_tool_window_panel.rs`](../../crates/archipel-kit/src/wm/simple_tool_window_panel.rs) ·
Tests: [`tests/data.rs`](../../crates/archipel-kit/tests/data.rs),
[`tests/tool_windows.rs`](../../crates/archipel-kit/tests/tool_windows.rs)

## Composable island headers

`SimpleToolWindowPanel::header(width, content)` replaces the painted title with a bounded left slot.
`padded(true)` applies the shared content inset; trees can retain edge-to-edge bodies.
`header_separator(true)` adds a hairline below the header, edge to edge in the frame color (as under Android Studio's Problems header). Header actions can
contain `SeparatorComponent::group()` between groups (a 20 pt line, as in Android Studio).

`hover_actions(true)` shows the actions only while the pointer is anywhere over the island
(header or body) or keyboard focus is inside it, like an Android Studio tool window. The
timing was measured in Android Studio (Rabbit, Islands UI) at 60 fps:

- **Show:** at once, no fade-in.
- **Hide:** after `tokens::motion::ISLAND_ACTIONS_HIDE_DELAY` (200 ms), a 70 ms fade
  (`ISLAND_ACTIONS_FADE_OUT`). Crossing a gap or a scroll bar does not flicker them.
- **Narrow islands:** hidden actions take no space, so the title gets the whole header.
  Shown actions win: a title (or `header` slot) that no longer fits beside them is hidden
  whole, not cut, and the right-anchored actions are clipped at the island's left edge.
  The title returns when they hide.

Hidden actions keep their accessible names and Tab order; focusing one reveals them.
Always-visible actions remain the default for compatibility.

```rust
# egui::__run_test_ui(|ui| {
use archipel_kit::{Glyph, wm::SimpleToolWindowPanel, action_system::ActionButton, ui::SeparatorComponent};
SimpleToolWindowPanel::new("Inspector")
    .header(180.0, |ui| { ui.label("Inspector · JSON"); })
    .padded(true).header_separator(true).hover_actions(true)
    .actions(|ui| {
        ui.add(ActionButton::new(Glyph::Refresh, "Reload inspector"));
        ui.add(SeparatorComponent::group());
        ui.add(ActionButton::new(Glyph::Search, "Find in inspector"));
    })
    .show(ui, |ui| { ui.label("Properties"); });
# });
```

![Composed island dark](img/island_header.dark.png)
![Composed island light](img/island_header.light.png)

## Content tabs

`SimpleToolWindowPanel::tabs(&tabs, &mut selected)` puts content tabs after the title, like Android Studio's
Problems tool window ("File", "Project Errors"), and draws the header hairline. Measured
there: 22 pt after the title, 10 pt inside each tab, 10 pt between tabs, 28 pt pills. Tabs
that don't fit before the header actions go behind a "More tabs" chevron (the selected one
stays in the header). The
selected tab is blue while the tool window is active (keyboard focus inside it) and outlined
gray otherwise; the others show a hover fill.

```rust
# egui::__run_test_ui(|ui| {
use archipel_kit::wm::SimpleToolWindowPanel;

let tabs = ["File".to_owned(), "Project Errors".to_owned()];
let mut selected = 0;
SimpleToolWindowPanel::new("Problems")
    .tabs(&tabs, &mut selected)
    .show(ui, |ui| { ui.label("No problems in main.rs"); });
# });
```

## View switcher

`SimpleToolWindowPanel::views(&views, &mut selected)` makes the title a view switcher, like Android Studio's
"Project ▾": the title reads the selected view with a chevron; a click or <kbd>Enter</kbd>
opens the list right below the header, as wide as its longest name, with the current view
highlighted. The title has no hover fill, as in Android Studio.

```rust
# egui::__run_test_ui(|ui| {
use archipel_kit::wm::SimpleToolWindowPanel;

let views = ["Project".to_owned(), "Packages".to_owned(), "Tests".to_owned()];
let mut selected = 0;
SimpleToolWindowPanel::new("Project")
    .views(&views, &mut selected)
    .show(ui, |ui| { ui.label("src/"); });
# });
```
