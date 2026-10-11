# Getting started

## Add the crate

```toml
[dependencies]
archipel-kit = { git = "https://github.com/archipel-dev/kit" }
eframe = "0.33"

[dev-dependencies]
archipel-kit = { git = "https://github.com/archipel-dev/kit", features = ["testing"] }
```

Archipel Kit re-exports the `egui` version it is built against as `archipel_kit::egui`, so
you never mix two egui versions.

## Install the theme

Call `install` once, before the first frame. In eframe that is the app creation closure:

```rust,no_run
use archipel_kit::{Theme, egui};

fn main() -> eframe::Result {
    eframe::run_native(
        "My tool",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            archipel_kit::install(&cc.egui_ctx, Theme::dark());
            Ok(Box::new(App::default()))
        }),
    )
}

#[derive(Default)]
struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello");
        });
    }
}
```

`install` loads the bundled fonts (Inter and JetBrains Mono), sets spacing and styles plain
egui widgets to match. Switch themes at runtime with `Theme::light().apply(ctx)`.

## Build a panel

Workbench layouts are islands (panels) on the frame color:

```rust
use archipel_kit::components::{IconButton, ListRow, Panel};
use archipel_kit::{Icon, egui};

fn sidebar(ui: &mut egui::Ui, files: &[&str], selected: &mut usize) {
    Panel::new("Project")
        .actions(|ui| {
            ui.add(IconButton::new(Icon::CollapseAll, "Collapse all"));
        })
        .show(ui, |ui| {
            for (index, name) in files.iter().enumerate() {
                let row = ListRow::new(*name)
                    .icon(Icon::for_file(name))
                    .selected(*selected == index)
                    .show(ui);
                if row.response.clicked() {
                    *selected = index;
                }
            }
        });
}
```

Components are stateless: they borrow your state (`&mut String`, `&mut bool`) or report
what happened (`clicked()`, `RowResponse::toggled`, `MessageResponse::action`), and your app
decides what changes. That keeps undo, persistence and commands in your code, not in the
widgets.

Next: [theming](theming.md), [testing](testing.md), or browse the
[components](README.md#components).
