//! The inline action link.

use crate::common;
use crate::theme::Theme;
use crate::tokens::typography;
use egui::{CursorIcon, Response, Sense, Stroke, Ui, Widget, WidgetInfo, WidgetType};

/// A text link: underlined on hover, pointer cursor.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::ActionLink;
///
/// if ui.add(ActionLink::new("Configure…")).clicked() { /* ... */ }
/// # });
/// ```
#[must_use = "add it with `ui.add(link)`"]
pub struct ActionLink {
    text: String,
}

impl ActionLink {
    /// A link reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for ActionLink {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(ui, self.text.as_str(), typography::body(typography::BODY));
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &self.text));
        crate::cursor::on_hover(&response, CursorIcon::PointingHand);
        if response.hovered() {
            ui.painter().hline(
                rect.x_range(),
                rect.bottom() - 1.0,
                Stroke::new(1.0_f32, colors.link),
            );
        }
        if response.has_focus() {
            common::focus_ring(ui.painter(), rect, 2, &colors);
        }
        common::text(ui.painter(), rect.min, galley, colors.link);
        response
    }
}
