//! Styled text runs: IntelliJ's `SimpleTextAttributes` and the fragments a
//! `SimpleColoredComponent` is made of.

use super::{State, Tone};
use crate::theme::Theme;
use crate::tokens::{speed_search, typography};
use egui::text::{CCursor, LayoutJob, LayoutSection, TextFormat};
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, Ui, vec2};
use std::ops::Range;
use std::sync::Arc;

/// How one fragment of text is drawn: its color by meaning ([`Tone`]) and style flags.
///
/// The presets carry IntelliJ's names (`REGULAR_ATTRIBUTES`, `GRAYED_ATTRIBUTES`,
/// `LINK_ATTRIBUTES` …). Colors never appear here: a [`Tone`] resolves to the theme's color
/// when the text is painted, selected rows included.
///
/// ```
/// use archipel_kit::parts::SimpleTextAttributes;
///
/// let attributes = SimpleTextAttributes::GRAYED_ATTRIBUTES.bold(true);
/// assert!(attributes.is_bold());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SimpleTextAttributes {
    tone: Tone,
    bold: bool,
    italic: bool,
    strikeout: bool,
    underline: bool,
    smaller: bool,
    search_match: bool,
}

impl SimpleTextAttributes {
    /// Plain text in the default color (`REGULAR_ATTRIBUTES`).
    pub const REGULAR_ATTRIBUTES: Self = Self::new(Tone::Default);
    /// Semibold text (`REGULAR_BOLD_ATTRIBUTES`).
    pub const REGULAR_BOLD_ATTRIBUTES: Self = Self::new(Tone::Default).bold(true);
    /// Italic text (`REGULAR_ITALIC_ATTRIBUTES`).
    pub const REGULAR_ITALIC_ATTRIBUTES: Self = Self::new(Tone::Default).italic(true);
    /// Error text (`ERROR_ATTRIBUTES`).
    pub const ERROR_ATTRIBUTES: Self = Self::new(Tone::Error);
    /// Dimmed text: details, hints, empty text (`GRAYED_ATTRIBUTES`).
    pub const GRAYED_ATTRIBUTES: Self = Self::new(Tone::Muted);
    /// Dimmed semibold text (`GRAYED_BOLD_ATTRIBUTES`).
    pub const GRAYED_BOLD_ATTRIBUTES: Self = Self::new(Tone::Muted).bold(true);
    /// Dimmed italic text (`GRAYED_ITALIC_ATTRIBUTES`).
    pub const GRAYED_ITALIC_ATTRIBUTES: Self = Self::new(Tone::Muted).italic(true);
    /// Dimmed small text (`GRAYED_SMALL_ATTRIBUTES`).
    pub const GRAYED_SMALL_ATTRIBUTES: Self = Self::new(Tone::Muted).smaller(true);
    /// Same as [`GRAYED_ATTRIBUTES`](Self::GRAYED_ATTRIBUTES) (`GRAY_ATTRIBUTES`).
    pub const GRAY_ATTRIBUTES: Self = Self::GRAYED_ATTRIBUTES;
    /// Same as [`GRAYED_ITALIC_ATTRIBUTES`](Self::GRAYED_ITALIC_ATTRIBUTES)
    /// (`GRAY_ITALIC_ATTRIBUTES`).
    pub const GRAY_ITALIC_ATTRIBUTES: Self = Self::GRAYED_ITALIC_ATTRIBUTES;
    /// Same as [`GRAYED_SMALL_ATTRIBUTES`](Self::GRAYED_SMALL_ATTRIBUTES)
    /// (`GRAY_SMALL_ATTRIBUTES`).
    pub const GRAY_SMALL_ATTRIBUTES: Self = Self::GRAYED_SMALL_ATTRIBUTES;
    /// A link: link color, underlined (`LINK_ATTRIBUTES`).
    pub const LINK_ATTRIBUTES: Self = Self::new(Tone::Link).underline(true);
    /// A link without underline (`LINK_PLAIN_ATTRIBUTES`).
    pub const LINK_PLAIN_ATTRIBUTES: Self = Self::new(Tone::Link);
    /// A semibold link (`LINK_BOLD_ATTRIBUTES`).
    pub const LINK_BOLD_ATTRIBUTES: Self = Self::new(Tone::Link).bold(true);
    /// A shortcut hint (`SHORTCUT_ATTRIBUTES`): the secondary text color.
    pub const SHORTCUT_ATTRIBUTES: Self = Self::new(Tone::Secondary);
    /// A text that matches the speed search: the theme's search match background
    /// (`STYLE_SEARCH_MATCH`).
    pub const SEARCH_MATCH_ATTRIBUTES: Self = Self::new(Tone::Default).search_match(true);
    /// Struck-out dimmed text, for excluded items (`EXCLUDED_ATTRIBUTES`).
    pub const EXCLUDED_ATTRIBUTES: Self = Self::new(Tone::Muted).strikeout(true);

    /// Plain text in the color of `tone`.
    pub const fn new(tone: Tone) -> Self {
        Self {
            tone,
            bold: false,
            italic: false,
            strikeout: false,
            underline: false,
            smaller: false,
            search_match: false,
        }
    }

    /// The color's meaning.
    pub const fn tone(self) -> Tone {
        self.tone
    }

    /// The same attributes in the color of `tone` (`derive`).
    pub const fn with_tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Semibold (`STYLE_BOLD`).
    pub const fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Italic (`STYLE_ITALIC`).
    pub const fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Struck out (`STYLE_STRIKEOUT`).
    pub const fn strikeout(mut self, strikeout: bool) -> Self {
        self.strikeout = strikeout;
        self
    }

    /// Underlined (`STYLE_UNDERLINE`).
    pub const fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Two points smaller than the body text (`STYLE_SMALLER`).
    pub const fn smaller(mut self, smaller: bool) -> Self {
        self.smaller = smaller;
        self
    }

    /// Drawn on the search match background (`STYLE_SEARCH_MATCH`).
    pub const fn search_match(mut self, search_match: bool) -> Self {
        self.search_match = search_match;
        self
    }

    /// Whether the text is semibold.
    pub const fn is_bold(self) -> bool {
        self.bold
    }

    /// Whether the text is italic.
    pub const fn is_italic(self) -> bool {
        self.italic
    }

    /// Whether the text is struck out.
    pub const fn is_strikeout(self) -> bool {
        self.strikeout
    }

    /// Whether the text is underlined.
    pub const fn is_underline(self) -> bool {
        self.underline
    }

    /// Whether the text is drawn smaller than the body text.
    pub const fn is_smaller(self) -> bool {
        self.smaller
    }

    /// Whether the text is drawn on the search match background.
    pub const fn is_search_match(self) -> bool {
        self.search_match
    }
}

/// A piece of text with its [`SimpleTextAttributes`]: one `append` of a
/// `SimpleColoredComponent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFragment {
    /// The text.
    pub text: String,
    /// How it is drawn.
    pub attributes: SimpleTextAttributes,
}

impl TextFragment {
    /// A fragment reading `text` in `attributes`.
    pub fn new(text: impl Into<String>, attributes: SimpleTextAttributes) -> Self {
        Self {
            text: text.into(),
            attributes,
        }
    }
}

/// Splits `fragments` so that the characters in `matches` (char ranges over the
/// concatenation of all fragments) carry the search match style.
pub(crate) fn apply_matches(
    fragments: &[(String, SimpleTextAttributes)],
    matches: &[Range<usize>],
) -> Vec<(String, SimpleTextAttributes)> {
    if matches.is_empty() {
        return fragments.to_vec();
    }
    let mut out = Vec::with_capacity(fragments.len() + 2 * matches.len());
    let mut offset = 0;
    for (text, attributes) in fragments {
        let mut piece = String::new();
        let mut piece_match = false;
        for ch in text.chars() {
            let hit = matches.iter().any(|range| range.contains(&offset));
            if hit != piece_match && !piece.is_empty() {
                out.push((
                    std::mem::take(&mut piece),
                    attributes.search_match(piece_match || attributes.is_search_match()),
                ));
            }
            piece_match = hit;
            piece.push(ch);
            offset += 1;
        }
        if !piece.is_empty() {
            out.push((
                piece,
                attributes.search_match(piece_match || attributes.is_search_match()),
            ));
        }
    }
    out
}

/// The font of a fragment with `attributes`.
fn font(ui: &Ui, attributes: SimpleTextAttributes, base_size: f32) -> egui::FontId {
    let size = if attributes.is_smaller() {
        typography::SMALL
    } else {
        base_size
    };
    if attributes.is_bold() {
        typography::semibold(ui.ctx(), size)
    } else {
        typography::body(size)
    }
}

/// Lays out `runs` on one line, cut at `max_width` with `…`. Colors come from the run's tone in
/// `state`, so a selected or disabled row recolors its text like the rest of the kit.
pub(crate) fn layout_runs(
    ui: &Ui,
    runs: &[(String, SimpleTextAttributes)],
    state: State,
    max_width: f32,
) -> Arc<egui::Galley> {
    let colors = Theme::colors(ui.ctx());
    let mut job = LayoutJob::default();
    job.wrap.max_width = max_width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let mut text = String::new();
    for (piece, attributes) in runs {
        let start = text.len();
        text.push_str(piece);
        let color = state.text(attributes.tone(), &colors);
        job.sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: start..text.len(),
            format: TextFormat {
                font_id: font(ui, *attributes, typography::BODY),
                color,
                italics: attributes.is_italic(),
                underline: if attributes.is_underline() {
                    Stroke::new(1.0_f32, color)
                } else {
                    Stroke::NONE
                },
                strikethrough: if attributes.is_strikeout() {
                    Stroke::new(1.0_f32, color)
                } else {
                    Stroke::NONE
                },
                ..TextFormat::default()
            },
        });
    }
    job.text = text;
    ui.painter().layout_job(job)
}

/// Lays out one run of `text` in `attributes`: a single-fragment [`layout_runs`].
pub(crate) fn layout_run(
    ui: &Ui,
    text: &str,
    attributes: SimpleTextAttributes,
    state: State,
    max_width: f32,
) -> Arc<egui::Galley> {
    layout_runs(ui, &[(text.to_owned(), attributes)], state, max_width)
}

/// Paints the search match backgrounds of `runs` behind a galley laid out from them at
/// `pos`.
pub(crate) fn paint_match_backgrounds(
    painter: &Painter,
    galley: &egui::Galley,
    pos: Pos2,
    runs: &[(String, SimpleTextAttributes)],
    fill: Color32,
) {
    let mut offset = 0;
    let mut ranges = Vec::new();
    for (text, attributes) in runs {
        let count = text.chars().count();
        if attributes.is_search_match() && count > 0 {
            ranges.push(offset..offset + count);
        }
        offset += count;
    }
    paint_ranges(painter, galley, pos, &ranges, fill);
}

/// Paints `fill` behind the character `ranges` of a galley drawn at `pos` (one row).
pub(crate) fn paint_ranges(
    painter: &Painter,
    galley: &egui::Galley,
    pos: Pos2,
    ranges: &[Range<usize>],
    fill: Color32,
) {
    for range in ranges {
        let start = galley.pos_from_cursor(CCursor::new(range.start));
        let end = galley.pos_from_cursor(CCursor::new(range.end));
        let right = end.right().min(galley.size().x);
        if right > start.left() {
            let rect = Rect::from_min_max(
                pos + vec2(start.left(), start.top()),
                pos + vec2(right, start.bottom()),
            );
            painter.rect_filled(rect, CornerRadius::same(speed_search::MATCH_RADIUS), fill);
        }
    }
}
