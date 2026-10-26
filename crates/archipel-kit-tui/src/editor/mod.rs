//! [`EditorComponent`]: the terminal code editor, on the same model as the GUI editor.
//!
//! It looks like NvChad (a number column in grey with the current line bright, a sign column
//! for version control and breakpoint marks, the cursor line on `black2`, indent guides, a
//! Telescope-style find prompt, an nvim-cmp completion menu) and behaves like the IntelliJ
//! editor, because it **is** the GUI editor's logic: every key goes through
//! [`editor_action_for`](archipel_kit::editor::model::editor_action_for) and
//! [`Editor::execute`], typed text becomes [`EditorAction::Type`], the clipboard and the
//! mouse become the model's caret operations, and find and replace run on the model's
//! [`search`](archipel_kit::editor::model) functions. Undo, multiple carets, column
//! selection, folding and scrolling are the model's.
//!
//! The component is stateless like every kit component: it borrows an [`Editor`] you own and
//! an [`EditorState`] for what only the view knows (scroll position, the find bar, a drag in
//! progress, the completion menu). Build it once to draw ([`EditorComponent::show`]) and once
//! per event ([`EditorComponent::handle`]); both take the same options.
//!
//! ```
//! use archipel_kit_tui::editor::{EditorComponent, EditorState};
//! use archipel_kit_tui::{Id, Look, Runtime};
//! use archipel_kit_tui::ratatui::{buffer::Buffer, layout::Rect};
//! use archipel_kit::editor::model::Editor;
//!
//! let mut editor = Editor::new("fn main() {\n    println!(\"hi\");\n}\n");
//! let mut state = EditorState::new();
//! let mut rt = Runtime::new(Look::dark());
//! let area = Rect::new(0, 0, 40, 8);
//! let mut buf = Buffer::empty(area);
//! rt.begin_frame(area);
//! EditorComponent::new(Id::new("code"), &mut editor).show(&mut rt, &mut state, area, &mut buf);
//! ```

mod completion;
mod find;
mod highlight;
mod keys;
mod layout;
mod mouse;
mod paint;

pub use completion::default_kind;
pub use highlight::{CodeHighlighter, CodeLanguage};
pub use keys::{EditorChord, chords, label_of};

use self::completion::Menu;
use self::find::FindState;
use self::layout::{Ctx, RowMap};
use self::mouse::{Drag, Unit};
use crate::hit::Id;
use crate::input::{Event, KeyModifiers, MouseButton};
use crate::runtime::{CursorShape, Runtime};
use archipel_kit::action_system::Os as ActionOs;
use archipel_kit::editor::model::{
    ActionContext, ClipboardContent, ContentSize, Editor, EditorAction, Language, Os,
    ScrollRequest, ScrollType, SearchHistory, Viewport, clamp_first_line, scroll_to_caret,
};
use archipel_kit::editor::{
    ChangeMarker, GutterArea, GutterIcon, NavigationRequest, PlainText, RangeHighlight, Span,
    SyntaxHighlighter,
};
use archipel_kit::ui::Completer;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::collections::HashMap;
use std::rc::Rc;

/// How long a hint ("Not found, press F3 to search from the top") stays, in seconds.
const HINT_SECONDS: f64 = 4.0;

/// A click in the gutter that the component did not use itself (a click on a fold marker
/// folds, and a drag over the numbers selects lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GutterClick {
    /// The logical line (0-based).
    pub line: usize,
    /// Where in the gutter.
    pub area: GutterArea,
    /// Which button.
    pub button: MouseButton,
    /// The modifiers held.
    pub modifiers: KeyModifiers,
}

/// What the editor did with an event or in a frame.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct EditorOutput {
    /// The event was used: do not route it elsewhere.
    pub consumed: bool,
    /// The text changed (typing, paste, undo, a completion, replace).
    pub text_changed: bool,
    /// A caret or the selection moved, or the text changed.
    pub carets_moved: bool,
    /// A Ctrl (Cmd on macOS) click: the application resolves it.
    pub navigation: Option<NavigationRequest>,
    /// Clicks in the gutter the component did not use itself.
    pub gutter_clicks: Vec<GutterClick>,
    /// A hint for the user ("Not found, try again to search from top").
    pub hint: Option<String>,
    /// A right press on the text at this cell (the caret was moved there unless it was in
    /// the selection): show the context menu.
    pub context_menu: Option<(u16, u16)>,
}

/// What the view remembers between frames: scroll position, the find bar, a mouse gesture in
/// progress, the completion menu. One per editor.
pub struct EditorState {
    /// The recent queries and replacements of the find bar. Share one between editors by
    /// swapping it in and out, or save it with `SearchHistory::save`.
    pub history: SearchHistory,
    pub(crate) first_row: usize,
    pub(crate) first_col: usize,
    /// The pane of the last frame (the area the component was shown in).
    pub(crate) area: Rect,
    pub(crate) find: FindState,
    pub(crate) hint: Option<(String, f64)>,
    pub(crate) drag: Option<Drag>,
    pub(crate) unit: Option<Unit>,
    pub(crate) autoscroll_counter: u32,
    pub(crate) autoscroll_last: f64,
    pub(crate) autoscroll_at: Option<(u16, u16)>,
    pub(crate) menu: Option<Menu>,
    pub(crate) pending_scroll: Option<ScrollRequest>,
    pub(crate) center_line: Option<usize>,
    /// The text last selected with the mouse: the X11 primary selection of Linux.
    pub(crate) primary: String,
    pub(crate) caches: Caches,
    /// Whether the last frame had a stripe, find rows, … (the geometry of the last frame).
    pub(crate) geo: Option<Geo>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorState")
            .field("first_row", &self.first_row)
            .field("first_col", &self.first_col)
            .field("find_open", &self.find.open)
            .finish_non_exhaustive()
    }
}

/// Derived data that is rebuilt when the text, the folds or the width change.
#[derive(Default)]
pub(crate) struct Caches {
    rows: Option<(RowKey, Rc<RowMap>)>,
    max_cols: Option<(u64, usize, usize)>,
    pub spans: SpanCache,
}

type RowKey = (u64, u64, Option<usize>, usize);

/// Syntax spans per line and the tokenizer state at the start of each line.
#[derive(Default)]
pub(crate) struct SpanCache {
    key: Option<(u64, &'static str)>,
    /// `states[i]` is the state at the start of line `i`.
    states: Vec<u32>,
    lines: HashMap<usize, Vec<Span>>,
}

impl SpanCache {
    /// The spans of `line` (byte offsets relative to the line start).
    pub fn spans(
        &mut self,
        editor: &Editor,
        highlighter: &dyn SyntaxHighlighter,
        line: usize,
    ) -> &[Span] {
        let doc = &editor.document;
        let key = (doc.modification_stamp(), highlighter.name());
        if self.key != Some(key) {
            self.key = Some(key);
            self.states.clear();
            self.states.push(0);
            self.lines.clear();
        }
        if self.lines.len() > 4000 {
            self.lines.clear();
        }
        if !self.lines.contains_key(&line) {
            let mut state = 0;
            if !highlighter.stateless() {
                let mut scratch = Vec::new();
                while self.states.len() <= line {
                    let at = self.states.len() - 1;
                    let mut s = self.states[at];
                    scratch.clear();
                    highlighter.highlight_line(doc.line_text(at), &mut s, &mut scratch);
                    self.states.push(s);
                }
                state = self.states[line];
            }
            let mut spans = Vec::new();
            highlighter.highlight_line(doc.line_text(line), &mut state, &mut spans);
            self.lines.insert(line, spans);
        }
        &self.lines[&line]
    }
}

impl EditorState {
    /// A state scrolled to the top, with no find bar.
    pub fn new() -> Self {
        Self {
            history: SearchHistory::new(),
            first_row: 0,
            first_col: 0,
            area: Rect::default(),
            find: FindState::default(),
            hint: None,
            drag: None,
            unit: None,
            autoscroll_counter: 0,
            autoscroll_last: 0.0,
            autoscroll_at: None,
            menu: None,
            pending_scroll: None,
            center_line: None,
            primary: String::new(),
            caches: Caches::default(),
            geo: None,
        }
    }

    /// The first visible row.
    pub fn first_row(&self) -> usize {
        self.first_row
    }

    /// The first visible cell column.
    pub fn first_column(&self) -> usize {
        self.first_col
    }

    /// Scrolls so that `line` (0-based) is centered on the next frame (a "go to line" action).
    pub fn scroll_to_line(&mut self, line: usize) {
        self.center_line = Some(line);
    }

    /// Whether the find bar is open.
    pub fn find_open(&self) -> bool {
        self.find.open
    }

    /// The hint to show now, if one is still fresh at `now` seconds.
    pub fn hint(&self, now: f64) -> Option<&str> {
        self.hint
            .as_ref()
            .filter(|(_, until)| now < *until)
            .map(|(text, _)| text.as_str())
    }

    /// Whether the completion menu is open.
    pub fn completion_open(&self) -> bool {
        self.menu.is_some()
    }

    /// The time (in [`Runtime::now`] seconds) the editor next needs a
    /// [`tick`](EditorComponent::tick): while the mouse drags beyond the edge of the text, the
    /// autoscroll runs on a timer.
    pub fn deadline(&self) -> Option<f64> {
        self.autoscroll_at.map(|_| {
            self.autoscroll_last + f64::from(archipel_kit::tokens::motion::EDITOR_AUTOSCROLL_TICK)
        })
    }

    fn rows(&mut self, editor: &Editor, wrap: Option<usize>) -> Rc<RowMap> {
        let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for region in &ctx.outer {
            std::hash::Hash::hash(&(region.start, region.end), &mut hasher);
        }
        let key: RowKey = (
            editor.document.modification_stamp(),
            std::hash::Hasher::finish(&hasher),
            wrap,
            ctx.tab,
        );
        if let Some((k, map)) = &self.caches.rows
            && *k == key
        {
            return map.clone();
        }
        let map = Rc::new(RowMap::new(&ctx, wrap));
        self.caches.rows = Some((key, map.clone()));
        map
    }

    fn max_cols(&mut self, editor: &Editor) -> usize {
        let key = (
            editor.document.modification_stamp(),
            editor.settings.tab_size,
        );
        if let Some((stamp, tab, cols)) = self.caches.max_cols
            && (stamp, tab) == key
        {
            return cols;
        }
        let cols = layout::max_cols(&editor.document, key.1);
        self.caches.max_cols = Some((key.0, key.1, cols));
        cols
    }

    fn set_hint(&mut self, text: String, now: f64) {
        self.hint = Some((text, now + HINT_SECONDS));
    }
}

/// The columns of the gutter on screen.
#[derive(Clone, Debug, Default)]
pub(crate) struct GutterCols {
    pub vcs: Option<u16>,
    pub lanes: Vec<u16>,
    /// First cell and width of the number column.
    pub numbers: Option<(u16, u16)>,
    pub fold: Option<u16>,
}

/// Where things are on screen in one frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct Geo {
    /// The find bar rows.
    pub bar: Rect,
    pub gutter: Rect,
    pub cols: GutterCols,
    /// The text cells.
    pub text: Rect,
    pub stripe: Option<Rect>,
    pub wrap_cols: Option<usize>,
}

impl Geo {
    pub fn rows(&self) -> usize {
        self.text.height as usize
    }

    pub fn width(&self) -> usize {
        self.text.width as usize
    }

    /// Which part of the gutter column `x` falls in.
    pub fn gutter_area(&self, x: u16) -> GutterArea {
        if let Some(lane) = self
            .cols
            .lanes
            .iter()
            .position(|&lx| x >= lx && x < lx.saturating_add(2))
        {
            return GutterArea::Icon(lane);
        }
        if self.cols.vcs == Some(x) {
            return GutterArea::ChangeMarker;
        }
        if let Some((nx, w)) = self.cols.numbers
            && x >= nx
            && x <= nx + w
        {
            return GutterArea::LineNumber;
        }
        GutterArea::Other
    }

    /// Whether column `x` is the fold marker column.
    pub fn in_fold_column(&self, x: u16) -> bool {
        self.cols.fold.is_some_and(|fx| x >= fx && x < fx + 2)
    }
}

/// The component: build it with [`new`](Self::new) and the options, then
/// [`show`](Self::show) it or [`handle`](Self::handle) an event.
#[must_use = "show it with `component.show(...)` or feed it events with `component.handle(...)`"]
pub struct EditorComponent<'a> {
    id: Id,
    editor: &'a mut Editor,
    highlighter: &'a dyn SyntaxHighlighter,
    language: Option<&'a Language>,
    viewer: bool,
    line_numbers: bool,
    wrap: bool,
    folding: bool,
    whitespace: bool,
    indent_guides: bool,
    right_margin: bool,
    gutter_click_selects_line: bool,
    virtual_text: bool,
    scrollbar: Option<bool>,
    icons: &'a [GutterIcon],
    markers: &'a [ChangeMarker],
    highlights: &'a [RangeHighlight],
    completer: Option<&'a dyn Completer>,
    placeholder: Option<&'a str>,
    os: Option<Os>,
}

/// The resolved options: what `show` and `handle` pass around.
pub(crate) struct Opts<'a> {
    pub id: Id,
    pub highlighter: &'a dyn SyntaxHighlighter,
    pub language: &'a Language,
    pub viewer: bool,
    pub line_numbers: bool,
    pub wrap: bool,
    pub folding: bool,
    pub whitespace: bool,
    pub indent_guides: bool,
    pub right_margin: bool,
    pub gutter_click_selects_line: bool,
    pub virtual_text: bool,
    pub scrollbar: Option<bool>,
    pub icons: &'a [GutterIcon],
    pub markers: &'a [ChangeMarker],
    pub highlights: &'a [RangeHighlight],
    pub completer: Option<&'a dyn Completer>,
    pub placeholder: Option<&'a str>,
    pub os: Os,
}

impl Opts<'_> {
    pub fn gutter_id(&self) -> Id {
        self.id.with(1)
    }
    pub fn stripe_id(&self) -> Id {
        self.id.with(2)
    }
    pub fn bar_id(&self) -> Id {
        self.id.with(3)
    }
    pub fn menu_id(&self) -> Id {
        self.id.with(4)
    }
}

fn model_os(os: ActionOs) -> Os {
    if os.is_mac() {
        Os::MacOs
    } else {
        Os::WindowsLinux
    }
}

impl<'a> EditorComponent<'a> {
    /// A component on `editor`, identified by `id` (its focus and hit rectangles).
    pub fn new(id: Id, editor: &'a mut Editor) -> Self {
        Self {
            id,
            editor,
            highlighter: &PlainText,
            language: None,
            viewer: false,
            line_numbers: true,
            wrap: false,
            folding: false,
            whitespace: false,
            indent_guides: true,
            right_margin: true,
            gutter_click_selects_line: false,
            virtual_text: false,
            scrollbar: None,
            icons: &[],
            markers: &[],
            highlights: &[],
            completer: None,
            placeholder: None,
            os: None,
        }
    }

    /// The [`SyntaxHighlighter`] that colors the text (default: none). Any highlighter of the
    /// GUI kit works; [`CodeHighlighter`] is a small one for Rust, Markdown and TOML.
    pub fn highlighter(mut self, highlighter: &'a dyn SyntaxHighlighter) -> Self {
        self.highlighter = highlighter;
        self
    }

    /// The language services: brackets, quotes and comments (default
    /// [`Language::c_like`]; pass [`Language::plain_text`] for prose).
    pub fn language(mut self, language: &'a Language) -> Self {
        self.language = Some(language);
        self
    }

    /// Read-only (IntelliJ `isViewer`): carets, selection, copy, find and folding work,
    /// editing does not.
    pub fn viewer(mut self, viewer: bool) -> Self {
        self.viewer = viewer;
        self
    }

    /// Shows line numbers (default on).
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Wraps long lines at the width of the text instead of scrolling sideways. Caret
    /// movement by Home, End, Up and Down follows logical lines, not wrapped rows.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Shows fold markers in the gutter for the regions of the model's `FoldingModel`.
    pub fn folding(mut self, on: bool) -> Self {
        self.folding = on;
        self
    }

    /// Shows spaces as `·` and tabs as `→` (default off).
    pub fn show_whitespace(mut self, on: bool) -> Self {
        self.whitespace = on;
        self
    }

    /// Indent guides, thin `│` at the indent levels (default on).
    pub fn indent_guides(mut self, on: bool) -> Self {
        self.indent_guides = on;
        self
    }

    /// Tints the column of the model's `right_margin` setting, 120 by default (default on).
    pub fn right_margin(mut self, on: bool) -> Self {
        self.right_margin = on;
        self
    }

    /// Clicking a line number selects the line, as the classic IDE gutter does. By default
    /// (the new UI, where the number column belongs to breakpoints) a click only reports a
    /// [`GutterClick`] and a drag selects lines.
    pub fn gutter_click_selects_line(mut self, on: bool) -> Self {
        self.gutter_click_selects_line = on;
        self
    }

    /// Icons in the sign column (breakpoints, bookmarks, run arrows); clicks are reported in
    /// [`EditorOutput::gutter_clicks`].
    pub fn gutter_icons(mut self, icons: &'a [GutterIcon]) -> Self {
        self.icons = icons;
        self
    }

    /// Version control change markers (`▎`) in the sign column.
    pub fn change_markers(mut self, markers: &'a [ChangeMarker]) -> Self {
        self.markers = markers;
        self
    }

    /// Ranges to mark: diagnostics as undercurl in the text and marks in the scrollbar
    /// column.
    pub fn highlights(mut self, highlights: &'a [RangeHighlight]) -> Self {
        self.highlights = highlights;
        self
    }

    /// Shows the message of the first error or warning of a line after the line, as Neovim's
    /// diagnostic virtual text does (default off).
    pub fn virtual_text(mut self, on: bool) -> Self {
        self.virtual_text = on;
        self
    }

    /// Forces the scrollbar column (thumb and marks) on or off. By default it shows when
    /// there are highlights or find matches, like the GUI's error stripe.
    pub fn scrollbar(mut self, on: bool) -> Self {
        self.scrollbar = Some(on);
        self
    }

    /// Offers completions while typing, in an nvim-cmp style menu under the caret (also on
    /// Ctrl+Space).
    pub fn completer(mut self, completer: &'a dyn Completer) -> Self {
        self.completer = Some(completer);
        self
    }

    /// Text shown while the document is empty.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// The platform whose editor keymap applies (default: the runtime's).
    pub fn keymap_os(mut self, os: Os) -> Self {
        self.os = Some(os);
        self
    }

    /// The options and the model, for the submodules.
    fn into_parts<'b>(self, rt_os: ActionOs, owned: &'b Language) -> (Opts<'b>, &'b mut Editor)
    where
        'a: 'b,
    {
        let opts = Opts {
            id: self.id,
            highlighter: self.highlighter,
            language: self.language.unwrap_or(owned),
            viewer: self.viewer,
            line_numbers: self.line_numbers,
            wrap: self.wrap,
            folding: self.folding,
            whitespace: self.whitespace,
            indent_guides: self.indent_guides,
            right_margin: self.right_margin,
            gutter_click_selects_line: self.gutter_click_selects_line,
            virtual_text: self.virtual_text,
            scrollbar: self.scrollbar,
            icons: self.icons,
            markers: self.markers,
            highlights: self.highlights,
            completer: self.completer,
            placeholder: self.placeholder,
            os: self.os.unwrap_or_else(|| model_os(rt_os)),
        };
        (opts, self.editor)
    }

    /// Draws the editor in `area`: the find bar on top when it is open, the gutter, the text
    /// and the scrollbar column. Registers its rectangles in the hit map and asks the runtime
    /// for the terminal cursor at the primary caret while it has the focus.
    pub fn show(
        self,
        rt: &mut Runtime,
        state: &mut EditorState,
        area: Rect,
        buf: &mut Buffer,
    ) -> EditorOutput {
        let owned = Language::c_like();
        let os = rt.caps.os;
        let (opts, editor) = self.into_parts(os, &owned);
        paint::show(&opts, editor, rt, state, area, buf)
    }

    /// Handles `event`: keys, text, paste and the mouse, as the GUI editor does. Call it for
    /// every event after [`Runtime::preprocess`]; keys only count while the editor has the
    /// focus (`rt.focused(id)`).
    pub fn handle(self, rt: &mut Runtime, state: &mut EditorState, event: &Event) -> EditorOutput {
        let owned = Language::c_like();
        let os = rt.caps.os;
        let (opts, editor) = self.into_parts(os, &owned);
        let mut out = EditorOutput::default();
        let stamp = editor.document.modification_stamp();
        let carets = editor.carets.snapshot();
        match event {
            Event::Key(key) => keys::key(&opts, editor, rt, state, key, &mut out),
            Event::Paste(text) => keys::paste(&opts, editor, rt, state, text, &mut out),
            Event::Mouse(_) => mouse::mouse(&opts, editor, rt, state, &mut out),
            Event::FocusLost => {
                state.drag = None;
                state.autoscroll_at = None;
            }
            _ => {}
        }
        out.text_changed |= editor.document.modification_stamp() != stamp;
        out.carets_moved |= out.text_changed || editor.carets.snapshot() != carets;
        out
    }

    /// Runs the timers: the autoscroll while the mouse drags beyond the edge of the text.
    /// Call it from [`App::tick`](crate::App::tick) and use
    /// [`EditorState::deadline`] for the loop's wake-up.
    pub fn tick(self, rt: &mut Runtime, state: &mut EditorState) -> EditorOutput {
        let owned = Language::c_like();
        let os = rt.caps.os;
        let (opts, editor) = self.into_parts(os, &owned);
        let mut out = EditorOutput::default();
        let carets = editor.carets.snapshot();
        mouse::tick(&opts, editor, rt, state);
        out.carets_moved = editor.carets.snapshot() != carets;
        out
    }
}

// ---- shared helpers of the submodules ------------------------------------------------

/// The geometry of `area` for the current options, text and find bar.
pub(crate) fn geometry(opts: &Opts<'_>, editor: &Editor, state: &EditorState, area: Rect) -> Geo {
    let bar_rows = state
        .find
        .rows(opts.viewer)
        .min(area.height.saturating_sub(1));
    let bar = Rect::new(area.x, area.y, area.width, bar_rows);
    let body = Rect::new(
        area.x,
        area.y + bar_rows,
        area.width,
        area.height - bar_rows,
    );
    let stripe_on = opts.scrollbar.unwrap_or(
        !opts.highlights.is_empty() || (state.find.open && !state.find.query.is_empty()),
    ) && body.width > 12;
    let lane_count = opts.icons.iter().map(|i| i.lane + 1).max().unwrap_or(0);
    let has_sign = opts.line_numbers || lane_count > 0 || !opts.markers.is_empty();
    let lanes = if has_sign { lane_count.max(1) } else { 0 };
    let digits = if opts.line_numbers {
        editor.document.line_count().to_string().len().max(2) as u16
    } else {
        0
    };
    let mut x = body.x;
    let mut cols = GutterCols::default();
    if has_sign {
        cols.vcs = Some(x);
        x += 1;
        for _ in 0..lanes {
            cols.lanes.push(x);
            x += 2;
        }
    }
    if opts.line_numbers {
        cols.numbers = Some((x, digits));
        x += digits + 1;
    }
    if opts.folding {
        cols.fold = Some(x);
        x += 2;
    }
    let gutter_w = (x - body.x).min(body.width.saturating_sub(4));
    let pad = u16::from(gutter_w == 0);
    let text_x = body.x + gutter_w + pad;
    let stripe_w = u16::from(stripe_on);
    let text_w = body.width.saturating_sub(gutter_w + pad + stripe_w);
    let text = Rect::new(text_x, body.y, text_w, body.height);
    let stripe = stripe_on.then(|| Rect::new(body.right() - 1, body.y, 1, body.height));
    Geo {
        bar,
        gutter: Rect::new(body.x, body.y, gutter_w, body.height),
        cols,
        text,
        stripe,
        wrap_cols: opts.wrap.then(|| (text_w as usize).max(1)),
    }
}

/// The row and cell column of the caret at `offset` (plus virtual spaces).
pub(crate) fn row_col(
    ctx: &Ctx<'_>,
    map: &RowMap,
    offset: usize,
    virtual_spaces: usize,
) -> (usize, usize) {
    let v = ctx.visual_of(offset);
    let layout = ctx.layout(v);
    let col = layout.col_of(ctx.doc, ctx.tab, offset) + virtual_spaces;
    match map.wrap() {
        Some(wc) => (map.first_row(v) + col / wc, col % wc),
        None => (map.first_row(v), col),
    }
}

/// Moves the viewport as `request` says, for the geometry of the last frame (or `geo`).
pub(crate) fn apply_scroll(
    editor: &Editor,
    state: &mut EditorState,
    geo: &Geo,
    request: ScrollRequest,
) {
    if geo.rows() == 0 || geo.width() == 0 {
        state.pending_scroll = Some(request);
        return;
    }
    let map = state.rows(editor, geo.wrap_cols);
    let content = ContentSize {
        lines: map.total_rows(),
        columns: state.max_cols(editor),
    };
    let viewport = Viewport {
        first_line: state.first_row as f64,
        visible_lines: geo.rows() as f64,
        first_column: state.first_col as f64,
        visible_columns: geo.width() as f64,
    };
    match request {
        ScrollRequest::ToCaret {
            scroll_type,
            animate,
        } => {
            let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
            let caret = editor.carets.primary();
            let (row, col) = row_col(&ctx, &map, caret.offset, caret.virtual_spaces);
            let target = scroll_to_caret(
                &viewport,
                content,
                row as f64,
                col as f64,
                scroll_type,
                animate,
                &editor.settings,
            );
            state.first_row = target.first_line.round().max(0.0) as usize;
            state.first_col = if geo.wrap_cols.is_some() {
                0
            } else {
                target.first_column.round().max(0.0) as usize
            };
        }
        ScrollRequest::Lines(n) => {
            let first = clamp_first_line(
                state.first_row as f64 + n as f64,
                content,
                viewport.visible_lines,
                &editor.settings,
            );
            state.first_row = first.round().max(0.0) as usize;
        }
    }
}

/// Keeps the scroll position inside the content (after the text shrank or the pane resized).
pub(crate) fn clamp_scroll(editor: &Editor, state: &mut EditorState, geo: &Geo) {
    if geo.rows() == 0 {
        return;
    }
    let map = state.rows(editor, geo.wrap_cols);
    let content = ContentSize {
        lines: map.total_rows(),
        columns: state.max_cols(editor),
    };
    let first = clamp_first_line(
        state.first_row as f64,
        content,
        geo.rows() as f64,
        &editor.settings,
    );
    state.first_row = first.round().max(0.0) as usize;
    if geo.wrap_cols.is_some() {
        state.first_col = 0;
    } else {
        let max = archipel_kit::editor::model::max_first_column(
            content,
            geo.width() as f64,
            &editor.settings,
        );
        state.first_col = (state.first_col as f64).clamp(0.0, max).round() as usize;
    }
}

/// Runs `action` for all carets and handles what it asks of the view: scrolling, the
/// clipboard, a hint. Returns whether it did something.
pub(crate) fn run_action(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    state: &mut EditorState,
    action: &EditorAction,
    clipboard: Option<&ClipboardContent>,
    out: &mut EditorOutput,
) -> bool {
    let geo = geometry(opts, editor, state, state.area);
    let first_visible = {
        let map = state.rows(editor, geo.wrap_cols);
        let (v, _) = map.visual_at_row(state.first_row);
        editor.folding.visual_to_logical_line(&editor.document, v)
    };
    let mut ctx = ActionContext::new(opts.language).with_viewport(first_visible, geo.rows().max(1));
    if let Some(clip) = clipboard {
        ctx = ctx.with_clipboard(clip);
    }
    let outcome = editor.execute(action, &ctx);
    if !outcome.handled {
        return false;
    }
    out.consumed = true;
    if let Some(content) = &outcome.clipboard {
        rt.clipboard
            .set_content(content.text.clone(), content.whole_line);
    }
    if let Some(hint) = outcome.hint {
        state.set_hint(hint.clone(), rt.now);
        out.hint = Some(hint);
    }
    if let Some(request) = outcome.scroll {
        apply_scroll(editor, state, &geo, request);
    }
    true
}

/// Brings `offset` into view, unfolding what hides it (a found match, a jump).
pub(crate) fn reveal(editor: &mut Editor, state: &mut EditorState, offset: usize) {
    while let Some(region) = editor.folding.collapsed_region_hiding(offset) {
        let start = region.start;
        match editor
            .folding
            .regions()
            .iter()
            .position(|r| r.start == start && !r.expanded)
        {
            Some(index) => editor.folding.set_expanded(index, true),
            None => break,
        }
    }
    let geo = state.geo.clone().unwrap_or_default();
    if geo.rows() == 0 {
        return;
    }
    let map = state.rows(editor, geo.wrap_cols);
    let ctx = Ctx::new(&editor.document, &editor.folding, editor.settings.tab_size);
    let (row, col) = row_col(&ctx, &map, offset, 0);
    let content = ContentSize {
        lines: map.total_rows(),
        columns: state.max_cols(editor),
    };
    let viewport = Viewport {
        first_line: state.first_row as f64,
        visible_lines: geo.rows() as f64,
        first_column: state.first_col as f64,
        visible_columns: geo.width() as f64,
    };
    let target = scroll_to_caret(
        &viewport,
        content,
        row as f64,
        col as f64,
        ScrollType::MakeVisible,
        false,
        &editor.settings,
    );
    state.first_row = target.first_line.round().max(0.0) as usize;
    if geo.wrap_cols.is_none() {
        state.first_col = target.first_column.round().max(0.0) as usize;
    }
}

/// The primary caret's position as the status line shows it: line and column, 1-based (the
/// column counts like IntelliJ's: tabs advance to the next tab stop, virtual spaces count).
pub fn cursor_position(editor: &Editor) -> (usize, usize) {
    let caret = editor.carets.primary();
    let position = archipel_kit::editor::model::offset_to_logical(
        &editor.document,
        caret.offset,
        editor.settings.tab_size,
    );
    (
        position.line + 1,
        position.column + caret.virtual_spaces + 1,
    )
}

/// The shape the terminal cursor takes: a bar while inserting, a block while overwriting.
pub(crate) fn cursor_shape(editor: &Editor) -> CursorShape {
    if editor.carets.overwrite {
        CursorShape::Block
    } else {
        CursorShape::Bar
    }
}

/// Scrolls the minimum that shows the primary caret (after the application changed it).
pub(crate) fn run_scroll_to_caret(editor: &Editor, state: &mut EditorState) {
    let geo = state.geo.clone().unwrap_or_default();
    apply_scroll(
        editor,
        state,
        &geo,
        ScrollRequest::ToCaret {
            scroll_type: ScrollType::Relative,
            animate: false,
        },
    );
}
