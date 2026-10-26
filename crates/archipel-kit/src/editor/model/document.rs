//! The text buffer: IntelliJ's `Document`.

use std::borrow::Cow;

/// One applied text change: `removed` was replaced by `inserted` at byte `offset`.
///
/// Edits are what the undo manager stores and what range markers (folds, tab-out scopes,
/// carets) are shifted by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Byte offset of the change.
    pub offset: usize,
    /// The text that was there before.
    pub removed: String,
    /// The text that is there now.
    pub inserted: String,
}

impl Edit {
    /// Where a position moves to when this edit is applied (a range-marker shift).
    ///
    /// Positions at or before the edit offset stay, positions after the removed range move by
    /// the length difference, positions inside the removed range collapse to the offset.
    pub fn shift(&self, position: usize) -> usize {
        let removed_end = self.offset + self.removed.len();
        if position <= self.offset {
            position
        } else if position >= removed_end {
            position - self.removed.len() + self.inserted.len()
        } else {
            self.offset
        }
    }

    /// The edit that undoes this one.
    pub fn inverted(&self) -> Edit {
        Edit {
            offset: self.offset,
            removed: self.inserted.clone(),
            inserted: self.removed.clone(),
        }
    }
}

/// Converts `\r\n` and lone `\r` to `\n`, the only line separator the model knows.
///
/// IntelliJ does the same on load and on paste: documents hold `\n` only and the line
/// separator is remembered per file by the caller.
pub fn normalize_newlines(text: &str) -> Cow<'_, str> {
    if text.contains('\r') {
        Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// The text of an editor, with an incrementally maintained line-start index.
///
/// Counterpart of IntelliJ's `Document`. Offsets are byte offsets; every method clamps its
/// offsets into the text and snaps them back to a character boundary, so no input can panic.
/// Line breaks are `\n` only ([`normalize_newlines`]); text ending in `\n` has a last, empty
/// line, so [`Document::line_count`] is always at least 1.
#[derive(Clone, Debug)]
pub struct Document {
    text: String,
    line_starts: Vec<usize>,
    stamp: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new("")
    }
}

impl Document {
    /// A document holding `text` (line breaks normalized to `\n`).
    pub fn new(text: &str) -> Self {
        let text = normalize_newlines(text).into_owned();
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(index, _)| index + 1));
        Self {
            text,
            line_starts,
            stamp: 0,
        }
    }

    /// The whole text (IntelliJ `getCharsSequence`).
    pub fn char_sequence(&self) -> &str {
        &self.text
    }

    /// The whole text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Length in bytes (IntelliJ `getTextLength`, which counts UTF-16 units).
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the document is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Increases with every change (IntelliJ `getModificationStamp`).
    pub fn modification_stamp(&self) -> u64 {
        self.stamp
    }

    /// Number of lines, at least 1.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Clamps `offset` into the text and moves it back to a character boundary.
    pub fn snap(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    /// The offset one character after `offset` (or the end of the text).
    pub fn next_offset(&self, offset: usize) -> usize {
        let offset = self.snap(offset);
        self.text[offset..]
            .chars()
            .next()
            .map_or(offset, |c| offset + c.len_utf8())
    }

    /// The offset one character before `offset` (or 0).
    pub fn prev_offset(&self, offset: usize) -> usize {
        let offset = self.snap(offset);
        self.text[..offset]
            .chars()
            .next_back()
            .map_or(offset, |c| offset - c.len_utf8())
    }

    /// The character starting at `offset`.
    pub fn char_at(&self, offset: usize) -> Option<char> {
        self.text.get(offset..)?.chars().next()
    }

    /// The character ending at `offset`.
    pub fn char_before(&self, offset: usize) -> Option<char> {
        self.text.get(..offset)?.chars().next_back()
    }

    /// The text between two offsets (any order, clamped).
    pub fn slice(&self, a: usize, b: usize) -> &str {
        let (a, b) = (self.snap(a), self.snap(b));
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        &self.text[start..end]
    }

    /// Offset of the first character of `line` (IntelliJ `getLineStartOffset`); lines past
    /// the end give the text length.
    pub fn line_start_offset(&self, line: usize) -> usize {
        self.line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len())
    }

    /// Offset of the end of `line`, before its `\n` (IntelliJ `getLineEndOffset`).
    pub fn line_end_offset(&self, line: usize) -> usize {
        let line = line.min(self.line_count() - 1);
        match self.line_starts.get(line + 1) {
            Some(next) => next - 1,
            None => self.text.len(),
        }
    }

    /// Offset where `line` and its line break end: the start of the next line, or the text
    /// end for the last line.
    pub fn line_end_with_break(&self, line: usize) -> usize {
        self.line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len())
    }

    /// The line containing `offset` (IntelliJ `getLineNumber`).
    pub fn line_number(&self, offset: usize) -> usize {
        let offset = offset.min(self.text.len());
        self.line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1)
    }

    /// The text of `line` without its line break.
    pub fn line_text(&self, line: usize) -> &str {
        let line = line.min(self.line_count() - 1);
        &self.text[self.line_start_offset(line)..self.line_end_offset(line)]
    }

    /// Inserts `text` at `offset` (IntelliJ `insertString`).
    pub fn insert_string(&mut self, offset: usize, text: &str) -> Option<Edit> {
        self.replace_string(offset, offset, text)
    }

    /// Deletes the text between two offsets (IntelliJ `deleteString`).
    pub fn delete_string(&mut self, start: usize, end: usize) -> Option<Edit> {
        self.replace_string(start, end, "")
    }

    /// Replaces the text between two offsets (IntelliJ `replaceString`); returns the applied
    /// change, or `None` when nothing changed.
    pub fn replace_string(&mut self, start: usize, end: usize, text: &str) -> Option<Edit> {
        let (a, b) = (self.snap(start), self.snap(end));
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        let text = normalize_newlines(text);
        if start == end && text.is_empty() {
            return None;
        }
        let removed = self.text[start..end].to_owned();
        if removed == text {
            return None;
        }
        let first = self.line_number(start);
        let last = self.line_number(end);
        let delta = text.len() as isize - (end - start) as isize;
        self.text.replace_range(start..end, &text);
        self.line_starts.drain(first + 1..=last);
        for line_start in &mut self.line_starts[first + 1..] {
            *line_start = (*line_start as isize + delta) as usize;
        }
        let new_starts: Vec<usize> = text
            .match_indices('\n')
            .map(|(index, _)| start + index + 1)
            .collect();
        self.line_starts.splice(first + 1..first + 1, new_starts);
        self.stamp += 1;
        Some(Edit {
            offset: start,
            removed,
            inserted: text.into_owned(),
        })
    }

    /// Replaces the whole text.
    pub fn set_text(&mut self, text: &str) {
        let stamp = self.stamp + 1;
        *self = Self::new(text);
        self.stamp = stamp;
    }
}
