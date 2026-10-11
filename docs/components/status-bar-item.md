# StatusBarItem

A flat widget for the status bar or a toolbar: an optional icon, text and an optional chevron
for items that open a popup (the branch, the run configuration). It fills on hover and press
like a toolbar button, has the action cursor, and is a button named by its text. It takes at
most the available width (or `.max_width`): longer text ends with `…` and the full text is
the tooltip.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/status_bar_item.dark.png">
  <img alt="A toolbar with a run button and a configuration dropdown, and a status bar with branch, encoding, an error count, a disabled item and an elided one" src="img/status_bar_item.light.png" width="420">
</picture>

## Usage

Draw the bar on `Surface::Chrome`, the frame color of the workbench.

```rust
use archipel_kit::behaviour::WidgetExt;
use archipel_kit::components::{StatusBarItem, Tooltip};
use archipel_kit::parts::Surface;
use archipel_kit::{Glyph, Tone, egui};

fn status_bar(ui: &mut egui::Ui, branch: &str, errors: usize) {
    Surface::Chrome.frame(ui.ctx()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let item = StatusBarItem::new(branch).icon(Glyph::Branch).chevron(true);
            if ui.add(item.with(Tooltip::new("Git branch"))).clicked() {
                // open the branches popup
            }
            ui.add(StatusBarItem::new("UTF-8"));
            if errors > 0 {
                ui.add(StatusBarItem::new(format!("{errors} errors")).tone(Tone::Error));
            }
        });
    });
}
```

## API

| Builder | Effect |
|---|---|
| `StatusBarItem::new(text)` | A flat item; `ui.add(...)` returns the `Response`. |
| `.icon(Icon)` | Icon before the text. |
| `.chevron(bool)` | Chevron after the text: opens a popup. |
| `.tone(Tone)` | Meaning of the text and icon color. |
| `.enabled(bool)` | Disabled items are dimmed and ignore clicks. |
| `.max_width(f32)` | Widest the item may be; text is elided beyond it. |
| `.with(behaviour)` | A tooltip, context menu … (see [behaviours](../behaviours.md)). A tooltip you attach replaces the one for elided text. |

## Behavior

- Click, <kbd>Enter</kbd> and <kbd>Space</kbd> activate it; <kbd>Tab</kbd> reaches it.
- Hover fills it; the cursor is the theme's action cursor (the arrow when disabled).

## Accessibility

A `Button` named by its text, reporting disabled.

Source: [`components/status_bar.rs`](../../crates/archipel-kit/src/components/status_bar.rs) ·
Tests: [`tests/status_bar.rs`](../../crates/archipel-kit/tests/status_bar.rs)
