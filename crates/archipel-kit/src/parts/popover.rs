//! A popup attached to a widget.

use super::Surface;
use crate::tokens::{popup, size};
use egui::{Id, Key, Rect};

/// A popup attached below a widget (above it when there is no room), on the popup
/// [`Surface`]. The caller owns `open`; the popover clears it on <kbd>Escape</kbd> or a
/// click outside the popup and its anchor. Build pickers, filters and suggestion lists on
/// it instead of on raw areas, and they close, layer and look like every other popup.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Button, MenuItem};
/// use archipel_kit::parts::Popover;
///
/// let mut open = false;
/// let button = ui.add(Button::secondary("Filter"));
/// if button.clicked() {
///     open = !open;
/// }
/// Popover::below("filter", &button).width(200.0).show(ui.ctx(), &mut open, |ui| {
///     ui.add(MenuItem::new("Errors only").checked(true));
/// });
/// # });
/// ```
#[must_use = "show it with `popover.show(ctx, &mut open, content)`"]
pub struct Popover {
    id: Id,
    anchor: Rect,
    width: Option<f32>,
    beside: bool,
}

impl Popover {
    /// A popover with a unique `id_salt` attached to `anchor`.
    pub fn new(id_salt: impl std::hash::Hash, anchor: Rect) -> Self {
        Self {
            id: Id::new(id_salt),
            anchor,
            width: None,
            beside: false,
        }
    }

    /// A popover attached to the right of `response`'s widget (to its left when there is no
    /// room), aligned with its top edge: submenus.
    pub fn right_of(id_salt: impl std::hash::Hash, response: &egui::Response) -> Self {
        Self {
            beside: true,
            ..Self::new(id_salt, response.rect)
        }
    }

    /// A popover attached below `response`'s widget.
    pub fn below(id_salt: impl std::hash::Hash, response: &egui::Response) -> Self {
        Self::new(id_salt, response.rect)
    }

    /// Width of the content (default: the anchor's width, at least 160).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Shows the popover while `*open`, and closes it on Escape or a click outside.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        add: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        if !*open {
            return None;
        }
        let width = self.width.unwrap_or(self.anchor.width().max(160.0));
        // Below the anchor, or above it when the popup would leave the screen.
        let last_height: f32 = ctx.data(|d| d.get_temp(self.id)).unwrap_or(0.0);
        let screen = ctx.content_rect();
        let below = self.anchor.left_bottom() + egui::vec2(0.0, size::POPUP_PADDING);
        let pos = if self.beside {
            // The popup's own padding: line its first row up with the anchor.
            let x = if self.anchor.right() + width > screen.right() && self.anchor.left() > width {
                self.anchor.left() - width
            } else {
                self.anchor.right()
            };
            let y = (self.anchor.top() - popup::PADDING)
                .min((screen.bottom() - last_height).max(screen.top()));
            egui::pos2(x, y)
        } else if below.y + last_height > screen.bottom() && self.anchor.top() > last_height {
            self.anchor.left_top() - egui::vec2(0.0, last_height + size::POPUP_PADDING)
        } else {
            below
        };
        let shown = egui::Area::new(self.id)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                Surface::Popup.frame(ctx).show(ui, |ui| {
                    ui.set_width(width);
                    // Popover content is rows (menus, completions): they touch, as in menus.
                    ui.spacing_mut().item_spacing.y = 0.0;
                    add(ui)
                })
            });
        let rect = shown.response.rect;
        ctx.data_mut(|d| d.insert_temp(self.id, rect.height()));
        let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
        let clicked_outside = ctx.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !rect.contains(p) && !self.anchor.contains(p))
        });
        if escape || clicked_outside {
            *open = false;
        }
        Some(shown.inner.inner)
    }
}
