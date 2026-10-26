//! Interaction state and the colors it maps to.

use super::Tone;
use crate::theme::Colors;
use crate::tokens::size;
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

    /// Fill of a row in a list, tree or table: the selection, or the quieter selection of a
    /// list without focus. Rows have no hover fill, as in Android Studio's trees and lists.
    pub fn row_fill(&self, colors: &Colors) -> Color32 {
        match (self.selected, self.inactive) {
            (true, false) => colors.selection,
            (true, true) => colors.selection_inactive,
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

    /// The color of text meant as `tone`: disabled text when disabled, the theme's selection
    /// foreground for the default tone on a selected row (`List.selectionForeground`: white
    /// on the classic light themes' dark blue selection; its inactive variant without focus).
    pub fn text(&self, tone: Tone, colors: &Colors) -> Color32 {
        if !self.enabled {
            colors.text_disabled
        } else if self.selected && tone == Tone::Default {
            if self.inactive {
                colors.selection_text_inactive
            } else {
                colors.selection_text
            }
        } else if self.selected
            && !self.inactive
            && colors.selection_text != colors.text_strong
            && matches!(tone, Tone::Strong | Tone::Secondary | Tone::Muted)
        {
            // A selection with a foreground of its own (white on the classic themes' blue)
            // paints the gray texts of the row in it too, so they stay readable.
            colors.selection_text
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
    // As in Android Studio: the control's 1 pt border becomes 2 pt in the focus color, one
    // of them outside the control.
    painter.rect_stroke(
        rect.expand(1.0),
        CornerRadius::same(corner.saturating_add(1)),
        Stroke::new(size::FOCUS_RING, colors.focus_ring),
        StrokeKind::Inside,
    );
}

/// The rounded pill behind a row spanning `rect` (inset from the edges), in `fill`.
/// In the Classic look it is a flat rectangle over the whole row.
pub fn row_pill(painter: &Painter, rect: Rect, fill: Color32) {
    if fill != Color32::TRANSPARENT {
        let metrics = crate::tokens::metrics(painter.ctx());
        let pill = rect.shrink2(egui::vec2(metrics.row_pill_inset, 0.0));
        painter.rect_filled(pill, CornerRadius::same(metrics.row_radius), fill);
    }
}

/// A tree expander in `rect`: a chevron in New UI and Islands, a small filled triangle in the
/// Classic look (looks spec §2.7). `expanded` points it down.
pub fn paint_expander(painter: &Painter, rect: Rect, expanded: bool, color: Color32) {
    use crate::look::ExpanderStyle;
    match crate::tokens::metrics(painter.ctx()).expander {
        ExpanderStyle::Chevron => crate::icons::paint(
            painter,
            rect,
            if expanded {
                crate::Glyph::ChevronDown
            } else {
                crate::Glyph::ChevronRight
            },
            color,
        ),
        ExpanderStyle::Triangle => {
            // An 8 pt triangle centered in the column, like the classic tree icons.
            let c = rect.center();
            let h = size::CLASSIC_EXPANDER / 2.0;
            let points = if expanded {
                vec![
                    egui::pos2(c.x - h, c.y - h / 2.0),
                    egui::pos2(c.x + h, c.y - h / 2.0),
                    egui::pos2(c.x, c.y + h / 2.0 + 1.0),
                ]
            } else {
                vec![
                    egui::pos2(c.x - h / 2.0, c.y - h),
                    egui::pos2(c.x + h / 2.0 + 1.0, c.y),
                    egui::pos2(c.x - h / 2.0, c.y + h),
                ]
            };
            painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
        }
    }
}

/// Fills `rect` (with `corner`) with a vertical gradient from `top` to `bottom`: the Classic
/// look's button fill (looks spec §2.9). A plain fill when both are equal.
pub fn fill_gradient(
    painter: &Painter,
    rect: Rect,
    corner: CornerRadius,
    top: Color32,
    bottom: Color32,
) {
    if top == bottom {
        painter.rect_filled(rect, corner, top);
        return;
    }
    let shape = egui::epaint::RectShape::filled(rect, corner, Color32::WHITE);
    let mut mesh = egui::Mesh::default();
    let mut tessellator = egui::epaint::Tessellator::new(
        painter.ctx().pixels_per_point(),
        egui::epaint::TessellationOptions::default(),
        [1, 1],
        Vec::new(),
    );
    tessellator.tessellate_rect(&shape, &mut mesh);
    let height = rect.height().max(1.0);
    for vertex in &mut mesh.vertices {
        let t = ((vertex.pos.y - rect.top()) / height).clamp(0.0, 1.0);
        let color = crate::common::lerp_color(top, bottom, t);
        vertex.color = color.gamma_multiply(f32::from(vertex.color.a()) / 255.0);
    }
    painter.add(egui::Shape::mesh(mesh));
}
