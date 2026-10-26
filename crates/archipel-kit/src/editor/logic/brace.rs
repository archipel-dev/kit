//! Brace matching: the bracket at the caret and its partner.

use crate::editor::model::{Editor, Language};

/// The matched (or unmatched) brace at the caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BracePair {
    /// Byte offset of the brace at the caret.
    pub brace: usize,
    /// Byte offset of its partner, `None` when it has none (drawn as an unmatched brace).
    pub partner: Option<usize>,
}

/// The brace at the caret and its partner (spec §8.1): the bracket directly before the caret,
/// else the one directly after. Only with one caret and no selection.
pub fn match_brace(editor: &Editor, language: &Language) -> Option<BracePair> {
    if editor.carets.caret_count() != 1 || editor.carets.primary().has_selection() {
        return None;
    }
    let doc = &editor.document;
    let offset = editor.carets.primary().offset;
    let is_bracket = |c: char| {
        language
            .brackets
            .iter()
            .any(|&(open, close)| c == open || c == close)
    };
    let before = doc.char_before(offset).filter(|c| is_bracket(*c));
    let (brace, c) = match before {
        Some(c) => (offset - c.len_utf8(), c),
        None => {
            let c = doc.char_at(offset).filter(|c| is_bracket(*c))?;
            (offset, c)
        }
    };
    let (open, close, forward) = language.brackets.iter().find_map(|&(o, cl)| {
        if c == o {
            Some((o, cl, true))
        } else if c == cl {
            Some((o, cl, false))
        } else {
            None
        }
    })?;
    let text = doc.text();
    let mut depth = 0usize;
    let limit = 200_000;
    let partner = if forward {
        text[brace..]
            .char_indices()
            .take(limit)
            .find_map(|(i, ch)| {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(brace + i);
                    }
                }
                None
            })
    } else {
        let mut found = None;
        for (i, ch) in text[..brace + c.len_utf8()]
            .char_indices()
            .rev()
            .take(limit)
        {
            if ch == close {
                depth += 1;
            } else if ch == open {
                depth -= 1;
                if depth == 0 {
                    found = Some(i);
                    break;
                }
            }
        }
        found
    };
    Some(BracePair { brace, partner })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_match_across_nesting() {
        let mut editor = Editor::new("f(a(b), [c])");
        let language = Language::c_like();
        editor.carets.set_caret(&editor.document, 2); // after the first `(`
        assert_eq!(
            match_brace(&editor, &language),
            Some(BracePair {
                brace: 1,
                partner: Some(11)
            })
        );
        editor.carets.set_caret(&editor.document, 12); // after the last `)`... end
        let pair = match_brace(&editor, &language).expect("a brace");
        assert_eq!((pair.brace, pair.partner), (11, Some(1)));
        editor.carets.set_caret(&editor.document, 0);
        assert_eq!(match_brace(&editor, &language), None);
    }

    #[test]
    fn a_lone_brace_has_no_partner() {
        let mut editor = Editor::new("a (b");
        editor.carets.set_caret(&editor.document, 3);
        let pair = match_brace(&editor, &Language::c_like()).expect("a brace");
        assert_eq!((pair.brace, pair.partner), (2, None));
    }

    #[test]
    fn no_brace_while_text_is_selected() {
        let mut editor = Editor::new("(a)");
        editor.carets.set_selection(&editor.document, 0, 2);
        assert_eq!(match_brace(&editor, &Language::c_like()), None);
    }
}
