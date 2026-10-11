# Theming

## Themes

A `Theme` is plain data: workbench `colors`, `editor` colors, a name and whether it is dark.
`Theme::dark()` and `Theme::light()` are built in; `install` or `Theme::apply` stores a theme
in the `egui::Context`, and components read it with `Theme::colors(ctx)`. Two contexts (two
windows, two tests running in parallel) can use different themes at the same time.

```rust
use archipel_kit::{Theme, egui};

fn use_sunset(ctx: &egui::Context) {
    let mut theme = Theme::dark();
    theme.name = "Sunset".into();
    theme.colors.accent = egui::Color32::from_rgb(0xE0, 0x88, 0x55);
    theme.apply(ctx);
}
```

Use the semantic colors in your own widgets so they follow the theme:

| Group | Colors |
|---|---|
| Surfaces | `frame`, `island`, `popup`, `popup_border`, `field`, `notification`, `tooltip` (`Surface::Chrome` paints the workbench frame, toolbars and the status bar in `frame`) |
| Text | `text`, `text_strong`, `text_secondary`, `text_placeholder`, `text_disabled`, `text_on_accent` |
| Interaction | `hover`, `pressed`, `selection`, `accent`, `focus_ring`, `border`, `separator` |
| Semantic | `error`, `warning`, `success`, `link`, `banner_*` |
| Developer tools | `run`, `stop`, `vcs_modified`, `vcs_added`, `vcs_untracked`, `vcs_ignored` |
| Data | `categorical`, `sequential` (see [Data colors](#data-colors)) |

The test suite checks WCAG contrast of text on islands (7:1) and secondary text (4.5:1) for
the built-in themes; keep custom themes readable too.

## Data colors

Data-heavy tools (JSON trees, document tables, response viewers, charts) take their colors
from the theme too, never from hard-coded values.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="components/img/data-colors.dark.png">
  <img alt="Value kinds, the chart palette, the sequential ramp and method and status tags" src="components/img/data-colors.light.png" width="360">
</picture>

### Typed values

`ValueKind` is the type of a value in a data view. `kind.color(&theme)` or
`kind.resolve(ctx)` returns the editor syntax color, so values match the code editor.
It is separate from `Tone` because the colors live in `EditorColors`, not `Colors`.

| `ValueKind` | `EditorColors` field |
|---|---|
| `String` | `string` |
| `Number` | `number` |
| `Boolean` | `keyword` |
| `Null` | `comment` |
| `Date` | `constant` |
| `Id` (ObjectId) | `type_name` |
| `Binary` | `annotation` |
| `Key` | `property` |
| `Punctuation` | `punctuation` |

The syntax fields are new in `EditorColors`: `string`, `number`, `keyword`, `constant`,
`type_name`, `annotation`, `property`, `punctuation`, `comment`. Custom themes must set them;
the tests require 3:1 against the editor background for the built-in themes.

```rust
use archipel_kit::{ValueKind, egui};

fn json_number(ui: &mut egui::Ui, value: f64) {
    let color = ValueKind::Number.resolve(ui.ctx());
    ui.label(egui::RichText::new(value.to_string()).monospace().color(color));
}
```

### Charts

`Colors::categorical` (also `Theme::chart(index)` and `Colors::chart(index)`, wrapping after
8) holds eight series colors. Each has at least 3:1 contrast against `island` and a
distance of at least 60 (RGB) from every other series; a test checks both for the built-in
themes. Don't use color alone to tell series apart in dense charts: add labels or markers.

`Colors::sequential` is a five-stop ramp for heatmaps and histograms; `colors.ramp(t)` blends
it for `t` in 0..=1. Higher values are further from the island in luminance (brighter in
dark, darker in light), so "more" always stands out.

```rust
use archipel_kit::{Theme, egui};

fn bar(ui: &mut egui::Ui, series: usize, rect: egui::Rect, share: f32) {
    let colors = Theme::colors(ui.ctx());
    ui.painter().rect_filled(rect, 2.0, colors.chart(series));
    let _cell = colors.ramp(share);
}
```

### HTTP methods and statuses

`Tone::method("GET")` and `Tone::status(404)` return an ordinary `Tone`, for `Tag`, text and
icons. Methods: GET success, POST warning, PUT link, PATCH accent, DELETE error, HEAD and
OPTIONS secondary, anything else default (case-insensitive). Statuses: 1xx secondary, 2xx
success, 3xx link, 4xx warning, 5xx error, anything else muted.

## Tokens

`archipel_kit::tokens` holds the shared measurements, so custom widgets line up with the
components:

| Module | Examples |
|---|---|
| `size` | `ROW_HEIGHT` 24, `CONTROL_HEIGHT` 28, `ICON` 16, `CONTENT_INSET` 12, `HEADER_HEIGHT` 31 |
| `radius` | `ISLAND` 10, `POPUP` 8, `CONTROL` 4 |
| `typography` | `BODY` 13, `SMALL` 11, `semibold(ctx, size)`, `mono_bold(ctx, size)` |

## Where the colors come from

The built-in palettes are derived from public, Apache-2.0 licensed theme data (see
[`NOTICE.md`](../NOTICE.md)). No third-party code, icons or artwork are included.
