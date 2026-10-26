//! Fold regions as data (spec section 6): no painting, no language providers.

use super::caret::CaretModel;
use super::document::{Document, Edit};
use super::position::{VisualPosition, advance_column};

/// A foldable range (IntelliJ `FoldRegion`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldRegion {
    /// Start offset of the folded text.
    pub start: usize,
    /// End offset of the folded text.
    pub end: usize,
    /// What the collapsed region shows instead (`{...}`, `...`).
    pub placeholder: String,
    /// Whether the text is shown. Collapsed regions are `false`.
    pub expanded: bool,
}

impl FoldRegion {
    fn contains_strictly(&self, offset: usize) -> bool {
        self.start < offset && offset < self.end
    }
}

/// All fold regions of an editor (IntelliJ `FoldingModel`), sorted by start offset with
/// outer regions before the regions nested in them.
///
/// Regions are range markers: [`FoldingModel::apply_edit`] keeps them in place, and an edit
/// inside a collapsed region expands it. Typing at a region boundary does not.
#[derive(Clone, Debug, Default)]
pub struct FoldingModel {
    regions: Vec<FoldRegion>,
}

impl FoldingModel {
    /// A model without regions.
    pub fn new() -> Self {
        Self::default()
    }

    /// The regions, outer before inner.
    pub fn regions(&self) -> &[FoldRegion] {
        &self.regions
    }

    /// Adds a region; rejected when empty, out of range or partially overlapping an existing
    /// one (regions nest or are disjoint). Returns the region's index.
    pub fn add_region(
        &mut self,
        doc: &Document,
        start: usize,
        end: usize,
        placeholder: &str,
        expanded: bool,
    ) -> Option<usize> {
        let (start, end) = (doc.snap(start), doc.snap(end));
        if start >= end {
            return None;
        }
        let clash = self.regions.iter().any(|region| {
            let overlap = start < region.end && region.start < end;
            let nested = (start >= region.start && end <= region.end)
                || (region.start >= start && region.end <= end);
            overlap && !nested
        });
        if clash
            || self
                .regions
                .iter()
                .any(|region| region.start == start && region.end == end)
        {
            return None;
        }
        let index = self.regions.partition_point(|region| {
            (region.start, std::cmp::Reverse(region.end)) < (start, std::cmp::Reverse(end))
        });
        self.regions.insert(
            index,
            FoldRegion {
                start,
                end,
                placeholder: placeholder.to_owned(),
                expanded,
            },
        );
        Some(index)
    }

    /// Replaces all regions by `regions`, which must be sorted by start (outer regions before
    /// the regions nested in them) and nest or be disjoint, as a folding provider's output is.
    /// Unlike [`FoldingModel::add_region`] it does not check them against each other, so it
    /// stays linear for thousands of regions. Regions that are empty are dropped.
    pub fn replace_regions(&mut self, mut regions: Vec<FoldRegion>) {
        regions.retain(|region| region.start < region.end);
        self.regions = regions;
    }

    /// Removes all regions.
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// Collapsed regions that are not inside another collapsed region: what is drawn.
    pub fn outer_collapsed(&self) -> Vec<&FoldRegion> {
        let mut result: Vec<&FoldRegion> = Vec::new();
        for region in self.regions.iter().filter(|region| !region.expanded) {
            if result.last().is_some_and(|last| region.end <= last.end) {
                continue;
            }
            result.push(region);
        }
        result
    }

    /// The outermost collapsed region hiding `offset` (strictly inside it).
    pub fn collapsed_region_hiding(&self, offset: usize) -> Option<&FoldRegion> {
        self.outer_collapsed()
            .into_iter()
            .find(|region| region.contains_strictly(offset))
    }

    /// Whether `offset` is hidden by a collapsed region.
    pub fn is_offset_folded(&self, offset: usize) -> bool {
        self.collapsed_region_hiding(offset).is_some()
    }

    /// Sets the expanded state of the region at `index`.
    pub fn set_expanded(&mut self, index: usize, expanded: bool) {
        if let Some(region) = self.regions.get_mut(index) {
            region.expanded = expanded;
        }
    }

    /// Collapses or expands the region at `index` (a click on its gutter chevron).
    pub fn toggle(&mut self, index: usize) {
        if let Some(region) = self.regions.get_mut(index) {
            region.expanded = !region.expanded;
        }
    }

    fn nested_indices(&self, index: usize) -> Vec<usize> {
        let Some(outer) = self.regions.get(index) else {
            return Vec::new();
        };
        self.regions
            .iter()
            .enumerate()
            .filter(|(_, region)| region.start >= outer.start && region.end <= outer.end)
            .map(|(i, _)| i)
            .collect()
    }

    /// Index of the innermost region satisfying `pick` whose range contains `offset`
    /// (boundaries included).
    fn innermost(&self, offset: usize, pick: impl Fn(&FoldRegion) -> bool) -> Option<usize> {
        self.regions
            .iter()
            .enumerate()
            .filter(|(_, region)| region.start <= offset && offset <= region.end && pick(region))
            .min_by_key(|(_, region)| region.end - region.start)
            .map(|(index, _)| index)
    }

    /// `CollapseRegion`: collapses the innermost expanded region containing `offset`.
    /// Returns the region index.
    pub fn collapse_at(&mut self, offset: usize, recursive: bool) -> Option<usize> {
        let index = self.innermost(offset, |region| region.expanded)?;
        let targets = if recursive {
            self.nested_indices(index)
        } else {
            vec![index]
        };
        for target in targets {
            self.regions[target].expanded = false;
        }
        Some(index)
    }

    /// `ExpandRegion`: expands the collapsed region at the caret (the innermost one touching
    /// `offset`), else the nearest enclosing collapsed one. Returns the region index.
    pub fn expand_at(&mut self, offset: usize, recursive: bool) -> Option<usize> {
        let index = self.innermost(offset, |region| !region.expanded)?;
        let targets = if recursive {
            self.nested_indices(index)
        } else {
            vec![index]
        };
        for target in targets {
            self.regions[target].expanded = true;
        }
        Some(index)
    }

    /// `ExpandAllRegions`.
    pub fn expand_all(&mut self) {
        for region in &mut self.regions {
            region.expanded = true;
        }
    }

    /// `CollapseAllRegions`.
    pub fn collapse_all(&mut self) {
        for region in &mut self.regions {
            region.expanded = false;
        }
    }

    /// Expands every collapsed region hiding a caret or selection end, so a caret is never
    /// inside a collapsed region (find results and navigation unfold, spec section 6).
    /// Returns whether anything was expanded.
    pub fn expand_around_carets(&mut self, carets: &CaretModel) -> bool {
        let mut changed = false;
        for caret in carets.carets() {
            for offset in [caret.offset, caret.anchor] {
                for region in &mut self.regions {
                    if !region.expanded && region.contains_strictly(offset) {
                        region.expanded = true;
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// Keeps the regions in place after an edit; an edit touching the inside of a collapsed
    /// region expands it, regions that became empty disappear.
    pub fn apply_edit(&mut self, edit: &Edit) {
        let edit_end = edit.offset + edit.removed.len();
        for region in &mut self.regions {
            let touches_inside = (region.start < edit.offset && edit.offset < region.end)
                || (edit.offset < region.end && edit_end > region.start);
            if touches_inside && !region.expanded {
                region.expanded = true;
            }
            region.start = edit.shift(region.start);
            region.end = edit.shift(region.end);
        }
        self.regions.retain(|region| region.start < region.end);
    }

    /// Hidden logical lines: for each outer collapsed region, the lines after its first line
    /// up to the line of its end.
    fn hidden_ranges(&self, doc: &Document) -> Vec<(usize, usize)> {
        self.outer_collapsed()
            .into_iter()
            .filter_map(|region| {
                let first = doc.line_number(region.start);
                let last = doc.line_number(region.end);
                (last > first).then_some((first + 1, last))
            })
            .collect()
    }

    /// The visual line a logical line is drawn on (hidden lines map to the line of their
    /// region's placeholder).
    pub fn logical_to_visual_line(&self, doc: &Document, line: usize) -> usize {
        let mut hidden_before = 0;
        for (first, last) in self.hidden_ranges(doc) {
            if line > last {
                hidden_before += last - first + 1;
            } else if line >= first {
                hidden_before += line - first + 1;
                break;
            } else {
                break;
            }
        }
        line.saturating_sub(hidden_before)
    }

    /// The first logical line drawn on visual line `visual`.
    pub fn visual_to_logical_line(&self, doc: &Document, visual: usize) -> usize {
        let mut line = visual;
        for (first, last) in self.hidden_ranges(doc) {
            if line >= first {
                line += last - first + 1;
            } else {
                break;
            }
        }
        line.min(doc.line_count() - 1)
    }

    /// Number of visual lines.
    pub fn visual_line_count(&self, doc: &Document) -> usize {
        let hidden: usize = self
            .hidden_ranges(doc)
            .iter()
            .map(|(first, last)| last - first + 1)
            .sum();
        doc.line_count().saturating_sub(hidden).max(1)
    }

    /// The text drawn on visual line `visual`: placeholders replace collapsed regions.
    pub fn visual_line_text(&self, doc: &Document, visual: usize) -> String {
        let logical = self.visual_to_logical_line(doc, visual);
        let mut text = String::new();
        let mut pos = doc.line_start_offset(logical);
        let mut line_end = doc.line_end_offset(logical);
        for region in self.outer_collapsed() {
            if region.end <= pos {
                continue;
            }
            if region.start > line_end {
                break;
            }
            text.push_str(doc.slice(pos, region.start.max(pos)));
            text.push_str(&region.placeholder);
            pos = region.end;
            line_end = doc.line_end_offset(doc.line_number(pos));
        }
        text.push_str(doc.slice(pos, line_end));
        text
    }

    /// The visual position of `offset`; hidden offsets map to their region's placeholder
    /// start. Tabs advance to the next tab stop.
    pub fn visual_position(
        &self,
        doc: &Document,
        offset: usize,
        tab_size: usize,
    ) -> VisualPosition {
        let mut offset = doc.snap(offset);
        if let Some(region) = self.collapsed_region_hiding(offset) {
            offset = region.start;
        }
        let logical = doc.line_number(offset);
        let visual = self.logical_to_visual_line(doc, logical);
        // Walk the visual line from its first logical line up to `offset`.
        let first_logical = self.visual_to_logical_line(doc, visual);
        let mut pos = doc.line_start_offset(first_logical);
        let mut column = 0;
        for region in self.outer_collapsed() {
            if region.end <= pos || region.start >= offset {
                continue;
            }
            for c in doc.slice(pos, region.start).chars() {
                column = advance_column(column, c, tab_size);
            }
            column += region.placeholder.chars().count();
            pos = region.end;
        }
        for c in doc.slice(pos, offset).chars() {
            column = advance_column(column, c, tab_size);
        }
        VisualPosition::new(visual, column)
    }
}
