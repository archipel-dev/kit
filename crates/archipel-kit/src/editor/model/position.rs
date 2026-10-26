//! Logical and visual positions and the column arithmetic behind them.

use super::document::Document;

/// A line and a column (IntelliJ `LogicalPosition`).
///
/// `column` counts characters, except that a tab advances to the next multiple of the tab
/// size, exactly like the logical columns of IntelliJ. Wide (CJK) characters count as one
/// column: the model has no font, the painter decides the cell width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalPosition {
    /// Zero-based line.
    pub line: usize,
    /// Zero-based column.
    pub column: usize,
}

impl LogicalPosition {
    /// A position.
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// A line and a column as drawn (IntelliJ `VisualPosition`): folded regions shrink to their
/// placeholder, so visual lines can be fewer than logical lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisualPosition {
    /// Zero-based visual line.
    pub line: usize,
    /// Zero-based visual column.
    pub column: usize,
}

impl VisualPosition {
    /// A position.
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Column reached after `c` when it starts at `column`.
pub fn advance_column(column: usize, c: char, tab_size: usize) -> usize {
    if c == '\t' {
        let tab = tab_size.max(1);
        (column / tab + 1) * tab
    } else {
        column + 1
    }
}

/// The logical column of `offset` on its line.
pub fn column_of(doc: &Document, offset: usize, tab_size: usize) -> usize {
    let offset = doc.snap(offset);
    let start = doc.line_start_offset(doc.line_number(offset));
    doc.slice(start, offset)
        .chars()
        .fold(0, |column, c| advance_column(column, c, tab_size))
}

/// Width of `line` in logical columns.
pub fn line_width(doc: &Document, line: usize, tab_size: usize) -> usize {
    doc.line_text(line)
        .chars()
        .fold(0, |column, c| advance_column(column, c, tab_size))
}

/// The logical position of `offset` (IntelliJ `offsetToLogicalPosition`).
pub fn offset_to_logical(doc: &Document, offset: usize, tab_size: usize) -> LogicalPosition {
    let offset = doc.snap(offset);
    LogicalPosition::new(doc.line_number(offset), column_of(doc, offset, tab_size))
}

/// The offset of a column on a line plus the columns that lie beyond the line end.
///
/// A column inside a tab snaps to the nearer side of the tab (the caret never stands inside a
/// tab; ties go after it).
pub fn offset_at_column(
    doc: &Document,
    line: usize,
    column: usize,
    tab_size: usize,
) -> (usize, usize) {
    let line = line.min(doc.line_count() - 1);
    let start = doc.line_start_offset(line);
    let mut current = 0;
    let mut offset = start;
    for c in doc.line_text(line).chars() {
        let next = advance_column(current, c, tab_size);
        if next > column {
            // `column` is in [current, next): pick the nearer edge.
            if column - current < next - column {
                return (offset, 0);
            }
            return (offset + c.len_utf8(), 0);
        }
        current = next;
        offset += c.len_utf8();
    }
    (offset, column.saturating_sub(current))
}

/// The offset of a logical position, clamped to the line end (IntelliJ
/// `logicalPositionToOffset` with virtual space off).
pub fn logical_to_offset(doc: &Document, position: LogicalPosition, tab_size: usize) -> usize {
    offset_at_column(doc, position.line, position.column, tab_size).0
}
