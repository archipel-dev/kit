//! Links, dropdowns, menu items, empty states and property grids.

use super::{Button, common};
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use egui::{
    Align, CursorIcon, Layout, Rect, Response, Sense, Stroke, Ui, Widget, WidgetInfo, WidgetType,
    pos2, vec2,
};

/// A text link: underlined on hover, pointer cursor.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Link;
///
/// if ui.add(Link::new("Configure…")).clicked() { /* ... */ }
/// # });
/// ```
#[must_use = "add it with `ui.add(link)`"]
pub struct Link {
    text: String,
}

impl Link {
    /// A link reading `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for Link {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(ui, self.text.as_str(), typography::body(typography::BODY));
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &self.text));
        let response = response.on_hover_cursor(CursorIcon::PointingHand);
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

/// A dropdown (combo box) picking one value from a list.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Dropdown;
///
/// let mut method = "GET";
/// ui.add(Dropdown::new("method", &mut method, ["GET", "POST", "PUT", "DELETE"]));
/// # });
/// ```
#[must_use = "add it with `ui.add(dropdown)`"]
pub struct Dropdown<'a, T: PartialEq + Clone + ToString> {
    id: egui::Id,
    selected: &'a mut T,
    options: Vec<T>,
    width: f32,
    enabled: bool,
    label: Option<String>,
}

impl<'a, T: PartialEq + Clone + ToString> Dropdown<'a, T> {
    /// A dropdown with a unique `id_salt`, editing `selected`, offering `options`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        selected: &'a mut T,
        options: impl IntoIterator<Item = T>,
    ) -> Self {
        Self {
            id: egui::Id::new(id_salt),
            selected,
            options: options.into_iter().collect(),
            width: 160.0,
            enabled: true,
            label: None,
        }
    }

    /// Accessible name, e.g. "HTTP method" (default: the selected value). Screen readers
    /// announce it with the selected value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Width in points (default 160).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Disabled dropdowns are dimmed and cannot open.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<T: PartialEq + Clone + ToString> Widget for Dropdown<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let mut changed = false;
        let inner = ui.add_enabled_ui(self.enabled, |ui| {
            ui.spacing_mut().interact_size.y = size::CONTROL_HEIGHT;
            ui.spacing_mut().button_padding = vec2(8.0, 5.0);
            ui.style_mut().visuals.widgets.inactive.bg_fill = colors.field;
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = colors.field;
            egui::ComboBox::from_id_salt(self.id)
                .selected_text(self.selected.to_string())
                .width(self.width)
                .height(320.0)
                .icon(|ui, rect, _visuals, open| {
                    let colors = Theme::colors(ui.ctx());
                    let icon = if open {
                        Icon::ChevronUp
                    } else {
                        Icon::ChevronDown
                    };
                    let color = if ui.is_enabled() {
                        colors.text_secondary
                    } else {
                        colors.text_disabled
                    };
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(rect.center(), vec2(size::ICON, size::ICON)),
                        icon,
                        color,
                    );
                })
                .show_ui(ui, |ui| {
                    ui.set_min_width(self.width);
                    for option in &self.options {
                        let selected = option == self.selected;
                        if ui.selectable_label(selected, option.to_string()).clicked() && !selected
                        {
                            *self.selected = option.clone();
                            changed = true;
                        }
                    }
                })
        });
        let mut response = inner.inner.response;
        if changed {
            response.mark_changed();
        }
        let selected = self.selected.to_string();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(
                WidgetType::ComboBox,
                self.enabled,
                self.label.clone().unwrap_or_else(|| selected.clone()),
            );
            info.current_text_value = Some(selected.clone());
            info
        });
        response
    }
}

/// A menu entry with an optional icon and right-aligned shortcut, for context menus and
/// menu bars (inside `ui.menu_button` or `response.context_menu`).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Icon, components::MenuItem};
///
/// ui.menu_button("Edit", |ui| {
///     if ui.add(MenuItem::new("Copy").icon(Icon::Copy).shortcut("Ctrl+C")).clicked() {
///         ui.close();
///     }
/// });
/// # });
/// ```
#[must_use = "add it with `ui.add(item)`"]
pub struct MenuItem {
    label: String,
    icon: Option<Icon>,
    shortcut: Option<String>,
    enabled: bool,
    checked: Option<bool>,
}

impl MenuItem {
    /// An item reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            enabled: true,
            checked: None,
        }
    }

    /// Icon in the left column.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Shortcut shown on the right, e.g. `"Ctrl+Shift+F"`.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Disabled items are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A check mark in the icon column (for toggle items).
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
}

impl Widget for MenuItem {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        let label = common::galley(ui, self.label.as_str(), font.clone());
        let shortcut = self
            .shortcut
            .as_ref()
            .map(|s| common::galley(ui, s.as_str(), font.clone()));
        let min_width =
            24.0 + label.size().x + shortcut.as_ref().map_or(0.0, |s| 32.0 + s.size().x) + 8.0;
        let width = ui.available_width().max(min_width).max(180.0);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, size::ROW_HEIGHT), sense);
        response.widget_info(|| match self.checked {
            Some(checked) => {
                WidgetInfo::selected(WidgetType::Checkbox, self.enabled, checked, &self.label)
            }
            None => WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label),
        });
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if self.enabled && response.hovered() {
                painter.rect_filled(rect, 4.0, colors.selection);
            }
            let icon_rect = Rect::from_min_size(
                pos2(rect.left() + 4.0, rect.center().y - 8.0),
                vec2(16.0, 16.0),
            );
            let text = common::enabled_text(self.enabled, colors.text, &colors);
            match (self.checked, self.icon) {
                (Some(true), _) => icons::paint(painter, icon_rect, Icon::Success, text),
                (_, Some(icon)) => icons::paint(
                    painter,
                    icon_rect,
                    icon,
                    common::enabled_text(self.enabled, colors.icon, &colors),
                ),
                _ => {}
            }
            common::text_left(painter, rect, rect.left() + 26.0, label, text);
            if let Some(shortcut) = shortcut {
                let x = rect.right() - 8.0 - shortcut.size().x;
                common::text_left(painter, rect, x, shortcut, colors.text_placeholder);
            }
        }
        response
    }
}

/// What to show where a list, table or panel has nothing to display: a title, a hint and
/// optionally one action.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Icon, components::EmptyState};
///
/// let out = EmptyState::new("No requests yet")
///     .icon(Icon::Send)
///     .hint("Create a request to start testing your API.")
///     .action("New request")
///     .show(ui);
/// if out.clicked() { /* ... */ }
/// # });
/// ```
pub struct EmptyState {
    title: String,
    hint: Option<String>,
    icon: Option<Icon>,
    action: Option<String>,
}

impl EmptyState {
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
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
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

/// Key/value rows with aligned columns, for inspectors and detail panes.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::PropertyGrid;
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
