//! Undo and redo with IntelliJ's command grouping (spec 3.12).

use super::caret::{CaretModel, CaretSnapshot};
use super::document::{Document, Edit};

/// The command group an action belongs to (IntelliJ `CommandProcessor` group id).
///
/// Consecutive commands of the same group merge into one undo step; [`UndoGroup::None`]
/// commands are always a step of their own and end any run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoGroup {
    /// No group: Enter, paste, line actions, ... each is its own step.
    None,
    /// Typed characters: a whole typing run is one step, however long the pauses.
    Typing,
    /// Backspace: consecutive Backspaces merge; one that joins lines is [`UndoGroup::None`].
    Backspace,
    /// Tab: consecutive Tabs merge.
    Tab,
}

/// One undo step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoEntry {
    /// The changes, in the order they were applied.
    pub edits: Vec<Edit>,
    /// Carets before the first change (restored by undo).
    pub caret_before: CaretSnapshot,
    /// Carets after the last change (restored by redo, and by the caret-only undo).
    pub caret_after: CaretSnapshot,
    /// The group the step belongs to.
    pub group: UndoGroup,
}

/// What an undo or redo request did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoResult {
    /// Nothing to undo or redo.
    Nothing,
    /// Only the caret and selection moved back to where the last edit left them; the text is
    /// unchanged (spec 3.12: the first Ctrl+Z after pure caret movement). The viewport
    /// should scroll to the caret.
    CaretRestored,
    /// A step was undone or redone; these are the edits applied, in order.
    Applied(Vec<Edit>),
}

/// The undo stack of one document (IntelliJ `UndoManager`), 100 steps by default.
///
/// The caller feeds it every command with [`UndoManager::record`] (the
/// [`Editor`](super::Editor) does this for its actions).
#[derive(Clone, Debug)]
pub struct UndoManager {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
    open_group: UndoGroup,
    baseline: Option<CaretSnapshot>,
    /// Maximum number of undo steps.
    pub limit: usize,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(100)
    }
}

impl UndoManager {
    /// A manager keeping at most `limit` steps.
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            open_group: UndoGroup::None,
            baseline: None,
            limit: limit.max(1),
        }
    }

    /// Forgets all history (a new text was loaded).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.open_group = UndoGroup::None;
        self.baseline = None;
    }

    /// Steps that can be undone.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// Steps that can be redone.
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    /// Whether Undo would do anything (a step, or a caret move to restore).
    pub fn can_undo(&self, carets: &CaretModel) -> bool {
        !self.undo.is_empty() || self.caret_moved(carets)
    }

    /// Whether Redo would do anything.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The steps, oldest first.
    pub fn entries(&self) -> &[UndoEntry] {
        &self.undo
    }

    /// Ends the current group: the next command starts a new step.
    pub fn break_group(&mut self) {
        self.open_group = UndoGroup::None;
    }

    fn caret_moved(&self, carets: &CaretModel) -> bool {
        self.baseline
            .as_ref()
            .is_some_and(|baseline| *baseline != carets.snapshot())
    }

    /// Records one command: its edits and the caret states around it.
    ///
    /// It merges into the previous step when both have the same group other than
    /// [`UndoGroup::None`] and the carets have not moved since the previous step (a click or
    /// arrow key in between ends the run).
    pub fn record(
        &mut self,
        edits: Vec<Edit>,
        caret_before: CaretSnapshot,
        caret_after: CaretSnapshot,
        group: UndoGroup,
    ) {
        if edits.is_empty() {
            return;
        }
        self.redo.clear();
        let unmoved = self.baseline.as_ref() == Some(&caret_before);
        let merge = group != UndoGroup::None
            && self.open_group == group
            && unmoved
            && !self.undo.is_empty();
        if merge {
            if let Some(top) = self.undo.last_mut() {
                top.edits.extend(edits);
                top.caret_after = caret_after.clone();
            }
        } else {
            self.undo.push(UndoEntry {
                edits,
                caret_before,
                caret_after: caret_after.clone(),
                group,
            });
            if self.undo.len() > self.limit {
                self.undo.remove(0);
            }
        }
        self.open_group = group;
        self.baseline = Some(caret_after);
    }

    /// Tells the manager where the carets are after something that is not a command (an
    /// external caret change): the baseline of the caret-only undo.
    pub fn set_baseline(&mut self, carets: &CaretModel) {
        self.baseline = Some(carets.snapshot());
    }

    /// Undo (`$Undo`). If the carets moved since the last edit, the first call only restores
    /// them to where that edit left them.
    pub fn undo(&mut self, doc: &mut Document, carets: &mut CaretModel) -> UndoResult {
        if self.caret_moved(carets) {
            if let Some(baseline) = self.baseline.clone() {
                carets.restore(doc, &baseline);
                self.open_group = UndoGroup::None;
                return UndoResult::CaretRestored;
            }
        }
        let Some(entry) = self.undo.pop() else {
            return UndoResult::Nothing;
        };
        let mut applied = Vec::new();
        for edit in entry.edits.iter().rev() {
            let inverse = edit.inverted();
            let end = inverse.offset + inverse.removed.len();
            if let Some(done) = doc.replace_string(inverse.offset, end, &inverse.inserted) {
                applied.push(done);
            }
        }
        carets.restore(doc, &entry.caret_before);
        self.baseline = Some(carets.snapshot());
        self.open_group = UndoGroup::None;
        self.redo.push(entry);
        UndoResult::Applied(applied)
    }

    /// Redo (`$Redo`).
    pub fn redo(&mut self, doc: &mut Document, carets: &mut CaretModel) -> UndoResult {
        let Some(entry) = self.redo.pop() else {
            return UndoResult::Nothing;
        };
        let mut applied = Vec::new();
        for edit in &entry.edits {
            let end = edit.offset + edit.removed.len();
            if let Some(done) = doc.replace_string(edit.offset, end, &edit.inserted) {
                applied.push(done);
            }
        }
        carets.restore(doc, &entry.caret_after);
        self.baseline = Some(carets.snapshot());
        self.open_group = UndoGroup::None;
        self.undo.push(entry);
        UndoResult::Applied(applied)
    }
}
