//! A titled group that can be collapsed: IntelliJ's `CollapsiblePanel`.

use crate::ui::TitledSeparator;
use egui::{Response, Ui};

/// What [`CollapsiblePanel::show`] reports.
#[derive(Debug)]
pub struct CollapsiblePanelResponse<R> {
    /// The header row's response; `changed()` when it was toggled this frame.
    pub header: Response,
    /// What the content returned, `None` while the panel is collapsed.
    pub inner: Option<R>,
}

impl<R> CollapsiblePanelResponse<R> {
    /// The panel was collapsed or expanded this frame.
    pub fn toggled(&self) -> bool {
        self.header.changed()
    }
}

/// A header with a chevron and a title that shows and hides its content (`CollapsiblePanel`;
/// the "Advanced settings" of a form).
///
/// The caller owns the collapsed flag. A click on the header, <kbd>Enter</kbd> or
/// <kbd>Space</kbd> on it when focused toggle it. The header is a
/// [`TitledSeparator`] with the chevron before the title, or at the end of the line with
/// [`collapse_button_at_left(false)`](Self::collapse_button_at_left).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::CollapsiblePanel;
///
/// let mut collapsed = true;
/// CollapsiblePanel::new("Advanced", &mut collapsed).show(ui, |ui| {
///     ui.label("Only shown while expanded");
/// });
/// # });
/// ```
#[must_use = "show it with `panel.show(ui, content)`"]
pub struct CollapsiblePanel<'a> {
    title: String,
    collapsed: &'a mut bool,
    button_at_left: bool,
}

impl<'a> CollapsiblePanel<'a> {
    /// A panel titled `title`, collapsed while `*collapsed`.
    pub fn new(title: impl Into<String>, collapsed: &'a mut bool) -> Self {
        Self {
            title: title.into(),
            collapsed,
            button_at_left: true,
        }
    }

    /// Whether the chevron sits before the title (the default) or at the right end of the
    /// header (`collapseButtonAtLeft`).
    pub fn collapse_button_at_left(mut self, at_left: bool) -> Self {
        self.button_at_left = at_left;
        self
    }

    /// Draws the header and, while expanded, `content`.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> CollapsiblePanelResponse<R> {
        let mut open = !*self.collapsed;
        let header = ui.add(
            TitledSeparator::new(self.title)
                .collapsible(&mut open)
                .chevron_at_end(!self.button_at_left),
        );
        *self.collapsed = !open;
        let inner = open.then(|| content(ui));
        CollapsiblePanelResponse { header, inner }
    }
}
