//! The action executor: shared machinery and caret movement (spec sections 1.1 to 1.4).

use super::action::{ActionContext, EditOutcome, EditorAction, ScrollRequest};
use super::caret::{Caret, CaretModel};
use super::document::{Document, Edit};
use super::lines::LineGroup;
use super::motion::{
    motion_left, motion_line_end, motion_line_start, motion_next_word, motion_prev_word,
    motion_right,
};
use super::position::{column_of, offset_at_column};
use super::scrolling::ScrollType;
use super::settings::EditorSettings;

/// Runs `action` for every caret (IntelliJ `EditorActionHandler.execute`).
///
/// Pure model logic: the document, the carets and the outcome are all the caller sees.
/// Offsets that are out of range or inside a character are clamped first, so no input
/// panics. [`EditorAction::Undo`], [`EditorAction::Redo`] and the folding actions need the
/// undo stack or the fold model; [`Editor`](super::Editor) runs them, here they report
/// `handled == false`.
pub fn execute(
    doc: &mut Document,
    carets: &mut CaretModel,
    settings: &EditorSettings,
    action: &EditorAction,
    ctx: &ActionContext<'_>,
) -> EditOutcome {
    carets.clamp_to(doc);
    carets.max_carets = settings.max_carets;
    let mut work = Work {
        doc,
        model: carets,
        settings,
        ctx,
        out: EditOutcome::new(),
        extending: false,
    };
    work.run(action);
    if work.out.text_changed {
        work.model.block = None;
    }
    work.model.normalize();
    work.out
}

/// What a motion computed for one caret.
pub(super) struct Move {
    /// The new caret offset.
    pub offset: usize,
    /// The desired column to keep (vertical motion); `None` resets it.
    pub desired: Option<usize>,
}

impl Move {
    pub(super) fn to(offset: usize) -> Self {
        Self {
            offset,
            desired: None,
        }
    }
}

pub(super) struct Work<'a> {
    pub doc: &'a mut Document,
    pub model: &'a mut CaretModel,
    pub settings: &'a EditorSettings,
    pub ctx: &'a ActionContext<'a>,
    pub out: EditOutcome,
    pub extending: bool,
}

impl Work<'_> {
    pub(super) fn tab_size(&self) -> usize {
        self.settings.tab_size.max(1)
    }

    pub(super) fn n(&self) -> usize {
        self.model.caret_count()
    }

    pub(super) fn caret(&self, i: usize) -> &Caret {
        &self.model.carets()[i]
    }

    pub(super) fn caret_mut(&mut self, i: usize) -> &mut Caret {
        &mut self.model.carets_mut()[i]
    }

    /// Replaces a range, keeping every caret and range marker in place (shifted).
    pub(super) fn replace(&mut self, start: usize, end: usize, text: &str) -> Option<Edit> {
        let edit = self.doc.replace_string(start, end, text)?;
        self.model.apply_edit(&edit);
        self.out.text_changed = true;
        self.out.edits.push(edit.clone());
        Some(edit)
    }

    /// Deletes the selection of caret `i` (if any) and leaves the caret at its start.
    pub(super) fn delete_selection(&mut self, i: usize) {
        let caret = self.caret(i).clone();
        if caret.has_selection() {
            self.replace(caret.selection_start(), caret.selection_end(), "");
        }
        let start = caret.selection_start();
        self.caret_mut(i).move_to(start);
    }

    /// Materializes virtual space (spaces up to the caret) before text goes in there.
    pub(super) fn pad_virtual(&mut self, i: usize) {
        let caret = self.caret(i).clone();
        if caret.virtual_spaces > 0 {
            let pad = " ".repeat(caret.virtual_spaces);
            self.replace(caret.offset, caret.offset, &pad);
            let offset = caret.offset + pad.len();
            self.caret_mut(i).move_to(offset);
        }
    }

    /// The lines a caret touches: its selection's lines, without a last line that the
    /// selection only reaches at column 0.
    pub(super) fn line_range_of(&self, caret: &Caret) -> (usize, usize) {
        let start = caret.selection_start();
        let end = caret.selection_end();
        let first = self.doc.line_number(start);
        let mut last = self.doc.line_number(end);
        if caret.has_selection() && last > first && end == self.doc.line_start_offset(last) {
            last -= 1;
        }
        (first, last)
    }

    /// Carets grouped by the lines they touch, overlapping line ranges merged, ascending.
    pub(super) fn line_groups(&self, only_without_selection: Option<bool>) -> Vec<LineGroup> {
        let mut groups: Vec<LineGroup> = Vec::new();
        for (index, caret) in self.model.carets().iter().enumerate() {
            if let Some(without) = only_without_selection {
                if caret.has_selection() == without {
                    continue;
                }
            }
            let (first, last) = self.line_range_of(caret);
            match groups.last_mut() {
                Some(group) if first <= group.last => {
                    group.last = group.last.max(last);
                    group.members.push(index);
                }
                _ => groups.push(LineGroup {
                    first,
                    last,
                    members: vec![index],
                }),
            }
        }
        groups
    }

    fn run(&mut self, action: &EditorAction) {
        use EditorAction::*;
        let scroll = ScrollRequest::ToCaret {
            scroll_type: ScrollType::Relative,
            animate: true,
        };
        if self.model.column_mode {
            let step = match action {
                LeftWithSelection => Some((-1, 0)),
                RightWithSelection => Some((1, 0)),
                UpWithSelection => Some((0, -1)),
                DownWithSelection => Some((0, 1)),
                _ => None,
            };
            if let Some((dx, dy)) = step {
                self.block_step(dx, dy);
                self.out.scroll = Some(scroll);
                return;
            }
        }
        match action {
            Left => self.motion(false, motion_left),
            LeftWithSelection => self.motion(true, motion_left),
            Right => self.motion(false, motion_right),
            RightWithSelection => self.motion(true, motion_right),
            Up => self.motion(false, |w, c| w.vertical(c, -1)),
            UpWithSelection => self.motion(true, |w, c| w.vertical(c, -1)),
            Down => self.motion(false, |w, c| w.vertical(c, 1)),
            DownWithSelection => self.motion(true, |w, c| w.vertical(c, 1)),
            PreviousWord => self.motion(false, motion_prev_word),
            PreviousWordWithSelection => self.motion(true, motion_prev_word),
            NextWord => self.motion(false, motion_next_word),
            NextWordWithSelection => self.motion(true, motion_next_word),
            LineStart => {
                self.motion(false, motion_line_start);
                self.no_animation();
            }
            LineStartWithSelection => {
                self.motion(true, motion_line_start);
                self.no_animation();
            }
            LineEnd => {
                self.motion(false, motion_line_end);
                self.no_animation();
            }
            LineEndWithSelection => {
                self.motion(true, motion_line_end);
                self.no_animation();
            }
            PageUp => self.page(false, -1),
            PageUpWithSelection => self.page(true, -1),
            PageDown => self.page(false, 1),
            PageDownWithSelection => self.page(true, 1),
            TextStart => self.text_edge(false, false),
            TextStartWithSelection => self.text_edge(true, false),
            TextEnd => self.text_edge(false, true),
            TextEndWithSelection => self.text_edge(true, true),
            MoveToPageTop => self.motion(false, |w, c| w.page_edge(c, false)),
            MoveToPageTopWithSelection => self.motion(true, |w, c| w.page_edge(c, false)),
            MoveToPageBottom => self.motion(false, |w, c| w.page_edge(c, true)),
            MoveToPageBottomWithSelection => self.motion(true, |w, c| w.page_edge(c, true)),
            ScrollUp => self.scroll_lines(-1),
            ScrollDown => self.scroll_lines(1),
            ScrollToCenter => {
                self.out.scroll = Some(ScrollRequest::ToCaret {
                    scroll_type: ScrollType::Center,
                    animate: true,
                });
            }
            CodeBlockStart => self.motion(false, |w, c| w.code_block(c, false)),
            CodeBlockStartWithSelection => self.motion(true, |w, c| w.code_block(c, false)),
            CodeBlockEnd => self.motion(false, |w, c| w.code_block(c, true)),
            CodeBlockEndWithSelection => self.motion(true, |w, c| w.code_block(c, true)),
            MatchBrace => self.motion(false, |w, c| w.match_brace(c)),

            SelectAll => {
                self.model.select_all(self.doc);
                self.out.scroll = Some(scroll);
            }
            SelectWord => self.extend_selection(),
            UnSelectWord => self.shrink_selection(),
            SelectNextOccurrence => self.select_next_occurrence(),
            UnselectPreviousOccurrence => self.unselect_previous_occurrence(),
            SelectAllOccurrences => self.select_all_occurrences(),
            CloneCaretAbove => self.clone_caret(-1),
            CloneCaretBelow => self.clone_caret(1),
            AddCaretPerSelectedLine => self.add_caret_per_selected_line(),
            ToggleColumnMode => self.toggle_column_mode(),
            ToggleInsertState => self.model.overwrite = !self.model.overwrite,
            Escape => self.escape(),

            Type(c) => self.type_char(*c),
            Enter => self.enter(),
            StartNewLine | CompleteStatement => self.start_new_line(),
            StartNewLineBefore => self.start_new_line_before(),
            SplitLine => self.split_line(),
            Tab => self.tab(),
            UnindentSelection => self.unindent(),
            Backspace => self.backspace(),
            Delete => self.delete(),
            DeleteToWordStart => self.delete_to_word(false),
            DeleteToWordEnd => self.delete_to_word(true),
            DeleteLine => self.delete_lines(),
            CutLineEnd => self.cut_line_end(),
            Duplicate => self.duplicate(),
            JoinLines => self.join_lines(),
            ToggleCase => self.toggle_case(),
            MoveLineUp | MoveStatementUp => self.move_lines(-1),
            MoveLineDown | MoveStatementDown => self.move_lines(1),
            CommentByLineComment => self.comment_by_line(),
            CommentByBlockComment => self.comment_by_block(),
            Cut => self.copy(true),
            Copy => self.copy(false),
            Paste => self.paste(true),
            PasteSimple => self.paste(false),

            Undo
            | Redo
            | ExpandRegion
            | CollapseRegion
            | ExpandRegionRecursively
            | CollapseRegionRecursively
            | ExpandAllRegions
            | CollapseAllRegions => {
                self.out.handled = false;
            }
        }
        if self.out.scroll.is_none() && self.out.handled {
            self.out.scroll = Some(ScrollRequest::ToCaret {
                scroll_type: ScrollType::Relative,
                animate: !self.out.text_changed,
            });
        }
    }

    fn no_animation(&mut self) {
        self.out.scroll = Some(ScrollRequest::ToCaret {
            scroll_type: ScrollType::Relative,
            animate: false,
        });
    }

    /// Applies a motion to every caret. In column mode a plain motion collapses the block
    /// selection to the primary caret first; Shift+arrows grow the block.
    fn motion(&mut self, extend: bool, f: fn(&Work<'_>, &Caret) -> Move) {
        self.extending = extend;
        if !extend && self.model.block.is_some() {
            self.model.remove_secondary_carets();
            self.model.block = None;
        }
        for i in 0..self.n() {
            let caret = self.caret(i).clone();
            let target = f(self, &caret);
            let offset = self.fold_adjust(target.offset, target.offset >= caret.offset);
            let c = self.caret_mut(i);
            c.offset = offset;
            if !extend {
                c.anchor = offset;
            }
            c.desired_column = target.desired;
            c.virtual_spaces = 0;
        }
    }

    /// A caret cannot stand inside a collapsed fold: it lands on the near boundary of the
    /// direction it came from.
    fn fold_adjust(&self, offset: usize, forward: bool) -> usize {
        match self
            .ctx
            .folding
            .and_then(|f| f.collapsed_region_hiding(offset))
        {
            Some(region) if forward => region.end,
            Some(region) => region.start,
            None => offset,
        }
    }

    pub(super) fn vertical(&self, caret: &Caret, direction: isize) -> Move {
        let doc = &*self.doc;
        let base = if caret.has_selection() && self.is_plain_motion(caret) {
            if direction < 0 {
                caret.selection_start()
            } else {
                caret.selection_end()
            }
        } else {
            caret.offset
        };
        let tab = self.tab_size();
        let line = doc.line_number(base);
        let desired = caret
            .desired_column
            .unwrap_or_else(|| column_of(doc, base, tab) + caret.virtual_spaces);
        let target = match self.ctx.folding {
            Some(folds) => {
                let visual = folds.logical_to_visual_line(doc, line) as isize + direction;
                if visual < 0 || visual as usize >= folds.visual_line_count(doc) {
                    None
                } else {
                    Some(folds.visual_to_logical_line(doc, visual as usize))
                }
            }
            None => {
                let next = line as isize + direction;
                (next >= 0 && (next as usize) < doc.line_count()).then_some(next as usize)
            }
        };
        match target {
            Some(line) => Move {
                offset: offset_at_column(doc, line, desired, tab).0,
                desired: Some(desired),
            },
            // Up on the first line goes to the document start, Down on the last to its end.
            None if direction < 0 => Move::to(0),
            None => Move::to(doc.len()),
        }
    }

    /// Whether the motion is being run without Shift (selection collapses to its edge).
    fn is_plain_motion(&self, _caret: &Caret) -> bool {
        !self.extending
    }
}
