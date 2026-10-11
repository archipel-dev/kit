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
| Surfaces | `frame`, `island`, `popup`, `popup_border`, `field`, `notification`, `tooltip` |
| Text | `text`, `text_strong`, `text_secondary`, `text_placeholder`, `text_disabled`, `text_on_accent` |
| Interaction | `hover`, `pressed`, `selection`, `accent`, `focus_ring`, `border`, `separator` |
| Semantic | `error`, `warning`, `success`, `link`, `banner_*` |
| Developer tools | `run`, `stop`, `vcs_modified`, `vcs_added`, `vcs_untracked`, `vcs_ignored` |

The test suite checks WCAG contrast of text on islands (7:1) and secondary text (4.5:1) for
the built-in themes; keep custom themes readable too.

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
