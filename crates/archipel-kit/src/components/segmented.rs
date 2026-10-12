//! Segmented controls and tab bars: pick one of a few options.

use super::common;
use crate::icons::{self, Glyph, Icon};
use crate::theme::Theme;
use crate::tokens::{radius, size, tab, typography};
use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// A row of connected buttons of which exactly one is selected, for switching views or
/// modes (e.g. *Body | Headers | Auth*).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::SegmentedControl;
///
/// #[derive(PartialEq, Clone, Copy)]
/// enum View { Pretty, Raw, Preview }
/// let mut view = View::Pretty;
/// ui.add(SegmentedControl::new(
///     &mut view,
///     [(View::Pretty, "Pretty"), (View::Raw, "Raw"), (View::Preview, "Preview")],
/// ));
/// # });
/// ```
#[must_use = "add it with `ui.add(control)`"]
pub struct SegmentedControl<'a, T: PartialEq + Clone> {
    selected: &'a mut T,
    options: Vec<(T, String, Option<Icon>)>,
    enabled: bool,
    icon_only: bool,
}

impl<'a, T: PartialEq + Clone> SegmentedControl<'a, T> {
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

impl<T: PartialEq + Clone> Widget for SegmentedControl<'_, T> {
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
                super::Tooltip::new(label).show_for(&segment);
            }
        }
        response
    }
}

/// One tab of a [`TabBar`].
#[derive(Debug, Clone)]
pub struct Tab {
    label: String,
    icon: Option<Icon>,
    closable: bool,
    modified: bool,
}

impl Tab {
    /// A tab titled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            closable: false,
            modified: false,
        }
    }

    /// Icon before the title (e.g. [`Icon::for_file`]).
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Shows a close button (always on the selected tab, on hover on the others).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Marks unsaved changes with a dot after the title.
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }
}

/// What happened in a [`TabBar`] this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TabBarResponse {
    /// Index of the tab the user selected, if they selected another one.
    pub selected: Option<usize>,
    /// Index of the tab whose close button was clicked (or that was middle-clicked).
    pub closed: Option<usize>,
}

/// Editor tabs as in Android Studio's Islands UI: the selected tab is a tinted pill with a
/// border; others show a hover pill and their close button on hover.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Icon, components::{Tab, TabBar}};
///
/// let mut current = 0;
/// let tabs = [
///     Tab::new("main.rs").icon(Icon::for_file("main.rs")).closable(true),
///     Tab::new("Cargo.toml").icon(Icon::for_file("Cargo.toml")).closable(true).modified(true),
/// ];
/// let out = TabBar::new(&tabs, current).show(ui);
/// if let Some(index) = out.selected { current = index; }
/// # });
/// ```
pub struct TabBar<'a> {
    tabs: &'a [Tab],
    selected: usize,
}

impl<'a> TabBar<'a> {
    /// `tabs` with the one at `selected` active.
    pub fn new(tabs: &'a [Tab], selected: usize) -> Self {
        Self { tabs, selected }
    }

    /// Draws the tabs in a row, [`size::HEADER_HEIGHT`] high: put it at the top of an
    /// island, where Android Studio has its editor tabs.
    pub fn show(self, ui: &mut Ui) -> TabBarResponse {
        let colors = Theme::colors(ui.ctx());
        let mut out = TabBarResponse::default();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = tab::GAP;
            for (index, item) in self.tabs.iter().enumerate() {
                let selected = index == self.selected;
                let galley =
                    common::galley(ui, item.label.as_str(), typography::body(typography::BODY));
                let icon_w = if item.icon.is_some() {
                    size::ICON + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
                let dot_w = if item.modified { 10.0 } else { 0.0 };
                // The close button's room is kept on every closable tab, so showing it on
                // hover does not move the tabs (as in Android Studio).
                let end_w = if item.closable {
                    tab::CLOSE_GAP + tab::CLOSE + tab::PADDING_RIGHT
                } else {
                    tab::PADDING_LEFT
                };
                let width = tab::PADDING_LEFT + icon_w + galley.size().x + dot_w + end_w;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, size::HEADER_HEIGHT), Sense::click());
                let pill = Rect::from_center_size(rect.center(), vec2(width, tab::HEIGHT));
                crate::cursor::on_hover(&response, egui::CursorIcon::Default);
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &item.label)
                });
                if response.clicked() && !selected {
                    out.selected = Some(index);
                }
                if response.middle_clicked() && item.closable {
                    out.closed = Some(index);
                }
                let painter = ui.painter().clone();
                let corner = CornerRadius::same(radius::TAB);
                if selected {
                    // The selected tab: a tinted pill with a 1 pt border.
                    painter.rect_filled(pill, corner, colors.tab_selected);
                    painter.rect_stroke(
                        pill,
                        corner,
                        Stroke::new(1.0_f32, colors.tab_selected_border),
                        egui::StrokeKind::Inside,
                    );
                } else if response.hovered() {
                    painter.rect_filled(pill, corner, colors.hover);
                }
                if response.has_focus() {
                    crate::parts::focus_ring(&painter, pill, radius::TAB, &colors);
                }
                let mut x = pill.left() + tab::PADDING_LEFT;
                if let Some(icon) = item.icon {
                    let icon_rect = Rect::from_min_size(
                        pos2(x, pill.center().y - size::ICON / 2.0),
                        vec2(size::ICON, size::ICON),
                    );
                    icons::paint(&painter, icon_rect, icon, colors.icon);
                    x += icon_w;
                }
                // Hover brings an unselected title up to full strength, as in Android Studio.
                let text_color = if selected || response.hovered() {
                    colors.text
                } else {
                    colors.tab_text
                };
                let text_width = galley.size().x;
                common::text_left(&painter, pill, x, galley, text_color);
                x += text_width;
                if item.modified {
                    painter.circle_filled(
                        pos2(x + 6.0, pill.center().y),
                        3.0,
                        colors.text_secondary,
                    );
                }
                if item.closable && (selected || response.hovered()) {
                    let center = pos2(
                        pill.right() - tab::PADDING_RIGHT - tab::CLOSE / 2.0,
                        pill.center().y,
                    );
                    let close_rect = Rect::from_center_size(
                        center,
                        vec2(2.0 * tab::CLOSE_HOVER_RADIUS, 2.0 * tab::CLOSE_HOVER_RADIUS),
                    );
                    let close = ui.interact(close_rect, response.id.with("close"), Sense::click());
                    crate::cursor::on_hover_action(&close);
                    close.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            true,
                            format!("Close {}", item.label),
                        )
                    });
                    // A round hover behind the cross, lighter than the tab's.
                    if close.hovered() {
                        painter.circle_filled(center, tab::CLOSE_HOVER_RADIUS, colors.hover);
                    }
                    let glyph =
                        Rect::from_center_size(center, vec2(tab::CLOSE_GLYPH, tab::CLOSE_GLYPH));
                    icons::paint(
                        &painter,
                        glyph,
                        Glyph::Close,
                        if close.hovered() {
                            colors.text_secondary
                        } else {
                            colors.text_placeholder
                        },
                    );
                    if close.clicked() {
                        out.closed = Some(index);
                        out.selected = None;
                    }
                }
            }
        });
        out
    }
}
