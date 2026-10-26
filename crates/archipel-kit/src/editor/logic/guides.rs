//! Indent guides: the indentation of a line in columns, what a blank line inherits, and the
//! guide the caret's block highlights. Pure functions of the document; the GUI and the
//! terminal editor draw the result.

use crate::editor::model::{Document, advance_column};

/// The indentation of `line` in columns, `None` for a blank line.
pub fn indent_cols(doc: &Document, line: usize, tab: usize) -> Option<usize> {
    let text = doc.line_text(line);
    let mut col = 0;
    for c in text.chars() {
        match c {
            ' ' | '\t' => col = advance_column(col, c, tab),
            _ => return Some(col),
        }
    }
    None
}

/// The indentation guides of `line` follow: its own, or for a blank line the smaller of the
/// nearest non-blank lines around it.
pub fn effective_indent(doc: &Document, line: usize, tab: usize) -> usize {
    if let Some(indent) = indent_cols(doc, line, tab) {
        return indent;
    }
    let near = |step: isize| -> usize {
        let mut l = line as isize + step;
        for _ in 0..64 {
            if l < 0 || l as usize >= doc.line_count() {
                return 0;
            }
            if let Some(indent) = indent_cols(doc, l as usize, tab) {
                return indent;
            }
            l += step;
        }
        0
    };
    near(-1).min(near(1))
}

/// The guide the caret's block highlights: its column and the lines it spans.
pub fn selected_guide(
    doc: &Document,
    line: usize,
    tab: usize,
    unit: usize,
) -> Option<(usize, usize, usize)> {
    let indent = effective_indent(doc, line, tab);
    // A block header highlights the guide of its body; a line in a body, the body's guide.
    let next = {
        let mut l = line + 1;
        let mut found = None;
        while l < doc.line_count() && l < line + 64 {
            if let Some(i) = indent_cols(doc, l, tab) {
                found = Some(i);
                break;
            }
            l += 1;
        }
        found
    };
    let col = if next.is_some_and(|n| n > indent) && indent_cols(doc, line, tab).is_some() {
        indent
    } else if indent >= unit {
        (indent - 1) / unit * unit
    } else {
        return None;
    };
    let mut first = line;
    while first > 0 && line - first < 1000 && effective_indent(doc, first - 1, tab) > col {
        first -= 1;
    }
    let mut last = line;
    while last + 1 < doc.line_count()
        && last - line < 1000
        && effective_indent(doc, last + 1, tab) > col
    {
        last += 1;
    }
    // The header line itself is not in the body.
    if effective_indent(doc, line, tab) <= col {
        first = line + 1;
    }
    Some((col, first, last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_helpers() {
        let doc = Document::new("a\n    b\n\n        c\n  d");
        assert_eq!(indent_cols(&doc, 0, 4), Some(0));
        assert_eq!(indent_cols(&doc, 1, 4), Some(4));
        assert_eq!(indent_cols(&doc, 2, 4), None);
        assert_eq!(
            effective_indent(&doc, 2, 4),
            4,
            "a blank line takes the smaller neighbor"
        );
    }

    #[test]
    fn tabs_advance_to_the_next_stop() {
        let doc = Document::new("\tx\n  \ty");
        assert_eq!(indent_cols(&doc, 0, 4), Some(4));
        assert_eq!(indent_cols(&doc, 1, 4), Some(4));
    }

    #[test]
    fn the_selected_guide_is_the_blocks() {
        let doc = Document::new("fn x() {\n    a;\n    b;\n}\nz");
        // Caret on `a;`: the body's guide at column 0, lines 1..=2.
        assert_eq!(selected_guide(&doc, 1, 4, 4), Some((0, 1, 2)));
        // Caret on the header: the same guide.
        assert_eq!(selected_guide(&doc, 0, 4, 4), Some((0, 1, 2)));
        assert_eq!(selected_guide(&doc, 4, 4, 4), None);
    }
}
