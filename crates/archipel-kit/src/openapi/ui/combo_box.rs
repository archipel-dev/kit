//! The combo box.

use crate::common;
use crate::icons::{self, Glyph};
use crate::parts::{Slot, State};
use crate::theme::Theme;
use crate::tokens::size;
use egui::{
    Align, Color32, Rect, Response, Stroke, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

/// A dropdown (combo box) picking one value from a list.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::ComboBox;
///
/// let mut method = "GET";
/// ui.add(ComboBox::new("method", &mut method, ["GET", "POST", "PUT", "DELETE"]));
/// # });
/// ```
#[must_use = "add it with `ui.add(dropdown)`"]
pub struct ComboBox<'a, T: PartialEq + Clone + ToString> {
    id: egui::Id,
    selected: &'a mut T,
    options: Vec<T>,
    width: f32,
    enabled: bool,
    label: Option<String>,
    leading: Option<OptionLeading<'a, T>>,
    tone: Option<OptionTone<'a, T>>,
    separators: Vec<usize>,
}

type OptionLeading<'a, T> = Box<dyn Fn(&T) -> Slot<'a> + 'a>;
type OptionTone<'a, T> = Box<dyn Fn(&T) -> crate::parts::Tone + 'a>;

impl<'a, T: PartialEq + Clone + ToString> ComboBox<'a, T> {
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
            tone: None,
            separators: Vec::new(),
        }
    }

    /// Lines in the list before the options at these indices, grouping them like Android
    /// Studio's theme picker (Islands themes, classic themes, "Get More Themes…").
    pub fn separators(mut self, before: impl IntoIterator<Item = usize>) -> Self {
        self.separators = before.into_iter().collect();
        self
    }

    /// Accessible name, e.g. "HTTP method" (default: the selected value). Screen readers
    /// announce it with the selected value.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Colors each option's text, and the selected value's, by meaning: HTTP methods,
    /// log levels. The text stays the option's name for screen readers.
    pub fn tone(mut self, tone: impl Fn(&T) -> crate::parts::Tone + 'a) -> Self {
        self.tone = Some(Box::new(tone));
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

impl<T: PartialEq + Clone + ToString> Widget for ComboBox<'_, T> {
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
            if !self.enabled {
                // Measured on Android Studio: a disabled combo has no fill, a faint
                // border and dimmed text, instead of a faded copy of the enabled one.
                for state in [
                    &mut visuals.noninteractive,
                    &mut visuals.inactive,
                    &mut visuals.hovered,
                ] {
                    state.bg_fill = Color32::TRANSPARENT;
                    state.weak_bg_fill = Color32::TRANSPARENT;
                    state.bg_stroke = Stroke::new(1.0_f32, colors.border_disabled);
                }
                ui.style_mut().visuals.override_text_color = Some(colors.text_disabled);
                ui.set_opacity(1.0);
            }
            let leading = self.leading.as_ref().map(|slot| slot(self.selected));
            let leading_width = leading.as_ref().map_or(0.0, |slot| slot.width(ui));
            let text_color = match &self.tone {
                Some(tone) if self.enabled => tone(self.selected).color(&colors),
                _ => common::enabled_text(self.enabled, colors.text, &colors),
            };
            let mut selected_text = egui::text::LayoutJob::simple_singleline(
                self.selected.to_string(),
                egui::TextStyle::Button.resolve(ui.style()),
                text_color,
            );
            if let Some(section) = selected_text.sections.first_mut() {
                section.leading_space = if leading.is_some() {
                    leading_width + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
            }
            let selected_text = if leading.is_some() || self.tone.is_some() {
                egui::WidgetText::from(selected_text)
            } else {
                egui::WidgetText::from(self.selected.to_string())
            };
            let shown = egui::ComboBox::from_id_salt(self.id)
                .selected_text(selected_text)
                .width(self.width)
                .height(320.0)
                .icon(|ui, rect, _visuals, _open| {
                    let colors = Theme::colors(ui.ctx());
                    // The chevron keeps pointing down while open, as in Android Studio.
                    let icon = Glyph::ChevronDown;
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
                        if index > 0 && self.separators.contains(&index) {
                            ui.add(crate::ui::popup::ListSeparator::new());
                        }
                        let selected = option == self.selected;
                        // Rows like the rest of the kit: a rounded pill for hover and
                        // selection, inset from the popup edge.
                        let mut row = crate::ui::ColoredListCellRenderer::new(option.to_string())
                            .flat(true)
                            .selected(index == highlight);
                        if let Some(slot) = &self.leading {
                            row = row.leading(slot(option));
                        }
                        if let Some(tone) = &self.tone {
                            row = row.label_tone(tone(option));
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
