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
| Window chrome | WindowChrome, WindowToolbar | Main title area and toolbar | A single themed title/toolbar area; native window controls; drag only the empty background; double-click background maximizes/restores; toolbar controls keep their normal input. |
| App layout | Workbench, Panel, SplitPane, Surface::Island | Main window and docked panels | Caller-owned visibility/sizes; consistent outer and inter-island gaps; bounded resizing and collapse; preserve central content in a small window. |
| Actions | Button, IconButton, Link | Settings footer, action bars | Press/release activation; named icon actions; disabled actions inert; Enter/Space; focus indication; default dialog action only when no child consumes Enter. |
| Text entry | TextField, NumberField | Settings fields and numeric controls | Caret, selection, clipboard and undo; single-line submission versus multi-line Enter; Tab leaves ordinary fields; visible validation; no data mutation while disabled. |
| Choices | Checkbox, Radio, Toggle | Settings | Checked, unchecked and mixed states; label is part of the target; selection/disabled states announced; keyboard activation. |
| Pickers | Dropdown, Popover, MenuItem | Theme and project pickers | Open from keyboard; highlighted option distinct from committed value; arrows/Home/End; Enter commits; Escape cancels and restores focus; long lists scroll; changing data never indexes stale selection. |
| View navigation | SegmentedControl, TabBar | Connected controls and editor tabs | Named selected states; arrow focus moves once; close and middle-click report events; overflow must remain reachable; closing the final tab has a defined empty state. |
| Lists and trees | ListRow, VirtualList, Tree, TreeRow | Settings tree and project browser | Active/inactive selection; Up/Down/Home/End/Page; expand/collapse by chevron, double-click and Left/Right; focus retained; loading does not block navigation; virtualization preserves identity. |
| Data grids | Table, TreeTable, Column, Cell, PropertyGrid | Structured data and settings tables | Sort, resize and reorder headers; row selection; F2/Enter edit; Enter commits, Escape cancels; clicks on children do not also activate the row. |
| Editable pairs | KeyValueTable, ValueCell | Repeated properties | Stable row identity; add/remove; per-row enablement; edit navigation; read-only remains readable and inert. |
| Text/code documents | CodeView, CodeEditor, Highlighter | Editor and text previews | Select/copy; caret and scrolling; find; folds; code indentation belongs to CodeEditor, ordinary multi-line TextField uses Tab to leave. |
| Menus and hints | MenuItem, MenuSeparator, Tooltip, behaviours | Context menu, submenus, hover hints | Submenu Right/Left; Escape unwinds the top layer first; disabled reasons visible; tooltips do not flicker; no hidden hover-only essential action. |
| Dialogs | Dialog, Surface::Dialog | Settings/confirmation dialogs | Named modal; focus stays inside; sensible initial/default action; Escape dismisses; popup Escape and Enter consumed before dialog shortcuts; narrow content wraps. |
| Status and feedback | StatusBarItem, Banner, Toast, Spinner, ProgressBar | Status bar and notifications | Status items are named, keyboard reachable actions; messages keep semantic tone; dismiss/secondary actions work; progress exposes value; animations stop when invisible. |
| Inline metadata | Tag, Counter, Kbd, Avatar | Badges, counts and shortcuts | Same density, tones and fonts as adjacent content; descriptive accessible text; decorative parts do not become extra Tab stops. |
| Charts and empty content | Histogram, Sparkline, EmptyState | General desktop data surfaces | Use the same surfaces, focus and semantic colors; selectable plots use keys; read-only plots carry a value summary; degenerate data does not panic. |
| Foundation | Theme, tokens, State, RowContent, Slot, Surface | All reference surfaces | One mapping for color/size/meaning; per-context state; no app policy; no custom glyph substitutions. |

Charts, HTTP key/value editors, avatars and some tag compositions have no direct reference
in the inspected Android Studio surfaces. They inherit the same surface, padding, type,
focus, disabled and popup rules, rather than pretending an exact reference was observed.

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
| Picker, field, tree and segment corrections | [#13](https://github.com/archipel-dev/kit/issues/13), [#14](https://github.com/archipel-dev/kit/issues/14), [#15](https://github.com/archipel-dev/kit/issues/15), [#16](https://github.com/archipel-dev/kit/issues/16) | Behavior/a11y regressions and both-theme snapshots; gallery demonstrations. |
| Histogram/Sparkline | [PR #12](https://github.com/archipel-dev/kit/pull/12) | Fix count totals and caller-tooltip detection before merging; CI required. |
| Streaming, rich cells and numbers | [#20](https://github.com/archipel-dev/kit/issues/20) | Follow-tail releases on user scroll; jump-to-latest; slot cells preserve virtualization; bounded numeric stepping and invalid state. |
| Shared island shell | [#24](https://github.com/archipel-dev/kit/issues/24) | Common toolbar/title integration and docked island layout; caller-owned state; snapshots, conformance, small-window geometry. SDK/app migrations require follow-up in their own repositories. |
| Independent gallery pages | [#25](https://github.com/archipel-dev/kit/issues/25) | Each page owns its state and rendering; snapshots unchanged by extraction. |

Completion requires the contribution checklist, fmt/clippy/workspace tests, inspecting
changed docs snapshots, and native interaction against the actual gallery. Snapshots are
not a substitute for verifying focus, nested popups, clipboard and window behavior.
