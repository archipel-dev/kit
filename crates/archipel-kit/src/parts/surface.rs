//! The surfaces things are drawn on.

use crate::theme::Theme;
use crate::tokens::{radius, size};
use egui::{CornerRadius, Margin, Stroke};

/// A themed surface. Every container in the kit is one of these, so a new component that
/// floats, frames or groups content looks like the rest by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// A workbench island: tool windows, editors, panels.
    Island,
    /// A popup or menu: below its anchor, with shadow and border.
    Popup,
    /// A tooltip: small, tight padding.
    Tooltip,
    /// A modal dialog: island radius, generous padding.
    Dialog,
    /// The workbench chrome: the frame around the islands, toolbars and the status bar, in
    /// the theme's frame color (square, with a gap's padding so islands do not touch it).
    Chrome,
    /// An input field's box (without focus or error state; see [`Surface::field_stroke`]).
    Field,
    /// A flush panel: an island's fill with square corners, for the New UI and Classic
    /// looks, where panels touch and 1 px lines separate them
    /// ([`Surface::island_lines`] draws those). [`Surface::Island`] turns into this by
    /// itself in those looks.
    Flat,
}

impl Surface {
    /// The frame for this surface in the theme of `ctx`.
    pub fn frame(self, ctx: &egui::Context) -> egui::Frame {
        let colors = Theme::colors(ctx);
        let style = ctx.style();
        let metrics = crate::tokens::metrics(ctx);
        match self {
            // In the flush looks an island is a square panel (its radius is 0).
            Surface::Island => egui::Frame::new()
                .fill(colors.island)
                .corner_radius(metrics.island_radius)
                .inner_margin(Margin::same(size::CONTENT_INSET as i8)),
            Surface::Flat => egui::Frame::new()
                .fill(colors.island)
                .inner_margin(Margin::same(size::CONTENT_INSET as i8)),
            Surface::Popup => egui::Frame::popup(&style)
                .fill(colors.popup)
                .stroke(Stroke::new(1.0_f32, colors.popup_border))
                .corner_radius(metrics.popup_radius)
                .inner_margin(Margin::symmetric(0, metrics.popup_padding as i8)),
            Surface::Tooltip => egui::Frame::popup(&style)
                .fill(colors.tooltip)
                .stroke(Stroke::new(1.0_f32, colors.popup_border))
                .corner_radius(CornerRadius::same(metrics.control_radius))
                // Android Studio's tooltips: 16 pt at the sides, 36 pt high for one line
                // (Classic: the smaller classic insets).
                .inner_margin(Margin::symmetric(
                    metrics.tooltip_padding_x as i8,
                    metrics.tooltip_padding_y as i8,
                )),
            Surface::Dialog => egui::Frame::popup(&style)
                .fill(colors.popup)
                .stroke(Stroke::new(1.0_f32, colors.popup_border))
                .corner_radius(CornerRadius::same(
                    if metrics.look == crate::UiLook::Classic {
                        0
                    } else {
                        radius::ISLAND
                    },
                ))
                .inner_margin(Margin::same(size::DIALOG_PADDING as i8 + 4)),
            Surface::Chrome => egui::Frame::new()
                .fill(colors.frame)
                .inner_margin(Margin::same(metrics.island_gap as i8)),
            Surface::Field => egui::Frame::new()
                .fill(colors.field)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .corner_radius(CornerRadius::same(metrics.control_radius))
                .inner_margin(Margin::symmetric(8, 0)),
        }
    }

    /// The 1 px lines of a flush panel at `rect` on the `sides` that touch another panel
    /// (`[left, top, right, bottom]`), in the theme's divider color. Nothing in Islands, where
    /// gaps separate the panels.
    pub fn island_lines(painter: &egui::Painter, rect: egui::Rect, sides: [bool; 4]) {
        let metrics = crate::tokens::metrics(painter.ctx());
        if metrics.panel_line <= 0.0 {
            return;
        }
        let stroke = Stroke::new(metrics.panel_line, Theme::colors(painter.ctx()).divider);
        let h = metrics.panel_line / 2.0;
        if sides[0] {
            painter.vline(rect.left() + h, rect.y_range(), stroke);
        }
        if sides[1] {
            painter.hline(rect.x_range(), rect.top() + h, stroke);
        }
        if sides[2] {
            painter.vline(rect.right() - h, rect.y_range(), stroke);
        }
        if sides[3] {
            painter.hline(rect.x_range(), rect.bottom() - h, stroke);
        }
    }

    /// The border of an input field: accent and thicker when focused, red on error.
    pub fn field_stroke(ctx: &egui::Context, focused: bool, error: bool) -> Stroke {
        let colors = Theme::colors(ctx);
        match (error, focused) {
            (true, true) => Stroke::new(size::FOCUS_RING, colors.error),
            (true, false) => Stroke::new(1.0_f32, colors.error),
            (false, true) => Stroke::new(size::FOCUS_RING, colors.accent),
            (false, false) => Stroke::new(1.0_f32, colors.border),
        }
    }
}

/// A vertical scroll area set up for desktop: wheel, trackpad and scroll bar, no
/// drag-to-scroll (which on desktop steals clicks and drags from the content).
pub fn scroll_vertical() -> egui::ScrollArea {
    desktop(egui::ScrollArea::vertical())
}

/// `area` with desktop scrolling (see [`scroll_vertical`]).
pub fn desktop(area: egui::ScrollArea) -> egui::ScrollArea {
    use egui::scroll_area::ScrollSource;
    area.scroll_source(ScrollSource::MOUSE_WHEEL | ScrollSource::SCROLL_BAR)
}
