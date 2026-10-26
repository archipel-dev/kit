# JBLoadingPanel

Content that is being loaded: a veil over it with a spinner and a text (IntelliJ's
`JBLoadingPanel`). It shows only after a start delay, so loads that finish quickly do not
flicker.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/loading_panel.dark.png">
  <img alt="A list under a dark veil with a spinner and the text Loading results" src="img/loading_panel.light.png" width="340">
</picture>

## Usage

```rust
use archipel_kit::ui::{ColoredListCellRenderer, JBList, JBLoadingPanel};
use archipel_kit::egui;

fn results(ui: &mut egui::Ui, loading: bool, rows: &[String], selected: &mut Option<usize>) {
    JBLoadingPanel::new("results", loading)
        .start_delay_ms(300)
        .text("Loading results…")
        .show(ui, |ui| {
            JBList::new("rows", rows.len(), selected)
                .show(ui, |i| ColoredListCellRenderer::new(&rows[i]));
        });
}
```

## Behavior

| Rule | Behavior |
|---|---|
| Start delay | The veil appears once `loading` has been true for `start_delay_ms` (default 0: at once). A load that ends sooner never shows it. |
| Veil | `Colors::loading_veil`: the IDE's rgba(200, 200, 200, 240) in the light theme, the island color at 240/255 in the dark one. It takes the pointer: the content under it cannot be clicked. |
| Spinner and text | An `AsyncProcessIcon` centered, the text under it in the secondary color. |
| End | The veil fades out over `tokens::loading::FADE_OUT` (0.2 s; the spec only says "fade-out animator"). |

The caller owns the `loading` flag; the panel remembers only when loading began.

Source: [`ui/loading_panel.rs`](../../crates/archipel-kit/src/ui/loading_panel.rs) ·
Tests: [`tests/status_and_loading.rs`](../../crates/archipel-kit/tests/status_and_loading.rs)
