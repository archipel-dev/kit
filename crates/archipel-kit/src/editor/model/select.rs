//! Selection and multi-caret actions: extend/shrink, occurrences, clone, column mode
//! (spec 1.5 to 1.7).

use super::action::ScrollRequest;
use super::caret::{BlockSelection, Caret, CaretModel};
use super::document::Document;
use super::exec::Work;
use super::position::{
    LogicalPosition, column_of, line_width, offset_at_column, offset_to_logical,
};
use super::scrolling::ScrollType;
use super::words::{is_blank_line, is_identifier_part, word_range_at};

// ---- block (column) selection on the caret model ------------------------------------

impl CaretModel {
    /// Rebuilds one caret per line for `block` (Alt+drag, column mode). Short lines get a
    /// caret at their end with an empty selection and the missing columns as virtual space.
    pub fn apply_block_selection(
        &mut self,
        doc: &Document,
        tab_size: usize,
        block: BlockSelection,
    ) {
        let (first, last) = block.lines();
        let last = last.min(doc.line_count() - 1);
        let first = first.min(last);
        let (left, right) = block.columns();
        let lead_right = block.lead.column >= block.anchor.column;
        let mut built: Vec<Caret> = Vec::new();
        let mut lead_caret = None;
        for line in first..=last {
            let (start, _) = offset_at_column(doc, line, left, tab_size);
            let (end, _) = offset_at_column(doc, line, right, tab_size);
            let width = line_width(doc, line, tab_size);
            let lead_column = if lead_right { right } else { left };
            let (anchor, offset) = if lead_right {
                (start, end)
            } else {
                (end, start)
            };
            let mut caret = Caret::at(offset, 0);
            caret.anchor = anchor;
            if offset == doc.line_end_offset(line) {
                caret.virtual_spaces = lead_column.saturating_sub(width);
            }
            if line == block.lead.line.clamp(first, last) {
                lead_caret = Some(caret);
            } else {
                built.push(caret);
            }
        }
        if let Some(caret) = lead_caret {
            built.push(caret);
        }
        let mut carets = Vec::new();
        for mut caret in built {
            caret.id = self.next_caret_id();
            carets.push(caret);
        }
        *self.carets_mut() = carets;
        self.normalize();
        self.block = Some(BlockSelection {
            anchor: block.anchor,
            lead: block.lead,
        });
        self.column_mode = true;
    }

    /// Starts a rectangular selection at `at` (Alt+press).
    pub fn start_block_selection(&mut self, doc: &Document, tab_size: usize, at: LogicalPosition) {
        self.apply_block_selection(
            doc,
            tab_size,
            BlockSelection {
                anchor: at,
                lead: at,
            },
        );
    }

    /// Moves the lead corner of the rectangular selection to `to` (Alt+drag); starts one at
    /// the primary caret when there is none.
    pub fn extend_block_selection(&mut self, doc: &Document, tab_size: usize, to: LogicalPosition) {
        let anchor = match self.block {
            Some(block) => block.anchor,
            None => {
                let caret = self.primary();
                let mut position = offset_to_logical(doc, caret.offset, tab_size);
                position.column += caret.virtual_spaces;
                position
            }
        };
        self.apply_block_selection(doc, tab_size, BlockSelection { anchor, lead: to });
    }

    /// One caret at the end of every line from `first` to `last` (Add Caret per Selected
    /// Line).
    pub fn caret_per_line(&mut self, doc: &Document, first: usize, last: usize) {
        let last = last.min(doc.line_count() - 1);
        let first = first.min(last);
        let ends: Vec<usize> = (first..=last).map(|l| doc.line_end_offset(l)).collect();
        self.set_caret(doc, ends[0]);
        for end in &ends[1..] {
            self.add_caret(doc, *end);
        }
    }
}

// ---- candidates for extend selection --------------------------------------------------

fn run_around(doc: &Document, p: usize, pred: impl Fn(char) -> bool) -> Option<(usize, usize)> {
    let mut start = p;
    while let Some(c) = doc.char_before(start) {
        if !pred(c) {
            break;
        }
        start -= c.len_utf8();
    }
    let mut end = p;
    while let Some(c) = doc.char_at(end) {
        if !pred(c) {
            break;
        }
        end += c.len_utf8();
    }
    (start < end).then_some((start, end))
}

const WINDOW: usize = 1000;

fn enclosing_pair(doc: &Document, p: usize, open: char, close: char) -> Option<(usize, usize)> {
    let lo = doc.snap(p.saturating_sub(WINDOW * 5));
    let hi = doc.snap(p + WINDOW * 5);
    let mut depth = 0usize;
    let mut left = None;
    for (index, c) in doc.slice(lo, p).char_indices().rev() {
        if c == close {
            depth += 1;
        } else if c == open {
            if depth == 0 {
                left = Some(lo + index);
                break;
            }
            depth -= 1;
        }
    }
    let left = left?;
    let mut depth = 0usize;
    for (index, c) in doc.slice(p, hi).char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            if depth == 0 {
                return Some((left, p + index));
            }
            depth -= 1;
        }
    }
    None
}

fn enclosing_quotes(doc: &Document, p: usize, quote: char) -> Option<(usize, usize)> {
    let line = doc.line_number(p);
    let (ls, le) = (doc.line_start_offset(line), doc.line_end_offset(line));
    let mut positions = Vec::new();
    let mut escaped = false;
    for (index, c) in doc.slice(ls, le).char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            positions.push(ls + index);
        }
    }
    positions
        .chunks(2)
        .find(|pair| pair.len() == 2 && pair[0] < p && p <= pair[1])
        .map(|pair| (pair[0], pair[1]))
}

fn paragraph_bounds(doc: &Document, p: usize) -> Option<(usize, usize)> {
    let line = doc.line_number(p);
    if is_blank_line(doc, line) {
        return None;
    }
    let mut first = line;
    while first > 0 && !is_blank_line(doc, first - 1) {
        first -= 1;
    }
    let mut last = line;
    while last + 1 < doc.line_count() && !is_blank_line(doc, last + 1) {
        last += 1;
    }
    Some((doc.line_start_offset(first), doc.line_end_offset(last)))
}

fn sentence(doc: &Document, p: usize) -> Option<(usize, usize)> {
    let (pstart, pend) = paragraph_bounds(doc, p)?;
    let lo = doc.snap(pstart.max(p.saturating_sub(WINDOW)));
    let hi = doc.snap(pend.min(p + WINDOW));
    let text = doc.text();
    let is_end = |index: usize, c: char| {
        matches!(c, '.' | '!' | '?')
            && text[index + c.len_utf8()..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
    };
    let mut start = pstart.max(lo);
    for (index, c) in text[lo..p].char_indices() {
        if is_end(lo + index, c) {
            start = lo + index + c.len_utf8();
        }
    }
    while let Some(c) = doc.char_at(start) {
        if c.is_whitespace() && start < p {
            start += c.len_utf8();
        } else {
            break;
        }
    }
    let mut end = pend.min(hi);
    for (index, c) in text[p..hi].char_indices() {
        if is_end(p + index, c) {
            end = p + index + c.len_utf8();
            break;
        }
    }
    (start < end).then_some((start, end))
}

/// The plain-text chain of extend-selection candidates around `p` (spec 1.7).
fn candidates(doc: &Document, p: usize, camel: bool) -> Vec<(usize, usize)> {
    let mut list = Vec::new();
    if camel {
        list.extend(word_range_at(doc, p, true));
    }
    list.extend(word_range_at(doc, p, false));
    list.extend(run_around(doc, p, |c| is_identifier_part(c) || c == '\''));
    list.extend(run_around(doc, p, |c| {
        is_identifier_part(c) || c == '\'' || c == '-'
    }));
    for (open, close) in [('(', ')'), ('[', ']'), ('<', '>'), ('{', '}')] {
        if let Some((l, r)) = enclosing_pair(doc, p, open, close) {
            list.push((l + 1, r));
            list.push((l, r + 1));
        }
    }
    for quote in ['"', '\'', '`'] {
        if let Some((l, r)) = enclosing_quotes(doc, p, quote) {
            list.push((l + 1, r));
            list.push((l, r + 1));
        }
    }
    list.extend(run_around(doc, p, |c| {
        c.is_alphanumeric() || "().,:;!?$@%\"'<>[]_".contains(c)
    }));
    list.extend(sentence(doc, p));
    list.extend(paragraph_bounds(doc, p));
    let line = doc.line_number(p);
    let (ls, le) = (doc.line_start_offset(line), doc.line_end_offset(line));
    if ls < le {
        list.push((ls, le));
    }
    if !doc.is_empty() {
        list.push((0, doc.len()));
    }
    list
}

impl Work<'_> {
    /// `EditorSelectWord`.
    pub(super) fn extend_selection(&mut self) {
        let camel = self.settings.camel_humps;
        let before = self.model.snapshot();
        for i in 0..self.n() {
            let caret = self.caret(i).clone();
            let (s, e) = (caret.selection_start(), caret.selection_end());
            let best = candidates(self.doc, caret.offset, camel)
                .into_iter()
                .chain(candidates(self.doc, s, camel))
                .filter(|(a, b)| *a <= s && e <= *b && (*a < s || e < *b))
                .min_by_key(|(a, b)| b - a);
            if let Some((a, b)) = best {
                self.caret_mut(i).select(a, b);
            }
        }
        let after = self.model.snapshot();
        if after == before {
            self.out.handled = false;
            return;
        }
        self.model.push_selection_chain(before, after);
    }

    /// `EditorUnSelectWord`.
    pub(super) fn shrink_selection(&mut self) {
        if let Some(previous) = self.model.pop_selection_chain() {
            let doc = &*self.doc;
            self.model.restore(doc, &previous);
            return;
        }
        let camel = self.settings.camel_humps;
        for i in 0..self.n() {
            let caret = self.caret(i).clone();
            if !caret.has_selection() {
                continue;
            }
            let (s, e) = (caret.selection_start(), caret.selection_end());
            let mut list = candidates(self.doc, caret.offset, camel);
            list.extend(candidates(self.doc, s, camel));
            list.extend(candidates(self.doc, e, camel));
            let best = list
                .into_iter()
                .filter(|(a, b)| s <= *a && *b <= e && (s < *a || *b < e))
                .max_by_key(|(a, b)| b - a);
            match best {
                Some((a, b)) => self.caret_mut(i).select(a, b),
                None => {
                    let offset = caret.offset;
                    self.caret_mut(i).move_to(offset);
                }
            }
        }
    }

    fn occurrence_needle(&mut self) -> Option<String> {
        let primary = self.model.primary().clone();
        if primary.has_selection() {
            return Some(
                self.doc
                    .slice(primary.selection_start(), primary.selection_end())
                    .to_owned(),
            );
        }
        let (s, e) = word_range_at(self.doc, primary.offset, false)?;
        self.model.occurrence.whole_word = true;
        let index = self.model.primary_index();
        self.caret_mut(index).select(s, e);
        None
    }

    fn is_whole_word(&self, start: usize, end: usize) -> bool {
        !self.doc.char_before(start).is_some_and(is_identifier_part)
            && !self.doc.char_at(end).is_some_and(is_identifier_part)
    }

    fn already_selected(&self, start: usize, end: usize) -> bool {
        self.model
            .carets()
            .iter()
            .any(|c| c.selection_start() == start && c.selection_end() == end)
    }

    fn find_occurrence(&self, needle: &str, from: usize, before: usize) -> Option<(usize, usize)> {
        let text = self.doc.text();
        let mut at = self.doc.snap(from);
        while at < text.len() {
            let found = at + text[at..].find(needle)?;
            if found >= before {
                return None;
            }
            let end = found + needle.len();
            let ok = (!self.model.occurrence.whole_word || self.is_whole_word(found, end))
                && !self.already_selected(found, end);
            if ok {
                return Some((found, end));
            }
            at = self.doc.next_offset(found);
        }
        None
    }

    /// `SelectNextOccurrence`.
    pub(super) fn select_next_occurrence(&mut self) {
        let Some(needle) = self.occurrence_needle() else {
            // Only the word under the caret was selected.
            return;
        };
        if needle.is_empty() {
            return;
        }
        let primary = self.model.primary().clone();
        let found = self.find_occurrence(&needle, primary.selection_end(), usize::MAX);
        let found = match found {
            Some(f) => Some(f),
            None if self.model.occurrence.wrap_pending => {
                self.model.occurrence.wrap_pending = false;
                self.find_occurrence(&needle, 0, primary.selection_end())
            }
            None => {
                self.model.occurrence.wrap_pending = true;
                self.out.hint = Some("Not found, try again to search from top".to_owned());
                return;
            }
        };
        match found {
            Some((start, end)) => {
                self.model.occurrence.wrap_pending = false;
                let doc = &*self.doc;
                if !self.model.add_caret_with_selection(doc, start, end) {
                    self.out.hint = Some("Too many carets".to_owned());
                }
                self.out.scroll = Some(ScrollRequest::ToCaret {
                    scroll_type: ScrollType::MakeVisible,
                    animate: true,
                });
            }
            None => self.out.hint = Some("No more occurrences".to_owned()),
        }
    }

    /// `UnselectPreviousOccurrence`.
    pub(super) fn unselect_previous_occurrence(&mut self) {
        if self.n() > 1 {
            let index = self.model.primary_index();
            self.model.remove_caret(index);
        } else if self.model.primary().has_selection() {
            self.model.primary_mut().remove_selection();
        } else {
            self.out.handled = false;
        }
    }

    /// `SelectAllOccurrences`.
    pub(super) fn select_all_occurrences(&mut self) {
        let Some(needle) = self.occurrence_needle().or_else(|| {
            let p = self.model.primary();
            Some(
                self.doc
                    .slice(p.selection_start(), p.selection_end())
                    .to_owned(),
            )
        }) else {
            return;
        };
        if needle.is_empty() {
            return;
        }
        let mut from = 0;
        while let Some((start, end)) = self.find_occurrence(&needle, from, usize::MAX) {
            let doc = &*self.doc;
            if !self.model.add_caret_with_selection(doc, start, end) {
                break;
            }
            from = end;
        }
    }

    /// `EditorCloneCaretAbove` / `EditorCloneCaretBelow`.
    pub(super) fn clone_caret(&mut self, direction: isize) {
        let tab = self.tab_size();
        let originals: Vec<Caret> = self.model.carets().to_vec();
        let mut added = false;
        for caret in originals {
            let doc = &*self.doc;
            let line = doc.line_number(caret.offset);
            let column = caret
                .desired_column
                .unwrap_or_else(|| column_of(doc, caret.offset, tab));
            let (new_anchor, new_offset) = if caret.has_selection() {
                let (s, e) = (caret.selection_start(), caret.selection_end());
                if doc.line_number(s) != doc.line_number(e) {
                    continue;
                }
                let (c1, c2) = (column_of(doc, s, tab), column_of(doc, e, tab));
                let mut target = line as isize + direction;
                while target >= 0
                    && (target as usize) < doc.line_count()
                    && line_width(doc, target as usize, tab) < c2
                {
                    target += direction;
                }
                if target < 0 || target as usize >= doc.line_count() {
                    continue;
                }
                let a = offset_at_column(doc, target as usize, c1, tab).0;
                let b = offset_at_column(doc, target as usize, c2, tab).0;
                if caret.offset >= caret.anchor {
                    (a, b)
                } else {
                    (b, a)
                }
            } else {
                let target = line as isize + direction;
                if target < 0 || target as usize >= doc.line_count() {
                    continue;
                }
                let o = offset_at_column(doc, target as usize, column, tab).0;
                (o, o)
            };
            let doc = &*self.doc;
            if self
                .model
                .add_caret_with_selection(doc, new_anchor, new_offset)
            {
                added = true;
                let index = self.model.primary_index();
                self.caret_mut(index).desired_column = Some(column);
            }
        }
        if !added {
            self.out.handled = false;
        }
    }

    /// `EditorAddCaretPerSelectedLine`.
    pub(super) fn add_caret_per_selected_line(&mut self) {
        let mut offsets = Vec::new();
        let mut any = false;
        for caret in self.model.carets().to_vec() {
            let (first, last) = self.line_range_of(&caret);
            if caret.has_selection() && last > first {
                any = true;
                offsets.extend((first..=last).map(|l| self.doc.line_end_offset(l)));
            } else {
                offsets.push(caret.offset);
            }
        }
        if !any {
            self.out.handled = false;
            return;
        }
        offsets.sort_unstable();
        offsets.dedup();
        let doc = &*self.doc;
        self.model.set_caret(doc, offsets[0]);
        for offset in &offsets[1..] {
            self.model.add_caret(doc, *offset);
        }
    }

    /// `EditorToggleColumnMode`.
    pub(super) fn toggle_column_mode(&mut self) {
        let tab = self.tab_size();
        if self.model.column_mode {
            self.model.column_mode = false;
            if self.model.block.take().is_some() {
                let first = self.model.carets()[0].clone();
                let last = self.model.carets()[self.n() - 1].clone();
                let doc = &*self.doc;
                self.model
                    .set_selection(doc, first.selection_start(), last.selection_end());
            }
            return;
        }
        let primary = self.model.primary().clone();
        if primary.has_selection() && self.n() == 1 {
            let doc = &*self.doc;
            let anchor = offset_to_logical(doc, primary.anchor, tab);
            let lead = offset_to_logical(doc, primary.offset, tab);
            self.model
                .apply_block_selection(doc, tab, BlockSelection { anchor, lead });
        } else {
            self.model.column_mode = true;
        }
    }

    /// Shift+arrows in column mode grow the rectangle.
    pub(super) fn block_step(&mut self, dx: isize, dy: isize) {
        let tab = self.tab_size();
        let doc = &*self.doc;
        let block = match self.model.block {
            Some(block) => block,
            None => {
                let caret = self.model.primary();
                let mut at = offset_to_logical(doc, caret.offset, tab);
                at.column += caret.virtual_spaces;
                BlockSelection {
                    anchor: at,
                    lead: at,
                }
            }
        };
        let mut lead = block.lead;
        lead.column = (lead.column as isize + dx).max(0) as usize;
        lead.line = (lead.line as isize + dy).clamp(0, doc.line_count() as isize - 1) as usize;
        self.model.apply_block_selection(
            doc,
            tab,
            BlockSelection {
                anchor: block.anchor,
                lead,
            },
        );
    }

    /// `EditorEscape`: clears the selection and all but the oldest caret.
    pub(super) fn escape(&mut self) {
        if !self.model.has_selection() && self.n() <= 1 {
            self.out.handled = false;
            return;
        }
        let keep = self.model.oldest_index();
        let mut caret = self.model.carets()[keep].clone();
        caret.remove_selection();
        *self.model.carets_mut() = vec![caret];
        self.model.block = None;
    }
}
