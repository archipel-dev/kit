# JBTextField, JBIntSpinner

A single-line or multi-line text input with an optional label, placeholder, leading icon, clear button and
validation message. `JBTextField::search` is the preset for filter and search boxes.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/jb_text_field.dark.png">
  <img alt="Text fields: labeled with placeholder, search with clear button, invalid with an error message, disabled monospace" src="img/jb_text_field.light.png" width="360">
</picture>

## Usage

```rust
use archipel_kit::ui::JBTextField;
use archipel_kit::egui;

fn connection_form(ui: &mut egui::Ui, host: &mut String, port: &mut String) {
    ui.add(JBTextField::new(host).label("Host").placeholder("localhost"));
    let error = port.parse::<u16>().is_err().then_some("Enter a number from 0 to 65535");
    ui.add(JBTextField::new(port).label("Port").error(error));
}
```

## API

| Builder | Effect |
|---|---|
| `JBTextField::new(&mut String)` | Edits the string in place. `.id(Id)` sets the text edit's id (like `egui::TextEdit::id`): `response.id == id`, so `ctx.memory_mut(\|m\| m.request_focus(id))` focuses the field. |
| `JBTextField::search(&mut String)` | Magnifier icon, "Search" placeholder, clear button. |
| `.label(str)` | Visible label above the input, also its accessible name. |
| `.name(str)` | Accessible name without a visible label; defaults to the label or placeholder. |
| `.multiline(rows)` | At least two visible lines, wrapping and internal scrolling; Enter inserts a newline. |
| `.placeholder(str)` | Hint shown while empty. |
| `.error(Option<str>)` | Error outline and message below; `None` clears it, so `.error(validate(&text))` works. |
| `.leading_icon(Icon)` | Icon inside the input, before the text. |
| `.clearable(bool)` | Clear button while the input has text. |
| `.password(bool)` | Masks the text. |
| `.monospace(bool)` | Code font, for paths, URLs and ids. |
| `.enabled(bool)` | Disabled inputs are dimmed and read-only. |
| `.highlighter(&dyn SyntaxHighlighter)` | Colors the text (URLs, `{{variables}}`); see [single-line fields](code.md#single-line-fields-and-variables). |
| `.completer(&dyn Completer)` | A completion list under the caret, operated by the kit; see [autocomplete](code.md#autocomplete). |
| `.suggestions(&[String])` | An editable combo box: free text plus a chevron listing the suggestions; see [combo box](#combo-box). |
| `.width(f32)` | Total width (default: the available width). |

Returns the text edit's `Response`: `changed()` after each edit, `lost_focus()` when the user
leaves the field. `.show(ui)` returns a `JBTextFieldOutput` instead: the response, `submitted`
(<kbd>Enter</kbd>), the `caret_rect` and the `{{variable}}` at the caret, for autocomplete.

## Behavior

- Focus draws a 2 pt accent outline; an error keeps the outline red.
- The clear button empties the field and keeps the focus in it.
- Multi-line fields insert newlines with Enter and leave the field with Tab. `submitted` is
  always false for them. Password masking and highlighting still apply.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/jb_text_field_multiline.dark.png">
  <img alt="Multi-line request body and a disabled host list" src="img/jb_text_field_multiline.light.png" width="360">
</picture>

### Combo box

`.suggestions(&values)` turns the field into an editable combo box, as Android Studio uses for
a field name, a recent value or a path: the user may type anything, and a chevron at the
right lists the suggestions below the field, as wide as it.

- Typing lists the suggestions containing the text, ignoring case; <kbd>Up</kbd>/<kbd>Down</kbd>
  move, <kbd>Enter</kbd>, <kbd>Tab</kbd> or a click replace the whole text, <kbd>Esc</kbd>
  closes the list and keeps the focus.
- The chevron, <kbd>Alt</kbd>+<kbd>Down</kbd> or <kbd>Ctrl</kbd>+<kbd>Space</kbd> list all
  suggestions on an empty field or one holding a suggestion; a second click on the chevron
  closes the list.
- Text that is no suggestion stays: validate it with `.error(..)` if it must be one, or use a
  [`ComboBox`](combo-box.md) when only the listed values are valid.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/jb_text_field_suggestions.dark.png">
  <img alt="A field name combo box: e typed, the fields containing e listed below the field" src="img/jb_text_field_suggestions.light.png" width="240">
</picture>

## JBIntSpinner

An integer input (page size, timeout, port), as Android Studio's integer fields: a text
field whose number is written to the value as soon as it is valid and in range. Anything
else shows the error below the field and leaves the value alone; leaving the field shows the
value again. <kbd>Up</kbd>/<kbd>Down</kbd> step it, clamped to the range. `changed()`
reports a new value, not every keystroke. It edits any `Integer`: `i32`, `i64`, `u16`,
`u32`, `u64`, `usize`.

```rust
use archipel_kit::ui::{FieldLabel, Form, JBIntSpinner};

fn query_options(ui: &mut archipel_kit::egui::Ui, limit: &mut u64) {
    Form::new().show(ui, |form| {
        form.fields(|ui| {
            ui.add(FieldLabel::new("Limit:"));
            ui.add(JBIntSpinner::new(limit).name("Limit").range(1..=10_000).step(10));
        });
    });
}
```

| Builder | Effect |
|---|---|
| `JBIntSpinner::new(&mut T)` | Edits the integer in place; every value of `T` is valid. |
| `.range(min..=max)` | The accepted values; the error names them. |
| `.step(n)` | What <kbd>Up</kbd>/<kbd>Down</kbd> add or subtract (default 1). |
| `.name(str)` | Accessible name; put a `FieldLabel` or form row label before the field. |
| `.width(f32)` | Default `NUMBER_FIELD_WIDTH` (64 pt, six digits). |
| `.enabled(bool)`, `.id(Id)` | As on `JBTextField`. |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/jb_int_spinner.dark.png">
  <img alt="A page size field holding 50 and a timeout field holding 900 with the error Enter a number from 1 to 600" src="img/jb_int_spinner.light.png" width="320">
</picture>

## Accessibility

Role `TextInput`, `MultilineTextInput` (or `PasswordInput`), named explicitly with `.name`,
otherwise by the visible label or placeholder. The error message is
also the input's tooltip. A combo box's chevron is a button named "Show suggestions".

## Testing

```rust
use archipel_kit::ui::JBTextField;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::Theme;

let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
    ui.add(JBTextField::new(text).label("Name"));
});
let input = harness.get_by_label("Name");
input.focus();
input.type_text("archipel");
harness.run();
assert_eq!(harness.state(), "archipel");
```

Source: [`ui/text_field.rs`](../../crates/archipel-kit/src/ui/text_field.rs) ·
Tests: [`tests/jb_text_field.rs`](../../crates/archipel-kit/tests/jb_text_field.rs)
