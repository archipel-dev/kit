# Migration to IntelliJ Platform names

Archipel Kit renamed its public API onto IntelliJ Platform names and replaced the `archipel_kit::components`
module with modules named after the IntelliJ Platform areas. There are no deprecated aliases: old names
and paths do not compile. This page is the mapping, meant for downstream tools (search and replace
the names, then fix the imports with the paths).

## Modules

| Module | Contents |
|---|---|
| `ui` | Button, ActionLink, JBTextField, JBIntSpinner, JBCheckBox, JBRadioButton, OnOffButton, SegmentedButton, JBList, Tree, JBTable, TreeTable, ColoredListCellRenderer, TitledSeparator, SeparatorComponent, InlineBanner, Status, AsyncProcessIcon, ProgressBar, StatusText, BreadcrumbsComponent, KeyValueTable, badges, charts, PropertyGrid, DropZone, Form, completion |
| `ui::tabs` | JBTabs, TabInfo, JBTabsResponse |
| `ui::popup` | JBPopup, ListSeparator |
| `openapi::ui` | ComboBox, Splitter, SplitterResponse, Orientation, Side, DialogWrapper, ExitCode |
| `ide` | HelpTooltip, HelpTooltipExt |
| `action_system` | ActionButton, ActionMenuItem |
| `wm` | ToolWindowPane, ToolWindowPaneState, ToolWindowAnchor, ToolWindowPlace, SquareStripeButton, ToolWindowToolbar, StatusBarWidget, MainToolbar, IdeFrameDecorator, ToolbarComboButton, SimpleToolWindowPanel |
| `notification` | Notification |
| `editor` | EditorComponent, the highlighters (formerly `components::code`), `editor::model` |
| `parts`, `behaviour`, `cursor`, `icons`, `theme`, `tokens`, `testing` | Unchanged, minus the moved items below |

## Renamed types

| Old name | New name | Old path | New path |
|---|---|---|---|
| `Banner` | `InlineBanner` | `archipel_kit::components::Banner` | `archipel_kit::ui::InlineBanner` |
| `Breadcrumbs` | `BreadcrumbsComponent` | `archipel_kit::components::Breadcrumbs` | `archipel_kit::ui::BreadcrumbsComponent` |
| `BreadcrumbsResponse` | `BreadcrumbsComponentResponse` | `archipel_kit::components::BreadcrumbsResponse` | `archipel_kit::ui::BreadcrumbsComponentResponse` |
| `Checkbox` | `JBCheckBox` | `archipel_kit::components::Checkbox` | `archipel_kit::ui::JBCheckBox` |
| `Column` | `ColumnInfo` | `archipel_kit::components::Column` | `archipel_kit::ui::ColumnInfo` |
| `ContextMenu` | `PopupHandler` | `archipel_kit::behaviour::ContextMenu` | `archipel_kit::behaviour::PopupHandler` |
| `Dialog` | `DialogWrapper` | `archipel_kit::components::Dialog` | `archipel_kit::openapi::ui::DialogWrapper` |
| `DialogAction` | `ExitCode` | `archipel_kit::components::DialogAction` | `archipel_kit::openapi::ui::ExitCode` |
| `Dropdown` | `ComboBox` | `archipel_kit::components::Dropdown` | `archipel_kit::openapi::ui::ComboBox` |
| `EmptyState` | `StatusText` | `archipel_kit::components::EmptyState` | `archipel_kit::ui::StatusText` |
| `IconButton` | `ActionButton` | `archipel_kit::components::IconButton` | `archipel_kit::action_system::ActionButton` |
| `Link` | `ActionLink` | `archipel_kit::components::Link` | `archipel_kit::ui::ActionLink` |
| `ListRow` | `ColoredListCellRenderer` | `archipel_kit::components::ListRow` | `archipel_kit::ui::ColoredListCellRenderer` |
| `MenuHeader` | `ListSeparator::with_text(title)` | `archipel_kit::components::MenuHeader` | `archipel_kit::ui::popup::ListSeparator` |
| `MenuItem` | `ActionMenuItem` | `archipel_kit::components::MenuItem` | `archipel_kit::action_system::ActionMenuItem` |
| `MenuSeparator` | `ListSeparator::new()` | `archipel_kit::components::MenuSeparator` | `archipel_kit::ui::popup::ListSeparator` |
| `NumberField` | `JBIntSpinner` | `archipel_kit::components::NumberField` | `archipel_kit::ui::JBIntSpinner` |
| `Panel` | `SimpleToolWindowPanel` | `archipel_kit::components::Panel` | `archipel_kit::wm::SimpleToolWindowPanel` |
| `Popover` | `JBPopup` | `archipel_kit::parts::Popover` | `archipel_kit::ui::popup::JBPopup` |
| `Radio` | `JBRadioButton` | `archipel_kit::components::Radio` | `archipel_kit::ui::JBRadioButton` |
| `RowContent` | `SimpleColoredComponent` | `archipel_kit::parts::RowContent` | `archipel_kit::parts::SimpleColoredComponent` |
| `RowResponse` | `CellRendererResponse` | `archipel_kit::components::RowResponse` | `archipel_kit::ui::CellRendererResponse` |
| `SectionHeader` | `TitledSeparator` | `archipel_kit::components::SectionHeader` | `archipel_kit::ui::TitledSeparator` |
| `SegmentedControl` | `SegmentedButton` | `archipel_kit::components::SegmentedControl` | `archipel_kit::ui::SegmentedButton` |
| `Separator` | `SeparatorComponent` | `archipel_kit::components::Separator` | `archipel_kit::ui::SeparatorComponent` |
| `Severity` | `Status` | `archipel_kit::components::Severity` | `archipel_kit::ui::Status` |
| `Spinner` | `AsyncProcessIcon` | `archipel_kit::components::Spinner` | `archipel_kit::ui::AsyncProcessIcon` |
| `SplitPane` | `Splitter` | `archipel_kit::components::SplitPane` | `archipel_kit::openapi::ui::Splitter` |
| `SplitResponse` | `SplitterResponse` | `archipel_kit::components::SplitResponse` | `archipel_kit::openapi::ui::SplitterResponse` |
| `StatusBarItem` | `StatusBarWidget` | `archipel_kit::components::StatusBarItem` | `archipel_kit::wm::StatusBarWidget` |
| `StripeButton` | `SquareStripeButton` | `archipel_kit::components::StripeButton` | `archipel_kit::wm::SquareStripeButton` |
| `Tab` | `TabInfo` | `archipel_kit::components::Tab` | `archipel_kit::ui::tabs::TabInfo` |
| `TabBar` | `JBTabs` | `archipel_kit::components::TabBar` | `archipel_kit::ui::tabs::JBTabs` |
| `TabBarResponse` | `JBTabsResponse` | `archipel_kit::components::TabBarResponse` | `archipel_kit::ui::tabs::JBTabsResponse` |
| `Table` | `JBTable` | `archipel_kit::components::Table` | `archipel_kit::ui::JBTable` |
| `TextField` | `JBTextField` | `archipel_kit::components::TextField` | `archipel_kit::ui::JBTextField` |
| `TextFieldOutput` | `JBTextFieldOutput` | `archipel_kit::components::TextFieldOutput` | `archipel_kit::ui::JBTextFieldOutput` |
| `Toast` | `Notification` | `archipel_kit::components::Toast` | `archipel_kit::notification::Notification` |
| `Toggle` | `OnOffButton` | `archipel_kit::components::Toggle` | `archipel_kit::ui::OnOffButton` |
| `ToolbarButton` | `ToolbarComboButton` | `archipel_kit::components::ToolbarButton` | `archipel_kit::wm::ToolbarComboButton` |
| `ToolStripe` | `ToolWindowToolbar` | `archipel_kit::components::ToolStripe` | `archipel_kit::wm::ToolWindowToolbar` |
| `Tooltip` | `HelpTooltip` | `archipel_kit::components::Tooltip` | `archipel_kit::ide::HelpTooltip` |
| `TooltipExt` | `HelpTooltipExt` | `archipel_kit::components::TooltipExt` | `archipel_kit::ide::HelpTooltipExt` |
| `TreeRow` | `TreeNode` | `archipel_kit::components::TreeRow` | `archipel_kit::ui::TreeNode` |
| `VirtualList` | `JBList` | `archipel_kit::components::VirtualList` | `archipel_kit::ui::JBList` |
| `WindowChrome` | `IdeFrameDecorator` | `archipel_kit::components::WindowChrome` | `archipel_kit::wm::IdeFrameDecorator` |
| `WindowToolbar` | `MainToolbar` | `archipel_kit::components::WindowToolbar` | `archipel_kit::wm::MainToolbar` |
| `Workbench` | `ToolWindowPane` | `archipel_kit::components::Workbench` | `archipel_kit::wm::ToolWindowPane` |
| `WorkbenchPlace` | `ToolWindowPlace` | `archipel_kit::components::WorkbenchPlace` | `archipel_kit::wm::ToolWindowPlace` |
| `WorkbenchRegion` | `ToolWindowAnchor` | `archipel_kit::components::WorkbenchRegion` | `archipel_kit::wm::ToolWindowAnchor` |
| `WorkbenchState` | `ToolWindowPaneState` | `archipel_kit::components::WorkbenchState` | `archipel_kit::wm::ToolWindowPaneState` |

`MenuSeparator` and `MenuHeader` became one type: `ListSeparator::new()` draws the line, `ListSeparator::with_text(title)` the group title (same rendering as before).

`ListRow`'s response `RowResponse` became `CellRendererResponse`; `Tab` became `TabInfo` and lives in `ui::tabs`.

## Same name, new path

These types keep their names; only the module changed (`archipel_kit::components::X` to the path below).

| Name | New path |
|---|---|
| `Button` | `archipel_kit::ui::Button` |
| `ButtonVariant` | `archipel_kit::ui::ButtonVariant` |
| `ProgressBar` | `archipel_kit::ui::ProgressBar` |
| `Tree` | `archipel_kit::ui::Tree` |
| `TreeTable` | `archipel_kit::ui::TreeTable` |
| `Cell` | `archipel_kit::ui::Cell` |
| `SortOrder` | `archipel_kit::ui::SortOrder` |
| `CollectionResponse` | `archipel_kit::ui::CollectionResponse` |
| `CellEdit` | `archipel_kit::ui::CellEdit` |
| `KeyValueTable` | `archipel_kit::ui::KeyValueTable` |
| `KeyValueRow` | `archipel_kit::ui::KeyValueRow` |
| `KeyValueEntry` | `archipel_kit::ui::KeyValueEntry` |
| `Change` | `archipel_kit::ui::Change` |
| `ValueCell` | `archipel_kit::ui::ValueCell` |
| `KeyValueTableResponse` | `archipel_kit::ui::KeyValueTableResponse` |
| `Counter` | `archipel_kit::ui::Counter` |
| `Tag` | `archipel_kit::ui::Tag` |
| `Kbd` | `archipel_kit::ui::Kbd` |
| `Avatar` | `archipel_kit::ui::Avatar` |
| `PropertyGrid` | `archipel_kit::ui::PropertyGrid` |
| `Histogram` | `archipel_kit::ui::Histogram` |
| `HistogramResponse` | `archipel_kit::ui::HistogramResponse` |
| `Sparkline` | `archipel_kit::ui::Sparkline` |
| `Bucket` | `archipel_kit::ui::Bucket` |
| `ChartColor` | `archipel_kit::ui::ChartColor` |
| `DropZone` | `archipel_kit::ui::DropZone` |
| `DropZoneResponse` | `archipel_kit::ui::DropZoneResponse` |
| `MessageResponse` | `archipel_kit::ui::MessageResponse` |
| `Form` | `archipel_kit::ui::Form` |
| `FormUi` | `archipel_kit::ui::FormUi` |
| `FieldLabel` | `archipel_kit::ui::FieldLabel` |
| `Completer` | `archipel_kit::ui::Completer` |
| `Completion` | `archipel_kit::ui::Completion` |
| `Completions` | `archipel_kit::ui::Completions` |
| `Integer` | `archipel_kit::ui::Integer` |
| `initials` | `archipel_kit::ui::initials` |
| `Orientation` | `archipel_kit::openapi::ui::Orientation` |
| `Side` | `archipel_kit::openapi::ui::Side` |
| `CodeView` | `archipel_kit::editor::EditorComponent` with `.viewer(true)` on a `model::Editor` |
| `CodeViewResponse` | `archipel_kit::editor::EditorComponentResponse` |
| `CodeEditor` | `archipel_kit::editor::EditorComponent` on a `model::Editor` |
| `CodeEditorOutput` | `archipel_kit::editor::EditorComponentResponse` (`selection` is still the byte range) |
| `Highlighter` | `archipel_kit::editor::Highlighter` |
| `TokenKind` | `archipel_kit::editor::TokenKind` |
| `Span` | `archipel_kit::editor::Span` |
| `PlainText` | `archipel_kit::editor::PlainText` |
| `Json` | `archipel_kit::editor::Json` |
| `ExtendedJson` | `archipel_kit::editor::ExtendedJson` |
| `JavaScript` | `archipel_kit::editor::JavaScript` |
| `GraphQl` | `archipel_kit::editor::GraphQl` |
| `Xml` | `archipel_kit::editor::Xml` |
| `TemplateHighlighter` | `archipel_kit::editor::TemplateHighlighter` |
| `TemplateToken` | `archipel_kit::editor::TemplateToken` |
| `TemplateAt` | `archipel_kit::editor::TemplateAt` |
| `template_at` | `archipel_kit::editor::template_at` |

## Other identifiers

Documentation pages and snapshot images were renamed after the new type names (for example
`docs/components/dropdown.md` is now `docs/components/combo-box.md`, `img/tab_bar.*.png` is `img/jb_tabs.*.png`).
AccessKit labels and visible strings are unchanged.

## CodeView and CodeEditor became EditorComponent

`CodeView` and `CodeEditor` are gone (no aliases); `EditorComponent` replaces both. It
borrows a `model::Editor` instead of a `&str` or `&mut String`: the caller owns the document,
the carets, the undo history, the folds and the settings.

| Before | Now |
|---|---|
| `CodeView::new(&text)` | `EditorComponent::new(&mut editor).viewer(true)` with `let mut editor = Editor::new(&text)`; `editor.set_text(..)` when the text changes (it replaces the `.revision(n)` call) |
| `CodeEditor::new(&mut text)` | `EditorComponent::new(&mut editor)`; read the text with `editor.document.text()` |
| `.folding(true)` (brackets) | `.bracket_folding(true)`; fold regions of your own: `editor.folding.add_region(..)` with `.folding(true)` |
| `.tab_width(n)`, `.use_tabs(b)`, `.auto_indent(b)`, `.bracket_pairing(b)` | `editor.settings.tab_size` and `indent_size`, `use_tab_character`, `smart_indent_on_enter`, `auto_pair_brackets` and `auto_pair_quotes` |
| `.scroll_to_line`, `.stick_to_bottom`, `.wrap`, `.line_numbers`, `.label`, `.placeholder`, `.completer`, `.enabled`, `.tab_indents`, `.height`, `.id`, `.highlighter` | Unchanged |
| `response.changed()` | `response.changed()` and `EditorComponentResponse::text_changed` |
| `CodeEditorOutput::selection` | `EditorComponentResponse::selection` |
| `CodeViewResponse::{line_count, visible_lines, laid_out_rows, matches}` | The same fields of `EditorComponentResponse` |
| <kbd>Ctrl</kbd>+<kbd>]</kbd> / <kbd>[</kbd> indent | Gone: they are the IntelliJ code block navigation. Use <kbd>Tab</kbd> and <kbd>Shift</kbd>+<kbd>Tab</kbd> |
| egui's undo and word movement | The model's: IntelliJ word stops, smart Home/End, caret-restoring undo |
