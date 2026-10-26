//! Typing, Enter, Tab, Backspace and Delete (spec 3.1 to 3.6, 3.10).

use super::caret::TabOutScope;
use super::exec::Work;
use super::position::column_of;
use super::settings::SmartBackspace;
use super::undo::UndoGroup;
use super::words::{indent_len, is_identifier_part, range_to_word_end, range_to_word_start};

const WRAP_OPEN: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];
const WRAP_QUOTES: [char; 3] = ['"', '\'', '`'];

impl Work<'_> {
    /// Typing a character for every caret (spec 3.1 to 3.3).
    pub(super) fn type_char(&mut self, c: char) {
        self.out.undo_group = UndoGroup::Typing;
        for i in (0..self.n()).rev() {
            self.type_char_at(i, c);
        }
    }

    fn type_char_at(&mut self, i: usize, c: char) {
        self.pad_virtual(i);
        let doc_len_char = c.len_utf8();
        let caret = self.caret(i).clone();
        if self.model.overwrite && !caret.has_selection() {
            if let Some(next) = self.doc.char_at(caret.offset).filter(|n| *n != '\n') {
                self.replace(caret.offset, caret.offset + next.len_utf8(), &c.to_string());
                self.caret_mut(i).move_to(caret.offset + doc_len_char);
                return;
            }
        }
        if caret.has_selection() {
            if self.settings.surround_selection_on_quote_typed && self.try_surround(i, c) {
                return;
            }
            self.delete_selection(i);
        }
        let offset = self.caret(i).offset;
        let lang = self.ctx.language;
        let next = self.doc.char_at(offset);
        // Overtype a closer that has a partner to its left.
        if next == Some(c) {
            let bracket = self.settings.auto_pair_brackets && lang.opener_of(c).is_some();
            let quote = self.settings.auto_pair_quotes && lang.quotes.contains(&c);
            if (bracket && self.closer_has_partner(offset, c))
                || (quote && self.inside_literal(offset, c) && !self.escaped_before(offset))
            {
                self.caret_mut(i).move_to(offset + doc_len_char);
                return;
            }
        }
        let mut text = c.to_string();
        let mut paired = None;
        if self.settings.auto_pair_brackets {
            if let Some(close) = lang.closer_of(c) {
                if self.should_pair_bracket(offset, c, close) {
                    text.push(close);
                    paired = Some(close);
                }
            }
        }
        if paired.is_none()
            && self.settings.auto_pair_quotes
            && lang.quotes.contains(&c)
            && self.should_pair_quote(offset, c)
        {
            text.push(c);
            paired = Some(c);
        }
        self.replace(offset, offset, &text);
        self.caret_mut(i).move_to(offset + doc_len_char);
        if let Some(close) = paired {
            self.model.tab_out.push(TabOutScope {
                start: offset + doc_len_char,
                end: offset + doc_len_char + close.len_utf8(),
            });
        }
    }

    fn count_in_doc(&self, c: char) -> usize {
        self.doc.text().chars().filter(|x| *x == c).count()
    }

    fn should_pair_bracket(&self, offset: usize, open: char, close: char) -> bool {
        let lang = self.ctx.language;
        let allowed = match self.doc.char_at(offset) {
            None => true,
            Some(n) => {
                n.is_whitespace()
                    || matches!(n, ')' | ']' | '}' | ';' | ',' | '.' | '>' | ':')
                    || lang.opener_of(n).is_some()
            }
        };
        allowed && self.count_in_doc(open) >= self.count_in_doc(close)
    }

    /// Whether `closer` at `offset` closes an opener somewhere to its left.
    fn closer_has_partner(&self, offset: usize, closer: char) -> bool {
        let Some(opener) = self.ctx.language.opener_of(closer) else {
            return false;
        };
        let mut depth = 0usize;
        for c in self.doc.text()[..offset].chars().rev() {
            if c == closer {
                depth += 1;
            } else if c == opener {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
        }
        false
    }

    /// Quote characters before `offset` on its line that are not escaped.
    fn quotes_before(&self, offset: usize, quote: char) -> usize {
        let line = self.doc.line_number(offset);
        let start = self.doc.line_start_offset(line);
        let mut count = 0;
        let mut escaped = false;
        for c in self.doc.slice(start, offset).chars() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                count += 1;
            }
        }
        count
    }

    fn inside_literal(&self, offset: usize, quote: char) -> bool {
        self.quotes_before(offset, quote) % 2 == 1
    }

    /// Whether the character at `offset` is escaped by an odd run of backslashes.
    fn escaped_before(&self, offset: usize) -> bool {
        let backslashes = self
            .doc
            .slice(0, offset)
            .chars()
            .rev()
            .take_while(|c| *c == '\\')
            .count();
        backslashes % 2 == 1
    }

    fn should_pair_quote(&self, offset: usize, quote: char) -> bool {
        if self.inside_literal(offset, quote) || self.escaped_before(offset) {
            return false;
        }
        if self.doc.char_at(offset).is_some_and(is_identifier_part) {
            return false;
        }
        if quote == '\'' && self.doc.char_before(offset).is_some_and(is_identifier_part) {
            return false;
        }
        true
    }

    /// Wraps the selection of caret `i` when a bracket or quote is typed (spec 3.3).
    fn try_surround(&mut self, i: usize, c: char) -> bool {
        let (open, close) = if let Some((o, cl)) = WRAP_OPEN.iter().find(|(o, _)| *o == c) {
            (*o, *cl)
        } else if WRAP_QUOTES.contains(&c) {
            (c, c)
        } else {
            return false;
        };
        let caret = self.caret(i).clone();
        let (start, end) = (caret.selection_start(), caret.selection_end());
        let selected = self.doc.slice(start, end).to_owned();
        if open == '<' && selected.chars().all(|x| matches!(x, '<' | '>' | '=' | '!')) {
            return false;
        }
        let first = selected.chars().next();
        let last = selected.chars().next_back();
        let forward = caret.offset >= caret.anchor;
        // Replace an existing pair of the same kind.
        let replaced = match (first, last) {
            (Some(f), Some(l)) if selected.chars().count() >= 2 => {
                let quote_pair = WRAP_QUOTES.contains(&f) && f == l && WRAP_QUOTES.contains(&open);
                let bracket_pair = WRAP_OPEN.iter().any(|(o, cl)| *o == f && *cl == l)
                    && WRAP_OPEN.iter().any(|(o, _)| *o == open);
                (quote_pair || bracket_pair).then(|| {
                    let inner: String = {
                        let mut chars = selected.chars();
                        chars.next();
                        chars.next_back();
                        chars.collect()
                    };
                    (inner, f.len_utf8() + l.len_utf8())
                })
            }
            (Some(f), Some(_))
                if selected.chars().count() == 1
                    && WRAP_QUOTES.contains(&f)
                    && WRAP_QUOTES.contains(&open) =>
            {
                Some((String::new(), f.len_utf8()))
            }
            _ => None,
        };
        let text = match replaced {
            Some((inner, _)) => format!("{open}{inner}{close}"),
            None => format!("{open}{selected}{close}"),
        };
        self.replace(start, end, &text);
        // The wrapped text stays selected, inside the new delimiters.
        let inner_start = start + open.len_utf8();
        let inner_end = start + text.len() - close.len_utf8();
        let c = self.caret_mut(i);
        if forward {
            c.select(inner_start, inner_end);
        } else {
            c.select(inner_end, inner_start);
        }
        true
    }

    // ---- Enter ------------------------------------------------------------------------

    /// The whitespace of the line of `offset` up to `offset`.
    fn indent_before(&self, offset: usize) -> String {
        let line = self.doc.line_number(offset);
        let start = self.doc.line_start_offset(line);
        let end = (start + indent_len(self.doc, line)).min(offset);
        self.doc.slice(start, end).to_owned()
    }

    /// The text Enter inserts at `offset` and where the caret ends relative to `offset`.
    fn enter_text(&self, offset: usize) -> (String, usize) {
        let indent = self.indent_before(offset);
        let plain = format!("\n{indent}");
        if !self.settings.smart_indent_on_enter {
            let len = plain.len();
            return (plain, len);
        }
        let lang = self.ctx.language;
        let Some(before) = self.doc.char_before(offset) else {
            let len = plain.len();
            return (plain, len);
        };
        let Some(closer) = lang.closer_of(before) else {
            let len = plain.len();
            return (plain, len);
        };
        let unit = self.settings.indent_unit();
        let middle = format!("\n{indent}{unit}");
        let next = self.doc.char_at(offset);
        if next == Some(closer) {
            let len = middle.len();
            return (format!("{middle}\n{indent}"), len);
        }
        let line = self.doc.line_number(offset);
        let rest_blank = self
            .doc
            .slice(offset, self.doc.line_end_offset(line))
            .trim()
            .is_empty();
        if before == '{'
            && lang.close_curly_on_enter
            && rest_blank
            && self.count_in_doc('{') > self.count_in_doc('}')
        {
            let len = middle.len();
            return (format!("{middle}\n{indent}}}"), len);
        }
        let len = middle.len();
        (middle, len)
    }

    fn enter_at(&mut self, i: usize) {
        self.pad_virtual(i);
        self.delete_selection(i);
        let offset = self.caret(i).offset;
        if self.model.overwrite {
            let line = self.doc.line_number(offset);
            if line + 1 < self.doc.line_count() {
                let next = self.doc.line_start_offset(line + 1);
                self.caret_mut(i).move_to(next);
            } else {
                let end = self.doc.len();
                self.replace(end, end, "\n");
                self.caret_mut(i).move_to(end + 1);
            }
            return;
        }
        let (text, caret_at) = self.enter_text(offset);
        self.replace(offset, offset, &text);
        self.caret_mut(i).move_to(offset + caret_at);
    }

    /// `EditorEnter`.
    pub(super) fn enter(&mut self) {
        for i in (0..self.n()).rev() {
            self.enter_at(i);
        }
    }

    /// `EditorStartNewLine`: from anywhere in the line, a new line below.
    pub(super) fn start_new_line(&mut self) {
        for group in self.line_groups(None).into_iter().rev() {
            let first = group.members[0];
            self.caret_mut(first).remove_selection();
            let line = self.doc.line_number(self.caret(first).offset);
            let end = self.doc.line_end_offset(line);
            self.caret_mut(first).move_to(end);
            self.enter_at(first);
            let at = self.caret(first).offset;
            for &member in &group.members[1..] {
                self.caret_mut(member).move_to(at);
            }
        }
    }

    /// `EditorStartNewLineBefore`: an empty line above, with the indent of the current one.
    pub(super) fn start_new_line_before(&mut self) {
        for group in self.line_groups(None).into_iter().rev() {
            let line = group.first;
            let start = self.doc.line_start_offset(line);
            let indent = self
                .doc
                .slice(start, start + indent_len(self.doc, line))
                .to_owned();
            self.replace(start, start, &format!("{indent}\n"));
            for &member in &group.members {
                self.caret_mut(member).move_to(start + indent.len());
            }
        }
    }

    /// `EditorSplitLine` (Ctrl+Enter): the caret stays before the new line break.
    pub(super) fn split_line(&mut self) {
        for i in (0..self.n()).rev() {
            self.pad_virtual(i);
            self.delete_selection(i);
            let offset = self.caret(i).offset;
            let line = self.doc.line_number(offset);
            let start = self.doc.line_start_offset(line);
            if self.doc.slice(start, offset).trim().is_empty() {
                let indent = self.indent_before(offset);
                let at = start;
                self.replace(at, at, &format!("{indent}\n"));
                self.caret_mut(i).move_to(offset + indent.len() + 1);
            } else {
                let indent = self.indent_before(offset);
                self.replace(offset, offset, &format!("\n{indent}"));
                self.caret_mut(i).move_to(offset);
            }
        }
    }

    // ---- Tab ---------------------------------------------------------------------------

    fn whitespace_only_selection(&self, i: usize) -> bool {
        let c = self.caret(i);
        self.doc
            .slice(c.selection_start(), c.selection_end())
            .chars()
            .all(char::is_whitespace)
    }

    /// `EditorTab`.
    pub(super) fn tab(&mut self) {
        self.out.undo_group = UndoGroup::Tab;
        // Selections that are not only whitespace indent their lines.
        let mut indent_any = false;
        for i in 0..self.n() {
            if self.caret(i).has_selection() && !self.whitespace_only_selection(i) {
                indent_any = true;
            }
        }
        if indent_any {
            self.out.undo_group = UndoGroup::None;
        }
        // Lines with selections first (they do not move the other carets' text before them
        // in a way that matters: edits shift all carets).
        if indent_any {
            let unit = self.settings.indent_unit();
            let groups: Vec<_> = self
                .line_groups(Some(false))
                .into_iter()
                .filter(|g| {
                    g.members
                        .iter()
                        .any(|m| !self.whitespace_only_selection(*m))
                })
                .collect();
            for group in groups.into_iter().rev() {
                for line in (group.first..=group.last).rev() {
                    if self.doc.line_text(line).is_empty() {
                        continue;
                    }
                    let start = self.doc.line_start_offset(line);
                    self.replace(start, start, &unit);
                }
            }
        }
        for i in (0..self.n()).rev() {
            let caret = self.caret(i).clone();
            if caret.has_selection() && !self.whitespace_only_selection(i) {
                continue;
            }
            if !caret.has_selection() && self.try_tab_out(i) {
                continue;
            }
            self.pad_virtual(i);
            self.delete_selection(i);
            let offset = self.caret(i).offset;
            let text = self.tab_text(offset);
            self.replace(offset, offset, &text);
            self.caret_mut(i).move_to(offset + text.len());
        }
    }

    /// What Tab inserts at `offset`: spaces to the next indent multiple, or a tab.
    fn tab_text(&self, offset: usize) -> String {
        let line = self.doc.line_number(offset);
        let start = self.doc.line_start_offset(line);
        let use_tab = self.settings.use_tab_character
            && (!self.settings.smart_tabs
                || self.doc.slice(start, offset).chars().all(|c| c == '\t'));
        if use_tab {
            return "\t".to_owned();
        }
        let indent = self.settings.indent_size.max(1);
        let column = column_of(self.doc, offset, self.tab_size());
        " ".repeat(indent - column % indent)
    }

    fn try_tab_out(&mut self, i: usize) -> bool {
        if !self.settings.tab_exits_brackets_and_quotes {
            return false;
        }
        let offset = self.caret(i).offset;
        let found = self
            .model
            .tab_out
            .iter()
            .position(|scope| scope.start <= offset && offset < scope.end);
        let Some(index) = found else {
            return false;
        };
        let scope = self.model.tab_out.remove(index);
        let between = self.doc.slice(offset, scope.end);
        if between.is_empty() || between.chars().any(char::is_whitespace) {
            return false;
        }
        self.caret_mut(i).move_to(scope.end);
        true
    }

    /// `EditorUnindentSelection` (Shift+Tab).
    pub(super) fn unindent(&mut self) {
        let indent = self.settings.indent_size.max(1);
        for group in self.line_groups(None).into_iter().rev() {
            for line in (group.first..=group.last).rev() {
                let start = self.doc.line_start_offset(line);
                let text = self.doc.line_text(line);
                let remove = if text.starts_with('\t') {
                    1
                } else {
                    text.chars().take(indent).take_while(|c| *c == ' ').count()
                };
                if remove > 0 {
                    self.replace(start, start + remove, "");
                }
            }
        }
    }

    // ---- Backspace and Delete ------------------------------------------------------

    /// `EditorBackSpace`.
    pub(super) fn backspace(&mut self) {
        self.out.undo_group = UndoGroup::Backspace;
        for i in (0..self.n()).rev() {
            self.backspace_at(i);
        }
        if self.out.edits.iter().any(|e| e.removed.contains('\n')) {
            self.out.undo_group = UndoGroup::None;
        }
    }

    fn backspace_at(&mut self, i: usize) {
        let caret = self.caret(i).clone();
        if caret.virtual_spaces > 0 {
            self.caret_mut(i).virtual_spaces -= 1;
            return;
        }
        if caret.has_selection() {
            self.delete_selection(i);
            return;
        }
        let offset = caret.offset;
        if offset == 0 {
            return;
        }
        let lang = self.ctx.language;
        let prev = self.doc.char_before(offset);
        let next = self.doc.char_at(offset);
        if let (Some(p), Some(n)) = (prev, next) {
            let bracket = self.settings.auto_pair_brackets && lang.closer_of(p) == Some(n);
            let quote = self.settings.auto_pair_quotes && p == n && lang.quotes.contains(&p);
            if bracket || quote {
                let start = offset - p.len_utf8();
                self.replace(start, offset + n.len_utf8(), "");
                self.caret_mut(i).move_to(start);
                return;
            }
        }
        let line = self.doc.line_number(offset);
        let line_start = self.doc.line_start_offset(line);
        let mut start = self.doc.prev_offset(offset);
        if self.settings.smart_backspace == SmartBackspace::Indent
            && offset > line_start
            && self.doc.slice(line_start, offset).chars().all(|c| c == ' ')
        {
            let indent = self.settings.indent_size.max(1);
            let column = offset - line_start;
            let target = (column - 1) / indent * indent;
            start = line_start + target;
        }
        self.replace(start, offset, "");
        self.caret_mut(i).move_to(start);
    }

    /// `EditorDelete`.
    pub(super) fn delete(&mut self) {
        for i in (0..self.n()).rev() {
            let caret = self.caret(i).clone();
            if caret.has_selection() {
                self.delete_selection(i);
                continue;
            }
            self.pad_virtual(i);
            let offset = self.caret(i).offset;
            let end = self.doc.next_offset(offset);
            self.replace(offset, end, "");
            self.caret_mut(i).move_to(offset);
        }
    }

    /// Ctrl+Backspace (`to_end == false`) and Ctrl+Delete.
    pub(super) fn delete_to_word(&mut self, to_end: bool) {
        let camel = self.settings.camel_humps;
        for i in (0..self.n()).rev() {
            let caret = self.caret(i).clone();
            if caret.has_selection() {
                self.delete_selection(i);
                continue;
            }
            let (start, end) = if to_end {
                range_to_word_end(self.doc, caret.offset, camel)
            } else {
                range_to_word_start(self.doc, caret.offset, camel)
            };
            self.replace(start, end, "");
            self.caret_mut(i).move_to(start);
        }
    }
}
