# Archipel Kit TUI

`archipel-kit-tui` is Archipel Kit for the terminal, built on [ratatui](https://ratatui.rs).
It keeps the GUI kit's theme colors, icons and component names, in the flat style of modern
Neovim setups such as NvChad. Panels are blocks of color without borders. Titles are pills.
Open files are a tab line of buffers, and the status line has a mode block. Input fields look
like prompts. It looks like Archipel, but it should feel like a good terminal app, not like a
GUI squeezed into cells.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/ide.dark.svg">
  <img alt="An IDE screen in the terminal: project tree, editor with tabs, run panel" src="img/ide.light.svg" width="840">
</picture>

## Font: use a Nerd Font

All icons in the TUI are [Nerd Font](https://www.nerdfonts.com) characters: codicons for kit
glyphs and symbols, language logos for files, Material Design checkbox and switch icons, and
Powerline half circles for the pill shapes in the status bar and chips. Set your terminal font to a Nerd Font. We recommend **JetBrainsMono Nerd Font**: it is
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
cells show the matching Archipel icon, and the half circles, box marks and frame lines are
drawn as shapes.
In a terminal, the font draws the codicons and logos. The two look alike.

## How the GUI maps to cells

The colors and icons come straight from the GUI. The shapes follow terminal conventions.

| GUI | Terminal |
|---|---|
| Floating islands on the frame color | `Island`: a flat block of the island color, one cell apart on a `Frame`; no borders |
| Tool window header, active tool window | A title pill on the first row, in the accent when the panel has focus (like Telescope titles) |
| Editor tabs | `JBTabs`, a tab line like NvChad's tabufline: buffers on the frame color, the selected one in the island color so it joins the editor |
| Status bar | `StatusBar`: a mode block in the accent ending in a half circle, key hints, and a status block on the right |
| Text field | A prompt: a pill of the field color with the label pill (or a search icon) inside, the error on the next row |
| Selected row (inset pill) | A full-width bar in the selection color; quieter when the list isn't focused |
| Hover | The hover color on the focused row (keyboard focus stands in for the mouse) |
| Rounded buttons | A filled label with one cell of padding; the shortcut key is underlined; focus shows `❯ … ❮` |
| InlineBanner | One row: an edge in the severity color, icon and text on the tint, actions as bold links |
| Theme colors (`Theme::colors`) | 24-bit colors from the same `Theme`; translucent colors are blended over the island |
| Glyphs, language and symbol icons | Nerd Font codicons and logos in the same colors (language colors lifted to 3:1 contrast) |

The kit `Theme` is the single source: `Look::new(theme)` turns any GUI theme, including a
custom one, into a terminal look. Every bundled theme works: `Look::new(Theme::darcula())`, or
pick from `Theme::all()` (the gallery example's `t` key cycles through them).

## Getting started

```toml
[dependencies]
archipel-kit-tui = { git = "https://github.com/archipel-dev/kit" }
```

Components are ratatui `Widget`s that borrow a `Look`. They are stateless like the GUI
components: your app owns the state and passes it in on every frame.

```rust
use archipel_kit_tui::components::{Button, JBCheckBox, Pane, PaneKind};
use archipel_kit_tui::ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use archipel_kit_tui::{Glyph, Look};

fn draw(look: &Look, format_on_save: bool, area: Rect, buf: &mut Buffer) {
    let pane = Pane::new(look).kind(PaneKind::Sidebar).title("Settings");
    let inner = pane.inner(area);
    pane.render(area, buf);
    JBCheckBox::new(look, format_on_save, "Format on save")
        .on(look.palette.darker_black)
        .render(Rect::new(inner.x + 1, inner.y + 1, inner.width, 1), buf);
    let run = Button::primary(look, "Apply")
        .icon(Glyph::Success)
        .on(look.palette.darker_black);
    let width = run.width();
    run.render(Rect::new(inner.x + 1, inner.y + 3, width, 1), buf);
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

Each component shows dark and light; the images are SVG snapshots from the tests.

### Island and Frame

`Frame` fills the background, and `Island::new(look).title("Project").focused(true)` is a
panel. `inner(area)` is its content area. Leave one cell of frame between panels, as in the
GUI. The IDE screen at the top shows the layout.

### Button

`Button::primary`, `secondary`, `ghost` and `danger`, with an optional icon and `key` (the
underlined shortcut), plus `focused` and `enabled`. `width()` gives the cells it needs,
focus markers included.

<img alt="Buttons in the terminal" src="img/button.dark.svg" width="538"> <img alt="Buttons, light" src="img/button.light.svg" width="538">

### JBCheckBox, JBRadioButton, OnOffButton

`JBCheckBox` (checked, unchecked, `mixed`, disabled), `JBRadioButton` and the `OnOffButton` switch. A
`focused` row gets the hover color.

<img alt="Checkboxes, radio buttons and toggles" src="img/choice.dark.svg" width="403"> <img alt="Choices, light" src="img/choice.light.svg" width="403">

### JBTextField

A prompt-style input: the label in a pill (a search icon without a label), a placeholder
while empty, an accent pill and a cursor while focused, and a red pill with the message on
the next row on error. `height()` is 1, or 2 with an error.

<img alt="Text fields: placeholder, focused, error" src="img/text-field.dark.svg" width="370"> <img alt="Text fields, light" src="img/text-field.light.svg" width="370">

### ColoredListCellRenderer

Rows of lists and trees: `depth`, an `expanded` chevron, an icon (`Glyph`, `Language` or
`Symbol`), `secondary` text, and `selected`, shown as a full-width bar. Use
`active(false)` for a selection in a list that doesn't have focus.

<img alt="A project tree and a structure view" src="img/list-row.dark.svg" width="336"> <img alt="List rows, light" src="img/list-row.light.svg" width="336">

### JBTabs

The tab line of open buffers. Each tab has its language icon. A green dot marks a modified
buffer, and the selected tab has a red close icon. Put it directly above the editor panel.

<img alt="Editor tabs" src="img/tab-bar.dark.svg" width="504"> <img alt="Editor tabs, light" src="img/tab-bar.light.svg" width="504">

### InlineBanner

Info, success, warning and error messages with actions on the right.

<img alt="Banners in four severities" src="img/banner.dark.svg" width="538"> <img alt="Banners, light" src="img/banner.light.svg" width="538">

### StatusBar

The status line: `mode` (the accent block at the left), key hints as (key, action), and the
`right` status block.

<img alt="Status bar with mode, key hints and position" src="img/status-bar.dark.svg" width="605"> <img alt="Status bar, light" src="img/status-bar.light.svg" width="605">

### SegmentedButton, ProgressBar, Kbd, Chip

<img alt="Segmented control, progress bar, key caps and chips" src="img/misc.dark.svg" width="437"> <img alt="Misc, light" src="img/misc.light.svg" width="437">

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
assert_eq!(lines(&buf), ["  OK"]);
```

The SVG snapshots need no GPU, so they run everywhere.

## Gallery

```bash
cargo run -p archipel-kit-tui --example gallery
```

The gallery has four pages: Controls, Lists, Feedback and Icons. Keys: `←`/`→` (or `h`/`l`,
`Tab`) switch pages, `↑`/`↓` (or `j`/`k`) move, `Space` toggles, `t` switches between dark and
light, and `q` quits.

## Roadmap

- **Same app, two front ends.** The IDE, Nexus HTTP and Mongo Raabit describe their screens
  once and render them with the GUI kit or the TUI kit (`archipel-ide --tui`). This needs the
  declarative view layer from the kit roadmap, so a screen isn't written twice by hand.
- **Interaction helpers**: focus order, mouse hit testing and scrolling lists, shared by all
  components.
- **More components**: ComboBox and menus as popups, Notification, dialogs, tooltips (on focus), and a
  split layout with draggable dividers.
- **`archipel-theme` crate**: the theme and tokens without egui, so the TUI doesn't compile
  egui.

## Interactive controls

<img alt="Controls" src="img/controls.dark.svg" width="504"> <img alt="Controls, light" src="img/controls.light.svg" width="504">

## Lists, trees, tables and speed search

<img alt="List with speed search, tree and table" src="img/collections.dark.svg" width="504"> <img alt="Collections, light" src="img/collections.light.svg" width="504">

## Popups and menus

<img alt="Action popup menu with a submenu, and a Telescope-style list" src="img/popup.dark.svg" width="605"> <img alt="Popups, light" src="img/popup.light.svg" width="605">

## Toolbar, tabs and combo box

<img alt="Toolbar with a tooltip, tab line and combo box" src="img/toolbar.dark.svg" width="672"> <img alt="Toolbar, light" src="img/toolbar.light.svg" width="672">
