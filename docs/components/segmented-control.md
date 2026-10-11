# SegmentedControl

Connected buttons of which exactly one is selected, for switching between views or modes of
the same content (*Pretty | Raw | Preview*, *Body | Headers | Auth*).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/segmented_control.dark.png">
  <img alt="A segmented control with Pretty selected and a disabled one" src="img/segmented_control.light.png" width="227">
</picture>

## Usage

```rust
use archipel_kit::components::SegmentedControl;
use archipel_kit::egui;

#[derive(PartialEq, Clone, Copy)]
enum View { Pretty, Raw, Preview }

fn view_switch(ui: &mut egui::Ui, view: &mut View) {
    ui.add(SegmentedControl::new(
        view,
        [(View::Pretty, "Pretty"), (View::Raw, "Raw"), (View::Preview, "Preview")],
    ));
}
```

## API

| Builder | Effect |
|---|---|
| `SegmentedControl::new(&mut T, [(T, label)])` | Options as value/label pairs. |
| `.enabled(bool)` | Disabled controls are dimmed and ignore clicks. |

The returned `Response` is `changed()` when the selection changed.

## Accessibility

Each segment is a `RadioButton` named by its label and reports whether it is selected.

Source: [`components/segmented.rs`](../../crates/archipel-kit/src/components/segmented.rs) ·
Tests: [`tests/navigation.rs`](../../crates/archipel-kit/tests/navigation.rs)
