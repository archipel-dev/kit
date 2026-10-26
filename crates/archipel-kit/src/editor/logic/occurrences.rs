//! The other occurrences of the selected text (IntelliJ "highlight usages of the selection").

use crate::editor::model::{Document, Editor};
use crate::tokens::editor::MAX_SELECTION_OCCURRENCES;
use std::ops::Range;

/// The other occurrences of the text at `selection` (exact match), or none when there are
/// more than [`MAX_SELECTION_OCCURRENCES`]. The selection itself is not in the list.
pub fn occurrences_of(doc: &Document, selection: Range<usize>) -> Vec<Range<usize>> {
    let needle = doc.slice(selection.start, selection.end);
    let mut found = Vec::new();
    if !needle.is_empty() {
        for (at, m) in doc.text().match_indices(needle) {
            if at != selection.start {
                found.push(at..at + m.len());
            }
            if found.len() > MAX_SELECTION_OCCURRENCES {
                found.clear();
                break;
            }
        }
    }
    found
}

/// The range to look for occurrences of: the selection of a single caret, when it is one
/// line of more than whitespace.
pub fn occurrence_source(editor: &Editor) -> Option<Range<usize>> {
    let carets = editor.carets.carets();
    if carets.len() != 1 || !carets[0].has_selection() {
        return None;
    }
    let (s, e) = (carets[0].selection_start(), carets[0].selection_end());
    let text = editor.document.slice(s, e);
    (!text.trim().is_empty() && !text.contains('\n')).then_some(s..e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occurrences_skip_the_selection_itself() {
        let doc = Document::new("foo bar foo baz foo");
        assert_eq!(occurrences_of(&doc, 0..3), vec![8..11, 16..19]);
    }

    #[test]
    fn more_than_fifty_occurrences_highlight_nothing() {
        let text = "ab ".repeat(60);
        assert!(occurrences_of(&Document::new(&text), 0..2).is_empty());
        let text = "ab ".repeat(51);
        assert_eq!(occurrences_of(&Document::new(&text), 0..2).len(), 50);
    }

    #[test]
    fn only_a_single_line_selection_of_one_caret_is_a_source() {
        let mut editor = Editor::new("foo bar\nfoo");
        assert_eq!(occurrence_source(&editor), None);
        editor.carets.set_selection(&editor.document, 0, 3);
        assert_eq!(occurrence_source(&editor), Some(0..3));
        editor.carets.set_selection(&editor.document, 0, 8);
        assert_eq!(occurrence_source(&editor), None, "multi-line");
        editor.carets.set_selection(&editor.document, 3, 4);
        assert_eq!(occurrence_source(&editor), None, "whitespace only");
    }
}
