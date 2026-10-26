//! The editor actions, their context and their outcome.

use super::document::Edit;
use super::folding::FoldingModel;
use super::scrolling::ScrollType;
use super::settings::Language;
use super::undo::UndoGroup;

/// An editor action (IntelliJ `EditorAction`, identified by its action id, listed in the
/// docs of each variant). Every action runs for all carets.
///
/// Movement actions come in two forms: plain, and `...WithSelection` (the Shift variant),
/// which keeps the selection anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditorAction {
    // ---- caret movement -------------------------------------------------------------
    /// `EditorLeft`.
    Left,
    /// `EditorLeftWithSelection`.
    LeftWithSelection,
    /// `EditorRight`.
    Right,
    /// `EditorRightWithSelection`.
    RightWithSelection,
    /// `EditorUp`.
    Up,
    /// `EditorUpWithSelection`.
    UpWithSelection,
    /// `EditorDown`.
    Down,
    /// `EditorDownWithSelection`.
    DownWithSelection,
    /// `EditorPreviousWord`.
    PreviousWord,
    /// `EditorPreviousWordWithSelection`.
    PreviousWordWithSelection,
    /// `EditorNextWord`.
    NextWord,
    /// `EditorNextWordWithSelection`.
    NextWordWithSelection,
    /// `EditorLineStart`: smart Home.
    LineStart,
    /// `EditorLineStartWithSelection`.
    LineStartWithSelection,
    /// `EditorLineEnd`.
    LineEnd,
    /// `EditorLineEndWithSelection`.
    LineEndWithSelection,
    /// `EditorPageUp`.
    PageUp,
    /// `EditorPageUpWithSelection`.
    PageUpWithSelection,
    /// `EditorPageDown`.
    PageDown,
    /// `EditorPageDownWithSelection`.
    PageDownWithSelection,
    /// `EditorTextStart`.
    TextStart,
    /// `EditorTextStartWithSelection`.
    TextStartWithSelection,
    /// `EditorTextEnd`.
    TextEnd,
    /// `EditorTextEndWithSelection`.
    TextEndWithSelection,
    /// `EditorMoveToPageTop`: first fully visible line.
    MoveToPageTop,
    /// `EditorMoveToPageTopWithSelection`.
    MoveToPageTopWithSelection,
    /// `EditorMoveToPageBottom`: last fully visible line.
    MoveToPageBottom,
    /// `EditorMoveToPageBottomWithSelection`.
    MoveToPageBottomWithSelection,
    /// `EditorScrollUp`: scroll one line, the caret follows only when it would leave the view.
    ScrollUp,
    /// `EditorScrollDown`.
    ScrollDown,
    /// `EditorScrollToCenter`.
    ScrollToCenter,
    /// `EditorCodeBlockStart`: the enclosing unmatched opening brace.
    CodeBlockStart,
    /// `EditorCodeBlockStartWithSelection`.
    CodeBlockStartWithSelection,
    /// `EditorCodeBlockEnd`: the matching closing brace.
    CodeBlockEnd,
    /// `EditorCodeBlockEndWithSelection`.
    CodeBlockEndWithSelection,
    /// `EditorMatchBrace`: jump to the partner of the bracket at the caret.
    MatchBrace,

    // ---- selection and carets -------------------------------------------------------
    /// `$SelectAll`.
    SelectAll,
    /// `EditorSelectWord`: extend selection (plain-text chain).
    SelectWord,
    /// `EditorUnSelectWord`: shrink selection.
    UnSelectWord,
    /// `SelectNextOccurrence`.
    SelectNextOccurrence,
    /// `UnselectPreviousOccurrence`.
    UnselectPreviousOccurrence,
    /// `SelectAllOccurrences`.
    SelectAllOccurrences,
    /// `EditorCloneCaretAbove`.
    CloneCaretAbove,
    /// `EditorCloneCaretBelow`.
    CloneCaretBelow,
    /// `EditorAddCaretPerSelectedLine`.
    AddCaretPerSelectedLine,
    /// `EditorToggleColumnMode`.
    ToggleColumnMode,
    /// `EditorToggleInsertState`: insert/overwrite.
    ToggleInsertState,
    /// `EditorEscape`: clears selection and all but the oldest caret.
    Escape,

    // ---- editing --------------------------------------------------------------------
    /// A typed character (`TypedAction`).
    Type(char),
    /// `EditorEnter`.
    Enter,
    /// `EditorStartNewLine` (Shift+Enter).
    StartNewLine,
    /// `EditorStartNewLineBefore` (Ctrl+Alt+Enter).
    StartNewLineBefore,
    /// `EditorCompleteStatement` (Smart Enter): without a language processor, Start New Line.
    CompleteStatement,
    /// `EditorSplitLine` (Ctrl+Enter).
    SplitLine,
    /// `EditorTab` / `EditorIndentSelection`.
    Tab,
    /// `EditorUnindentSelection` (Shift+Tab).
    UnindentSelection,
    /// `EditorBackSpace`.
    Backspace,
    /// `EditorDelete` (`$Delete`).
    Delete,
    /// `EditorDeleteToWordStart`.
    DeleteToWordStart,
    /// `EditorDeleteToWordEnd`.
    DeleteToWordEnd,
    /// `EditorDeleteLine`.
    DeleteLine,
    /// `EditorCutLineEnd`: kill to the line end into the clipboard.
    CutLineEnd,
    /// `EditorDuplicate`.
    Duplicate,
    /// `EditorJoinLines`.
    JoinLines,
    /// `EditorToggleCase`.
    ToggleCase,
    /// `MoveLineUp`.
    MoveLineUp,
    /// `MoveLineDown`.
    MoveLineDown,
    /// `MoveStatementUp`: without a language, the line mover.
    MoveStatementUp,
    /// `MoveStatementDown`.
    MoveStatementDown,
    /// `CommentByLineComment`.
    CommentByLineComment,
    /// `CommentByBlockComment`.
    CommentByBlockComment,
    /// `$Cut`.
    Cut,
    /// `$Copy`.
    Copy,
    /// `$Paste`.
    Paste,
    /// `EditorPasteSimple`: paste without any processing.
    PasteSimple,
    /// `$Undo`: handled by [`Editor`](super::Editor).
    Undo,
    /// `$Redo`: handled by [`Editor`](super::Editor).
    Redo,

    // ---- folding (handled by Editor) -----------------------------------------------
    /// `ExpandRegion`.
    ExpandRegion,
    /// `CollapseRegion`.
    CollapseRegion,
    /// `ExpandRegionRecursively`.
    ExpandRegionRecursively,
    /// `CollapseRegionRecursively`.
    CollapseRegionRecursively,
    /// `ExpandAllRegions`.
    ExpandAllRegions,
    /// `CollapseAllRegions`.
    CollapseAllRegions,
}

impl EditorAction {
    /// The JetBrains action id.
    pub fn id(&self) -> &'static str {
        use EditorAction::*;
        match self {
            Left => "EditorLeft",
            LeftWithSelection => "EditorLeftWithSelection",
            Right => "EditorRight",
            RightWithSelection => "EditorRightWithSelection",
            Up => "EditorUp",
            UpWithSelection => "EditorUpWithSelection",
            Down => "EditorDown",
            DownWithSelection => "EditorDownWithSelection",
            PreviousWord => "EditorPreviousWord",
            PreviousWordWithSelection => "EditorPreviousWordWithSelection",
            NextWord => "EditorNextWord",
            NextWordWithSelection => "EditorNextWordWithSelection",
            LineStart => "EditorLineStart",
            LineStartWithSelection => "EditorLineStartWithSelection",
            LineEnd => "EditorLineEnd",
            LineEndWithSelection => "EditorLineEndWithSelection",
            PageUp => "EditorPageUp",
            PageUpWithSelection => "EditorPageUpWithSelection",
            PageDown => "EditorPageDown",
            PageDownWithSelection => "EditorPageDownWithSelection",
            TextStart => "EditorTextStart",
            TextStartWithSelection => "EditorTextStartWithSelection",
            TextEnd => "EditorTextEnd",
            TextEndWithSelection => "EditorTextEndWithSelection",
            MoveToPageTop => "EditorMoveToPageTop",
            MoveToPageTopWithSelection => "EditorMoveToPageTopWithSelection",
            MoveToPageBottom => "EditorMoveToPageBottom",
            MoveToPageBottomWithSelection => "EditorMoveToPageBottomWithSelection",
            ScrollUp => "EditorScrollUp",
            ScrollDown => "EditorScrollDown",
            ScrollToCenter => "EditorScrollToCenter",
            CodeBlockStart => "EditorCodeBlockStart",
            CodeBlockStartWithSelection => "EditorCodeBlockStartWithSelection",
            CodeBlockEnd => "EditorCodeBlockEnd",
            CodeBlockEndWithSelection => "EditorCodeBlockEndWithSelection",
            MatchBrace => "EditorMatchBrace",
            SelectAll => "$SelectAll",
            SelectWord => "EditorSelectWord",
            UnSelectWord => "EditorUnSelectWord",
            SelectNextOccurrence => "SelectNextOccurrence",
            UnselectPreviousOccurrence => "UnselectPreviousOccurrence",
            SelectAllOccurrences => "SelectAllOccurrences",
            CloneCaretAbove => "EditorCloneCaretAbove",
            CloneCaretBelow => "EditorCloneCaretBelow",
            AddCaretPerSelectedLine => "EditorAddCaretPerSelectedLine",
            ToggleColumnMode => "EditorToggleColumnMode",
            ToggleInsertState => "EditorToggleInsertState",
            Escape => "EditorEscape",
            Type(_) => "EditorChar",
            Enter => "EditorEnter",
            StartNewLine => "EditorStartNewLine",
            StartNewLineBefore => "EditorStartNewLineBefore",
            CompleteStatement => "EditorCompleteStatement",
            SplitLine => "EditorSplitLine",
            Tab => "EditorTab",
            UnindentSelection => "EditorUnindentSelection",
            Backspace => "EditorBackSpace",
            Delete => "EditorDelete",
            DeleteToWordStart => "EditorDeleteToWordStart",
            DeleteToWordEnd => "EditorDeleteToWordEnd",
            DeleteLine => "EditorDeleteLine",
            CutLineEnd => "EditorCutLineEnd",
            Duplicate => "EditorDuplicate",
            JoinLines => "EditorJoinLines",
            ToggleCase => "EditorToggleCase",
            MoveLineUp => "MoveLineUp",
            MoveLineDown => "MoveLineDown",
            MoveStatementUp => "MoveStatementUp",
            MoveStatementDown => "MoveStatementDown",
            CommentByLineComment => "CommentByLineComment",
            CommentByBlockComment => "CommentByBlockComment",
            Cut => "$Cut",
            Copy => "$Copy",
            Paste => "$Paste",
            PasteSimple => "EditorPasteSimple",
            Undo => "$Undo",
            Redo => "$Redo",
            ExpandRegion => "ExpandRegion",
            CollapseRegion => "CollapseRegion",
            ExpandRegionRecursively => "ExpandRegionRecursively",
            CollapseRegionRecursively => "CollapseRegionRecursively",
            ExpandAllRegions => "ExpandAllRegions",
            CollapseAllRegions => "CollapseAllRegions",
        }
    }
}

/// What the clipboard holds, as far as the editor cares.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClipboardContent {
    /// The text (line breaks are normalized on paste).
    pub text: String,
    /// The text is whole lines copied with an empty selection (IntelliJ "copied from empty
    /// selection"): paste puts it above the caret line.
    pub whole_line: bool,
}

impl ClipboardContent {
    /// Ordinary text.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            whole_line: false,
        }
    }

    /// Whole lines copied from an empty selection.
    pub fn lines(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            whole_line: true,
        }
    }
}

static PLAIN_TEXT: Language = Language {
    line_comment: None,
    line_comment_space: false,
    block_comment: None,
    brackets: Vec::new(),
    quotes: Vec::new(),
    close_curly_on_enter: false,
};

/// What an action may need besides the model (IntelliJ `DataContext`).
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct ActionContext<'a> {
    /// Lines per page for PgUp/PgDn: whole visible lines.
    pub page_lines: usize,
    /// First visible line (for page top/bottom and one-line scrolling).
    pub first_visible_line: usize,
    /// Fully visible lines.
    pub visible_lines: usize,
    /// The clipboard, for paste.
    pub clipboard: Option<&'a ClipboardContent>,
    /// The language services.
    pub language: &'a Language,
    /// Fold regions, so motion skips collapsed text.
    pub folding: Option<&'a FoldingModel>,
}

impl<'a> ActionContext<'a> {
    /// A context for `language` with a 30-line viewport and no clipboard.
    pub fn new(language: &'a Language) -> Self {
        Self {
            page_lines: 30,
            first_visible_line: 0,
            visible_lines: 30,
            clipboard: None,
            language,
            folding: None,
        }
    }

    /// A context for plain text.
    pub fn plain_text() -> ActionContext<'static> {
        ActionContext::new(&PLAIN_TEXT)
    }

    /// Sets the viewport: first visible line and number of fully visible lines.
    pub fn with_viewport(mut self, first_visible_line: usize, visible_lines: usize) -> Self {
        self.first_visible_line = first_visible_line;
        self.visible_lines = visible_lines;
        self.page_lines = visible_lines.saturating_sub(1).max(1);
        self
    }

    /// Sets the clipboard content for paste.
    pub fn with_clipboard(mut self, clipboard: &'a ClipboardContent) -> Self {
        self.clipboard = Some(clipboard);
        self
    }

    /// Sets the fold regions.
    pub fn with_folding(mut self, folding: &'a FoldingModel) -> Self {
        self.folding = Some(folding);
        self
    }
}

/// How the viewport should follow an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollRequest {
    /// Bring the primary caret into view.
    ToCaret {
        /// The policy.
        scroll_type: ScrollType,
        /// Animate the jump (false for typing, Home/End and document start/end).
        animate: bool,
    },
    /// Scroll by whole lines (positive: down); clamp with
    /// [`clamp_first_line`](super::scrolling::clamp_first_line).
    Lines(isize),
}

/// What an action did.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EditOutcome {
    /// The action applied (false: nothing for it to do here, so e.g. Escape can fall
    /// through to a popup).
    pub handled: bool,
    /// The text changed.
    pub text_changed: bool,
    /// The changes, in order.
    pub edits: Vec<Edit>,
    /// How to scroll.
    pub scroll: Option<ScrollRequest>,
    /// Text for the system clipboard (Copy, Cut).
    pub clipboard: Option<ClipboardContent>,
    /// A hint to show under the caret ("Not found, try again to search from top").
    pub hint: Option<String>,
    /// The undo group of the command.
    pub undo_group: UndoGroup,
}

impl EditOutcome {
    pub(crate) fn new() -> Self {
        Self {
            handled: true,
            text_changed: false,
            edits: Vec::new(),
            scroll: None,
            clipboard: None,
            hint: None,
            undo_group: UndoGroup::None,
        }
    }
}
