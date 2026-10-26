//! The segmented button.

use crate::common;
use crate::icons::{self, Icon};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// A row of connected buttons of which exactly one is selected, for switching views or
/// modes (e.g. *Body | Headers | Auth*).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::SegmentedButton;
///
/// #[derive(PartialEq, Clone, Copy)]
/// enum View { Pretty, Raw, Preview }
/// let mut view = View::Pretty;
/// ui.add(SegmentedButton::new(
///     &mut view,
///     [(View::Pretty, "Pretty"), (View::Raw, "Raw"), (View::Preview, "Preview")],
/// ));
/// # });
/// ```
#[must_use = "add it with `ui.add(control)`"]
pub struct SegmentedButton<'a, T: PartialEq + Clone> {
    selected: &'a mut T,
    options: Vec<(T, String, Option<Icon>)>,
    enabled: bool,
    icon_only: bool,
}

impl<'a, T: PartialEq + Clone> SegmentedButton<'a, T> {
    /// Options as `(value, label)` pairs; `selected` holds the current value.
    pub fn new<L: Into<String>>(
        selected: &'a mut T,
        options: impl IntoIterator<Item = (T, L)>,
    ) -> Self {
        Self {
            selected,
            options: options
                .into_iter()
                .map(|(value, label)| (value, label.into(), None))
                .collect(),
            enabled: true,
            icon_only: false,
        }
    }

    /// Disabled controls are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Icon before the label of the segment with `value`. Unknown values are ignored.
    pub fn icon(mut self, value: T, icon: impl Into<Icon>) -> Self {
        if let Some(option) = self.options.iter_mut().find(|option| option.0 == value) {
            option.2 = Some(icon.into());
        }
        self
    }

    /// Hide labels on segments with icons, keeping the labels as accessible names and
    /// hover tooltips. Segments without an icon still show their label.
    pub fn icon_only(mut self, icon_only: bool) -> Self {
        self.icon_only = icon_only;
        self
    }
}

impl<T: PartialEq + Clone> Widget for SegmentedButton<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let height = size::CONTROL_HEIGHT;
        let padding = 12.0;
        let galleys: Vec<_> = self
            .options
            .iter()
            .map(|(_, label, icon)| {
                let text = if self.icon_only && icon.is_some() {
                    ""
                } else {
                    label.as_str()
                };
                common::galley(ui, text, typography::body(typography::BODY))
            })
            .collect();
        let widths: Vec<f32> = galleys
            .iter()
            .zip(&self.options)
            .map(|(g, (_, _, icon))| {
                g.size().x
                    + 2.0 * padding
                    + if icon.is_some() {
                        size::ICON
                            + if g.text().is_empty() {
                                0.0
                            } else {
                                size::ICON_TEXT_GAP
                            }
                    } else {
                        0.0
                    }
            })
            .collect();
        let total = widths.iter().sum::<f32>() + 4.0;
        let (outer, mut response) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
        if self.enabled && ui.is_enabled() {
            for index in 0..self.options.len() {
                if ui.memory(|m| m.has_focus(response.id.with(index))) {
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            response.id.with(index),
                            egui::EventFilter {
                                horizontal_arrows: true,
                                ..Default::default()
                            },
                        )
                    });
                    let next = ui.input_mut(|input| {
                        if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                            Some((index + 1).min(self.options.len() - 1))
                        } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                            Some(index.saturating_sub(1))
                        } else {
                            None
                        }
                    });
                    if let Some(next) = next {
                        if *self.selected != self.options[next].0 {
                            *self.selected = self.options[next].0.clone();
                            response.mark_changed();
                        }
                        ui.memory_mut(|m| {
                            m.request_focus(response.id.with(next));
                            m.move_focus(egui::FocusDirection::None);
                        });
                        ui.memory_mut(|m| {
                            m.set_focus_lock_filter(
                                response.id.with(next),
                                egui::EventFilter {
                                    horizontal_arrows: true,
                                    ..Default::default()
                                },
                            )
                        });
                    }
                    break;
                }
            }
        }
        let painter = ui.painter().clone();
        painter.rect(
            outer,
            CornerRadius::same(radius::CONTROL + 2),
            Color32::TRANSPARENT,
            Stroke::new(1.0_f32, colors.border),
            StrokeKind::Inside,
        );
        let mut x = outer.left() + 2.0;
        for (index, ((value, label, icon), galley)) in self.options.iter().zip(galleys).enumerate()
        {
            let rect = Rect::from_min_size(
                pos2(x, outer.top() + 2.0),
                vec2(widths[index], height - 4.0),
            );
            x += widths[index];
            let id = response.id.with(index);
            let sense = if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let segment = ui.interact(rect, id, sense);
            let mut selected = *self.selected == *value;
            if segment.clicked() && !selected {
                *self.selected = value.clone();
                selected = true;
                response.mark_changed();
            }
            crate::cursor::on_hover_action(&segment);
            segment.widget_info(|| {
                WidgetInfo::selected(WidgetType::RadioButton, self.enabled, selected, label)
            });
            let fill = if selected {
                colors.segment_selected
            } else if self.enabled {
                common::flat_fill(&segment, &colors)
            } else {
                Color32::TRANSPARENT
            };
            let stroke = if selected {
                Stroke::new(1.0_f32, colors.segment_selected_border)
            } else {
                Stroke::NONE
            };
            painter.rect(
                rect,
                CornerRadius::same(radius::CONTROL),
                fill,
                stroke,
                StrokeKind::Inside,
            );
            if segment.has_focus() {
                common::focus_ring(&painter, rect, radius::CONTROL, &colors);
            }
            let color = match (self.enabled, selected) {
                (false, _) => colors.text_disabled,
                (true, true) => colors.text_strong,
                (true, false) => colors.text_secondary,
            };
            let icon_width = if icon.is_some() {
                size::ICON
                    + if galley.text().is_empty() {
                        0.0
                    } else {
                        size::ICON_TEXT_GAP
                    }
            } else {
                0.0
            };
            let mut pos = rect.center() - vec2(galley.size().x + icon_width, galley.size().y) / 2.0;
            if let Some(icon) = icon {
                icons::paint(
                    &painter,
                    Rect::from_min_size(
                        pos2(pos.x, rect.center().y - size::ICON / 2.0),
                        vec2(size::ICON, size::ICON),
                    ),
                    *icon,
                    color,
                );
                pos.x += icon_width;
            }
            common::text(&painter, pos, galley, color);
            if self.icon_only && icon.is_some() {
                crate::ide::HelpTooltip::new(label).show_for(&segment);
            }
        }
        response
    }
}
