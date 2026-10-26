//! Carets, the caret model and block (column) selection state.

use super::document::{Document, Edit};
use super::position::LogicalPosition;

/// One caret with its selection (IntelliJ `Caret`).
///
/// `offset` is the lead (where the caret is drawn), `anchor` the other end of the selection;
/// both equal when nothing is selected. Shift+movement keeps the anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Caret {
    /// The caret position (selection lead), a byte offset on a character boundary.
    pub offset: usize,
    /// The selection anchor.
    pub anchor: usize,
    /// The remembered column of vertical motion (IntelliJ keeps the desired X in pixels; in a
    /// monospace grid a visual column is the same thing). `None` until the first Up/Down;
    /// horizontal motion and edits clear it.
    pub desired_column: Option<usize>,
    /// Columns the caret stands beyond the line end (column mode and virtual space only).
    pub virtual_spaces: usize,
    /// Creation sequence number: the highest is the primary caret, the lowest the oldest.
    pub id: u64,
}

impl Caret {
    /// A caret without selection.
    pub fn at(offset: usize, id: u64) -> Self {
        Self {
            offset,
            anchor: offset,
            desired_column: None,
            virtual_spaces: 0,
            id,
        }
    }

    /// Whether a non-empty range is selected.
    pub fn has_selection(&self) -> bool {
        self.offset != self.anchor
    }

    /// Start of the selection (the caret offset when empty).
    pub fn selection_start(&self) -> usize {
        self.offset.min(self.anchor)
    }

    /// End of the selection (the caret offset when empty).
    pub fn selection_end(&self) -> usize {
        self.offset.max(self.anchor)
    }

    /// Puts the caret at `offset`, dropping the selection, the desired column and virtual
    /// space.
    pub fn move_to(&mut self, offset: usize) {
        self.offset = offset;
        self.anchor = offset;
        self.desired_column = None;
        self.virtual_spaces = 0;
    }

    /// Selects `anchor..lead` with the caret at `lead`.
    pub fn select(&mut self, anchor: usize, lead: usize) {
        self.anchor = anchor;
        self.offset = lead;
        self.desired_column = None;
        self.virtual_spaces = 0;
    }

    /// Removes the selection, leaving the caret where it is.
    pub fn remove_selection(&mut self) {
        self.anchor = self.offset;
    }
}

/// The saved state of every caret, for undo (offset and anchor).
pub type CaretSnapshot = Vec<(usize, usize)>;

/// A rectangular selection (IntelliJ column selection): two logical corners. Columns may lie
/// beyond line ends; the painter draws the continuous rectangle, the carets hold the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockSelection {
    /// The corner where the selection started.
    pub anchor: LogicalPosition,
    /// The corner that moves.
    pub lead: LogicalPosition,
}

impl BlockSelection {
    /// Lines covered, first to last.
    pub fn lines(&self) -> (usize, usize) {
        (
            self.anchor.line.min(self.lead.line),
            self.anchor.line.max(self.lead.line),
        )
    }

    /// Columns covered, left to right.
    pub fn columns(&self) -> (usize, usize) {
        (
            self.anchor.column.min(self.lead.column),
            self.anchor.column.max(self.lead.column),
        )
    }
}

/// A closer the editor inserted by itself that Tab can still jump over (spec 3.4). A range
/// marker: it grows with edits inside and ends when the caret leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabOutScope {
    /// Offset right after the opener.
    pub start: usize,
    /// Offset right after the closer: where Tab jumps to.
    pub end: usize,
}

/// State of "Select next occurrence" between presses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OccurrenceState {
    /// Search whole words only (the first press started from a bare word).
    pub whole_word: bool,
    /// The previous press found nothing after the caret; the next press searches from the top.
    pub wrap_pending: bool,
}

/// All carets of an editor (IntelliJ `CaretModel`), kept sorted by position.
///
/// The primary caret is the most recently added one. Carets that become identical or whose
/// selections overlap are merged ([`CaretModel::normalize`]); at most
/// [`CaretModel::max_carets`] exist.
#[derive(Clone, Debug)]
pub struct CaretModel {
    carets: Vec<Caret>,
    next_id: u64,
    /// Overwrite mode (the Insert key toggles it).
    pub overwrite: bool,
    /// Column selection mode (Alt+Shift+Insert).
    pub column_mode: bool,
    /// The rectangle of the current column selection.
    pub block: Option<BlockSelection>,
    /// Live tab-out scopes.
    pub tab_out: Vec<TabOutScope>,
    /// "Select next occurrence" state.
    pub occurrence: OccurrenceState,
    /// Maximum number of carets.
    pub max_carets: usize,
    chain: Option<(CaretSnapshot, Vec<CaretSnapshot>)>,
}

impl Default for CaretModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CaretModel {
    /// One caret at offset 0.
    pub fn new() -> Self {
        Self {
            carets: vec![Caret::at(0, 0)],
            next_id: 1,
            overwrite: false,
            column_mode: false,
            block: None,
            tab_out: Vec::new(),
            occurrence: OccurrenceState::default(),
            max_carets: 1000,
            chain: None,
        }
    }

    pub(super) fn next_caret_id(&mut self) -> u64 {
        self.fresh_id()
    }

    /// Remembers an extend-selection step so Shrink can undo it exactly.
    pub(super) fn push_selection_chain(&mut self, before: CaretSnapshot, after: CaretSnapshot) {
        let mut history = match self.chain.take() {
            Some((result, history)) if result == before => history,
            _ => Vec::new(),
        };
        history.push(before);
        self.chain = Some((after, history));
    }

    /// The selection before the last extend step, if the carets still are where it left them.
    pub(super) fn pop_selection_chain(&mut self) -> Option<CaretSnapshot> {
        let (result, mut history) = self.chain.take()?;
        if result != self.snapshot() {
            return None;
        }
        let previous = history.pop()?;
        if !history.is_empty() {
            self.chain = Some((previous.clone(), history));
        }
        Some(previous)
    }

    fn fresh_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// The carets, sorted by position.
    pub fn carets(&self) -> &[Caret] {
        &self.carets
    }

    /// The carets, mutable. Call [`CaretModel::normalize`] after changing positions.
    pub fn carets_mut(&mut self) -> &mut Vec<Caret> {
        &mut self.carets
    }

    /// Number of carets (IntelliJ `getCaretCount`), at least 1.
    pub fn caret_count(&self) -> usize {
        self.carets.len()
    }

    /// Index of the primary caret: the most recently added.
    pub fn primary_index(&self) -> usize {
        self.carets
            .iter()
            .enumerate()
            .max_by_key(|(_, caret)| caret.id)
            .map_or(0, |(index, _)| index)
    }

    /// The primary caret (IntelliJ `getPrimaryCaret`).
    pub fn primary(&self) -> &Caret {
        &self.carets[self.primary_index()]
    }

    /// The primary caret, mutable.
    pub fn primary_mut(&mut self) -> &mut Caret {
        let index = self.primary_index();
        &mut self.carets[index]
    }

    /// Index of the oldest caret.
    pub fn oldest_index(&self) -> usize {
        self.carets
            .iter()
            .enumerate()
            .min_by_key(|(_, caret)| caret.id)
            .map_or(0, |(index, _)| index)
    }

    /// Replaces all carets by one at `offset`.
    pub fn set_caret(&mut self, doc: &Document, offset: usize) {
        let id = self.fresh_id();
        self.carets = vec![Caret::at(doc.snap(offset), id)];
        self.reset_modes_state();
    }

    /// Replaces all carets by one with the selection `anchor..lead`.
    pub fn set_selection(&mut self, doc: &Document, anchor: usize, lead: usize) {
        let id = self.fresh_id();
        let mut caret = Caret::at(doc.snap(lead), id);
        caret.anchor = doc.snap(anchor);
        self.carets = vec![caret];
        self.reset_modes_state();
    }

    fn reset_modes_state(&mut self) {
        self.block = None;
        self.tab_out.clear();
        self.occurrence = OccurrenceState::default();
    }

    /// Adds a caret at `offset`, which becomes the primary caret (Alt+Shift+click). Returns
    /// whether a caret was added (not at the maximum, not on an existing caret).
    pub fn add_caret(&mut self, doc: &Document, offset: usize) -> bool {
        self.add_caret_with_selection(doc, offset, offset)
    }

    /// Adds a caret with a selection (`anchor..lead`).
    pub fn add_caret_with_selection(&mut self, doc: &Document, anchor: usize, lead: usize) -> bool {
        if self.carets.len() >= self.max_carets {
            return false;
        }
        let (anchor, lead) = (doc.snap(anchor), doc.snap(lead));
        if self
            .carets
            .iter()
            .any(|caret| caret.offset == lead && caret.anchor == anchor)
        {
            return false;
        }
        let id = self.fresh_id();
        let mut caret = Caret::at(lead, id);
        caret.anchor = anchor;
        self.carets.push(caret);
        self.normalize();
        true
    }

    /// Removes the caret at `index`; the last caret stays. Returns whether one was removed.
    pub fn remove_caret(&mut self, index: usize) -> bool {
        if self.carets.len() <= 1 || index >= self.carets.len() {
            return false;
        }
        self.carets.remove(index);
        true
    }

    /// Removes the caret standing at `offset` (clicking an existing caret again).
    pub fn remove_caret_at(&mut self, offset: usize) -> bool {
        match self.carets.iter().position(|caret| caret.offset == offset) {
            Some(index) => self.remove_caret(index),
            None => false,
        }
    }

    /// Keeps only the primary caret (IntelliJ `removeSecondaryCarets`).
    pub fn remove_secondary_carets(&mut self) {
        let keep = self.primary_index();
        let caret = self.carets.swap_remove(keep);
        self.carets = vec![caret];
    }

    /// Runs `f` on every caret in document order (IntelliJ `runForEachCaret`), then merges.
    pub fn run_for_each_caret(&mut self, mut f: impl FnMut(&mut Caret)) {
        for caret in &mut self.carets {
            f(caret);
        }
        self.normalize();
    }

    /// Runs `f` on every caret from the last to the first, then merges.
    pub fn run_for_each_caret_reversed(&mut self, mut f: impl FnMut(&mut Caret)) {
        for caret in self.carets.iter_mut().rev() {
            f(caret);
        }
        self.normalize();
    }

    /// Clamps every offset into `doc` onto a character boundary.
    pub fn clamp_to(&mut self, doc: &Document) {
        for caret in &mut self.carets {
            caret.offset = doc.snap(caret.offset);
            caret.anchor = doc.snap(caret.anchor);
        }
        let len = doc.len();
        self.tab_out.retain(|scope| scope.end <= len);
        self.normalize();
    }

    /// Sorts the carets and merges those that coincide or whose selections overlap.
    ///
    /// Merged carets become one covering both selections; it keeps the newer caret's id and
    /// direction, so the primary caret survives.
    pub fn normalize(&mut self) {
        self.carets
            .sort_by_key(|caret| (caret.selection_start(), caret.selection_end(), caret.id));
        let mut merged: Vec<Caret> = Vec::with_capacity(self.carets.len());
        for caret in self.carets.drain(..) {
            if let Some(last) = merged.last_mut() {
                let same = last.offset == caret.offset;
                let overlap = last.has_selection()
                    && caret.has_selection()
                    && caret.selection_start() < last.selection_end();
                let inside =
                    caret.offset > last.selection_start() && caret.offset < last.selection_end();
                if same || overlap || inside {
                    let start = last.selection_start().min(caret.selection_start());
                    let end = last.selection_end().max(caret.selection_end());
                    let newer = if caret.id > last.id {
                        caret.clone()
                    } else {
                        last.clone()
                    };
                    let forward = newer.offset >= newer.anchor;
                    let mut result = newer;
                    if forward {
                        result.anchor = start;
                        result.offset = end;
                    } else {
                        result.anchor = end;
                        result.offset = start;
                    }
                    *last = result;
                    continue;
                }
            }
            merged.push(caret);
        }
        if merged.is_empty() {
            merged.push(Caret::at(0, self.next_id));
            self.next_id += 1;
        }
        self.carets = merged;
    }

    /// Shifts every caret and tab-out scope by an edit (range-marker semantics).
    pub fn apply_edit(&mut self, edit: &Edit) {
        for caret in &mut self.carets {
            caret.offset = edit.shift(caret.offset);
            caret.anchor = edit.shift(caret.anchor);
        }
        for scope in &mut self.tab_out {
            scope.start = edit.shift(scope.start);
            scope.end = edit.shift(scope.end);
        }
    }

    /// The saved state of all carets.
    pub fn snapshot(&self) -> CaretSnapshot {
        self.carets
            .iter()
            .map(|caret| (caret.offset, caret.anchor))
            .collect()
    }

    /// Restores a snapshot: the first caret keeps the oldest id, the last is primary.
    pub fn restore(&mut self, doc: &Document, snapshot: &CaretSnapshot) {
        self.carets.clear();
        for (offset, anchor) in snapshot {
            let id = self.fresh_id();
            let mut caret = Caret::at(doc.snap(*offset), id);
            caret.anchor = doc.snap(*anchor);
            self.carets.push(caret);
        }
        self.reset_modes_state();
        self.normalize();
    }

    /// Selects the whole document with one caret at its end (`$SelectAll`).
    pub fn select_all(&mut self, doc: &Document) {
        self.set_selection(doc, 0, doc.len());
    }

    /// The selected text of every caret that has a selection, in order.
    pub fn selected_texts<'a>(&self, doc: &'a Document) -> Vec<&'a str> {
        self.carets
            .iter()
            .filter(|caret| caret.has_selection())
            .map(|caret| doc.slice(caret.selection_start(), caret.selection_end()))
            .collect()
    }

    /// Whether any caret has a selection (IntelliJ `SelectionModel.hasSelection` for all).
    pub fn has_selection(&self) -> bool {
        self.carets.iter().any(Caret::has_selection)
    }

    /// Moves the primary caret to `offset`, dropping other carets and selection (a plain
    /// click).
    pub fn click(&mut self, doc: &Document, offset: usize) {
        self.set_caret(doc, offset);
    }

    /// Extends the primary selection from its anchor to `offset` (Shift+click, drag).
    pub fn extend_selection_to(&mut self, doc: &Document, offset: usize) {
        let offset = doc.snap(offset);
        let caret = self.primary_mut();
        caret.offset = offset;
        caret.desired_column = None;
        caret.virtual_spaces = 0;
        self.normalize();
    }
}
