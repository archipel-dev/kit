# Archipel Kit documentation

- [Getting started](getting-started.md): add the crate, install the theme, build a panel.
- [Theming](theming.md): colors, tokens, dark and light, custom themes.
- [Testing](testing.md): behavior, accessibility and snapshot tests for your own UI.
- [Cursors](cursors.md): which cursor shows where, and how parents override it.
- [Icon kit](icons/README.md): 113 designed glyphs, [78 language icons](icons/languages.md), [code symbols](icons/symbols.md), their [catalog](icons/catalog.md), [design rules](icons/design.md) and [review](icons/review.md).
- [Terminal UI](tui/README.md): the same theme, icons and components in the terminal (ratatui).
- [Contributing a component](contributing.md): the checklist every component meets.

## Components

| Component | | Use it for |
|---|---|---|
| [Button](components/button.md) | <img src="components/img/button.dark.png" width="160"> | Actions: primary, secondary, ghost, danger. |
| [IconButton](components/icon-button.md) | <img src="components/img/icon_button.dark.png" width="160"> | Toolbar actions and toggles. |
| [TextField](components/text-field.md) | <img src="components/img/text_field.dark.png" width="160"> | Text input, search, validation. |
| [Checkbox, Radio, Toggle](components/choice.md) | <img src="components/img/choice.dark.png" width="160"> | Booleans and one-of-many choices. |
| [Dropdown](components/dropdown.md) | <img src="components/img/dropdown.dark.png" width="100"> | Picking one value from a long list. |
| [SegmentedControl](components/segmented-control.md) | <img src="components/img/segmented_control.dark.png" width="140"> | Switching views. |
| [TabBar](components/tab-bar.md) | <img src="components/img/tab_bar.dark.png" width="160"> | Editor tabs. |
| [VirtualList, Tree, Table](components/collections.md) | <img src="components/img/table.dark.png" width="160"> | Large lists, trees and tables, virtualized. |
| [ListRow](components/list-row.md) | <img src="components/img/list_row.dark.png" width="160"> | Lists and trees. |
| [Banner](components/banner.md) | <img src="components/img/banner.dark.png" width="160"> | Inline messages with actions. |
| [Toast](components/toast.md) | <img src="components/img/toast.dark.png" width="160"> | Notifications. |
| [Spinner, ProgressBar](components/progress.md) | <img src="components/img/progress.dark.png" width="160"> | Work in progress. |
| [EmptyState](components/empty-state.md) | <img src="components/img/empty_state.dark.png" width="140"> | Nothing to show yet. |
| [Counter, Tag, Kbd, Avatar](components/badges.md) | <img src="components/img/badges.dark.png" width="140"> | Inline labels. |
| [Panel, SectionHeader, PropertyGrid](components/panel.md) | <img src="components/img/panel.dark.png" width="160"> | Workbench layout and details. |
| [MenuItem, MenuSeparator, Link](components/menu-item.md) | <img src="components/img/menu_item.dark.png" width="140"> | Menus and inline actions. |
| [Tooltip](components/tooltip.md) | <img src="components/img/tooltip.dark.png" width="140"> | Naming controls, shortcuts, why something is disabled. |
| [Dialog](components/dialog.md) | <img src="components/img/dialog.dark.png" width="160"> | Decisions and short forms. |
| [Icons](components/icons.md) | <img src="icons/img/actual-size.dark.png" width="160"> | Glyphs and file-type icons. |

Every image on these pages is a snapshot test: it is rendered by `cargo test` and compared
pixel by pixel, so the docs always show what the code draws. Every code block on the
component pages is compiled and run as a doctest.

Try everything live in the gallery: `cargo run -p archipel-kit-gallery`.
