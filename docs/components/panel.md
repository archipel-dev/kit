# Panel, SectionHeader, Separator, PropertyGrid

Layout building blocks. A **panel** is an island with a title bar: the unit of a workbench
layout (tool windows, sidebars, inspectors). **Section headers** group content inside it;
**property grids** show key/value details.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/panel.dark.png">
  <img alt="A Response panel with copy and more actions, a General section and a property grid" src="img/panel.light.png" width="360">
</picture>

## Usage

```rust
use archipel_kit::components::{IconButton, Panel, PropertyGrid, SectionHeader};
use archipel_kit::{Glyph, egui};

fn response_panel(ui: &mut egui::Ui) {
    Panel::new("Response")
        .actions(|ui| {
            // Laid out right to left: the first action is the rightmost.
            ui.add(IconButton::new(Glyph::More, "More"));
            ui.add(IconButton::new(Glyph::Copy, "Copy"));
        })
        .show(ui, |ui| {
            ui.add(SectionHeader::new("General"));
            PropertyGrid::new("response")
                .row("Status", "200 OK")
                .mono_row("Content-Type", "application/json")
                .show(ui);
        });
}
```

### Tool window header

`.on_hide(|| …)` adds the tool window's hide button at the right end of the title bar (named
"Hide <title>", tooltip "Hide"). With `.actions(...)` the title bar is the IntelliJ-style
header: title, action icon buttons, hide. Panels without `on_hide` are unchanged.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/panel_tool_window.dark.png">
  <img alt="A Project tool window with find and collapse-all actions and a hide button" src="img/panel_tool_window.light.png" width="360">
</picture>

```rust
use archipel_kit::components::{IconButton, Panel};
use archipel_kit::{Glyph, egui};

fn project_window(ui: &mut egui::Ui, visible: &mut bool) {
    Panel::new("Project")
        .actions(|ui| {
            ui.add(IconButton::new(Glyph::CollapseAll, "Collapse all"));
            ui.add(IconButton::new(Glyph::Search, "Find"));
        })
        .on_hide(|| *visible = false)
        .show(ui, |ui| {
            ui.label("src/");
        });
}
```

The kit has one container with a title bar, so the header is part of `Panel` rather than a
second component that would paint the same title and toolbar.

## API

| Component | API |
|---|---|
| `Panel::new(title)` | `.actions(|ui| …)` toolbar on the right of the title (right to left), `.on_hide(\|\| …)` hide button, `.show(ui, |ui| body)`. |
| `SectionHeader::new(title)` | Title with a hairline, with section spacing above. |
| `Separator` | 1 pt line across the available width, or a vertical one in a horizontal layout. |
| `PropertyGrid::new(id)` | `.row(key, value)`, `.mono_row(key, value)`, `.show(ui)`. |

Panels sit on the frame color with a 2 pt gap between them (`tokens::size::ISLAND_GAP`); see
the [gallery](../../crates/archipel-kit-gallery/src/main.rs) for a full workbench layout.

Source: [`components/layout.rs`](../../crates/archipel-kit/src/components/layout.rs) ·
Tests: [`tests/data.rs`](../../crates/archipel-kit/tests/data.rs)
