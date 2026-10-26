//! Hover: where the pointer rests, what the text under it carries, and which mark of the
//! error stripe it is on. The GUI measures positions in pixels and the terminal in cells; the
//! rules (rest tolerance, delays, which highlights apply, how severe is first) are the same.

use crate::editor::decorations::{HighlightSeverity, RangeHighlight};
use std::ops::Range;

/// Where the pointer has been resting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rest {
    /// The text offset under the pointer.
    pub offset: usize,
    /// The pointer position (pixels or cells).
    pub pos: (f32, f32),
    /// Input time the rest began.
    pub since: f64,
}

impl Rest {
    /// Updates `rest` for a pointer at `pos` over `offset` at `now` and returns how long it
    /// has rested and whether this call restarted the rest (the pointer moved to another
    /// offset or farther than `tolerance`).
    pub fn observe(
        rest: &mut Option<Rest>,
        offset: usize,
        pos: (f32, f32),
        now: f64,
        tolerance: f32,
    ) -> (f64, bool) {
        let still = rest.is_some_and(|r| {
            r.offset == offset
                && ((r.pos.0 - pos.0).powi(2) + (r.pos.1 - pos.1).powi(2)).sqrt() < tolerance
        });
        if !still {
            *rest = Some(Rest {
                offset,
                pos,
                since: now,
            });
        }
        (rest.map_or(0.0, |r| (now - r.since).max(0.0)), !still)
    }
}

/// How long until a rest of `waited` seconds reaches `delay`: when to ask for another frame.
pub fn remaining(delay: f32, waited: f64) -> f64 {
    (f64::from(delay) - waited + 0.001).max(0.001)
}

/// Tracks the pointer resting on a fold placeholder (it previews the folded text after a
/// delay): the placeholder's start offset and since when.
pub fn fold_dwell(rest: &mut Option<(usize, f64)>, placeholder: usize, now: f64) -> f64 {
    let since = match *rest {
        Some((start, since)) if start == placeholder => since,
        _ => {
            *rest = Some((placeholder, now));
            now
        }
    };
    (now - since).max(0.0)
}

/// What the pointer rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverTarget {
    /// The byte offset under the pointer.
    pub offset: usize,
    /// The index (in the highlights passed in) of the first highlight containing `offset`.
    pub highlight: Option<usize>,
}

/// The index of the first highlight whose range contains `offset`.
pub fn highlight_at(highlights: &[RangeHighlight], offset: usize) -> Option<usize> {
    highlights.iter().position(|h| h.range.contains(&offset))
}

impl HighlightSeverity {
    /// How serious it is: 0 for an error, up to 4 for a text search result. Lower is more
    /// severe and wins where marks overlap.
    pub fn rank(self) -> u8 {
        match self {
            HighlightSeverity::Error => 0,
            HighlightSeverity::Warning => 1,
            HighlightSeverity::WeakWarning => 2,
            HighlightSeverity::Info => 3,
            _ => 4,
        }
    }
}

/// Every highlight with a message that contains `offset`, most severe first (the tooltip of
/// the text under the pointer); the index is into `highlights`.
pub fn messages_at(highlights: &[RangeHighlight], offset: usize) -> Vec<(usize, &RangeHighlight)> {
    let mut found: Vec<(usize, &RangeHighlight)> = highlights
        .iter()
        .enumerate()
        .filter(|(_, h)| h.message.is_some() && h.range.contains(&offset))
        .collect();
    found.sort_by_key(|(i, h)| (h.severity.rank(), *i));
    found
}

/// The index of the highlight whose mark of the error stripe is on `row`: marks sit at the
/// stripe row of the start of their range (`mark_row` maps a byte offset to it; the caller
/// knows folds, wrapping and the scale), the most severe wins, the last given breaks a tie
/// (it is drawn on top). Text search marks are the find bar's and never hit.
pub fn stripe_hit(
    highlights: &[RangeHighlight],
    row: usize,
    mut mark_row: impl FnMut(usize) -> usize,
) -> Option<usize> {
    let mut best: Option<(u8, usize)> = None;
    for (i, h) in highlights.iter().enumerate() {
        if h.severity == HighlightSeverity::TextSearch {
            continue;
        }
        if mark_row(h.range.start) == row {
            let rank = h.severity.rank();
            if best.is_none_or(|(r, _)| rank <= r) {
                best = Some((rank, i));
            }
        }
    }
    best.map(|(_, i)| i)
}

/// The first `lines` lines of the text in `range`: the preview of what a fold hides.
pub fn preview_lines(text: &str, range: Range<usize>, lines: usize) -> Vec<String> {
    let range = range.start.min(text.len())..range.end.min(text.len());
    text.get(range)
        .unwrap_or("")
        .lines()
        .take(lines)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rest_restarts_when_the_pointer_moves_or_changes_offset() {
        let mut rest = None;
        assert_eq!(
            Rest::observe(&mut rest, 5, (10.0, 4.0), 1.0, 3.0),
            (0.0, true)
        );
        let (waited, restarted) = Rest::observe(&mut rest, 5, (11.0, 4.0), 1.6, 3.0);
        assert!((waited - 0.6).abs() < 1e-9 && !restarted);
        // Another offset restarts the clock.
        assert_eq!(
            Rest::observe(&mut rest, 6, (11.0, 4.0), 1.7, 3.0),
            (0.0, true)
        );
        // Moving farther than the tolerance restarts it too.
        Rest::observe(&mut rest, 6, (11.0, 4.0), 2.0, 3.0);
        assert_eq!(
            Rest::observe(&mut rest, 6, (20.0, 4.0), 2.5, 3.0),
            (0.0, true)
        );
    }

    #[test]
    fn the_wait_left_is_never_zero() {
        assert!(remaining(0.5, 0.7) > 0.0);
        assert!((remaining(0.5, 0.2) - 0.301).abs() < 1e-6);
    }

    #[test]
    fn a_fold_dwell_counts_per_placeholder() {
        let mut rest = None;
        assert_eq!(fold_dwell(&mut rest, 10, 1.0), 0.0);
        assert!((fold_dwell(&mut rest, 10, 1.4) - 0.4).abs() < 1e-9);
        assert_eq!(fold_dwell(&mut rest, 20, 1.5), 0.0);
    }

    fn marks() -> Vec<RangeHighlight> {
        vec![
            RangeHighlight::new(0..4, HighlightSeverity::Warning).message("w"),
            RangeHighlight::new(2..6, HighlightSeverity::Error).message("e"),
            RangeHighlight::new(2..3, HighlightSeverity::Info),
        ]
    }

    #[test]
    fn the_highlight_under_an_offset_is_the_first_that_contains_it() {
        let h = marks();
        assert_eq!(highlight_at(&h, 3), Some(0));
        assert_eq!(highlight_at(&h, 5), Some(1));
        assert_eq!(highlight_at(&h, 9), None);
    }

    #[test]
    fn messages_come_most_severe_first_and_skip_silent_marks() {
        let h = marks();
        let found: Vec<_> = messages_at(&h, 2).iter().map(|(i, _)| *i).collect();
        assert_eq!(found, [1, 0]);
    }

    #[test]
    fn severity_ranks_order_errors_first() {
        assert!(HighlightSeverity::Error.rank() < HighlightSeverity::Warning.rank());
        assert!(HighlightSeverity::Warning.rank() < HighlightSeverity::WeakWarning.rank());
        assert!(HighlightSeverity::WeakWarning.rank() < HighlightSeverity::Info.rank());
        assert!(HighlightSeverity::Info.rank() < HighlightSeverity::TextSearch.rank());
    }

    #[test]
    fn the_stripe_hit_prefers_the_most_severe_mark_on_the_row() {
        let h = marks();
        // Starts 0, 2 and 2 map to rows 0, 1 and 1.
        let row_of = |o: usize| usize::from(o >= 2);
        assert_eq!(stripe_hit(&h, 1, row_of), Some(1));
        assert_eq!(stripe_hit(&h, 0, row_of), Some(0));
        assert_eq!(stripe_hit(&h, 2, row_of), None);
    }

    #[test]
    fn the_preview_shows_the_first_lines_of_the_folded_text() {
        let text = "a {\n  b\n  c\n  d\n}";
        let lines = preview_lines(text, 2..text.len(), 3);
        assert_eq!(lines, ["{", "  b", "  c"]);
        assert!(preview_lines(text, 100..200, 3).is_empty());
    }
}
