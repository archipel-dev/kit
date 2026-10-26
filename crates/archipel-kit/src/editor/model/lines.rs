//! Line operations (spec 3.7): delete, duplicate, move, join, toggle case, comment.

use super::action::ClipboardContent;
use super::exec::Work;
use super::position::{column_of, offset_at_column};
use super::undo::UndoGroup;
use super::words::{indent_len, is_blank_line, word_range_at};

/// Carets sharing a range of lines.
pub(super) struct LineGroup {
    pub first: usize,
    pub last: usize,
    pub members: Vec<usize>,
}

impl Work<'_> {
    /// Removes lines `first..=last` (with a line break) and puts `members` on the line that
    /// moved up, at column `column`.
    pub(super) fn delete_line_range(
        &mut self,
        first: usize,
        last: usize,
        members: &[usize],
        column: usize,
    ) {
        let count = self.doc.line_count();
        let (start, end) = if last + 1 < count {
            (
                self.doc.line_start_offset(first),
                self.doc.line_start_offset(last + 1),
            )
        } else if first > 0 {
            (self.doc.line_end_offset(first - 1), self.doc.len())
        } else {
            (0, self.doc.len())
        };
        self.replace(start, end, "");
        let line = first.min(self.doc.line_count() - 1);
        let tab = self.tab_size();
        let offset = offset_at_column(self.doc, line, column, tab).0;
        for &member in members {
            let desired = Some(column);
            let caret = self.caret_mut(member);
            caret.move_to(offset);
            caret.desired_column = desired;
        }
    }

    /// `EditorDeleteLine`.
    pub(super) fn delete_lines(&mut self) {
        let tab = self.tab_size();
        for group in self.line_groups(None).into_iter().rev() {
            let lead = self.caret(group.members[0]).offset;
            let column = column_of(self.doc, lead, tab);
            self.delete_line_range(group.first, group.last, &group.members, column);
        }
    }

    /// `EditorDuplicate`.
    pub(super) fn duplicate(&mut self) {
        // Selections: the copy goes right behind the selection and becomes the selection.
        for i in (0..self.n()).rev() {
            let caret = self.caret(i).clone();
            if !caret.has_selection() {
                continue;
            }
            let text = self
                .doc
                .slice(caret.selection_start(), caret.selection_end())
                .to_owned();
            let end = caret.selection_end();
            self.replace(end, end, &text);
            let c = self.caret_mut(i);
            c.select(end, end + text.len());
        }
        // Lines: below, caret to the same place in the copy.
        for group in self.line_groups(Some(true)).into_iter().rev() {
            let start = self.doc.line_start_offset(group.first);
            let end = self.doc.line_end_offset(group.last);
            let block = self.doc.slice(start, end).to_owned();
            let has_break = end < self.doc.len();
            let (at, text) = if has_break {
                (end + 1, format!("{block}\n"))
            } else {
                (end, format!("\n{block}"))
            };
            let delta = block.len() + 1;
            let saved: Vec<(usize, usize)> = group
                .members
                .iter()
                .map(|m| (self.caret(*m).offset, self.caret(*m).anchor))
                .collect();
            self.replace(at, at, &text);
            for (&member, (offset, anchor)) in group.members.iter().zip(saved) {
                let c = self.caret_mut(member);
                c.offset = offset + delta;
                c.anchor = anchor + delta;
                c.desired_column = None;
            }
        }
    }

    /// `MoveLineUp` / `MoveLineDown`.
    pub(super) fn move_lines(&mut self, direction: isize) {
        let mut groups = self.line_groups(None);
        if direction > 0 {
            groups.reverse();
        }
        let count = self.doc.line_count();
        let mut blocked: Option<usize> = None;
        for group in groups {
            let at_edge = if direction < 0 {
                group.first == 0
            } else {
                group.last + 1 >= count
            };
            let touches_blocked = blocked.is_some_and(|edge| {
                if direction < 0 {
                    group.first <= edge + 1
                } else {
                    group.last + 1 >= edge
                }
            });
            if at_edge || touches_blocked {
                blocked = Some(if direction < 0 {
                    group.last
                } else {
                    group.first
                });
                continue;
            }
            let (a, b) = (group.first, group.last);
            let block_start = self.doc.line_start_offset(a);
            let block_end = self.doc.line_end_offset(b);
            let block = self.doc.slice(block_start, block_end).to_owned();
            let neighbor = if direction < 0 { a - 1 } else { b + 1 };
            let neighbor_text = self.doc.line_text(neighbor).to_owned();
            let (region_start, region_end, text, delta) = if direction < 0 {
                (
                    self.doc.line_start_offset(neighbor),
                    block_end,
                    format!("{block}\n{neighbor_text}"),
                    -((neighbor_text.len() + 1) as isize),
                )
            } else {
                (
                    block_start,
                    self.doc.line_end_offset(neighbor),
                    format!("{neighbor_text}\n{block}"),
                    (neighbor_text.len() + 1) as isize,
                )
            };
            let saved: Vec<(usize, usize)> = group
                .members
                .iter()
                .map(|m| (self.caret(*m).offset, self.caret(*m).anchor))
                .collect();
            self.replace(region_start, region_end, &text);
            let doc_len = self.doc.len();
            let move_pos = |pos: usize| {
                if pos >= block_start && pos <= block_end + 1 {
                    ((pos as isize + delta) as usize).min(doc_len)
                } else {
                    pos
                }
            };
            for (&member, (offset, anchor)) in group.members.iter().zip(saved) {
                let c = self.caret_mut(member);
                c.offset = move_pos(offset);
                c.anchor = move_pos(anchor);
            }
        }
    }

    /// `EditorJoinLines`.
    pub(super) fn join_lines(&mut self) {
        let mut groups = self.line_groups(None);
        // A caret without selection (or within one line) joins its line with the next.
        for group in &mut groups {
            if group.last == group.first {
                group.last = group.first + 1;
            }
        }
        groups.reverse();
        for group in groups {
            let last_line = self.doc.line_count() - 1;
            let last = group.last.min(last_line);
            if last <= group.first {
                continue;
            }
            let with_selection = group.members.iter().any(|m| self.caret(*m).has_selection());
            let mut caret_at = None;
            for line in (group.first..last).rev() {
                let break_at = self.doc.line_end_offset(line);
                let next_start = self.doc.line_start_offset(line + 1);
                let blank_end = next_start + indent_len(self.doc, line + 1);
                let rest_empty = blank_end >= self.doc.line_end_offset(line + 1);
                let first_empty = break_at == self.doc.line_start_offset(line);
                let (text, caret) = if rest_empty || first_empty {
                    ("", break_at)
                } else {
                    (" ", break_at + 1)
                };
                self.replace(break_at, blank_end, text);
                caret_at = Some(caret);
            }
            if with_selection {
                for &member in &group.members {
                    let end = self.caret(member).selection_end();
                    self.caret_mut(member).move_to(end);
                }
            } else if let Some(at) = caret_at {
                for &member in &group.members {
                    self.caret_mut(member).move_to(at);
                }
            }
        }
    }

    /// `EditorToggleCase`.
    pub(super) fn toggle_case(&mut self) {
        let camel = self.settings.camel_humps;
        let mut regions = Vec::new();
        for i in 0..self.n() {
            let caret = self.caret(i).clone();
            if caret.has_selection() {
                regions.push((i, caret.selection_start(), caret.selection_end()));
            } else if let Some((s, e)) = word_range_at(self.doc, caret.offset, camel) {
                regions.push((i, s, e));
            }
        }
        if regions.is_empty() {
            self.out.handled = false;
            return;
        }
        let all_lower = regions
            .iter()
            .all(|(_, s, e)| self.doc.slice(*s, *e) == self.doc.slice(*s, *e).to_lowercase());
        for (i, start, end) in regions.into_iter().rev() {
            let old = self.doc.slice(start, end).to_owned();
            let mut new = String::with_capacity(old.len());
            let mut escaped = false;
            for c in old.chars() {
                if escaped {
                    escaped = false;
                    new.push(c);
                } else if c == '\\' {
                    escaped = true;
                    new.push(c);
                } else if all_lower {
                    new.extend(c.to_uppercase());
                } else {
                    new.extend(c.to_lowercase());
                }
            }
            let forward = self.caret(i).offset >= self.caret(i).anchor;
            self.replace(start, end, &new);
            let c = self.caret_mut(i);
            if forward {
                c.select(start, start + new.len());
            } else {
                c.select(start + new.len(), start);
            }
        }
    }

    /// `CommentByLineComment`.
    pub(super) fn comment_by_line(&mut self) {
        let lang = self.ctx.language;
        let (open, close) = match (&lang.line_comment, &lang.block_comment) {
            (Some(prefix), _) => {
                let space = if lang.line_comment_space { " " } else { "" };
                (format!("{prefix}{space}"), String::new())
            }
            (None, Some((open, close))) => (open.clone(), close.clone()),
            (None, None) => {
                self.out.handled = false;
                return;
            }
        };
        let prefix_trimmed = open.trim_end().to_owned();
        let mut lines: Vec<usize> = Vec::new();
        for group in self.line_groups(None) {
            lines.extend(group.first..=group.last);
        }
        lines.dedup();
        let multi = lines.len() > 1;
        let targets: Vec<usize> = lines
            .iter()
            .copied()
            .filter(|l| !(multi && is_blank_line(self.doc, *l)))
            .collect();
        let non_blank: Vec<usize> = targets
            .iter()
            .copied()
            .filter(|l| !is_blank_line(self.doc, *l))
            .collect();
        let is_commented = |w: &Self, line: usize| {
            let text = w.doc.line_text(line).trim();
            text.starts_with(&prefix_trimmed)
                && (close.is_empty() || text.ends_with(close.trim_end()))
        };
        let uncomment = !non_blank.is_empty() && non_blank.iter().all(|l| is_commented(self, *l));
        let indent_col = non_blank
            .iter()
            .map(|l| indent_len(self.doc, *l))
            .min()
            .unwrap_or(0);
        let had_selection = self.model.has_selection();
        for &line in targets.iter().rev() {
            let start = self.doc.line_start_offset(line);
            if uncomment {
                let at = start + indent_len(self.doc, line);
                let text = self.doc.line_text(line);
                let body = &text[indent_len(self.doc, line)..];
                let mut remove = prefix_trimmed.len();
                if body[remove..].starts_with(' ') && open.ends_with(' ') {
                    remove += 1;
                }
                if !close.is_empty() {
                    let end = self.doc.line_end_offset(line);
                    let tail = close.trim_end().len();
                    self.replace(end - tail, end, "");
                    if self.doc.char_before(end - tail) == Some(' ') && open.ends_with(' ') {
                        self.replace(end - tail - 1, end - tail, "");
                    }
                }
                self.replace(at, at + remove, "");
            } else {
                let width = self.doc.line_end_offset(line) - start;
                let at = if self.settings.line_comment_at_first_column && close.is_empty() {
                    start
                } else {
                    start + indent_col.min(width)
                };
                if !close.is_empty() {
                    let end = self.doc.line_end_offset(line);
                    self.replace(end, end, &format!(" {}", close));
                }
                let insert = if close.is_empty() {
                    open.clone()
                } else {
                    format!("{} ", open)
                };
                self.replace(at, at, &insert);
            }
        }
        if !had_selection && self.settings.comment_line_caret_down {
            let tab = self.tab_size();
            for i in 0..self.n() {
                let offset = self.caret(i).offset;
                let line = self.doc.line_number(offset);
                if line + 1 < self.doc.line_count() {
                    let column = column_of(self.doc, offset, tab);
                    let target = offset_at_column(self.doc, line + 1, column, tab).0;
                    self.caret_mut(i).move_to(target);
                }
            }
        }
    }

    /// `CommentByBlockComment`.
    pub(super) fn comment_by_block(&mut self) {
        let Some((open, close)) = self.ctx.language.block_comment.clone() else {
            self.out.handled = false;
            return;
        };
        for i in (0..self.n()).rev() {
            let caret = self.caret(i).clone();
            let (start, end) = if caret.has_selection() {
                (caret.selection_start(), caret.selection_end())
            } else {
                let line = self.doc.line_number(caret.offset);
                (
                    self.doc.line_start_offset(line) + indent_len(self.doc, line),
                    self.doc.line_end_offset(line),
                )
            };
            let text = self.doc.slice(start, end).to_owned();
            if text.starts_with(&open)
                && text.ends_with(&close)
                && text.len() >= open.len() + close.len()
            {
                let inner = text[open.len()..text.len() - close.len()].to_owned();
                self.replace(start, end, &inner);
                if caret.has_selection() {
                    self.caret_mut(i).select(start, start + inner.len());
                } else {
                    let at = start.min(self.doc.len());
                    self.caret_mut(i).move_to(at);
                }
            } else {
                let wrapped = format!("{open}{text}{close}");
                self.replace(start, end, &wrapped);
                if caret.has_selection() {
                    self.caret_mut(i)
                        .select(start + open.len(), start + open.len() + text.len());
                } else {
                    self.caret_mut(i).move_to(start + wrapped.len());
                }
            }
        }
    }

    /// `EditorCutLineEnd`.
    pub(super) fn cut_line_end(&mut self) {
        let mut texts = Vec::new();
        let mut ranges = Vec::new();
        for caret in self.model.carets() {
            let line = self.doc.line_number(caret.offset);
            let end = self.doc.line_end_offset(line);
            let stop = if caret.offset == end && end < self.doc.len() {
                end + 1
            } else {
                end
            };
            texts.push(self.doc.slice(caret.offset, stop).to_owned());
            ranges.push((caret.offset, stop));
        }
        for (i, (start, stop)) in ranges.into_iter().enumerate().rev() {
            self.replace(start, stop, "");
            self.caret_mut(i).move_to(start);
        }
        self.out.clipboard = Some(ClipboardContent::text(texts.join("\n")));
        self.out.undo_group = UndoGroup::None;
    }
}
