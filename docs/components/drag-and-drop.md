# DragSource, DropZone

Drag and drop: the user picks up a `DragSource` (a field in a list, a value in a table) and
drops it on a `DropZone` that takes its payload type (the Query, Projection and Sort areas of a
query builder, a folder in a tree).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/drag_and_drop.dark.png">
  <img alt="The price row dragged over a drop zone: the row dimmed in place, a copy on a card under the pointer, the zone outlined in the focus color; another zone that takes something else stays plain" src="img/drag_and_drop.light.png" width="320">
</picture>

## Usage

```rust
use archipel_kit::components::{DragSource, DropZone, ListRow};
use archipel_kit::egui;

#[derive(Clone)]
struct Field(String);

fn builder(ui: &mut egui::Ui, fields: &[String], filter: &mut Vec<String>) {
    for name in fields {
        let row = DragSource::new(name, Field(name.clone()))
            .show(ui, |ui| ListRow::new(name.as_str()).show(ui))
            .inner;
        // The keyboard way to the same result.
        if row.response.double_clicked() {
            filter.push(name.clone());
        }
    }
    let zone = DropZone::new("filter")
        .hint("Drop fields here to filter")
        .show_hint(filter.is_empty())
        .show::<Field, _>(ui, |ui| {
            for name in filter.iter() {
                ui.label(name.as_str());
            }
        });
    if let Some(field) = zone.dropped {
        filter.push(field.0.clone());
    }
}
```

## API

| Builder | Effect |
|---|---|
| `DragSource::new(id_salt, payload)` | Content the user can drag, carrying `payload` (any `Send + Sync + 'static` type). |
| `.show(ui, \|ui\| …)` | Draws the content; called twice per frame while dragging, so it must not change state. Returns an `InnerResponse` whose response reports `drag_started()`, `dragged()`, `drag_stopped()`. |
| `DropZone::new(id_salt)` | A target for payloads of the type given to `show`. |
| `.hint(str)` | Gray text centered in the zone while it holds nothing; also its accessible name. |
| `.show_hint(bool)` | Hide the hint once the zone shows content. |
| `.show::<P, _>(ui, \|ui\| …)` | Lays the zone out around the content; returns a `DropZoneResponse` with `inner`, `response`, `hovered` (a matching payload is over it) and `dropped` (`Option<Arc<P>>`, the frame it is released). |

## Behavior

- The source shows a grab cursor on hover and a grabbing one while dragged. Its content stays
  in place at 40 % opacity; a copy on a popup card follows the pointer at 85 %, held where it
  was grabbed.
- Clicks still reach the content: a `ListRow` inside a source stays selectable and
  double-clickable.
- While a matching payload is dragged, every zone that takes it shows a 1 pt focus-color
  outline; the zone under the pointer a 2 pt focus ring and the inactive selection tint. Zones
  for other payload types stay plain, and a release over them drops nothing.
- Zones are at least 40 pt tall with 8 pt padding and the control radius.

## Accessibility

Dragging is a pointer gesture. An app that offers it also offers a keyboard way to the same
result: a double-click or <kbd>Enter</kbd> on the row, or a menu item. The zone is exposed with
its hint as its name; the source keeps its content's role and name.

Source: [`components/drag.rs`](../../crates/archipel-kit/src/components/drag.rs) ·
Tests: [`tests/drag.rs`](../../crates/archipel-kit/tests/drag.rs)
