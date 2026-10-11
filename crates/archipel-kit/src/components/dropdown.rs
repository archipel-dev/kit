//! The dropdown (combo box).

use crate::icons::{self, Glyph};
use crate::parts::{Slot, State};
use crate::theme::Theme;
use crate::tokens::size;
use egui::{Rect, Response, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

type Leading<'a, T> = Box<dyn Fn(&T) -> Slot<'a> + 'a>;

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
///
/// [`leading`](Self::leading) shows a [`Slot`] before each option and before the selected
/// value in the closed button: a color dot or an icon, for example an environment picker.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Dropdown;
/// use archipel_kit::parts::{Slot, Tone};
/// use archipel_kit::{Glyph, Theme};
///
/// let colors = Theme::colors(ui.ctx());
/// let mut env = "Staging";
/// ui.add(
///     Dropdown::new("env", &mut env, ["Local", "Staging", "Production"])
///         .label("Environment")
///         .leading(move |env| match *env {
///             "Production" => Slot::icon_tone(Glyph::Warning, Tone::Error),
///             _ => {
///                 let color = if *env == "Local" { colors.success } else { colors.warning };
///                 Slot::ui_width(16.0, move |ui| {
///                     ui.painter().circle_filled(ui.max_rect().center(), 4.0, color);
///                 })
///             }
///         }),
/// );
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
    leading: Option<Leading<'a, T>>,
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
            leading: None,
        }
    }

    /// Accessible name, e.g. "HTTP method" (default: the selected value). Screen readers
    /// announce it with the selected value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Content before each option and before the selected value in the closed button,
    /// chosen per value. The slot's content is painted, so use [`Slot::icon`] or a
    /// [`Slot::ui_width`] that draws (a color dot); the same [`Slot`] [`ListRow`](super::ListRow)
    /// takes.
    pub fn leading(mut self, leading: impl Fn(&T) -> Slot<'a> + 'a) -> Self {
        self.leading = Some(Box::new(leading));
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
            let lead_width = self
                .leading
                .as_ref()
                .map_or(0.0, |lead| lead(self.selected).width(ui));
            let lead_space = if self.leading.is_some() {
                lead_width + size::ICON_TEXT_GAP
            } else {
                0.0
            };
            ui.spacing_mut().button_padding = vec2(9.0 + lead_space, 5.0);
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
                        let mut row = super::ListRow::new(option.to_string())
                            .flat(true)
                            .selected(selected);
                        if let Some(lead) = &self.leading {
                            row = row.leading(lead(option));
                        }
                        let row = row.show(ui);
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
        if let Some(lead) = &self.leading {
            let slot = lead(self.selected);
            let width = slot.width(ui);
            let rect = Rect::from_min_size(
                pos2(response.rect.left() + 9.0, response.rect.top()),
                vec2(width, response.rect.height()),
            );
            slot.show(ui, rect, State::of(&response));
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
