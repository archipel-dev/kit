//! [`CodeView`]: read-only, virtualized code.

use super::find::{self, FindState};
use super::highlight::{Highlighter, PlainText, Span, styled_job, token_format};
use super::text::{self, Cache};
use super::{
    A11Y_VALUE_LIMIT, code_font, gutter_width, paint_background, paint_border, paint_number,
};
use crate::icons;
use crate::parts::desktop;
use crate::theme::Theme;
use crate::tokens::{radius, size};
use crate::{Glyph, cursor};
use egui::{
    CornerRadius, CursorIcon, Event, Id, Key, Modifiers, Pos2, Rect, Response, Sense, Ui,
    UiBuilder, Vec2, Widget, WidgetInfo, WidgetType, accesskit, pos2, vec2,
};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;

/// A position in the text: line and display column (a tab counts up to its tab stop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Pos {
    line: usize,
    col: usize,
}

/// What a view remembers between frames.
#[derive(Clone, Default)]
struct ViewState {
    anchor: Pos,
    head: Pos,
    folded: BTreeSet<usize>,
    offset: Vec2,
}

impl ViewState {
    fn selection(&self) -> Option<(Pos, Pos)> {
        (self.anchor != self.head).then(|| (self.anchor.min(self.head), self.anchor.max(self.head)))
    }
}

/// Read-only, monospace code with a line-number gutter, selection, find and optional
/// folding. Only the visible lines are laid out, so a 10 MB, 200 000-line body scrolls as
/// smoothly as a short one.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::CodeView;
/// use archipel_kit::components::code::Json;
///
/// let body = r#"{ "id": 7, "tags": ["a", "b"] }"#;
/// CodeView::new(body)
///     .label("Response body")
///     .highlighter(&Json)
///     .line_numbers(true)
///     .height(120.0)
///     .show(ui);
/// # });
/// ```
///
/// - Click and drag select, double click selects a word, triple click a line; `Ctrl+A`
///   selects everything and `Ctrl+C` copies the selection.
/// - `Ctrl+F` opens the find bar: all matches are highlighted, the current one stronger;
///   `Enter` and `Shift+Enter` (or the arrows) step through them, `Esc` closes it.
/// - Lines are a monospace grid: tabs expand to four columns, and wide or combining
///   characters are laid out one cell per `char`.
/// - The text is borrowed and never copied. The view keeps an index of it (line starts,
///   highlighter checkpoints) and recognizes the text by its length, address and a
///   sample of its content; after changing a text in place without changing its length,
///   pass a new [`revision`](CodeView::revision).
#[must_use = "show it with `view.show(ui)` or add it with `ui.add(view)`"]
pub struct CodeView<'a> {
    text: &'a str,
    label: Option<String>,
    highlighter: &'a dyn Highlighter,
    line_numbers: bool,
    wrap: bool,
    folding: bool,
    scroll_to: Option<usize>,
    revision: u64,
    height: Option<f32>,
    id: Option<Id>,
}

/// What [`CodeView::show`] reports.
pub struct CodeViewResponse {
    /// The view's response: focus, hover, and the accessible node.
    pub response: Response,
    /// Number of lines in the text.
    pub line_count: usize,
    /// The lines that are at least partly visible (without the lines hidden by folds).
    pub visible_lines: Range<usize>,
    /// How many rows were laid out this frame: the visible ones, never the whole text.
    pub laid_out_rows: usize,
    /// Matches of the find bar (0 while it is closed).
    pub matches: usize,
}

impl<'a> CodeView<'a> {
    /// A view of `text`.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            label: None,
            highlighter: &PlainText,
            line_numbers: false,
            wrap: false,
            folding: false,
            scroll_to: None,
            revision: 0,
            height: None,
            id: None,
        }
    }

    /// The accessible name of the view ("Response body"). Texts up to 64 KB are also
    /// exposed as its value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The [`Highlighter`] that colors the text (default: none).
    pub fn highlighter(mut self, highlighter: &'a dyn Highlighter) -> Self {
        self.highlighter = highlighter;
        self
    }

    /// Shows line numbers in a gutter.
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Wraps long lines at the view's width instead of scrolling sideways.
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Fold arrows in the gutter for `{…}` and `[…]` regions that span several lines.
    pub fn folding(mut self, on: bool) -> Self {
        self.folding = on;
        self
    }

    /// Scrolls so that `line` (0-based) is centered, on the frame it is passed. Pass it
    /// once, for example from a "go to line" action; passing it every frame pins the view.
    pub fn scroll_to_line(mut self, line: Option<usize>) -> Self {
        self.scroll_to = line;
        self
    }

    /// Bump it when the text changes without changing its length or address.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Height in points (default: the available height, or 300 when it is unbounded).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Gives the view an explicit id, for several views sharing one place in the ui.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Draws the view.
    pub fn show(self, ui: &mut Ui) -> CodeViewResponse {
        View::new(self, ui).run(ui)
    }
}

impl Widget for CodeView<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// Which text row is where, with wrapping and folds applied.
struct Rows {
    prefix: Option<Arc<Vec<usize>>>,
    lines: usize,
    /// Folded regions (first line, last line): sorted, disjoint.
    regions: Vec<(usize, usize)>,
    /// Rows hidden by the regions before each region.
    hidden_before: Vec<usize>,
}

impl Rows {
    fn new(
        lines: usize,
        prefix: Option<Arc<Vec<usize>>>,
        folds: &[(usize, usize)],
        folded: &BTreeSet<usize>,
    ) -> Self {
        let mut rows = Rows {
            prefix,
            lines,
            regions: Vec::new(),
            hidden_before: Vec::new(),
        };
        for &start in folded {
            if let Ok(at) = folds.binary_search_by_key(&start, |r| r.0)
                && rows.regions.last().is_none_or(|last| start > last.1)
            {
                rows.regions.push(folds[at]);
            }
        }
        let mut hidden = 0;
        for &(start, end) in &rows.regions {
            rows.hidden_before.push(hidden);
            hidden += rows.prefix(end + 1) - rows.prefix(start + 1);
        }
        rows
    }

    fn prefix(&self, line: usize) -> usize {
        self.prefix.as_ref().map_or(line, |p| p[line])
    }

    /// First row of `line`; every line of a folded region shares the row after its start.
    fn pos(&self, line: usize) -> usize {
        let k = self.regions.partition_point(|r| r.0 < line);
        if k == 0 {
            return self.prefix(line);
        }
        let (start, end) = self.regions[k - 1];
        let upto = line.min(end + 1);
        self.prefix(line) - self.hidden_before[k - 1] - (self.prefix(upto) - self.prefix(start + 1))
    }

    fn total(&self) -> usize {
        self.pos(self.lines)
    }

    fn rows_of(&self, line: usize) -> usize {
        self.prefix(line + 1) - self.prefix(line)
    }

    /// The region folded at `line`, if `line` is its first line.
    fn folded_at(&self, line: usize) -> Option<(usize, usize)> {
        self.regions
            .binary_search_by_key(&line, |r| r.0)
            .ok()
            .map(|i| self.regions[i])
    }

    /// The line shown on `row`.
    fn row_to_line(&self, row: usize) -> usize {
        let line = self
            .lines_partition(row)
            .saturating_sub(1)
            .min(self.lines.saturating_sub(1));
        let k = self.regions.partition_point(|r| r.0 < line);
        match k.checked_sub(1).map(|k| self.regions[k]) {
            Some((start, end)) if line <= end => {
                if end + 1 < self.lines {
                    end + 1
                } else {
                    start
                }
            }
            _ => line,
        }
    }

    fn lines_partition(&self, row: usize) -> usize {
        let (mut lo, mut hi) = (0, self.lines);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.pos(mid) <= row {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }
}

/// A text row to draw: part of a line, in columns.
struct RowPaint {
    row: usize,
    line: usize,
    first_of_line: bool,
}

struct View<'a> {
    spec: CodeView<'a>,
    id: Id,
    state: ViewState,
    find: FindState,
}

impl<'a> View<'a> {
    fn new(spec: CodeView<'a>, ui: &mut Ui) -> Self {
        let id = spec.id.unwrap_or_else(|| ui.next_auto_id());
        ui.skip_ahead_auto_ids(1);
        let state = ui
            .ctx()
            .data(|d| d.get_temp(id.with("archipel-code-view")))
            .unwrap_or_default();
        let find = FindState::load(ui.ctx(), id);
        Self {
            spec,
            id,
            state,
            find,
        }
    }

    fn run(mut self, ui: &mut Ui) -> CodeViewResponse {
        let ctx = ui.ctx().clone();
        let theme = Theme::current(&ctx);
        let editor = theme.editor;
        let (font, cell) = code_font(ui);
        let line_height = size::CODE_LINE_HEIGHT;
        let pad = size::CODE_PADDING;
        let top = pad / 2.0;
        let text = self.spec.text;
        let highlighter = self.spec.highlighter;

        let cache = Cache::get(&ctx, self.id, text, highlighter, self.spec.revision);
        let line_count = cache.lines.len();

        // Frame: the available width, a fixed height.
        let height = self.spec.height.unwrap_or_else(|| {
            let h = ui.available_height();
            if h.is_finite() && (40.0..10_000.0).contains(&h) {
                h
            } else {
                300.0
            }
        });
        let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        paint_background(ui, outer);
        let inner = outer.shrink(1.0);
        let folding = self.spec.folding;
        let gutter_w = gutter_width(line_count, cell, self.spec.line_numbers, folding);
        let gutter = Rect::from_min_max(inner.min, pos2(inner.min.x + gutter_w, inner.max.y));
        let area = Rect::from_min_max(pos2(inner.min.x + gutter_w, inner.min.y), inner.max);

        // Rows.
        let scroll_bar = ui.spacing().scroll.allocated_width();
        let wrap_cols = self
            .spec
            .wrap
            .then(|| (((area.width() - 2.0 * pad - scroll_bar) / cell).floor() as usize).max(1));
        let folds = if folding {
            cache.folds(text)
        } else {
            Arc::new(Vec::new())
        };

        // Find: matches, and unfold what the current match hides.
        let key_focused = ctx.memory(|m| m.has_focus(self.id));
        let closed_by_escape = self.find.keys(ui, key_focused);
        if closed_by_escape {
            ctx.memory_mut(|m| m.request_focus(self.id));
        }
        let found = if self.find.open {
            cache.find(text, &self.find.query)
        } else {
            Arc::new(Vec::new())
        };
        if !found.is_empty() {
            self.find.current = self.find.current.min(found.len() - 1);
        }
        let current = found.get(self.find.current).cloned();
        if self.find.jump
            && let Some(m) = &current
        {
            let line = cache.line_of_byte(m.start);
            self.state.folded.retain(|&s| {
                folds
                    .binary_search_by_key(&s, |r| r.0)
                    .map_or(true, |i| !(s < line && line <= folds[i].1))
            });
        }

        let prefix = wrap_cols.map(|cols| cache.wrap_prefix(cols));
        let rows = Rows::new(line_count, prefix, &folds, &self.state.folded);
        let total_rows = rows.total();
        let content_w = match wrap_cols {
            Some(_) => area.width(),
            None => cache.max_cols as f32 * cell + 2.0 * pad + 3.0 * cell,
        };
        let content_h = total_rows as f32 * line_height + pad;

        // Scroll requests: "go to line" and the find jump.
        let mut scroll_y = None;
        let mut scroll_x = None;
        let row_of = |line: usize, col: usize| {
            rows.pos(line.min(line_count - 1)) + wrap_cols.map_or(0, |wc| col / wc)
        };
        let center_y = |row: usize| {
            (top + row as f32 * line_height - (area.height() - line_height) / 2.0).max(0.0)
        };
        if let Some(line) = self.spec.scroll_to {
            scroll_y = Some(center_y(row_of(line, 0)));
        }
        if self.find.jump
            && let Some(m) = &current
        {
            let line = cache.line_of_byte(m.start);
            let info = &cache.lines[line];
            let content = cache.line(text, line);
            let col = text::col_of_byte(content, info.simple, m.start - info.start);
            let row = row_of(line, col);
            let y = top + row as f32 * line_height;
            let off = self.state.offset;
            if y < off.y || y + line_height > off.y + area.height() {
                scroll_y = Some(center_y(row));
            }
            if wrap_cols.is_none() {
                let x = pad + col as f32 * cell;
                if x < off.x || x + cell > off.x + area.width() - scroll_bar {
                    scroll_x = Some((x - area.width() / 2.0).max(0.0));
                }
            }
            self.find.jump = false;
        }

        // Scroll area with the visible rows.
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        child.set_clip_rect(area.intersect(ui.clip_rect()));
        let mut scroll = desktop(egui::ScrollArea::new([wrap_cols.is_none(), true]))
            .id_salt(self.id.with("scroll"))
            .auto_shrink(false);
        if let Some(y) = scroll_y {
            scroll = scroll.vertical_scroll_offset(y);
        }
        if let Some(x) = scroll_x {
            scroll = scroll.horizontal_scroll_offset(x);
        }
        let mut painted: Vec<RowPaint> = Vec::new();
        let mut laid_out = 0;
        let mut response = None;
        let out = scroll.show_viewport(&mut child, |ui, viewport| {
            let (content, _) = ui.allocate_exact_size(
                vec2(
                    content_w.max(viewport.width()),
                    content_h.max(viewport.height()),
                ),
                Sense::hover(),
            );
            let visible =
                Rect::from_min_size(content.min + viewport.min.to_vec2(), viewport.size());
            let resp = ui.interact(
                visible,
                self.id,
                Sense::click_and_drag() | Sense::focusable_noninteractive(),
            );
            let left = content.min.x + pad;
            let row_y = |row: usize| content.min.y + top + row as f32 * line_height;

            // Where the pointer is, as a position in the text.
            let locate = |p: Pos2| -> Pos {
                let row = (((p.y - content.min.y - top) / line_height).floor().max(0.0)) as usize;
                let row = row.min(total_rows.saturating_sub(1));
                let line = rows.row_to_line(row);
                let k = row.saturating_sub(rows.pos(line));
                let base = wrap_cols.map_or(0, |wc| k * wc);
                let col = ((p.x - left) / cell).round().max(0.0) as usize + base;
                Pos {
                    line,
                    col: col.min(cache.lines[line].cols),
                }
            };

            // Pointer.
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Text);
            }
            if let Some(p) = pointer {
                if resp.hovered() && ui.input(|i| i.pointer.primary_pressed()) {
                    resp.request_focus();
                    let at = locate(p);
                    self.state.anchor = at;
                    self.state.head = at;
                }
                if resp.double_clicked() || resp.triple_clicked() {
                    let at = locate(p);
                    let (a, b) = if resp.triple_clicked() {
                        (
                            Pos {
                                line: at.line,
                                col: 0,
                            },
                            Pos {
                                line: at.line,
                                col: cache.lines[at.line].cols,
                            },
                        )
                    } else {
                        word_at(&cache, text, at)
                    };
                    self.state.anchor = a;
                    self.state.head = b;
                } else if resp.dragged() {
                    self.state.head = locate(p);
                    let scroll_by = vec2(
                        edge_delta(p.x, visible.left(), visible.right()),
                        edge_delta(p.y, visible.top(), visible.bottom()),
                    );
                    if scroll_by != Vec2::ZERO {
                        ui.scroll_with_delta(scroll_by);
                        ui.ctx().request_repaint();
                    }
                }
            }

            // Keys.
            if resp.has_focus() {
                self.keys(ui, &cache, text, viewport.height());
            }

            // Paint the visible rows.
            let first_row = ((viewport.min.y - top) / line_height).floor().max(0.0) as usize;
            let last_row = (((viewport.max.y - top) / line_height).ceil().max(0.0) as usize)
                .min(total_rows.saturating_sub(1));
            let (hc0, hc1) = {
                let c0 =
                    (((viewport.min.x - pad) / cell).floor().max(0.0) as usize).saturating_sub(2);
                let c1 =
                    (((viewport.max.x - pad) / cell).ceil().max(0.0) as usize).saturating_add(2);
                (c0, c1)
            };
            let selection = self.state.selection();
            let mut line = rows.row_to_line(first_row);
            let mut hl_state = cache.state_at(text, highlighter, line);
            let mut scratch = Vec::new();
            while line < line_count && rows.pos(line) <= last_row {
                let info = &cache.lines[line];
                let content_text = cache.line(text, line);
                scratch.clear();
                let long = cache.spans(text, highlighter, line, &mut hl_state, &mut scratch);
                let spans: &[Span] = long.as_deref().map_or(&scratch, |v| v);
                let (disp, dspans): (Cow<str>, Cow<[Span]>) = if info.simple {
                    (Cow::Borrowed(content_text), Cow::Borrowed(spans))
                } else {
                    let (s, sp) = text::expand(content_text, spans);
                    (Cow::Owned(s), Cow::Owned(sp))
                };
                let ascii = info.simple || disp.is_ascii();
                let line_rows = rows.rows_of(line);
                let first = rows.pos(line);
                for k in 0..line_rows {
                    let row = first + k;
                    if row < first_row {
                        continue;
                    }
                    if row > last_row {
                        break;
                    }
                    let (c0, c1) = match wrap_cols {
                        Some(wc) => (k * wc, (k + 1) * wc),
                        None => (hc0, hc1),
                    };
                    let base = if wrap_cols.is_some() { c0 } else { 0 };
                    let x_of = |col: usize| {
                        left + (col.saturating_sub(base)) as f32 * cell
                            - if col < base {
                                (base - col) as f32 * cell
                            } else {
                                0.0
                            }
                    };
                    let y = row_y(row);
                    let row_rect = |a: usize, b: usize| {
                        Rect::from_min_max(pos2(x_of(a), y), pos2(x_of(b), y + line_height))
                    };
                    // Selection.
                    if let Some((s, e)) = selection
                        && s.line <= line
                        && line <= e.line
                    {
                        let a = if line == s.line { s.col } else { 0 };
                        let b = if line == e.line { e.col } else { info.cols + 1 };
                        let (a, b) = (
                            a.max(c0),
                            b.min(c1.max(info.cols + 1)).min(if k + 1 == line_rows {
                                usize::MAX
                            } else {
                                c1
                            }),
                        );
                        if a < b {
                            ui.painter()
                                .rect_filled(row_rect(a, b), 0.0, editor.selection);
                        }
                    }
                    // Find matches.
                    if !found.is_empty() {
                        let (lo_b, hi_b) = if info.simple {
                            (info.start + c0, info.start + c1)
                        } else {
                            (info.start, info.end)
                        };
                        let from = found.partition_point(|m| m.end <= lo_b);
                        for (i, m) in found[from..].iter().enumerate() {
                            if m.start >= hi_b {
                                break;
                            }
                            let a =
                                text::col_of_byte(content_text, info.simple, m.start - info.start);
                            let b =
                                text::col_of_byte(content_text, info.simple, m.end - info.start);
                            let (a, b) = (a.max(c0), b.min(c1));
                            if a < b {
                                let color = if from + i == self.find.current {
                                    editor.search_result_current
                                } else {
                                    editor.search_result
                                };
                                ui.painter().rect_filled(row_rect(a, b), 2.0, color);
                            }
                        }
                    }
                    // Text.
                    let (b0, b1) = byte_window(&disp, ascii, c0, c1);
                    if b0 < b1 {
                        let galley = layout_row(ui, &font, &editor, &disp[b0..b1], &dspans, b0);
                        laid_out += 1;
                        ui.painter().galley(pos2(x_of(c0), y), galley, editor.text);
                    }
                    // The pill of a folded region.
                    if k + 1 == line_rows && rows.folded_at(line).is_some() {
                        let x = x_of(info.cols.max(c0)) + cell;
                        let pill = Rect::from_min_size(
                            pos2(x, y + 3.0),
                            vec2(cell * 3.0, line_height - 6.0),
                        );
                        ui.painter().rect_filled(
                            pill,
                            CornerRadius::same(radius::CONTROL),
                            editor.background_panel,
                        );
                        ui.painter().text(
                            pill.center(),
                            egui::Align2::CENTER_CENTER,
                            "…",
                            font.clone(),
                            editor.comment,
                        );
                    }
                    painted.push(RowPaint {
                        row,
                        line,
                        first_of_line: k == 0,
                    });
                }
                line = match rows.folded_at(line) {
                    Some((_, end)) => {
                        let next = end + 1;
                        if next < line_count {
                            hl_state = cache.state_at(text, highlighter, next);
                        }
                        next
                    }
                    None => line + 1,
                };
            }
            response = Some(resp);
        });
        let response = response.expect("scroll area ran its content");
        self.state.offset = out.state.offset;

        // Gutter: numbers and fold arrows, outside the scroll area so sideways scrolling
        // leaves it in place.
        let mut visible_lines = 0..0;
        if let (Some(first), Some(last)) = (painted.first(), painted.last()) {
            visible_lines = first.line..last.line + 1;
        }
        if gutter_w > 0.0 {
            let painter = ui
                .painter()
                .with_clip_rect(gutter.intersect(ui.clip_rect()));
            for item in painted.iter().filter(|p| p.first_of_line) {
                let y = area.min.y + top + item.row as f32 * line_height - out.state.offset.y;
                if self.spec.line_numbers {
                    paint_number(
                        &painter,
                        gutter,
                        y + line_height / 2.0,
                        item.line + 1,
                        &font,
                        editor.line_number,
                        folding,
                    );
                }
                if folding && let Ok(at) = folds.binary_search_by_key(&item.line, |r| r.0) {
                    let (start, end) = folds[at];
                    let folded = self.state.folded.contains(&start);
                    let rect = Rect::from_min_size(
                        pos2(gutter.right() - size::CHEVRON_COLUMN, y),
                        vec2(size::CHEVRON_COLUMN, line_height),
                    );
                    let click = ui.interact(
                        rect.intersect(gutter),
                        self.id.with(("fold", start)),
                        Sense::click(),
                    );
                    let verb = if folded { "Unfold" } else { "Fold" };
                    click.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            true,
                            format!("{verb} lines {} to {}", start + 1, end + 1),
                        )
                    });
                    cursor::on_hover(&click, CursorIcon::Default);
                    let color = if click.hovered() {
                        editor.line_number_caret
                    } else {
                        editor.line_number
                    };
                    let glyph = if folded {
                        Glyph::ChevronRight
                    } else {
                        Glyph::ChevronDown
                    };
                    icons::paint(
                        &painter,
                        Rect::from_center_size(rect.center(), vec2(size::ICON, size::ICON)),
                        glyph,
                        color,
                    );
                    if click.clicked() && !self.state.folded.remove(&start) {
                        self.state.folded.insert(start);
                    }
                }
            }
            if ui.rect_contains_pointer(gutter) {
                ui.ctx().set_cursor_icon(CursorIcon::Default);
            }
        }

        // Find bar, over the top right corner.
        if self.find.open {
            let events = find::bar(ui, area, &mut self.find, found.len());
            if events.query_changed || events.step != 0 {
                ctx.request_repaint();
            }
        }

        paint_border(ui, outer, response.has_focus());
        if response.has_focus() && self.find.open {
            // Escape closes the bar instead of dropping the focus.
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    self.id,
                    egui::EventFilter {
                        escape: true,
                        ..egui::EventFilter::default()
                    },
                );
            });
        }

        // Accessibility: a document named by the label, with the text as its value.
        let label = self.spec.label.clone().unwrap_or_else(|| "Code".to_owned());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, label.clone()));
        ctx.accesskit_node_builder(response.id, |node| {
            node.set_role(accesskit::Role::Document);
            node.set_label(label.clone());
            if text.len() <= A11Y_VALUE_LIMIT {
                node.set_value(text);
            }
        });

        let matches = found.len();
        self.find.clone().store(&ctx, self.id);
        ctx.data_mut(|d| d.insert_temp(self.id.with("archipel-code-view"), self.state.clone()));
        CodeViewResponse {
            response,
            line_count,
            visible_lines,
            laid_out_rows: laid_out,
            matches,
        }
    }

    /// Ctrl+A, Ctrl+C and scrolling keys.
    fn keys(&mut self, ui: &Ui, cache: &Cache, text: &str, page: f32) {
        let (select_all, copy) = ui.input_mut(|i| {
            let all = i.consume_key(Modifiers::COMMAND, Key::A);
            let key = i.consume_key(Modifiers::COMMAND, Key::C);
            let event = i.events.iter().any(|e| matches!(e, Event::Copy));
            i.events.retain(|e| !matches!(e, Event::Copy));
            (all, key || event)
        });
        if select_all {
            let last = cache.lines.len() - 1;
            self.state.anchor = Pos { line: 0, col: 0 };
            self.state.head = Pos {
                line: last,
                col: cache.lines[last].cols,
            };
        }
        if copy && let Some((a, b)) = self.state.selection() {
            ui.ctx().copy_text(selected_text(cache, text, a, b));
        }
        let step = ui.input_mut(|i| {
            let mut dy = 0.0;
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                dy -= size::CODE_LINE_HEIGHT;
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                dy += size::CODE_LINE_HEIGHT;
            }
            if i.consume_key(Modifiers::NONE, Key::PageDown) {
                dy -= page;
            }
            if i.consume_key(Modifiers::NONE, Key::PageUp) {
                dy += page;
            }
            if i.consume_key(Modifiers::COMMAND, Key::Home) {
                dy = f32::INFINITY;
            }
            if i.consume_key(Modifiers::COMMAND, Key::End) {
                dy = f32::NEG_INFINITY;
            }
            dy
        });
        if step != 0.0 {
            ui.scroll_with_delta(vec2(0.0, step));
        }
    }
}

/// How fast to scroll while a drag is beyond `low..high` on one axis.
fn edge_delta(p: f32, low: f32, high: f32) -> f32 {
    if p < low {
        (low - p).min(40.0)
    } else if p > high {
        -(p - high).min(40.0)
    } else {
        0.0
    }
}

/// The word around `at`, or the single char there.
fn word_at(cache: &Cache, text: &str, at: Pos) -> (Pos, Pos) {
    let info = &cache.lines[at.line];
    let line = cache.line(text, at.line);
    let byte = text::byte_of_col(line, info.simple, at.col);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let before = line[..byte]
        .chars()
        .rev()
        .take_while(|&c| is_word(c))
        .map(char::len_utf8)
        .sum::<usize>();
    let after = line[byte..]
        .chars()
        .take_while(|&c| is_word(c))
        .map(char::len_utf8)
        .sum::<usize>();
    let (start, end) = if before + after == 0 {
        let next = line[byte..].chars().next().map_or(0, char::len_utf8);
        (byte, byte + next)
    } else {
        (byte - before, byte + after)
    };
    (
        Pos {
            line: at.line,
            col: text::col_of_byte(line, info.simple, start),
        },
        Pos {
            line: at.line,
            col: text::col_of_byte(line, info.simple, end),
        },
    )
}

/// The text between `a` and `b` (line breaks included).
fn selected_text(cache: &Cache, text: &str, a: Pos, b: Pos) -> String {
    let byte = |p: Pos| {
        let info = &cache.lines[p.line];
        info.start + text::byte_of_col(cache.line(text, p.line), info.simple, p.col)
    };
    text[byte(a)..byte(b).max(byte(a))].to_owned()
}

/// Byte range of columns `c0..c1` in `disp`.
fn byte_window(disp: &str, ascii: bool, c0: usize, c1: usize) -> (usize, usize) {
    if ascii {
        return (c0.min(disp.len()), c1.min(disp.len()));
    }
    let mut chars = disp
        .char_indices()
        .map(|(b, _)| b)
        .chain(std::iter::once(disp.len()));
    let b0 = chars.nth(c0).unwrap_or(disp.len());
    let b1 = if c1 > c0 {
        chars.nth(c1 - c0 - 1).unwrap_or(disp.len())
    } else {
        b0
    };
    (b0, b1)
}

/// One row: `slice` (starting at byte `offset` of the displayed line) colored by `spans`.
fn layout_row(
    ui: &Ui,
    font: &egui::FontId,
    colors: &crate::theme::EditorColors,
    slice: &str,
    spans: &[Span],
    offset: usize,
) -> Arc<egui::Galley> {
    let job = styled_job(slice, offset, spans, f32::INFINITY, |kind| {
        token_format(kind, font, colors, Some(size::CODE_LINE_HEIGHT), true)
    });
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}
