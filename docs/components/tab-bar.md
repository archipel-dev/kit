# TabBar

Editor tabs as in Android Studio's Islands UI. The selected tab is a tinted pill with a 1 pt
border; tabs can show a file icon, an unsaved-changes dot and a close button.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/tab_bar.dark.png">
  <img alt="Three editor tabs at the top of an island: main.rs selected, Cargo.toml modified, README.md" src="img/tab_bar.light.png" width="460">
</picture>

When the tabs don't fit, the strip scrolls sideways. With the pointer over it, a thin scroll
bar shows at its top edge; "Show hidden tabs" (the chevron) and "More actions" (⋮) sit at
its right end:

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/tab_bar_overflow.dark.png">
  <img alt="An overflowing tab strip under the pointer: a thin scroll bar along its top edge, theme.rs selected, a chevron and a three-dot button at the right end" src="img/tab_bar_overflow.light.png" width="380">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/tab_bar_hidden_tabs.dark.png">
  <img alt="The hidden tabs list open below the chevron: tokens.rs, Cargo.toml, README.md, segmented.rs and navigation.rs with their file icons" src="img/tab_bar_hidden_tabs.light.png" width="380">
</picture>

## Usage

```rust
use archipel_kit::components::{MenuItem, Tab, TabBar};
use archipel_kit::{Icon, egui};

fn editor_tabs(ui: &mut egui::Ui, files: &mut Vec<String>, current: &mut usize) {
    let tabs: Vec<Tab> = files
        .iter()
        .map(|name| Tab::new(name.as_str()).icon(Icon::for_file(name)).closable(true))
        .collect();
    let mut close_all = false;
    let out = TabBar::new(&tabs, *current)
        .more_menu(|ui| {
            if ui.add(MenuItem::new("Close All Tabs")).clicked() {
                close_all = true;
            }
        })
        .show(ui);
    if let Some(index) = out.selected {
        *current = index;
    }
    if let Some(index) = out.closed {
        files.remove(index);
        *current = (*current).min(files.len().saturating_sub(1));
    }
    if close_all {
        files.clear();
        *current = 0;
    }
}
```

## API

| Item | Effect |
|---|---|
| `Tab::new(label)` | A tab. `.icon(Icon)`, `.closable(bool)`, `.modified(bool)`. |
| `TabBar::new(&tabs, selected).show(ui)` | Draws the strip as wide as the `Ui`. Returns `TabBarResponse { selected, closed, hidden_tabs_button, more_button }`. |
| `.more_menu(\|ui\| …)` | A "More actions" button (⋮) at the right end, opening a menu of the app's [`MenuItem`](menu-item.md)s. A clicked item closes it. |
| `.separator(bool)` | The 1 pt line under the strip (on by default). |
| `.id_salt(salt)` | Tells two tab bars in one `Ui` apart (each remembers its scroll position). |

The caller owns the tab list and the selection; `show` only reports what the user did. The
scroll position and the open popups are kept in egui's memory, like a scroll area's.

## Behavior

- Clicking a tab selects it; middle-clicking a closable tab closes it.
- The close button shows on the selected tab and on hover. Its room is kept on every
  closable tab, so tabs never move when it appears; hovering the cross shows a round
  highlight behind it.
- Hovering an unselected tab shows a hover pill and brings its dimmed title (`tab_text`) up
  to full strength.

- Tabs that don't fit scroll sideways: the mouse wheel scrolls the strip (a vertical wheel
  too), and so do <kbd>Shift</kbd>+wheel and a trackpad's sideways swipe. Tabs are cut off
  at the ends of the scrolling area and fade out over 10 pt there; the strip never changes
  height.
- While the pointer is over an overflowing strip (and 0.2 s after it leaves or the wheel
  stops, then fading out), a thin scroll bar shows along the strip's top edge: a 2 pt thumb,
  1 pt from the edge, 4 pt thick under the pointer. Drag it to scroll.
- The selected tab is scrolled into view when it changes (a click, a shortcut, a new tab),
  and so is a tab that gets keyboard focus.
- **Show hidden tabs** (a chevron) appears while tabs don't fit. It lists the tabs that are
  not wholly in view, with their icons; the list opens on its first row, the arrow keys move
  through it, <kbd>Enter</kbd> or a click picks a tab, which is selected (reported in
  `selected`) and scrolled into view. <kbd>Escape</kbd> closes the list and gives the focus
  back to the button.
- **More actions** (⋮), with `more_menu`, opens the app's menu below it.

Geometry (`tokens::tab`, measured on Android Studio): the bar is `HEADER_HEIGHT` (40 pt) and
as wide as the island; the first pill starts `STRIP_INSET` (6 pt) from the island's left
edge and the inset scrolls away with the tabs. Pills are 28 pt high, radius 6, 6 pt apart;
inside a pill: 8 pt, icon, 4 pt, title, 4 pt, close (16 pt), 4 pt. The buttons are 22 pt,
4 pt apart, the last 4 pt from the island's right edge; the scrolling area ends 9 pt before
the first button. Put the bar at the island's top, from edge to edge.

## Accessibility

Tabs are selectable items named by their label that report whether they are selected, in a
tab list whose bounds are the scrolling area (tabs out of view are cut off there); close
buttons are named "Close <label>". The buttons are named "Show hidden tabs" and "More
actions" and pass the kit's conformance test; the hidden tabs list is a menu of items named
like their tabs. The scroll bar's thumb is a pointer target only, like any scroll bar: the
keyboard reaches hidden tabs through Tab and the hidden tabs list.

Source: [`components/segmented.rs`](../../crates/archipel-kit/src/components/segmented.rs) ·
Tests: [`tests/navigation.rs`](../../crates/archipel-kit/tests/navigation.rs),
[`tests/conformance.rs`](../../crates/archipel-kit/tests/conformance.rs)
