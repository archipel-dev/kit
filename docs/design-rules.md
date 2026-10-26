# Design rules

The kit is the Islands UI of Android Studio and IntelliJ, rebuilt clean-room. A component
looks right only when it behaves like its counterpart there: sizes, colors, motion and
interaction. These rules are what the measurements in the
[component map](component-map.md#measured-behavior) add up to. `tests/design_rules.rs`
checks the ones a machine can check; reviews check the rest.

## Checked by `cargo test`

| Rule | Instead |
|---|---|
| No color literals (`Color32::from_rgb`, `from_gray`, `from_black_alpha` …) | A `Theme::colors(ctx)` field or a `Tone`. A missing color is measured in Islands Dark **and** Light and added to both themes. |
| No corner radius literals | `tokens::radius`. |
| No stock egui controls (`ui.button`, `egui::Checkbox`, `egui::Slider` …) | The kit's `Button`, `JBCheckBox`, `OnOffButton`, `JBTextField`, `ComboBox` … |
| No egui tooltips (`on_hover_text`) | `HelpTooltip` (surface, shortcut, placement, delay). |
| No literal animation durations | `tokens::motion`. |
| Every interactive component passes `testing::conformance` | Add it to `tests/conformance.rs`, or list it in `NOT_INTERACTIVE` with why. |

## The Islands look and feel

- **Islands, gaps, frame.** Content lives in islands (`SimpleToolWindowPanel`, `ToolWindowPane`) on the window
  frame, 4 pt apart. Gaps have no lines; they are the splitters. Tool windows have stripes.
- **Headers are 40 pt.** A tool window's title bar and an editor's tab bar have the same
  height. Header actions are hidden until the pointer is over the island or focus is in it;
  they appear at once and fade out 200 ms after the pointer leaves. In a narrow island they
  win over the title, which hides whole.
- **Rows have no hover.** Trees, lists and tables show only the selection: blue with focus,
  gray without. Popup rows (menus, completion lists) highlight under the pointer instead,
  with one highlight per menu shared with the keyboard.
- **Buttons don't react to hover or press.** Only focus shows: the border becomes a 2 pt
  focus-colored border, one point of it outside. Flat toolbar controls (icon buttons, toolbar
  and status bar widgets, stripe buttons, tabs) do show a hover fill.
- **Checkboxes and radios** have their own blue and no hover change.
- **Menus**: 24 pt rows that touch, 8 pt popup padding, pills inset 8 pt, icons 8 pt inside
  the pill, text 4 pt after the icon; first pointer move after opening ignored.
- **Text is dimmed when it is secondary**: unselected tabs, status bar items, breadcrumbs.
  Hover brings it to full strength.
- **Motion is short and functional.** Things appear at once; delays are for hiding
  (header actions) and for tooltips (0.5 s; stripe buttons at once).

## Adding a component

1. Find its counterpart in Android Studio and measure it: [tools/reference](../tools/reference/README.md).
2. Build it from [parts](building-components.md) and tokens; add tokens and theme colors
   for what you measured, with a comment saying so.
3. Prove the behavior in tests (states, keys, timing), pass conformance and the audit, and
   add snapshots in both themes. Compare them side by side with the reference.
4. Record the measurements in the [component map](component-map.md#measured-behavior).
