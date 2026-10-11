# Archipel Kit TUI

`archipel-kit-tui` is Archipel Kit for the terminal. It uses the same theme colors, the same
icon set and the same component names, drawn into terminal cells with
[ratatui](https://ratatui.rs). An Archipel tool can open as a window or in a terminal and look
and work much the same in both.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/ide.dark.svg">
  <img alt="An IDE screen in the terminal: project tree, editor with tabs, run panel" src="img/ide.light.svg" width="806">
</picture>

## Font: use a Nerd Font

All icons in the TUI are [Nerd Font](https://www.nerdfonts.com) characters: codicons for kit
glyphs and symbols, language logos for files, and Powerline half circles for rounded
shapes. Set your terminal font to a Nerd Font. We recommend **JetBrainsMono Nerd Font**: it is
JetBrains Mono, the GUI kit's code font, with the icons added. Use the plain variant, not
"Mono", because the Mono variant shrinks the icons to one cell. The components leave a space
after every icon for it to draw into.

```bash
# macOS
brew install --cask font-jetbrains-mono-nerd-font
# Linux: download JetBrainsMono.zip from the Nerd Fonts releases into ~/.local/share/fonts
```

Use a terminal with 24-bit color, such as WezTerm, kitty, Ghostty, iTerm2, Windows Terminal,
Alacritty or GNOME Terminal.

The images on this page are SVG snapshots from the tests. Nerd Font characters can't be text
in them, because a browser has no Nerd Font and the kit ships no third-party artwork. So icon
cells show the matching Archipel icon, and the half circles and box marks are drawn as shapes.
In a terminal, the font draws the codicons and logos. The two look alike.

## How the GUI maps to cells

| GUI | Terminal |
|---|---|
| Floating islands on the frame color | `Island`s filled with the island color one cell apart on a `Frame`; quadrant blocks notch the corners |
| Rounded buttons, chips, selected rows and tabs | Pills: a fill between Powerline half circles (`` ``) |
| Theme colors (`Theme::colors`) | 24-bit colors from the same `Theme`; translucent colors are blended over the island |
| Glyphs (`archipel_icons::Glyph`) | The matching codicon, in the same color (icon gray, run green, error red) |
| Language icons | The language's Nerd Font logo in its Linguist color, lifted to 3:1 contrast on dark islands |
| Symbol icons (class, method, field …) | The codicon symbol in the symbol color |
| Focus ring | Bold, underlined label; accent caps on a text field |
| Inter (UI) / JetBrains Mono (code) | The terminal font; JetBrainsMono Nerd Font recommended |

The kit `Theme` is the single source: `Look::new(theme)` turns any GUI theme, including a
custom one, into a terminal look.

## Getting started

```toml
[dependencies]
archipel-kit-tui = { git = "https://github.com/archipel-dev/kit" }
```

Components are ratatui `Widget`s that borrow a `Look`. They are stateless like the GUI
components: your app owns the state and passes it in on every frame.

```rust
use archipel_kit_tui::components::{Button, Checkbox, Frame, Island};
use archipel_kit_tui::ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use archipel_kit_tui::{Glyph, Look};

fn draw(look: &Look, format_on_save: bool, area: Rect, buf: &mut Buffer) {
    Frame::new(look).render(area, buf);
    let island = Island::new(look).title("Settings");
    let panel = Rect::new(1, 1, area.width - 2, area.height - 2);
    let inner = island.inner(panel);
    island.render(panel, buf);
    Checkbox::new(look, format_on_save, "Format on save")
        .render(Rect::new(inner.x, inner.y, inner.width, 1), buf);
    let run = Button::primary(look, "Apply").icon(Glyph::Success);
    let width = run.width();
    run.render(Rect::new(inner.x, inner.y + 2, width, 1), buf);
}

let look = Look::dark();
let area = Rect::new(0, 0, 40, 8);
let mut buf = Buffer::empty(area);
draw(&look, true, area, &mut buf);
```

In an app, draw inside `terminal.draw(|frame| draw(&look, …, frame.area(), frame.buffer_mut()))`
and handle keys with crossterm, which ratatui re-exports. The [gallery](#gallery) is a small
complete example.

## Components

All images show the dark theme; the light versions are next to them in [`img/`](img/).

### Button

`Button::primary`, `secondary`, `ghost` and `danger`, with an optional icon, `focused` and
`enabled`. `width()` gives the cells it needs.

<img alt="Buttons in the terminal" src="img/button.dark.svg" width="538"> <img alt="Buttons, light" src="img/button.light.svg" width="538">

### Checkbox, Radio, Toggle

`Checkbox` (checked, unchecked, `mixed`, disabled), `Radio` and the `Toggle` switch.

<img alt="Checkboxes, radio buttons and toggles" src="img/choice.dark.svg" width="370"> <img alt="Choices, light" src="img/choice.light.svg" width="370">

### TextField

A pill with the value, a placeholder while empty, accent caps and a cursor while focused, an
optional label above and an error below. `height()` counts the rows.

<img alt="Text fields: placeholder, focused, error" src="img/text-field.dark.svg" width="370"> <img alt="Text fields, light" src="img/text-field.light.svg" width="370">

### ListRow

Rows of lists and trees: `depth`, an `expanded` chevron, an icon (`Glyph`, `Language` or
`Symbol`), `secondary` text and `selected`, an inset pill like in the GUI.

<img alt="A project tree and a structure view" src="img/list-row.dark.svg" width="336"> <img alt="List rows, light" src="img/list-row.light.svg" width="336">

### TabBar

Editor tabs with icons and modified dots. The selected tab is a tinted pill with a close
button.

<img alt="Editor tabs" src="img/tab-bar.dark.svg" width="538"> <img alt="Editor tabs, light" src="img/tab-bar.light.svg" width="538">

### Banner

Info, success, warning and error messages with action pills on the right.

<img alt="Banners in four severities" src="img/banner.dark.svg" width="538"> <img alt="Banners, light" src="img/banner.light.svg" width="538">

### SegmentedControl, ProgressBar, Kbd, Chip

<img alt="Segmented control, progress bar, key caps and chips" src="img/misc.dark.svg" width="403"> <img alt="Misc, light" src="img/misc.light.svg" width="403">

### Icons

`LanguageIcon`, `SymbolIcon`, and `Look::glyph` with `Look::glyph_color`. The character
tables are in `archipel_kit_tui::glyphs`: every `Glyph`, `Language` and `SymbolKind` has a
Nerd Font character, and a test checks that none is missing.

<img alt="Language logos, symbol icons and glyphs" src="img/icons.dark.svg" width="605"> <img alt="Icons, light" src="img/icons.light.svg" width="605">

## Testing

`archipel_kit_tui::testing` renders widgets into an in-memory buffer without a terminal:

- `render(width, height, |area, buf| …)` draws into a fresh buffer.
- `lines(&buf)` gives the text of each row, for behavior assertions.
- `snapshot(name, &buf)` compares the buffer, as SVG, with `docs/tui/img/<name>.svg`.
  `UPDATE_SNAPSHOTS=1 cargo test -p archipel-kit-tui` accepts changes.

```rust
use archipel_kit_tui::components::Button;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::testing::{lines, render};
use archipel_kit_tui::Look;

let look = Look::dark();
let buf = render(12, 1, |area, buf| Button::secondary(&look, "OK").render(area, buf));
assert_eq!(lines(&buf), ["\u{e0b6} OK \u{e0b4}"]);
```

The SVG snapshots need no GPU, so they run everywhere.

## Gallery

```bash
cargo run -p archipel-kit-tui --example gallery
```

Every component lives on four pages: Controls, Lists, Feedback and Icons. Keys: `←`/`→` (or
`h`/`l`, `Tab`) switch pages, `↑`/`↓` (or `j`/`k`) move, `Space` toggles, `t` switches between
dark and light, and `q` quits.

## Roadmap

- **Same app, two front ends.** The IDE, Nexus HTTP and Mongo Raabit describe their screens
  once and render them with the GUI kit or the TUI kit (`archipel-ide --tui`). This needs the
  declarative view layer from the kit roadmap, so a screen isn't written twice by hand.
- **Interaction helpers**: focus order, mouse hit testing and scrolling lists, shared by all
  components.
- **More components**: Dropdown and menus as popups, Toast, dialogs, tooltips (on focus), and a
  split layout with draggable dividers.
- **`archipel-theme` crate**: the theme and tokens without egui, so the TUI doesn't compile
  egui.
