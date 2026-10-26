//! Placeholder text for empty content.

use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::typography;
use crate::ui::Button;
use egui::{Align, Layout, Response, Sense, Ui, vec2};

/// What to show where a list, table or panel has nothing to display: a title, a hint and
/// optionally one action.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, ui::StatusText};
///
/// let out = StatusText::new("No requests yet")
///     .icon(Glyph::Send)
///     .hint("Create a request to start testing your API.")
///     .action("New request")
///     .show(ui);
/// if out.clicked() { /* ... */ }
/// # });
/// ```
pub struct StatusText {
    title: String,
    hint: Option<String>,
    icon: Option<Icon>,
    action: Option<String>,
}

impl StatusText {
    /// An empty state titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            hint: None,
            icon: None,
            action: None,
        }
    }

    /// Explanation below the title: why it is empty and how to fill it.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Large icon above the title.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// One primary action button below the hint.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    /// Draws it centered in the available width; the response is the action button's
    /// (never clicked if there is no action).
    pub fn show(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.add_space(12.0);
            if let Some(icon) = self.icon {
                icons::show(ui, icon, 32.0, colors.text_placeholder);
                ui.add_space(2.0);
            }
            ui.label(
                egui::RichText::new(&self.title)
                    .font(typography::semibold(ui.ctx(), typography::BODY + 1.0))
                    .color(colors.text_strong),
            );
            if let Some(hint) = &self.hint {
                ui.add(
                    egui::Label::new(egui::RichText::new(hint).color(colors.text_secondary)).wrap(),
                );
            }
            let response = match &self.action {
                Some(action) => {
                    ui.add_space(4.0);
                    ui.add(Button::primary(action.as_str()))
                }
                None => ui.allocate_response(vec2(0.0, 0.0), Sense::hover()),
            };
            ui.add_space(12.0);
            response
        })
        .inner
    }
}
