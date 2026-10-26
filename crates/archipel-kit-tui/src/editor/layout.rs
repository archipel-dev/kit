//! Where text is in cells: the segments of a visual line (text and fold placeholders), the
//! column arithmetic with tabs and wide characters, and the rows soft wrap makes of lines.
//!
//! The model counts every character as one column (it has no font); the terminal knows better.
//! This module counts **cells**: a tab advances to the next tab stop, a wide character takes
//! two cells, a combining mark none. Mouse positions, scrolling and painting all use cells.

use archipel_kit::editor::model::{Document, FoldRegion, FoldingModel};
use unicode_width::UnicodeWidthChar;

/// What a collapsed fold region shows: its placeholder with `...` as `…` (`{…}`), or ` … `
/// without one.
pub(crate) fn pill_text(placeholder: &str) -> String {
    let text = placeholder.replace("...", "…");
    if text.trim().is_empty() {
        " … ".to_owned()
    } else {
        text
    }
}

/// The cells `c` takes when it starts at cell column `col`.
pub(crate) fn char_cols(c: char, col: usize, tab: usize) -> usize {
    if c == '\t' {
        let tab = tab.max(1);
        (col / tab + 1) * tab - col
    } else if c.is_control() {
        1
    } else {
        c.width().unwrap_or(0)
    }
}

/// The width of `text` in cells when it starts at cell column `col`.
pub(crate) fn text_cols(text: &str, col: usize, tab: usize) -> usize {
    let mut end = col;
    for c in text.chars() {
        end += char_cols(c, end, tab);
    }
    end - col
}

/// One piece of a visual line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Seg {
    /// First document offset.
    pub off0: usize,
    /// One past the last.
    pub off1: usize,
    /// First cell column.
    pub col0: usize,
    /// Width in cells.
    pub cols: usize,
    /// A collapsed fold region (drawn as the placeholder) instead of text.
    pub fold: bool,
    /// The placeholder of a fold, an index into [`LineLayout::pills`].
    pub pill: usize,
}

/// A visual line: the text of a logical line with its collapsed regions replaced by
/// placeholders (the following line's end joins when a region ends on a later line).
#[derive(Clone, Debug)]
pub(crate) struct LineLayout {
    /// The logical line the visual line starts on.
    pub logical: usize,
    /// The offset of its first character.
    pub first_offset: usize,
    /// The end offset of its last text (before the line break).
    pub last_offset: usize,
    pub segs: Vec<Seg>,
    /// The texts of the collapsed regions.
    pub pills: Vec<String>,
    /// Width in cells.
    pub cols: usize,
}

impl LineLayout {
    /// The cell column of `offset`: a hidden offset maps to its placeholder.
    pub fn col_of(&self, doc: &Document, tab: usize, offset: usize) -> usize {
        for seg in &self.segs {
            if seg.fold {
                if offset > seg.off0 && offset < seg.off1 {
                    return seg.col0;
                }
            } else if offset >= seg.off0 && offset <= seg.off1 {
                return seg.col0 + text_cols(doc.slice(seg.off0, offset), seg.col0, tab);
            }
        }
        if offset < self.first_offset {
            0
        } else {
            self.cols
        }
    }

    /// The offset at cell column `col` and the cells beyond the line end. A click in the
    /// first half of a wide character is before it, in the second half after it.
    pub fn offset_at(&self, doc: &Document, tab: usize, col: usize) -> (usize, usize) {
        for seg in &self.segs {
            if col >= seg.col0 + seg.cols {
                continue;
            }
            if seg.fold {
                let half = seg.cols / 2;
                return (
                    if col - seg.col0 < half {
                        seg.off0
                    } else {
                        seg.off1
                    },
                    0,
                );
            }
            let mut at = seg.col0;
            let mut offset = seg.off0;
            for c in doc.slice(seg.off0, seg.off1).chars() {
                let w = char_cols(c, at, tab);
                if col < at + w {
                    let before = col - at < w.div_ceil(2);
                    return (
                        if before {
                            offset
                        } else {
                            offset + c.len_utf8()
                        },
                        0,
                    );
                }
                at += w;
                offset += c.len_utf8();
            }
        }
        (self.last_offset, col.saturating_sub(self.cols))
    }

    /// The placeholder at cell column `col`, if one is there.
    pub fn fold_at(&self, col: usize) -> Option<&Seg> {
        self.segs
            .iter()
            .find(|s| s.fold && col >= s.col0 && col < s.col0 + s.cols)
    }
}

/// What layouts are built from: the document, the collapsed regions and the tab size.
pub(crate) struct Ctx<'a> {
    pub doc: &'a Document,
    pub folding: &'a FoldingModel,
    pub outer: Vec<&'a FoldRegion>,
    pub tab: usize,
}

impl<'a> Ctx<'a> {
    pub fn new(doc: &'a Document, folding: &'a FoldingModel, tab: usize) -> Self {
        Self {
            doc,
            folding,
            outer: folding.outer_collapsed(),
            tab: tab.max(1),
        }
    }

    /// The number of visual lines.
    pub fn visual_lines(&self) -> usize {
        self.folding.visual_line_count(self.doc)
    }

    /// The visual line `offset` is drawn on (an offset hidden in a fold is on its placeholder).
    pub fn visual_of(&self, offset: usize) -> usize {
        let offset = match self.folding.collapsed_region_hiding(offset) {
            Some(region) => region.start,
            None => offset,
        };
        self.folding
            .logical_to_visual_line(self.doc, self.doc.line_number(offset))
    }

    /// The layout of visual line `visual`.
    pub fn layout(&self, visual: usize) -> LineLayout {
        let doc = self.doc;
        let logical = self.folding.visual_to_logical_line(doc, visual);
        let first_offset = doc.line_start_offset(logical);
        let mut pos = first_offset;
        let mut line_end = doc.line_end_offset(logical);
        let mut col = 0;
        let mut segs = Vec::new();
        let mut pills = Vec::new();
        for region in &self.outer {
            if region.end <= pos {
                continue;
            }
            if region.start > line_end {
                break;
            }
            let end = region.start.max(pos);
            let cols = text_cols(doc.slice(pos, end), col, self.tab);
            segs.push(Seg {
                off0: pos,
                off1: end,
                col0: col,
                cols,
                fold: false,
                pill: 0,
            });
            col += cols;
            let pill = pill_text(&region.placeholder);
            let pill_cols = text_cols(&pill, col, self.tab);
            segs.push(Seg {
                off0: region.start,
                off1: region.end,
                col0: col,
                cols: pill_cols,
                fold: true,
                pill: pills.len(),
            });
            pills.push(pill);
            col += pill_cols;
            pos = region.end;
            line_end = doc.line_end_offset(doc.line_number(pos));
        }
        let cols = text_cols(doc.slice(pos, line_end), col, self.tab);
        segs.push(Seg {
            off0: pos,
            off1: line_end,
            col0: col,
            cols,
            fold: false,
            pill: 0,
        });
        LineLayout {
            logical,
            first_offset,
            last_offset: line_end,
            segs,
            pills,
            cols: col + cols,
        }
    }
}

/// Which row is which: logical lines, the visual lines left after folding, and the rows soft
/// wrap makes of them.
#[derive(Clone, Debug, Default)]
pub(crate) struct RowMap {
    wrap: Option<usize>,
    /// The first row of each visual line, and the total at the end (only with wrap).
    firsts: Vec<usize>,
    visual_lines: usize,
}

impl RowMap {
    pub fn new(ctx: &Ctx<'_>, wrap: Option<usize>) -> Self {
        let visual_lines = ctx.visual_lines();
        let Some(wc) = wrap.map(|w| w.max(1)) else {
            return Self {
                wrap: None,
                firsts: Vec::new(),
                visual_lines,
            };
        };
        let mut firsts = Vec::with_capacity(visual_lines + 1);
        let mut rows = 0;
        for v in 0..visual_lines {
            firsts.push(rows);
            rows += ctx.layout(v).cols / wc + 1;
        }
        firsts.push(rows);
        Self {
            wrap: Some(wc),
            firsts,
            visual_lines,
        }
    }

    pub fn wrap(&self) -> Option<usize> {
        self.wrap
    }

    pub fn visual_lines(&self) -> usize {
        self.visual_lines
    }

    pub fn total_rows(&self) -> usize {
        match self.wrap {
            Some(_) => self.firsts.last().copied().unwrap_or(1).max(1),
            None => self.visual_lines.max(1),
        }
    }

    pub fn first_row(&self, visual: usize) -> usize {
        match self.wrap {
            Some(_) => self.firsts[visual.min(self.visual_lines.saturating_sub(1))],
            None => visual,
        }
    }

    /// The visual line a row belongs to and the row's index within it.
    pub fn visual_at_row(&self, row: usize) -> (usize, usize) {
        match self.wrap {
            Some(_) => {
                let row = row.min(self.total_rows() - 1);
                let v = self
                    .firsts
                    .partition_point(|&first| first <= row)
                    .saturating_sub(1)
                    .min(self.visual_lines.saturating_sub(1));
                (v, row - self.firsts[v])
            }
            None => (row.min(self.visual_lines.saturating_sub(1)), 0),
        }
    }
}

/// The widest line in cells (for horizontal scrolling).
pub(crate) fn max_cols(doc: &Document, tab: usize) -> usize {
    (0..doc.line_count())
        .map(|l| text_cols(doc.line_text(l), 0, tab))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_wide_characters_and_marks_count_in_cells() {
        assert_eq!(text_cols("ab", 0, 4), 2);
        assert_eq!(text_cols("\tx", 0, 4), 5);
        assert_eq!(text_cols("a\tx", 0, 4), 5);
        assert_eq!(text_cols("漢字", 0, 4), 4);
        assert_eq!(text_cols("e\u{301}", 0, 4), 1);
    }

    #[test]
    fn offsets_and_columns_map_both_ways() {
        let doc = Document::new("a\t漢b");
        let folding = FoldingModel::new();
        let ctx = Ctx::new(&doc, &folding, 4);
        let layout = ctx.layout(0);
        assert_eq!(layout.cols, 4 + 2 + 1);
        assert_eq!(layout.col_of(&doc, 4, 2), 4);
        assert_eq!(layout.col_of(&doc, 4, 5), 6);
        // The first cell of the wide character is before it, the second after it.
        assert_eq!(layout.offset_at(&doc, 4, 4), (2, 0));
        assert_eq!(layout.offset_at(&doc, 4, 5), (5, 0));
        // A click in the first half of the tab is before it, in the second after it.
        assert_eq!(layout.offset_at(&doc, 4, 1), (1, 0));
        assert_eq!(layout.offset_at(&doc, 4, 3), (2, 0));
        // Beyond the end: virtual columns.
        assert_eq!(layout.offset_at(&doc, 4, 10), (6, 3));
    }

    #[test]
    fn a_collapsed_region_is_one_placeholder() {
        let doc = Document::new("fn f() {\n    a;\n}\nb");
        let mut folding = FoldingModel::new();
        folding.add_region(&doc, 7, 17, "{...}", false);
        let ctx = Ctx::new(&doc, &folding, 4);
        assert_eq!(ctx.visual_lines(), 2);
        let layout = ctx.layout(0);
        assert_eq!(layout.cols, 7 + 3);
        assert_eq!(layout.pills, ["{…}"]);
        assert!(layout.fold_at(8).is_some());
        assert_eq!(layout.col_of(&doc, 4, 12), 7);
        assert_eq!(layout.col_of(&doc, 4, 17), 10);
    }

    #[test]
    fn wrapped_lines_take_several_rows() {
        let doc = Document::new("aaaaaaaaaa\nb\n");
        let folding = FoldingModel::new();
        let ctx = Ctx::new(&doc, &folding, 4);
        let map = RowMap::new(&ctx, Some(4));
        assert_eq!(map.first_row(1), 3);
        assert_eq!(map.visual_at_row(2), (0, 2));
        assert_eq!(map.visual_at_row(3), (1, 0));
        assert_eq!(map.total_rows(), 3 + 1 + 1);
    }
}
