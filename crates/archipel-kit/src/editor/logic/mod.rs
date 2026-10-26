//! Editor logic that has no renderer: what the GUI [`EditorComponent`](crate::editor::EditorComponent)
//! and the terminal editor of `archipel-kit-tui` both run, so they cannot disagree.
//!
//! | Module | What it decides |
//! |---|---|
//! | [`brace`] | The brace at the caret and its partner. |
//! | [`guides`] | Indent guide columns and the guide the caret's block highlights. |
//! | [`folds`] | Bracket folding: `{...}` and `[...]` regions of a text. |
//! | [`occurrences`] | The other occurrences of the selected text. |
//! | [`gestures`] | Mouse gesture rules: units, selections by drag, autoscroll, the fold region of a gutter line. |
//! | [`find`] | The find bar's session: live selection, next and previous with the wrap hint, replace, replace all. |
//! | [`lookup`] | The completion list's session: opening, narrowing, ranking, keys, and inserting an item at every caret. |
//! | [`hover`] | Where the pointer rests, what is under it, and the marks of the error stripe. |

pub mod brace;
pub mod find;
pub mod folds;
pub mod gestures;
pub mod guides;
pub mod hover;
pub mod lookup;
pub mod occurrences;

pub use brace::{BracePair, match_brace};
pub use folds::{BracketRegion, bracket_regions, fold_regions};
pub use guides::{effective_indent, indent_cols, selected_guide};
pub use occurrences::{occurrence_source, occurrences_of};
