//! List and tree rows.

use super::common;
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{Color32, Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// What happened to a [`ListRow`] this frame.
pub struct RowResponse {
    /// The row's response: `clicked()`, `double_clicked()`, `secondary_clicked()` for a
    /// context menu, `context_menu(...)`.
    pub response: Response,
    /// The expander chevron was clicked (or the row was double-clicked, or Left/Right was
    /// pressed while it had focus); the caller flips the node's expanded state.
    pub toggled: bool,
}

/// One row of a list or tree: chevron, icon, label, secondary text and a trailing slot.
/// The whole row is the click target; selection and hover draw a rounded pill.
///
/// Rows are stateless: the caller owns selection and expansion, so the same row works for
/// a flat list, a lazy tree or a virtualized table.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Icon, components::ListRow};
///
/// let mut expanded = true;
/// let mut selected = 1;
/// let row = ListRow::new("src")
///     .icon(Icon::Folder)
///     .expanded(Some(expanded))
///     .selected(selected == 0)
///     .show(ui);
/// if row.toggled { expanded = !expanded; }
/// if row.response.clicked() { selected = 0; }
///
/// if expanded {
///     let row = ListRow::new("main.rs")
///         .icon(Icon::for_file("main.rs"))
///         .depth(1)
///         .secondary("12 KB")
///         .selected(selected == 1)
///         .show(ui);
///     if row.response.clicked() { selected = 1; }
/// }
/// # });
/// ```
pub struct ListRow {
    label: String,
    secondary: Option<String>,
    icon: Option<Icon>,
    icon_color: Option<Color32>,
    label_color: Option<Color32>,
    depth: usize,
    expanded: Option<bool>,
    selected: bool,
    enabled: bool,
}

impl ListRow {
    /// A row showing `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            secondary: None,
            icon: None,
            icon_color: None,
            label_color: None,
            depth: 0,
            expanded: None,
            selected: false,
            enabled: true,
        }
    }

    /// Dimmed text after the label: a path, a size, a type.
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Icon color (default: the theme's icon color).
    pub fn icon_color(mut self, color: impl Into<Color32>) -> Self {
        self.icon_color = Some(color.into());
        self
    }

    /// Label color, e.g. `vcs_modified` for a changed file.
    pub fn label_color(mut self, color: impl Into<Color32>) -> Self {
        self.label_color = Some(color.into());
        self
    }

    /// Tree depth: indents the row by one level per step.
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// `Some(expanded)` draws an expander chevron (a tree node with children); `None`
    /// reserves its space so leaves align with their siblings.
    pub fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }

    /// Highlights the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disabled rows are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the row across the available width.
    pub fn show(self, ui: &mut Ui) -> RowResponse {
        let colors = Theme::colors(ui.ctx());
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), size::ROW_HEIGHT), sense);
        response.widget_info(|| {
            let mut info = WidgetInfo::selected(
                WidgetType::SelectableLabel,
                self.enabled,
                self.selected,
                &self.label,
            );
            if let Some(secondary) = &self.secondary {
                info.label = Some(format!("{} {secondary}", self.label));
            }
            info
        });

        let mut toggled = false;
        let chevron = Rect::from_min_size(
            pos2(
                rect.left() + size::indent(self.depth) - size::ROW_PILL_INSET,
                rect.center().y - size::CHEVRON_COLUMN / 2.0,
            ),
            vec2(size::CHEVRON_COLUMN, size::CHEVRON_COLUMN),
        );
        if let Some(expanded) = self.expanded {
            let chevron_response =
                ui.interact(chevron.expand(2.0), response.id.with("chevron"), sense);
            toggled = chevron_response.clicked() || response.double_clicked();
            if response.has_focus() {
                let key = if expanded {
                    egui::Key::ArrowLeft
                } else {
                    egui::Key::ArrowRight
                };
                toggled |= ui.input(|input| input.key_pressed(key));
            }
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let fill = if self.selected {
                colors.selection
            } else if self.enabled && response.hovered() {
                colors.hover
            } else {
                Color32::TRANSPARENT
            };
            common::row_pill(painter, rect, fill);
            if response.has_focus() && !self.selected {
                common::focus_ring(
                    painter,
                    rect.shrink2(vec2(size::ROW_PILL_INSET + 1.0, 1.0)),
                    4,
                    &colors,
                );
            }
            let mut x = chevron.right();
            if let Some(expanded) = self.expanded {
                let icon = if expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                };
                icons::paint(painter, chevron, icon, colors.text_secondary);
            }
            if let Some(icon) = self.icon {
                let icon_rect = Rect::from_min_size(
                    pos2(x + 2.0, rect.center().y - size::ICON / 2.0),
                    vec2(size::ICON, size::ICON),
                );
                let color = if self.enabled {
                    self.icon_color.unwrap_or(colors.icon)
                } else {
                    colors.text_disabled
                };
                icons::paint(painter, icon_rect, icon, color);
                x = icon_rect.right() + size::ICON_TEXT_GAP;
            } else {
                x += 2.0;
            }
            let label_color = if !self.enabled {
                colors.text_disabled
            } else if let Some(color) = self.label_color {
                color
            } else if self.selected {
                colors.text_strong
            } else {
                colors.text
            };
            let font = typography::body(typography::BODY);
            let galley = common::galley(ui, self.label.as_str(), font.clone());
            let label_width = galley.size().x;
            common::text_left(painter, rect, x, galley, label_color);
            if let Some(secondary) = &self.secondary {
                let galley = common::galley(ui, secondary.as_str(), font);
                common::text_left(
                    painter,
                    rect,
                    x + label_width + size::SECONDARY_GAP,
                    galley,
                    colors.text_placeholder,
                );
            }
        }
        RowResponse { response, toggled }
    }
}
