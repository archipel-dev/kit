//! The pure rules of mouse gestures that both editors share: which selection a unit
//! (word, line, lines) makes, whether a press is inside the selection, the drag autoscroll
//! timer, and which fold region a gutter line belongs to. The renderers only turn pointer
//! positions into offsets and call these.

use crate::editor::model::{Editor, word_range_at};

/// Whether `offset` is inside a selection of any caret.
pub fn inside_selection(editor: &Editor, offset: usize) -> bool {
    editor
        .carets
        .carets()
        .iter()
        .any(|c| c.has_selection() && c.selection_start() <= offset && offset < c.selection_end())
}

/// The line at `offset` including its line break, as a start and end offset.
pub fn line_range(editor: &Editor, offset: usize) -> (usize, usize) {
    let doc = &editor.document;
    let line = doc.line_number(offset);
    (doc.line_start_offset(line), doc.line_end_with_break(line))
}

/// The word at `offset`, or the text of its line without the break when there is none.
pub fn word_or_line(editor: &Editor, offset: usize) -> (usize, usize) {
    let doc = &editor.document;
    if let Some(range) = word_range_at(
        doc,
        offset,
        editor.settings.mouse_selection_honors_camel_humps && editor.settings.camel_humps,
    ) {
        return range;
    }
    // No word: the text of the line, without its break.
    let line = doc.line_number(offset);
    (doc.line_start_offset(line), doc.line_end_offset(line))
}

/// Sets the primary selection to `anchor..lead` keeping the other carets when `multi`.
pub fn set_selection(editor: &mut Editor, anchor: usize, lead: usize, multi: bool) {
    if multi {
        let doc = &editor.document;
        let (anchor, lead) = (doc.snap(anchor), doc.snap(lead));
        let caret = editor.carets.primary_mut();
        caret.select(anchor, lead);
        editor.carets.normalize();
    } else {
        editor.carets.set_selection(&editor.document, anchor, lead);
    }
}

/// Selects whole lines from `anchor_line` to `line` (a drag over the line numbers).
pub fn select_lines(editor: &mut Editor, anchor_line: usize, line: usize) {
    let doc = &editor.document;
    let anchor_line = anchor_line.min(doc.line_count() - 1);
    let (anchor, lead) = if line >= anchor_line {
        (
            doc.line_start_offset(anchor_line),
            doc.line_end_with_break(line),
        )
    } else {
        (
            doc.line_end_with_break(anchor_line),
            doc.line_start_offset(line),
        )
    };
    set_selection(editor, anchor, lead, false);
}

/// The fold region drawn at a line, as an index into the model's regions.
pub fn region_at_line(editor: &Editor, line: usize) -> Option<usize> {
    let doc = &editor.document;
    let start = doc.line_start_offset(line);
    let end = doc.line_end_offset(line);
    let regions = editor.folding.regions();
    let first = regions.partition_point(|r| r.start < start);
    let mut found = None;
    for (i, region) in regions.iter().enumerate().skip(first) {
        if region.start > end {
            break;
        }
        if editor.folding.is_offset_folded(region.start) {
            continue;
        }
        if !region.expanded {
            return Some(i);
        }
        found.get_or_insert(i);
    }
    found
}

/// One tick of the drag autoscroll timer (spec 2.3) with the pointer `distance` beyond the
/// edge of the text, in the same unit as `unit`: a step happens every `floor(unit / d) + 1`
/// ticks and moves `1 + floor(d / unit)` lines. `counter` counts the ticks since the last
/// step.
pub fn autoscroll_tick(counter: &mut u32, distance: f32, unit: f32) -> Option<usize> {
    let d = distance.abs();
    if d <= 0.0 {
        *counter = 0;
        return None;
    }
    *counter += 1;
    let every = (unit / d).floor() as u32 + 1;
    if *counter >= every {
        *counter = 0;
        Some(1 + (d / unit).floor() as usize)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_inside_a_selection_is_detected() {
        let mut editor = Editor::new("hello world");
        editor.carets.set_selection(&editor.document, 0, 5);
        assert!(inside_selection(&editor, 0));
        assert!(inside_selection(&editor, 4));
        assert!(!inside_selection(&editor, 5));
    }

    #[test]
    fn a_unit_is_a_word_or_the_line_without_its_break() {
        let editor = Editor::new("foo bar\n  \nbaz");
        assert_eq!(word_or_line(&editor, 5), (4, 7));
        assert_eq!(line_range(&editor, 5), (0, 8));
        // On the blank line there is no word: the line's text.
        assert_eq!(word_or_line(&editor, 9), (8, 10));
    }

    #[test]
    fn dragging_over_line_numbers_selects_whole_lines_both_ways() {
        let mut editor = Editor::new("a\nb\nc\n");
        select_lines(&mut editor, 0, 1);
        let c = editor.carets.primary();
        assert_eq!((c.selection_start(), c.selection_end()), (0, 4));
        select_lines(&mut editor, 2, 1);
        let c = editor.carets.primary();
        assert_eq!((c.selection_start(), c.selection_end()), (2, 6));
    }

    #[test]
    fn the_collapsed_region_wins_a_line() {
        let mut editor = Editor::new("{\n  a\n}\n{\n  b\n}");
        editor
            .folding
            .replace_regions(crate::editor::logic::fold_regions(
                &crate::editor::logic::bracket_regions(&editor.document),
                &[],
            ));
        assert_eq!(region_at_line(&editor, 0), Some(0));
        assert_eq!(region_at_line(&editor, 3), Some(1));
        assert_eq!(region_at_line(&editor, 1), None);
    }

    #[test]
    fn autoscroll_steps_faster_the_farther_the_pointer_is() {
        let mut counter = 0;
        let ticks: Vec<_> = (0..6)
            .map(|_| autoscroll_tick(&mut counter, 10.0, 20.0))
            .collect();
        assert_eq!(ticks, [None, None, Some(1), None, None, Some(1)]);
        let mut counter = 0;
        assert_eq!(autoscroll_tick(&mut counter, 40.0, 20.0), Some(3));
        assert_eq!(autoscroll_tick(&mut counter, 0.0, 20.0), None);
    }
}
