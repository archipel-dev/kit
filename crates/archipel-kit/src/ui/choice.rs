//! Checkboxes, radio buttons and toggle switches.

use crate::common;
use crate::theme::{Colors, Theme};
use crate::tokens::{radius, size, typography};
use egui::{
    Color32, CornerRadius, Galley, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget,
    WidgetInfo, WidgetType, pos2, vec2,
};
use std::sync::Arc;

const BOX: f32 = 16.0;

/// Comments that explain a checkbox or radio, as on Android Studio's Settings pages: one
/// next to the label (`Requires restart`) and one below it, wrapped, aligned with the label.
#[derive(Default)]
struct Comments {
    inline: Option<String>,
    below: Option<String>,
}

/// The laid-out label of a control row.
struct RowText {
    label: Option<Arc<Galley>>,
    inline: Option<Arc<Galley>>,
}

/// Allocates a control of `control` size followed by its label and inline comment,
/// returning the whole row's response and the control's rect.
fn control_row(
    ui: &mut Ui,
    control: egui::Vec2,
    label: &str,
    inline: Option<&str>,
    enabled: bool,
) -> (Response, Rect, RowText) {
    let label =
        (!label.is_empty()).then(|| common::galley(ui, label, typography::body(typography::BODY)));
    let inline = inline.map(|text| common::galley(ui, text, typography::body(typography::COMMENT)));
    let text_width = label
        .as_ref()
        .map_or(0.0, |g| size::CHECK_LABEL_GAP + g.size().x)
        + inline
            .as_ref()
            .map_or(0.0, |g| size::INLINE_COMMENT_GAP + g.size().x);
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
    (response, control_rect, RowText { label, inline })
}

fn paint_label(ui: &Ui, response: &Response, control: Rect, text: RowText, enabled: bool) {
    let colors = Theme::colors(ui.ctx());
    let mut x = control.right() + size::CHECK_LABEL_GAP;
    let mut label_baseline = None;
    if let Some(galley) = text.label {
        let y = response.rect.center().y - galley.size().y / 2.0;
        label_baseline = Some(y + common::baseline(&galley));
        x += galley.size().x + size::INLINE_COMMENT_GAP;
        common::text(
            ui.painter(),
            pos2(control.right() + size::CHECK_LABEL_GAP, y),
            galley,
            common::enabled_text(enabled, colors.text, &colors),
        );
    }
    if let Some(galley) = text.inline {
        // On the label's baseline, in the smaller comment text.
        let y = label_baseline.map_or_else(
            || response.rect.center().y - galley.size().y / 2.0,
            |base| base - common::baseline(&galley),
        );
        common::text(
            ui.painter(),
            pos2(x, y),
            galley,
            common::enabled_text(enabled, colors.text_placeholder, &colors),
        );
    }
}

/// Makes a comment below a control its accessible description.
fn describe(ui: &Ui, response: &Response, description: Option<&str>) {
    if let Some(description) = description {
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_description(description));
    }
}

/// Adds a control row and, if there is one, its comment below, aligned with the label.
fn with_comment(
    ui: &mut Ui,
    comment: Option<&str>,
    enabled: bool,
    row: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    let Some(comment) = comment else {
        return row(ui);
    };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let response = row(ui);
        let indent = BOX + size::CHECK_LABEL_GAP;
        let wrap = (ui.available_width() - indent).max(size::ICON);
        let mut job = egui::text::LayoutJob::default();
        job.append(
            comment,
            0.0,
            egui::TextFormat {
                font_id: typography::body(typography::COMMENT),
                line_height: Some(size::COMMENT_LINE_HEIGHT),
                color: Color32::PLACEHOLDER,
                ..Default::default()
            },
        );
        job.wrap.max_width = wrap;
        let galley = ui.painter().layout_job(job);
        let lines = galley.rows.len().max(1) as f32;
        let (rect, _) = ui.allocate_exact_size(
            vec2(
                indent + galley.size().x,
                size::COMMENT_SPACE + lines * size::COMMENT_LINE_HEIGHT,
            ),
            Sense::hover(),
        );
        let colors = Theme::colors(ui.ctx());
        common::text(
            ui.painter(),
            pos2(rect.left() + indent, rect.top() + size::COMMENT_GAP),
            galley,
            common::enabled_text(enabled, colors.text_placeholder, &colors),
        );
        response
    })
    .inner
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
/// use archipel_kit::ui::JBCheckBox;
///
/// let mut format_on_save = true;
/// ui.add(JBCheckBox::new(&mut format_on_save, "Reformat code on save"));
/// # });
/// ```
#[must_use = "add it with `ui.add(checkbox)`"]
pub struct JBCheckBox<'a> {
    checked: &'a mut bool,
    label: String,
    enabled: bool,
    mixed: bool,
    label_visible: bool,
    comments: Comments,
}

impl<'a> JBCheckBox<'a> {
    /// A checkbox bound to `checked`.
    pub fn new(checked: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            checked,
            label: label.into(),
            enabled: true,
            mixed: false,
            label_visible: true,
            comments: Comments::default(),
        }
    }

    /// A short note after the label, in smaller gray text: `Requires restart`.
    pub fn inline_comment(mut self, comment: impl Into<String>) -> Self {
        self.comments.inline = Some(comment.into());
        self
    }

    /// An explanation below the checkbox, in smaller gray text aligned with the label and
    /// wrapped to the available width. It is also the checkbox's accessible description.
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comments.below = Some(comment.into());
        self
    }

    /// Whether the label is drawn next to the box (default). Without it the label is only
    /// the accessible name: use it where another widget names the box, e.g. in the
    /// [leading slot](crate::ui::ColoredListCellRenderer::leading) of a row.
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

impl Widget for JBCheckBox<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let below = self.comments.below.take();
        let enabled = self.enabled;
        with_comment(ui, below.as_deref(), enabled, |ui| {
            self.row(ui, below.as_deref())
        })
    }
}

impl JBCheckBox<'_> {
    fn row(self, ui: &mut Ui, description: Option<&str>) -> Response {
        let colors = Theme::colors(ui.ctx());
        let shown = if self.label_visible { &self.label } else { "" };
        let inline = self
            .comments
            .inline
            .as_deref()
            .filter(|_| self.label_visible);
        let (mut response, rect, galley) =
            control_row(ui, vec2(BOX, BOX), shown, inline, self.enabled);
        describe(ui, &response, description);
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
            let corner = CornerRadius::same(radius::CHECKBOX);
            let filled = state != CheckState::Off;
            // Measured on Android Studio: no hover change; checked in its own blue.
            let (fill, stroke) = match (filled, self.enabled) {
                (true, true) => (colors.control_selected, Stroke::NONE),
                (true, false) => (colors.accent_disabled, Stroke::NONE),
                (false, enabled) => (
                    colors.control,
                    Stroke::new(
                        1.0_f32,
                        if enabled {
                            colors.control_border
                        } else {
                            colors.border_disabled
                        },
                    ),
                ),
            };
            painter.rect(rect, corner, fill, stroke, StrokeKind::Inside);
            if response.has_focus() {
                // The border becomes 2 pt in the checkbox blue, one of them outside.
                painter.rect_stroke(
                    rect.expand(1.0),
                    CornerRadius::same(radius::CHECKBOX + 1),
                    Stroke::new(size::FOCUS_RING, colors.control_selected),
                    StrokeKind::Inside,
                );
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
/// use archipel_kit::ui::JBRadioButton;
///
/// #[derive(PartialEq, Clone, Copy)]
/// enum Indent { Tabs, Spaces }
/// let mut indent = Indent::Spaces;
/// ui.add(JBRadioButton::new(&mut indent, Indent::Tabs, "Tabs"));
/// ui.add(JBRadioButton::new(&mut indent, Indent::Spaces, "Spaces"));
/// # });
/// ```
#[must_use = "add it with `ui.add(radio)`"]
pub struct JBRadioButton<'a, T: PartialEq> {
    current: &'a mut T,
    value: T,
    label: String,
    enabled: bool,
    comments: Comments,
}

impl<'a, T: PartialEq> JBRadioButton<'a, T> {
    /// The option `value` of the group whose selection is `current`.
    pub fn new(current: &'a mut T, value: T, label: impl Into<String>) -> Self {
        Self {
            current,
            value,
            label: label.into(),
            enabled: true,
            comments: Comments::default(),
        }
    }

    /// A short note after the label, in smaller gray text.
    pub fn inline_comment(mut self, comment: impl Into<String>) -> Self {
        self.comments.inline = Some(comment.into());
        self
    }

    /// An explanation below the option, in smaller gray text aligned with the label and
    /// wrapped to the available width. It is also the option's accessible description.
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comments.below = Some(comment.into());
        self
    }

    /// Disabled options are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<T: PartialEq> Widget for JBRadioButton<'_, T> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let below = self.comments.below.take();
        let enabled = self.enabled;
        with_comment(ui, below.as_deref(), enabled, |ui| {
            self.row(ui, below.as_deref())
        })
    }
}

impl<T: PartialEq> JBRadioButton<'_, T> {
    fn row(self, ui: &mut Ui, description: Option<&str>) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (mut response, rect, galley) = control_row(
            ui,
            vec2(BOX, BOX),
            &self.label,
            self.comments.inline.as_deref(),
            self.enabled,
        );
        describe(ui, &response, description);
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
                    colors.control_selected
                } else {
                    colors.accent_disabled
                };
                painter.circle_filled(center, BOX / 2.0, fill);
                let dot = mark_color(ui, &colors, self.enabled);
                painter.circle_filled(center, 3.0, dot);
            } else {
                let outline = if self.enabled {
                    colors.control_border
                } else {
                    colors.border_disabled
                };
                painter.circle(
                    center,
                    BOX / 2.0 - 0.5,
                    colors.control,
                    Stroke::new(1.0_f32, outline),
                );
            }
            if response.has_focus() {
                // Like the checkbox: a 2 pt border in the checkbox blue, one of them outside.
                painter.circle_stroke(
                    center,
                    BOX / 2.0,
                    Stroke::new(size::FOCUS_RING, colors.control_selected),
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
/// use archipel_kit::ui::OnOffButton;
///
/// let mut auto_save = true;
/// ui.add(OnOffButton::new(&mut auto_save, "Save files automatically"));
/// # });
/// ```
#[must_use = "add it with `ui.add(toggle)`"]
pub struct OnOffButton<'a> {
    on: &'a mut bool,
    label: String,
    enabled: bool,
}

impl<'a> OnOffButton<'a> {
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

impl Widget for OnOffButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        const TRACK: egui::Vec2 = vec2(28.0, 16.0);
        let colors = Theme::colors(ui.ctx());
        let (mut response, rect, galley) = control_row(ui, TRACK, &self.label, None, self.enabled);
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
                CornerRadius::same(radius::PILL),
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
