//! Which text row is where: logical lines, the visual lines left after collapsed fold regions
//! hide some, and the rows those wrap into.

use crate::editor::model::{Document, FoldRegion};

/// The mapping between logical lines, visual lines and rows for one frame.
pub(crate) struct RowMap {
    /// Hidden logical line ranges `(first, last)`, sorted and disjoint.
    hidden: Vec<(usize, usize)>,
    /// Lines hidden by the ranges before each range.
    hidden_before: Vec<usize>,
    /// Lines hidden by each range and the ones before it.
    hidden_cum: Vec<usize>,
    /// The visual line of each range's placeholder (the line before its first hidden line).
    starts: Vec<usize>,
    lines: usize,
    visual_lines: usize,
    /// Wrap width in columns and the first row of every visual line (plus the total).
    wrap: Option<(usize, Vec<usize>)>,
}

impl RowMap {
    /// The map of `doc` with the collapsed regions `outer` (outermost only, sorted). With
    /// `wrap`, long visual lines break into rows of that many columns; `widths` are the
    /// column widths of the logical lines.
    pub fn new(doc: &Document, outer: &[FoldRegion], wrap: Option<(usize, &[u32])>) -> RowMap {
        let lines = doc.line_count();
        let mut hidden = Vec::new();
        let mut hidden_before = Vec::new();
        let mut hidden_cum = Vec::new();
        let mut starts = Vec::new();
        let mut total_hidden = 0;
        for region in outer {
            let first = doc.line_number(region.start);
            let last = doc.line_number(region.end);
            if last > first {
                if let Some(&(_, previous_last)) = hidden.last()
                    && first < previous_last
                {
                    continue;
                }
                hidden.push((first + 1, last));
                hidden_before.push(total_hidden);
                starts.push(first - total_hidden);
                total_hidden += last - first;
                hidden_cum.push(total_hidden);
            }
        }
        let visual_lines = lines.saturating_sub(total_hidden).max(1);
        let mut map = RowMap {
            hidden,
            hidden_before,
            hidden_cum,
            starts,
            lines,
            visual_lines,
            wrap: None,
        };
        if let Some((cols, widths)) = wrap {
            let cols = cols.max(1);
            let mut prefix = Vec::with_capacity(visual_lines + 1);
            let mut rows = 0;
            for v in 0..visual_lines {
                prefix.push(rows);
                let logical = map.visual_to_logical(v);
                let width = widths.get(logical).copied().unwrap_or(0) as usize;
                rows += width.div_ceil(cols).max(1);
            }
            prefix.push(rows);
            map.wrap = Some((cols, prefix));
        }
        map
    }

    /// The wrap width in columns, when wrapping.
    pub fn wrap_cols(&self) -> Option<usize> {
        self.wrap.as_ref().map(|(cols, _)| *cols)
    }

    /// Number of visual lines.
    pub fn visual_lines(&self) -> usize {
        self.visual_lines
    }

    /// Number of rows.
    pub fn total_rows(&self) -> usize {
        match &self.wrap {
            Some((_, prefix)) => prefix.last().copied().unwrap_or(0),
            None => self.visual_lines,
        }
    }

    /// The visual line a logical line is drawn on (hidden lines map to their region's first
    /// line).
    pub fn logical_to_visual(&self, line: usize) -> usize {
        let k = self.hidden.partition_point(|&(first, _)| first <= line);
        if k == 0 {
            return line;
        }
        let (first, last) = self.hidden[k - 1];
        let before = self.hidden_before[k - 1];
        if line <= last {
            first - 1 - before
        } else {
            line - before - (last - first + 1)
        }
    }

    /// The first logical line drawn on visual line `visual`.
    pub fn visual_to_logical(&self, visual: usize) -> usize {
        // Ranges whose placeholder line is above `visual` have all their lines before it.
        let k = self.starts.partition_point(|&start| start < visual);
        let hidden = if k == 0 { 0 } else { self.hidden_cum[k - 1] };
        (visual + hidden).min(self.lines - 1)
    }

    /// The first row of visual line `visual`.
    pub fn first_row(&self, visual: usize) -> usize {
        match &self.wrap {
            Some((_, prefix)) => prefix[visual.min(prefix.len() - 1)],
            None => visual,
        }
    }

    /// The number of rows of visual line `visual`.
    #[cfg(test)]
    pub fn rows_of(&self, visual: usize) -> usize {
        match &self.wrap {
            Some((_, prefix)) if visual + 1 < prefix.len() => prefix[visual + 1] - prefix[visual],
            Some(_) => 1,
            None => 1,
        }
    }

    /// The visual line shown on `row` and the row's index within it.
    pub fn visual_at_row(&self, row: usize) -> (usize, usize) {
        match &self.wrap {
            Some((_, prefix)) => {
                let v = prefix
                    .partition_point(|&first| first <= row)
                    .saturating_sub(1)
                    .min(self.visual_lines - 1);
                (v, row.saturating_sub(prefix[v]))
            }
            None => (row.min(self.visual_lines - 1), 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(doc: &Document, first_line: usize, last_line: usize) -> FoldRegion {
        FoldRegion {
            start: doc.line_start_offset(first_line),
            end: doc.line_end_offset(last_line),
            placeholder: "...".into(),
            expanded: false,
        }
    }

    #[test]
    fn collapsed_regions_hide_their_lines() {
        let doc = Document::new("a\nb\nc\nd\ne\nf\ng\nh");
        let r1 = region(&doc, 1, 3);
        let r2 = region(&doc, 5, 6);
        let map = RowMap::new(&doc, &[r1, r2], None);
        // Lines 0, 1, (2 3 hidden), 4, 5, (6 hidden), 7.
        assert_eq!(map.visual_lines(), 5);
        let shown: Vec<usize> = (0..5).map(|v| map.visual_to_logical(v)).collect();
        assert_eq!(shown, [0, 1, 4, 5, 7]);
        for (v, logical) in shown.iter().enumerate() {
            assert_eq!(map.logical_to_visual(*logical), v);
        }
        assert_eq!(
            map.logical_to_visual(2),
            1,
            "hidden lines map to the placeholder line"
        );
        assert_eq!(map.logical_to_visual(6), 3);
    }

    #[test]
    fn without_regions_it_is_the_identity() {
        let doc = Document::new("a\nb\nc");
        let map = RowMap::new(&doc, &[], None);
        assert_eq!(map.visual_lines(), 3);
        assert_eq!(map.visual_to_logical(2), 2);
        assert_eq!(map.total_rows(), 3);
    }

    #[test]
    fn long_lines_wrap_into_rows() {
        let doc = Document::new("abcdefghij\nxy\n");
        let widths = [10u32, 2, 0];
        let map = RowMap::new(&doc, &[], Some((4, &widths)));
        assert_eq!(map.rows_of(0), 3);
        assert_eq!(map.rows_of(1), 1);
        assert_eq!(map.total_rows(), 5);
        assert_eq!(map.visual_at_row(2), (0, 2));
        assert_eq!(map.visual_at_row(3), (1, 0));
        assert_eq!(map.first_row(2), 4);
    }
}
