//! Titled and plain separators.

use crate::common;
use crate::icons::Glyph;
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{Sense, Stroke, Ui, Widget, vec2};

/// A section title with a hairline, separating groups inside a panel or
/// a settings page. [`collapsible`](Self::collapsible) adds a chevron that shows and hides
/// the group (Android Studio's "Advanced settings").
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::TitledSeparator;
///
/// ui.add(TitledSeparator::new("Code style"));
///
/// let mut advanced = false;
/// if ui.add(TitledSeparator::new("Advanced").collapsible(&mut advanced)).changed() {
///     // `advanced` was toggled.
/// }
/// # });
/// ```
#[must_use = "add it with `ui.add(header)`"]
pub struct TitledSeparator<'a> {
    title: String,
    open: Option<&'a mut bool>,
    chevron_at_end: bool,
}

impl TitledSeparator<'static> {
    /// A header titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: None,
            chevron_at_end: false,
        }
    }
}

impl<'a> TitledSeparator<'a> {
    /// Makes the header show and hide its group: a chevron before the title, pointing
    /// down while `*open`. A click, <kbd>Enter</kbd> or <kbd>Space</kbd> toggle it; the
    /// response is `changed()` then. Show the group's rows only while `*open`.
    pub fn collapsible(self, open: &'a mut bool) -> TitledSeparator<'a> {
        TitledSeparator {
            title: self.title,
            open: Some(open),
            chevron_at_end: self.chevron_at_end,
        }
    }

    /// Puts the chevron of a [`collapsible`](Self::collapsible) header at the right end of
    /// the line instead of before the title (`CollapsiblePanel`'s `collapseButtonAtLeft` off).
    pub fn chevron_at_end(mut self, at_end: bool) -> Self {
        self.chevron_at_end = at_end;
        self
    }
}

impl Widget for TitledSeparator<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(ui, self.title.as_str(), typography::body(typography::BODY));
        // Space above separates the group from the previous one; the first group of a
        // page starts at the top, as on Android Studio's Settings pages.
        let first = ui.min_rect().height() <= 0.0;
        let above = if first { 0.0 } else { size::FORM_GROUP_SPACING };
        ui.add_space(above);
        let collapsible = self.open.is_some();
        let sense = if collapsible {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut response) =
            ui.allocate_exact_size(vec2(ui.available_width(), size::ROW_HEIGHT), sense);
        let at_end = self.chevron_at_end;
        let chevron_width = if collapsible && !at_end {
            size::ICON + size::ICON_TEXT_GAP
        } else {
            0.0
        };
        let enabled = ui.is_enabled();
        let mut open = self.open;
        if let Some(open) = open.as_deref_mut()
            && response.clicked()
        {
            *open = !*open;
            response.mark_changed();
        }
        let expanded = open.as_deref().copied();
        response.widget_info(|| match expanded {
            Some(_) => {
                egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, enabled, &self.title)
            }
            None => egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.title),
        });
        if let Some(expanded) = expanded {
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_expanded(expanded));
        }
        let text_left = rect.left() + chevron_width;
        let text_right = text_left + galley.size().x;
        let top = rect.center().y - galley.size().y / 2.0;
        // Measured: the line runs 4 pt above the title's baseline, to the right edge.
        let y = (top + common::baseline(&galley) - 4.0).round() + 0.5;
        let text_color = common::enabled_text(enabled, colors.text, &colors);
        if let Some(expanded) = expanded {
            let glyph = if expanded {
                Glyph::ChevronDown
            } else {
                Glyph::ChevronRight
            };
            crate::icons::paint(
                ui.painter(),
                egui::Rect::from_center_size(
                    egui::pos2(
                        if at_end {
                            rect.right() - size::ICON / 2.0
                        } else {
                            rect.left() + size::ICON / 2.0
                        },
                        rect.center().y,
                    ),
                    vec2(size::ICON, size::ICON),
                ),
                glyph,
                text_color,
            );
            if response.has_focus() {
                // Around the chevron and title, not the line (at the end: around the whole
                // row, as the chevron is far from the title).
                let target = if at_end {
                    rect
                } else {
                    egui::Rect::from_min_max(
                        rect.left_top(),
                        egui::pos2(
                            (text_right + size::ICON_TEXT_GAP).min(rect.right()),
                            rect.bottom(),
                        ),
                    )
                };
                common::focus_ring(ui.painter(), target, radius::CONTROL, &colors);
            }
        }
        common::text(ui.painter(), egui::pos2(text_left, top), galley, text_color);
        let start = text_right + size::GROUP_LINE_GAP;
        // A chevron at the end keeps its room: the line stops before it.
        let line_end = if at_end && collapsible {
            rect.right() - size::ICON - size::ICON_TEXT_GAP
        } else {
            rect.right()
        };
        if start < line_end {
            ui.painter().hline(
                start..=line_end,
                y,
                Stroke::new(1.0_f32, colors.group_separator),
            );
        }
        response
    }
}

/// A 1 px separator line across the available width (or height in a horizontal layout).
#[must_use = "add it with `ui.add(separator)`"]
#[derive(Default)]
pub struct SeparatorComponent;

impl SeparatorComponent {
    /// The vertical line between groups of actions in an island header or toolbar,
    /// [`size::HEADER_DIVIDER`] long, as in Android Studio.
    pub fn group() -> impl Widget {
        Self::length(size::HEADER_DIVIDER)
    }

    /// A separator with an explicit line length in points, centered in its layout.
    /// Useful for action groups inside panel headers. Invalid lengths become zero.
    pub fn length(points: f32) -> impl Widget {
        SizedSeparator(if points.is_finite() {
            points.max(0.0)
        } else {
            0.0
        })
    }
}

struct SizedSeparator(f32);
impl Widget for SizedSeparator {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let vertical = ui.layout().main_dir().is_horizontal();
        let length = self
            .0
            .min(if vertical {
                ui.available_height()
            } else {
                ui.available_width()
            })
            .max(0.0);
        let (rect, response) = ui.allocate_exact_size(
            if vertical {
                vec2(9.0, length)
            } else {
                vec2(length, 9.0)
            },
            Sense::hover(),
        );
        let stroke = Stroke::new(1.0_f32, Theme::colors(ui.ctx()).separator);
        if vertical {
            ui.painter()
                .vline(rect.center().x.round() + 0.5, rect.y_range(), stroke);
        } else {
            ui.painter()
                .hline(rect.x_range(), rect.center().y.round() + 0.5, stroke);
        }
        response
    }
}

impl Widget for SeparatorComponent {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let colors = Theme::colors(ui.ctx());
        let horizontal = ui.layout().main_dir().is_horizontal();
        let size = if horizontal {
            vec2(9.0, ui.available_height().min(size::ICON_BUTTON))
        } else {
            vec2(ui.available_width(), 9.0)
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let stroke = Stroke::new(1.0_f32, colors.separator);
        if horizontal {
            ui.painter()
                .vline(rect.center().x.round() + 0.5, rect.y_range(), stroke);
        } else {
            ui.painter()
                .hline(rect.x_range(), rect.center().y.round() + 0.5, stroke);
        }
        response
    }
}
