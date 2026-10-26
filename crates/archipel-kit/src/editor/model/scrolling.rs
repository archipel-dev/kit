//! Scroll-to-caret math (spec 4.1): where the viewport has to go, no animation.

use super::settings::EditorSettings;

/// How the caret is brought into view (IntelliJ `ScrollType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollType {
    /// After typing or moving: scroll just enough to keep the margin.
    Relative,
    /// Navigation: when the caret is not inside the margins, center it.
    MakeVisible,
    /// Always center the caret line.
    Center,
    /// Center only when the caret is above the viewport (needs to scroll up).
    CenterUp,
    /// Center only when the caret is below the viewport (needs to scroll down).
    CenterDown,
}

/// The visible part of the editor, in lines and columns (fractional for pixel scrolling).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// First visible visual line (may be fractional).
    pub first_line: f64,
    /// Visible height in lines (may be fractional).
    pub visible_lines: f64,
    /// First visible column.
    pub first_column: f64,
    /// Visible width in columns.
    pub visible_columns: f64,
}

/// The size of the content: visual lines and the longest line in columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentSize {
    /// Number of visual lines.
    pub lines: usize,
    /// Width of the longest line in columns.
    pub columns: usize,
}

/// Where the viewport should end up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollTarget {
    /// Target first visible line.
    pub first_line: f64,
    /// Target first visible column.
    pub first_column: f64,
    /// Suggested animation length in milliseconds (0: jump).
    pub animation_ms: f64,
}

/// Duration of the longest caret-jump animation.
pub const MAX_ANIMATION_MS: f64 = 100.0;

/// The largest first visible line: content plus the scroll-past-end slack.
pub fn max_first_line(content: ContentSize, visible_lines: f64, settings: &EditorSettings) -> f64 {
    let slack = if settings.additional_page_at_bottom {
        visible_lines - 1.0
    } else {
        settings.scroll_past_end_lines as f64
    };
    (content.lines as f64 + slack - visible_lines).max(0.0)
}

/// The largest first visible column.
pub fn max_first_column(
    content: ContentSize,
    visible_columns: f64,
    settings: &EditorSettings,
) -> f64 {
    ((content.columns + settings.additional_columns) as f64 - visible_columns).max(0.0)
}

fn center_line(line: f64, visible: f64) -> f64 {
    // The "one third from the top" target when the viewport can show the margin above it;
    // the exact center otherwise.
    let third = visible / 3.0;
    if third >= 1.0 {
        line - third
    } else {
        line + 0.5 - visible / 2.0
    }
}

/// The first line the viewport must show so the caret on `caret_line` follows `scroll_type`.
pub fn vertical_target(
    viewport: &Viewport,
    caret_line: f64,
    scroll_type: ScrollType,
    settings: &EditorSettings,
) -> f64 {
    let margin = settings.vertical_scroll_offset as f64;
    let jump = settings.vertical_scroll_jump as f64;
    let top = caret_line - margin;
    let bottom = caret_line + margin + 1.0;
    let band = bottom - top;
    let first = viewport.first_line;
    let visible = viewport.visible_lines;
    let below = bottom > first + visible;
    let above = top < first;
    match scroll_type {
        ScrollType::Relative => {
            if band > visible {
                caret_line + 0.5 - visible / 2.0
            } else if below {
                bottom - visible + jump
            } else if above {
                top - jump
            } else {
                first
            }
        }
        ScrollType::MakeVisible => {
            if below || above {
                center_line(caret_line, visible)
            } else {
                first
            }
        }
        ScrollType::Center => center_line(caret_line, visible),
        ScrollType::CenterUp => {
            if above {
                center_line(caret_line, visible)
            } else {
                first
            }
        }
        ScrollType::CenterDown => {
            if below {
                center_line(caret_line, visible)
            } else {
                first
            }
        }
    }
}

/// The first column the viewport must show so the caret at `caret_column` keeps its margin.
pub fn horizontal_target(
    viewport: &Viewport,
    caret_column: f64,
    scroll_type: ScrollType,
    settings: &EditorSettings,
) -> f64 {
    let margin = settings.horizontal_scroll_offset as f64;
    let jump = settings.horizontal_scroll_jump as f64;
    let first = viewport.first_column;
    let visible = viewport.visible_columns;
    let left = caret_column - margin;
    let right = caret_column + margin;
    if scroll_type == ScrollType::Center {
        return caret_column - visible / 2.0;
    }
    if right - left > visible {
        // Both margins cannot fit.
        return caret_column - visible / 2.0;
    }
    if left < first {
        if scroll_type == ScrollType::MakeVisible && right <= visible {
            0.0
        } else {
            left - jump
        }
    } else if right > first + visible {
        right - visible + jump
    } else {
        first
    }
}

/// Computes the scroll target for a caret at (`caret_line`, `caret_column`) in visual
/// coordinates, clamped to the content (scroll past end included).
///
/// `animate` is false for typing, Home/End and document start/end (spec 4.1); otherwise the
/// duration follows the jump distance: nothing for one line or less, [`MAX_ANIMATION_MS`]
/// from eleven lines.
pub fn scroll_to_caret(
    viewport: &Viewport,
    content: ContentSize,
    caret_line: f64,
    caret_column: f64,
    scroll_type: ScrollType,
    animate: bool,
    settings: &EditorSettings,
) -> ScrollTarget {
    let line = vertical_target(viewport, caret_line, scroll_type, settings);
    let column = horizontal_target(viewport, caret_column, scroll_type, settings);
    let line = line.clamp(
        0.0,
        max_first_line(content, viewport.visible_lines, settings),
    );
    let column = column.clamp(
        0.0,
        max_first_column(content, viewport.visible_columns, settings),
    );
    let distance = (line - viewport.first_line).abs();
    let animation_ms = if animate {
        ((distance - 1.0) / 10.0).clamp(0.0, 1.0) * MAX_ANIMATION_MS
    } else {
        0.0
    };
    ScrollTarget {
        first_line: line,
        first_column: column,
        animation_ms,
    }
}

/// Clamps a first line into the scrollable range (page scrolling, wheel).
pub fn clamp_first_line(
    first_line: f64,
    content: ContentSize,
    visible_lines: f64,
    settings: &EditorSettings,
) -> f64 {
    first_line.clamp(0.0, max_first_line(content, visible_lines, settings))
}
