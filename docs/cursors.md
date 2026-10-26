# Cursors

Every kit component decides its own mouse cursor. Cursors nest the way you'd expect: the
**innermost** widget under the pointer decides, unless a parent explicitly **overrides** it
for everything inside.

## What the components show

| Component | Cursor |
|---|---|
| Buttons, icon buttons, checkboxes, radios, toggles, segments, dropdowns, close buttons, sortable column headers | The theme's `action_cursor`: the pointing hand by default |
| Reorderable table headers | Open hand on hover, closed hand while dragging (`DragSource`) |
| List rows, tree rows, table rows, tabs, menu items | Default arrow |
| Links | Pointing hand |
| Text fields, inline cell editors | Text beam; default over the clear button |
| Split pane dividers | `ResizeHorizontal` (side by side) or `ResizeVertical` (stacked), also while dragging |
| Disabled controls | Default arrow |
| Anything with `.with(Cursor(icon))` | `icon` |

## How it works

In egui the last cursor set during a frame wins, and children draw after their parent. So
when each component sets its cursor on hover (`cursor::on_hover`), the innermost one wins
without any extra machinery.

Two helpers let a parent take part:

- `cursor::override_in(ui, Some(icon), |ui| …)` sets `icon` **after** the children draw, so
  it wins over all of them. Use it for a busy panel (`CursorIcon::Progress`) or during a
  drag (`Grabbing`). Pass `None` to switch the override off by state.
- `cursor::fallback_in(ui, icon, |ui| …)` sets `icon` **before** the children draw, so any
  child that sets a cursor wins. Use it for the empty space of a canvas or a drag-handle
  area.

```rust
use archipel_kit::ui::{Button, ActionLink};
use archipel_kit::{cursor, egui::{self, CursorIcon}};

fn panel(ui: &mut egui::Ui, loading: bool) {
    cursor::override_in(ui, loading.then_some(CursorIcon::Progress), |ui| {
        cursor::fallback_in(ui, CursorIcon::Grab, |ui| {
            ui.add(ActionLink::new("Configure…")); // hand, unless loading
            ui.add(Button::primary("Run")); // hand, unless loading
            ui.add_space(40.0); // grab, unless loading
        });
    });
}
```

In your own widgets, call `cursor::on_hover(&response, icon)` instead of setting the cursor
unconditionally, so parents can still override it. In tests, `cursor::current(ctx)` reads
the cursor at the end of your UI code.

Source: [`cursor.rs`](../crates/archipel-kit/src/cursor.rs) ·
Tests: [`tests/overlays.rs`](../crates/archipel-kit/tests/overlays.rs)
