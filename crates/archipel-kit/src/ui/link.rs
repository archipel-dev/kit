//! The inline action link.

use crate::common;
use crate::icons::{self, Glyph};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{
    CursorIcon, Rect, Response, Sense, Stroke, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

/// A text link (IntelliJ `ActionLink` / `HyperlinkLabel`): link-colored text, underlined on
/// hover only, with the hand cursor.
///
/// - Click performs it **on release** (primary button); <kbd>Space</kbd> and <kbd>Enter</kbd>
///   do when it has focus, which shows the focus ring.
/// - Disabled links are dimmed, without underline or hand cursor, and ignore input.
/// - [`external`](Self::external) adds the external-link icon after the text.
///   [`dropdown`](Self::dropdown) adds a drop-down arrow: the link opens a popup (open it
///   from [`Response::clicked`], e.g. with an [`ActionPopupMenu`](crate::action_system::ActionPopupMenu)).
/// - Hover, pressed and visited states keep the link color (IntelliJ's New UI uses the same
///   blue for `Link.activeForeground`, `hoverForeground` and `pressedForeground`: only the
///   underline marks hover).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::ActionLink;
///
/// if ui.add(ActionLink::new("Configure…")).clicked() { /* ... */ }
/// ui.add(ActionLink::new("Documentation").external(true));
/// # });
/// ```
#[must_use = "add it with `ui.add(link)`"]
pub struct ActionLink {
    text: String,
    enabled: bool,
    external: bool,
    dropdown: bool,
}

impl ActionLink {
    /// A link reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            enabled: true,
            external: false,
            dropdown: false,
        }
    }

    /// Disabled links are dimmed and ignore input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// An external-link icon after the text (IntelliJ `setExternalLinkIcon`).
    pub fn external(mut self, external: bool) -> Self {
        self.external = external;
        self
    }

    /// A drop-down arrow after the text: the link opens a popup.
    pub fn dropdown(mut self, dropdown: bool) -> Self {
        self.dropdown = dropdown;
        self
    }
}

impl Widget for ActionLink {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(ui, self.text.as_str(), typography::body(typography::BODY));
        let icon = if self.external || self.dropdown {
            size::ICON
        } else {
            0.0
        };
        let text_size = galley.size();
        let total = vec2(text_size.x + icon, text_size.y.max(icon));
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(total, sense);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, self.enabled, &self.text));
        if self.enabled {
            crate::cursor::on_hover(&response, CursorIcon::PointingHand);
        }
        let color = if self.enabled {
            colors.link
        } else {
            colors.text_disabled
        };
        let text_rect = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - text_size.y / 2.0),
            text_size,
        );
        if self.enabled && response.hovered() {
            ui.painter().hline(
                text_rect.x_range(),
                text_rect.bottom() - 1.0,
                Stroke::new(1.0_f32, color),
            );
        }
        if self.enabled && response.has_focus() {
            common::focus_ring(ui.painter(), rect, 2, &colors);
        }
        common::text(ui.painter(), text_rect.min, galley, color);
        if icon > 0.0 {
            let glyph = if self.dropdown {
                Glyph::ChevronDown
            } else {
                Glyph::ExternalLink
            };
            let icon_rect = Rect::from_center_size(
                pos2(rect.right() - icon / 2.0, rect.center().y),
                vec2(icon, icon),
            );
            icons::paint(ui.painter(), icon_rect, glyph, color);
        }
        response
    }
}
