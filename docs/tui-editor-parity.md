# TUI editor parity with the GUI `EditorComponent`

The terminal editor (`archipel_kit_tui::editor::EditorComponent`) runs on the same model as the
GUI editor (`archipel_kit::editor::model`) and looks like NvChad: flat, no islands, the colors of
`NvChadPalette`. This page is the audit of the GUI editor feature by feature: what the terminal
editor had (**before**), what it has (**after**), where the code lives and which test covers it.

- **present**: works like the GUI.
- **partial**: there, but with a gap.
- **missing**: not there.
- **by design**: the terminal cannot or should not do it; the reason is given.

GUI tests and snapshots did not change. The logic both editors need now lives in
`archipel_kit::editor::logic` (see the last section), so a rule cannot differ between them.

Tests: `crates/archipel-kit-tui/tests/editor_parity.rs` (new behavior), `tests/editor.rs`
(the earlier suite), the unit tests of each `logic` module.

## Builder options

| GUI option | Before | After | Notes |
|---|---|---|---|
| `label` (accessible name) | missing | by design | a terminal has no accessibility tree; the screen reader reads the cells |
| `placeholder` | present | present | `a_placeholder_shows_while_the_document_is_empty` |
| `highlighter`, `language` | present | present | any GUI `SyntaxHighlighter` works, `CodeHighlighter` is the small built-in one |
| `viewer` | present | present | `a_viewer_selects_and_copies_but_does_not_edit` |
| `line_numbers`, `wrap`, `folding` | present | present | |
| `bracket_folding` | missing | present | `bracket_folding_makes_regions_that_follow_the_text`; the regions come from `logic::folds` |
| `enabled` | missing | present | dimmed text, no input, no focus, no terminal cursor |
| `tab_indents` | missing | present | with `false`, Tab and Shift+Tab are not consumed, so the focus traversal gets them |
| `height` | n/a | by design | the area you pass to `show` |
| `id` | present | present | the first argument of `EditorComponent::new` |
| `completer` | partial | present | now the GUI list's state machine (see Completion) |
| `scroll_to_line` | partial (a method of the state) | present | `scroll_to_line(Option<usize>)` on the builder, `EditorState::scroll_to_line` stays |
| `stick_to_bottom` | missing | present | `stick_to_bottom_follows_a_growing_document_until_the_user_scrolls_up`; no room past the last row |
| `gutter_icons`, `change_markers`, `highlights` | present | present | |
| `show_whitespace`, `indent_guides`, `right_margin` | present | present | |
| `gutter_click_selects_line` | present | present | |
| `keymap_os` | present | present | `the_macos_keymap_moves_by_words_with_option_and_deletes_lines_with_cmd` |
| `search_history` | present (`EditorState::history`) | present | swap one `SearchHistory` between editors |

## What `show` and `handle` report

| GUI response field | Before | After (`EditorOutput`) |
|---|---|---|
| `text_changed`, `carets_moved`, `navigation`, `gutter_clicks`, `hint` | present | present |
| `hover` | missing | `hover: Option<HoverTarget>`, the offset and the first highlight under the pointer after 500 ms |
| `fold_preview` | missing | `fold_preview: Option<Range<usize>>` after 300 ms on a placeholder |
| `stripe_hover` | missing | `stripe_hover: Option<usize>`, the mark under the pointer |
| `selection` | missing | `selection: Option<Range<usize>>` |
| `caret_rect` | missing | `caret_cell: Option<(u16, u16)>` |
| `line_count`, `visible_lines`, `matches` | missing | `line_count`, `visible_lines`, `matches` |
| `completed` | missing | `completed: Option<Completion>`, for `LookupStats` |
| `laid_out_rows` | n/a | by design: only visible rows are ever laid out |

## Editing

| Area | Before | After | Notes |
|---|---|---|---|
| Caret, multi-caret, add caret above / below | present | present | Alt+Shift+click, `Ctrl+K Ctrl+Up / Down`; `carets_can_be_cloned_above_and_below_and_type_together` |
| Column / block selection | present | present | Alt+drag and middle drag; with the column mode (`Alt+Shift+Insert`, `Ctrl+K C`) Shift+arrows extend the block; `column_mode_extends_a_block_with_shift_and_the_arrows`. In the IntelliJ keymap Alt+Shift+Up / Down move the line |
| Selection shape | present | by design | the Islands outline rounds corners by a sixth of a line, below a cell; the line break shows as one extra selected cell, like the GUI |
| Undo / redo grouping | present | present | `undo_groups_a_typed_word_and_redo_brings_it_back` |
| Auto-close brackets and quotes, wrap selection | present | present | `brackets_and_quotes_close_themselves_and_are_overtyped`, `typing_a_bracket_over_a_selection_wraps_it` |
| Smart Enter and indent | present | present | `enter_between_braces_indents_and_puts_the_closer_on_its_own_line` |
| Smart backspace | present | present | `smart_backspace_deletes_an_empty_pair_and_an_indent_level` |
| Comment toggle | present | present | `ctrl_slash_toggles_line_comments_on_the_selected_lines` (Ctrl+7 in a legacy terminal) |
| Duplicate, delete, move line | present | present | `duplicate_delete_and_move_line` |
| Extend / shrink selection | present | present | `extend_and_shrink_selection_grow_by_syntax_units` (Alt+Up / Down) |
| Keymap OS variants | present | present | |
| Viewer mode | present | present | |

## Folding and the gutter

| Area | Before | After | Notes |
|---|---|---|---|
| Fold chevrons, placeholders (`{…}` pill), click to expand | present | present | |
| Bracket folding | missing | present | `bracket_folding(true)` |
| Expand / collapse (Ctrl+- / +, recursive, all) | present | present | `Ctrl+K =` / `-` (Alt: recursively, Shift: all) |
| Fold preview on hover | missing | present | `resting_on_a_fold_placeholder_previews_the_folded_lines` |
| Line numbers, icons, change markers, fold column | present | present | |
| Click areas, Alt+click on a chevron | present | present | |
| Gutter icon names | missing | present | the icon's `name` floats after 500 ms: `a_gutter_icon_names_itself_after_the_delay` |
| Classic outline folding (anchors and a line along the region) | missing | by design | it belongs to the Classic look; NvChad folds with chevrons |

## Error stripe, highlights, hover

| Area | Before | After | Notes |
|---|---|---|---|
| Scrollbar marks for highlights and matches, thumb | present | present | |
| Click on a mark: caret there, centered | present | present | `a_stripe_mark_reports_the_hover_and_names_itself` |
| Underline styles | present | present | undercurl `SGR 4:3` with the color in `58` where the terminal has it (`runtime::detect_undercurl`), a plain colored underline elsewhere; color by severity |
| Hover info | missing | present | the float of the messages under the pointer, most severe first: `resting_on_a_diagnostic_opens_a_float_with_its_messages_after_the_delay`; a key dismisses it until the pointer moves |
| Hover over a stripe mark | missing | present | the message floats to the left of the column |
| Navigation requests | partial (Ctrl+click) | present | Ctrl+click (Cmd on macOS) as before, now also Ctrl+B and Ctrl+Alt+B (Cmd+B on macOS); the GUI keymap has no key for it, the terminal has no better way to ask |
| Brace matching | present | present | `the_unmatched_brace_is_bold_in_the_invalid_color_and_a_matched_pair_is_tinted`; `logic::brace` |
| Indent guides, the selected guide | present | present | `logic::guides` |
| Whitespace, right margin | present | present | |
| Soft wrap | present | present | breaks at the last cell, as the GUI's rows do |

## Find and replace

| Area | Before | After | Notes |
|---|---|---|---|
| History, Match Case, Words, Regex | present | present | |
| In selection (the scope) | present | present | `a_multi_line_selection_becomes_the_scope_of_the_search` |
| Replace, replace all, preserve case | present | present | `replace_all_works_inside_the_scope_and_is_one_undo_step`, `preserve_case_replaces_with_the_case_of_the_match` |
| Word at the caret (Ctrl+F3) | present | present | |
| Clearing the query restores caret and view | present | present | |
| Wrap hint, `n/m` | present | present | |
| Current match emphasis | partial (a slightly lighter tint) | present | the current match is inverted and bold (the caret color on the editor background, as IncSearch); the others keep the tint: `the_current_match_is_inverted_and_bold_the_others_only_tinted` |
| Multi-line query (Ctrl+Shift+Enter) | missing | missing | the prompt is one row; a multi-line selection still becomes the scope |
| The search itself | duplicated | shared | `logic::find::FindSession` runs both bars |

## Completion and templates

| Area | Before | After | Notes |
|---|---|---|---|
| Popup under the caret, kind icons, matched characters | present | present | nvim-cmp look |
| Filtering and ranking | missing (the completer's order) | present | camel humps, middle matching, exact case first, recent first: `camel_humps_narrow_the_list_and_prefix_matches_come_first`, `a_recently_chosen_item_ranks_first_among_equal_matches` |
| Commit characters (`. , ; = space : (`) | missing | present | `a_commit_character_inserts_the_item_and_is_typed_after_it` |
| Ctrl+Space: lone answer goes in, second press asks for more | missing | present | `a_lone_answer_to_ctrl_space_goes_in_at_once`, `the_second_ctrl_space_asks_for_more` |
| Backspace on an empty prefix, caret leaving the prefix | partial | present | |
| Insert at every caret, one undo step | missing (primary caret only) | present | `an_item_goes_in_at_every_caret_with_the_same_prefix` |
| Tail text, type text, bold, strikeout, icon | missing | present | `bold_struck_out_items_and_the_tail_and_type_text_draw_as_the_presentation_says` |
| Documentation (Ctrl+Q, Ctrl+J, F1) | missing | present | a box beside the menu |
| Advertisement line | missing | present | |
| Auto-popup delay | missing | present | `tick` runs it; `EditorState::deadline` says when |
| `{{variable}}` templates | present (highlighter) | present | `TemplateHighlighter` colors, `template_at` completion: `template_variables_complete_inside_braces`, `template_variables_are_colored_by_what_they_resolve_to`. The kit has no IntelliJ-style live templates |
| `completed` for `LookupStats` | missing | present | |

## Highlighter

| Area | Before | After | Notes |
|---|---|---|---|
| Doc comments `//!`, `///`, `/** */`, `/*! */` | tinted as comments in the spans, not covered | present | `doc_comments_are_tinted_as_comments` and the highlighter's unit test |

## What is shared now

`archipel_kit::editor::logic` has no renderer in it; the GUI and the terminal editor both call it.

| Module | What moved there | Was |
|---|---|---|
| `logic::brace` | `match_brace`, `BracePair` | a copy in `editor_component/paint.rs` and in `archipel-kit-tui` |
| `logic::guides` | `indent_cols`, `effective_indent`, `selected_guide` | the same two copies |
| `logic::folds` | `bracket_regions`, `fold_regions`, `BracketRegion` | `editor_component/cache.rs` |
| `logic::occurrences` | `occurrences_of`, `occurrence_source` | `cache.rs` and a loop in the terminal paint |
| `logic::find` | `FindSession` (query, options, scope, origin, wrap, live selection, next / previous, replace, replace all, select all, word at caret), `FindCmd`, `current_match` | `editor/find.rs` and a port in the terminal's `find.rs` |
| `logic::lookup` | `LookupState` (the list's state machine), `LookupKeys`, `LookupFrame`, `Chosen`, `insert_chosen` (at every caret), the commit characters | `ui/completion.rs`, `editor/lookup.rs` and a smaller port in the terminal |
| `logic::hover` | `Rest` (the rest tracker), `HoverTarget`, `highlight_at`, `messages_at`, `stripe_hit`, `fold_dwell`, `preview_lines`, `HighlightSeverity::rank` | inline in `EditorComponent::show` |
| `logic::gestures` | `inside_selection`, `line_range`, `word_or_line`, `set_selection`, `select_lines`, `region_at_line`, `autoscroll_tick` | `editor_component/mouse.rs`, `gutter.rs`, `state.rs` and a port in the terminal's `mouse.rs` |

The GUI components call the shared functions in place of their own copies; their behavior is
unchanged. `HoverInfo::target()` gives the renderer-independent part of the GUI's hover.
