//! The JetBrains-faithful editor model: document, carets, actions, undo, folding, scrolling
//! and the default keymap. Pure logic, no painting; see `docs/editor-model.md`.

// Nested `if let` reads clearer than let chains for the many guard-then-act steps here.
#![allow(clippy::collapsible_if)]

mod action;
mod caret;
mod clipboard;
mod document;
mod editor;
mod exec;
mod folding;
mod keymap;
mod lines;
mod motion;
mod position;
mod scrolling;
mod select;
mod settings;
mod typing;
mod undo;
mod words;

pub use action::{ActionContext, ClipboardContent, EditOutcome, EditorAction, ScrollRequest};
pub use caret::{BlockSelection, Caret, CaretModel, CaretSnapshot, OccurrenceState, TabOutScope};
pub use document::{Document, Edit, normalize_newlines};
pub use editor::Editor;
pub use exec::execute;
pub use folding::{FoldRegion, FoldingModel};
pub use keymap::{Binding, KeyModifiers, Os, default_bindings, editor_action_for};
pub use position::{
    LogicalPosition, VisualPosition, advance_column, column_of, line_width, logical_to_offset,
    offset_at_column, offset_to_logical,
};
pub use scrolling::{
    ContentSize, MAX_ANIMATION_MS, ScrollTarget, ScrollType, Viewport, clamp_first_line,
    horizontal_target, max_first_column, max_first_line, scroll_to_caret, vertical_target,
};
pub use settings::{
    CaretStopOptions, EditorSettings, Language, LineStop, ReformatOnPaste, SmartBackspace, WordStop,
};
pub use undo::{UndoEntry, UndoGroup, UndoManager, UndoResult};
pub use words::{
    CharClass, class_of, is_hump_bound, is_identifier_part, is_word_end, is_word_start, next_stop,
    prev_stop, word_range_at,
};
