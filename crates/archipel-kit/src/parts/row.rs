//! The content of a row: icon, label, secondary text, trailing text.

use super::text_attributes::{self, SimpleTextAttributes, TextFragment};
use super::{State, Tone};
use crate::common;
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{Color32, Painter, Rect, Ui, pos2, vec2};
use std::fmt;
use std::ops::Range;

/// Paints a custom icon (a badged file icon) into the icon column of a row: the painter, the
/// 16×16 rect and the color the row would paint a plain icon in (from the icon tone and
/// the row state, so it dims when the row is disabled).
#[derive(Clone, Copy)]
pub struct IconPainter<'a>(pub &'a dyn Fn(&Painter, Rect, Color32));

impl fmt::Debug for IconPainter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IconPainter")
    }
}

/// What a row shows, laid out the same in lists, trees, menus and popups: an icon column,
/// the label, dimmed secondary text after it, and right-aligned trailing text (a shortcut, a
/// count).
#[derive(Debug, Clone, Default)]
pub struct SimpleColoredComponent<'a> {
    /// Icon in the icon column.
    pub icon: Option<Icon>,
    /// Meaning of the icon's color (default: the icon gray).
    pub icon_tone: Option<Tone>,
    /// A custom-painted icon, instead of `icon`.
    pub icon_painter: Option<IconPainter<'a>>,
    /// Keep the icon column even without an icon, so labels of mixed rows line up (menus).
    pub reserve_icon: bool,
    /// The label.
    pub label: &'a str,
    /// Meaning of the label's color.
    pub label_tone: Tone,
    /// Semibold label (group titles, running configurations).
    pub strong: bool,
    /// Dimmed text after the label.
    pub secondary: Option<&'a str>,
    /// Right-aligned dimmed text.
    pub trailing: Option<&'a str>,
    /// Width kept free at the right end for something else drawn there (a trailing slot).
    /// Text longer than the room left is elided with `…`.
    pub reserve_right: f32,
    /// More fragments of text after the label, each with its own
    /// [`SimpleTextAttributes`] (`SimpleColoredComponent.append`).
    pub fragments: &'a [TextFragment],
    /// Character ranges of the label and the fragments together (the label first) drawn as
    /// search matches.
    pub matches: &'a [Range<usize>],
}

/// The laid out label of a row: plain, or styled runs.
struct Styled {
    galley: std::sync::Arc<egui::Galley>,
    runs: Vec<(String, SimpleTextAttributes)>,
    elided: bool,
    /// Width before eliding.
    natural: f32,
}

impl Styled {
    fn new(
        galley: std::sync::Arc<egui::Galley>,
        runs: Vec<(String, SimpleTextAttributes)>,
        elided: bool,
        natural: f32,
    ) -> Self {
        Self {
            galley,
            runs,
            elided,
            natural,
        }
    }
}

impl<'a> SimpleColoredComponent<'a> {
    /// The label and the fragments as runs, search matches applied.
    fn runs(&self) -> Vec<(String, SimpleTextAttributes)> {
        let label = SimpleTextAttributes::new(self.label_tone).bold(self.strong);
        let mut runs = vec![(self.label.to_owned(), label)];
        runs.extend(
            self.fragments
                .iter()
                .map(|f| (f.text.clone(), f.attributes)),
        );
        text_attributes::apply_matches(&runs, self.matches)
    }

    /// A row reading `label`.
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            ..Self::default()
        }
    }

    /// The label font: regular, or semibold when [`strong`](Self::strong).
    fn label_font(&self, ui: &Ui) -> egui::FontId {
        if self.strong {
            typography::semibold(ui.ctx(), typography::BODY)
        } else {
            typography::body(typography::BODY)
        }
    }

    /// Width the content needs (without trailing padding).
    pub fn min_width(&self, ui: &Ui) -> f32 {
        let font = typography::body(typography::BODY);
        let text = |s: &str| common::galley(ui, s, font.clone()).size().x;
        let label = common::galley(ui, self.label, self.label_font(ui)).size().x;
        let icon = if self.icon.is_some() || self.icon_painter.is_some() || self.reserve_icon {
            size::ICON + size::ICON_TEXT_GAP
        } else {
            0.0
        };
        let fragments: f32 = self
            .fragments
            .iter()
            .map(|f| common::galley(ui, f.text.as_str(), font.clone()).size().x)
            .sum();
        icon + label
            + fragments
            + self
                .secondary
                .map_or(0.0, |s| size::SECONDARY_GAP + text(s))
            + self.trailing.map_or(0.0, |s| 32.0 + text(s))
    }

    /// Paints the content in `rect`, starting at `x`, in `state`. Returns whether the label
    /// was elided, so the caller can show it in full as a tooltip.
    pub fn paint(&self, ui: &Ui, rect: Rect, x: f32, state: State) -> bool {
        let colors = Theme::colors(ui.ctx());
        let painter = ui.painter();
        let mut x = x;
        if self.icon.is_some() || self.icon_painter.is_some() || self.reserve_icon {
            let icon_rect = Rect::from_min_size(
                pos2(x, rect.center().y - size::ICON / 2.0),
                vec2(size::ICON, size::ICON),
            );
            let tone = self.icon_tone.unwrap_or(Tone::Icon);
            if let Some(IconPainter(paint)) = self.icon_painter {
                paint(painter, icon_rect, state.icon(tone, &colors));
            } else if let Some(icon) = self.icon {
                icons::paint(painter, icon_rect, icon, state.icon(tone, &colors));
            }
            x = icon_rect.right() + size::ICON_TEXT_GAP;
        }
        let font = typography::body(typography::BODY);
        let trailing = self
            .trailing
            .map(|text| common::galley(ui, text, font.clone()));
        // Text may use the row up to the trailing text or the reserved slot.
        let mut right = rect.right() - 8.0 - self.reserve_right;
        if let Some(trailing) = &trailing {
            right = right.min(rect.right() - 8.0 - trailing.size().x - size::SECONDARY_GAP);
        }
        let available = (right - x).max(0.0);
        let styled = !self.fragments.is_empty() || !self.matches.is_empty();
        let label = if styled {
            // Several fragments, or search matches: one galley of styled runs.
            let runs = self.runs();
            let natural = text_attributes::layout_runs(ui, &runs, state, f32::INFINITY);
            let width = natural.size().x;
            if width > available {
                let cut = text_attributes::layout_runs(ui, &runs, state, available);
                Styled::new(cut, runs, true, width)
            } else {
                Styled::new(natural, runs, false, width)
            }
        } else {
            let label = common::galley(ui, self.label, self.label_font(ui));
            let width = label.size().x;
            if width > available {
                let cut = common::galley_elided(ui, self.label, self.label_font(ui), available);
                Styled::new(cut, Vec::new(), true, width)
            } else {
                Styled::new(label, Vec::new(), false, width)
            }
        };
        let secondary = self
            .secondary
            .map(|text| common::galley(ui, text, font.clone()));
        // The dimmed secondary text gives way first; the label gets what is left.
        let (label_width, elided) = (label.galley.size().x, label.elided);
        let secondary_room = available - label.natural - size::SECONDARY_GAP;
        if styled {
            let y = rect.center().y - label.galley.size().y / 2.0;
            let pos = pos2(x, y);
            text_attributes::paint_match_backgrounds(
                painter,
                &label.galley,
                pos,
                &label.runs,
                colors.search_match,
            );
            painter.galley(pos, label.galley.clone(), Color32::PLACEHOLDER);
        } else {
            common::text_left(
                painter,
                rect,
                x,
                label.galley.clone(),
                state.text(self.label_tone, &colors),
            );
        }
        if let (Some(text), Some(galley)) = (self.secondary, secondary) {
            let x = x + label_width + size::SECONDARY_GAP;
            let galley = if galley.size().x <= secondary_room {
                Some(galley)
            } else if secondary_room >= size::ROW_MIN_SECONDARY {
                Some(common::galley_elided(
                    ui,
                    text,
                    font.clone(),
                    secondary_room,
                ))
            } else {
                None
            };
            if let Some(galley) = galley {
                common::text_left(painter, rect, x, galley, state.text(Tone::Muted, &colors));
            }
        }
        if let Some(galley) = trailing {
            let x = rect.right() - 8.0 - galley.size().x;
            common::text_left(painter, rect, x, galley, state.text(Tone::Muted, &colors));
        }
        elided
    }
}
