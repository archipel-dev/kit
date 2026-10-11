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

## API

| Component | API |
|---|---|
| `Panel::new(title)` | `.actions(|ui| …)` toolbar on the right of the title (right to left), `.show(ui, |ui| body)`. |
| `SectionHeader::new(title)` | Title with a hairline, with section spacing above. |
| `Separator` | 1 pt line across the available width, or a vertical one in a horizontal layout. |
| `PropertyGrid::new(id)` | `.row(key, value)`, `.mono_row(key, value)`, `.show(ui)`. |

Panels sit on the frame color with a 2 pt gap between them (`tokens::size::ISLAND_GAP`); see
the [gallery](../../crates/archipel-kit-gallery/src/main.rs) for a full workbench layout.

Source: [`components/layout.rs`](../../crates/archipel-kit/src/components/layout.rs) ·
Tests: [`tests/data.rs`](../../crates/archipel-kit/tests/data.rs)
