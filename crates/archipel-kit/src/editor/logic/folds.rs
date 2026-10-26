//! Bracket folding: the `{...}` and `[...]` pairs that span several lines as fold regions.

use crate::editor::model::{Document, FoldRegion};

/// A bracket pair spanning several lines: the range from the opener to just after the closer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketRegion {
    /// Byte offset of the opener.
    pub start: usize,
    /// Byte offset just after the closer.
    pub end: usize,
    /// `{` or `[`.
    pub opener: char,
}

/// Bracket pairs (`{}` and `[]`) whose closer is on a later line than the opener, sorted by
/// start, one per opener line (the longest). Strings in double quotes are skipped.
pub fn bracket_regions(doc: &Document) -> Vec<BracketRegion> {
    let text = doc.text();
    let mut stack: Vec<(usize, usize, u8)> = Vec::new();
    let mut regions: Vec<(usize, usize, usize, u8)> = Vec::new();
    let mut line_start = 0;
    for (line, piece) in text.split('\n').enumerate() {
        let (mut in_string, mut escaped) = (false, false);
        for (i, &b) in piece.as_bytes().iter().enumerate() {
            if in_string {
                match b {
                    _ if escaped => escaped = false,
                    b'\\' => escaped = true,
                    b'"' => in_string = false,
                    _ => {}
                }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'{' | b'[' => stack.push((line_start + i, line, b)),
                    b'}' | b']' => {
                        if let Some((open, open_line, opener)) = stack.pop()
                            && open_line < line
                        {
                            regions.push((open, line_start + i + 1, open_line, opener));
                        }
                    }
                    _ => {}
                }
            }
        }
        line_start += piece.len() + 1;
    }
    regions.sort_unstable_by_key(|&(start, end, _, _)| (start, std::cmp::Reverse(end)));
    let mut out: Vec<BracketRegion> = Vec::new();
    let mut last_line = usize::MAX;
    for (start, end, open_line, opener) in regions {
        if open_line == last_line {
            continue;
        }
        last_line = open_line;
        out.push(BracketRegion {
            start,
            end,
            opener: opener as char,
        });
    }
    out
}

/// The fold regions for `brackets`, keeping the collapsed state of regions in `old` that
/// start at the same offset.
pub fn fold_regions(brackets: &[BracketRegion], old: &[FoldRegion]) -> Vec<FoldRegion> {
    brackets
        .iter()
        .map(|b| {
            let expanded = old
                .iter()
                .find(|r| r.start == b.start)
                .is_none_or(|r| r.expanded);
            FoldRegion {
                start: b.start,
                end: b.end,
                placeholder: if b.opener == '{' { "{...}" } else { "[...]" }.to_owned(),
                expanded,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_regions_span_lines_one_per_opener_line() {
        let doc = Document::new("{\n  \"a\": [1,\n  2],\n  \"b\": \"}\"\n}\n[ 1 ]");
        let brackets = bracket_regions(&doc);
        let spans: Vec<_> = brackets
            .iter()
            .map(|b| (b.start, b.end, b.opener))
            .collect();
        // The object spans lines 0..=4; the array lines 1..=2; `[ 1 ]` is on one line and the
        // `}` inside the string does not close anything.
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert_eq!(spans[0].2, '{');
        assert_eq!(doc.line_number(spans[0].1), 4);
        assert_eq!(spans[1].2, '[');
    }

    #[test]
    fn fold_regions_keep_what_was_collapsed() {
        let doc = Document::new("{\n  [\n  1\n  ]\n}");
        let brackets = bracket_regions(&doc);
        let first = fold_regions(&brackets, &[]);
        assert!(first.iter().all(|r| r.expanded));
        let mut old = first.clone();
        old[1].expanded = false;
        let again = fold_regions(&brackets, &old);
        assert_eq!(
            again.iter().map(|r| r.expanded).collect::<Vec<_>>(),
            [true, false]
        );
        assert_eq!(again[0].placeholder, "{...}");
        assert_eq!(again[1].placeholder, "[...]");
    }

    #[test]
    fn an_unbalanced_text_never_panics() {
        for text in ["}", "]]", "{\n", "[\n{\n]\n}", "\"\n{\n}", ""] {
            let _ = bracket_regions(&Document::new(text));
        }
    }
}
