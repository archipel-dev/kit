# Desktop component map and interaction contract

Archipel Kit is a general desktop component library with the feel of the Islands UI in
Android Studio and IntelliJ. Applications supply their content, actions and state; the kit
supplies the surfaces, geometry, focus rules and interaction. The existing Archipel icon
artwork stays unchanged.

This map was established by inspecting the installed Android Studio Islands Dark UI at
100% zoom through native computer use: main window, settings, project tree, pickers and
editor. Values are logical points, not Retina screenshot pixels. It is a clean-room
behavioral reference; no third-party UI code or artwork is copied.

## Reusable families

| Family | Kit components / parts | Reference surface | Required interaction |
|---|---|---|---|
| Window chrome | IdeFrameDecorator, MainToolbar | Main title area and toolbar | A single themed title/toolbar area; native window controls; drag only the empty background; double-click background maximizes/restores; toolbar controls keep their normal input. |
| App layout | ToolWindowPane, SimpleToolWindowPanel, Splitter, Surface::Island | Main window and docked panels | Caller-owned visibility/sizes; consistent outer and inter-island gaps; bounded resizing and collapse; preserve central content in a small window. |
| Actions | Button, ActionButton, ActionLink | Settings footer, action bars | Press/release activation; named icon actions; disabled actions inert; Enter/Space; focus indication; default dialog action only when no child consumes Enter. |
| Text entry | JBTextField, JBIntSpinner | Settings fields and numeric controls | Caret, selection, clipboard and undo; single-line submission versus multi-line Enter; Tab leaves ordinary fields; visible validation; no data mutation while disabled. |
| Choices | JBCheckBox, JBRadioButton, OnOffButton | Settings | Checked, unchecked and mixed states; label is part of the target; selection/disabled states announced; keyboard activation. |
| Pickers | ComboBox, JBPopup, ActionMenuItem | Theme and project pickers | Open from keyboard; highlighted option distinct from committed value; arrows/Home/End; Enter commits; Escape cancels and restores focus; long lists scroll; changing data never indexes stale selection. |
| View navigation | SegmentedButton, JBTabs | Connected controls and editor tabs | Named selected states; arrow focus moves once; close and middle-click report events; overflow must remain reachable; closing the final tab has a defined empty state. |
| Lists and trees | ColoredListCellRenderer, JBList, Tree, TreeNode | Settings tree and project browser | Active/inactive selection; Up/Down/Home/End/Page; expand/collapse by chevron, double-click and Left/Right; focus retained; loading does not block navigation; virtualization preserves identity. |
| Data grids | JBTable, TreeTable, ColumnInfo, Cell, PropertyGrid | Structured data and settings tables | Sort, resize and reorder headers; row selection; F2/Enter edit; Enter commits, Escape cancels; clicks on children do not also activate the row. |
| Editable pairs | KeyValueTable, ValueCell | Repeated properties | Stable row identity; add/remove; per-row enablement; edit navigation; read-only remains readable and inert. |
| Text/code documents | CodeView, CodeEditor, Highlighter | Editor and text previews | Select/copy; caret and scrolling; find; folds; code indentation belongs to CodeEditor, ordinary multi-line JBTextField uses Tab to leave. |
| Menus and hints | ActionMenuItem, ListSeparator, HelpTooltip, behaviours | Context menu, submenus, hover hints | Submenu Right/Left; Escape unwinds the top layer first; disabled reasons visible; tooltips do not flicker; no hidden hover-only essential action. |
| Dialogs | DialogWrapper, Surface::Dialog | Settings/confirmation dialogs | Named modal; focus stays inside; sensible initial/default action; Escape dismisses; popup Escape and Enter consumed before dialog shortcuts; narrow content wraps. |
| Status and feedback | StatusBarWidget, InlineBanner, Notification, AsyncProcessIcon, ProgressBar | Status bar and notifications | Status items are named, keyboard reachable actions; messages keep semantic tone; dismiss/secondary actions work; progress exposes value; animations stop when invisible. |
| Inline metadata | Tag, Counter, Kbd, Avatar | Badges, counts and shortcuts | Same density, tones and fonts as adjacent content; descriptive accessible text; decorative parts do not become extra Tab stops. |
| Charts and empty content | Histogram, Sparkline, StatusText | General desktop data surfaces | Use the same surfaces, focus and semantic colors; selectable plots use keys; read-only plots carry a value summary; degenerate data does not panic. |
| Foundation | Theme, tokens, State, SimpleColoredComponent, Slot, Surface | All reference surfaces | One mapping for color/size/meaning; per-context state; no app policy; no custom glyph substitutions. |

Charts, HTTP key/value editors, avatars and some tag compositions have no direct reference
in the inspected Android Studio surfaces. They inherit the same surface, padding, type,
focus, disabled and popup rules, rather than pretending an exact reference was observed.

## Measured behavior

### Islands

Observed in Android Studio Rabbit 1 (2026.2.1, Islands Dark, 100% scale) by driving it with
pointer and keyboard and recording the screen at 60 fps. The kit reproduces each point.

| Situation | Android Studio | Kit |
|---|---|---|
| Tool window header actions at rest | Hidden; only the title (`Project ▾`) shows. | `SimpleToolWindowPanel::hover_actions(true)` |
| Pointer anywhere over the island, or the tool window active (focus inside) | Actions show at once, no fade-in. | Same; focus inside the island counts as active. |
| Pointer leaves, focus elsewhere | Actions stay ~200 ms, then fade out in ~60-70 ms. | `tokens::motion::ISLAND_ACTIONS_HIDE_DELAY`, `ISLAND_ACTIONS_FADE_OUT` |
| Hidden actions | Take no space; the title can use the whole header. | Same. |
| Narrow island, actions shown | The title disappears whole (no ellipsis). Actions stay right-anchored and are clipped at the island's left edge until only Hide remains. | Same. |
| Island minimum width | About 38 pt; tree labels are clipped hard at the edge. | `Splitter` minimum sizes |
| Gap between islands | No line; the resize cursor is the only feedback. | `Splitter::line(false)` |

### Popups and menus

| Situation | Android Studio | Kit |
|---|---|---|
| Popup surface | `#26282C`, 1 pt border `#33353B`, 8 pt padding top and bottom | `Surface::Popup`, `tokens::popup::PADDING` |
| Items | 24 pt, touching; pill inset 8 pt, radius 4, `#2A4371`; icon 8 pt inside the pill; text 4 pt after the icon; shortcut ends 8 pt before the pill's end | `ActionMenuItem`, `tokens::popup` |
| Separator | 9 pt row, 1 pt line in the border color, inset like pills | `ListSeparator::new()` |
| Group title | "Open Projects": semibold 13 pt, `#73767C`, aligned with icons, 24 pt row, not selectable | `ListSeparator::with_text` |
| Clear button in fields | Gray 8 pt cross (`#676B71` / `#B0B4C3`) centered 15 pt from the right edge; under the pointer a 16 pt round background (`#353637` / `#EBEBED`) and a darker cross | `JBTextField::clearable`, `Colors::clear_*`, `size::CLEAR_*` |
| Settings search | Filters the tree as you type and dims everything on the page that doesn't match; the match gets an orange outline | Not in the kit yet: app behavior |
| Two-line item | 20 pt icon, label, path below in `#73767C`, 48 pt | `ActionMenuItem::description` |
| Icon column | Reserved only when an item of the menu has an icon or a check | `ActionMenuItem` |
| Pointer and keys | One highlight, shared; the first pointer move after opening is ignored; it stays on the last item when the pointer leaves; arrows continue from it and skip headers and separators | `ActionMenuItem` |
| List popup keys | Up/Down skip separators and disabled rows and wrap; Home/End, PageUp/PageDown; Enter chooses (opens a sub-step instead when the row has one); Right opens, Left goes back; Esc goes back one level (clears speed search first); Shift+Esc closes the chain; mnemonics choose; typing filters | IntelliJ spec §5.1; `ListPopup`, `ActionPopupMenu`, tests/list_popup.rs |
| List popup pointer | Hover selects (first move after opening ignored); submenu after 250 ms on hover; auto-selection after 500 ms idle; click chooses on release; chevron press opens without choosing; press outside closes and the popup cannot reopen for 200 ms; moving toward an open submenu does not retarget | IntelliJ spec §5.2; `tokens::motion::POPUP_*` |
| List popup values kept from the measurement | Selection pill radius 4 (spec `Popup.Selection.arc` 8), minimum width 180 (spec 170), 24 pt rows, 8 pt padding, 9 pt separator rows | `tokens::popup` |
| Mnemonic hint | `Popup.mnemonicForeground` Gray.x78 dark / Gray.x99 light, before the icon column (not measured on Android Studio) | `Colors::popup_mnemonic` |
| Action button | 22×22, fires on release inside (primary only), hover suppressed while a press started elsewhere, not in the Tab order without a screen reader, toggled = pressed fill (`#FFFFFF29` / `#00000020`), hover `#FFFFFF17` / `#00000012`; radius stays 4 (6 at 30 pt) rather than `Button.arc` 8 | IntelliJ spec §1; `ActionButton` |
| Action toolbar | 22 pt buttons 4 pt apart, 2 pt border (5, 7 decorated), separators 1 pt × 20 pt, drop-downs open list popups with the first row selected, overflow in a "More actions" menu | IntelliJ spec §1.4, §1.5; `ActionToolbar`, `tokens::action_toolbar` |
| Main toolbar | Zones left / center (centered in the window) / right; 30 pt buttons with 20 pt icons; split run button: 26 pt arrow part, 3 pt separator margins, 20 pt line; drop-downs at most 350 pt; 40 pt title pane buttons; right click opens the customize menu; arrows move focus | IntelliJ spec §14; `MainToolbar::show_zones`, `ToolbarSplitButton` |
| Action link | Link color, underline on hover only, hand cursor, Enter/Space, focus ring, external-link icon, drop-down variant; one blue for hover/pressed/visited (the spec gives no other values) | IntelliJ spec §13.6; `ActionLink` |

### Editor tabs and headers

| Situation | Android Studio | Kit |
|---|---|---|
| Island header (tool window title bar, editor tab bar) | 40 pt high; Android Studio draws a 1 pt line below in the frame color, edge to edge, only under tool windows with content tabs. The suite draws it under every island header (ToolWindowPane tool windows, `JBTabs`) | `tokens::size::HEADER_HEIGHT`, `SimpleToolWindowPanel::header_separator`, `JBTabs::separator` |
| Header actions | 22 pt icon buttons, radius 4, 4 pt apart, the last 10 pt from the island's edge; group lines 20 pt in the separator color | `ActionButton`, `SeparatorComponent::group`, `HEADER_ACTIONS_INSET` |
| View switcher ("Project ▾") | Title + chevron, no hover fill; click opens the views right below the header, content-wide, current one highlighted, no icon column | `SimpleToolWindowPanel::views` |
| Content tabs (Problems: File, Project Errors) | 22 pt after the title, 10 pt padding and gap; selected: blue while active, gray outline otherwise | `SimpleToolWindowPanel::tabs` |
| Selected tab | 28 pt pill, radius 6, fill `#233558` / light `#E3EBFE`, 1 pt border `#2E4D89` / `#A7C5FF`, text at full strength | `JBTabs` |
| Other tabs | No fill; title `#AFB1B6` / light `#515151`; on hover a `hover` pill and full-strength title | `Colors::tab_text` |
| Close button | Only on the selected and the hovered tab; its room is always kept; small cross; round hover behind it | `tokens::tab` |
| Spacing | 8 pt, icon, 4 pt, title, 4 pt, close 16 pt, 4 pt; 6 pt between pills | `tokens::tab` |
| Strip inset | The first pill starts 6 pt from the island's left edge, centered in the 40 pt header; when the tabs scroll, the inset scrolls away and tabs are cut at the island's edge | `tab::STRIP_INSET` |
| Overflow | Tabs that don't fit scroll sideways: the wheel (vertical too), Shift+wheel and trackpad swipes scroll the strip; tabs are cut at the ends of the scrolling area and fade out over ~10 pt there (Android Studio also fades a fully shown last tab at the end; the kit fades only where tabs are cut off) | `JBTabs`, `tab::EDGE_FADE` |
| Strip scroll bar | Only while the pointer is over an overflowing strip, or scrolling: a 2 pt thumb 1 pt below the island's top edge, track across the island inside its rounded corners (~5 pt), length viewport/content; `#434344` / light `#ABABAB` on the island. Under the pointer and while dragged 4 pt and `#535455` / `#7F7F7F`; draggable. Appears within ~2 frames; stays ~0.2 s after the pointer leaves, then fades in ~0.1 s | `tab::SCROLL_*`, `Colors::tab_scroll_thumb`, `motion::TAB_SCROLL_*` |
| Strip buttons | At the right end, 22 pt hover pills, 4 pt apart, the last 4 pt from the island's right edge: "Show hidden tabs" (chevron down, only while tabs don't fit) and "More actions" (⋮, always). The scrolling area ends 9 pt before the first | `tab::BUTTON_GAP`, `BUTTONS_INSET`, `STRIP_BUTTONS_GAP`, `JBTabs::more_menu` |
| Hidden tabs list | Opens right below the header at the chevron (moved left to stay on screen), lists only the tabs out of view with their icons, first row highlighted; a pick selects the tab and scrolls it into view (Android Studio rows are 26 pt with a close cross; the kit uses its 24 pt menu rows) | `JBTabs` with `JBPopup`, `ActionMenuItem` |
| More actions menu | A menu below ⋮ (Recent Files, Close All Tabs, … in Android Studio) | `JBTabs::more_menu` |

### Trees and lists

| Situation | Android Studio | Kit |
|---|---|---|
| Row | 24 pt; selection pill inset 12 pt from the island's sides, radius 4 | `ColoredListCellRenderer`, `tokens::size::ROW_INSET` |
| Levels | Expander column (16 pt) starts at the pill's edge and moves 19 pt per level; icon 5 pt after it; text 4 pt after the icon | `TREE_INDENT`, `CHEVRON_GAP` |
| Selection | Focused `#2A4371` / light `#D0DFFE`; without focus `#33353B` / light `#E9EAEE` | `Colors::selection`, `selection_inactive` |
| Hover | No fill in trees and lists (popup lists highlight like menus) | `State::row_fill` |

### Tool window stripes

| Situation | Android Studio | Kit |
|---|---|---|
| Stripe | 38 pt on the frame beside the islands (islands start at 42 pt); top and bottom groups | `ToolWindowToolbar`, `tokens::stripe` |
| Button | 30 pt, radius 6, 20 pt icon, 40 pt pitch; first button 3 pt below the island's top | `SquareStripeButton` |
| States | Open: pressed fill (~16 % white); active (focus in the tool window): accent fill, white icon; hover: `hover` fill; tooltip beside it at once | `SquareStripeButton::open`, `active` |

### Form controls (Settings dialog)

| Situation | Android Studio | Kit |
|---|---|---|
| Buttons | 28 pt, at least 72 pt wide, radius 4, 12 pt apart; **no hover or press change** | `Button` |
| Focus | The 1 pt border becomes 2 pt in `#3871E1` (both themes), one of them outside; filled buttons get an outer 2 pt ring with a 1 pt gap | `parts::focus_ring`, `Colors::focus_ring` |
| Checkbox / radio | 16 pt, radius 3; unchecked `#2B2D30` / white with `#5A5D63` / `#A8ADBD` border; checked `#3574F0`; no hover change; focus: 2 pt border in `#3574F0` | `Colors::control`, `control_selected`, `control_border` |
| Disabled borders | `#33353B` / light `#DDDFE4` | `Colors::border_disabled` |
| Combo box list | Below the field, at least as wide, current value highlighted, groups separated by lines, chevron stays down | `ComboBox::separators` |
| Fields and combo boxes | 28 pt, `#40434A` / `#D1D3D9` border; combo fill `#26282C` / white | `JBTextField`, `ComboBox` |
| Integer field | A 28 pt text field, six digits wide; out-of-range input gets the field's error outline and message, the value keeps its last valid number | `JBIntSpinner` |
| Drag and drop | The dragged item leaves a dimmed original and follows the pointer as a translucent card; targets that take it are outlined in the focus color, the one under the pointer gets the 2 pt focus ring | `behaviour::DragSource` on `ColoredListCellRenderer`, `DropZone`, `DropTarget` |
| Editable combo box | A text field with the combo box's chevron (centered 16 pt from the right edge, as on the non-editable one); the list hangs below the field, as wide as it | `JBTextField::suggestions` |
| Checkbox label | Starts 8 pt after the box (radio the same); radios in a row 16 pt apart | `size::CHECK_LABEL_GAP`, `FORM_CONTROL_GAP` |
| Disabled combo box | No fill, `border_disabled` border, dimmed text (not a faded copy) | `ComboBox::enabled(false)` |
| Settings rows | Checkbox rows 29 pt apart; fields 9 pt from a box or field above and below; label 6 pt before its field | `Form`, `size::FORM_*` |
| Comments | 12 pt in `#73767C` (both themes), 16 pt lines, aligned with the label; first line's cap 6 pt below the row; a comment adds 7 pt + 16 per line. Inline comment 12 pt after the label, same baseline | `JBCheckBox::comment`, `inline_comment`, `size::COMMENT_*` |
| Group title | Regular weight, text color; 1 pt line `#33353B` / `#E9EAEE` from 6 pt after the title, 4 pt above its baseline, to the right edge; 20 pt extra above, none for the first group; rows indented 24 pt | `Form::group`, `TitledSeparator`, `Colors::group_separator` |
| Collapsible group title | The group title after a 16 pt chevron (right closed, down open) and a 4 pt gap, in the text color; the title and chevron toggle it | `FormUi::collapsible`, `TitledSeparator::collapsible` |

### Tooltips and status bar

| Situation | Android Studio | Kit |
|---|---|---|
| Tooltip look | `#33353B`, 36 pt for one line, 16 pt at the sides, text `#D1D3D9`, shortcut in `text_secondary` 9 pt after it | `Surface::Tooltip`, `tokens::size::TOOLTIP_*` |
| Tooltip delay | About 0.5 s on toolbar buttons; stripe buttons name themselves at once, beside the button | egui's 0.5 s; `HelpTooltip::immediate`, `HelpTooltip::align` |
| Tooltip linger | Help tooltips stay about 0.5 s after the pointer leaves, so links in them can be reached | Not reproduced: kit tooltips have no links |
| Status bar | 30 pt on the frame, 8 pt from the left; items 21 pt hover pills, text `text_secondary`, full strength on hover | `ToolWindowPane::status`, `StatusBarWidget` |
| Scroll bars | A 7 pt thumb, 2 pt from the edge, visible at rest (`#3C3D3E` on a dark island), brighter under the pointer (`#565657`); a faint track appears on hover | `tokens::scroll` (no track) |
| Editor banner | 39 pt, inset 6 pt in the editor island under the tabs; 16 pt before the icon, 10 pt to the text; actions are links 22 pt apart, underlined on hover; 20 pt at the end; info `#233558` / `#2E4D89` | `InlineBanner`, `tokens::size::BANNER_*` |
| Notifications | 370 pt card, 12 pt padding, icon on the title line, semibold title, body in the text color, links below; dismiss button in the corner on hover | `Notification`, `tokens::size::NOTIFICATION_*` |
| Navigation path | Crumbs with icons, 4 pt padding, dimmed chevrons between | `BreadcrumbsComponent` |

## Fixed invariants

- Body font: Inter 13 pt. Code: the bundled mono font. Existing icon artwork is fixed.
- Kit colors come from the active context's Theme; an explicit kit theme controls egui's
  appearance too. App/popup colors must not drift because the OS uses another appearance.
- Rows are 24 pt and standard controls 28 pt. Islands use the kit radius and 4 pt gaps.
  Window and toolbar geometry is expressed in tokens, including native-control clearances.
- A click or key has one owner. Popups, inline editors and nested actions consume handled
  keys before parent activation. Selection and activation are distinct events.
- Every interactive widget has a name, enabled state and focus path. Pointer-only
  decoration does not steal focus or become an unnamed target.
- Invalid, empty, changing and huge data are normal inputs. Clamp geometry/indices, keep
  caller data intact during invalid edits, and never panic on user input.
- Native main windows may use integrated chrome. Modal windows can retain normal native
  chrome. Window creation remains the application's responsibility; the kit exposes
  viewport configuration and toolbar parts without depending on eframe.
- Cross-platform behavior must be explicit: macOS keeps native traffic lights in a
  transparent full-size title area; other platforms can use native decorations until
  their custom window-control path is verified. Do not claim untested platform parity.

## Backlog and proof

| Work | Tracking | Proof / next acceptance |
|---|---|---|
| Picker, field, tree and segment corrections | [#13](https://github.com/archipel-dev/kit/issues/13), [#14](https://github.com/archipel-dev/kit/issues/14), [#15](https://github.com/archipel-dev/kit/issues/15), [#16](https://github.com/archipel-dev/kit/issues/16) | Merged in PR #27 with behavior/a11y regressions, both-theme snapshots and gallery demonstrations. |
| Histogram/Sparkline | [PR #12](https://github.com/archipel-dev/kit/pull/12) | Merged in PR #12 after count-total and caller-tooltip fixes; CI passed. |
| Streaming, rich cells and numbers | [#20](https://github.com/archipel-dev/kit/issues/20) | Follow-tail releases on user scroll; jump-to-latest; slot cells preserve virtualization; bounded numeric stepping and invalid state. |
| Shared island shell | [#24](https://github.com/archipel-dev/kit/issues/24) | IdeFrameDecorator/MainToolbar and docked ToolWindowPane implemented with caller-owned serializable state, keyboard splitters, hide/maximize/restore, both-theme snapshots and small-window audits. Native macOS traffic lights centered in the full toolbar band and tested live. Tool-window stripes, multiple tools/floating modes and SDK/app migrations remain follow-ups. |
| Independent gallery pages | [#25](https://github.com/archipel-dev/kit/issues/25) | Each page owns its state and rendering; extraction preserved snapshots. The new Workbench entry intentionally extends the gallery navigation. |

Completion requires the contribution checklist, fmt/clippy/workspace tests, inspecting
changed docs snapshots, and native interaction against the actual gallery. Snapshots are
not a substitute for verifying focus, nested popups, clipboard and window behavior.
