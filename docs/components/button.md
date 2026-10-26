# Button

A push button with a text label and an optional leading icon. Use one **primary** button for
the default action of a dialog or form; make everything else **secondary**, **ghost** inside
dense toolbars and banners, and **danger** for destructive actions.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/button.dark.png">
  <img alt="Primary, secondary, ghost and danger buttons, enabled and disabled" src="img/button.light.png" width="347">
</picture>

## Usage

```rust
use archipel_kit::ui::Button;
use archipel_kit::{Glyph, Icon, egui};

fn dialog_buttons(ui: &mut egui::Ui, can_create: bool) -> bool {
    let mut created = false;
    ui.horizontal(|ui| {
        created = ui.add(Button::primary("Create").enabled(can_create)).clicked();
        ui.add(Button::secondary("Cancel"));
        ui.add(Button::secondary("Run").icon(Glyph::Run));
    });
    created
}
```

## API

| Builder | Effect |
|---|---|
| `Button::primary(label)` | Accent fill: the one default action. |
| `Button::secondary(label)` | Outlined. |
| `Button::ghost(label)` | No outline until hovered. |
| `Button::danger(label)` | Error fill, for Delete and other destructive actions. |
| `Button::new(label, ButtonVariant)` | The same, with the variant as a value. |
| `.icon(Icon)` | Icon before the label. |
| `.enabled(bool)` | Disabled buttons are dimmed and ignore clicks. A disabled primary or danger button keeps a label of at least 3:1 contrast (`text_on_accent_disabled` on `accent_disabled`). |
| `.small(bool)` | 24 pt instead of 28 pt tall. |
| `.min_width(f32)` | Minimum width, 72 pt by default so "OK" does not look cramped. |

Returns an `egui::Response`: `clicked()`, `hovered()`, `on_hover_text(...)`.

## Behavior

IntelliJ spec section 13.2 (`DarculaButtonUI`):

| Input | Result |
|---|---|
| Hover | No change (primary, secondary, danger): the spec has no hover paint for push buttons, and Android Studio measured none. Ghost buttons, for toolbars, do show a hover fill. |
| Press | Primary and danger darken (12%), secondary fills with the pressed overlay. The spec says only that pressing follows Swing's model; the amount is the kit's own. |
| <kbd>Tab</kbd> | Focuses the button: the 1 pt border becomes a 2 pt focus-colored border, one of them outside; a filled button keeps a 1 pt gap inside its ring. |
| <kbd>Enter</kbd> | Clicks the focused button at once. |
| <kbd>Space</kbd> | Clicks it when released (a press alone does nothing). |
| <kbd>Alt</kbd>+letter | `.mnemonic(letter)`: clicks an enabled button; the first matching letter is underlined. |
| Disabled | Dimmed text and border; ignores clicks, keys and mnemonics. |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/inputs_states.dark.png">
  <img alt="Buttons (primary, one with a mnemonic, disabled), spinners, a combo box and a three-state checkbox" src="img/inputs_states.light.png" width="300">
</picture>

Minimum size 72 x 28 (`Button.minimumSize`; `.small(true)` 24 high), horizontal padding 14, corner arc 8
(radius 4). The loading state's text color and the split button (`Button.Split.*`) are not provided.

## Accessibility

Role `Button`, named by its label, with the disabled state reported.

## Testing

```rust
use archipel_kit::ui::Button;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::Theme;

let mut harness = testing::harness_state(Theme::dark(), 0, |ui, clicks| {
    if ui.add(Button::primary("Run")).clicked() {
        *clicks += 1;
    }
});
harness.get_by_label("Run").click();
harness.run();
assert_eq!(*harness.state(), 1);
```

Source: [`ui/button.rs`](../../crates/archipel-kit/src/ui/button.rs) ·
Tests: [`tests/button.rs`](../../crates/archipel-kit/tests/button.rs), [`tests/inputs_behavior.rs`](../../crates/archipel-kit/tests/inputs_behavior.rs)
