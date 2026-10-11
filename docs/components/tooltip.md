# Tooltip

A **tooltip** names what a control does, after a short hover. It can show a shortcut after
the label and a longer hint below. Tooltips also appear on disabled controls, where they say
why the control is disabled.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/tooltip.dark.png">
  <img alt="Two tooltips: Run 'gallery' with its shortcut, and Search Everywhere with a hint" src="img/tooltip.light.png" width="318">
</picture>

## Usage

```rust
use archipel_kit::components::{Button, IconButton, Tooltip, TooltipExt};
use archipel_kit::{Glyph, egui};

fn toolbar(ui: &mut egui::Ui, connected: bool) {
    ui.add(IconButton::new(Glyph::Run, "Run"))
        .tooltip_with(Tooltip::new("Run 'gallery'").shortcut("Shift+F10"));
    ui.add(Button::primary("Deploy").enabled(connected))
        .tooltip("Connect a cluster first");
}
```

## API

| Item | Use |
|---|---|
| `response.tooltip(label)` | A plain tooltip (`TooltipExt`, implemented for `egui::Response`). |
| `response.tooltip_with(Tooltip)` | A tooltip with a shortcut or hint. |
| `Tooltip::new(label)` | `.shortcut(str)`, `.hint(str)` |
| `Tooltip::frame(ctx)`, `tooltip.content(ui)` | The frame and content, to show a tooltip outside of hover (onboarding, docs). |

## Behavior

- Tooltips appear after egui's hover delay (`style.interaction.tooltip_delay`). Once one is
  open, moving to a neighbor shows its tooltip right away.
- Tooltips use the theme's tooltip color, the popup border and a control radius, and stay
  narrow: hints wrap at 280 points.
- Write the label as the action ("Run 'gallery'"), not the control ("Run button"). Put the
  shortcut in `.shortcut`, not in the label.

## Accessibility

The label is an accessible label in the tooltip, so tests find it with
`get_by_label` once it shows. Icon-only buttons still need their own accessible name
(`IconButton::new(glyph, name)`): a tooltip doesn't replace it.

Source: [`components/overlay.rs`](../../crates/archipel-kit/src/components/overlay.rs) ·
Tests: [`tests/overlays.rs`](../../crates/archipel-kit/tests/overlays.rs)
