//! Caret motions: horizontal, word, line, page and bracket (spec 1.1 to 1.4).

use super::action::ScrollRequest;
use super::caret::Caret;
use super::exec::{Move, Work};
use super::position::{column_of, offset_at_column};
use super::scrolling::ScrollType;
use super::words::{indent_len, is_blank_line, next_stop, prev_stop};

pub(super) fn motion_left(w: &Work<'_>, caret: &Caret) -> Move {
    if !w.extending && caret.has_selection() {
        return Move::to(caret.selection_start());
    }
    Move::to(w.doc.prev_offset(caret.offset))
}

pub(super) fn motion_right(w: &Work<'_>, caret: &Caret) -> Move {
    if !w.extending && caret.has_selection() {
        return Move::to(caret.selection_end());
    }
    Move::to(w.doc.next_offset(caret.offset))
}

pub(super) fn motion_prev_word(w: &Work<'_>, caret: &Caret) -> Move {
    Move::to(prev_stop(
        w.doc,
        caret.offset,
        w.settings.caret_stop,
        w.settings.camel_humps,
    ))
}

pub(super) fn motion_next_word(w: &Work<'_>, caret: &Caret) -> Move {
    Move::to(next_stop(
        w.doc,
        caret.offset,
        w.settings.caret_stop,
        w.settings.camel_humps,
    ))
}

/// Smart Home (spec 1.3).
pub(super) fn motion_line_start(w: &Work<'_>, caret: &Caret) -> Move {
    let doc = &*w.doc;
    let offset = caret.offset;
    let line = doc.line_number(offset);
    let start = doc.line_start_offset(line);
    if !w.settings.smart_home {
        return Move::to(start);
    }
    if is_blank_line(doc, line) {
        if offset == start {
            // A blank line goes to the indentation of the nearest non-blank line above.
            let above = (0..line).rev().find(|l| !is_blank_line(doc, *l));
            if let Some(above) = above {
                let wanted = indent_len(doc, above);
                let width = doc.line_end_offset(line) - start;
                return Move::to(start + wanted.min(width));
            }
        }
        return Move::to(start);
    }
    let first = start + indent_len(doc, line);
    if offset == start || first < offset {
        Move::to(first)
    } else {
        Move::to(start)
    }
}

/// End with the stop before trailing whitespace (spec 1.3).
pub(super) fn motion_line_end(w: &Work<'_>, caret: &Caret) -> Move {
    let doc = &*w.doc;
    let offset = caret.offset;
    let line = doc.line_number(offset);
    let end = doc.line_end_offset(line);
    if !w.settings.smart_end {
        return Move::to(end);
    }
    let text = doc.line_text(line);
    let trimmed = text.trim_end_matches([' ', '\t']).len();
    let before_trailing = doc.line_start_offset(line) + trimmed;
    if offset < before_trailing {
        Move::to(before_trailing)
    } else {
        Move::to(end)
    }
}

impl Work<'_> {
    pub(super) fn no_animation_scroll(&mut self, scroll_type: ScrollType) {
        self.out.scroll = Some(ScrollRequest::ToCaret {
            scroll_type,
            animate: false,
        });
    }

    /// PgUp / PgDn: the caret moves by a page of lines, the viewport by the same.
    pub(super) fn page(&mut self, extend: bool, direction: isize) {
        self.extending = extend;
        if !extend && self.model.block.is_some() {
            self.model.remove_secondary_carets();
            self.model.block = None;
        }
        let lines = self.ctx.page_lines.max(1) as isize * direction;
        let tab = self.tab_size();
        for i in 0..self.n() {
            let caret = self.caret(i).clone();
            let doc = &*self.doc;
            let line = doc.line_number(caret.offset) as isize;
            let target = (line + lines).clamp(0, doc.line_count() as isize - 1) as usize;
            let desired = caret
                .desired_column
                .unwrap_or_else(|| column_of(doc, caret.offset, tab));
            let offset = offset_at_column(doc, target, desired, tab).0;
            let c = self.caret_mut(i);
            c.offset = offset;
            if !extend {
                c.anchor = offset;
            }
            c.desired_column = Some(desired);
        }
        self.out.scroll = Some(ScrollRequest::Lines(lines));
    }

    /// Ctrl+Home / Ctrl+End: a single caret (or every caret extended) at the text edge.
    pub(super) fn text_edge(&mut self, extend: bool, end: bool) {
        let target = if end { self.doc.len() } else { 0 };
        if extend {
            for i in 0..self.n() {
                let c = self.caret_mut(i);
                c.offset = target;
                c.desired_column = None;
            }
        } else {
            self.model.remove_secondary_carets();
            self.model.block = None;
            self.model.primary_mut().move_to(target);
        }
        let scroll_type = if end {
            ScrollType::Center
        } else {
            ScrollType::Relative
        };
        self.no_animation_scroll(scroll_type);
    }

    /// First or last fully visible line, keeping the column.
    pub(super) fn page_edge(&self, caret: &Caret, bottom: bool) -> Move {
        let doc = &*self.doc;
        let tab = self.tab_size();
        let first = self.ctx.first_visible_line;
        let line = if bottom {
            first + self.ctx.visible_lines.max(1) - 1
        } else {
            first
        }
        .min(doc.line_count() - 1);
        let desired = caret
            .desired_column
            .unwrap_or_else(|| column_of(doc, caret.offset, tab));
        Move {
            offset: offset_at_column(doc, line, desired, tab).0,
            desired: Some(desired),
        }
    }

    /// Ctrl+Up / Ctrl+Down: scroll a line; the caret follows only if it would leave the
    /// band between the second visible line and the third from last.
    pub(super) fn scroll_lines(&mut self, delta: isize) {
        let last_line = self.doc.line_count() as isize - 1;
        let first = (self.ctx.first_visible_line as isize + delta).clamp(0, last_line);
        let visible = self.ctx.visible_lines as isize;
        let low = (first + 1).min(last_line);
        let high = (first + visible - 3).max(low).min(last_line);
        let tab = self.tab_size();
        for i in 0..self.n() {
            let caret = self.caret(i).clone();
            let doc = &*self.doc;
            let line = doc.line_number(caret.offset) as isize;
            let target = line.clamp(low, high);
            if target != line {
                let desired = caret
                    .desired_column
                    .unwrap_or_else(|| column_of(doc, caret.offset, tab));
                let offset = offset_at_column(doc, target as usize, desired, tab).0;
                let c = self.caret_mut(i);
                c.move_to(offset);
                c.desired_column = Some(desired);
            }
        }
        self.out.scroll = Some(ScrollRequest::Lines(delta));
    }

    pub(super) fn pairs(&self) -> Vec<(char, char)> {
        if self.ctx.language.brackets.is_empty() {
            vec![('(', ')'), ('[', ']'), ('{', '}')]
        } else {
            self.ctx.language.brackets.clone()
        }
    }

    /// Ctrl+[ / Ctrl+]: the enclosing unmatched `{` (caret on it) or `}` (caret after it).
    pub(super) fn code_block(&self, caret: &Caret, end: bool) -> Move {
        let doc = &*self.doc;
        let text = doc.text();
        let offset = caret.offset;
        if end {
            let mut depth = 0usize;
            for (index, c) in text[offset..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' if depth == 0 => return Move::to(offset + index + 1),
                    '}' => depth -= 1,
                    _ => {}
                }
            }
        } else {
            let mut depth = 0usize;
            for (index, c) in text[..offset].char_indices().rev() {
                match c {
                    '}' => depth += 1,
                    '{' if depth == 0 => return Move::to(index),
                    '{' => depth -= 1,
                    _ => {}
                }
            }
        }
        Move::to(offset)
    }

    /// The partner of the bracket next to the caret.
    pub(super) fn match_brace(&self, caret: &Caret) -> Move {
        let doc = &*self.doc;
        let offset = caret.offset;
        let pairs = self.pairs();
        let before = doc.char_before(offset);
        let at = doc.char_at(offset);
        if let Some(c) = before {
            if let Some((open, _)) = pairs.iter().find(|(_, close)| *close == c) {
                if let Some(found) = self.scan_backward(offset - c.len_utf8(), *open, c) {
                    return Move::to(found);
                }
            }
        }
        if let Some(c) = at {
            if let Some((_, close)) = pairs.iter().find(|(open, _)| *open == c) {
                if let Some(found) = self.scan_forward(offset + c.len_utf8(), c, *close) {
                    return Move::to(found + close.len_utf8());
                }
            }
            if let Some((open, _)) = pairs.iter().find(|(_, close)| *close == c) {
                if let Some(found) = self.scan_backward(offset, *open, c) {
                    return Move::to(found);
                }
            }
        }
        if let Some(c) = before {
            if let Some((_, close)) = pairs.iter().find(|(open, _)| *open == c) {
                if let Some(found) = self.scan_forward(offset, c, *close) {
                    return Move::to(found + close.len_utf8());
                }
            }
        }
        Move::to(offset)
    }

    /// Offset of the opener matching a closer that ends at `from` (scanning left of `from`).
    fn scan_backward(&self, from: usize, open: char, close: char) -> Option<usize> {
        let mut depth = 0usize;
        for (index, c) in self.doc.text()[..from].char_indices().rev() {
            if c == close {
                depth += 1;
            } else if c == open {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
            }
        }
        None
    }

    /// Offset of the closer matching an opener, scanning right from `from`.
    fn scan_forward(&self, from: usize, open: char, close: char) -> Option<usize> {
        let mut depth = 0usize;
        for (index, c) in self.doc.text()[from..].char_indices() {
            if c == open {
                depth += 1;
            } else if c == close {
                if depth == 0 {
                    return Some(from + index);
                }
                depth -= 1;
            }
        }
        None
    }
}
