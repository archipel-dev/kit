# Behaviours

A component draws itself. Interaction that is not part of its look is added on top with
`.with(...)`, and works the same on every component. The kit's own components and the ones
you build from [parts](building-components.md) all accept it.

```rust
use archipel_kit::behaviour::{ContextMenu, Cursor, WidgetExt};
use archipel_kit::components::{Button, IconButton, MenuItem, Tooltip};
use archipel_kit::{Glyph, egui};

fn toolbar(ui: &mut egui::Ui) {
    ui.add(
        IconButton::new(Glyph::Run, "Run")
            .with(Tooltip::new("Run 'app'").shortcut("Shift+F10"))
            .with(ContextMenu::new(|ui| {
                ui.add(MenuItem::new("Edit Configurations…"));
            })),
    );
    ui.add(Button::secondary("Split").with(Cursor(egui::CursorIcon::ResizeHorizontal)));
}
```

Widgets (`ui.add(...)`) get `with` from `behaviour::WidgetExt`. Components with a `show`
method, such as `ListRow`, have their own `with`. Behaviours are attached in order, after
the component has drawn, so each one sees the final response.

| Behaviour | What it does |
|---|---|
| `Tooltip::new(label)` | Kit tooltip after the hover delay, also over disabled widgets (say why). `.shortcut(...)`, `.hint(...)`. |
| `ContextMenu::new(\|ui\| …)` | Menu on secondary click, on the popup surface. Fill it with `MenuItem`s. |
| `Cursor(icon)` | The cursor over this widget, instead of the component's (splitters, handles). |
| `DragSource(payload)` | The widget can be dragged, carrying `payload`. Open hand on hover, closed while dragging; clicks still click. Doesn't add a Tab stop, so draggable rows in a list keep the list's arrow-key focus. |
| `DropTarget::new(&mut Option<P>)` | Outlines the widget while a `P` hovers it; stores the payload when it is dropped. |
| A closure `\|ui, response\| { …; response }` | Anything else, e.g. react to `double_clicked()`. |

## Drag and drop

```rust
use archipel_kit::behaviour::{DragSource, DropTarget};
use archipel_kit::components::ListRow;
use archipel_kit::{Glyph, egui};

fn move_files(ui: &mut egui::Ui, dropped_on_src: &mut Option<String>) {
    ListRow::new("main.rs").with(DragSource("main.rs".to_string())).show(ui);
    ListRow::new("src")
        .icon(Glyph::Folder)
        .with(DropTarget::new(dropped_on_src))
        .show(ui);
}
```

## Writing a behaviour

Implement `Behaviour` for a type when it is reused. A closure is enough for one place:

```rust
use archipel_kit::behaviour::Behaviour;
use archipel_kit::egui::{Response, Ui};

/// Opens the item on double click.
struct OpenOnDoubleClick<'a>(&'a mut bool);

impl Behaviour for OpenOnDoubleClick<'_> {
    fn attach(self, _ui: &mut Ui, response: Response) -> Response {
        if response.double_clicked() {
            *self.0 = true;
        }
        response
    }
}
```

A behaviour must not change how the component looks at rest. It may draw feedback on top
(the drop outline) and may add sense (`response.interact(Sense::drag())`). It must keep the
component's accessible name and role.

Source: [`behaviour.rs`](../crates/archipel-kit/src/behaviour.rs) ·
Tests: [`tests/composition.rs`](../crates/archipel-kit/tests/composition.rs)
