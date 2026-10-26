//! One visual line as a monospace grid of cells: its pieces of text and collapsed fold
//! placeholders, the mapping between document offsets and columns, and the text to draw.

use super::cache::DocCache;
use super::rows::RowMap;
use crate::editor::highlight::{Span, SyntaxHighlighter};
use crate::editor::model::{Document, FoldRegion, advance_column};

/// A piece of a visual line.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seg {
    /// Document offsets it covers.
    pub off0: usize,
    pub off1: usize,
    /// First column and width in columns.
    pub col0: usize,
    pub cols: usize,
    /// A collapsed fold region (drawn as its placeholder) instead of text.
    pub fold: Option<usize>,
}

/// A visual line.
pub(crate) struct LineLayout<'a> {
    /// The visual line and the logical line it starts with.
    pub visual: usize,
    pub logical: usize,
    /// Offset of the first character, and where the last logical line ends (the line break).
    pub first_offset: usize,
    pub last_offset: usize,
    pub segs: Vec<Seg>,
    /// Width in columns, placeholders included.
    pub cols: usize,
    /// One text piece, ASCII without tabs: column and byte index coincide.
    pub simple: bool,
    pub doc: &'a Document,
    pub tab: usize,
    /// The placeholders of the fold segments, by `Seg::fold`.
    pub placeholders: Vec<String>,
}

impl<'a> LineLayout<'a> {
    /// The layout of visual line `visual`. `outer` are the collapsed regions that are not
    /// inside another collapsed region, sorted by start.
    pub fn build(
        doc: &'a Document,
        outer: &[FoldRegion],
        map: &RowMap,
        visual: usize,
        tab: usize,
    ) -> LineLayout<'a> {
        let tab = tab.max(1);
        let logical = map.visual_to_logical(visual);
        let first_offset = doc.line_start_offset(logical);
        let mut pos = first_offset;
        let mut line_end = doc.line_end_offset(logical);
        let mut segs: Vec<Seg> = Vec::new();
        let mut placeholders = Vec::new();
        let mut col = 0;
        let first_region = outer.partition_point(|r| r.end <= pos);
        for region in &outer[first_region..] {
            if region.start > line_end {
                break;
            }
            let start = region.start.max(pos);
            if start > pos {
                push_text(doc, &mut segs, &mut col, pos, start, tab);
            }
            let cols = region.placeholder.chars().count();
            segs.push(Seg {
                off0: start,
                off1: region.end,
                col0: col,
                cols,
                fold: Some(placeholders.len()),
            });
            placeholders.push(region.placeholder.clone());
            col += cols;
            pos = region.end;
            line_end = doc.line_end_offset(doc.line_number(pos));
        }
        push_text(doc, &mut segs, &mut col, pos, line_end, tab);
        let simple = segs.len() == 1 && segs[0].fold.is_none() && {
            let text = doc.slice(segs[0].off0, segs[0].off1);
            text.is_ascii() && !text.as_bytes().contains(&b'\t')
        };
        LineLayout {
            visual,
            logical,
            first_offset,
            last_offset: line_end,
            segs,
            cols: col,
            simple,
            doc,
            tab,
            placeholders,
        }
    }

    /// The column of `offset` (offsets inside a placeholder map to its start).
    pub fn col_of(&self, offset: usize) -> usize {
        let mut last_end = 0;
        for seg in &self.segs {
            last_end = seg.col0 + seg.cols;
            if offset > seg.off1 {
                continue;
            }
            if seg.fold.is_some() {
                return if offset >= seg.off1 {
                    seg.col0 + seg.cols
                } else {
                    seg.col0
                };
            }
            if offset <= seg.off0 {
                return seg.col0;
            }
            if self.simple {
                return seg.col0 + (offset - seg.off0);
            }
            let mut col = seg.col0;
            for c in self.doc.slice(seg.off0, offset).chars() {
                col = advance_column(col, c, self.tab);
            }
            return col;
        }
        last_end
    }

    /// The offset at the boundary of column `col` (nearer side) and the columns beyond the
    /// end of the line.
    pub fn offset_at(&self, col: usize) -> (usize, usize) {
        for (i, seg) in self.segs.iter().enumerate() {
            let end = seg.col0 + seg.cols;
            let last = i + 1 == self.segs.len();
            if col >= end && !last {
                continue;
            }
            if col >= end {
                return (seg.off1, col - end);
            }
            if seg.fold.is_some() {
                return if col - seg.col0 < seg.cols.div_ceil(2) {
                    (seg.off0, 0)
                } else {
                    (seg.off1, 0)
                };
            }
            if self.simple {
                return (seg.off0 + (col - seg.col0), 0);
            }
            let mut current = seg.col0;
            let mut offset = seg.off0;
            for c in self.doc.slice(seg.off0, seg.off1).chars() {
                let next = advance_column(current, c, self.tab);
                if next > col {
                    return if col - current < next - col {
                        (offset, 0)
                    } else {
                        (offset + c.len_utf8(), 0)
                    };
                }
                current = next;
                offset += c.len_utf8();
            }
            return (offset, 0);
        }
        (self.last_offset, 0)
    }

    /// The fold segment at column `col`, if the column is inside a placeholder.
    pub fn fold_at(&self, col: usize) -> Option<&Seg> {
        self.segs
            .iter()
            .find(|s| s.fold.is_some() && col >= s.col0 && col < s.col0 + s.cols)
    }

    /// The text to draw for columns `from..to`, tabs expanded and placeholders blanked, and
    /// the highlighter spans in its byte positions. `from` is the column of the first
    /// returned char; spans are relative to the returned text.
    pub fn display(
        &self,
        cache: &DocCache,
        highlighter: &dyn SyntaxHighlighter,
        from: usize,
        to: usize,
    ) -> (String, Vec<Span>) {
        let mut text = String::new();
        let mut spans: Vec<Span> = Vec::new();
        let mut scratch: Vec<Span> = Vec::new();
        for seg in &self.segs {
            if seg.col0 + seg.cols <= from || seg.col0 >= to {
                continue;
            }
            if seg.fold.is_some() {
                let a = seg.col0.max(from);
                let b = (seg.col0 + seg.cols).min(to);
                text.extend(std::iter::repeat_n(' ', b - a));
                continue;
            }
            // Spans of the logical line the piece belongs to.
            let line = self.doc.line_number(seg.off0);
            let line_start = self.doc.line_start_offset(line);
            let mut state = cache.state_at(self.doc, highlighter, line);
            scratch.clear();
            let long = cache.spans(self.doc, highlighter, line, &mut state, &mut scratch);
            let line_spans: &[Span] = long.as_deref().map_or(&scratch, |v| v);
            if self.simple {
                // Column == byte: slice the window directly.
                let rel0 = seg.off0 - line_start;
                let a = from.max(seg.col0) - seg.col0;
                let b = to.min(seg.col0 + seg.cols) - seg.col0;
                let text_slice = self.doc.slice(seg.off0 + a, seg.off0 + b);
                let base = text.len();
                text.push_str(text_slice);
                for s in line_spans {
                    let (s0, s1) = (s.start.max(rel0 + a), s.end.min(rel0 + b));
                    if s0 < s1 {
                        spans.push(Span {
                            start: base + s0 - rel0 - a,
                            end: base + s1 - rel0 - a,
                            kind: s.kind,
                        });
                    }
                }
                continue;
            }
            // General: expand tabs char by char, keeping a byte map for the spans.
            let seg_text = self.doc.slice(seg.off0, seg.off1);
            let rel0 = seg.off0 - line_start;
            let mut map = vec![0usize; seg_text.len() + 1];
            let mut col = seg.col0;
            let base = text.len();
            let mut out = String::new();
            for (byte, c) in seg_text.char_indices() {
                map[byte] = out.len();
                let next = advance_column(col, c, self.tab);
                for k in col..next {
                    if k >= from && k < to {
                        if c == '\t' {
                            out.push(' ');
                        } else {
                            out.push(c);
                        }
                    }
                }
                col = next;
            }
            map[seg_text.len()] = out.len();
            for byte in (0..seg_text.len()).rev() {
                if !seg_text.is_char_boundary(byte) {
                    map[byte] = map[byte + 1];
                }
            }
            text.push_str(&out);
            for s in line_spans {
                let (s0, s1) = (s.start.max(rel0), s.end.min(rel0 + seg_text.len()));
                if s0 < s1 {
                    spans.push(Span {
                        start: base + map[s0 - rel0],
                        end: base + map[s1 - rel0],
                        kind: s.kind,
                    });
                }
            }
        }
        spans.retain(|s| s.start < s.end);
        (text, spans)
    }

    /// Visits every character of the visual line's text pieces with its first column.
    pub fn for_each_char(&self, mut f: impl FnMut(usize, char)) {
        for seg in &self.segs {
            if seg.fold.is_some() {
                continue;
            }
            let mut col = seg.col0;
            for c in self.doc.slice(seg.off0, seg.off1).chars() {
                f(col, c);
                col = advance_column(col, c, self.tab);
            }
        }
    }

    /// The text of the visual line as the document has it, first column `from`: used by tests.
    #[cfg(test)]
    pub fn plain(&self) -> std::borrow::Cow<'_, str> {
        let mut s = String::new();
        for seg in &self.segs {
            if seg.fold.is_some() {
                s.push_str(&self.placeholders[seg.fold.unwrap_or(0)]);
            } else {
                s.push_str(self.doc.slice(seg.off0, seg.off1));
            }
        }
        std::borrow::Cow::Owned(s)
    }
}

fn push_text(
    doc: &Document,
    segs: &mut Vec<Seg>,
    col: &mut usize,
    off0: usize,
    off1: usize,
    tab: usize,
) {
    let text = doc.slice(off0, off1);
    let cols = if text.is_ascii() && !text.as_bytes().contains(&b'\t') {
        text.len()
    } else {
        let mut c = *col;
        for ch in text.chars() {
            c = advance_column(c, ch, tab);
        }
        c - *col
    };
    segs.push(Seg {
        off0,
        off1,
        col0: *col,
        cols,
        fold: None,
    });
    *col += cols;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::model::FoldingModel;

    fn layout_of<'a>(doc: &'a Document, folding: &FoldingModel, v: usize) -> LineLayout<'a> {
        let outer: Vec<FoldRegion> = folding.outer_collapsed().into_iter().cloned().collect();
        let map = RowMap::new(doc, &outer, None);
        LineLayout::build(doc, &outer, &map, v, 4)
    }

    #[test]
    fn plain_ascii_lines_are_simple() {
        let doc = Document::new("hello\nworld");
        let l = layout_of(&doc, &FoldingModel::new(), 1);
        assert!(l.simple);
        assert_eq!(l.cols, 5);
        assert_eq!(l.col_of(doc.line_start_offset(1) + 3), 3);
        assert_eq!(l.offset_at(3), (doc.line_start_offset(1) + 3, 0));
        assert_eq!(
            l.offset_at(9),
            (doc.line_end_offset(1), 4),
            "beyond the end"
        );
    }

    #[test]
    fn tabs_advance_to_the_next_stop_and_clicks_snap_to_the_nearer_side() {
        let doc = Document::new("a\tb");
        let l = layout_of(&doc, &FoldingModel::new(), 0);
        assert!(!l.simple);
        assert_eq!(l.cols, 5);
        assert_eq!(l.col_of(2), 4);
        assert_eq!(l.offset_at(2), (1, 0), "column 2 is nearer the tab's start");
        assert_eq!(l.offset_at(3), (2, 0), "column 3 is nearer its end");
        let (text, _) = l.display(
            &DocCache::get(
                &egui::Context::default(),
                egui::Id::new("t"),
                &doc,
                &crate::editor::PlainText,
                4,
            ),
            &crate::editor::PlainText,
            0,
            100,
        );
        assert_eq!(text, "a   b");
    }

    #[test]
    fn a_collapsed_region_is_one_placeholder() {
        let mut doc_folding = FoldingModel::new();
        let doc = Document::new("{\n  a\n}\nx");
        doc_folding.add_region(&doc, 0, 7, "{...}", false);
        let l = layout_of(&doc, &doc_folding, 0);
        assert_eq!(l.plain(), "{...}");
        assert_eq!(l.cols, 5);
        assert_eq!(l.col_of(0), 0);
        assert_eq!(l.col_of(3), 0, "inside the region");
        assert_eq!(l.col_of(7), 5);
        assert!(l.fold_at(2).is_some());
        assert_eq!(l.offset_at(1), (0, 0));
        assert_eq!(l.offset_at(4), (7, 0));
        let l = layout_of(&doc, &doc_folding, 1);
        assert_eq!(l.plain(), "x");
    }
}
