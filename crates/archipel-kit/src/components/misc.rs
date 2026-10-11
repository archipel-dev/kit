//! Links, dropdowns, menu items, empty states and property grids.

use super::{Button, common};
use crate::icons::{self, Glyph, Icon};
use crate::parts::{RowContent, Slot, State, Tone};
use crate::theme::Theme;
use crate::tokens::{popup, radius, size, typography};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, Layout, Rect, Response, Sense, Stroke, Ui, Widget,
    WidgetInfo, WidgetType, pos2, vec2,
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
    leading: Option<OptionLeading<'a, T>>,
}

type OptionLeading<'a, T> = Box<dyn Fn(&T) -> Slot<'a> + 'a>;

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
            leading: None,
        }
    }

    /// Accessible name, e.g. "HTTP method" (default: the selected value). Screen readers
    /// announce it with the selected value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Leading content for each option and the selected value: an icon or color dot.
    /// Reuses the same [`Slot`] as list rows. Keep this decorative; the option itself
    /// is the keyboard target.
    pub fn leading(mut self, slot: impl Fn(&T) -> Slot<'a> + 'a) -> Self {
        self.leading = Some(Box::new(slot));
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
        let mut escaped = false;
        let mut picked = false;
        let mut handled_enter = false;
        let inner = ui.add_enabled_ui(self.enabled, |ui| {
            // Escape closes the popup and is used up, so a surrounding dialog stays open.
            // `ComboBox::from_id_salt` hashes the salt once more.
            let button_id = ui.make_persistent_id(egui::Id::new(self.id));
            let popup_id = button_id.with("popup");
            let highlight_id = button_id.with("highlight");
            let was_open = egui::ComboBox::is_open(ui.ctx(), button_id);
            if !ui.is_enabled() {
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
            let initial = self
                .options
                .iter()
                .position(|v| v == self.selected)
                .unwrap_or(0);
            let mut highlight = (if was_open {
                ui.data(|d| d.get_temp::<usize>(highlight_id))
                    .unwrap_or(initial)
            } else {
                initial
            })
            .min(self.options.len().saturating_sub(1));
            let mut accept = false;
            let mut navigated = false;
            if was_open && ui.is_enabled() {
                ui.input_mut(|input| {
                    for key in crate::parts::nav::KEYS {
                        if input.consume_key(egui::Modifiers::NONE, key)
                            && let Some(next) = crate::parts::nav::step(
                                key,
                                Some(highlight),
                                self.options.len(),
                                10,
                            )
                        {
                            highlight = next;
                            navigated = true;
                        }
                    }
                    accept = input.key_pressed(egui::Key::Enter);
                });
                handled_enter = accept;
                if navigated {
                    ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                }
                if accept && let Some(option) = self.options.get(highlight) {
                    changed = *self.selected != *option;
                    *self.selected = option.clone();
                    picked = true;
                }
            } else if ui.is_enabled()
                && ui.memory(|m| m.has_focus(button_id))
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
            {
                egui::Popup::open_id(ui.ctx(), popup_id);
                ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
            if egui::ComboBox::is_open(ui.ctx(), button_id)
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                egui::Popup::close_id(ui.ctx(), button_id.with("popup"));
                escaped = true;
            }
            ui.spacing_mut().interact_size.y = size::CONTROL_HEIGHT;
            ui.spacing_mut().button_padding = vec2(9.0, 5.0);
            let visuals = &mut ui.style_mut().visuals.widgets;
            for state in [&mut visuals.inactive, &mut visuals.hovered] {
                state.bg_fill = colors.combo;
                state.weak_bg_fill = colors.combo;
            }
            let leading = self.leading.as_ref().map(|slot| slot(self.selected));
            let leading_width = leading.as_ref().map_or(0.0, |slot| slot.width(ui));
            let mut selected_text = egui::text::LayoutJob::simple_singleline(
                self.selected.to_string(),
                egui::TextStyle::Button.resolve(ui.style()),
                common::enabled_text(self.enabled, colors.text, &colors),
            );
            if let Some(section) = selected_text.sections.first_mut() {
                section.leading_space = if leading.is_some() {
                    leading_width + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
            }
            let selected_text = if leading.is_some() {
                egui::WidgetText::from(selected_text)
            } else {
                egui::WidgetText::from(self.selected.to_string())
            };
            let shown = egui::ComboBox::from_id_salt(self.id)
                .selected_text(selected_text)
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
                    for (index, option) in self.options.iter().enumerate() {
                        let selected = option == self.selected;
                        // Rows like the rest of the kit: a rounded pill for hover and
                        // selection, inset from the popup edge.
                        let mut row = super::ListRow::new(option.to_string())
                            .flat(true)
                            .selected(index == highlight);
                        if let Some(slot) = &self.leading {
                            row = row.leading(slot(option));
                        }
                        let row = row.show(ui);
                        if index == highlight && navigated {
                            row.response.scroll_to_me(Some(Align::Center));
                        }
                        if row.response.clicked() || (accept && index == highlight) {
                            if !selected {
                                *self.selected = option.clone();
                                changed = true;
                            }
                            picked = true;
                            ui.close();
                        }
                    }
                    ui.data_mut(|d| d.insert_temp(highlight_id, highlight));
                });
            if let Some(slot) = leading {
                let rect = Rect::from_min_size(
                    pos2(
                        shown.response.rect.left() + ui.spacing().button_padding.x,
                        shown.response.rect.top(),
                    ),
                    vec2(leading_width, shown.response.rect.height()),
                );
                slot.show(ui, rect, State::of(&shown.response));
            }
            shown
        });
        let mut response = inner.inner.response;
        if handled_enter {
            ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            });
        }
        if escaped || picked {
            response.request_focus();
        }
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
        }
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
    highlighted: bool,
    description: Option<String>,
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
            highlighted: false,
            description: None,
            submenu: None,
        }
    }

    /// A second, dimmed line under the label, like the path under a project in Android
    /// Studio's project picker. The item becomes two lines high with a larger icon.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
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

    /// Draws the hover fill without the pointer: the row the keyboard is on in a
    /// suggestion list (see the template autocomplete in the code page).
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
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
        let two_line = self.description.is_some();
        let reserve_icons = menu_icon_column(ui, icon.is_some() || self.checked.is_some());
        let content = RowContent {
            // A two-line item paints its larger icon itself.
            icon: if two_line { None } else { icon },
            icon_tone,
            reserve_icon: !two_line && reserve_icons,
            label: &self.label,
            label_tone: Tone::Default,
            secondary: None,
            trailing: self.shortcut.as_deref().filter(|_| self.submenu.is_none()),
            ..RowContent::default()
        };
        // Geometry measured on Android Studio: the highlight pill is inset from the popup's
        // sides, and the icon column is inset inside the pill.
        let chrome = 2.0 * (popup::ITEM_INSET + popup::ITEM_PADDING);
        let large_icon = popup::LARGE_ICON + 2.0 * size::ICON_TEXT_GAP;
        let width = ui
            .available_width()
            .max(
                content.min_width(ui)
                    + chrome
                    + if two_line { large_icon } else { 0.0 }
                    + if self.submenu.is_some() {
                        size::ICON
                    } else {
                        0.0
                    },
            )
            .max(180.0);
        let height = if two_line {
            size::ROW_HEIGHT + popup::DESCRIPTION_HEIGHT
        } else {
            size::ROW_HEIGHT
        };
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        menu_rows(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
        crate::cursor::on_hover(&response, egui::CursorIcon::Default);
        response.widget_info(|| {
            let mut info = match self.checked {
                Some(checked) => {
                    WidgetInfo::selected(WidgetType::Checkbox, self.enabled, checked, &self.label)
                }
                None => WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label),
            };
            if let Some(description) = &self.description {
                info.label = Some(format!("{} {description}", self.label));
            }
            info
        });
        let current = menu_current(ui, &response, self.enabled);
        if ui.is_rect_visible(rect) {
            let mut state = State::of(&response);
            state.hovered = self.enabled && (self.highlighted || current);
            let pill = rect.shrink2(vec2(popup::ITEM_INSET, 0.0));
            let fill = state.menu_fill(&colors);
            if fill != Color32::TRANSPARENT {
                ui.painter()
                    .rect_filled(pill, CornerRadius::same(radius::ROW), fill);
            }
            let x = pill.left() + popup::ITEM_PADDING;
            if let Some(description) = &self.description {
                let line = Rect::from_min_size(pill.min, vec2(pill.width(), size::ROW_HEIGHT));
                if let Some(icon) = icon {
                    let icon_rect = Rect::from_center_size(
                        pos2(x + popup::LARGE_ICON / 2.0, line.center().y),
                        vec2(popup::LARGE_ICON, popup::LARGE_ICON),
                    );
                    icons::paint(
                        ui.painter(),
                        icon_rect,
                        icon,
                        state.icon(icon_tone.unwrap_or(Tone::Icon), &colors),
                    );
                }
                let text_x = x + large_icon;
                content.paint(ui, line, text_x, state);
                let below = Rect::from_min_size(
                    pos2(line.left(), line.bottom()),
                    vec2(
                        line.width(),
                        popup::DESCRIPTION_HEIGHT - 2.0 * size::ICON_TEXT_GAP,
                    ),
                );
                let galley = common::galley_elided(
                    ui,
                    description,
                    typography::body(typography::BODY),
                    (pill.right() - popup::ITEM_PADDING - text_x).max(0.0),
                );
                let color = if self.enabled {
                    colors.text_placeholder
                } else {
                    colors.text_disabled
                };
                common::text_left(ui.painter(), below, text_x, galley, color);
            } else {
                content.paint(ui, pill, x, state);
            }
            if self.submenu.is_some() {
                let chevron = Rect::from_min_size(
                    pos2(
                        pill.right() - size::ROW_PILL_INSET - size::ICON,
                        rect.top() + (size::ROW_HEIGHT - size::ICON) / 2.0,
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

/// Menu rows touch, 24 pt apart, as in Android Studio. Menu widgets set it on the `Ui` they
/// are added to, so a menu looks right in any popup, not only in the kit's.
pub(crate) fn menu_rows(ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 0.0;
}

/// The row a menu's pointer and keyboard share, per menu (the `Ui` its items are in).
#[derive(Clone, Copy, Default)]
struct MenuCursor {
    /// The current item: the last one hovered or focused. It stays when the pointer leaves.
    item: Option<egui::Id>,
    /// The frame the pointer first moved after the menu opened; that move is ignored.
    armed: Option<u64>,
    /// The last frame the menu was drawn, to notice it was closed and reopened.
    seen: u64,
    /// Where the pointer was, to tell a real move from a frame drawn under a still pointer.
    pointer: Option<egui::Pos2>,
    /// Whether the pointer moved in frame `seen`.
    moved: bool,
    /// Whether an item of this menu has an icon (this frame or the last): only then do the
    /// items reserve an icon column, as in Android Studio.
    icons: bool,
    /// The same, for the frame being drawn.
    icons_now: bool,
    /// Whether a frame was already rolled over (the first one keeps `icons` as found).
    seen_once: bool,
}

/// Whether the item of `response` is its menu's current row, as in Android Studio: hover and
/// arrow keys move one shared highlight; it stays on the last item when the pointer leaves;
/// and the first pointer move after the menu opens is ignored, so a menu that opens under
/// the pointer does not highlight whatever happens to be there.
fn menu_current(ui: &Ui, response: &Response, enabled: bool) -> bool {
    let ctx = ui.ctx();
    let frame = ctx.cumulative_frame_nr();
    let (key, mut cursor) = menu_cursor(ui);
    let moved = cursor.moved;
    if moved && cursor.armed.is_none() {
        cursor.armed = Some(frame);
    }
    let armed = cursor.armed.is_some_and(|f| f < frame);
    if enabled && response.hovered() && moved && armed {
        cursor.item = Some(response.id);
        // The keyboard continues from where the pointer is.
        if !response.has_focus() {
            response.request_focus();
        }
    }
    if enabled && response.has_focus() {
        cursor.item = Some(response.id);
    }
    ctx.data_mut(|d| d.insert_temp(key, cursor));
    enabled && cursor.item == Some(response.id)
}

/// Whether the items of the menu `ui` holds reserve an icon column: only when one of them
/// has an icon, as in Android Studio (a list of plain names starts its text at the pill's
/// padding). `has_icon` is this item's.
fn menu_icon_column(ui: &Ui, has_icon: bool) -> bool {
    let (key, mut cursor) = menu_cursor(ui);
    cursor.icons_now |= has_icon;
    ui.ctx().data_mut(|d| d.insert_temp(key, cursor));
    cursor.icons || cursor.icons_now
}

/// The shared state of the menu `ui` holds, rolled over to the current frame.
fn menu_cursor(ui: &Ui) -> (egui::Id, MenuCursor) {
    let ctx = ui.ctx();
    let key = ui.id().with("kit-menu-cursor");
    let frame = ctx.cumulative_frame_nr();
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let mut cursor = ctx
        .data(|d| d.get_temp::<MenuCursor>(key))
        .unwrap_or_default();
    if cursor.seen + 1 < frame {
        // Not drawn last frame: the menu (re)opened, under a pointer that has not moved yet.
        // What its items look like (icons) is kept.
        cursor = MenuCursor {
            pointer,
            icons: cursor.icons || cursor.icons_now,
            ..MenuCursor::default()
        };
    }
    if cursor.seen != frame {
        // First item this frame: did the pointer move since the last one?
        cursor.moved = pointer.is_some() && pointer != cursor.pointer;
        cursor.pointer = pointer;
        cursor.seen = frame;
        if cursor.seen_once {
            cursor.icons = cursor.icons_now;
        }
        cursor.icons_now = false;
        cursor.seen_once = true;
    }
    (key, cursor)
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
