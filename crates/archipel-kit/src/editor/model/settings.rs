//! Editor settings (defaults of spec section 0) and the language hook.

/// Which word boundaries Ctrl+Arrow stops at in one direction (IntelliJ `CaretStopPolicy`
/// as used by `CaretStopOptions.wordBoundary`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordStop {
    /// The boundary of the word the caret is in: the word **end** going right, the word
    /// **start** going left. The IntelliJ default.
    Current,
    /// The boundary of the next word: word start going right, word end going left.
    Neighbor,
    /// Word starts only, both directions.
    Start,
    /// Word ends only, both directions.
    End,
    /// Both starts and ends.
    Both,
}

/// Where the line limit of a word motion lies (IntelliJ `CaretStopOptions.lineBoundary`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineStop {
    /// The motion cannot pass the **neighboring** line boundary: going right it stops at the
    /// start of the next line at the latest, going left at the end of the previous line. The
    /// IntelliJ default.
    Neighbor,
    /// The motion stops at the boundary of the **current** line (end going right, start going
    /// left); from that boundary it crosses to the neighbor line.
    Current,
    /// Lines are not boundaries: the motion runs to the next word or the document end.
    None,
}

/// IntelliJ's `CaretStopOptions`: where word motions stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaretStopOptions {
    /// Word boundary rule.
    pub word_boundary: WordStop,
    /// Line limit rule.
    pub line_boundary: LineStop,
}

impl CaretStopOptions {
    /// The persisted default: word boundary "current", line boundary "neighbor". Ctrl+Right
    /// lands at the end of the current word, Ctrl+Left at the start of the word.
    pub const DEFAULT: Self = Self {
        word_boundary: WordStop::Current,
        line_boundary: LineStop::Neighbor,
    };
    /// The "Windows" preset of IntelliJ: word starts both ways (line rule not in the spec:
    /// neighbor).
    pub const WINDOWS: Self = Self {
        word_boundary: WordStop::Start,
        line_boundary: LineStop::Neighbor,
    };
    /// The "Unix" preset of IntelliJ: current word boundary, no line stops.
    pub const UNIX: Self = Self {
        word_boundary: WordStop::Current,
        line_boundary: LineStop::None,
    };
}

impl Default for CaretStopOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What Backspace does inside leading whitespace (IntelliJ `smartBackspace`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartBackspace {
    /// Always one character.
    Off,
    /// Back to the previous multiple of the indent size.
    Indent,
    /// Re-indent through the formatter. The kit has none, so this deletes one character
    /// like [`SmartBackspace::Off`] (spec 3.6): the IntelliJ default.
    AutoIndent,
}

/// What paste does to the indentation of multi-line text (IntelliJ `REFORMAT_ON_PASTE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReformatOnPaste {
    /// Paste verbatim.
    None,
    /// Shift lines 2..N by the caret column.
    IndentBlock,
    /// Re-indent each line; without a formatter this is the block fallback: the default.
    IndentEachLine,
}

/// Editor settings with the IntelliJ defaults (spec section 0).
///
/// Everything here is data owned by the caller; nothing is global.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EditorSettings {
    /// Tab width in columns (4).
    pub tab_size: usize,
    /// Indent unit in columns (4).
    pub indent_size: usize,
    /// Indent with `\t` instead of spaces (false).
    pub use_tab_character: bool,
    /// With `use_tab_character`: a tab only when everything before the caret is tabs (false).
    pub smart_tabs: bool,
    /// Smart Home (true).
    pub smart_home: bool,
    /// Smart End: the first press stops before trailing whitespace (true).
    pub smart_end: bool,
    /// Caret past the line end (false). Column mode behaves as if it were on.
    pub virtual_space: bool,
    /// Caret inside tab characters (false).
    pub caret_inside_tabs: bool,
    /// CamelHumps words for Ctrl+Arrow, Ctrl+Backspace, Toggle Case (false).
    pub camel_humps: bool,
    /// Double click honors CamelHumps when `camel_humps` is on (true).
    pub mouse_selection_honors_camel_humps: bool,
    /// Auto-insert closing brackets (true).
    pub auto_pair_brackets: bool,
    /// Auto-insert closing quotes (true).
    pub auto_pair_quotes: bool,
    /// Typing a bracket or quote over a selection wraps it (true).
    pub surround_selection_on_quote_typed: bool,
    /// Tab jumps over an auto-inserted closer (true).
    pub tab_exits_brackets_and_quotes: bool,
    /// Enter copies/derives the indent (true).
    pub smart_indent_on_enter: bool,
    /// Backspace in leading whitespace ([`SmartBackspace::AutoIndent`]).
    pub smart_backspace: SmartBackspace,
    /// Paste indentation ([`ReformatOnPaste::IndentEachLine`]).
    pub reformat_on_paste: ReformatOnPaste,
    /// Copy/Cut without a selection acts on the line (true).
    pub copy_cut_empty_selection_line: bool,
    /// A line copied without selection pastes above the caret line (true).
    pub paste_line_above: bool,
    /// After Copy of an empty selection, put the caret state back instead of leaving the
    /// line selected (false; advanced setting `editor.skip.selecting.line.after.copy.empty.selection`).
    pub skip_selecting_line_after_copy: bool,
    /// Comment Line moves the caret down when there is no selection (true).
    pub comment_line_caret_down: bool,
    /// The line comment prefix goes to column 0 (true); otherwise at the common indent.
    pub line_comment_at_first_column: bool,
    /// Scroll-to-caret margin in lines (1).
    pub vertical_scroll_offset: usize,
    /// Extra lines scrolled past the margin (0).
    pub vertical_scroll_jump: usize,
    /// Scroll-to-caret margin in columns (3).
    pub horizontal_scroll_offset: usize,
    /// Extra columns scrolled past the margin (0).
    pub horizontal_scroll_jump: usize,
    /// Empty lines below the last line that can be scrolled to (5).
    pub scroll_past_end_lines: usize,
    /// Extra columns right of the longest line (3).
    pub additional_columns: usize,
    /// Scroll past end by a whole page (false; overrides `scroll_past_end_lines`).
    pub additional_page_at_bottom: bool,
    /// Where word motions stop ([`CaretStopOptions::DEFAULT`]).
    pub caret_stop: CaretStopOptions,
    /// Maximum number of carets (1000).
    pub max_carets: usize,
    /// Undo steps kept (100).
    pub undo_limit: usize,
    /// Right margin column (120); informative for the painter.
    pub right_margin: usize,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            tab_size: 4,
            indent_size: 4,
            use_tab_character: false,
            smart_tabs: false,
            smart_home: true,
            smart_end: true,
            virtual_space: false,
            caret_inside_tabs: false,
            camel_humps: false,
            mouse_selection_honors_camel_humps: true,
            auto_pair_brackets: true,
            auto_pair_quotes: true,
            surround_selection_on_quote_typed: true,
            tab_exits_brackets_and_quotes: true,
            smart_indent_on_enter: true,
            smart_backspace: SmartBackspace::AutoIndent,
            reformat_on_paste: ReformatOnPaste::IndentEachLine,
            copy_cut_empty_selection_line: true,
            paste_line_above: true,
            skip_selecting_line_after_copy: false,
            comment_line_caret_down: true,
            line_comment_at_first_column: true,
            vertical_scroll_offset: 1,
            vertical_scroll_jump: 0,
            horizontal_scroll_offset: 3,
            horizontal_scroll_jump: 0,
            scroll_past_end_lines: 5,
            additional_columns: 3,
            additional_page_at_bottom: false,
            caret_stop: CaretStopOptions::DEFAULT,
            max_carets: 1000,
            undo_limit: 100,
            right_margin: 120,
        }
    }
}

impl EditorSettings {
    /// The indent unit as text: spaces, or a tab with `use_tab_character`.
    pub fn indent_unit(&self) -> String {
        if self.use_tab_character {
            "\t".to_owned()
        } else {
            " ".repeat(self.indent_size.max(1))
        }
    }
}

/// The language services the editor can use (the slim counterpart of IntelliJ's `Commenter`,
/// `BraceMatcher` and `QuoteHandler`). Plain text has none: no auto-pairing of brackets or
/// quotes, no comment actions.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Language {
    /// Line comment prefix, such as `//` or `#`.
    pub line_comment: Option<String>,
    /// Add and remove one space after the line comment prefix (false).
    pub line_comment_space: bool,
    /// Block comment delimiters, such as `/*` and `*/`.
    pub block_comment: Option<(String, String)>,
    /// Bracket pairs that auto-close, wrap a selection and match.
    pub brackets: Vec<(char, char)>,
    /// Quote characters that auto-close and overtype.
    pub quotes: Vec<char>,
    /// Enter after an unmatched `{` inserts the closing `}` (IntelliJ "insert pair curly
    /// brace on Enter").
    pub close_curly_on_enter: bool,
}

impl Language {
    /// Plain text: nothing.
    pub fn plain_text() -> Self {
        Self {
            line_comment: None,
            line_comment_space: false,
            block_comment: None,
            brackets: Vec::new(),
            quotes: Vec::new(),
            close_curly_on_enter: false,
        }
    }

    /// A C-like language: `//` and `/* */` comments, `()[]{}`, quotes `"` `'` `` ` ``.
    pub fn c_like() -> Self {
        Self {
            line_comment: Some("//".to_owned()),
            line_comment_space: false,
            block_comment: Some(("/*".to_owned(), "*/".to_owned())),
            brackets: vec![('(', ')'), ('[', ']'), ('{', '}')],
            quotes: vec!['"', '\'', '`'],
            close_curly_on_enter: true,
        }
    }

    /// Same as [`Language::c_like`] with another line comment prefix and no block comment.
    pub fn with_line_comment(mut self, prefix: &str) -> Self {
        self.line_comment = Some(prefix.to_owned());
        self
    }

    /// The closer paired with `opener`.
    pub fn closer_of(&self, opener: char) -> Option<char> {
        self.brackets
            .iter()
            .find(|(open, _)| *open == opener)
            .map(|(_, close)| *close)
    }

    /// The opener paired with `closer`.
    pub fn opener_of(&self, closer: char) -> Option<char> {
        self.brackets
            .iter()
            .find(|(_, close)| *close == closer)
            .map(|(open, _)| *open)
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::plain_text()
    }
}
