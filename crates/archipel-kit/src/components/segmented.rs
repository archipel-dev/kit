//! Segmented controls and tab bars: pick one of a few options.

use super::common;
use crate::icons::{self, Glyph, Icon};
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
    options: Vec<(T, String)>,
    enabled: bool,
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
                .map(|(value, label)| (value, label.into()))
                .collect(),
            enabled: true,
        }
    }

    /// Disabled controls are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
            .map(|(_, label)| {
                common::galley(ui, label.as_str(), typography::body(typography::BODY))
            })
            .collect();
        let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 2.0 * padding).collect();
        let total = widths.iter().sum::<f32>() + 4.0;
        let (outer, mut response) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
        let painter = ui.painter().clone();
        painter.rect(
            outer,
            CornerRadius::same(radius::CONTROL + 2),
            colors.frame,
            Stroke::new(1.0_f32, colors.border),
            StrokeKind::Inside,
        );
        let mut x = outer.left() + 2.0;
        for (index, ((value, label), galley)) in self.options.iter().zip(galleys).enumerate() {
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
            segment.widget_info(|| {
                WidgetInfo::selected(WidgetType::RadioButton, self.enabled, selected, label)
            });
            let fill = if selected {
                colors.island
            } else if self.enabled {
                common::flat_fill(&segment, &colors)
            } else {
                Color32::TRANSPARENT
            };
            let stroke = if selected {
                Stroke::new(1.0_f32, colors.border)
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
            let pos = rect.center() - galley.size() / 2.0;
            common::text(&painter, pos, galley, color);
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

/// Editor-style tabs: the selected tab is tinted and underlined.
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

    /// Draws the tabs in a row.
    pub fn show(self, ui: &mut Ui) -> TabBarResponse {
        let colors = Theme::colors(ui.ctx());
        let mut out = TabBarResponse::default();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (index, tab) in self.tabs.iter().enumerate() {
                let selected = index == self.selected;
                let galley =
                    common::galley(ui, tab.label.as_str(), typography::body(typography::BODY));
                let icon_w = if tab.icon.is_some() {
                    size::ICON + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
                let dot_w = if tab.modified { 10.0 } else { 0.0 };
                let close_w = if tab.closable { 20.0 } else { 0.0 };
                let width = 12.0 + icon_w + galley.size().x + dot_w + close_w + 12.0;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, size::HEADER_HEIGHT), Sense::click());
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &tab.label)
                });
                if response.clicked() && !selected {
                    out.selected = Some(index);
                }
                if response.middle_clicked() && tab.closable {
                    out.closed = Some(index);
                }
                let painter = ui.painter().clone();
                if selected {
                    painter.rect_filled(rect, CornerRadius::ZERO, colors.tab_selected);
                    let underline =
                        Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
                    painter.rect_filled(underline, CornerRadius::ZERO, colors.tab_selected_border);
                } else if response.hovered() {
                    painter.rect_filled(rect, CornerRadius::ZERO, colors.hover);
                }
                let mut x = rect.left() + 12.0;
                if let Some(icon) = tab.icon {
                    let icon_rect =
                        Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0));
                    icons::paint(&painter, icon_rect, icon, colors.icon);
                    x += icon_w;
                }
                let text_color = if selected {
                    colors.text_strong
                } else {
                    colors.text
                };
                let text_width = galley.size().x;
                common::text_left(&painter, rect, x, galley, text_color);
                x += text_width;
                if tab.modified {
                    painter.circle_filled(
                        pos2(x + 6.0, rect.center().y),
                        3.0,
                        colors.text_secondary,
                    );
                    x += dot_w;
                }
                if tab.closable && (selected || response.hovered()) {
                    let close_rect =
                        Rect::from_center_size(pos2(x + 11.0, rect.center().y), vec2(18.0, 18.0));
                    let close = ui.interact(close_rect, response.id.with("close"), Sense::click());
                    close.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            true,
                            format!("Close {}", tab.label),
                        )
                    });
                    if close.hovered() {
                        painter.rect_filled(
                            close_rect,
                            CornerRadius::same(radius::CONTROL),
                            colors.hover,
                        );
                    }
                    icons::paint(
                        &painter,
                        close_rect.shrink(3.0),
                        Glyph::Close,
                        colors.text_secondary,
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
