//! Copy, cut and paste (spec 3.8).

use super::action::{ClipboardContent, ScrollRequest};
use super::document::normalize_newlines;
use super::exec::Work;
use super::position::column_of;
use super::scrolling::ScrollType;
use super::settings::ReformatOnPaste;

impl Work<'_> {
    /// `$Copy` and `$Cut`. Without a selection the caret lines are copied (and cut).
    pub(super) fn copy(&mut self, cut: bool) {
        if self.model.has_selection() {
            let texts: Vec<String> = self
                .model
                .selected_texts(self.doc)
                .into_iter()
                .map(str::to_owned)
                .collect();
            self.out.clipboard = Some(ClipboardContent::text(texts.join("\n")));
            if cut {
                for i in (0..self.n()).rev() {
                    self.delete_selection(i);
                }
            }
            return;
        }
        if !self.settings.copy_cut_empty_selection_line {
            self.out.handled = false;
            return;
        }
        let groups = self.line_groups(None);
        let mut text = String::new();
        for group in &groups {
            for line in group.first..=group.last {
                text.push_str(self.doc.line_text(line));
                text.push('\n');
            }
        }
        self.out.clipboard = Some(ClipboardContent::lines(text));
        if cut {
            self.delete_lines();
        } else if !self.settings.skip_selecting_line_after_copy {
            // The copied line stays selected: line start to the next line start.
            for group in &groups {
                let start = self.doc.line_start_offset(group.first);
                let end = self.doc.line_end_with_break(group.last);
                for &member in &group.members {
                    self.caret_mut(member).select(start, end);
                }
            }
        }
    }

    /// `$Paste` (`process == true`) and `EditorPasteSimple`.
    pub(super) fn paste(&mut self, process: bool) {
        let Some(clip) = self.ctx.clipboard else {
            self.out.handled = false;
            return;
        };
        let text = normalize_newlines(&clip.text).into_owned();
        if text.is_empty() {
            self.out.handled = false;
            return;
        }
        self.out.scroll = Some(ScrollRequest::ToCaret {
            scroll_type: ScrollType::Relative,
            animate: false,
        });
        let n = self.n();
        // Whole lines copied from an empty selection go above the caret line.
        if clip.whole_line && self.settings.paste_line_above && !self.model.has_selection() {
            let lines: Vec<&str> = text
                .strip_suffix('\n')
                .unwrap_or(&text)
                .split('\n')
                .collect();
            let per_caret = n > 1 && lines.len() == n;
            let groups = self.line_groups(None);
            let count = groups.len();
            for (index, group) in groups.into_iter().enumerate().rev() {
                let insert = if per_caret && count == n {
                    format!("{}\n", lines[index])
                } else if text.ends_with('\n') {
                    text.clone()
                } else {
                    format!("{text}\n")
                };
                let start = self.doc.line_start_offset(group.first);
                let saved: Vec<(usize, usize)> = group
                    .members
                    .iter()
                    .map(|m| (self.caret(*m).offset, self.caret(*m).anchor))
                    .collect();
                self.replace(start, start, &insert);
                for (&member, (offset, anchor)) in group.members.iter().zip(saved) {
                    let c = self.caret_mut(member);
                    c.offset = offset + insert.len();
                    c.anchor = anchor + insert.len();
                }
            }
            return;
        }
        let parts: Vec<&str> = text.split('\n').collect();
        if self.model.column_mode && n == 1 && parts.len() > 1 && !self.model.has_selection() {
            self.paste_column(&parts);
            return;
        }
        let distribute = n > 1 && parts.len() == n;
        for i in (0..n).rev() {
            self.pad_virtual(i);
            let caret = self.caret(i).clone();
            let start = caret.selection_start();
            let piece = if distribute { parts[i] } else { &text };
            let piece = if process && self.settings.reformat_on_paste != ReformatOnPaste::None {
                self.indent_block(start, piece)
            } else {
                piece.to_owned()
            };
            self.replace(start, caret.selection_end(), &piece);
            self.caret_mut(i).move_to(start + piece.len());
        }
    }

    /// The "indent block" fallback of reformat-on-paste: lines 2..N get the caret column in
    /// spaces (spec 3.8). Skipped at column 0, for a blank first line and for one line.
    fn indent_block(&self, start: usize, text: &str) -> String {
        let column = column_of(self.doc, start, self.tab_size());
        let mut lines = text.split('\n');
        let first = lines.next().unwrap_or("");
        let rest: Vec<&str> = lines.collect();
        if column == 0 || first.trim().is_empty() || rest.is_empty() {
            return text.to_owned();
        }
        let pad = " ".repeat(column);
        let mut out = first.to_owned();
        for line in rest {
            out.push('\n');
            if !line.is_empty() {
                out.push_str(&pad);
            }
            out.push_str(line);
        }
        out
    }

    /// Column mode with one caret: each pasted line goes to the next line at the same column
    /// and every inserted piece gets its own caret.
    fn paste_column(&mut self, parts: &[&str]) {
        let tab = self.tab_size();
        let offset = self.caret(0).offset;
        let line0 = self.doc.line_number(offset);
        let column = column_of(self.doc, offset, tab) + self.caret(0).virtual_spaces;
        let mut ends = Vec::new();
        for (k, part) in parts.iter().enumerate() {
            let line = line0 + k;
            while line >= self.doc.line_count() {
                let end = self.doc.len();
                self.replace(end, end, "\n");
            }
            let end = self.doc.line_end_offset(line);
            let width = column_of(self.doc, end, tab);
            let pad = " ".repeat(column.saturating_sub(width));
            let (at, _) = super::position::offset_at_column(self.doc, line, column, tab);
            let text = format!("{pad}{part}");
            self.replace(at, at, &text);
            ends.push(at + text.len());
        }
        self.model.set_caret(self.doc, ends[0]);
        for end in &ends[1..] {
            self.model.add_caret(self.doc, *end);
        }
        self.model.column_mode = true;
    }
}
