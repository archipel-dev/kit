//! The help tooltip.

use crate::common;
use crate::parts::Surface;
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{Response, Sense, Ui, vec2};

/// A tooltip: a label, an optional shortcut after it, and an optional hint below, on the
/// tooltip color. Attach it with [`HelpTooltipExt::tooltip`] or [`HelpTooltipExt::tooltip_with`];
/// it shows after the hover delay, also over disabled widgets (to say why they are
/// disabled).
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
    align: Option<egui::RectAlign>,
    immediate: bool,
}

impl HelpTooltip {
    /// A tooltip reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            hint: None,
            align: None,
            immediate: false,
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

    /// A longer explanation below the label, wrapped.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The tooltip frame: the tooltip color, popup border and radius.
    pub fn frame(ctx: &egui::Context) -> egui::Frame {
        Surface::Tooltip.frame(ctx)
    }

    /// Draws the content (without the frame) into `ui`. Use it to show a tooltip outside
    /// of hover, e.g. in a snapshot or an onboarding hint.
    pub fn content(&self, ui: &mut Ui) {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        ui.spacing_mut().item_spacing = vec2(size::TOOLTIP_SHORTCUT_GAP, 3.0);
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
            ui.set_max_width(280.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(hint)
                        .font(typography::body(typography::BODY))
                        .color(colors.text_secondary),
                )
                .wrap(),
            );
        }
    }

    /// Shows the tooltip for `response` when it is hovered long enough.
    pub fn show_for(&self, response: &Response) {
        let mut tooltip = if self.immediate {
            let mut tooltip = egui::Tooltip::for_widget(response);
            let dragging = response.ctx.dragged_id().is_some();
            tooltip.popup = tooltip.popup.open(response.hovered() && !dragging);
            tooltip
        } else if response.enabled() {
            egui::Tooltip::for_enabled(response)
        } else {
            egui::Tooltip::for_disabled(response)
        };
        tooltip.popup = tooltip.popup.frame(Self::frame(&response.ctx));
        if let Some(align) = self.align {
            tooltip.popup = tooltip.popup.align(align).gap(size::POPUP_PADDING);
        }
        tooltip.show(|ui| self.content(ui));
    }
}

/// Kit tooltips on any [`Response`].
pub trait HelpTooltipExt {
    /// A tooltip reading `label`.
    fn tooltip(self, label: impl Into<String>) -> Self;
    /// A tooltip with a shortcut or hint.
    fn tooltip_with(self, tooltip: HelpTooltip) -> Self;
}

impl HelpTooltipExt for Response {
    fn tooltip(self, label: impl Into<String>) -> Self {
        self.tooltip_with(HelpTooltip::new(label))
    }

    fn tooltip_with(self, tooltip: HelpTooltip) -> Self {
        tooltip.show_for(&self);
        self
    }
}
