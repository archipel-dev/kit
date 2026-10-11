# IconButton

A transparent square button showing one icon, for toolbars and panel headers. It can act as
a toggle (filled with the accent color while selected) and take a semantic color, such as
green for Run.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/icon_button.dark.png">
  <img alt="Icon buttons: reload, run, stop, a selected pin and a disabled debug button" src="img/icon_button.light.png" width="194">
</picture>

## Usage

```rust
use archipel_kit::components::IconButton;
use archipel_kit::{Glyph, Icon, Theme, Tone, egui};

fn toolbar(ui: &mut egui::Ui, pinned: &mut bool) {
    let colors = Theme::colors(ui.ctx());
    ui.horizontal(|ui| {
        ui.add(IconButton::new(Glyph::Run, "Run").tone(Tone::Run));
        ui.add(IconButton::new(Glyph::Stop, "Stop").tone(Tone::Stop).enabled(false));
        if ui.add(IconButton::new(Glyph::Pin, "Pin tab").selected(*pinned)).clicked() {
            *pinned = !*pinned;
        }
    });
}
```

## API

| Builder | Effect |
|---|---|
| `IconButton::new(icon, label)` | The label is required: it is the tooltip and the accessible name. |
| `.selected(bool)` | Makes it a toggle; selected toggles are accent-filled. |
| `.tone(Tone)` | Meaning of the icon color, e.g. `Tone::Run` (default: the icon gray). |
| `.enabled(bool)` | Disabled buttons are dimmed and ignore clicks. |

## Accessibility

Role `Button`, named by its label. Toggles report their state, so screen readers announce
"Pin tab, toggle button, pressed".

Source: [`components/icon_button.rs`](../../crates/archipel-kit/src/components/icon_button.rs)
