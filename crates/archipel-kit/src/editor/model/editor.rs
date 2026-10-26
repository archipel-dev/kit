//! The whole model of one editor in one struct.

use super::action::{ActionContext, EditOutcome, EditorAction, ScrollRequest};
use super::caret::CaretModel;
use super::document::Document;
use super::exec::execute;
use super::folding::FoldingModel;
use super::scrolling::ScrollType;
use super::settings::EditorSettings;
use super::undo::{UndoGroup, UndoManager, UndoResult};

/// Everything one editor owns: document, carets, undo stack, folds and settings
/// (IntelliJ `Editor`). The caller keeps it between frames; nothing is global.
///
/// [`Editor::execute`] runs an action for all carets and records it for undo with the
/// command grouping of spec 3.12; Undo, Redo and the folding actions are handled here
/// because they need the undo stack and the fold model.
#[derive(Clone, Debug)]
pub struct Editor {
    /// The text.
    pub document: Document,
    /// The carets and the selection.
    pub carets: CaretModel,
    /// The undo history.
    pub undo: UndoManager,
    /// The fold regions.
    pub folding: FoldingModel,
    /// The settings.
    pub settings: EditorSettings,
}

impl Editor {
    /// An editor on `text` with the IntelliJ defaults and one caret at 0.
    pub fn new(text: &str) -> Self {
        Self::with_settings(text, EditorSettings::default())
    }

    /// An editor with explicit settings.
    pub fn with_settings(text: &str, settings: EditorSettings) -> Self {
        Self {
            document: Document::new(text),
            carets: CaretModel::new(),
            undo: UndoManager::new(settings.undo_limit),
            folding: FoldingModel::new(),
            settings,
        }
    }

    /// Replaces the text and forgets history and folds (a file was loaded).
    pub fn set_text(&mut self, text: &str) {
        self.document.set_text(text);
        self.carets.set_caret(&self.document, 0);
        self.undo.clear();
        self.folding.clear();
    }

    /// Replaces a range as one undo step of its own (an edit made by the application, such
    /// as a completion or a refactoring). Carets keep their text position.
    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        let before = self.carets.snapshot();
        if let Some(edit) = self.document.replace_string(start, end, text) {
            self.carets.apply_edit(&edit);
            self.carets.normalize();
            self.folding.apply_edit(&edit);
            self.undo
                .record(vec![edit], before, self.carets.snapshot(), UndoGroup::None);
        }
    }

    /// Runs `action` for every caret.
    pub fn execute(&mut self, action: &EditorAction, ctx: &ActionContext<'_>) -> EditOutcome {
        self.undo.limit = self.settings.undo_limit.max(1);
        self.carets.clamp_to(&self.document);
        match action {
            EditorAction::Undo => return self.run_undo(false),
            EditorAction::Redo => return self.run_undo(true),
            EditorAction::ExpandRegion
            | EditorAction::ExpandRegionRecursively
            | EditorAction::ExpandAllRegions
            | EditorAction::CollapseRegion
            | EditorAction::CollapseRegionRecursively
            | EditorAction::CollapseAllRegions => return self.run_fold(action),
            _ => {}
        }
        let before = self.carets.snapshot();
        let mut context = *ctx;
        if context.folding.is_none() {
            context.folding = Some(&self.folding);
        }
        let outcome = execute(
            &mut self.document,
            &mut self.carets,
            &self.settings,
            action,
            &context,
        );
        for edit in &outcome.edits {
            self.folding.apply_edit(edit);
        }
        self.folding.expand_around_carets(&self.carets);
        if outcome.text_changed {
            self.undo.record(
                outcome.edits.clone(),
                before,
                self.carets.snapshot(),
                outcome.undo_group,
            );
        }
        outcome
    }

    fn run_undo(&mut self, redo: bool) -> EditOutcome {
        let mut outcome = EditOutcome::new();
        let result = if redo {
            self.undo.redo(&mut self.document, &mut self.carets)
        } else {
            self.undo.undo(&mut self.document, &mut self.carets)
        };
        match result {
            UndoResult::Nothing => outcome.handled = false,
            UndoResult::CaretRestored => {}
            UndoResult::Applied(edits) => {
                for edit in &edits {
                    self.folding.apply_edit(edit);
                }
                outcome.text_changed = !edits.is_empty();
                outcome.edits = edits;
            }
        }
        self.folding.expand_around_carets(&self.carets);
        if outcome.handled {
            outcome.scroll = Some(ScrollRequest::ToCaret {
                scroll_type: ScrollType::Relative,
                animate: false,
            });
        }
        outcome
    }

    fn run_fold(&mut self, action: &EditorAction) -> EditOutcome {
        let mut outcome = EditOutcome::new();
        let offsets: Vec<usize> = self.carets.carets().iter().map(|c| c.offset).collect();
        match action {
            EditorAction::ExpandRegion | EditorAction::ExpandRegionRecursively => {
                let recursive = matches!(action, EditorAction::ExpandRegionRecursively);
                for offset in offsets {
                    self.folding.expand_at(offset, recursive);
                }
            }
            EditorAction::CollapseRegion | EditorAction::CollapseRegionRecursively => {
                let recursive = matches!(action, EditorAction::CollapseRegionRecursively);
                for offset in offsets {
                    self.folding.collapse_at(offset, recursive);
                }
            }
            EditorAction::ExpandAllRegions => self.folding.expand_all(),
            _ => self.folding.collapse_all(),
        }
        // A caret cannot stay inside a region that just collapsed: it moves to its start.
        for caret in self.carets.carets_mut() {
            for offset in [&mut caret.offset, &mut caret.anchor] {
                if let Some(region) = self.folding.collapsed_region_hiding(*offset) {
                    *offset = region.start;
                }
            }
        }
        self.carets.normalize();
        outcome.scroll = None;
        outcome
    }
}
