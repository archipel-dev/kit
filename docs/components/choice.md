# Checkbox, Radio, Toggle

Boolean and one-of-many choices. Use a **checkbox** for options that are confirmed later
(a dialog's OK), **radio buttons** for two to five mutually exclusive options, and a
**toggle** for a setting that applies immediately.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/choice.dark.png">
  <img alt="Checkboxes (checked, unchecked, mixed, disabled), radio buttons and toggles" src="img/choice.light.png" width="330">
</picture>

## Usage

```rust
use archipel_kit::components::{Checkbox, Radio, Toggle};
use archipel_kit::egui;

#[derive(PartialEq, Clone, Copy)]
enum Indent { Tabs, Spaces }

fn editor_settings(ui: &mut egui::Ui, format: &mut bool, indent: &mut Indent, auto_save: &mut bool) {
    ui.add(Checkbox::new(format, "Reformat code on save"));
    ui.add(Radio::new(indent, Indent::Tabs, "Tabs"));
    ui.add(Radio::new(indent, Indent::Spaces, "Spaces"));
    ui.add(Toggle::new(auto_save, "Save files automatically"));
}
```

A parent checkbox shows **mixed** while some of its children are checked; clicking it checks
all of them:

```rust
use archipel_kit::components::Checkbox;
use archipel_kit::egui;

fn inspections(ui: &mut egui::Ui, children: &mut [bool; 3]) {
    let all = children.iter().all(|c| *c);
    let some = children.iter().any(|c| *c);
    let mut parent = all;
    if ui.add(Checkbox::new(&mut parent, "All inspections").mixed(some && !all)).changed() {
        *children = [parent; 3];
    }
}
```

## API

| Component | Builders |
|---|---|
| `Checkbox::new(&mut bool, label)` | `.mixed(bool)`, `.enabled(bool)` |
| `Radio::new(&mut T, value, label)` | `.enabled(bool)`; selected when `*current == value` |
| `Toggle::new(&mut bool, label)` | `.enabled(bool)` |

All return a `Response` whose `changed()` is true on the frame the value changed.

## Behavior

- The whole row (control and label) is the click target.
- <kbd>Space</kbd> toggles a focused control.
- Toggles animate the knob.

## Accessibility

Checkboxes and toggles have role `CheckBox`, radios `RadioButton`; all are named by their
label and report checked or unchecked.

Source: [`components/choice.rs`](../../crates/archipel-kit/src/components/choice.rs) ·
Tests: [`tests/choice.rs`](../../crates/archipel-kit/tests/choice.rs)
