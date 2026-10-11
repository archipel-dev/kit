//! Progress indicators.

use crate::theme::Theme;
use crate::tokens::radius;
use egui::{
    CornerRadius, Rect, Response, Sense, Stroke, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};
use std::f32::consts::TAU;

/// A spinning arc for work of unknown length (indexing, connecting, loading).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::Spinner;
///
/// ui.add(Spinner::new().label("Indexing"));
/// # });
/// ```
#[must_use = "add it with `ui.add(spinner)`"]
pub struct Spinner {
    size: f32,
    label: String,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// A 16 pt spinner.
    pub fn new() -> Self {
        Self {
            size: 16.0,
            label: "Loading".into(),
        }
    }

    /// Edge length in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Accessible name (default "Loading").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::ProgressIndicator, true, &self.label));
        if ui.is_rect_visible(rect) {
            ui.ctx().request_repaint();
            let time = ui.input(|input| input.time) as f32;
            let radius = self.size / 2.0 - 1.5;
            let stroke = Stroke::new((self.size / 8.0).max(1.5), colors.text_secondary);
            ui.painter().circle_stroke(
                rect.center(),
                radius,
                Stroke::new(stroke.width, colors.separator),
            );
            let start = time * TAU * 0.8;
            let points: Vec<_> = (0..=24)
                .map(|i| {
                    let angle = start + i as f32 / 24.0 * TAU * 0.28;
                    rect.center() + radius * vec2(angle.cos(), angle.sin())
                })
                .collect();
            ui.painter().line(points, stroke);
        }
        response
    }
}

/// A thin horizontal bar: determinate (`fraction`) or indeterminate (a sliding segment).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::ProgressBar;
///
/// ui.add(ProgressBar::new(Some(0.4)).label("Downloading JDK"));
/// ui.add(ProgressBar::new(None).label("Resolving dependencies"));
/// # });
/// ```
#[must_use = "add it with `ui.add(bar)`"]
pub struct ProgressBar {
    fraction: Option<f32>,
    label: String,
    width: Option<f32>,
}

impl ProgressBar {
    /// `Some(0.0..=1.0)` for known progress, `None` for indeterminate.
    pub fn new(fraction: Option<f32>) -> Self {
        Self {
            fraction: fraction.map(|f| f.clamp(0.0, 1.0)),
            label: "Progress".into(),
            width: None,
        }
    }

    /// Accessible name (default "Progress").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Width in points (default: the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Widget for ProgressBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (outer, response) = ui.allocate_exact_size(vec2(width, 8.0), Sense::hover());
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, &self.label);
            info.value = self.fraction.map(|f| (f * 100.0).round() as f64);
            info
        });
        let track = Rect::from_center_size(outer.center(), vec2(width, 4.0));
        let corner = CornerRadius::same(radius::PILL);
        ui.painter().rect_filled(track, corner, colors.separator);
        let bar = match self.fraction {
            Some(fraction) => Rect::from_min_size(track.min, vec2(width * fraction, 4.0)),
            None => {
                ui.ctx().request_repaint();
                let t = (ui.input(|input| input.time) as f32 * 0.6).fract();
                let segment = width * 0.3;
                let left = track.left() - segment + t * (width + segment);
                Rect::from_min_max(
                    pos2(left.max(track.left()), track.top()),
                    pos2((left + segment).min(track.right()), track.bottom()),
                )
            }
        };
        if bar.width() > 0.0 {
            ui.painter().rect_filled(bar, corner, colors.accent);
        }
        response
    }
}
