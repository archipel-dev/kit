# AsyncProcessIcon, ProgressBar

Progress indicators: a **spinner** for work of unknown length that blocks a small area
(connecting, loading a panel), a **progress bar** for downloads and builds, determinate when
the fraction is known.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/progress.dark.png">
  <img alt="A spinner with a label and progress bars at 0, 40 and 100 percent" src="img/progress.light.png" width="360">
</picture>

## Usage

```rust
use archipel_kit::ui::{ProgressBar, AsyncProcessIcon};
use archipel_kit::egui;

fn status(ui: &mut egui::Ui, download: Option<f32>) {
    ui.horizontal(|ui| {
        ui.add(AsyncProcessIcon::new().label("Indexing"));
        ui.label("Indexing…");
    });
    ui.add(ProgressBar::new(download).label("Downloading JDK"));
}
```

## API

| Builder | Effect |
|---|---|
| `AsyncProcessIcon::new()` | `.size(f32)` (16 pt), `.label(str)` (accessible name, "Loading"). |
| `ProgressBar::new(Option<f32>)` | `Some(0.0..=1.0)` determinate, `None` indeterminate. `.label(str)`, `.width(f32)`. |

Animated indicators request repaints only while they are visible.

## Accessibility

Role `ProgressIndicator`; determinate bars report their value in percent.

Source: [`ui/progress.rs`](../../crates/archipel-kit/src/ui/progress.rs) ·
Tests: [`tests/feedback.rs`](../../crates/archipel-kit/tests/feedback.rs)
