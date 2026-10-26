//! Word classification and word-stop rules (spec 1.2).

use super::document::Document;
use super::settings::{CaretStopOptions, LineStop, WordStop};

/// The three character classes word motion distinguishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharClass {
    /// Identifier part: Unicode letters and digits, `_`, `$` and other currency symbols.
    Word,
    /// Whitespace, line breaks included.
    Whitespace,
    /// Everything else (punctuation, operators).
    Other,
}

/// Whether `c` may be part of a Java identifier (the notion IntelliJ uses for words).
pub fn is_identifier_part(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || matches!(c, '$' | '\u{a2}'..='\u{a5}' | '\u{20a0}'..='\u{20cf}')
        || matches!(c, '\u{300}'..='\u{36f}')
}

/// The class of `c`.
pub fn class_of(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Whitespace
    } else if is_identifier_part(c) {
        CharClass::Word
    } else {
        CharClass::Other
    }
}

fn classes(doc: &Document, offset: usize) -> (CharClass, CharClass) {
    let before = doc
        .char_before(offset)
        .map_or(CharClass::Whitespace, class_of);
    let after = doc.char_at(offset).map_or(CharClass::Whitespace, class_of);
    (before, after)
}

/// Whether a word starts at `offset`: a word character follows a non-word one, or a
/// punctuation character follows a non-punctuation one (so `->` is one word).
pub fn is_word_start(doc: &Document, offset: usize) -> bool {
    let (before, after) = classes(doc, offset);
    (after == CharClass::Word && before != CharClass::Word)
        || (after == CharClass::Other && before != CharClass::Other)
}

/// Whether a word ends at `offset` (the mirror image of [`is_word_start`]).
pub fn is_word_end(doc: &Document, offset: usize) -> bool {
    let (before, after) = classes(doc, offset);
    (before == CharClass::Word && after != CharClass::Word)
        || (before == CharClass::Other && after != CharClass::Other)
}

/// Whether `offset` is a CamelHumps boundary inside an identifier.
pub fn is_hump_bound(doc: &Document, offset: usize) -> bool {
    let (Some(before), Some(after)) = (doc.char_before(offset), doc.char_at(offset)) else {
        return false;
    };
    if !is_identifier_part(before) || !is_identifier_part(after) {
        return false;
    }
    if (before.is_lowercase() || before.is_ascii_digit()) && after.is_uppercase() {
        return true;
    }
    if (before == '_') != (after == '_') {
        return true;
    }
    if before == '$' && after.is_alphanumeric() {
        return true;
    }
    if before.is_uppercase()
        && after.is_uppercase()
        && doc
            .char_at(offset + after.len_utf8())
            .is_some_and(char::is_lowercase)
    {
        return true;
    }
    false
}

#[derive(Clone, Copy)]
enum Wanted {
    Start,
    End,
    Both,
}

fn is_stop(doc: &Document, offset: usize, wanted: Wanted, camel: bool) -> bool {
    let base = match wanted {
        Wanted::Start => is_word_start(doc, offset),
        Wanted::End => is_word_end(doc, offset),
        Wanted::Both => is_word_start(doc, offset) || is_word_end(doc, offset),
    };
    base || (camel && is_hump_bound(doc, offset))
}

/// Where Ctrl+Right (`EditorNextWord`) goes from `offset` (IntelliJ
/// `getNextCaretStopOffset`).
///
/// With the default options the caret stops at word ends only, and never passes the start of
/// the next line: with no word end left on the line it lands in column 0 of the next line.
pub fn next_stop(doc: &Document, offset: usize, options: CaretStopOptions, camel: bool) -> usize {
    let offset = doc.snap(offset);
    let len = doc.len();
    if offset >= len {
        return len;
    }
    let line = doc.line_number(offset);
    let limit = match options.line_boundary {
        LineStop::Neighbor => doc.line_end_with_break(line),
        LineStop::Current => {
            let end = doc.line_end_offset(line);
            if offset < end {
                end
            } else {
                doc.line_end_with_break(line)
            }
        }
        LineStop::None => len,
    };
    let wanted = match options.word_boundary {
        WordStop::Current | WordStop::End => Wanted::End,
        WordStop::Neighbor | WordStop::Start => Wanted::Start,
        WordStop::Both => Wanted::Both,
    };
    let mut i = doc.next_offset(offset);
    while i < limit {
        if is_stop(doc, i, wanted, camel) {
            return i;
        }
        i = doc.next_offset(i);
    }
    limit
}

/// Where Ctrl+Left (`EditorPreviousWord`) goes from `offset`.
///
/// Default: word starts only; with none left before the end of the previous line the caret
/// jumps to that line end.
pub fn prev_stop(doc: &Document, offset: usize, options: CaretStopOptions, camel: bool) -> usize {
    let offset = doc.snap(offset);
    if offset == 0 {
        return 0;
    }
    let line = doc.line_number(offset);
    let line_start = doc.line_start_offset(line);
    let previous_line_end = line_start.saturating_sub(1);
    let limit = match options.line_boundary {
        LineStop::Neighbor => previous_line_end,
        LineStop::Current => {
            if offset > line_start {
                line_start
            } else {
                previous_line_end
            }
        }
        LineStop::None => 0,
    };
    let wanted = match options.word_boundary {
        WordStop::Current | WordStop::Start => Wanted::Start,
        WordStop::Neighbor | WordStop::End => Wanted::End,
        WordStop::Both => Wanted::Both,
    };
    let mut i = doc.prev_offset(offset);
    while i > limit {
        if is_stop(doc, i, wanted, camel) {
            return i;
        }
        i = doc.prev_offset(i);
    }
    limit
}

/// The range Ctrl+Backspace deletes: back to the word start, but not out of the line (at
/// column 0 just the line break).
pub fn range_to_word_start(doc: &Document, offset: usize, camel: bool) -> (usize, usize) {
    let offset = doc.snap(offset);
    let options = CaretStopOptions {
        word_boundary: WordStop::Start,
        line_boundary: LineStop::Current,
    };
    (prev_stop(doc, offset, options, camel), offset)
}

/// The range Ctrl+Delete deletes: to the next word boundary, but not out of the line (at the
/// line end just the line break).
pub fn range_to_word_end(doc: &Document, offset: usize, camel: bool) -> (usize, usize) {
    let offset = doc.snap(offset);
    let options = CaretStopOptions {
        word_boundary: WordStop::Both,
        line_boundary: LineStop::Current,
    };
    (offset, next_stop(doc, offset, options, camel))
}

/// The identifier run around `offset` (the caret at the end of a word counts as inside it).
/// With `camel` the CamelHumps segment of the run.
pub fn word_range_at(doc: &Document, offset: usize, camel: bool) -> Option<(usize, usize)> {
    let offset = doc.snap(offset);
    let mut start = offset;
    while let Some(c) = doc.char_before(start) {
        if !is_identifier_part(c) {
            break;
        }
        start -= c.len_utf8();
    }
    let mut end = offset;
    while let Some(c) = doc.char_at(end) {
        if !is_identifier_part(c) {
            break;
        }
        end += c.len_utf8();
    }
    if start == end {
        return None;
    }
    if !camel {
        return Some((start, end));
    }
    // Segment of the run that holds the character at the caret (or before it at the end).
    let probe = if offset < end {
        offset
    } else {
        doc.prev_offset(offset)
    };
    let mut segment_start = start;
    let mut i = doc.next_offset(start);
    while i < end {
        if is_hump_bound(doc, i) {
            if i <= probe {
                segment_start = i;
            } else {
                return Some((segment_start, i));
            }
        }
        i = doc.next_offset(i);
    }
    Some((segment_start, end))
}

/// Leading whitespace (spaces and tabs) of `line`, in bytes.
pub fn indent_len(doc: &Document, line: usize) -> usize {
    doc.line_text(line)
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(char::len_utf8)
        .sum()
}

/// Whether `line` is empty or only spaces and tabs.
pub fn is_blank_line(doc: &Document, line: usize) -> bool {
    doc.line_text(line).chars().all(|c| c == ' ' || c == '\t')
}
