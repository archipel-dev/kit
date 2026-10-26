# Editor model

`archipel_kit::editor::model` is the logic of a JetBrains-style code editor with no painting
in it: the document, the carets, every editing action, undo, folding data, scroll-to-caret
math and the default keymap. A painter (egui, or a test) sits on top of it. It was written
from the behavior of the IntelliJ editor, not from its code, and it follows the defaults of
the IntelliJ platform editor, not egui's `TextEdit`.

Everything is plain data owned by the caller. There is no global state, nothing allocates a
widget, and no input can panic: offsets outside the text are clamped and snapped to a
character boundary.

## The pieces

| Type | IntelliJ counterpart | Role |
|---|---|---|
| `Document` | `Document` | The text with an incremental line index, `insert_string`, `delete_string`, `replace_string`. |
| `Caret`, `CaretModel` | `Caret`, `CaretModel` | Carets sorted by position, merged when they overlap, the primary caret is the newest. |
| `LogicalPosition` | `LogicalPosition` | Line and column; tabs advance to the next tab stop. |
| `EditorSettings` | `EditorSettingsExternalizable`, `CodeInsightSettings` | Every default the model needs. |
| `Language` | `Commenter`, `BraceMatcher`, `QuoteHandler` | Line comment prefix, bracket pairs, quotes. Plain text has none. |
| `EditorAction`, `execute` | `EditorActionHandler` | One action for all carets. |
| `UndoManager` | `UndoManager` | Command grouping and caret-restoring undo. |
| `FoldingModel` | `FoldingModel` | Fold regions, offset to visual line mapping. |
| `scroll_to_caret` | `ScrollPositionCalculator` | Where the viewport must go, no animation. |
| `editor_action_for` | the default keymaps | Key chord to action, Windows/Linux and macOS. |
| `SearchOptions`, `find_all`, `Searcher`, `SearchHistory` | `FindModel`, `FindManager` | Find and replace without a screen: Unicode case folding, Words, Regex with `$1`, the 10 000 result cap, replacement templates, Preserve Case, recent queries. |
| `Editor` | `Editor` | All of the above in one struct, with undo recording; `replace_ranges` (Replace All as one undo step) and `select_ranges` (one caret per match). |

## Running actions

`Editor` owns the state and records undo steps:

```rust
use archipel_kit::editor::model::{ActionContext, Editor, EditorAction};

let mut editor = Editor::new("hello world");
let ctx = ActionContext::plain_text();

editor.execute(&EditorAction::NextWord, &ctx); // Ctrl+Right: to the end of the word
assert_eq!(editor.carets.primary().offset, 5);

editor.execute(&EditorAction::Type('!'), &ctx);
assert_eq!(editor.document.text(), "hello! world");

editor.execute(&EditorAction::Undo, &ctx);
assert_eq!(editor.document.text(), "hello world");
```

If you keep the state yourself, call `execute(doc, carets, settings, action, ctx)` and feed
the returned `EditOutcome::edits` to your own undo stack. The outcome also tells the painter
what to do next: `scroll` (follow the caret, or scroll by lines), `clipboard` (text for the
system clipboard after Copy and Cut), `hint` (a message to show under the caret) and
`handled` (false when the action had nothing to do, so Escape can fall through to a popup).

The `ActionContext` carries what an action may need besides the model: the page size and the
first visible line, the clipboard content for paste, the `Language` and the fold regions.

```rust
use archipel_kit::editor::model::{
    ActionContext, ClipboardContent, Editor, EditorAction, Language,
};

let language = Language::c_like(); // `//` comments, `()[]{}` pairs, quotes
let clip = ClipboardContent::text("x");
let ctx = ActionContext::new(&language)
    .with_viewport(0, 40) // first visible line, visible lines
    .with_clipboard(&clip);

let mut editor = Editor::new("f(");
editor.carets.set_caret(&editor.document, 2);
editor.execute(&EditorAction::Type('a'), &ctx);
editor.execute(&EditorAction::Type(')'), &ctx);
assert_eq!(editor.document.text(), "f(a)");
```

## From keys to actions

`editor_action_for` maps a chord to an action with the default JetBrains keymap of the
platform. Typed characters are not keys: send each one as `EditorAction::Type`.

```rust
use archipel_kit::editor::model::{
    editor_action_for, EditorAction, KeyModifiers, Os,
};
use egui::Key;

let word = editor_action_for(Key::ArrowRight, KeyModifiers::CTRL, Os::WindowsLinux);
assert_eq!(word, Some(EditorAction::NextWord));

// macOS: Option moves by word, Cmd+Left is the line start.
let word = editor_action_for(Key::ArrowRight, KeyModifiers::ALT, Os::MacOs);
assert_eq!(word, Some(EditorAction::NextWord));
let home = editor_action_for(Key::ArrowLeft, KeyModifiers::CMD, Os::MacOs);
assert_eq!(home, Some(EditorAction::LineStart));

// Select next occurrence: Alt+J, and Ctrl+G on macOS.
let select = editor_action_for(Key::J, KeyModifiers::ALT, Os::WindowsLinux);
assert_eq!(select, Some(EditorAction::SelectNextOccurrence));
```

`KeyModifiers: From<egui::Modifiers>` takes `mac_cmd` as Cmd, so the same conversion works
on every platform. Chords of actions the model does not implement (Go to line,
completion) return `None`: the application handles them. Find, Replace and Find Next are
chords of the widget's find bar (see `docs/components/code.md`), which uses
`model::search` for the matching.

## Defaults

`EditorSettings::default()` has the IntelliJ defaults:

| Setting | Default |
|---|---|
| tab size, indent size, tabs | 4, 4, spaces |
| smart Home, smart End | on, on |
| virtual space, caret inside tabs | off, off |
| CamelHumps in word motion | **off** |
| auto-pair brackets, quotes | on, on (only with a `Language` that has pairs) |
| surround selection on quote typed, Tab exits brackets | on, on |
| smart indent on Enter, smart backspace | on, auto-indent (one character without a formatter) |
| reformat on paste | indent each line (the block fallback without a formatter) |
| Copy/Cut with no selection | the whole line |
| paste of a line copied that way | above the caret line |
| Comment Line moves the caret down | on; prefix at column 0 |
| scroll margins | 1 line vertical, 3 columns horizontal, jump 0 |
| scroll past end | 5 lines, 3 columns |
| word stops | `CaretStopOptions::DEFAULT`: Ctrl+Right at the word end, Ctrl+Left at the word start, never past the neighboring line boundary |
| carets, undo steps | 1000, 100 |

Word classes: a word is a run of identifier characters (Unicode letters and digits, `_`,
`$`), a punctuation run such as `->` is a word too, whitespace is neither. Offsets are byte
offsets; columns count characters and expand tabs, wide characters count as one column.

## Multi-caret and column selection

Every action runs for all carets, bottom-up so earlier edits do not move later carets, and
carets that meet or whose selections overlap merge. Line actions (Duplicate, Delete Line,
Move Line, Join Lines) treat carets that share a line as one.

```rust
use archipel_kit::editor::model::{Editor, LogicalPosition};

let mut editor = Editor::new("abc\nabc\nabc");
// Alt+drag from line 0, column 1 to line 2, column 2.
let tab = editor.settings.tab_size;
editor.carets.start_block_selection(&editor.document, tab, LogicalPosition::new(0, 1));
editor.carets.extend_block_selection(&editor.document, tab, LogicalPosition::new(2, 2));
assert_eq!(editor.carets.caret_count(), 3);
assert_eq!(editor.carets.block.unwrap().columns(), (1, 2));
```

Lines shorter than the rectangle get a caret at their end with `virtual_spaces` set: typing
pads the line first, like IntelliJ.

## Undo

A typing run is one step, however long the pauses. Enter, paste and the line actions are
steps of their own; consecutive Backspaces merge unless one joins two lines, consecutive Tabs
merge. Moving the caret ends a run. When the carets moved after the last edit, the first
Undo only puts them back where the edit left them.

## Folding and scrolling

`FoldingModel` stores regions (start, end, placeholder, expanded), maps logical lines to
visual lines and expands a region when a caret ends up inside it or an edit touches it.
Typing at the boundary of a collapsed region does not expand it.

```rust
use archipel_kit::editor::model::{Document, FoldingModel};

let doc = Document::new("fn f() {\n    a;\n}\nb");
let mut folds = FoldingModel::new();
folds.add_region(&doc, 7, 17, "{...}", false);
assert_eq!(folds.visual_line_count(&doc), 2);
assert_eq!(folds.visual_line_text(&doc, 0), "fn f() {...}");
```

`scroll_to_caret` returns the first visible line and column for a caret, with the margins
and the center rule of IntelliJ; `ScrollTarget::animation_ms` is 0 for jumps of one line or
less and 100 ms from eleven lines.

```rust
use archipel_kit::editor::model::{
    scroll_to_caret, ContentSize, EditorSettings, ScrollType, Viewport,
};

let viewport = Viewport {
    first_line: 0.0,
    visible_lines: 10.0,
    first_column: 0.0,
    visible_columns: 80.0,
};
let content = ContentSize { lines: 100, columns: 60 };
let target = scroll_to_caret(
    &viewport, content, 9.0, 0.0, ScrollType::Relative, false, &EditorSettings::default(),
);
assert_eq!(target.first_line, 1.0); // one line of margin below the caret
```

## The widget

[`EditorComponent`](components/code.md) is the painter on top of this model: it borrows an
`Editor`, turns keys into actions with `editor_action_for`, typed text, IME commits and the
clipboard into `Type`, `Paste`, `Copy` and `Cut`, and the mouse into caret operations
(`set_caret`, `set_selection`, `add_caret`, `start_block_selection`, `toggle_fold_region`), then
follows the `ScrollRequest` of each outcome with `scroll_to_caret`. With `.viewer(true)` it
leaves out the editing actions and nothing else.

## Differences from IntelliJ, forced by immediate mode or by the missing language services

- **Pixels become columns.** The desired X of vertical motion is a visual column. Autoscroll
  and mouse selection by word or line are the component's (drag and drop of selected text is
  not implemented); the model offers
  `CaretModel::click`, `extend_selection_to`, `add_caret`, `remove_caret_at` and the block
  selection helpers for it.
- **Document model.** Line breaks are normalized to `\n` (`CRLF` and `CR` on load and on
  paste); the file's separator is the caller's to remember. Offsets are UTF-8 bytes, not
  UTF-16 units. There are no range markers beyond carets, tab-out scopes and fold regions,
  which shift by `Edit::shift`.
- **Language services are a small struct.** No PSI: auto-pairing counts brackets in the
  whole text, quotes look at the current line, Extend Selection uses the plain-text chain
  (word, apostrophe and hyphen words, brackets and quotes, natural runs, sentence,
  paragraph, line, text), Move Statement is Move Line, Smart Enter is Start New Line, no
  re-indent on `}`, no syntax token boundaries for word stops.
- **Set up by the caller:** the double-Ctrl gesture for clone caret and its "reverse arrow
  removes the clone" rule, the sticky selection mode, soft wrap (visual rows: the component wraps the display, the model still moves by logical lines), inlays, sticky lines,
  paste history and the system clipboard itself.
- **Open points to verify in the running IDE:** the line selected after Copy with an empty
  selection, where Ctrl+] puts the caret (the model puts it after the brace), the new line
  of Start New Line Before (the model copies the indent), the exact join-line spacing.
