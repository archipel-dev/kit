//! Drawing the editor the way NvChad draws a buffer: the sign column and the number column
//! (the current line bright), the cursor line on `black2`, selections as cell backgrounds,
//! indent guides, diagnostics as undercurl, a thin scrollbar column with marks, the find
//! prompt and the completion menu on top.

use super::hover;
use super::layout::{Ctx, LineLayout, RowMap, char_cols};
use super::mouse::{region_at_line, stripe_y};
use super::{
    EditorOutput, EditorState, Geo, Opts, apply_scroll, clamp_scroll, completion, cursor_shape,
    find, geometry, row_col,
};
use crate::components::{fill, put, text_width};
use crate::glyphs;
use crate::runtime::Runtime;
use crate::{Glyph, Look};
use archipel_kit::Tone;
use archipel_kit::editor::logic::{
    BracePair, bracket_regions, effective_indent, fold_regions, match_brace, occurrence_source,
    occurrences_of, selected_guide,
};
use archipel_kit::editor::model::Editor;
use archipel_kit::editor::{ChangeKind, HighlightSeverity, Span, TextAttributesKey};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashSet;
use std::ops::Range;

/// One cell of a composed row.
#[derive(Clone)]
struct Px {
    ch: String,
    fg: Color,
    bg: Color,
    mods: Modifier,
    underline: Option<Color>,
    /// The second cell of a wide character.
    cont: bool,
}

impl Px {
    fn blank(fg: Color, bg: Color) -> Self {
        Self {
            ch: " ".to_owned(),
            fg,
            bg,
            mods: Modifier::empty(),
            underline: None,
            cont: false,
        }
    }
}

/// What the rows need to know, computed once per frame.
struct Paint<'a> {
    look: &'a Look,
    editor: &'a Editor,
    opts: &'a Opts<'a>,
    geo: &'a Geo,
    focused: bool,
    selection_active: bool,
    selections: Vec<Range<usize>>,
    found: &'a [Range<usize>],
    current: Option<Range<usize>>,
    occurrences: Vec<Range<usize>>,
    brace: Option<BracePair>,
    diagnostics: Vec<(Range<usize>, Color)>,
    guide: Option<(usize, usize, usize)>,
    primary: usize,
}

fn tone_color(look: &Look, tone: Tone) -> Color {
    let p = look.palette;
    match tone {
        Tone::Default | Tone::Strong => p.white,
        Tone::Secondary => p.light_grey,
        Tone::Muted | Tone::Disabled => p.grey,
        Tone::Accent => p.nord_blue,
        Tone::Link => p.blue,
        Tone::Success | Tone::Run => p.green,
        Tone::Warning => p.yellow,
        Tone::Error | Tone::Stop => p.red,
        _ => p.white,
    }
}

fn kind_color(look: &Look, kind: TextAttributesKey) -> Color {
    if kind == TextAttributesKey::Plain {
        return look.palette.white;
    }
    look.color(kind.color(&look.theme.editor))
}

fn any_contains(ranges: &[Range<usize>], offset: usize) -> bool {
    let at = ranges.partition_point(|r| r.end <= offset);
    ranges.get(at).is_some_and(|r| r.start <= offset)
}

impl Paint<'_> {
    /// Composes the cells `c0..c1` of one row of `layout`.
    #[allow(clippy::too_many_arguments)]
    fn compose(
        &self,
        spans_of: &mut dyn FnMut(usize) -> Vec<Span>,
        layout: &LineLayout,
        c0: usize,
        c1: usize,
        row_bg: Color,
        first_of_line: bool,
        last_of_line: bool,
    ) -> Vec<Px> {
        let look = self.look;
        let p = look.palette;
        let ed = &look.theme.editor;
        let doc = &self.editor.document;
        let tab = self.editor.settings.tab_size;
        let width = c1 - c0;
        let mut px = vec![Px::blank(p.white, row_bg); width];
        let sel_bg = if self.focused || self.selection_active {
            p.one_bg3
        } else {
            p.one_bg2
        };
        let occurrence_bg = look.color(ed.rename_occurrence);
        let search_bg = look.color(ed.search_result);
        let current_bg = look.color(ed.caret);
        let current_fg = p.black;
        let brace_bg = look.color(ed.matched_brace);
        let ws_color = look.color(ed.whitespace);
        let mut last_written: Option<usize> = None;
        for seg in &layout.segs {
            if seg.fold {
                let mut col = seg.col0;
                for ch in layout.pills[seg.pill].chars() {
                    let w = char_cols(ch, col, tab).max(1);
                    let start = col;
                    col += w;
                    let col = start;
                    if col >= c0 && col < c1 {
                        let cell = &mut px[col - c0];
                        cell.ch = ch.to_string();
                        cell.fg = p.light_grey;
                        cell.bg = p.one_bg2;
                        cell.mods = Modifier::BOLD;
                    }
                }
                continue;
            }
            if seg.col0 >= c1 {
                break;
            }
            let mut col = seg.col0;
            let mut off = seg.off0;
            let mut line = doc.line_number(seg.off0);
            let mut line_start = doc.line_start_offset(line);
            let mut spans = spans_of(line);
            let mut si = 0;
            for c in doc.slice(seg.off0, seg.off1).chars() {
                let w = char_cols(c, col, tab);
                let o = off;
                off += c.len_utf8();
                let start = col;
                col += w;
                if col <= c0 && w > 0 {
                    continue;
                }
                if start >= c1 {
                    break;
                }
                if o >= doc.line_end_with_break(line) && line + 1 < doc.line_count() {
                    line = doc.line_number(o);
                    line_start = doc.line_start_offset(line);
                    spans = spans_of(line);
                    si = 0;
                }
                let rel = o - line_start;
                while si < spans.len() && spans[si].end <= rel {
                    si += 1;
                }
                let kind = spans
                    .get(si)
                    .filter(|s| s.start <= rel)
                    .map_or(TextAttributesKey::Plain, |s| s.kind);
                let mut fg = if self.opts.enabled {
                    kind_color(look, kind)
                } else {
                    p.grey
                };
                let mut bg = row_bg;
                if any_contains(&self.occurrences, o) {
                    bg = occurrence_bg;
                }
                if any_contains(self.found, o) {
                    bg = search_bg;
                }
                if let Some(pair) = self.brace
                    && pair.partner.is_some()
                    && (o == pair.brace || Some(o) == pair.partner)
                {
                    bg = brace_bg;
                }
                if self.selections.iter().any(|r| r.start <= o && o < r.end) {
                    bg = sel_bg;
                }
                let mut mods = Modifier::empty();
                if self
                    .current
                    .as_ref()
                    .is_some_and(|r| r.start <= o && o < r.end)
                {
                    // The current match is inverted and bold, as Neovim's IncSearch: the
                    // other matches only tint.
                    bg = current_bg;
                    fg = current_fg;
                    mods |= Modifier::BOLD;
                }
                if let Some(pair) = self.brace
                    && pair.partner.is_none()
                    && o == pair.brace
                {
                    fg = look.color(ed.invalid);
                    mods |= Modifier::BOLD;
                }
                let underline = self
                    .diagnostics
                    .iter()
                    .find(|(r, _)| r.start <= o && o < r.end)
                    .map(|(_, color)| *color);
                if w == 0 {
                    if let Some(i) = last_written {
                        px[i].ch.push(c);
                    }
                    continue;
                }
                for n in 0..w {
                    let cell_col = start + n;
                    if cell_col < c0 || cell_col >= c1 {
                        continue;
                    }
                    let idx = cell_col - c0;
                    let mut cell = Px::blank(fg, bg);
                    cell.mods = mods;
                    cell.underline = underline;
                    if underline.is_some() {
                        cell.mods |= Modifier::UNDERLINED;
                    }
                    if c == '\t' {
                        if n == 0 && self.opts.whitespace {
                            cell.ch = "→".to_owned();
                            cell.fg = ws_color;
                        }
                    } else if c == ' ' {
                        if self.opts.whitespace {
                            cell.ch = "·".to_owned();
                            cell.fg = ws_color;
                        }
                    } else if n == 0 {
                        cell.ch = if c.is_control() {
                            "·".to_owned()
                        } else {
                            c.to_string()
                        };
                        // A wide character cut by the edge of the window shows as a blank.
                        if start < c0 || start + w > c1 {
                            cell.ch = " ".to_owned();
                        }
                    } else {
                        cell.ch = String::new();
                        cell.cont = true;
                    }
                    px[idx] = cell;
                    if !px[idx].cont {
                        last_written = Some(idx);
                    }
                }
            }
        }
        // The line break of a selection that continues on the next line.
        if last_of_line
            && self
                .selections
                .iter()
                .any(|r| r.start <= layout.last_offset && layout.last_offset < r.end)
        {
            let col = layout.cols;
            if col >= c0 && col < c1 {
                px[col - c0].bg = sel_bg;
            }
        }
        // The rectangle of a column selection.
        if let Some(block) = self
            .editor
            .carets
            .block
            .filter(|_| self.editor.carets.column_mode)
        {
            let (l0, l1) = block.lines();
            let (b0, b1) = block.columns();
            if layout.logical >= l0 && layout.logical <= l1 && b0 < b1 && first_of_line {
                for col in b0.max(c0)..b1.min(c1) {
                    px[col - c0].bg = sel_bg;
                }
            }
        }
        // Virtual text: the message of the first diagnostic of the line.
        if self.opts.virtual_text && last_of_line {
            self.virtual_text(layout, c0, c1, row_bg, &mut px);
        }
        // Indent guides.
        if self.opts.indent_guides && first_of_line {
            let unit = self.editor.settings.indent_size.max(1);
            let indent = effective_indent(doc, layout.logical, tab);
            let guide = look.palette.line;
            let guide_selected = look.color(ed.indent_guide_selected);
            let mut c = 0;
            while c < indent {
                if c >= c0 && c < c1 && px[c - c0].ch.trim().is_empty()
                    || c >= c0 && c < c1 && px[c - c0].ch == "·"
                {
                    let selected = self
                        .guide
                        .is_some_and(|(col, a, b)| col == c && (a..=b).contains(&layout.logical));
                    let cell = &mut px[c - c0];
                    cell.ch = "│".to_owned();
                    cell.fg = if selected { guide_selected } else { guide };
                }
                c += unit;
            }
        }
        // The right margin tint.
        let margin = self.editor.settings.right_margin;
        if self.opts.right_margin
            && margin > 0
            && self.geo.wrap_cols.is_none()
            && margin >= c0
            && margin < c1
        {
            let cell = &mut px[margin - c0];
            if cell.bg == row_bg {
                cell.bg = if row_bg == p.black {
                    p.one_bg
                } else {
                    p.one_bg2
                };
            }
        }
        px
    }

    fn virtual_text(
        &self,
        layout: &LineLayout,
        c0: usize,
        c1: usize,
        row_bg: Color,
        px: &mut [Px],
    ) {
        let look = self.look;
        let rank = |s: HighlightSeverity| match s {
            HighlightSeverity::Error => 0,
            HighlightSeverity::Warning => 1,
            HighlightSeverity::WeakWarning => 2,
            _ => 3,
        };
        let Some(h) = self
            .opts
            .highlights
            .iter()
            .filter(|h| {
                h.message.is_some()
                    && rank(h.severity) < 3
                    && h.range.start >= layout.first_offset
                    && h.range.start <= layout.last_offset
            })
            .min_by_key(|h| rank(h.severity))
        else {
            return;
        };
        let ed = &look.theme.editor;
        let color = look.color(match h.severity {
            HighlightSeverity::Error => ed.error_effect,
            HighlightSeverity::Warning => ed.warning_effect,
            _ => ed.weak_warning_effect,
        });
        let text = format!("■ {}", h.message.as_deref().unwrap_or(""));
        let mut col = layout.cols + 2;
        for ch in text.chars() {
            let w = char_cols(ch, col, 4);
            if w == 0 {
                continue;
            }
            if col >= c0 && col + w <= c1 {
                let cell = &mut px[col - c0];
                cell.ch = ch.to_string();
                cell.fg = color;
                cell.bg = row_bg;
                cell.mods = Modifier::ITALIC;
                if w == 2 {
                    px[col - c0 + 1].ch = String::new();
                    px[col - c0 + 1].cont = true;
                }
            }
            col += w;
        }
    }
}

fn write_row(buf: &mut Buffer, x: u16, y: u16, px: &[Px]) {
    for (i, cell) in px.iter().enumerate() {
        let Some(target) = buf.cell_mut((x + i as u16, y)) else {
            continue;
        };
        target.set_symbol(&cell.ch);
        target.set_fg(cell.fg);
        target.set_bg(cell.bg);
        target.modifier = cell.mods;
        if let Some(color) = cell.underline {
            target.underline_color = color;
        }
    }
}

/// Draws the editor in `area`.
pub(crate) fn show(
    opts: &Opts<'_>,
    editor: &mut Editor,
    rt: &mut Runtime,
    st: &mut EditorState,
    area: Rect,
    buf: &mut Buffer,
) -> EditorOutput {
    let mut out = EditorOutput::default();
    let look = rt.look.clone();
    let p = look.palette;
    st.area = area;
    let area = area.intersection(buf.area);
    if area.is_empty() {
        return out;
    }
    crate::components::fill_rect(buf, area, Style::default().bg(p.black).fg(p.white));
    st.stick = opts.stick_to_bottom;
    // Bracket folds follow the text.
    if opts.bracket_folding && st.bracket_stamp != Some(editor.document.modification_stamp()) {
        let regions = fold_regions(&bracket_regions(&editor.document), editor.folding.regions());
        editor.folding.replace_regions(regions);
    }
    st.bracket_stamp = Some(editor.document.modification_stamp());
    let geo = geometry(opts, editor, st, area);
    st.geo = Some(geo.clone());
    if geo.text.width == 0 || geo.text.height == 0 {
        find::draw(opts, editor, rt, st, geo.bar, buf);
        return out;
    }
    if opts.enabled {
        rt.widget(opts.id, geo.text);
        if !geo.gutter.is_empty() {
            rt.clickable(opts.gutter_id(), geo.gutter);
        }
        if let Some(stripe) = geo.stripe {
            rt.clickable(opts.stripe_id(), stripe);
        }
    }
    if let Some(line) = opts.scroll_to_line {
        st.center_line = Some(line);
    }
    // Scrolling asked for since the last frame.
    if let Some(line) = st.center_line.take() {
        let map = st.rows(editor, geo.wrap_cols);
        let line = line.min(editor.document.line_count() - 1);
        let v = editor
            .folding
            .logical_to_visual_line(&editor.document, line);
        let row = map.first_row(v);
        st.first_row = row.saturating_sub(geo.rows() / 2);
    }
    if let Some(request) = st.pending_scroll.take() {
        apply_scroll(editor, st, &geo, request);
    }
    clamp_scroll(editor, st, &geo);
    let map = st.rows(editor, geo.wrap_cols);
    if opts.stick_to_bottom {
        // A growing document keeps its last row in view while the user has not scrolled up.
        let total = map.total_rows();
        let last_first = total.saturating_sub(geo.rows());
        if st.stuck && total != st.last_rows {
            st.first_row = last_first;
        }
        st.stuck = st.first_row >= last_first;
        st.last_rows = total;
    }
    let focused = rt.focused(opts.id) && opts.enabled;
    let found = st.find.results(editor);
    let current = if st.find.open {
        find::current_match(editor, &found).map(|i| found.ranges[i].clone())
    } else {
        None
    };
    // Occurrences of the selected text.
    let occurrences = occurrence_source(editor)
        .map(|range| occurrences_of(&editor.document, range))
        .unwrap_or_default();
    let tab = editor.settings.tab_size;
    let diagnostics: Vec<(Range<usize>, Color)> = {
        let ed = &look.theme.editor;
        let len = editor.document.len();
        opts.highlights
            .iter()
            .filter_map(|h| {
                let color = match h.severity {
                    HighlightSeverity::Error => ed.error_effect,
                    HighlightSeverity::Warning => ed.warning_effect,
                    HighlightSeverity::WeakWarning => ed.weak_warning_effect,
                    _ => return None,
                };
                Some((
                    h.range.start.min(len)..h.range.end.min(len),
                    look.color(color),
                ))
            })
            .collect()
    };
    let unit = editor.settings.indent_size.max(1);
    let guide = opts.indent_guides.then(|| {
        let line = editor.document.line_number(editor.carets.primary().offset);
        selected_guide(&editor.document, line, tab, unit)
    });
    let paint = Paint {
        look: &look,
        editor,
        opts,
        geo: &geo,
        focused,
        selection_active: st.find.open && st.find.focus != find::FindFocus::Editor,
        selections: {
            let mut ranges: Vec<Range<usize>> = editor
                .carets
                .carets()
                .iter()
                .filter(|c| c.has_selection())
                .map(|c| c.selection_start()..c.selection_end())
                .collect();
            ranges.sort_by_key(|r| r.start);
            ranges
        },
        found: &found.ranges,
        current,
        occurrences,
        brace: if focused {
            match_brace(editor, opts.language)
        } else {
            None
        },
        diagnostics,
        guide: guide.flatten(),
        primary: editor.carets.primary_index(),
    };
    let ctx = Ctx::new(&editor.document, &editor.folding, tab);
    // Rows that hold a caret.
    let mut caret_rows: HashSet<usize> = HashSet::new();
    for caret in editor.carets.carets() {
        caret_rows.insert(row_col(&ctx, &map, caret.offset, caret.virtual_spaces).0);
    }
    let digits = geo.cols.numbers.map_or(0, |(_, w)| w as usize);
    let mut spans = std::mem::take(&mut st.caches.spans);
    let text = geo.text;
    let total = map.total_rows();
    let secondary: Vec<(usize, usize)> = editor
        .carets
        .carets()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != paint.primary)
        .map(|(_, c)| (c.offset, c.virtual_spaces))
        .collect();
    for y in 0..text.height {
        let row = st.first_row + y as usize;
        let sy = text.y + y;
        if row >= total {
            break;
        }
        let (v, k) = map.visual_at_row(row);
        let layout = ctx.layout(v);
        let (c0, c1) = match map.wrap() {
            Some(wc) => (k * wc, (k + 1) * wc),
            None => (st.first_col, st.first_col + text.width as usize),
        };
        let caret_row = caret_rows.contains(&row);
        let row_bg = if caret_row { p.black2 } else { p.black };
        let last_of_line = map.wrap().is_none_or(|wc| (k + 1) * wc > layout.cols);
        let mut spans_of = |line: usize| spans.spans(editor, opts.highlighter, line).to_vec();
        let mut cells = paint.compose(&mut spans_of, &layout, c0, c1, row_bg, k == 0, last_of_line);
        // Secondary carets: reversed cells.
        for (offset, vs) in &secondary {
            if ctx.visual_of(*offset) != v {
                continue;
            }
            let col = layout.col_of(&editor.document, tab, *offset) + vs;
            if col >= c0 && col < c1 {
                cells[col - c0].mods |= Modifier::REVERSED;
            }
        }
        write_row(buf, text.x, sy, &cells);
        // The gutter row.
        if geo.gutter.width > 0 {
            gutter_row(
                opts,
                &look,
                editor,
                &geo,
                buf,
                sy,
                &layout,
                k == 0,
                caret_row,
                digits,
            );
        }
        // The placeholder of an empty document.
        if row == 0
            && editor.document.is_empty()
            && let Some(placeholder) = opts.placeholder
        {
            put(
                buf,
                text,
                text.x,
                sy,
                placeholder,
                Style::default()
                    .fg(p.grey_fg)
                    .bg(row_bg)
                    .add_modifier(Modifier::ITALIC),
            );
        }
    }
    st.caches.spans = spans;
    // The rows of the text below the document stay on the editor background; the gutter too.
    if geo.gutter.width > 0 {
        let rows = (total.saturating_sub(st.first_row)).min(text.height as usize) as u16;
        for y in rows..text.height {
            fill(
                buf,
                geo.gutter,
                geo.gutter.x,
                text.y + y,
                geo.gutter.width,
                Style::default().bg(p.black).fg(p.grey_fg),
            );
        }
    }
    if let Some(stripe) = geo.stripe {
        stripe_column(
            opts,
            &look,
            editor,
            st,
            &ctx,
            &map,
            &geo,
            stripe,
            &found.ranges,
            buf,
        );
    }
    // The cursor: the primary caret's cell, and the terminal cursor while focused.
    {
        let (row, col) = {
            let c = editor.carets.primary();
            row_col(&ctx, &map, c.offset, c.virtual_spaces)
        };
        if row >= st.first_row && row < st.first_row + text.height as usize {
            // `row_col` gives the column within the row when wrapping.
            let visible = map.wrap().is_some()
                || (col >= st.first_col && col < st.first_col + text.width as usize);
            let col_in_row = if map.wrap().is_some() {
                col
            } else {
                col.saturating_sub(st.first_col)
            };
            if visible {
                let x = text.x + col_in_row as u16;
                let y = text.y + (row - st.first_row) as u16;
                let (x, y) = (x.min(text.right().saturating_sub(1)), y);
                out.caret_cell = Some((x, y));
                if focused && st.find.focus == find::FindFocus::Editor {
                    rt.set_cursor(x, y, cursor_shape(editor));
                }
            }
        }
    }
    // What the caller may want to know about the frame.
    {
        let primary = editor.carets.primary();
        out.selection = primary
            .has_selection()
            .then(|| primary.selection_start()..primary.selection_end());
        out.line_count = editor.document.line_count();
        out.matches = found.ranges.len();
        let doc = &editor.document;
        let last_row = (st.first_row + text.height as usize)
            .min(total)
            .saturating_sub(1);
        let (v0, _) = map.visual_at_row(st.first_row.min(total.saturating_sub(1)));
        let (v1, _) = map.visual_at_row(last_row);
        let first = editor.folding.visual_to_logical_line(doc, v0);
        let last = editor.folding.visual_to_logical_line(doc, v1);
        out.visible_lines = first..(last + 1).min(doc.line_count());
    }
    drop(ctx);
    find::draw(opts, editor, rt, st, geo.bar, buf);
    // The completion menu under the caret.
    if st.menu.is_some() {
        let ctx = Ctx::new(&editor.document, &editor.folding, tab);
        let anchor = st.menu.as_ref().map_or(0, |m| m.anchor);
        let (_, col) = row_col(&ctx, &map, anchor, 0);
        let (crow, _) = {
            let c = editor.carets.primary();
            row_col(&ctx, &map, c.offset, c.virtual_spaces)
        };
        let base = if map.wrap().is_some() {
            0
        } else {
            st.first_col
        };
        if crow >= st.first_row && crow < st.first_row + text.height as usize {
            let ax = text.x + (col.saturating_sub(base) as u16).min(text.width);
            let cy = text.y + (crow - st.first_row) as u16;
            completion::draw(opts, rt, st, ax, cy, area, buf);
        } else {
            st.menu = None;
        }
    }
    // A hint at the bottom right of the text.
    if let Some(hint) = st.hint(rt.now).map(str::to_owned)
        && text.height >= 2
    {
        let room = area.width.saturating_sub(2);
        let hint = crate::components::clip(&hint, room.saturating_sub(2));
        let width = text_width(&hint) + 2;
        if room >= 6 {
            let x = area.right().saturating_sub(width + 1);
            let y = text.bottom() - 1;
            fill(
                buf,
                area,
                x,
                y,
                width,
                Style::default().bg(p.one_bg3).fg(p.white),
            );
            put(
                buf,
                area,
                x + 1,
                y,
                &hint,
                Style::default().fg(p.yellow).bg(p.one_bg3),
            );
        }
    }
    // The float of what the pointer rests on.
    match hover::update(opts, editor, rt, st, &geo, &map, &mut out) {
        Some(float) => hover::draw(&look, st, &float, area, buf),
        None => st.hover.rect = None,
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn gutter_row(
    opts: &Opts<'_>,
    look: &Look,
    editor: &Editor,
    geo: &Geo,
    buf: &mut Buffer,
    y: u16,
    layout: &LineLayout,
    first: bool,
    caret_row: bool,
    digits: usize,
) {
    let p = look.palette;
    let bg = if caret_row { p.black2 } else { p.black };
    let gutter = geo.gutter;
    fill(
        buf,
        gutter,
        gutter.x,
        y,
        gutter.width,
        Style::default().bg(bg).fg(p.grey_fg),
    );
    if !first {
        return;
    }
    let line = layout.logical;
    if let Some(x) = geo.cols.vcs {
        for m in opts.markers {
            let (symbol, color) = match m.kind {
                ChangeKind::Added if m.lines.contains(&line) => ("▎", p.green),
                ChangeKind::Modified if m.lines.contains(&line) => ("▎", p.blue),
                ChangeKind::Deleted if m.lines.start == line => ("▔", p.red),
                _ => continue,
            };
            put(buf, gutter, x, y, symbol, Style::default().fg(color).bg(bg));
        }
    }
    for icon in opts.icons.iter().filter(|i| i.line == line) {
        if let Some(&x) = geo.cols.lanes.get(icon.lane) {
            put(
                buf,
                gutter,
                x,
                y,
                &glyphs::nerd_font(icon.glyph).to_string(),
                Style::default().fg(tone_color(look, icon.tone)).bg(bg),
            );
        }
    }
    if let Some((x, w)) = geo.cols.numbers {
        let number = format!("{:>width$}", line + 1, width = w as usize);
        let style = if caret_row {
            Style::default()
                .fg(p.white)
                .bg(bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.grey_fg).bg(bg)
        };
        put(buf, gutter, x, y, &number, style);
        let _ = digits;
    }
    if let Some(x) = geo.cols.fold
        && let Some(index) = region_at_line(editor, line)
    {
        let region = &editor.folding.regions()[index];
        let (glyph, color) = if region.expanded {
            (Glyph::ChevronDown, p.grey)
        } else {
            (Glyph::ChevronRight, p.light_grey)
        };
        put(
            buf,
            gutter,
            x,
            y,
            &look.glyph(glyph),
            Style::default().fg(color).bg(bg),
        );
    }
}

/// The scrollbar column: the thumb, and a mark for every find match and highlight.
#[allow(clippy::too_many_arguments)]
fn stripe_column(
    opts: &Opts<'_>,
    look: &Look,
    editor: &Editor,
    st: &EditorState,
    ctx: &Ctx<'_>,
    map: &RowMap,
    geo: &Geo,
    stripe: Rect,
    found: &[Range<usize>],
    buf: &mut Buffer,
) {
    let p = look.palette;
    let ed = &look.theme.editor;
    let total = map.total_rows();
    let rows = geo.rows();
    fill(
        buf,
        stripe,
        stripe.x,
        stripe.y,
        1,
        Style::default().bg(p.black),
    );
    if total > rows {
        let track = stripe.height as usize;
        let thumb = (track * rows / total).max(1);
        let at = (st.first_row.min(total - rows) * (track - thumb) / (total - rows).max(1))
            .min(track - thumb);
        for n in at..at + thumb {
            if let Some(c) = buf.cell_mut((stripe.x, stripe.y + n as u16)) {
                c.set_bg(p.one_bg);
            }
        }
    }
    let mut mark = |offset: usize, color: Color| {
        let (row, _) = row_col(ctx, map, offset, 0);
        let y = stripe_y(stripe, row, total);
        if let Some(c) = buf.cell_mut((stripe.x, y)) {
            c.set_symbol("▐");
            c.set_fg(color);
        }
    };
    let search = look.color(ed.search_stripe);
    for m in found.iter().take(2000) {
        mark(m.start, search);
    }
    let len = editor.document.len();
    let rank = |s: HighlightSeverity| match s {
        HighlightSeverity::Error => 0,
        HighlightSeverity::Warning => 1,
        HighlightSeverity::WeakWarning => 2,
        HighlightSeverity::Info => 3,
        _ => 4,
    };
    // Weaker marks first, so errors end up on top.
    let mut order: Vec<_> = opts.highlights.iter().collect();
    order.sort_by_key(|h| std::cmp::Reverse(rank(h.severity)));
    for h in order {
        let color = match h.severity {
            HighlightSeverity::Error => look.color(ed.error_stripe),
            HighlightSeverity::Warning => look.color(ed.warning_stripe),
            HighlightSeverity::WeakWarning => look.color(ed.weak_warning_effect),
            HighlightSeverity::Info => p.blue,
            _ => search,
        };
        mark(h.range.start.min(len), color);
    }
}
