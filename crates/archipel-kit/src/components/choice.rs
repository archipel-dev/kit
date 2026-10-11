//! Checkboxes, radio buttons and toggle switches.

use super::common;
use crate::theme::{Colors, Theme};
use crate::tokens::{size, typography};
use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo,
    WidgetType, pos2, vec2,
};

const BOX: f32 = 16.0;

/// Allocates a control of `control_width` followed by `label`, returning the whole row's
/// response and the control's rect.
fn control_row(
    ui: &mut Ui,
    control: egui::Vec2,
    label: &str,
    enabled: bool,
) -> (Response, Rect, Option<std::sync::Arc<egui::Galley>>) {
    let galley =
        (!label.is_empty()).then(|| common::galley(ui, label, typography::body(typography::BODY)));
    let text_width = galley
        .as_ref()
        .map_or(0.0, |g| size::ICON_TEXT_GAP + g.size().x);
    let height = size::ROW_HEIGHT.max(control.y);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(control.x + text_width, height), sense);
    let control_rect = Rect::from_min_size(
        pos2(rect.left(), rect.center().y - control.y / 2.0),
        control,
    );
    (response, control_rect, galley)
}

fn paint_label(
    ui: &Ui,
    response: &Response,
    control: Rect,
    galley: Option<std::sync::Arc<egui::Galley>>,
    enabled: bool,
) {
    if let Some(galley) = galley {
        let colors = Theme::colors(ui.ctx());
        common::text_left(
            ui.painter(),
            response.rect,
            control.right() + size::ICON_TEXT_GAP,
            galley,
            common::enabled_text(enabled, colors.text, &colors),
        );
    }
}

/// Checked, unchecked or mixed (some children checked).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckState {
    Off,
    On,
    Mixed,
}

/// A checkbox with a label. Clicking the box or the label toggles it.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Checkbox;
///
/// let mut format_on_save = true;
/// ui.add(Checkbox::new(&mut format_on_save, "Reformat code on save"));
/// # });
/// ```
#[must_use = "add it with `ui.add(checkbox)`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: String,
    enabled: bool,
    mixed: bool,
    label_visible: bool,
}

impl<'a> Checkbox<'a> {
    /// A checkbox bound to `checked`.
    pub fn new(checked: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            checked,
            label: label.into(),
            enabled: true,
            mixed: false,
            label_visible: true,
        }
    }

    /// Whether the label is drawn next to the box (default). Without it the label is only
    /// the accessible name: use it where another widget names the box, e.g. in the
    /// [leading slot](crate::components::ListRow::leading) of a row.
    pub fn label_visible(mut self, visible: bool) -> Self {
        self.label_visible = visible;
        self
    }

    /// Shows a dash instead of a check mark: some, but not all, children are checked.
    /// Clicking a mixed checkbox checks it.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// Disabled checkboxes are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Color of a tick or dot on an accent fill. Disabled marks must stay visible: in dark the
/// disabled text color is as dark as the disabled accent, so they use the on-accent color.
fn mark_color(ui: &Ui, colors: &Colors, enabled: bool) -> Color32 {
    if enabled || ui.visuals().dark_mode {
        colors.text_on_accent
    } else {
        colors.text_disabled
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let shown = if self.label_visible { &self.label } else { "" };
        let (mut response, rect, galley) = control_row(ui, vec2(BOX, BOX), shown, self.enabled);
        if response.clicked() {
            *self.checked = self.mixed || !*self.checked;
            response.mark_changed();
        }
        let state = match (*self.checked, self.mixed && !response.changed()) {
            (_, true) => CheckState::Mixed,
            (true, false) => CheckState::On,
            (false, false) => CheckState::Off,
        };
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                self.enabled,
                state == CheckState::On,
                &self.label,
            )
        });
        if ui.is_rect_visible(response.rect) {
            let painter = ui.painter();
            let corner = CornerRadius::same(3);
            let filled = state != CheckState::Off;
            let (fill, stroke) = match (filled, self.enabled) {
                (true, true) => (colors.accent, Stroke::NONE),
                (true, false) => (colors.accent_disabled, Stroke::NONE),
                (false, enabled) => (
                    colors.field,
                    Stroke::new(
                        1.0_f32,
                        if enabled && response.hovered() {
                            colors.text_secondary
                        } else if enabled {
                            colors.control_border
                        } else {
                            colors.border
                        },
                    ),
                ),
            };
            painter.rect(rect, corner, fill, stroke, StrokeKind::Inside);
            if response.has_focus() {
                common::focus_ring(painter, rect, 3, &colors);
            }
            let mark = Stroke::new(1.6_f32, mark_color(ui, &colors, self.enabled));
            let p = |x: f32, y: f32| rect.min + vec2(x, y);
            match state {
                CheckState::On => {
                    painter.line(vec![p(4.0, 8.2), p(7.0, 11.0), p(12.0, 5.2)], mark);
                }
                CheckState::Mixed => {
                    painter.line_segment([p(4.5, 8.0), p(11.5, 8.0)], mark);
                }
                CheckState::Off => {}
            }
            paint_label(ui, &response, rect, galley, self.enabled);
        }
        response
    }
}

/// One option of a radio group: selected when `*current == value`; clicking selects it.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Radio;
///
/// #[derive(PartialEq, Clone, Copy)]
/// enum Indent { Tabs, Spaces }
/// let mut indent = Indent::Spaces;
/// ui.add(Radio::new(&mut indent, Indent::Tabs, "Tabs"));
/// ui.add(Radio::new(&mut indent, Indent::Spaces, "Spaces"));
/// # });
/// ```
#[must_use = "add it with `ui.add(radio)`"]
pub struct Radio<'a, T: PartialEq> {
    current: &'a mut T,
    value: T,
    label: String,
    enabled: bool,
}

impl<'a, T: PartialEq> Radio<'a, T> {
    /// The option `value` of the group whose selection is `current`.
    pub fn new(current: &'a mut T, value: T, label: impl Into<String>) -> Self {
        Self {
            current,
            value,
            label: label.into(),
            enabled: true,
        }
    }

    /// Disabled options are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<T: PartialEq> Widget for Radio<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (mut response, rect, galley) =
            control_row(ui, vec2(BOX, BOX), &self.label, self.enabled);
        let mut selected = *self.current == self.value;
        if response.clicked() && !selected {
            *self.current = self.value;
            selected = true;
            response.mark_changed();
        }
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, self.enabled, selected, &self.label)
        });
        if ui.is_rect_visible(response.rect) {
            let painter = ui.painter();
            let center = rect.center();
            if selected {
                let fill = if self.enabled {
                    colors.accent
                } else {
                    colors.accent_disabled
                };
                painter.circle_filled(center, BOX / 2.0, fill);
                let dot = mark_color(ui, &colors, self.enabled);
                painter.circle_filled(center, 3.0, dot);
            } else {
                let outline = if self.enabled && response.hovered() {
                    colors.text_secondary
                } else if self.enabled {
                    colors.control_border
                } else {
                    colors.border
                };
                painter.circle(
                    center,
                    BOX / 2.0 - 0.5,
                    colors.field,
                    Stroke::new(1.0_f32, outline),
                );
            }
            if response.has_focus() {
                painter.circle_stroke(
                    center,
                    BOX / 2.0 + 1.0 + size::FOCUS_RING / 2.0,
                    Stroke::new(size::FOCUS_RING, colors.focus_ring),
                );
            }
            paint_label(ui, &response, rect, galley, self.enabled);
        }
        response
    }
}

/// An on/off switch with a label, for settings that apply immediately.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Toggle;
///
/// let mut auto_save = true;
/// ui.add(Toggle::new(&mut auto_save, "Save files automatically"));
/// # });
/// ```
#[must_use = "add it with `ui.add(toggle)`"]
pub struct Toggle<'a> {
    on: &'a mut bool,
    label: String,
    enabled: bool,
}

impl<'a> Toggle<'a> {
    /// A switch bound to `on`.
    pub fn new(on: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            on,
            label: label.into(),
            enabled: true,
        }
    }

    /// Disabled switches are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for Toggle<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        const TRACK: egui::Vec2 = vec2(28.0, 16.0);
        let colors = Theme::colors(ui.ctx());
        let (mut response, rect, galley) = control_row(ui, TRACK, &self.label, self.enabled);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        crate::cursor::on_hover_action(&response);
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Checkbox, self.enabled, on, &self.label)
        });
        if ui.is_rect_visible(response.rect) {
            let t = ui.ctx().animate_bool_responsive(response.id, on);
            let painter = ui.painter();
            let (track, outline, knob) = if !self.enabled {
                (
                    colors.accent_disabled,
                    colors.accent_disabled,
                    colors.text_disabled,
                )
            } else {
                (
                    colors.toggle_off.lerp_to_gamma(colors.toggle_on, t),
                    colors.toggle_off_knob.lerp_to_gamma(colors.toggle_on, t),
                    colors.toggle_off_knob.lerp_to_gamma(Color32::WHITE, t),
                )
            };
            painter.rect(
                rect,
                CornerRadius::same(8),
                track,
                Stroke::new(1.0_f32, outline),
                StrokeKind::Inside,
            );
            if response.has_focus() {
                common::focus_ring(painter, rect, 8, &colors);
            }
            let knob_x = egui::lerp(rect.left() + 8.0..=rect.right() - 8.0, t);
            painter.circle_filled(pos2(knob_x, rect.center().y), 5.0, knob);
            paint_label(ui, &response, rect, galley, self.enabled);
        }
        response
    }
}
