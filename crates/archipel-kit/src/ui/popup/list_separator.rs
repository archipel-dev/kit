//! The separator of popup lists.

use crate::common;
use crate::theme::Theme;
use crate::tokens::{popup, typography};
use egui::{Response, Sense, Stroke, Ui, Widget, vec2};

/// A separator between the items of a menu or popup list: a thin line between groups
/// ([`ListSeparator::new`]), or a section title ([`ListSeparator::with_text`]).
///
/// The line is inset like the item pills, in the popup's border color. The title is dimmed
/// semibold text aligned with the items' icons, like "Open Projects" in Android Studio's
/// project picker; it is not focusable.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{action_system::ActionMenuItem, ui::popup::ListSeparator};
///
/// ui.add(ActionMenuItem::new("Open…"));
/// ui.add(ListSeparator::new());
/// ui.add(ListSeparator::with_text("Open Projects"));
/// ui.add(ActionMenuItem::new("archipel").description("~/code/archipel"));
/// # });
/// ```
#[derive(Debug, Clone, Default)]
#[must_use = "add it with `ui.add(separator)`"]
pub struct ListSeparator {
    title: Option<String>,
}

impl ListSeparator {
    /// A thin line between groups of items.
    pub fn new() -> Self {
        Self { title: None }
    }

    /// A group title reading `title`.
    pub fn with_text(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
        }
    }
}

impl ListSeparator {
    /// The group title, if any (IntelliJ `ListSeparator.getText`).
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl Widget for ListSeparator {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let Some(title) = self.title else {
            let width = ui.available_width().max(1.0);
            common::menu_rows(ui);
            let (rect, response) =
                ui.allocate_exact_size(vec2(width, popup::SEPARATOR_HEIGHT), Sense::hover());
            // As in Android Studio: inset like the item pills, in the popup's border color.
            ui.painter().hline(
                (rect.left() + crate::tokens::metrics(ui.ctx()).popup_item_inset)
                    ..=(rect.right() - crate::tokens::metrics(ui.ctx()).popup_item_inset),
                rect.top() + (popup::SEPARATOR_HEIGHT / 2.0).floor() + 0.5,
                Stroke::new(1.0_f32, colors.popup_border),
            );
            return response;
        };
        let galley = common::galley(
            ui,
            title.as_str(),
            typography::semibold(ui.ctx(), typography::BODY),
        );
        let metrics = crate::tokens::metrics(ui.ctx());
        let x = metrics.popup_item_inset + metrics.popup_item_padding;
        let width = ui.available_width().max(galley.size().x + 2.0 * x);
        common::menu_rows(ui);
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, popup::HEADER_HEIGHT), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &title));
        common::text_left(
            ui.painter(),
            rect,
            rect.left() + x,
            galley,
            colors.text_placeholder,
        );
        response
    }
}
