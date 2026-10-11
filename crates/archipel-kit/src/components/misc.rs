//! Links, dropdowns, menu items, empty states and property grids.

use super::{Button, common};
use crate::icons::{self, Glyph, Icon};
use crate::parts::{RowContent, State, Tone};
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
            ui.spacing_mut().button_padding = vec2(9.0, 5.0);
            let visuals = &mut ui.style_mut().visuals.widgets;
            for state in [&mut visuals.inactive, &mut visuals.hovered] {
                state.bg_fill = colors.combo;
                state.weak_bg_fill = colors.combo;
            }
            egui::ComboBox::from_id_salt(self.id)
                .selected_text(self.selected.to_string())
                .width(self.width)
                .height(320.0)
                .icon(|ui, rect, _visuals, open| {
                    let colors = Theme::colors(ui.ctx());
                    let icon = if open {
                        Glyph::ChevronUp
                    } else {
                        Glyph::ChevronDown
                    };
                    let color = if ui.is_enabled() {
                        colors.combo_arrow
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
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for option in &self.options {
                        let selected = option == self.selected;
                        // Rows like the rest of the kit: a rounded pill for hover and
                        // selection, inset from the popup edge.
                        let row = super::ListRow::new(option.to_string())
                            .flat(true)
                            .selected(selected)
                            .show(ui);
                        if row.response.clicked() {
                            if !selected {
                                *self.selected = option.clone();
                                changed = true;
                            }
                            ui.close();
                        }
                    }
                })
        });
        let mut response = inner.inner.response;
        if changed {
            response.mark_changed();
        }
        let selected = self.selected.to_string();
        crate::cursor::on_hover_action(&response);
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
/// use archipel_kit::{Glyph, components::MenuItem};
///
/// ui.menu_button("Edit", |ui| {
///     if ui.add(MenuItem::new("Copy").icon(Glyph::Copy).shortcut("Ctrl+C")).clicked() {
///         ui.close();
///     }
/// });
/// # });
/// ```
///
/// [`submenu`](Self::submenu) turns the item into a parent: a chevron at the right end, and
/// a menu to its right that opens on hover, <kbd>→</kbd> or <kbd>Enter</kbd> and closes on
/// <kbd>←</kbd>, <kbd>Escape</kbd> or when the pointer moves to another item. Clicking an
/// item of the submenu closes the whole menu.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::MenuItem;
///
/// ui.add(MenuItem::new("New").submenu(|ui| {
///     ui.add(MenuItem::new("File…"));
///     ui.add(MenuItem::new("Directory…"));
/// }));
/// # });
/// ```
#[must_use = "add it with `ui.add(item)`"]
pub struct MenuItem<'a> {
    label: String,
    icon: Option<Icon>,
    shortcut: Option<String>,
    enabled: bool,
    checked: Option<bool>,
    submenu: Option<Submenu<'a>>,
}

/// The content of a submenu.
type Submenu<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl<'a> MenuItem<'a> {
    /// An item reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            enabled: true,
            checked: None,
            submenu: None,
        }
    }

    /// Makes the item open a submenu drawn by `add` (with [`MenuItem`]s). A chevron
    /// replaces the shortcut.
    pub fn submenu(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.submenu = Some(Box::new(add));
        self
    }

    /// Icon in the left column.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
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

impl Widget for MenuItem<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (icon, icon_tone) = match (self.checked, self.icon) {
            (Some(true), _) => (Some(Icon::Glyph(Glyph::Success)), Some(Tone::Default)),
            (Some(false), _) => (None, None),
            (None, icon) => (icon, None),
        };
        let content = RowContent {
            icon,
            icon_tone,
            reserve_icon: true,
            label: &self.label,
            label_tone: Tone::Default,
            secondary: None,
            trailing: self.shortcut.as_deref().filter(|_| self.submenu.is_none()),
            ..RowContent::default()
        };
        let width = ui
            .available_width()
            .max(
                content.min_width(ui)
                    + 10.0
                    + if self.submenu.is_some() {
                        size::ICON
                    } else {
                        0.0
                    },
            )
            .max(180.0);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, size::ROW_HEIGHT), sense);
        crate::cursor::on_hover(&response, egui::CursorIcon::Default);
        response.widget_info(|| match self.checked {
            Some(checked) => {
                WidgetInfo::selected(WidgetType::Checkbox, self.enabled, checked, &self.label)
            }
            None => WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label),
        });
        if ui.is_rect_visible(rect) {
            let state = State::of(&response);
            common::row_pill(ui.painter(), rect, state.menu_fill(&colors));
            content.paint(ui, rect, rect.left() + 4.0, state);
            if self.submenu.is_some() {
                let chevron = Rect::from_min_size(
                    pos2(
                        rect.right() - size::ROW_PILL_INSET - size::ICON,
                        rect.center().y - size::ICON / 2.0,
                    ),
                    vec2(size::ICON, size::ICON),
                );
                icons::paint(
                    ui.painter(),
                    chevron,
                    Glyph::ChevronRight,
                    state.icon(Tone::Icon, &colors),
                );
            }
        }
        match self.submenu {
            Some(submenu) if self.enabled => show_submenu(ui, &response, submenu),
            Some(_) => {}
            // A click on an item closes the menu it is in.
            None if response.clicked() => ui.close(),
            None => {}
        }
        response
    }
}

/// What to show where a list, table or panel has nothing to display: a title, a hint and
/// optionally one action.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, components::EmptyState};
///
/// let out = EmptyState::new("No requests yet")
///     .icon(Glyph::Send)
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

/// Open state of a submenu: whether it is open, and the frame the item was last drawn in
/// (a menu that was closed forgets which submenu was open).
#[derive(Clone, Copy, Default)]
struct SubmenuState {
    open: bool,
    seen: u64,
}

/// Opens, shows and closes the submenu of the item `response`.
fn show_submenu(ui: &mut Ui, response: &Response, add: Submenu<'_>) {
    let ctx = ui.ctx().clone();
    // The state outlives the menu, which stops drawing its items when it closes: nothing
    // tells the item it was closed. So the item records the frame it was last drawn in, and
    // an item that was not drawn in the previous frame (its menu was closed and has just
    // reopened) starts closed instead of showing a stale open submenu.
    let frame = ctx.cumulative_frame_nr();
    let mut state: SubmenuState = ctx.data(|d| d.get_temp(response.id)).unwrap_or_default();
    if frame > state.seen + 1 {
        state.open = false;
    }
    state.seen = frame;
    let popup_id = response.id.with("submenu");
    let submenu_rect = ctx.memory(|m| m.area_rect(egui::Id::new(popup_id)));

    // (Not inside `ctx.input`: the context's lock is not reentrant.)
    let focused = response.has_focus();
    let keyed = (focused || response.hovered())
        && ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                || (focused && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        });
    let was_open = state.open;
    // Hover opens when the pointer moves onto the item, so Left can close it under a still
    // pointer.
    let moving = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
    if (response.hovered() && moving) || response.clicked() || keyed {
        state.open = true;
    }
    if state.open {
        // Left closes; so does moving the pointer to something that is neither this item
        // nor its submenu. (Escape is handled by the popover.)
        let left = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
        let away = ctx.input(|i| {
            i.pointer.delta() != egui::Vec2::ZERO
                && i.pointer.hover_pos().is_some_and(|p| {
                    !response.rect.contains(p) && !submenu_rect.is_some_and(|r| r.contains(p))
                })
        });
        if left || away {
            state.open = false;
            if left {
                response.request_focus();
            }
        }
    }
    let mut open = state.open;
    let shown = crate::parts::Popover::right_of(popup_id, response)
        .width(size::SUBMENU_WIDTH)
        .show(&ctx, &mut open, add);
    if state.open && !open {
        // Escape (or a click outside) closed it: keep the item focused for the next key.
        response.request_focus();
    }
    if shown.is_some() && keyed && !was_open {
        // Into the submenu: the first item is the next widget in the focus order.
        ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
    }
    state.open = open;
    ctx.data_mut(|d| d.insert_temp(response.id, state));
    ctx.accesskit_node_builder(response.id, |node| {
        node.set_expanded(state.open);
        node.set_has_popup(egui::accesskit::HasPopup::Menu);
    });
}
