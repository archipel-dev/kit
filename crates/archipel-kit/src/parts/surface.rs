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
}

impl Surface {
    /// The frame for this surface in the theme of `ctx`.
    pub fn frame(self, ctx: &egui::Context) -> egui::Frame {
        let colors = Theme::colors(ctx);
        let style = ctx.style();
        match self {
            Surface::Island => egui::Frame::new()
                .fill(colors.island)
                .corner_radius(radius::ISLAND)
                .inner_margin(Margin::same(size::CONTENT_INSET as i8)),
            Surface::Popup => egui::Frame::popup(&style)
                .fill(colors.popup)
                .stroke(Stroke::new(1.0_f32, colors.popup_border))
                .corner_radius(radius::POPUP)
                .inner_margin(Margin::symmetric(0, 4)),
            Surface::Tooltip => egui::Frame::popup(&style)
                .fill(colors.tooltip)
                .stroke(Stroke::new(1.0_f32, colors.popup_border))
                .corner_radius(CornerRadius::same(radius::CONTROL))
                .inner_margin(Margin::symmetric(8, 5)),
            Surface::Dialog => egui::Frame::popup(&style)
                .fill(colors.popup)
                .stroke(Stroke::new(1.0_f32, colors.popup_border))
                .corner_radius(CornerRadius::same(radius::ISLAND))
                .inner_margin(Margin::same(size::DIALOG_PADDING as i8 + 4)),
            Surface::Chrome => egui::Frame::new()
                .fill(colors.frame)
                .inner_margin(Margin::same(size::ISLAND_GAP as i8)),
            Surface::Field => egui::Frame::new()
                .fill(colors.field)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .corner_radius(CornerRadius::same(radius::CONTROL))
                .inner_margin(Margin::symmetric(8, 0)),
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
