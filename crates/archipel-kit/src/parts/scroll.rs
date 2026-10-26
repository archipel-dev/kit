//! The scroll bar of collections: a thin thumb that dims and brightens with the timing of
//! IntelliJ's `JBScrollBar` (spec section 13.1).
//!
//! egui draws its own scroll bars; [`ScrollBar`] hides them and draws the thumb itself, so
//! the hover fade can wait 150 ms before it starts and 300 ms before it ends, as in the
//! IDE. Width, margin and colors are the measured ones of [`tokens::scroll`].
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use archipel_kit::parts::ScrollBar;
//!
//! let bar = ScrollBar::new("log");
//! let area = bar.prepare(ui, egui::ScrollArea::vertical().id_salt("log"));
//! let output = area.show(ui, |ui| {
//!     for line in 0..200 {
//!         ui.label(format!("line {line}"));
//!     }
//! });
//! bar.show(ui, &output);
//! # });
//! ```

use crate::theme::Theme;
use crate::tokens::{scroll, scroll_bar};
use egui::scroll_area::{ScrollAreaOutput, ScrollBarVisibility};
use egui::{CornerRadius, Id, Rect, ScrollArea, Sense, Ui, pos2, vec2};

/// What the bar remembers between frames, kept in the `egui::Context`.
#[derive(Clone, Copy, Debug)]
struct BarState {
    /// The pointer is over the bar's strip (or drags the thumb).
    inside: bool,
    /// When `inside` last changed, in seconds of `egui` time.
    changed_at: f64,
    /// The fade factor (0 at rest, 1 hovered) at that moment.
    from: f32,
    /// Where in the thumb it was grabbed (distance from the thumb's top), while dragging.
    grab: Option<f32>,
    /// A scroll offset to apply on the next frame (a drag or a click on the track).
    pending: Option<f32>,
}

impl Default for BarState {
    fn default() -> Self {
        Self {
            inside: false,
            changed_at: f64::NEG_INFINITY,
            from: 0.0,
            grab: None,
            pending: None,
        }
    }
}

impl BarState {
    fn factor(&self, now: f64) -> f32 {
        let (target, delay) = if self.inside {
            (1.0, scroll_bar::ENTER_DELAY)
        } else {
            (0.0, scroll_bar::LEAVE_DELAY)
        };
        let elapsed = (now - self.changed_at) as f32;
        let t = ((elapsed - delay) / scroll_bar::FADE).clamp(0.0, 1.0);
        self.from + (target - self.from) * t
    }

    /// Whether the fade is still running or waiting to start.
    fn animating(&self, now: f64) -> bool {
        let delay = if self.inside {
            scroll_bar::ENTER_DELAY
        } else {
            scroll_bar::LEAVE_DELAY
        };
        ((now - self.changed_at) as f32) < delay + scroll_bar::FADE
    }
}

/// A vertical scroll bar for an [`egui::ScrollArea`], drawn like IntelliJ's: a thin thumb
/// at the right edge that brightens when the pointer stays over the bar.
///
/// Call [`prepare`](Self::prepare) on the area before it is shown and [`show`](Self::show)
/// with its output after. The thumb can be dragged; a click on the track above or below it
/// scrolls one page.
#[derive(Debug, Clone, Copy)]
pub struct ScrollBar {
    id: Id,
}

impl ScrollBar {
    /// A bar identified by `id_salt` (unique per scroll area).
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(id_salt).with("scroll-bar"),
        }
    }

    /// The same bar as another one with this id (use [`ScrollBar::new`] with the same salt).
    pub fn id(&self) -> Id {
        self.id
    }

    /// Hides egui's own bars and applies what the bar asked for last frame (a dragged
    /// thumb, a page jump).
    pub fn prepare(&self, ui: &Ui, area: ScrollArea) -> ScrollArea {
        let area = area.scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden);
        let pending = ui.ctx().data_mut(|d| {
            d.get_temp::<BarState>(self.id).and_then(|mut state| {
                let pending = state.pending.take();
                d.insert_temp(self.id, state);
                pending
            })
        });
        match pending {
            Some(offset) => area.vertical_scroll_offset(offset),
            None => area,
        }
    }

    /// How bright the thumb is: 0 at rest, 1 fully hovered, following the fade timing. For
    /// tests and for painting matching decorations.
    pub fn hover_factor(ctx: &egui::Context, bar: Id) -> f32 {
        let now = ctx.input(|i| i.time);
        ctx.data(|d| d.get_temp::<BarState>(bar))
            .map_or(0.0, |state| state.factor(now))
    }

    /// Draws the thumb for `output` and handles pointer input on the bar.
    pub fn show<R>(&self, ui: &mut Ui, output: &ScrollAreaOutput<R>) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let mut state: BarState = ctx.data(|d| d.get_temp(self.id)).unwrap_or_default();
        let viewport = output.inner_rect;
        let content = output.content_size.y;
        let page = viewport.height();
        let max_offset = (content - page).max(0.0);
        if !(max_offset > 0.5 && page > 0.0) {
            // Nothing to scroll.
            state.inside = false;
            state.from = 0.0;
            state.grab = None;
            ctx.data_mut(|d| d.insert_temp(self.id, state));
            return;
        }
        let colors = Theme::colors(&ctx);
        let offset = output.state.offset.y.clamp(0.0, max_offset);
        let thumb_len = (page * page / content).clamp(scroll_bar::MIN_THUMB.min(page), page);
        let travel = (page - thumb_len).max(0.0);
        let strip = Rect::from_min_max(
            pos2(
                viewport.right() - scroll::THUMB - 2.0 * scroll::MARGIN,
                viewport.top(),
            ),
            viewport.right_bottom(),
        );
        let thumb_top = |offset: f32| viewport.top() + offset / max_offset * travel;
        let response = ui.interact(strip, self.id, Sense::click_and_drag());

        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let origin = ctx.input(|i| i.pointer.press_origin());
        let mut shown_offset = offset;
        if response.drag_started()
            && let Some(origin) = origin
        {
            let top = thumb_top(offset);
            if origin.y >= top && origin.y <= top + thumb_len {
                state.grab = Some(origin.y - top);
            }
        }
        if !response.dragged() {
            state.grab = None;
        }
        if let (Some(grab), Some(pointer)) = (state.grab, pointer)
            && travel > 0.0
        {
            let top = (pointer.y - grab - viewport.top()).clamp(0.0, travel);
            let wanted = top / travel * max_offset;
            shown_offset = wanted;
            state.pending = Some(wanted);
            ctx.request_repaint();
        } else if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            // A click on the track pages toward the pointer.
            let top = thumb_top(offset);
            let wanted = if pointer.y < top {
                offset - page
            } else if pointer.y > top + thumb_len {
                offset + page
            } else {
                offset
            };
            state.pending = Some(wanted.clamp(0.0, max_offset));
            ctx.request_repaint();
        }

        let inside = response.hovered() || state.grab.is_some();
        if inside != state.inside {
            state.from = state.factor(now);
            state.changed_at = now;
            state.inside = inside;
        }
        let factor = state.factor(now);
        if state.animating(now) {
            ctx.request_repaint();
        }
        ctx.data_mut(|d| d.insert_temp(self.id, state));

        if ui.is_rect_visible(strip) {
            let opacity = scroll::THUMB_OPACITY
                + (scroll::THUMB_HOVER_OPACITY - scroll::THUMB_OPACITY) * factor;
            let top = thumb_top(shown_offset);
            let thumb = Rect::from_min_size(
                pos2(viewport.right() - scroll::MARGIN - scroll::THUMB, top),
                vec2(scroll::THUMB, thumb_len),
            );
            ui.painter().rect_filled(
                thumb,
                CornerRadius::same((scroll::THUMB / 2.0) as u8),
                colors.text.gamma_multiply(opacity),
            );
        }
    }
}
