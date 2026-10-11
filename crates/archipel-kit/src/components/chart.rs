//! Charts: a histogram of bucketed counts and an inline sparkline.

use super::{Tooltip, common};
use crate::parts::{Tone, focus_ring};
use crate::theme::{Colors, Theme};
use crate::tokens::{radius, size, typography};
use egui::{
    CornerRadius, Key, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo,
    WidgetType, pos2, vec2,
};

/// Formats a number for labels and summaries: integers plain, others with up to two
/// decimals and no trailing zeros.
fn number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.2}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// How a chart colors its marks. Always resolved from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartColor {
    /// Series `n` of the categorical chart palette (wraps after 8).
    Series(usize),
    /// A semantic tone, e.g. [`Tone::Error`] for failing requests.
    Tone(Tone),
}

impl ChartColor {
    fn resolve(self, colors: &Colors) -> egui::Color32 {
        match self {
            ChartColor::Series(index) => colors.chart(index),
            ChartColor::Tone(tone) => tone.color(colors),
        }
    }
}

/// One bar of a [`Histogram`]: a label (or numeric range) and a count.
#[derive(Debug, Clone, PartialEq)]
pub struct Bucket {
    label: String,
    count: u64,
    range: Option<(f64, f64)>,
}

impl Bucket {
    /// A bucket named `label` (a category, a date, a stage name).
    pub fn new(label: impl Into<String>, count: u64) -> Self {
        Self {
            label: label.into(),
            count,
            range: None,
        }
    }

    /// A numeric bucket covering `low..high`, labelled "low–high".
    pub fn range(low: f64, high: f64, count: u64) -> Self {
        Self {
            label: format!("{}–{}", number(low), number(high)),
            count,
            range: Some((low, high)),
        }
    }

    /// The label shown on the axis and in the tooltip.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The count.
    pub fn count(&self) -> u64 {
        self.count
    }
}

/// What a [`Histogram`] reports.
#[derive(Debug)]
pub struct HistogramResponse {
    /// The chart's response (hover, focus, `clicked()`).
    pub response: Response,
    /// The bucket under the pointer.
    pub hovered: Option<usize>,
    /// The bucket the user selected this frame, by click or with the arrow keys.
    pub selected: Option<usize>,
    /// The bucket the user activated this frame: a click, or Enter/Space on the selection.
    pub clicked: Option<usize>,
}

/// A bar chart of bucket counts: the value distribution of a field (vertical bars), or
/// the top-N values of a category (`horizontal`, with labels).
///
/// It is data, not an action: the cursor stays an arrow, hovering a bar shows a tooltip
/// with its range and count. Opt into [`selectable`](Histogram::selectable) and the bars
/// become targets: hand cursor, click to select, ←/→ (↑/↓ when horizontal), Home and End to
/// move while focused, Enter or Space to activate. Selection state is the caller's.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::{Bucket, Histogram};
///
/// let buckets = [Bucket::range(0.0, 20.0, 31), Bucket::range(20.0, 40.0, 148)];
/// let mut selected = None;
/// let out = Histogram::new("price", &buckets)
///     .selectable(true)
///     .selected(selected)
///     .show(ui);
/// if let Some(index) = out.selected {
///     selected = Some(index);
/// }
/// # });
/// ```
#[must_use = "draw it with `.show(ui)`"]
pub struct Histogram<'a> {
    name: &'a str,
    buckets: &'a [Bucket],
    horizontal: bool,
    height: Option<f32>,
    color: ChartColor,
    ramp: bool,
    unit: &'a str,
    selectable: bool,
    selected: Option<usize>,
    enabled: bool,
    tooltip: bool,
    behaviours: crate::behaviour::Behaviours<'a>,
}

impl<'a> Histogram<'a> {
    /// A histogram of `name` (the field it describes, used in the accessible name).
    pub fn new(name: &'a str, buckets: &'a [Bucket]) -> Self {
        Self {
            name,
            buckets,
            horizontal: false,
            height: None,
            color: ChartColor::Series(0),
            ramp: false,
            unit: "documents",
            selectable: false,
            selected: None,
            enabled: true,
            tooltip: true,
            behaviours: Default::default(),
        }
    }

    /// Horizontal bars with the bucket label at the left and the count at the right: for
    /// top-N category frequencies.
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Height of a vertical histogram in points (default 120). Horizontal ones are one
    /// row per bucket.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The bar color (default series 0 of the chart palette).
    pub fn color(mut self, color: ChartColor) -> Self {
        self.color = color;
        self
    }

    /// Colors bars by their share of the largest count on the sequential ramp instead.
    pub fn ramp(mut self, ramp: bool) -> Self {
        self.ramp = ramp;
        self
    }

    /// What is counted, for the tooltip ("148 documents"; default "documents").
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = unit;
        self
    }

    /// Makes bars selectable: hand cursor, click, arrow keys, Enter.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// The selected bucket (caller state); drawn highlighted.
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Disabled charts are dimmed and ignore input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the hover tooltip (default true).
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.tooltip = tooltip;
        self
    }

    /// Adds a behaviour (context menu, your own tooltip with `.tooltip(false)` …).
    pub fn with(mut self, behaviour: impl crate::behaviour::Behaviour + 'a) -> Self {
        self.behaviours.push(behaviour);
        self
    }

    /// The accessible name: what a screen reader announces.
    fn summary(&self) -> String {
        let name = self.name;
        let n = self.buckets.len();
        let Some(mode) = self.mode() else {
            return format!("Histogram of {name}: no data");
        };
        let mut text = format!(
            "Histogram of {name}: {n} bucket{}",
            if n == 1 { "" } else { "s" }
        );
        let first = self.buckets.first().and_then(|b| b.range);
        let last = self.buckets.last().and_then(|b| b.range);
        if let (Some((low, _)), Some((_, high))) = (first, last) {
            text.push_str(&format!(", min {}, max {}", number(low), number(high)));
        }
        text.push_str(&format!(", mode {}", self.buckets[mode].label));
        if let Some(selected) = self.selected.and_then(|i| self.buckets.get(i)) {
            text.push_str(&format!(
                "; selected {}: {} {}",
                selected.label, selected.count, self.unit
            ));
        }
        text
    }

    /// Index of the largest bucket (the first of equals).
    fn mode(&self) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (i, bucket) in self.buckets.iter().enumerate() {
            if best.is_none_or(|b| bucket.count > self.buckets[b].count) {
                best = Some(i);
            }
        }
        best
    }

    /// Draws the chart.
    pub fn show(self, ui: &mut Ui) -> HistogramResponse {
        let colors = Theme::colors(ui.ctx());
        let n = self.buckets.len();
        let width = ui.available_width();
        let row = size::ROW_HEIGHT;
        let height = match (self.horizontal, self.height) {
            (true, _) => (n.max(1) as f32 * row).max(row),
            (false, h) => h.unwrap_or(size::CHART_HEIGHT),
        };
        let interactive = self.selectable && self.enabled && n > 0;
        let sense = if interactive {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), sense);
        let summary_text = || self.summary();
        response.widget_info(|| {
            let label = summary_text();
            if self.selectable {
                WidgetInfo::selected(
                    WidgetType::SelectableLabel,
                    self.enabled,
                    self.selected.is_some(),
                    label,
                )
            } else {
                WidgetInfo::labeled(WidgetType::Image, self.enabled, label)
            }
        });

        // Which bucket is under the pointer: slots span the whole chart.
        let slot_at = |pos: Pos2| -> Option<usize> {
            if n == 0 || !rect.contains(pos) {
                return None;
            }
            let t = if self.horizontal {
                (pos.y - rect.top()) / height
            } else {
                (pos.x - rect.left()) / width
            };
            Some(((t * n as f32) as usize).min(n - 1))
        };
        let hovered = if self.enabled {
            response.hover_pos().and_then(slot_at)
        } else {
            None
        };

        // Selection and activation.
        let mut selected_now = None;
        let mut clicked = None;
        if interactive {
            if response.clicked() {
                // Clicking takes the keyboard focus, so the arrow keys continue from here.
                response.request_focus();
                let at = response.interact_pointer_pos().and_then(slot_at);
                if let Some(index) = at {
                    selected_now = Some(index);
                    clicked = Some(index);
                } else if response.has_focus() {
                    // Enter or Space.
                    clicked = Some(self.selected.unwrap_or(0).min(n - 1));
                    selected_now = clicked.filter(|i| Some(*i) != self.selected);
                }
            }
            if response.has_focus() {
                let id = response.id;
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        id,
                        egui::EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: self.horizontal,
                            ..Default::default()
                        },
                    );
                });
                let horizontal = self.horizontal;
                let next = ui.input(|input| {
                    let current = self.selected.map(|i| i.min(n - 1));
                    let (back, forward) = (
                        input.key_pressed(Key::ArrowLeft)
                            || (horizontal && input.key_pressed(Key::ArrowUp)),
                        input.key_pressed(Key::ArrowRight)
                            || (horizontal && input.key_pressed(Key::ArrowDown)),
                    );
                    if input.key_pressed(Key::Home) {
                        Some(0)
                    } else if input.key_pressed(Key::End) {
                        Some(n - 1)
                    } else if forward {
                        Some(current.map_or(0, |c| (c + 1).min(n - 1)))
                    } else if back {
                        Some(current.map_or(0, |c| c.saturating_sub(1)))
                    } else {
                        None
                    }
                });
                if let Some(next) = next {
                    selected_now = Some(next);
                }
            }
        }

        // Cursor: arrow for data, the action cursor only over selectable bars.
        if interactive {
            if hovered.is_some() {
                crate::cursor::on_hover_action(&response);
            } else {
                crate::cursor::on_hover(&response, egui::CursorIcon::Default);
            }
        } else {
            crate::cursor::on_hover(&response, egui::CursorIcon::Default);
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if n == 0 {
                let galley = common::galley(ui, "No data", typography::body(typography::BODY));
                let pos = rect.center() - galley.size() / 2.0;
                common::text(painter, pos, galley, colors.text_placeholder);
            } else if self.horizontal {
                self.paint_horizontal(ui, rect, hovered, &colors);
            } else {
                self.paint_vertical(ui, rect, hovered, &colors);
            }
            if response.has_focus() {
                focus_ring(painter, rect, radius::CONTROL, &colors);
            }
        }

        if self.tooltip
            && let Some(index) = hovered
        {
            let bucket = &self.buckets[index];
            let total: u64 = self.buckets.iter().map(|b| b.count).sum();
            let share = if total > 0 {
                format!(" · {:.1}%", bucket.count as f64 * 100.0 / total as f64)
            } else {
                String::new()
            };
            Tooltip::new(bucket.label.as_str())
                .hint(format!("{} {}{share}", bucket.count, self.unit))
                .show_for(&response);
        }

        if selected_now.is_some() || clicked.is_some() {
            response.mark_changed();
        }
        let response = self.behaviours.attach(ui, response);
        HistogramResponse {
            response,
            hovered,
            selected: selected_now,
            clicked,
        }
    }

    fn bar_color(&self, count: u64, max: u64, colors: &Colors) -> egui::Color32 {
        let color = if self.ramp {
            // Skip the faintest stops: a small bar must still be visible on the island.
            let share = count as f32 / max.max(1) as f32;
            colors.ramp(0.25 + 0.75 * share)
        } else {
            self.color.resolve(colors)
        };
        if self.enabled {
            color
        } else {
            color.gamma_multiply(0.4)
        }
    }

    fn paint_vertical(&self, ui: &Ui, rect: Rect, hovered: Option<usize>, colors: &Colors) {
        let painter = ui.painter();
        let n = self.buckets.len();
        let max = self.buckets.iter().map(|b| b.count).max().unwrap_or(0);
        let font = typography::body(typography::SMALL);
        let axis_height = typography::SMALL + 6.0;
        let plot = Rect::from_min_max(
            rect.min + vec2(0.0, 2.0),
            pos2(rect.right(), rect.bottom() - axis_height),
        );
        let slot = rect.width() / n as f32;
        let gap = (slot * 0.15).clamp(1.0, 4.0);
        let corner = CornerRadius {
            nw: 2,
            ne: 2,
            sw: 0,
            se: 0,
        };
        for (i, bucket) in self.buckets.iter().enumerate() {
            let x = rect.left() + i as f32 * slot;
            let column = Rect::from_min_max(pos2(x, rect.top()), pos2(x + slot, rect.bottom()));
            let is_selected = self.selected == Some(i);
            if is_selected {
                painter.rect_filled(column, CornerRadius::same(radius::ROW), colors.selection);
            } else if hovered == Some(i) {
                painter.rect_filled(column, CornerRadius::same(radius::ROW), colors.row_hover);
            }
            if bucket.count > 0 {
                let h = (plot.height() * bucket.count as f32 / max as f32).max(2.0);
                let bar = Rect::from_min_max(
                    pos2(x + gap / 2.0, plot.bottom() - h),
                    pos2(x + slot - gap / 2.0, plot.bottom()),
                );
                painter.rect_filled(bar, corner, self.bar_color(bucket.count, max, colors));
                if is_selected || hovered == Some(i) {
                    painter.rect_stroke(
                        bar,
                        corner,
                        Stroke::new(1.0_f32, colors.text_strong),
                        StrokeKind::Outside,
                    );
                }
            }
        }
        painter.hline(
            rect.x_range(),
            plot.bottom() + 0.5,
            Stroke::new(1.0_f32, colors.separator),
        );
        // Axis labels: every bar's if they fit, else the first and the last.
        let label_y = plot.bottom() + 3.0;
        let color = if self.enabled {
            colors.text_secondary
        } else {
            colors.text_disabled
        };
        let galleys: Vec<_> = self
            .buckets
            .iter()
            .map(|b| common::galley(ui, b.label.as_str(), font.clone()))
            .collect();
        let widest = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
        if widest + 4.0 <= slot {
            for (i, galley) in galleys.into_iter().enumerate() {
                let x = rect.left() + (i as f32 + 0.5) * slot - galley.size().x / 2.0;
                common::text(painter, pos2(x, label_y), galley, color);
            }
        } else {
            let mut ends = galleys.into_iter();
            if let Some(first) = ends.next() {
                let last = ends.next_back();
                common::text(painter, pos2(rect.left() + 2.0, label_y), first, color);
                if let Some(last) = last {
                    let x = rect.right() - 2.0 - last.size().x;
                    common::text(painter, pos2(x, label_y), last, color);
                }
            }
        }
    }

    fn paint_horizontal(&self, ui: &Ui, rect: Rect, hovered: Option<usize>, colors: &Colors) {
        let painter = ui.painter();
        let max = self.buckets.iter().map(|b| b.count).max().unwrap_or(0);
        let font = typography::body(typography::BODY);
        let count_galleys: Vec<_> = self
            .buckets
            .iter()
            .map(|b| common::galley(ui, b.count.to_string(), font.clone()))
            .collect();
        let count_width = count_galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
        let label_galleys: Vec<_> = self
            .buckets
            .iter()
            .map(|b| common::galley(ui, b.label.as_str(), font.clone()))
            .collect();
        let widest = label_galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
        let inset = size::CONTENT_INSET;
        let label_width = widest.min(rect.width() * 0.4);
        let bar_left = rect.left() + inset + label_width + size::SECONDARY_GAP;
        let bar_right = rect.right() - inset - count_width - size::SECONDARY_GAP;
        let bar_height = 12.0;
        let (text, muted) = if self.enabled {
            (colors.text, colors.text_secondary)
        } else {
            (colors.text_disabled, colors.text_disabled)
        };
        for (i, (bucket, (label, count))) in self
            .buckets
            .iter()
            .zip(label_galleys.into_iter().zip(count_galleys))
            .enumerate()
        {
            let row = Rect::from_min_size(
                pos2(rect.left(), rect.top() + i as f32 * size::ROW_HEIGHT),
                vec2(rect.width(), size::ROW_HEIGHT),
            );
            let is_selected = self.selected == Some(i);
            if is_selected {
                crate::parts::row_pill(painter, row, colors.selection);
            } else if hovered == Some(i) {
                crate::parts::row_pill(painter, row, colors.row_hover);
            }
            let y = row.center().y;
            // Truncate long labels to their column.
            let label = if label.size().x > label_width {
                common::galley_truncated(ui, bucket.label.as_str(), font.clone(), label_width)
            } else {
                label
            };
            common::text(
                painter,
                pos2(rect.left() + inset, y - label.size().y / 2.0),
                label,
                text,
            );
            if bar_right > bar_left && bucket.count > 0 {
                let w = ((bar_right - bar_left) * bucket.count as f32 / max as f32).max(2.0);
                let bar =
                    Rect::from_min_size(pos2(bar_left, y - bar_height / 2.0), vec2(w, bar_height));
                painter.rect_filled(
                    bar,
                    CornerRadius::same(2),
                    self.bar_color(bucket.count, max, colors),
                );
            }
            let x = rect.right() - inset - count.size().x;
            common::text(painter, pos2(x, y - count.size().y / 2.0), count, muted);
        }
    }
}

/// A tiny line chart that sits inline with text: query time, request latency, a trend.
///
/// Non-interactive; optionally a tooltip shows the value under the pointer. Non-finite
/// values are gaps. A single value draws a flat line.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Sparkline;
///
/// let latency = [12.0, 14.5, 11.0, 30.0, 18.0, 16.5];
/// ui.add(Sparkline::new("latency", &latency).last_marker(true).tooltip(true));
/// # });
/// ```
#[must_use = "add it with `ui.add(sparkline)`"]
pub struct Sparkline<'a> {
    name: &'a str,
    values: &'a [f32],
    width: f32,
    color: ChartColor,
    last_marker: bool,
    band: bool,
    scale: Option<(f32, f32)>,
    tooltip: bool,
    format: Option<fn(f32) -> String>,
}

impl<'a> Sparkline<'a> {
    /// A sparkline of `name` (used in the accessible name) over `values`.
    pub fn new(name: &'a str, values: &'a [f32]) -> Self {
        Self {
            name,
            values,
            width: size::SPARKLINE_WIDTH,
            color: ChartColor::Series(0),
            last_marker: false,
            band: false,
            scale: None,
            tooltip: false,
            format: None,
        }
    }

    /// Width in points (default 96). The height is fixed.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The line color (default series 0 of the chart palette).
    pub fn color(mut self, color: ChartColor) -> Self {
        self.color = color;
        self
    }

    /// A dot at the last value.
    pub fn last_marker(mut self, on: bool) -> Self {
        self.last_marker = on;
        self
    }

    /// A faint band from the lowest to the highest value, with dots at both extremes.
    /// Most useful together with [`scale`](Self::scale), which leaves room around it.
    pub fn min_max_band(mut self, on: bool) -> Self {
        self.band = on;
        self
    }

    /// Fixed vertical range, so several sparklines are comparable (default: the data's).
    pub fn scale(mut self, low: f32, high: f32) -> Self {
        self.scale = Some((low, high));
        self
    }

    /// Shows a tooltip with the value under the pointer.
    pub fn tooltip(mut self, on: bool) -> Self {
        self.tooltip = on;
        self
    }

    /// How values are written in the tooltip and the accessible name (default: up to two
    /// decimals).
    pub fn format(mut self, format: fn(f32) -> String) -> Self {
        self.format = Some(format);
        self
    }

    fn write(&self, value: f32) -> String {
        self.format
            .map_or_else(|| number(f64::from(value)), |f| f(value))
    }

    fn extremes(&self) -> Option<(f32, f32)> {
        self.values
            .iter()
            .filter(|v| v.is_finite())
            .fold(None, |acc, &v| match acc {
                None => Some((v, v)),
                Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
            })
    }

    fn summary(&self) -> String {
        let name = self.name;
        let Some((low, high)) = self.extremes() else {
            return format!("Sparkline of {name}: no data");
        };
        let last = self.values.iter().rev().find(|v| v.is_finite()).copied();
        let n = self.values.len();
        format!(
            "Sparkline of {name}: {n} point{}, min {}, max {}, last {}",
            if n == 1 { "" } else { "s" },
            self.write(low),
            self.write(high),
            self.write(last.unwrap_or(low)),
        )
    }
}

impl Widget for Sparkline<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (rect, response) =
            ui.allocate_exact_size(vec2(self.width, size::SPARKLINE_HEIGHT), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, self.summary()));
        crate::cursor::on_hover(&response, egui::CursorIcon::Default);
        let n = self.values.len();
        let extremes = self.extremes();
        let pad = 3.0;
        let plot = rect.shrink2(vec2(pad, pad));
        let (low, high) = self.scale.or(extremes).unwrap_or((0.0, 1.0));
        let span = high - low;
        // Maps point `i` to the plot; flat data and single points sit on the middle line.
        let at = |i: usize, v: f32| -> Pos2 {
            let x = if n > 1 {
                plot.left() + plot.width() * i as f32 / (n - 1) as f32
            } else {
                plot.center().x
            };
            let t = if span > f32::EPSILON {
                ((v - low) / span).clamp(0.0, 1.0)
            } else {
                0.5
            };
            pos2(x, plot.bottom() - t * plot.height())
        };
        let color = self.color.resolve(&colors);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if let Some((min, max)) = extremes {
                if self.band {
                    let (a, b) = (at(0, max).y, at(0, min).y);
                    painter.rect_filled(
                        Rect::from_min_max(
                            pos2(plot.left(), a),
                            pos2(plot.right(), b.max(a + 1.0)),
                        ),
                        CornerRadius::same(2),
                        color.gamma_multiply(0.12),
                    );
                }
                let stroke = Stroke::new(1.5_f32, color);
                let mut previous: Option<Pos2> = None;
                for (i, &v) in self.values.iter().enumerate() {
                    if !v.is_finite() {
                        previous = None;
                        continue;
                    }
                    let point = at(i, v);
                    if let Some(prev) = previous {
                        painter.line_segment([prev, point], stroke);
                    }
                    previous = Some(point);
                }
                if n == 1 || self.values.iter().filter(|v| v.is_finite()).count() == 1 {
                    // A lone point has no segment: draw it as a short flat line.
                    if let Some(i) = self.values.iter().position(|v| v.is_finite()) {
                        let p = at(i, self.values[i]);
                        painter.line_segment(
                            [pos2(plot.left(), p.y), pos2(plot.right(), p.y)],
                            stroke,
                        );
                    }
                }
                let dot = |p: Pos2, fill| {
                    painter.circle_filled(p, 3.5, colors.island);
                    painter.circle_filled(p, 2.5, fill);
                };
                if self.band {
                    for (v, extreme) in [(min, min), (max, max)] {
                        if let Some(i) = self.values.iter().position(|x| *x == extreme) {
                            dot(at(i, v), colors.text_secondary);
                        }
                    }
                }
                if self.last_marker
                    && let Some(i) = self.values.iter().rposition(|v| v.is_finite())
                {
                    dot(at(i, self.values[i]), color);
                }
            } else {
                // No data: a dashed-looking baseline in the muted color.
                painter.hline(
                    plot.x_range(),
                    plot.center().y,
                    Stroke::new(1.0_f32, colors.text_disabled),
                );
            }
        }
        if self.tooltip
            && let Some(pos) = response.hover_pos()
            && n > 0
            && extremes.is_some()
        {
            let t = ((pos.x - plot.left()) / plot.width().max(1.0)).clamp(0.0, 1.0);
            let index = (t * (n - 1) as f32).round() as usize;
            if let Some(&value) = self.values.get(index).filter(|v| v.is_finite()) {
                let p = at(index, value);
                ui.painter().circle_filled(p, 3.5, colors.island);
                ui.painter().circle_filled(p, 2.5, color);
                Tooltip::new(self.write(value))
                    .hint(format!("Point {} of {n}", index + 1))
                    .show_for(&response);
            }
        }
        response
    }
}
