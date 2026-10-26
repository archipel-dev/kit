//! The help tooltip.

use crate::common;
use crate::icons::{self, Glyph};
use crate::parts::Surface;
use crate::theme::Theme;
use crate::tokens::{motion, size, typography};
use egui::{Id, Rect, Response, Sense, Ui, vec2};

/// A tooltip: a title, an optional shortcut after it, an optional description below and an
/// optional link. Attach it with [`HelpTooltipExt::tooltip`] or
/// [`HelpTooltipExt::tooltip_with`]; also over disabled widgets (to say why they are
/// disabled).
///
/// It follows IntelliJ's `HelpTooltip` (IntelliJ spec section 11.1):
///
/// - it shows [500 ms](motion::HELP_TOOLTIP_SHOW_DELAY) after the pointer comes to rest on
///   its owner (moving the pointer starts the wait over) and hides
///   [150 ms](motion::HELP_TOOLTIP_HIDE_DELAY) after the pointer leaves;
/// - while the pointer is over the tooltip itself it stays, so its link can be clicked;
/// - it hides by itself after 10 s, or 30 s when it has a description or a link
///   ([`HelpTooltip::never_hide_on_timeout`] keeps it);
/// - pressing the owner hides it until the pointer has left and come back;
/// - the text is at most 250 pt wide.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{action_system::ActionButton, ide::{HelpTooltip, HelpTooltipExt}};
/// use archipel_kit::Glyph;
///
/// ui.add(ActionButton::new(Glyph::Run, "Run"))
///     .tooltip_with(HelpTooltip::new("Run 'gallery'").shortcut("Shift+F10"));
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct HelpTooltip {
    label: String,
    shortcut: Option<String>,
    hint: Option<String>,
    link: Option<(String, bool)>,
    align: Option<egui::RectAlign>,
    immediate: bool,
    never_hide: bool,
    show_delay: Option<f32>,
    hide_delay: Option<f32>,
}

/// What a [`HelpTooltip`] did this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HelpTooltipResponse {
    /// The tooltip is on screen.
    pub visible: bool,
    /// The user clicked the tooltip's link.
    pub link_clicked: bool,
}

/// Where a tooltip is in its life, kept in the context under the owner's id.
#[derive(Debug, Clone, Copy, Default)]
struct State {
    /// When the pointer came to rest on the owner, while the tooltip is not shown.
    waiting_since: Option<f64>,
    /// When the tooltip appeared.
    shown_since: Option<f64>,
    /// When the pointer left both the owner and the tooltip.
    left_since: Option<f64>,
    /// Hidden by a press or the timeout until the pointer leaves.
    suppressed: bool,
    /// Where the tooltip was last frame.
    popup: Option<Rect>,
}

impl HelpTooltip {
    /// A tooltip reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            hint: None,
            link: None,
            align: None,
            immediate: false,
            never_hide: false,
            show_delay: None,
            hide_delay: None,
        }
    }

    /// Shows the tooltip as soon as the widget is hovered, without the usual delay (about
    /// half a second, as in Android Studio). Android Studio's tool window stripe buttons
    /// name themselves at once.
    pub fn immediate(mut self, immediate: bool) -> Self {
        self.immediate = immediate;
        self
    }

    /// Where the tooltip goes relative to its widget (default: below it). Tool window
    /// stripes put it beside the button, `egui::RectAlign::RIGHT`, as Android Studio does.
    pub fn align(mut self, align: egui::RectAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// A shortcut shown after the label, e.g. `"Ctrl+Shift+F"`.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// A longer explanation below the label, wrapped (IntelliJ `setDescription`).
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// A link below the description (IntelliJ `setLink`). Its click is reported by
    /// [`HelpTooltipResponse::link_clicked`]. With `external` the link has the external
    /// icon, for a link that leaves the application.
    pub fn link(mut self, text: impl Into<String>, external: bool) -> Self {
        self.link = Some((text.into(), external));
        self
    }

    /// Keeps the tooltip on screen as long as the pointer is over its owner, without the
    /// 10 s / 30 s timeout (IntelliJ `setNeverHideOnTimeout`).
    pub fn never_hide_on_timeout(mut self, never: bool) -> Self {
        self.never_hide = never;
        self
    }

    /// Seconds the pointer must rest on the owner before the tooltip shows (default
    /// [`motion::HELP_TOOLTIP_SHOW_DELAY`]; IntelliJ `setInitialDelay`).
    pub fn show_delay(mut self, seconds: f32) -> Self {
        self.show_delay = Some(seconds);
        self
    }

    /// Seconds the tooltip stays after the pointer left (default
    /// [`motion::HELP_TOOLTIP_HIDE_DELAY`]; IntelliJ `setHideDelay`).
    pub fn hide_delay(mut self, seconds: f32) -> Self {
        self.hide_delay = Some(seconds);
        self
    }

    /// Whether the tooltip has more than a line: a description or a link. Such a tooltip
    /// stays 30 s instead of 10.
    pub fn is_multiline(&self) -> bool {
        self.hint.is_some() || self.link.is_some()
    }

    /// Seconds after which the tooltip hides by itself, `None` when it never does.
    pub fn dismiss_delay(&self) -> Option<f32> {
        if self.never_hide {
            None
        } else if self.is_multiline() {
            Some(motion::HELP_TOOLTIP_DISMISS_MULTI_LINE)
        } else {
            Some(motion::HELP_TOOLTIP_DISMISS_SINGLE_LINE)
        }
    }

    /// The tooltip frame: the tooltip color, popup border and radius.
    pub fn frame(ctx: &egui::Context) -> egui::Frame {
        Surface::Tooltip.frame(ctx)
    }

    /// Draws the content (without the frame) into `ui`: the title and the shortcut in a
    /// row, the description, the link. Use it to show a tooltip outside of hover, e.g. in a
    /// snapshot or an onboarding hint. Returns whether the link was clicked.
    pub fn content(&self, ui: &mut Ui) -> bool {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        let mut link_clicked = false;
        ui.spacing_mut().item_spacing = vec2(size::TOOLTIP_SHORTCUT_GAP, size::HELP_TOOLTIP_GAP);
        // One text line high, not a row of controls.
        ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
        ui.horizontal(|ui| {
            let label = common::galley(ui, self.label.as_str(), font.clone());
            let (rect, response) = ui.allocate_exact_size(label.size(), Sense::hover());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.label)
            });
            common::text(ui.painter(), rect.min, label, colors.text);
            if let Some(shortcut) = &self.shortcut {
                let shortcut = common::galley(ui, shortcut.as_str(), font.clone());
                let (rect, _) = ui.allocate_exact_size(shortcut.size(), Sense::hover());
                common::text(ui.painter(), rect.min, shortcut, colors.text_secondary);
            }
        });
        if let Some(hint) = &self.hint {
            ui.set_max_width(size::HELP_TOOLTIP_MAX_WIDTH);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(hint)
                        .font(typography::body(typography::BODY))
                        .color(colors.text_secondary),
                )
                .wrap(),
            );
        }
        if let Some((text, external)) = &self.link {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = size::ICON_TEXT_GAP;
                link_clicked = ui.add(crate::ui::ActionLink::new(text.as_str())).clicked();
                if *external {
                    icons::show(
                        ui,
                        Glyph::ExternalLink,
                        size::HELP_TOOLTIP_LINK_ICON,
                        colors.link,
                    );
                }
            });
        }
        link_clicked
    }

    /// Shows the tooltip for `response` when it is due, and reports it.
    ///
    /// Call it every frame with the owner's response: the delays are counted from the
    /// frames it sees.
    pub fn show_for(&self, response: &Response) -> HelpTooltipResponse {
        let ctx = &response.ctx;
        let key = Id::new(("archipel-help-tooltip", response.id));
        let mut state = ctx.data(|d| d.get_temp::<State>(key)).unwrap_or_default();
        let now = ctx.input(|i| i.time);
        let (pointer, pressed, moved) = ctx.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.any_pressed(),
                i.pointer.delta() != egui::Vec2::ZERO,
            )
        });
        let dragging = ctx.dragged_id().is_some();
        let over_owner = response.contains_pointer() && !dragging;
        let over_popup = state
            .popup
            .zip(pointer)
            .is_some_and(|(popup, pointer)| popup.contains(pointer));
        let show_delay = f64::from(self.show_delay.unwrap_or(motion::HELP_TOOLTIP_SHOW_DELAY));
        let hide_delay = f64::from(self.hide_delay.unwrap_or(motion::HELP_TOOLTIP_HIDE_DELAY));
        let dismiss = self.dismiss_delay().map(f64::from);

        if over_owner && pressed {
            // A press on the owner ends the tooltip until the pointer comes back.
            state.shown_since = None;
            state.waiting_since = None;
            state.suppressed = true;
        }
        if over_owner || over_popup {
            state.left_since = None;
            if over_owner && state.shown_since.is_none() && !state.suppressed {
                // Moving the pointer starts the wait over: it has to come to rest.
                if state.waiting_since.is_none() || moved {
                    state.waiting_since = Some(now);
                }
                if self.immediate || state.waiting_since.is_some_and(|t| now - t >= show_delay) {
                    state.shown_since = Some(now);
                    state.waiting_since = None;
                }
            }
            if let (Some(since), Some(dismiss)) = (state.shown_since, dismiss)
                && now - since >= dismiss
            {
                state.shown_since = None;
                state.suppressed = true;
            }
        } else {
            state.waiting_since = None;
            state.suppressed = false;
            if state.shown_since.is_some() {
                let since = *state.left_since.get_or_insert(now);
                if now - since >= hide_delay {
                    state.shown_since = None;
                    state.left_since = None;
                }
            }
        }

        let mut out = HelpTooltipResponse::default();
        let open = state.shown_since.is_some();
        let mut tooltip = egui::Tooltip::for_widget(response);
        tooltip.popup = tooltip
            .popup
            .open(open)
            .frame(Self::frame(ctx))
            .gap(size::POPUP_PADDING);
        // Placed where it fits on screen without covering its owner or the pointer, trying the
        // preferred side, its mirror, then the menu placements; with no room for any of them
        // it goes below (or above) the pointer, so it never blocks a click.
        let expected = tooltip.popup.get_expected_size();
        let owner = response.interact_rect;
        match expected.map(|size| placement(ctx, owner, pointer, size, self.align)) {
            Some(Placement::Align(align)) => {
                tooltip.popup = tooltip.popup.align(align).align_alternatives(&[]);
            }
            Some(Placement::At(pos)) => {
                tooltip.popup = tooltip
                    .popup
                    .anchor(egui::PopupAnchor::Position(pos))
                    .align(egui::RectAlign {
                        parent: egui::Align2::LEFT_TOP,
                        child: egui::Align2::LEFT_TOP,
                    })
                    .align_alternatives(&[])
                    .gap(0.0);
            }
            None => {
                if let Some(align) = self.align {
                    tooltip.popup = tooltip.popup.align(align);
                }
            }
        }
        state.popup = tooltip.show(|ui| self.content(ui)).map(|shown| {
            out.link_clicked = shown.inner;
            shown.response.rect
        });
        out.visible = open;

        // Wake up when the next delay runs out.
        let mut wake = f64::INFINITY;
        if let Some(since) = state.waiting_since {
            wake = wake.min(show_delay - (now - since));
        }
        if let Some(since) = state.shown_since {
            if let Some(dismiss) = dismiss {
                wake = wake.min(dismiss - (now - since));
            }
            if let Some(left) = state.left_since {
                wake = wake.min(hide_delay - (now - left));
            }
        }
        if wake.is_finite() {
            ctx.request_repaint_after_secs(wake.max(0.0) as f32);
        }
        ctx.data_mut(|d| d.insert_temp(key, state));
        out
    }
}

/// Kit tooltips on any [`Response`].
pub trait HelpTooltipExt {
    /// A tooltip reading `label`.
    fn tooltip(self, label: impl Into<String>) -> Self;
    /// A tooltip with a shortcut or hint.
    fn tooltip_with(self, tooltip: HelpTooltip) -> Self;
    /// Like [`HelpTooltipExt::tooltip_with`], also telling what the tooltip did (whether
    /// it is visible, whether its link was clicked).
    fn tooltip_with_response(self, tooltip: HelpTooltip) -> (Self, HelpTooltipResponse)
    where
        Self: Sized;
}

impl HelpTooltipExt for Response {
    fn tooltip(self, label: impl Into<String>) -> Self {
        self.tooltip_with(HelpTooltip::new(label))
    }

    fn tooltip_with(self, tooltip: HelpTooltip) -> Self {
        tooltip.show_for(&self);
        self
    }

    fn tooltip_with_response(self, tooltip: HelpTooltip) -> (Self, HelpTooltipResponse) {
        let shown = tooltip.show_for(&self);
        (self, shown)
    }
}

/// Where a tooltip goes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Placement {
    /// Beside its owner.
    Align(egui::RectAlign),
    /// At this top-left corner, clear of the pointer.
    At(egui::Pos2),
}

/// Where a tooltip of `size` for `owner` goes: the first of `preferred` (default below the
/// owner), its mirror images and the menu placements that fits on screen without covering
/// the owner or the pointer; else beside the pointer.
fn placement(
    ctx: &egui::Context,
    owner: egui::Rect,
    pointer: Option<egui::Pos2>,
    size: egui::Vec2,
    preferred: Option<egui::RectAlign>,
) -> Placement {
    let screen = ctx.content_rect();
    let gap = size::POPUP_PADDING;
    let preferred = preferred.unwrap_or(egui::RectAlign::BOTTOM_START);
    let clear = |rect: egui::Rect| {
        screen.contains_rect(rect)
            && !rect.intersects(owner.shrink(0.5))
            && !pointer.is_some_and(|p| rect.expand(1.0).contains(p))
    };
    let candidates = std::iter::once(preferred)
        .chain(preferred.symmetries())
        .chain(egui::RectAlign::MENU_ALIGNS);
    for align in candidates {
        if clear(align.align_rect(&owner, size, gap)) {
            return Placement::Align(align);
        }
    }
    // No side fits: below the pointer (past the cursor's height), else above it, kept on
    // screen sideways.
    let at = pointer.unwrap_or(owner.center());
    let x = (at.x - size.x / 2.0)
        .min(screen.right() - size.x)
        .max(screen.left());
    let below = at.y + size::TOOLTIP_POINTER_CLEARANCE;
    let y = if below + size.y <= screen.bottom() || at.y - gap - size.y < screen.top() {
        below
    } else {
        at.y - gap - size.y
    };
    Placement::At(egui::pos2(x, y))
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    #[test]
    fn tooltip_never_covers_the_pointer_or_its_owner() {
        let ctx = egui::Context::default();
        // A small window: a stripe button at the left edge, a wide tooltip to its right.
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(200.0, 120.0),
                )),
                ..Default::default()
            },
            |_| {},
        );
        let owner = egui::Rect::from_min_size(egui::pos2(4.0, 40.0), egui::vec2(30.0, 30.0));
        let pointer = Some(owner.center());
        let size = egui::vec2(190.0, 36.0);
        let at = placement(&ctx, owner, pointer, size, Some(egui::RectAlign::RIGHT));
        let rect = match at {
            Placement::Align(align) => align.align_rect(&owner, size, size::POPUP_PADDING),
            Placement::At(pos) => egui::Rect::from_min_size(pos, size),
        };
        assert!(
            !rect.contains(owner.center()),
            "{rect:?} covers the pointer"
        );
        assert!(
            !rect.intersects(owner.shrink(0.5)),
            "{rect:?} covers the owner"
        );
    }

    #[test]
    fn tooltip_keeps_the_preferred_side_when_it_fits() {
        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            },
            |_| {},
        );
        let owner = egui::Rect::from_min_size(egui::pos2(4.0, 40.0), egui::vec2(30.0, 30.0));
        let at = placement(
            &ctx,
            owner,
            Some(owner.center()),
            egui::vec2(120.0, 36.0),
            Some(egui::RectAlign::RIGHT),
        );
        assert_eq!(at, Placement::Align(egui::RectAlign::RIGHT));
    }
}
