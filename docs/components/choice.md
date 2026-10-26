# JBCheckBox, ThreeStateCheckBox, JBRadioButton, OnOffButton

Boolean and one-of-many choices. Use a **checkbox** for options that are confirmed later
(a dialog's OK), **radio buttons** for two to five mutually exclusive options, and a
**toggle** for a setting that applies immediately.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/choice.dark.png">
  <img alt="Checkboxes (checked, unchecked, mixed, disabled), radio buttons and toggles" src="img/choice.light.png" width="330">
</picture>

## Usage

```rust
use archipel_kit::ui::{JBCheckBox, JBRadioButton, OnOffButton};
use archipel_kit::egui;

#[derive(PartialEq, Clone, Copy)]
enum Indent { Tabs, Spaces }

fn editor_settings(ui: &mut egui::Ui, format: &mut bool, indent: &mut Indent, auto_save: &mut bool) {
    ui.add(JBCheckBox::new(format, "Reformat code on save"));
    ui.add(JBRadioButton::new(indent, Indent::Tabs, "Tabs"));
    ui.add(JBRadioButton::new(indent, Indent::Spaces, "Spaces"));
    ui.add(OnOffButton::new(auto_save, "Save files automatically"));
}
```

A parent checkbox shows **mixed** while some of its children are checked; clicking it checks
all of them:

```rust
use archipel_kit::ui::JBCheckBox;
use archipel_kit::egui;

fn inspections(ui: &mut egui::Ui, children: &mut [bool; 3]) {
    let all = children.iter().all(|c| *c);
    let some = children.iter().any(|c| *c);
    let mut parent = all;
    if ui.add(JBCheckBox::new(&mut parent, "All inspections").mixed(some && !all)).changed() {
        *children = [parent; 3];
    }
}
```

## API

| Component | Builders |
|---|---|
| `JBCheckBox::new(&mut bool, label)` | `.mixed(bool)`, `.enabled(bool)`, `.comment(text)`, `.inline_comment(text)` ([UI DSL](dsl.md)), `.label_visible(bool)` (off: the label is only the accessible name, e.g. in a row's leading slot) |
| `ThreeStateCheckBox::new(&mut ThreeState, label)` | `.enabled(bool)`; a click cycles `Unchecked`, `Checked`, `Indeterminate`, `Unchecked` |
| `JBRadioButton::new(&mut T, value, label)` | `.enabled(bool)`, `.comment(text)`, `.inline_comment(text)`; selected when `*current == value` |
| `OnOffButton::new(&mut bool, label)` | `.enabled(bool)` |

All return a `Response` whose `changed()` is true on the frame the value changed.

## Behavior

IntelliJ spec section 13.3 (`DarculaCheckBoxUI`, `DarculaRadioButtonUI`):

| Input | Result |
|---|---|
| Click on the box, the label or the inline comment | Toggles when the button is **released** inside the row; pressing and moving away cancels. A comment below is not a target. |
| <kbd>Space</kbd> on the focused control | Toggles when the key is **released**. |
| <kbd>Enter</kbd> | Does nothing here: in a dialog it triggers the default button. |
| Click on a `ThreeStateCheckBox` | Unchecked, checked, indeterminate, unchecked. A `JBCheckBox` with `.mixed(true)` only displays the dash and checks on a click. |
| Hover | No change (measured on Android Studio, and the spec has none). |
| Focus | A 2 pt border in the control's blue, one of them outside. |

- The label starts 8 pt after the box (measured on Android Studio). The glyph is 16 pt: the
  spec's `CheckBox.iconSize` is 24 with a 14 pt glyph inside 18; the measured 16 pt box
  is kept.
- Mnemonics (<kbd>Alt</kbd>+letter) are not provided for choices.
- Toggles animate the knob (`OnOffButton`: on/off colors `toggle_on`, `toggle_off`).

## Accessibility

Checkboxes and toggles have role `CheckBox`, radios `RadioButton`; all are named by their
label and report checked or unchecked.

Source: [`ui/choice.rs`](../../crates/archipel-kit/src/ui/choice.rs) ·
Tests: [`tests/choice.rs`](../../crates/archipel-kit/tests/choice.rs)
