//! Read-only key/value grids.

use crate::theme::Theme;
use egui::{Response, Ui, vec2};

/// Key/value rows with aligned columns, for inspectors and detail panes.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::PropertyGrid;
///
/// PropertyGrid::new("response")
///     .row("Status", "200 OK")
///     .row("Time", "142 ms")
///     .mono_row("Content-Type", "application/json")
///     .show(ui);
/// # });
/// ```
pub struct PropertyGrid {
    id: egui::Id,
    rows: Vec<(String, String, bool)>,
}

impl PropertyGrid {
    /// A grid with a unique `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: egui::Id::new(id_salt),
            rows: Vec::new(),
        }
    }

    /// Adds a row.
    pub fn row(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push((key.into(), value.into(), false));
        self
    }

    /// Adds a row whose value uses the code font (ids, paths, hashes).
    pub fn mono_row(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push((key.into(), value.into(), true));
        self
    }

    /// Draws the grid.
    pub fn show(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        egui::Grid::new(self.id)
            .num_columns(2)
            .spacing(vec2(16.0, 6.0))
            .min_row_height(18.0)
            .show(ui, |ui| {
                for (key, value, mono) in &self.rows {
                    ui.label(egui::RichText::new(key).color(colors.text_secondary));
                    let mut text = egui::RichText::new(value).color(colors.text);
                    if *mono {
                        text = text.monospace();
                    }
                    ui.add(egui::Label::new(text).wrap());
                    ui.end_row();
                }
            })
            .response
    }
}
