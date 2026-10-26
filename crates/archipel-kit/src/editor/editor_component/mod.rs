//! [`EditorComponent`]: the editor widget on a [`model::Editor`](crate::editor::model::Editor).

mod cache;
mod gutter;
mod input;
mod layout;
mod mouse;
mod paint;
mod rows;
mod state;

pub(super) use self::cache::MAX_MATCHES;
use self::cache::{DocCache, fold_regions};
use self::gutter::{GutterCfg, GutterInput, GutterLayout};
use self::state::{Frame, Geo, ScrollAnim, ViewState};
use super::decorations::{
    ChangeMarker, GutterClick, GutterIcon, HoverInfo, NavigationRequest, RangeHighlight,
};
use super::find::{self, FindState};
use super::highlight::{Highlighter, PlainText};
use super::model::{
    ContentSize, Editor, EditorAction, Language, Os, ScrollRequest, ScrollType, Viewport,
    clamp_first_line, scroll_to_caret,
};
use super::{A11Y_VALUE_LIMIT, code_font, paint_background, paint_border};
use crate::parts::desktop;
use crate::theme::Theme;
use crate::tokens::{editor as metrics, motion, size};
use egui::{
    CursorIcon, EventFilter, FontId, Id, Rect, Response, Sense, Ui, UiBuilder, Vec2, Widget,
    WidgetInfo, WidgetType, accesskit, pos2, vec2,
};
use std::ops::Range;
use std::time::Duration;

/// The editor's Swing component (IntelliJ `EditorComponentImpl`): an egui widget that paints
/// and operates a [`model::Editor`](crate::editor::model::Editor), replacing a code view and a
/// code editor with one component.
///
/// The model owns the text, the carets, undo, folds and the settings; the component only draws
/// them and turns keys, text, the clipboard and the mouse into model actions. All of it follows
/// the IntelliJ editor: the Islands selection, a 2 px caret, the gutter with fold chevrons
/// that show on hover, scroll past the end, multiple carets, column selection.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::editor::{EditorComponent, Json};
/// use archipel_kit::editor::model::Editor;
///
/// let mut editor = Editor::new("{\n  \"name\": \"archipel\"\n}");
/// let out = EditorComponent::new(&mut editor)
///     .label("Request body")
///     .highlighter(&Json)
///     .line_numbers(true)
///     .height(160.0)
///     .show(ui);
/// if out.text_changed { /* the body was edited */ }
/// # });
/// ```
///
/// With [`viewer`](EditorComponent::viewer) the component is read-only (IntelliJ `isViewer`):
/// everything but editing works, so carets, selections, copy, find and folding do.
///
/// Settings of typing (tab size, auto-pairing, smart indent …) are the model's
/// [`EditorSettings`](crate::editor::model::EditorSettings); the language services (brackets,
/// quotes, comments) are the [`Language`] passed with [`language`](EditorComponent::language),
/// by default C-like.
#[must_use = "show it with `component.show(ui)` or add it with `ui.add(component)`"]
pub struct EditorComponent<'a> {
    editor: &'a mut Editor,
    label: Option<String>,
    placeholder: Option<String>,
    highlighter: &'a dyn Highlighter,
    language: Option<&'a Language>,
    line_numbers: bool,
    wrap: bool,
    folding: bool,
    bracket_folding: bool,
    viewer: bool,
    enabled: bool,
    tab_indents: bool,
    height: Option<f32>,
    id: Option<Id>,
    completer: Option<&'a dyn crate::ui::Completer>,
    scroll_to_line: Option<usize>,
    stick_to_bottom: bool,
    icons: &'a [GutterIcon],
    markers: &'a [ChangeMarker],
    highlights: &'a [RangeHighlight],
    whitespace: bool,
    indent_guides: bool,
    right_margin: bool,
    gutter_click_selects_line: bool,
    os: Option<Os>,
}

/// What [`EditorComponent::show`] reports.
#[non_exhaustive]
pub struct EditorComponentResponse {
    /// The component's response: focus, hover, and the accessible node. `changed()` after
    /// every edit.
    pub response: Response,
    /// The text changed this frame (typing, paste, undo, a completion).
    pub text_changed: bool,
    /// A caret or selection moved, or the text changed, this frame.
    pub carets_moved: bool,
    /// A Ctrl (Cmd on macOS) click: the application resolves it.
    pub navigation: Option<NavigationRequest>,
    /// Clicks in the gutter the component did not use itself.
    pub gutter_clicks: Vec<GutterClick>,
    /// The pointer has rested on the text for 500 ms: show a tooltip for what is there.
    pub hover: Option<HoverInfo>,
    /// The pointer has rested on a fold placeholder for 300 ms: the byte range of the folded
    /// text, for a preview.
    pub fold_preview: Option<Range<usize>>,
    /// The index (in the highlights passed in) of the error stripe mark under the pointer.
    pub stripe_hover: Option<usize>,
    /// A hint to show under the caret ("Not found, try again to search from top").
    pub hint: Option<String>,
    /// The primary selection as a byte range, `None` for a bare caret.
    pub selection: Option<Range<usize>>,
    /// The primary caret on screen, for anchoring popups.
    pub caret_rect: Option<Rect>,
    /// Number of lines in the document.
    pub line_count: usize,
    /// The logical lines that are at least partly visible.
    pub visible_lines: Range<usize>,
    /// How many rows were laid out this frame: the visible ones, never the whole text.
    pub laid_out_rows: usize,
    /// Matches of the find bar (0 while it is closed).
    pub matches: usize,
}

impl<'a> EditorComponent<'a> {
    /// A component on `editor`.
    pub fn new(editor: &'a mut Editor) -> Self {
        Self {
            editor,
            label: None,
            placeholder: None,
            highlighter: &PlainText,
            language: None,
            line_numbers: false,
            wrap: false,
            folding: false,
            bracket_folding: false,
            viewer: false,
            enabled: true,
            tab_indents: true,
            height: None,
            id: None,
            completer: None,
            scroll_to_line: None,
            stick_to_bottom: false,
            icons: &[],
            markers: &[],
            highlights: &[],
            whitespace: false,
            indent_guides: true,
            right_margin: true,
            gutter_click_selects_line: false,
            os: None,
        }
    }

    /// The accessible name ("Request body"). Texts up to 64 KB are also exposed as the value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Hint shown while the document is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// The [`Highlighter`] that colors the text (default: none).
    pub fn highlighter(mut self, highlighter: &'a dyn Highlighter) -> Self {
        self.highlighter = highlighter;
        self
    }

    /// The language services: brackets, quotes and comments (default
    /// [`Language::c_like`]; pass [`Language::plain_text`] for prose).
    pub fn language(mut self, language: &'a Language) -> Self {
        self.language = Some(language);
        self
    }

    /// Read-only (IntelliJ `isViewer`): carets, selection, multiple carets, copy, find and
    /// folding work, editing does not. <kbd>Tab</kbd> moves the focus.
    pub fn viewer(mut self, viewer: bool) -> Self {
        self.viewer = viewer;
        self
    }

    /// Shows line numbers in the gutter.
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Wraps long lines at the component's width instead of scrolling sideways. Caret
    /// movement by Home, End, Up and Down follows logical lines, not wrapped rows.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Shows fold chevrons in the gutter for the regions of the model's
    /// [`FoldingModel`](crate::editor::model::FoldingModel).
    pub fn folding(mut self, on: bool) -> Self {
        self.folding = on;
        self
    }

    /// Folds `{…}` and `[…]` pairs that span several lines (and shows the chevrons): the
    /// component keeps the model's fold regions in sync with the text, keeping what is
    /// collapsed. For JSON and JSON-like text.
    pub fn bracket_folding(mut self, on: bool) -> Self {
        self.bracket_folding = on;
        if on {
            self.folding = true;
        }
        self
    }

    /// Disabled components are dimmed, read-only and skipped by <kbd>Tab</kbd>.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// <kbd>Tab</kbd> indents (default on). Off, it moves focus to the next control, for
    /// forms.
    pub fn tab_indents(mut self, on: bool) -> Self {
        self.tab_indents = on;
        self
    }

    /// Height in points (default: the available height, or 300 when it is unbounded).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Gives the component an explicit id, for several in one place in the ui.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Offers completions while typing, in a list under the caret. See
    /// [`JBTextField::completer`](crate::ui::JBTextField::completer).
    pub fn completer(mut self, completer: &'a dyn crate::ui::Completer) -> Self {
        self.completer = Some(completer);
        self
    }

    /// Scrolls so that `line` (0-based) is centered, on the frame it is passed. Pass it once,
    /// for example from a "go to line" action.
    pub fn scroll_to_line(mut self, line: Option<usize>) -> Self {
        self.scroll_to_line = line;
        self
    }

    /// Keeps the view scrolled to the end while the text grows, as a console or log does;
    /// scrolling up leaves the end until the user scrolls back down. Turns scroll past the
    /// end off.
    pub fn stick_to_bottom(mut self, on: bool) -> Self {
        self.stick_to_bottom = on;
        self
    }

    /// Icons in the gutter's lanes (breakpoints, run arrows, bookmarks); clicks are reported
    /// in [`EditorComponentResponse::gutter_clicks`].
    pub fn gutter_icons(mut self, icons: &'a [GutterIcon]) -> Self {
        self.icons = icons;
        self
    }

    /// Version control change markers next to the line numbers.
    pub fn change_markers(mut self, markers: &'a [ChangeMarker]) -> Self {
        self.markers = markers;
        self
    }

    /// Ranges to mark: squiggles in the text and marks on the error stripe beside the scroll
    /// bar. Hover and clicks on the stripe are reported.
    pub fn highlights(mut self, highlights: &'a [RangeHighlight]) -> Self {
        self.highlights = highlights;
        self
    }

    /// Shows spaces as dots and tabs as lines (default off).
    pub fn show_whitespace(mut self, on: bool) -> Self {
        self.whitespace = on;
        self
    }

    /// Vertical lines at the indent levels (default on).
    pub fn indent_guides(mut self, on: bool) -> Self {
        self.indent_guides = on;
        self
    }

    /// The right margin line at the model's `right_margin` column, 120 by default (default
    /// shown).
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

    /// The platform whose keymap applies (default: the one the program runs on).
    pub fn keymap_os(mut self, os: Os) -> Self {
        self.os = Some(os);
        self
    }

    /// Draws the component.
    pub fn show(self, ui: &mut Ui) -> EditorComponentResponse {
        run(self, ui)
    }
}

impl Widget for EditorComponent<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

fn build_frame(
    ctx: &egui::Context,
    id: Id,
    editor: &Editor,
    highlighter: &dyn Highlighter,
    wrap_cols: Option<usize>,
) -> Frame {
    let tab = editor.settings.tab_size;
    let cache = DocCache::get(ctx, id, &editor.document, highlighter, tab);
    let outer: Vec<_> = editor
        .folding
        .outer_collapsed()
        .into_iter()
        .cloned()
        .collect();
    let widths = wrap_cols.map(|_| cache.widths(&editor.document, tab));
    let wrap = wrap_cols
        .zip(widths.as_ref())
        .map(|(c, w)| (c, w.as_slice()));
    let map = rows::RowMap::new(&editor.document, &outer, wrap);
    Frame { cache, outer, map }
}

/// The caret's row and its column within the row.
fn caret_row_col(editor: &Editor, frame: &Frame) -> (usize, usize) {
    let caret = editor.carets.primary();
    offset_row_col(editor, frame, caret.offset, caret.virtual_spaces)
}

fn offset_row_col(
    editor: &Editor,
    frame: &Frame,
    offset: usize,
    virtual_spaces: usize,
) -> (usize, usize) {
    let v = frame.visual_of(editor, offset);
    let layout = frame.layout(&editor.document, v, editor.settings.tab_size);
    let col = layout.col_of(offset) + virtual_spaces;
    match frame.map.wrap_cols() {
        Some(wc) => (frame.map.first_row(v) + col / wc, col % wc),
        None => (frame.map.first_row(v), col),
    }
}

fn run(spec: EditorComponent<'_>, ui: &mut Ui) -> EditorComponentResponse {
    let ctx = ui.ctx().clone();
    let EditorComponent {
        editor,
        label,
        placeholder,
        highlighter,
        language,
        line_numbers,
        wrap,
        folding,
        bracket_folding,
        viewer,
        enabled,
        tab_indents,
        height,
        id,
        completer,
        scroll_to_line,
        stick_to_bottom,
        icons: gutter_icons,
        markers,
        highlights,
        whitespace,
        indent_guides,
        right_margin,
        gutter_click_selects_line,
        os,
    } = spec;
    let id = id.unwrap_or_else(|| ui.next_auto_id());
    ui.skip_ahead_auto_ids(1);
    let theme = Theme::current(&ctx);
    let colors = theme.editor;
    let enabled = enabled && ui.is_enabled();
    let os = os.unwrap_or_else(Os::current);
    let mut st = ViewState::load(&ctx, id);
    let now = ui.input(|i| i.time);
    let ppp = ctx.pixels_per_point();
    let (font, cell) = code_font(ui);
    let number_font = FontId::new(font.size + metrics::NUMBER_FONT_DELTA, font.family.clone());
    let number_cell = ui.fonts_mut(|f| f.glyph_width(&number_font, '0'));
    let row_h = size::CODE_LINE_HEIGHT;
    let pad = size::CODE_PADDING;
    let default_language;
    let language: &Language = match language {
        Some(language) => language,
        None => {
            default_language = Language::c_like();
            &default_language
        }
    };

    // The frame: background, border, gutter and text area.
    let height = height.unwrap_or_else(|| {
        let h = ui.available_height();
        if h.is_finite() && (40.0..10_000.0).contains(&h) {
            h
        } else {
            300.0
        }
    });
    // A layout that has run out of room must not make egui allocate a negative size.
    let frame_size = vec2(ui.available_width(), height).max(Vec2::ZERO);
    let (outer, _) = ui.allocate_exact_size(frame_size, Sense::hover());
    paint_background(ui, outer);
    let inner = outer.shrink(1.0);
    let tab = editor.settings.tab_size;

    let stripe_w = if highlights.is_empty() {
        0.0
    } else {
        metrics::STRIPE_WIDTH
    };
    let lanes = gutter_icons.iter().map(|i| i.lane + 1).max().unwrap_or(0);
    let cfg = GutterCfg {
        numbers: line_numbers,
        lines: editor.document.line_count(),
        change_markers: !markers.is_empty(),
        lanes,
        folding,
    };
    let gutter_w = gutter::width(&cfg, number_cell);
    let gutter_rect = Rect::from_min_max(inner.min, pos2(inner.min.x + gutter_w, inner.max.y));
    let area = Rect::from_min_max(
        pos2(inner.min.x + gutter_w, inner.min.y),
        pos2(
            (inner.max.x - stripe_w).max(inner.min.x + gutter_w),
            inner.max.y,
        ),
    );
    let stripe_rect = Rect::from_min_max(pos2(area.max.x, inner.min.y), inner.max);
    let scroll_bar = ui.spacing().scroll.allocated_width();
    let wrap_cols =
        wrap.then(|| (((area.width() - 2.0 * pad - scroll_bar) / cell).floor() as usize).max(1));
    let visible_rows = (area.height() / row_h).floor().max(1.0) as usize;
    let visible_cols = ((area.width() - pad - scroll_bar) / cell).max(1.0);

    // Bracket folds follow the text.
    {
        let cache = DocCache::get(&ctx, id, &editor.document, highlighter, tab);
        let key = std::sync::Arc::as_ptr(&cache) as usize;
        if bracket_folding && st.folds_key != key {
            let regions = fold_regions(&cache.brackets(&editor.document), editor.folding.regions());
            editor.folding.replace_regions(regions);
        }
        st.folds_key = key;
    }

    // Interaction surfaces.
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let resp_text = ui.interact(area, id, sense);
    let resp_gutter = (gutter_w > 0.0).then(|| {
        ui.interact(
            gutter_rect,
            id.with("gutter"),
            if enabled {
                Sense::CLICK | Sense::DRAG
            } else {
                Sense::hover()
            },
        )
    });
    let focus_before = ctx.memory(|m| m.has_focus(id));
    let focused = enabled && focus_before;

    // Find and the completion list take their keys first.
    let mut find = FindState::load(&ctx, id);
    // A press on the find bar's buttons must not take the focus out of the bar or the text:
    // egui drops it a frame or two later, so the previous holder asks for it back.
    if let Some(rect) = st.overlay
        && ui.input(|i| {
            i.events.iter().any(|e| {
                matches!(e, egui::Event::PointerButton { pos, pressed: true, .. } if rect.contains(*pos))
            })
        })
    {
        st.restore_focus = 3;
    }
    if st.restore_focus > 0 {
        st.restore_focus -= 1;
        if ctx.memory(|m| m.focused().is_none()) {
            // The field is gone when the bar was closed: the text takes the focus.
            let target = match st.last_focus {
                Some(field) if field != id && find.open => field,
                _ => id,
            };
            ctx.memory_mut(|m| m.request_focus(target));
            st.restore_focus = 0;
        }
    }
    let closed_by_escape = find.keys(ui, focused);
    if closed_by_escape {
        ctx.memory_mut(|m| m.request_focus(id));
    }
    let (list, _list_keys) =
        crate::ui::completion::List::begin(ui, id, focused && completer.is_some() && !viewer);
    let list_open = list.is_open();

    let mut frame = build_frame(&ctx, id, editor, highlighter, wrap_cols);
    let stamp_before = editor.document.modification_stamp();
    let carets_before = editor.carets.snapshot();
    let outer_before = frame.outer.clone();
    let geo = Geo::new(area, st.offset, cell, row_h, pad, ppp);

    // Mouse.
    let mut navigation = None;
    let mut gutter_clicks: Vec<GutterClick> = Vec::new();
    let mut scroll_request: Option<ScrollRequest> = std::mem::take(&mut st.pending_center)
        .then_some(ScrollRequest::ToCaret {
            scroll_type: ScrollType::Center,
            animate: true,
        });
    let mut hint = None;
    let mut text_changed = false;
    let gutter_layout = (gutter_w > 0.0).then(|| GutterLayout::new(gutter_rect, &cfg, number_cell));
    if enabled {
        let env = mouse::Env {
            geo: &geo,
            frame: &frame,
            gutter: gutter_layout.as_ref(),
            resp_text: &resp_text,
            resp_gutter: resp_gutter.as_ref(),
            overlay: st.overlay,
            icons: gutter_icons,
            os,
            gutter_click_selects_line,
        };
        let mut out = mouse::MouseOut::default();
        mouse::handle(ui, editor, &mut st, &env, &mut out);
        if out.request_focus {
            resp_text.request_focus();
        }
        navigation = out.navigation;
        gutter_clicks.extend(out.gutter_clicks);
        scroll_request = out.scroll;
        // The cursor.
        if resp_text.hovered() && st.drag.is_none() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                ctx.set_cursor_icon(mouse::cursor_at(editor, &env, pos));
            }
        } else if resp_text.hovered() {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        // Middle click on Linux pastes the primary selection.
        if !viewer
            && os == Os::WindowsLinux
            && !st.primary.is_empty()
            && resp_text.hovered()
            && ui.input(|i| i.pointer.button_released(egui::PointerButton::Middle))
            && st
                .click_pos
                .distance(ui.input(|i| i.pointer.hover_pos().unwrap_or_default()))
                < 4.0
            && editor.carets.block.is_none()
        {
            let text = st.primary.clone();
            let at = mouse::locate(editor, &geo, &frame, st.click_pos).offset;
            editor.carets.set_caret(&editor.document, at);
            let context = crate::editor::model::ActionContext::new(language);
            let content = crate::editor::model::ClipboardContent::text(text);
            let outcome = editor.execute(
                &EditorAction::PasteSimple,
                &context.with_clipboard(&content),
            );
            text_changed |= outcome.text_changed;
        }
    }
    if ui.rect_contains_pointer(gutter_rect) && gutter_w > 0.0 {
        ctx.set_cursor_icon(CursorIcon::Default);
    }
    if os == Os::MacOs
        && let Some(nav) = navigation
        && ui.input(|i| i.pointer.button_released(egui::PointerButton::Middle))
    {
        navigation = Some(nav);
    }

    // Keyboard, text and clipboard.
    let focused_now = enabled && ctx.memory(|m| m.has_focus(id));
    if focused_now {
        let first_row = geo
            .first_visible_row()
            .min(frame.map.total_rows().saturating_sub(1));
        let (v, _) = frame.map.visual_at_row(first_row);
        let cfg = input::InputCfg {
            os,
            viewer,
            tab_indents,
            language,
            first_visible_line: frame.map.visual_to_logical(v),
            visible_lines: visible_rows,
        };
        let out = input::process(ui, editor, &cfg, &mut st);
        text_changed |= out.text_changed;
        if out.scroll.is_some() {
            scroll_request = out.scroll;
        }
        if out.hint.is_some() {
            hint = out.hint;
        }
    }

    // The document may have changed: rebuild what depends on it.
    let dirty = editor.document.modification_stamp() != stamp_before
        || editor
            .folding
            .outer_collapsed()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            != outer_before;
    if dirty {
        frame = build_frame(&ctx, id, editor, highlighter, wrap_cols);
    }
    let cache = frame.cache.clone();

    // Find results.
    let found = if find.open {
        cache.find(&editor.document, &find.query)
    } else {
        std::sync::Arc::new(Vec::new())
    };
    if !found.is_empty() {
        find.current = find.current.min(found.len() - 1);
    }
    let mut jump_to: Option<usize> = None;
    if find.jump
        && let Some(m) = found.get(find.current)
    {
        let mut unfolded = false;
        while let Some(region) = editor.folding.collapsed_region_hiding(m.start) {
            let start = region.start;
            if let Some(index) = editor
                .folding
                .regions()
                .iter()
                .position(|r| r.start == start && !r.expanded)
            {
                editor.folding.set_expanded(index, true);
                unfolded = true;
            } else {
                break;
            }
        }
        if unfolded {
            frame = build_frame(&ctx, id, editor, highlighter, wrap_cols);
        }
        jump_to = Some(m.start);
        find.jump = false;
    }

    // Activity: caret moves and edits restart the blink.
    let carets_after = editor.carets.snapshot();
    let moved = carets_after != carets_before
        || editor.document.modification_stamp() != stamp_before
        || carets_after != st.last_carets;
    if moved || focused_now != st.focused_last {
        st.last_activity = now;
    }
    st.last_carets = carets_after;
    st.last_stamp = editor.document.modification_stamp();
    st.focused_last = focused_now;
    let carets_moved = moved;
    if moved && scroll_request.is_none() && st.drag.is_none() {
        // A caret moved by something else than the keyboard (a click) stays where it is.
    }

    // Scrolling: where the viewport goes this frame.
    let total_rows = frame.map.total_rows();
    let slack = if stick_to_bottom {
        0
    } else {
        editor.settings.scroll_past_end_lines
    };
    let content_h = (total_rows + slack) as f32 * row_h + pad;
    let content_w = match wrap_cols {
        Some(_) => area.width(),
        None => (cache.max_cols + editor.settings.additional_columns) as f32 * cell + 2.0 * pad,
    };
    let current_offset = st.offset;
    let mut target_offset: Option<Vec2> = None;
    let viewport = Viewport {
        first_line: current_offset.y as f64 / row_h as f64,
        visible_lines: area.height() as f64 / row_h as f64,
        first_column: current_offset.x as f64 / cell as f64,
        visible_columns: visible_cols as f64,
    };
    let content = ContentSize {
        lines: total_rows,
        columns: cache.max_cols,
    };
    let mut animate_ms = 0.0;
    let mut settings = editor.settings.clone();
    settings.scroll_past_end_lines = slack;
    if let Some(request) = scroll_request.take() {
        match request {
            ScrollRequest::ToCaret {
                scroll_type,
                animate,
            } => {
                let (row, col) = caret_row_col(editor, &frame);
                let target = scroll_to_caret(
                    &viewport,
                    content,
                    row as f64,
                    col as f64,
                    scroll_type,
                    animate,
                    &settings,
                );
                animate_ms = target.animation_ms;
                target_offset = Some(vec2(
                    if wrap_cols.is_some() {
                        0.0
                    } else {
                        target.first_column as f32 * cell
                    },
                    target.first_line as f32 * row_h,
                ));
            }
            ScrollRequest::Lines(n) => {
                let first = clamp_first_line(
                    viewport.first_line + n as f64,
                    content,
                    viewport.visible_lines,
                    &settings,
                );
                target_offset = Some(vec2(current_offset.x, first as f32 * row_h));
            }
        }
    }
    if let Some(offset) = jump_to {
        let (row, col) = offset_row_col(editor, &frame, offset, 0);
        let target = scroll_to_caret(
            &viewport,
            content,
            row as f64,
            col as f64,
            ScrollType::MakeVisible,
            true,
            &settings,
        );
        animate_ms = target.animation_ms;
        target_offset = Some(vec2(
            if wrap_cols.is_some() {
                0.0
            } else {
                target.first_column as f32 * cell
            },
            target.first_line as f32 * row_h,
        ));
    }
    if let Some(line) = scroll_to_line {
        let line = line.min(editor.document.line_count() - 1);
        let row = frame.map.first_row(frame.map.logical_to_visual(line));
        let first = (row as f64 + 0.5 - viewport.visible_lines / 2.0).max(0.0);
        let first = clamp_first_line(first, content, viewport.visible_lines, &settings);
        target_offset = Some(vec2(current_offset.x, first as f32 * row_h));
        animate_ms = 0.0;
    }
    if ui.input(|i| i.smooth_scroll_delta != Vec2::ZERO) && resp_text.hovered() {
        st.anim = None;
    }
    let mut override_offset = None;
    if let Some(to) = target_offset {
        if animate_ms > 0.0 && to != current_offset {
            st.anim = Some(ScrollAnim {
                from: current_offset,
                to,
                start: now,
                duration: animate_ms / 1000.0,
            });
        } else {
            st.anim = None;
            if to != current_offset {
                override_offset = Some(to);
            }
        }
    }
    if let Some(anim) = st.anim {
        let t = ((now - anim.start) / anim.duration).clamp(0.0, 1.0) as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        override_offset = Some(anim.from + (anim.to - anim.from) * eased);
        if t >= 1.0 {
            st.anim = None;
        } else {
            ctx.request_repaint();
        }
    }

    // The scroll area with the visible rows.
    let caret_visible = {
        let phase = ((now - st.last_activity) / motion::CARET_BLINK as f64).max(0.0);
        (phase.floor() as u64).is_multiple_of(2)
    };
    if focused_now {
        let phase = (now - st.last_activity).max(0.0) % motion::CARET_BLINK as f64;
        ctx.request_repaint_after(Duration::from_secs_f64(
            motion::CARET_BLINK as f64 - phase + 0.001,
        ));
    }
    let brace = if focused_now {
        paint::match_brace(editor, language)
    } else {
        None
    };
    let occurrences = {
        let carets = editor.carets.carets();
        if carets.len() == 1 && carets[0].has_selection() {
            let (s, e) = (carets[0].selection_start(), carets[0].selection_end());
            let text = editor.document.slice(s, e);
            if !text.trim().is_empty() && !text.contains('\n') {
                cache.occurrences(&editor.document, s..e)
            } else {
                std::sync::Arc::new(Vec::new())
            }
        } else {
            std::sync::Arc::new(Vec::new())
        }
    };
    let opts = paint::PaintOpts {
        whitespace,
        indent_guides,
        right_margin,
        focused: focused_now,
        enabled,
        caret_visible,
    };
    let mut child = ui.new_child(UiBuilder::new().max_rect(area));
    child.set_clip_rect(area.intersect(ui.clip_rect()));
    let mut scroll = desktop(egui::ScrollArea::new([wrap_cols.is_none(), true]))
        .id_salt(id.with("scroll"))
        .auto_shrink(false)
        .stick_to_bottom(stick_to_bottom);
    if let Some(offset) = override_offset {
        scroll = scroll.vertical_scroll_offset(offset.y);
        if wrap_cols.is_none() {
            scroll = scroll.horizontal_scroll_offset(offset.x);
        }
    }
    let mut paint_out = paint::PaintOut::default();
    let preedit = st.preedit.clone();
    let area_out = scroll.show_viewport(&mut child, |ui, viewport_rect| {
        ui.allocate_exact_size(
            vec2(
                content_w.max(viewport_rect.width()),
                content_h.max(viewport_rect.height()),
            ),
            Sense::hover(),
        );
        let geo = Geo::new(area, viewport_rect.min.to_vec2(), cell, row_h, pad, ppp);
        if editor.document.is_empty()
            && let Some(placeholder) = &placeholder
        {
            ui.painter().text(
                pos2(geo.x_of(0, 0), geo.row_top(0) + row_h / 2.0),
                egui::Align2::LEFT_CENTER,
                placeholder,
                font.clone(),
                Theme::colors(ui.ctx()).text_placeholder,
            );
        }
        let find_ranges: &[Range<usize>] = &found;
        paint_out = paint::paint(&paint::PaintInput {
            ui,
            painter: ui.painter().clone(),
            editor,
            frame: &frame,
            geo: &geo,
            colors,
            font: &font,
            highlighter,
            opts,
            find: find_ranges,
            find_current: find.open.then_some(find.current),
            occurrences: &occurrences,
            highlights,
            brace,
            preedit: preedit.as_deref(),
        });
    });
    st.offset = area_out.state.offset;
    let geo_final = Geo::new(area, st.offset, cell, row_h, pad, ppp);

    // Gutter.
    if let Some(layout) = &gutter_layout {
        let first_row = geo_final
            .first_visible_row()
            .min(total_rows.saturating_sub(1));
        let last_row = geo_final.last_visible_row(total_rows);
        let out = gutter::show(
            ui,
            &mut st,
            GutterInput {
                editor,
                frame: &frame,
                geo: &geo_final,
                layout,
                cfg,
                colors,
                number_font: &number_font,
                icons: gutter_icons,
                markers,
                first_row,
                last_row,
                enabled,
                id,
            },
        );
        gutter_clicks.extend(out.clicks);
        if out.toggled {
            ctx.request_repaint();
        }
    }

    // Error stripe.
    let mut stripe_hover = None;
    if stripe_w > 0.0 {
        let (hovered, clicked) = stripe(
            ui,
            id,
            editor,
            &frame,
            &colors,
            stripe_rect,
            highlights,
            found.as_slice(),
            content_h,
            row_h,
            enabled,
        );
        stripe_hover = hovered;
        if clicked {
            st.pending_center = true;
            ctx.request_repaint();
        }
    }

    // Hover: rests report after a delay.
    let mut hover = None;
    let mut fold_preview = None;
    if resp_text.hovered() && st.drag.is_none() && enabled {
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            let at = mouse::locate(editor, &geo_final, &frame, pos);
            let still = st
                .rest
                .is_some_and(|r| r.offset == at.offset && r.pos.distance(pos) < 3.0);
            if !still {
                st.rest = Some(state::Rest {
                    offset: at.offset,
                    since: now,
                    pos,
                });
                st.fold_rest = None;
            }
            if let Some(rest) = st.rest {
                let waited = now - rest.since;
                if waited >= motion::EDITOR_TOOLTIP_DELAY as f64 && at.virtual_cols == 0 {
                    let highlight = highlights.iter().position(|h| h.range.contains(&at.offset));
                    hover = Some(HoverInfo {
                        offset: at.offset,
                        pos,
                        highlight,
                    });
                } else {
                    ctx.request_repaint_after(Duration::from_secs_f32(
                        (motion::EDITOR_TOOLTIP_DELAY - waited as f32 + 0.001).max(0.001),
                    ));
                }
            }
            // A fold placeholder under the pointer.
            let layout = frame.layout(&editor.document, at.visual, tab);
            if let Some(seg) = layout.fold_at(at.col) {
                let since = match st.fold_rest {
                    Some((start, since)) if start == seg.off0 => since,
                    _ => {
                        st.fold_rest = Some((seg.off0, now));
                        now
                    }
                };
                if now - since >= motion::FOLD_PREVIEW_DELAY as f64 {
                    fold_preview = Some(seg.off0..seg.off1);
                } else {
                    ctx.request_repaint_after(Duration::from_secs_f32(
                        (motion::FOLD_PREVIEW_DELAY - (now - since) as f32 + 0.001).max(0.001),
                    ));
                }
            } else {
                st.fold_rest = None;
            }
        }
    } else {
        st.rest = None;
        st.fold_rest = None;
    }

    // Find bar over the top right corner.
    st.overlay = None;
    if find.open {
        let events = find::bar(ui, area, &mut find, found.len());
        st.overlay = events.rect;
        if events.query_changed || events.step != 0 {
            ctx.request_repaint();
        }
    }
    let mut response = resp_text;
    if !find.open && find.field_focused(&ctx) {
        response.request_focus();
    }
    let match_count = found.len();
    let find_open = find.open;
    if ctx.memory(|m| m.has_focus(id)) {
        st.last_focus = Some(id);
    } else if let Some(field) = find.field.filter(|f| ctx.memory(|m| m.has_focus(*f))) {
        st.last_focus = Some(field);
    }
    find.store(&ctx, id);

    // Completion list.
    if let Some(completer) = completer
        && !viewer
    {
        if text_changed {
            response.mark_changed();
        }
        let caret = paint_out.caret_rect.map(|rect| {
            let offset = editor.carets.primary().offset;
            (editor.document.text()[..offset].chars().count(), rect)
        });
        let text = editor.document.text().to_owned();
        let chosen = list.end_with(ui, completer, &text, &mut response, caret);
        if let Some((start, end, item)) = chosen {
            apply_completion(editor, language, start, end, &item);
            text_changed = true;
            st.last_activity = now;
        }
    }

    paint_border(ui, outer, response.has_focus());

    // Keep Tab and Escape for the editor.
    if focused_now {
        let escape = find_open
            || editor.carets.caret_count() > 1
            || editor.carets.has_selection()
            || list_open;
        ctx.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                EventFilter {
                    tab: !viewer && tab_indents,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape,
                },
            );
        });
        if !viewer {
            ui.output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: area,
                    cursor_rect: paint_out.caret_rect.unwrap_or(area),
                })
            });
        }
    }

    if text_changed {
        response.mark_changed();
    }

    // Accessibility: a document (viewer) or a text input (editor) named by the label.
    let name = label.unwrap_or_else(|| "Code".to_owned());
    if viewer {
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, name.clone()));
    } else {
        response.widget_info(|| {
            WidgetInfo::text_edit(enabled, String::new(), String::new(), name.clone())
        });
    }
    let text_len = editor.document.len();
    ctx.accesskit_node_builder(response.id, |node| {
        if viewer {
            node.set_role(accesskit::Role::Document);
        }
        node.set_label(name.clone());
        if text_len <= A11Y_VALUE_LIMIT {
            node.set_value(editor.document.text());
        }
    });

    let primary = editor.carets.primary();
    let selection = primary
        .has_selection()
        .then(|| primary.selection_start()..primary.selection_end());
    let line_count = editor.document.line_count();
    st.store(&ctx, id);
    EditorComponentResponse {
        response,
        text_changed,
        carets_moved,
        navigation,
        gutter_clicks,
        hover,
        fold_preview,
        stripe_hover,
        hint,
        selection,
        caret_rect: paint_out.caret_rect,
        line_count,
        visible_lines: paint_out.visible_lines,
        laid_out_rows: paint_out.laid_out_rows,
        matches: match_count,
    }
}

/// Applies an accepted completion: replaces the chars `start..end` with the suggestion as one
/// undo step and puts the caret where the suggestion asks.
fn apply_completion(
    editor: &mut Editor,
    _language: &Language,
    start: usize,
    end: usize,
    item: &crate::ui::Completion,
) {
    let text = editor.document.text();
    let byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b)
    };
    let (from, to) = (byte(start), byte(end.max(start)));
    let inserted = item.insert.chars().count();
    let after_chars = item.caret.unwrap_or(inserted).min(inserted);
    editor.replace_range(from, to, &item.insert);
    let after_byte: usize = item
        .insert
        .chars()
        .take(after_chars)
        .map(char::len_utf8)
        .sum();
    editor.carets.set_caret(&editor.document, from + after_byte);
}

/// Paints and operates the error stripe: marks proportional to the document, a click moves
/// the caret to the mark and centers it. Returns the mark under the pointer.
#[allow(clippy::too_many_arguments)]
fn stripe(
    ui: &mut Ui,
    id: Id,
    editor: &mut Editor,
    frame: &Frame,
    colors: &crate::theme::EditorColors,
    rect: Rect,
    highlights: &[RangeHighlight],
    found: &[Range<usize>],
    content_h: f32,
    row_h: f32,
    enabled: bool,
) -> (Option<usize>, bool) {
    use crate::editor::HighlightSeverity::*;
    let resp = ui.interact(
        rect,
        id.with("stripe"),
        if enabled {
            Sense::CLICK
        } else {
            Sense::hover()
        },
    );
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let track = rect.height().max(1.0);
    let y_of = |offset: usize| {
        let v = frame.visual_of(editor, offset);
        let row = frame.map.first_row(v);
        rect.min.y + (row as f32 * row_h / content_h.max(1.0)) * track
    };
    let theme = crate::Theme::colors(ui.ctx());
    let mark_rect = |y: f32| {
        Rect::from_min_size(
            pos2(rect.min.x + 3.0, y),
            vec2(rect.width() - 6.0, metrics::STRIPE_MARK_MIN),
        )
    };
    let pointer = ui
        .input(|i| i.pointer.hover_pos())
        .filter(|p| rect.contains(*p));
    let mut hovered = None;
    let draw = |range: &Range<usize>, color| {
        let y = y_of(range.start);
        let r = mark_rect(y);
        painter.rect_filled(r, 0.0, color);
        r
    };
    for m in found.iter().take(2000) {
        draw(m, colors.search_stripe);
    }
    for (i, h) in highlights.iter().enumerate() {
        let color = match h.severity {
            Error => colors.error_stripe,
            Warning => colors.warning_stripe,
            WeakWarning => colors.weak_warning_effect,
            Info => theme.link,
            TextSearch => colors.search_stripe,
        };
        let r = draw(&h.range, color);
        if pointer.is_some_and(|p| (p.y - r.center().y).abs() <= metrics::STRIPE_MARK_MIN + 1.0) {
            hovered = Some(i);
        }
    }
    let mut clicked = false;
    if resp.clicked()
        && let Some(i) = hovered
    {
        let start = highlights[i].range.start.min(editor.document.len());
        editor.carets.set_caret(&editor.document, start);
        clicked = true;
    }
    (hovered, clicked)
}
