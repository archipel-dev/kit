# Dialog

A **dialog** asks for a decision or a few inputs before work continues. It has a title, a
message or custom content, and buttons at the bottom right. The rest of the window is dimmed
and blocked while it is open.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/dialog.dark.png">
  <img alt="A dialog asking to delete main.rs, with Cancel and a red Delete button" src="img/dialog.light.png" width="520">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/dialog_form.dark.png">
  <img alt="A New Branch dialog with a branch name field, Cancel and Create" src="img/dialog_form.light.png" width="520">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/dialog_short.dark.png">
  <img alt="A short Select files dialog that is only as tall as its two rows" src="img/dialog_short.light.png" width="520">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/dialog_scroll.dark.png">
  <img alt="A Select files dialog with a long list: the rows scroll, the title and buttons stay" src="img/dialog_scroll.light.png" width="520">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/dialog_tertiary.dark.png">
  <img alt="A Select files dialog with Reset at the left edge and Cancel and Open at the right" src="img/dialog_tertiary.light.png" width="520">
</picture>

## Usage

The dialog is stateless. Show it while your `open` flag is set, and clear the flag when it
reports an action.

```rust
use archipel_kit::components::{Dialog, DialogAction, TextField};
use archipel_kit::egui;

struct NewBranch {
    open: bool,
    name: String,
}

fn new_branch(ctx: &egui::Context, state: &mut NewBranch) -> Option<String> {
    if !state.open {
        return None;
    }
    let valid = !state.name.trim().is_empty();
    let action = Dialog::new("New Branch")
        .content(|ui| {
            ui.add(TextField::new(&mut state.name).label("Branch name"));
        })
        .primary("Create")
        .primary_enabled(valid)
        .secondary("Cancel")
        .width(360.0)
        .show(ctx);
    match action {
        Some(DialogAction::Primary) => {
            state.open = false;
            Some(state.name.clone())
        }
        Some(_) => {
            state.open = false;
            None
        }
        None => None,
    }
}
```

## API

| Builder | Default | Effect |
|---|---|---|
| `Dialog::new(title)` | | Title in semibold. |
| `.message(str)` | none | Wrapped text below the title. |
| `.icon(Glyph)` | none | Icon before the title (`Glyph::Warning` for destructive questions). |
| `.content(FnOnce(&mut Ui))` | none | Fields or lists below the message. |
| `.primary(label)` | none | The rightmost, accent button. |
| `.secondary(label)` | none | The button left of it. |
| `.tertiary(label)` | none | A secondary-style button at the left edge (e.g. "Reset"); reports `Tertiary`. |
| `.danger(bool)` | `false` | Makes the primary button red. |
| `.primary_enabled(bool)` | `true` | Disables the primary button and <kbd>Enter</kbd>. |
| `.width(f32)` | 400 | Width in points. |
| `.max_height(f32)` | 80% of the window | Tallest the dialog grows; beyond it the message and content scroll. |
| `.show(ctx)` | | Returns `Option<DialogAction>`: `Primary`, `Secondary`, `Tertiary` or `Dismissed`. |

## Behavior

- <kbd>Enter</kbd> picks the primary button when it is enabled, also from a single-line
  field. In a multiline field <kbd>Enter</kbd> is a new line: <kbd>Ctrl</kbd>+<kbd>Enter</kbd>
  (<kbd>Cmd</kbd>+<kbd>Enter</kbd> on macOS) picks the primary button there. A focused
  button keeps <kbd>Enter</kbd> for itself.
- <kbd>Escape</kbd> and a click outside the dialog dismiss it. <kbd>Escape</kbd> inside an
  open [dropdown](dropdown.md) only closes its popup.
- The dialog is as tall as its content, up to the max height. The title and the buttons
  stay in view; the message and content scroll.
- Name buttons by what they do ("Delete", "Create"), not "OK". For destructive actions, use
  `.danger(true)`, a warning icon and a message that says what happens.
- One dialog at a time: a dialog that needs another decision replaces its content instead
  of stacking.

## Accessibility

The title is a label and the buttons are buttons named by their text, so tests can use
`get_by_label("Delete").click()`. Keyboard focus stays inside the dialog while it is open.

Source: [`components/overlay.rs`](../../crates/archipel-kit/src/components/overlay.rs) ·
Tests: [`tests/overlays.rs`](../../crates/archipel-kit/tests/overlays.rs)
