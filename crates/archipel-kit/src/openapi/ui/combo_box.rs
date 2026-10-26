//! The combo box.

use crate::common;
use crate::icons::{self, Glyph};
use crate::parts::{Slot, State, Tone};
use crate::theme::Theme;
use crate::tokens::{motion, size, typography};
use crate::ui::popup::{Options, Placement, PopupChain, RowSpec, drive};
use egui::{
    Color32, CornerRadius, Id, Key, Modifiers, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    Widget, WidgetInfo, WidgetType, pos2, vec2,
};

/// A dropdown (combo box) picking one value from a list: IntelliJ's `ComboBox`.
///
/// The list is the kit's popup engine ([`ListPopup`](crate::ui::popup::ListPopup)): it opens
/// below the combo (above when there is no room), as wide as the combo, shows
/// [ten rows](Self::max_rows) before it scrolls, and highlights the value the combo holds.
/// The highlight moves without changing the value; the value changes when a row is chosen.
///
/// | Input | Closed | Open |
/// |---|---|---|
/// | <kbd>Alt</kbd>+<kbd>Down</kbd>, <kbd>F4</kbd> | Opens the list. | |
/// | <kbd>Enter</kbd> (press), <kbd>Space</kbd> (release) | Opens the list. | <kbd>Enter</kbd> chooses the highlighted row. |
/// | <kbd>Up</kbd> / <kbd>Down</kbd> | Previous / next value, stopping at the ends. | Move the highlight, wrapping around. |
/// | <kbd>Home</kbd> <kbd>End</kbd> <kbd>PgUp</kbd> <kbd>PgDn</kbd> | | First, last, a page up or down. |
/// | letters | Select the next value starting with what is typed (reset after 1 s). | Filter, with [`speed_search`](Self::speed_search). |
/// | <kbd>Esc</kbd> | Not used (a dialog sees it). | Closes the list and keeps the value. |
/// | click | Opens the list. | A click on the combo closes it; a click on a row chooses it; a click outside closes it. |
///
/// An editable combo box is a [`JBTextField::suggestions`](crate::ui::JBTextField::suggestions).
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
    id: Id,
    selected: &'a mut T,
    options: Vec<T>,
    width: f32,
    enabled: bool,
    label: Option<String>,
    leading: Option<OptionLeading<'a, T>>,
    tone: Option<OptionTone<'a, T>>,
    separators: Vec<usize>,
    unselectable: Vec<usize>,
    max_rows: usize,
    speed_search: bool,
}

type OptionLeading<'a, T> = Box<dyn Fn(&T) -> Slot<'a> + 'a>;
type OptionTone<'a, T> = Box<dyn Fn(&T) -> Tone + 'a>;

impl<'a, T: PartialEq + Clone + ToString> ComboBox<'a, T> {
    /// A dropdown with a unique `id_salt`, editing `selected`, offering `options`.
    pub fn new(
        id_salt: impl std::hash::Hash,
        selected: &'a mut T,
        options: impl IntoIterator<Item = T>,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            selected,
            options: options.into_iter().collect(),
            width: 160.0,
            enabled: true,
            label: None,
            leading: None,
            tone: None,
            separators: Vec::new(),
            unselectable: Vec::new(),
            max_rows: size::COMBO_MAX_ROWS,
            speed_search: false,
        }
    }

    /// Lines in the list before the options at these indices, grouping them like Android
    /// Studio's theme picker (Islands themes, classic themes, "Get More Themes…").
    pub fn separators(mut self, before: impl IntoIterator<Item = usize>) -> Self {
        self.separators = before.into_iter().collect();
        self
    }

    /// Options at these indices are listed but cannot be chosen: the arrow keys skip them
    /// (IntelliJ `ComboBox.SelectableItem`).
    pub fn unselectable(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.unselectable = indices.into_iter().collect();
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
    pub fn tone(mut self, tone: impl Fn(&T) -> Tone + 'a) -> Self {
        self.tone = Some(Box::new(tone));
        self
    }

    /// Leading content for each option and the selected value: an icon or color dot. The
    /// selected value shows any [`Slot`]; the rows of the list only show icon slots. Keep this
    /// decorative; the option itself is the keyboard target.
    pub fn leading(mut self, slot: impl Fn(&T) -> Slot<'a> + 'a) -> Self {
        self.leading = Some(Box::new(slot));
        self
    }

    /// Width in points (default 160, at least 49: `ComboBox.minimumSize`).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Rows shown before the list scrolls (IntelliJ `getMaximumRowCount`, default 10).
    pub fn max_rows(mut self, rows: usize) -> Self {
        self.max_rows = rows.max(1);
        self
    }

    /// Typing in the open list filters its rows (default off, as in IntelliJ's combo
    /// boxes: typing then selects by prefix when the list is closed).
    pub fn speed_search(mut self, on: bool) -> Self {
        self.speed_search = on;
        self
    }

    /// Disabled dropdowns are dimmed and cannot open.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn selectable(&self, index: usize) -> bool {
        !self.unselectable.contains(&index)
    }

    /// The index `by` steps from `from` among selectable options, stopping at the ends.
    fn step(&self, from: usize, by: isize) -> Option<usize> {
        let mut index = from as isize;
        loop {
            index += by;
            if index < 0 || index >= self.options.len() as isize {
                return None;
            }
            if self.selectable(index as usize) {
                return Some(index as usize);
            }
        }
    }

    /// The next selectable option, after `current`, whose name starts with `typed`.
    fn starting_with(&self, typed: &str, current: usize, skip_current: bool) -> Option<usize> {
        let typed = typed.to_lowercase();
        let len = self.options.len();
        (0..len)
            .map(|offset| (current + usize::from(skip_current) + offset) % len)
            .find(|&index| {
                self.selectable(index)
                    && self.options[index]
                        .to_string()
                        .to_lowercase()
                        .starts_with(&typed)
            })
    }
}

impl<T: PartialEq + Clone + ToString> Widget for ComboBox<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let enabled = self.enabled && ui.is_enabled();
        let id = ui.make_persistent_id(self.id);
        let popup_id = id.with("popup");
        let typed_id = id.with("typed");
        let width = self.width.max(size::COMBO_MIN_WIDTH);
        let metrics = crate::tokens::metrics(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(vec2(width, metrics.control_height), Sense::hover());
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(rect, id, sense);
        // Space on release, Enter on press, like a push button.
        crate::ui::activation::key_activation(ui, &mut response, enabled, true);
        let now = ui.input(|i| i.time);
        let mut chain: PopupChain = ui.data(|d| d.get_temp(popup_id)).unwrap_or_default();
        if !enabled {
            chain.close(now);
        }
        let was_open = chain.is_open();
        let current = self
            .options
            .iter()
            .position(|v| v == self.selected)
            .unwrap_or(0);
        let mut changed = false;
        let mut done = false;

        if enabled && response.has_focus() {
            // The arrow keys belong to the combo (to step or to move in the list), not to
            // the focus.
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
        }
        if enabled && !was_open {
            let open_key = response.has_focus()
                && ui.input_mut(|i| {
                    i.consume_key(Modifiers::ALT, Key::ArrowDown)
                        || i.consume_key(Modifiers::NONE, Key::F4)
                });
            if (response.clicked() || open_key) && chain.open(now, true) {
                // The key that opened the list must not choose a row in the same frame.
                ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                // The list is measured on its first frame and placed on the next.
                ui.ctx().request_repaint();
            }
            if response.has_focus() && !chain.is_open() {
                // Up and Down change the value without opening the list.
                let by = ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        1
                    } else if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        -1
                    } else {
                        0
                    }
                });
                let mut target = if by == 0 {
                    None
                } else {
                    self.step(current, by)
                };
                // Letters select the next value starting with what is typed.
                let typed: String = ui.input_mut(|i| {
                    let mut typed = String::new();
                    i.events.retain(|event| match event {
                        egui::Event::Text(text)
                            if !text.chars().any(char::is_control) && !text.trim().is_empty() =>
                        {
                            typed.push_str(text);
                            false
                        }
                        _ => true,
                    });
                    typed
                });
                if !typed.is_empty() {
                    let (before, at): (String, f64) = ui
                        .data(|d| d.get_temp(typed_id))
                        .unwrap_or((String::new(), f64::NEG_INFINITY));
                    let buffer = if now - at > f64::from(motion::COMBO_TYPE_AHEAD_RESET) {
                        typed
                    } else {
                        before + &typed
                    };
                    // One letter, or the same letter again, cycles through the matches.
                    let first = buffer.chars().next().unwrap_or(' ');
                    let repeated = buffer.chars().all(|c| c == first);
                    let found = if repeated {
                        self.starting_with(&first.to_string(), current, true)
                    } else {
                        self.starting_with(&buffer, current, false)
                    };
                    if found.is_some() {
                        target = found;
                    }
                    ui.data_mut(|d| d.insert_temp(typed_id, (buffer, now)));
                }
                if let Some(index) = target
                    && index != current
                {
                    *self.selected = self.options[index].clone();
                    changed = true;
                }
            }
        }

        // Paint the closed combo box.
        let focused = response.has_focus();
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let corner = CornerRadius::same(metrics.control_radius);
            let (fill, stroke, text_color, arrow) = if enabled {
                (
                    colors.combo,
                    Stroke::new(1.0_f32, colors.border),
                    colors.text,
                    colors.combo_arrow,
                )
            } else {
                // Measured on Android Studio: a disabled combo has no fill, a faint
                // border and dimmed text, instead of a faded copy of the enabled one.
                (
                    Color32::TRANSPARENT,
                    Stroke::new(1.0_f32, colors.border_disabled),
                    colors.text_disabled,
                    colors.text_disabled,
                )
            };
            painter.rect(rect, corner, fill, stroke, StrokeKind::Inside);
            if focused && enabled {
                crate::parts::focus_ring(painter, rect, metrics.control_radius, &colors);
            }
            let chevron = Rect::from_center_size(
                pos2(rect.right() - size::COMBO_CHEVRON_INSET, rect.center().y),
                vec2(size::ICON, size::ICON),
            );
            // The chevron keeps pointing down while open, as in Android Studio.
            icons::paint(painter, chevron, Glyph::ChevronDown, arrow);
            let mut x = rect.left() + size::COMBO_PADDING_LEFT;
            if let Some(slot) = self.leading.as_ref().map(|slot| slot(self.selected)) {
                let width = slot.width(ui);
                slot.show(
                    ui,
                    Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height())),
                    State::of(&response),
                );
                x += width + size::ICON_TEXT_GAP;
            }
            let painter = ui.painter();
            let color = match &self.tone {
                Some(tone) if enabled => tone(self.selected).color(&colors),
                _ => text_color,
            };
            let galley = common::galley_elided(
                ui,
                &self.selected.to_string(),
                typography::body(typography::BODY),
                (chevron.left() - x - size::ICON_TEXT_GAP).max(0.0),
            );
            common::text_left(painter, rect, x, galley, color);
        }

        // The open list.
        if chain.is_open() {
            let mut source = crate::ui::popup_rows::ListRows {
                rows: self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| RowSpec {
                        text: option.to_string(),
                        icon: self
                            .leading
                            .as_ref()
                            .and_then(|slot| slot(option).icon_of()),
                        selectable: self.selectable(index),
                        separator: (index > 0 && self.separators.contains(&index)).then_some(None),
                        tone: self.tone.as_ref().map(|tone| tone(option)),
                        ..RowSpec::default()
                    })
                    .collect(),
                default: current,
                speed_search: self.speed_search,
                chosen: None,
            };
            let output = drive(
                ui.ctx(),
                popup_id,
                Placement::Combo {
                    anchor: rect,
                    max_rows: self.max_rows,
                },
                Options {
                    show_submenu_on_hover: false,
                    cancel_on_click_outside: true,
                },
                &mut chain,
                &mut source,
            );
            if let Some(index) = source.chosen.filter(|_| output.chosen.is_some()) {
                if index != current {
                    *self.selected = self.options[index].clone();
                    changed = true;
                }
                done = true;
            }
            if output.cancelled {
                done = true;
            }
        }
        ui.data_mut(|d| d.insert_temp(popup_id, chain));
        if done {
            response.request_focus();
        }
        if changed {
            response.mark_changed();
        }
        crate::cursor::on_hover_action(&response);
        let selected = self.selected.to_string();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(
                WidgetType::ComboBox,
                enabled,
                self.label.clone().unwrap_or_else(|| selected.clone()),
            );
            info.current_text_value = Some(selected.clone());
            info
        });
        response
    }
}
