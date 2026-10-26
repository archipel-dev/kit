# Archipel Kit documentation

- [Getting started](getting-started.md): add the crate, install the theme, build a panel.
- [Theming](theming.md): colors, tokens, dark and light, custom themes.
- [Testing](testing.md): behavior, accessibility and snapshot tests for your own UI.
- [Cursors](cursors.md): which cursor shows where, and how parents override it.
- [Icon kit](icons/README.md): 113 designed glyphs, [78 language icons](icons/languages.md), [code symbols](icons/symbols.md), their [catalog](icons/catalog.md), [design rules](icons/design.md) and [review](icons/review.md).
- [Terminal UI](tui/README.md): the same theme, icons and components in the terminal (ratatui).
- [Behaviours](behaviours.md): tooltips, context menus, cursors and drag and drop on any widget with `.with(...)`.
- [Action system](action-system.md): IntelliJ-style actions, presentations, groups, keymaps and host services.
- [Building a component](building-components.md): from parts, with slots and behaviours, proven by conformance and the audit.
- [Contributing a component](contributing.md): the checklist every component meets.
- [Design rules](design-rules.md): the Islands look and feel every component follows, and how to measure a new one against Android Studio ([tools](../tools/reference/README.md)).
- [Desktop component map](component-map.md): reusable families, Android Studio reference
  behavior, interaction invariants and verification.

## Components

| Component | | Use it for |
|---|---|---|
| [Button](components/button.md) | <img src="components/img/button.dark.png" width="160"> | Actions: primary, secondary, ghost, danger. |
| [ActionButton](components/action-button.md) | <img src="components/img/action_button.dark.png" width="160"> | Toolbar actions and toggles. |
| [JBTextField](components/jb-text-field.md) | <img src="components/img/jb_text_field.dark.png" width="160"> | Text input, search, validation. |
| [JBCheckBox, JBRadioButton, OnOffButton](components/choice.md) | <img src="components/img/choice.dark.png" width="160"> | Booleans and one-of-many choices. |
| [ComboBox](components/combo-box.md) | <img src="components/img/combo_box.dark.png" width="100"> | Picking one value from a long list. |
| [SegmentedButton](components/segmented-button.md) | <img src="components/img/segmented_button.dark.png" width="140"> | Switching views. |
| [JBTabs](components/jb-tabs.md) | <img src="components/img/jb_tabs.dark.png" width="160"> | Editor tabs. |
| [KeyValueTable](components/key-value-table.md) | <img src="components/img/key_value.dark.png" width="160"> | Query parameters, headers, form fields, environment variables. |
| [CodeView, CodeEditor](components/code.md) | <img src="components/img/code_view.dark.png" width="160"> | Read-only and editable code with highlighting and find. |
| [JBList, Tree, JBTable](components/collections.md) | <img src="components/img/jb_table.dark.png" width="160"> | Large lists, trees and tables, virtualized. |
| [TreeTable](components/tree-table.md) | <img src="components/img/tree_table.dark.png" width="160"> | A tree whose rows have columns, with inline cell editing. |
| [SpeedSearch](components/speed-search.md) | <img src="components/img/speed_search.dark.png" width="160"> | Type to jump to or filter rows of lists, trees and tables. |
| [ColoredListCellRenderer](components/colored-list-cell-renderer.md) | <img src="components/img/colored_list_cell_renderer.dark.png" width="160"> | Lists and trees. |
| [InlineBanner](components/inline-banner.md) | <img src="components/img/inline_banner.dark.png" width="160"> | Inline messages with actions. |
| [Notification](components/notification.md) | <img src="components/img/notification.dark.png" width="160"> | Notifications. |
| [AsyncProcessIcon, ProgressBar](components/progress.md) | <img src="components/img/progress.dark.png" width="160"> | Work in progress. |
| [JBLoadingPanel](components/loading-panel.md) | <img src="components/img/loading_panel.dark.png" width="160"> | Content that is loading, behind a veil after a start delay. |
| [CollapsiblePanel](components/collapsible-panel.md) | <img src="components/img/collapsible_panel.dark.png" width="160"> | A titled group that shows and hides its content. |
| [StatusText](components/status-text.md) | <img src="components/img/status_text.dark.png" width="140"> | Nothing to show yet. |
| [Counter, Tag, Kbd, Avatar](components/badges.md) | <img src="components/img/badges.dark.png" width="140"> | Inline labels. |
| [Histogram, Sparkline](components/chart.md) | <img src="components/img/chart.dark.png" width="160"> | Value distributions, top-N counts, inline trends. |
| [Form, FieldLabel](components/form.md) | <img src="components/img/form.dark.png" width="160"> | Settings pages: rows, comments, groups. |
| [SimpleToolWindowPanel, TitledSeparator, PropertyGrid](components/simple-tool-window-panel.md) | <img src="components/img/simple_tool_window_panel.dark.png" width="160"> | Workbench layout and details. |
| [StatusBarWidget](components/status-bar-widget.md) | <img src="components/img/status_bar_widget.dark.png" width="160"> | Status bar and toolbar widgets. |
| [Splitter](components/splitter.md) | <img src="components/img/splitter.dark.png" width="140"> | Resizable panes, side by side or stacked. |
| [MainToolbar](components/window.md) | <img src="components/img/main_toolbar.dark.png" width="160"> | Integrated native title area and application actions. |
| [ToolWindowPane](components/tool-window-pane.md) | <img src="components/img/tool_window_pane_4.dark.png" width="160"> | Shared docked island shell with caller-owned layout. |
| [ActionMenuItem, ListSeparator, ActionLink](components/action-menu-item.md) | <img src="components/img/action_menu_item.dark.png" width="140"> | Menus and inline actions. |
| [HelpTooltip](components/help-tooltip.md) | <img src="components/img/help_tooltip.dark.png" width="140"> | Naming controls, shortcuts, why something is disabled. |
| [DialogWrapper](components/dialog-wrapper.md) | <img src="components/img/dialog_wrapper.dark.png" width="160"> | Decisions and short forms. |
| [Icons](components/icons.md) | <img src="icons/img/actual-size.dark.png" width="160"> | Glyphs and file-type icons. |

Every image on these pages is a snapshot test: it is rendered by `cargo test` and compared
pixel by pixel, so the docs always show what the code draws. Every code block on the
component pages is compiled and run as a doctest.

Try everything live in the gallery: `cargo run -p archipel-kit-gallery`.
