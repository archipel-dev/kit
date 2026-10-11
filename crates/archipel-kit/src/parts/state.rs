//! Interaction state and the colors it maps to.

use super::Tone;
use crate::theme::Colors;
use crate::tokens::{radius, size};
use egui::{Color32, CornerRadius, Painter, Rect, Response, Stroke, StrokeKind};

/// The interaction state of a widget this frame. Build it from the response with
/// [`State::of`] and let it pick fills and text colors, so every component shows hover,
/// press, focus, selection and disabled the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    /// Accepts input.
    pub enabled: bool,
    /// The pointer is over it.
    pub hovered: bool,
    /// A pointer button is held on it.
    pub pressed: bool,
    /// It has keyboard focus.
    pub focused: bool,
    /// It is the selected item.
    pub selected: bool,
    /// Selected, but its list doesn't have focus (drawn quieter).
    pub inactive: bool,
}

impl State {
    /// The state of `response`.
    pub fn of(response: &Response) -> Self {
        Self {
            enabled: response.enabled() && response.sense.interactive(),
            hovered: response.hovered(),
            pressed: response.is_pointer_button_down_on(),
            focused: response.has_focus(),
            selected: false,
            inactive: false,
        }
    }

    /// Marks it selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Marks a selection as inactive (its list has no focus).
    pub fn inactive(mut self, inactive: bool) -> Self {
        self.inactive = inactive;
        self
    }

    /// Fill of a row in a list, tree or table: selection, quieter inactive selection, or
    /// hover.
    pub fn row_fill(&self, colors: &Colors) -> Color32 {
        match (self.selected, self.inactive) {
            (true, false) => colors.selection,
            (true, true) => colors.row_hover,
            (false, _) if self.enabled && self.hovered => colors.row_hover,
            _ => Color32::TRANSPARENT,
        }
    }

    /// Fill of a menu item: the selection color under the pointer.
    pub fn menu_fill(&self, colors: &Colors) -> Color32 {
        if self.enabled && (self.hovered || self.selected) {
            colors.selection
        } else {
            Color32::TRANSPARENT
        }
    }

    /// Fill of a flat control (toolbar button): pressed, hover, or nothing.
    pub fn flat_fill(&self, colors: &Colors) -> Color32 {
        if !self.enabled {
            Color32::TRANSPARENT
        } else if self.pressed {
            colors.pressed
        } else if self.hovered {
            colors.hover
        } else {
            Color32::TRANSPARENT
        }
    }

    /// The color of text meant as `tone`: disabled text when disabled, strong text for the
    /// default tone on a selected row.
    pub fn text(&self, tone: Tone, colors: &Colors) -> Color32 {
        if !self.enabled {
            colors.text_disabled
        } else if self.selected && tone == Tone::Default {
            colors.text_strong
        } else {
            tone.color(colors)
        }
    }

    /// The color of an icon meant as `tone` (disabled icons are dimmed).
    pub fn icon(&self, tone: Tone, colors: &Colors) -> Color32 {
        if self.enabled {
            tone.color(colors)
        } else {
            colors.text_disabled
        }
    }
}

/// The keyboard focus ring around `rect`.
pub fn focus_ring(painter: &Painter, rect: Rect, corner: u8, colors: &Colors) {
    painter.rect_stroke(
        rect.expand(1.0),
        CornerRadius::same(corner.saturating_add(1)),
        Stroke::new(size::FOCUS_RING, colors.focus_ring),
        StrokeKind::Outside,
    );
}

/// The rounded pill behind a row spanning `rect` (inset from the edges), in `fill`.
pub fn row_pill(painter: &Painter, rect: Rect, fill: Color32) {
    if fill != Color32::TRANSPARENT {
        let pill = rect.shrink2(egui::vec2(size::ROW_PILL_INSET, 0.0));
        painter.rect_filled(pill, CornerRadius::same(radius::ROW), fill);
    }
}
