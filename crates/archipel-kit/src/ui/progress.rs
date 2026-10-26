//! Progress indicators.

use crate::theme::Theme;
use crate::tokens::{motion, radius, size};
use egui::{
    CornerRadius, Rect, Response, Sense, Stroke, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};
use std::f32::consts::TAU;

/// A spinning arc for work of unknown length (indexing, connecting, loading).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::AsyncProcessIcon;
///
/// ui.add(AsyncProcessIcon::new().label("Indexing"));
/// # });
/// ```
#[must_use = "add it with `ui.add(spinner)`"]
pub struct AsyncProcessIcon {
    size: f32,
    label: String,
}

impl Default for AsyncProcessIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncProcessIcon {
    /// The frame shown `time` seconds after the start: the spinner advances one eighth of a
    /// turn every [`motion::ASYNC_PROCESS_FRAME`] (125 ms, IntelliJ `AnimatedIcon.Default`),
    /// it does not turn smoothly.
    pub fn frame_at(time: f64) -> u32 {
        let frames = f64::from(motion::ASYNC_PROCESS_FRAMES);
        let frame = (time.max(0.0) / f64::from(motion::ASYNC_PROCESS_FRAME)).floor();
        (frame % frames) as u32
    }

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

impl Widget for AsyncProcessIcon {
    fn ui(self, ui: &mut Ui) -> Response {
        let colors = Theme::colors(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::ProgressIndicator, true, &self.label));
        if ui.is_rect_visible(rect) {
            let time = ui.input(|input| input.time);
            // Wake up when the next frame is due, not on every frame.
            let into_frame = time.rem_euclid(f64::from(motion::ASYNC_PROCESS_FRAME));
            ui.ctx().request_repaint_after_secs(
                (f64::from(motion::ASYNC_PROCESS_FRAME) - into_frame) as f32,
            );
            let radius = self.size / 2.0 - 1.5;
            let stroke = Stroke::new((self.size / 8.0).max(1.5), colors.text_secondary);
            ui.painter().circle_stroke(
                rect.center(),
                radius,
                Stroke::new(stroke.width, colors.progress_track),
            );
            let start = Self::frame_at(time) as f32 * TAU / motion::ASYNC_PROCESS_FRAMES as f32;
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

/// How a [`ProgressBar`] reads: IntelliJ's `ProgressBar.progressColor` and its passed,
/// warning and failed variants.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressState {
    /// Work in progress: the accent color.
    #[default]
    Normal,
    /// Finished well.
    Passed,
    /// Needs attention.
    Warning,
    /// Failed.
    Failed,
}

/// A thin horizontal bar: determinate (`fraction`) or indeterminate (a sliding segment).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::ProgressBar;
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
    state: ProgressState,
}

impl ProgressBar {
    /// `Some(0.0..=1.0)` for known progress, `None` for indeterminate.
    pub fn new(fraction: Option<f32>) -> Self {
        Self {
            fraction: fraction.map(|f| f.clamp(0.0, 1.0)),
            label: "Progress".into(),
            width: None,
            state: ProgressState::Normal,
        }
    }

    /// The segment of an indeterminate bar of `width` at `time` seconds, as `start..end`
    /// from the track's left edge. It slides in from the left, crosses and leaves on the
    /// right, once every [`motion::PROGRESS_INDETERMINATE_PERIOD`].
    pub fn indeterminate_segment(width: f32, time: f64) -> (f32, f32) {
        let period = f64::from(motion::PROGRESS_INDETERMINATE_PERIOD);
        let t = (time.max(0.0) / period).fract() as f32;
        let segment = width * 0.3;
        let left = -segment + t * (width + segment);
        (left.max(0.0), (left + segment).min(width))
    }

    /// Colors the fill as passed, warning or failed.
    pub fn state(mut self, state: ProgressState) -> Self {
        self.state = state;
        self
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
        let (outer, response) =
            ui.allocate_exact_size(vec2(width, size::PROGRESS_BAR_ROW), Sense::hover());
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, &self.label);
            info.value = self.fraction.map(|f| (f * 100.0).round() as f64);
            info
        });
        let track = Rect::from_center_size(outer.center(), vec2(width, size::PROGRESS_BAR_HEIGHT));
        let corner = CornerRadius::same(radius::PILL);
        ui.painter()
            .rect_filled(track, corner, colors.progress_track);
        let bar = match self.fraction {
            Some(fraction) => {
                Rect::from_min_size(track.min, vec2(width * fraction, size::PROGRESS_BAR_HEIGHT))
            }
            None => {
                ui.ctx().request_repaint();
                let (start, end) = Self::indeterminate_segment(width, ui.input(|i| i.time));
                Rect::from_min_max(
                    pos2(track.left() + start, track.top()),
                    pos2(track.left() + end, track.bottom()),
                )
            }
        };
        if bar.width() > 0.0 {
            let fill = match self.state {
                ProgressState::Normal => colors.accent,
                ProgressState::Passed => colors.success,
                ProgressState::Warning => colors.warning,
                ProgressState::Failed => colors.error,
            };
            ui.painter().rect_filled(bar, corner, fill);
        }
        response
    }
}
