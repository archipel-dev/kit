# TabBar

Editor tabs as in Android Studio's Islands UI. The selected tab is a tinted pill with a 1 pt
border; tabs can show a file icon, an unsaved-changes dot and a close button.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/tab_bar.dark.png">
  <img alt="Three editor tabs: main.rs selected, Cargo.toml modified, README.md" src="img/tab_bar.light.png" width="437">
</picture>

## Usage

```rust
use archipel_kit::components::{Tab, TabBar};
use archipel_kit::{Icon, egui};

fn editor_tabs(ui: &mut egui::Ui, files: &mut Vec<String>, current: &mut usize) {
    let tabs: Vec<Tab> = files
        .iter()
        .map(|name| Tab::new(name.as_str()).icon(Icon::for_file(name)).closable(true))
        .collect();
    let out = TabBar::new(&tabs, *current).show(ui);
    if let Some(index) = out.selected {
        *current = index;
    }
    if let Some(index) = out.closed {
        files.remove(index);
        *current = (*current).min(files.len().saturating_sub(1));
    }
}
```

## API

| Item | Effect |
|---|---|
| `Tab::new(label)` | A tab. `.icon(Icon)`, `.closable(bool)`, `.modified(bool)`. |
| `TabBar::new(&tabs, selected).show(ui)` | Returns `TabBarResponse { selected, closed }`. |

The caller owns the tab list and the selection; `show` only reports what the user did.

## Behavior

- Clicking a tab selects it; middle-clicking a closable tab closes it.
- The close button shows on the selected tab and on hover. Its room is kept on every
  closable tab, so tabs never move when it appears; hovering the cross shows a round
  highlight behind it.
- Hovering an unselected tab shows a hover pill and brings its dimmed title (`tab_text`) up
  to full strength.

Geometry (`tokens::tab`, measured on Android Studio): the bar is `HEADER_HEIGHT` (40 pt);
pills are 28 pt high, radius 6, 6 pt apart; inside a pill: 8 pt, icon, 4 pt, title, 4 pt,
close (16 pt), 4 pt. Put the bar at the island's top, 6 pt from its left edge.

## Accessibility

Tabs are selectable items named by their label that report whether they are selected; close
buttons are named "Close <label>".

Source: [`components/segmented.rs`](../../crates/archipel-kit/src/components/segmented.rs) ·
Tests: [`tests/navigation.rs`](../../crates/archipel-kit/tests/navigation.rs)
