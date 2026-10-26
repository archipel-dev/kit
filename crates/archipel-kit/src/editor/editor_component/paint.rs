//! Painting the text area: backgrounds and highlights, the Islands selection, text, guides,
//! whitespace, fold placeholders, squiggles and carets.

use super::layout::LineLayout;
use super::state::{Frame, Geo};
use crate::editor::decorations::{HighlightSeverity, RangeHighlight};
use crate::editor::highlight::{Highlighter, styled_job, token_format};
use crate::editor::model::{Editor, Language, LogicalPosition, advance_column};
use crate::editor::selection_shape;
use crate::theme::EditorColors;
use crate::tokens::editor as metrics;
use egui::{
    Align2, Color32, CornerRadius, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Ui, pos2, vec2,
};
use std::ops::Range;

/// What the painter shows besides the text.
#[derive(Clone, Copy)]
pub(crate) struct PaintOpts {
    pub whitespace: bool,
    pub indent_guides: bool,
    pub right_margin: bool,
    pub focused: bool,
    pub enabled: bool,
    pub caret_visible: bool,
}

/// The matched (or unmatched) brace at the caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BracePair {
    pub brace: usize,
    pub partner: Option<usize>,
}

/// Everything the painter reads.
pub(crate) struct PaintInput<'a> {
    pub ui: &'a Ui,
    pub painter: Painter,
    pub editor: &'a Editor,
    pub frame: &'a Frame,
    pub geo: &'a Geo,
    pub colors: EditorColors,
    pub font: &'a FontId,
    pub highlighter: &'a dyn Highlighter,
    pub opts: PaintOpts,
    pub find: &'a [Range<usize>],
    pub find_current: Option<usize>,
    pub occurrences: &'a [Range<usize>],
    pub highlights: &'a [RangeHighlight],
    pub brace: Option<BracePair>,
    pub preedit: Option<&'a str>,
}

/// What painting found out.
#[derive(Default)]
pub(crate) struct PaintOut {
    pub laid_out_rows: usize,
    /// The primary caret on screen (also while it is blinked off).
    pub caret_rect: Option<Rect>,
    /// The logical lines at least partly visible.
    pub visible_lines: Range<usize>,
}

struct RowInfo {
    row: usize,
    line: usize,
    k: usize,
    c0: usize,
    c1: usize,
    base: usize,
    y: f32,
}

/// The height of a caret: the font's, shorter than the spaced row (spec §1.9).
pub(crate) fn caret_height(ui: &Ui, font: &FontId, row_h: f32) -> f32 {
    ui.fonts_mut(|f| f.row_height(font)).min(row_h)
}

/// Paints the visible rows.
pub(crate) fn paint(input: &PaintInput<'_>) -> PaintOut {
    let PaintInput {
        ui,
        painter,
        editor,
        frame,
        geo,
        colors,
        font,
        highlighter,
        opts,
        ..
    } = input;
    let doc = &editor.document;
    let tab = editor.settings.tab_size;
    let total_rows = frame.map.total_rows();
    let mut out = PaintOut::default();
    if total_rows == 0 {
        return out;
    }
    let first_row = geo.first_visible_row();
    let last_row = geo.last_visible_row(total_rows);
    // One extra row on each side keeps the selection outline right at the viewport edge.
    let ext_first = first_row.saturating_sub(1);
    let ext_last = (last_row + 1).min(total_rows - 1);
    let wrap = frame.map.wrap_cols();
    let (hc0, hc1) = {
        let c0 =
            (((geo.offset.x - geo.pad) / geo.cell).floor().max(0.0) as usize).saturating_sub(2);
        let c1 = (((geo.offset.x - geo.pad + geo.area.width()) / geo.cell)
            .ceil()
            .max(0.0) as usize)
            .saturating_add(2);
        (c0, c1)
    };

    // Lines and rows.
    let mut lines: Vec<LineLayout<'_>> = Vec::new();
    let mut rows: Vec<RowInfo> = Vec::new();
    let mut row = ext_first;
    while row <= ext_last {
        let (v, k) = frame.map.visual_at_row(row);
        if lines.last().is_none_or(|l| l.visual != v) {
            lines.push(frame.layout(doc, v, tab));
        }
        let (c0, c1, base) = match wrap {
            Some(wc) => (k * wc, (k + 1) * wc, k * wc),
            None => (hc0, hc1, 0),
        };
        rows.push(RowInfo {
            row,
            line: lines.len() - 1,
            k,
            c0,
            c1,
            base,
            y: geo.row_top(row),
        });
        row += 1;
    }
    let visible = |r: &&RowInfo| r.row >= first_row && r.row <= last_row;
    if let (Some(a), Some(b)) = (rows.iter().find(visible), rows.iter().rev().find(visible)) {
        out.visible_lines = lines[a.line].logical..lines[b.line].logical + 1;
    }
    let area = geo.area;
    let x_of = |col: usize, base: usize| geo.x_of(col, base);
    let row_rect = |r: &RowInfo, a: usize, b: usize| {
        Rect::from_min_max(
            pos2(x_of(a, r.base), r.y),
            pos2(x_of(b, r.base), r.y + geo.row_h),
        )
    };
    // The rows a document offset lies on.
    let line_of_visual = |v: usize| -> Option<usize> { lines.iter().position(|l| l.visual == v) };

    // ---- right margin and the caret rows --------------------------------------------
    if opts.right_margin && editor.settings.right_margin > 0 && wrap.is_none() {
        let x = x_of(editor.settings.right_margin, 0).round() - 0.5;
        if x > area.min.x && x < area.max.x {
            painter.line_segment(
                [pos2(x, area.min.y), pos2(x, area.max.y)],
                Stroke::new(1.0_f32, colors.right_margin),
            );
        }
    }
    let mut caret_rows: Vec<usize> = Vec::new();
    let mut caret_cells: Vec<(usize, usize, usize, usize)> = Vec::new();
    for caret in editor.carets.carets() {
        let v = frame.visual_of(editor, caret.offset);
        let Some(li) = line_of_visual(v) else {
            continue;
        };
        let layout = &lines[li];
        let col = layout.col_of(caret.offset) + caret.virtual_spaces;
        let k = wrap.map_or(0, |wc| col / wc);
        let row = frame.map.first_row(v) + k;
        if !caret_rows.contains(&row) {
            caret_rows.push(row);
        }
        caret_cells.push((row, col, li, caret.offset));
    }
    for r in rows.iter().filter(|r| caret_rows.contains(&r.row)) {
        painter.rect_filled(
            Rect::from_min_max(pos2(area.min.x, r.y), pos2(area.max.x, r.y + geo.row_h)),
            0.0,
            colors.caret_row,
        );
    }

    // ---- highlights under the text ---------------------------------------------------
    let radius = CornerRadius::same(metrics::HIGHLIGHT_RADIUS);
    let paint_ranges =
        |ranges: &[Range<usize>], current: Option<usize>, color: Color32, cur: Color32| {
            for r in &rows {
                let layout = &lines[r.line];
                let (lo, hi) = (layout.first_offset, layout.last_offset);
                let from = ranges.partition_point(|m| m.end <= lo);
                for (i, m) in ranges[from..].iter().enumerate() {
                    if m.start > hi {
                        break;
                    }
                    let a = layout.col_of(m.start.max(lo)).max(r.c0);
                    let b = layout.col_of(m.end.min(hi)).min(r.c1);
                    if a < b {
                        let c = if current == Some(from + i) {
                            cur
                        } else {
                            color
                        };
                        painter.rect_filled(row_rect(r, a, b), radius, c);
                    }
                }
            }
        };
    paint_ranges(
        input.occurrences,
        None,
        colors.rename_occurrence,
        colors.rename_occurrence,
    );
    paint_ranges(
        input.find,
        input.find_current,
        colors.search_result,
        colors.search_result_current,
    );
    // Matched braces.
    if let Some(pair) = input.brace
        && pair.partner.is_some()
    {
        for offset in [Some(pair.brace), pair.partner].into_iter().flatten() {
            let v = frame.visual_of(editor, offset);
            if let Some(li) = line_of_visual(v) {
                let col = lines[li].col_of(offset);
                for r in rows
                    .iter()
                    .filter(|r| r.line == li && col >= r.c0 && col < r.c1)
                {
                    painter.rect_filled(row_rect(r, col, col + 1), radius, colors.matched_brace);
                }
            }
        }
    }

    // ---- fold placeholders --------------------------------------------------------------
    for r in &rows {
        let layout = &lines[r.line];
        for seg in layout.segs.iter().filter(|s| s.fold.is_some()) {
            let (a, b) = (seg.col0.max(r.c0), (seg.col0 + seg.cols).min(r.c1));
            if a >= b {
                continue;
            }
            let rect = row_rect(r, a, b);
            painter.rect_filled(rect, 0.0, colors.folded_border);
            painter.rect_filled(
                rect.shrink(metrics::FOLD_BORDER),
                0.0,
                colors.folded_background,
            );
        }
    }

    // ---- selection --------------------------------------------------------------------
    let fill = if opts.focused {
        colors.selection
    } else {
        colors.selection_inactive
    };
    if let Some(block) = editor.carets.block.filter(|_| editor.carets.column_mode) {
        let (l0, l1) = block.lines();
        let (c0, c1) = block.columns();
        if c0 < c1 {
            for r in rows.iter().filter(|r| r.k == 0 || wrap.is_some()) {
                let layout = &lines[r.line];
                if layout.logical >= l0 && layout.logical <= l1 {
                    painter.rect_filled(row_rect(r, c0.max(r.c0), c1.min(r.c1).max(c0)), 0.0, fill);
                }
            }
        }
    } else {
        let sel_radius =
            (crate::tokens::size::CODE_LINE_HEIGHT / metrics::SELECTION_RADIUS_DIVISOR).round();
        for caret in editor.carets.carets().iter().filter(|c| c.has_selection()) {
            let (s, e) = (caret.selection_start(), caret.selection_end());
            let mut run: Vec<(f32, f32)> = Vec::new();
            let mut run_top = 0.0;
            let mut last_row = None::<usize>;
            let flush = |run: &mut Vec<(f32, f32)>, top: f32| {
                if run.is_empty() {
                    return;
                }
                paint_outline(painter, run, top, geo.row_h, sel_radius, fill);
                run.clear();
            };
            for r in &rows {
                let layout = &lines[r.line];
                let (lo, hi) = (layout.first_offset, layout.last_offset);
                if !(e > lo && s <= hi) {
                    continue;
                }
                let start_col = if s <= lo { 0 } else { layout.col_of(s) };
                let (end_col, continues) = if e > hi {
                    (layout.cols, true)
                } else {
                    (layout.col_of(e), false)
                };
                let last_of_line = wrap.is_none_or(|wc| (r.k + 1) * wc >= layout.cols.max(1));
                let tail = continues && last_of_line;
                let a = start_col.max(r.c0);
                let b = end_col.min(r.c1);
                if a >= b && !tail {
                    continue;
                }
                let left = x_of(a.min(b), r.base);
                let mut right = x_of(b.max(a), r.base);
                if tail {
                    right += geo.cell;
                }
                if last_row.is_some_and(|l| l + 1 != r.row) {
                    flush(&mut run, run_top);
                }
                if run.is_empty() {
                    run_top = r.y;
                }
                run.push((left, right));
                last_row = Some(r.row);
            }
            flush(&mut run, run_top);
        }
    }

    // ---- indent guides -------------------------------------------------------------------
    if opts.indent_guides {
        let unit = editor.settings.indent_size.max(1);
        let current = editor
            .carets
            .carets()
            .last()
            .map(|c| selected_guide(doc, doc.line_number(c.offset), tab, unit));
        for r in &rows {
            let layout = &lines[r.line];
            if r.k != 0 && wrap.is_some() {
                continue;
            }
            let indent = effective_indent(doc, layout.logical, tab);
            let mut c = 0;
            while c < indent {
                if c >= r.c0 && c < r.c1 {
                    let x = (x_of(c, r.base)).round() + 0.5;
                    let selected = current
                        .flatten()
                        .is_some_and(|(col, a, b)| col == c && (a..=b).contains(&layout.logical));
                    painter.line_segment(
                        [pos2(x, r.y), pos2(x, r.y + geo.row_h)],
                        Stroke::new(
                            1.0_f32,
                            if selected {
                                colors.indent_guide_selected
                            } else {
                                colors.indent_guide
                            },
                        ),
                    );
                }
                c += unit;
            }
        }
    }

    // ---- text -----------------------------------------------------------------------------
    let mut galleys = 0;
    for r in rows
        .iter()
        .filter(|r| r.row >= first_row && r.row <= last_row)
    {
        let layout = &lines[r.line];
        let from = r.c0;
        let to = r.c1.min(layout.cols);
        if from >= to {
            continue;
        }
        let state_cache = &frame.cache;
        let (text, spans) = layout.display(state_cache, *highlighter, from, to);
        if text.is_empty() {
            continue;
        }
        let job = styled_job(&text, 0, &spans, f32::INFINITY, |kind| {
            token_format(kind, font, colors, Some(geo.row_h), opts.enabled)
        });
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        painter.galley(pos2(x_of(from, r.base), r.y), galley, colors.text);
        galleys += 1;
        // The placeholder text of folded regions.
        for seg in layout.segs.iter().filter(|s| s.fold.is_some()) {
            if seg.col0 + seg.cols <= from || seg.col0 >= to {
                continue;
            }
            let placeholder = &layout.placeholders[seg.fold.unwrap_or(0)];
            painter.text(
                pos2(x_of(seg.col0, r.base), r.y + geo.row_h / 2.0),
                Align2::LEFT_CENTER,
                placeholder,
                (*font).clone(),
                colors.folded_text,
            );
        }
    }
    out.laid_out_rows = galleys;

    // ---- unmatched braces, whitespace, squiggles --------------------------------------
    if let Some(pair) = input.brace
        && pair.partner.is_none()
    {
        let v = frame.visual_of(editor, pair.brace);
        if let Some(li) = line_of_visual(v)
            && let Some(c) = doc.char_at(pair.brace)
        {
            let col = lines[li].col_of(pair.brace);
            for r in rows
                .iter()
                .filter(|r| r.line == li && col >= r.c0 && col < r.c1)
            {
                let rect = row_rect(r, col, col + 1);
                painter.rect_filled(rect, radius, colors.background);
                painter.text(
                    pos2(rect.min.x, r.y + geo.row_h / 2.0),
                    Align2::LEFT_CENTER,
                    c,
                    (*font).clone(),
                    colors.invalid,
                );
            }
        }
    }
    if opts.whitespace {
        for r in rows
            .iter()
            .filter(|r| r.row >= first_row && r.row <= last_row)
        {
            let layout = &lines[r.line];
            let mut tabs = Vec::new();
            layout.for_each_char(|col, c| {
                if col < r.c0 || col >= r.c1 {
                    return;
                }
                if c == ' ' {
                    painter.circle_filled(
                        pos2(x_of(col, r.base) + geo.cell / 2.0, r.y + geo.row_h / 2.0),
                        metrics::WHITESPACE_DOT / 2.0,
                        colors.whitespace,
                    );
                } else if c == '\t' {
                    tabs.push(col);
                }
            });
            for col in tabs {
                let end = advance_column(col, '\t', tab);
                let y = r.y + geo.row_h / 2.0;
                painter.line_segment(
                    [
                        pos2(x_of(col, r.base) + 1.0, y),
                        pos2(x_of(end, r.base) - geo.cell / 4.0, y),
                    ],
                    Stroke::new(1.0_f32, colors.whitespace),
                );
            }
        }
    }
    for h in input.highlights {
        let color = match h.severity {
            HighlightSeverity::Error => colors.error_effect,
            HighlightSeverity::Warning => colors.warning_effect,
            HighlightSeverity::WeakWarning => colors.weak_warning_effect,
            _ => continue,
        };
        let range = h.range.start.min(doc.len())..h.range.end.min(doc.len());
        for r in rows
            .iter()
            .filter(|r| r.row >= first_row && r.row <= last_row)
        {
            let layout = &lines[r.line];
            let (lo, hi) = (layout.first_offset, layout.last_offset);
            if range.end <= lo || range.start > hi {
                continue;
            }
            let a = layout.col_of(range.start.max(lo)).max(r.c0);
            let b = layout.col_of(range.end.min(hi)).min(r.c1);
            if a < b {
                squiggle(
                    painter,
                    x_of(a, r.base),
                    x_of(b, r.base),
                    r.y + geo.row_h - 2.0,
                    color,
                );
            }
        }
    }

    // ---- IME preedit and carets ---------------------------------------------------------
    let caret_h = caret_height(ui, font, geo.row_h);
    let primary = editor.carets.primary().offset;
    for (row, col, li, offset) in &caret_cells {
        let Some(r) = rows.iter().find(|r| r.row == *row) else {
            continue;
        };
        let x = x_of(*col, r.base);
        let top = r.y + (geo.row_h - caret_h) / 2.0;
        let is_primary = *offset == primary;
        let overwrite = editor.carets.overwrite;
        let rect = if overwrite {
            Rect::from_min_size(pos2(x, r.y), vec2(geo.cell, geo.row_h))
        } else {
            Rect::from_min_size(pos2(x, top), vec2(metrics::CARET_WIDTH, caret_h))
        };
        if is_primary && out.caret_rect.is_none() {
            out.caret_rect = Some(rect);
        }
        let _ = li;
        if !(opts.caret_visible && opts.focused && opts.enabled) {
            continue;
        }
        if overwrite {
            painter.rect_filled(rect, 0.0, colors.caret);
            if let Some(c) = doc.char_at(*offset).filter(|c| *c != '\n' && *c != '\t') {
                let job = styled_job(&c.to_string(), 0, &[], f32::INFINITY, |_| {
                    let mut format = token_format(
                        crate::editor::TokenKind::Plain,
                        font,
                        colors,
                        Some(geo.row_h),
                        true,
                    );
                    format.color = colors.background;
                    format
                });
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                painter.galley(pos2(x, r.y), galley, colors.background);
            }
        } else {
            painter.rect_filled(
                rect,
                CornerRadius::same(metrics::CARET_RADIUS),
                colors.caret,
            );
        }
    }
    if let (Some(preedit), Some(rect)) = (input.preedit, out.caret_rect) {
        painter.text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            preedit,
            (*font).clone(),
            colors.text,
        );
        let width = preedit.chars().count() as f32 * geo.cell;
        painter.line_segment(
            [
                pos2(rect.min.x, rect.max.y),
                pos2(rect.min.x + width, rect.max.y),
            ],
            Stroke::new(1.0_f32, colors.text),
        );
    }
    let _ = LogicalPosition::new(0, 0);
    out
}

/// Fills the selection outline of `rows`.
fn paint_outline(
    painter: &Painter,
    rows: &[(f32, f32)],
    top: f32,
    height: f32,
    radius: f32,
    color: Color32,
) {
    let outline = selection_shape::outline(rows, top, height, radius);
    for piece in &outline.rows {
        let r = piece.radii;
        let round = |v: f32| v.round().clamp(0.0, 255.0) as u8;
        painter.rect_filled(
            piece.rect,
            CornerRadius {
                nw: round(r[0]),
                ne: round(r[1]),
                se: round(r[2]),
                sw: round(r[3]),
            },
            color,
        );
    }
    for fillet in &outline.fillets {
        let points = fillet.fan(metrics::SELECTION_ARC_STEPS);
        let mut mesh = Mesh::default();
        for p in &points {
            mesh.colored_vertex(*p, color);
        }
        for i in 1..points.len() - 1 {
            mesh.add_triangle(0, i as u32, i as u32 + 1);
        }
        painter.add(Shape::mesh(mesh));
        painter.add(Shape::line(
            points[1..].to_vec(),
            Stroke::new(1.0_f32, color),
        ));
    }
}

/// A wavy line from `x0` to `x1` at `y` (an error or warning squiggle).
fn squiggle(painter: &Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    let half = metrics::SQUIGGLE_HALF_PERIOD;
    let mut points: Vec<Pos2> = Vec::new();
    let mut x = x0;
    let mut up = false;
    while x < x1 {
        points.push(pos2(
            x,
            y + if up { -metrics::SQUIGGLE_HEIGHT } else { 0.0 },
        ));
        x += half;
        up = !up;
    }
    points.push(pos2(
        x1,
        y + if up { -metrics::SQUIGGLE_HEIGHT } else { 0.0 },
    ));
    painter.add(Shape::line(points, Stroke::new(1.0_f32, color)));
}

/// The indentation of `line` in columns, `None` for a blank line.
pub(crate) fn indent_cols(
    doc: &crate::editor::model::Document,
    line: usize,
    tab: usize,
) -> Option<usize> {
    let text = doc.line_text(line);
    let mut col = 0;
    for c in text.chars() {
        match c {
            ' ' | '\t' => col = advance_column(col, c, tab),
            _ => return Some(col),
        }
    }
    None
}

/// The indentation guides of `line` follow: its own, or for a blank line the smaller of the
/// nearest non-blank lines around it.
pub(crate) fn effective_indent(
    doc: &crate::editor::model::Document,
    line: usize,
    tab: usize,
) -> usize {
    if let Some(indent) = indent_cols(doc, line, tab) {
        return indent;
    }
    let near = |step: isize| -> usize {
        let mut l = line as isize + step;
        for _ in 0..64 {
            if l < 0 || l as usize >= doc.line_count() {
                return 0;
            }
            if let Some(indent) = indent_cols(doc, l as usize, tab) {
                return indent;
            }
            l += step;
        }
        0
    };
    near(-1).min(near(1))
}

/// The guide the caret's block highlights: its column and the lines it spans.
pub(crate) fn selected_guide(
    doc: &crate::editor::model::Document,
    line: usize,
    tab: usize,
    unit: usize,
) -> Option<(usize, usize, usize)> {
    let indent = effective_indent(doc, line, tab);
    // A block header highlights the guide of its body; a line in a body, the body's guide.
    let next = {
        let mut l = line + 1;
        let mut found = None;
        while l < doc.line_count() && l < line + 64 {
            if let Some(i) = indent_cols(doc, l, tab) {
                found = Some(i);
                break;
            }
            l += 1;
        }
        found
    };
    let col = if next.is_some_and(|n| n > indent) && indent_cols(doc, line, tab).is_some() {
        indent
    } else if indent >= unit {
        (indent - 1) / unit * unit
    } else {
        return None;
    };
    let mut first = line;
    while first > 0 && line - first < 1000 && effective_indent(doc, first - 1, tab) > col {
        first -= 1;
    }
    let mut last = line;
    while last + 1 < doc.line_count()
        && last - line < 1000
        && effective_indent(doc, last + 1, tab) > col
    {
        last += 1;
    }
    // The header line itself is not in the body.
    if effective_indent(doc, line, tab) <= col {
        first = line + 1;
    }
    Some((col, first, last))
}

/// The brace at the caret and its partner (spec §8.1): the bracket directly before the caret,
/// else the one directly after. Only with one caret and no selection.
pub(crate) fn match_brace(editor: &Editor, language: &Language) -> Option<BracePair> {
    if editor.carets.caret_count() != 1 || editor.carets.primary().has_selection() {
        return None;
    }
    let doc = &editor.document;
    let offset = editor.carets.primary().offset;
    let is_bracket = |c: char| {
        language
            .brackets
            .iter()
            .any(|&(open, close)| c == open || c == close)
    };
    let before = doc.char_before(offset).filter(|c| is_bracket(*c));
    let (brace, c) = match before {
        Some(c) => (offset - c.len_utf8(), c),
        None => {
            let c = doc.char_at(offset).filter(|c| is_bracket(*c))?;
            (offset, c)
        }
    };
    let (open, close, forward) = language.brackets.iter().find_map(|&(o, cl)| {
        if c == o {
            Some((o, cl, true))
        } else if c == cl {
            Some((o, cl, false))
        } else {
            None
        }
    })?;
    let text = doc.text();
    let mut depth = 0usize;
    let limit = 200_000;
    let partner = if forward {
        text[brace..]
            .char_indices()
            .take(limit)
            .find_map(|(i, ch)| {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(brace + i);
                    }
                }
                None
            })
    } else {
        let mut found = None;
        for (i, ch) in text[..brace + c.len_utf8()]
            .char_indices()
            .rev()
            .take(limit)
        {
            if ch == close {
                depth += 1;
            } else if ch == open {
                depth -= 1;
                if depth == 0 {
                    found = Some(i);
                    break;
                }
            }
        }
        found
    };
    Some(BracePair { brace, partner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::model::{Document, Editor};

    #[test]
    fn indent_helpers() {
        let doc = Document::new("a\n    b\n\n        c\n  d");
        assert_eq!(indent_cols(&doc, 0, 4), Some(0));
        assert_eq!(indent_cols(&doc, 1, 4), Some(4));
        assert_eq!(indent_cols(&doc, 2, 4), None);
        assert_eq!(
            effective_indent(&doc, 2, 4),
            4,
            "a blank line takes the smaller neighbor"
        );
    }

    #[test]
    fn the_selected_guide_is_the_blocks() {
        let doc = Document::new("fn x() {\n    a;\n    b;\n}\nz");
        // Caret on `a;`: the body's guide at column 0, lines 1..=2.
        assert_eq!(selected_guide(&doc, 1, 4, 4), Some((0, 1, 2)));
        // Caret on the header: the same guide.
        assert_eq!(selected_guide(&doc, 0, 4, 4), Some((0, 1, 2)));
        assert_eq!(selected_guide(&doc, 4, 4, 4), None);
    }

    #[test]
    fn braces_match_across_nesting() {
        let mut editor = Editor::new("f(a(b), [c])");
        let language = Language::c_like();
        editor.carets.set_caret(&editor.document, 2); // after the first `(`
        assert_eq!(
            match_brace(&editor, &language),
            Some(BracePair {
                brace: 1,
                partner: Some(11)
            })
        );
        editor.carets.set_caret(&editor.document, 12); // after the last `)`... end
        let pair = match_brace(&editor, &language).expect("a brace");
        assert_eq!((pair.brace, pair.partner), (11, Some(1)));
        editor.carets.set_caret(&editor.document, 0);
        assert_eq!(match_brace(&editor, &language), None);
    }

    #[test]
    fn a_lone_brace_has_no_partner() {
        let mut editor = Editor::new("a (b");
        editor.carets.set_caret(&editor.document, 3);
        let pair = match_brace(&editor, &Language::c_like()).expect("a brace");
        assert_eq!((pair.brace, pair.partner), (2, None));
    }

    #[test]
    fn no_brace_while_text_is_selected() {
        let mut editor = Editor::new("(a)");
        editor.carets.set_selection(&editor.document, 0, 2);
        assert_eq!(match_brace(&editor, &Language::c_like()), None);
    }
}
