//! Content with a loading veil: IntelliJ's `JBLoadingPanel` / `LoadingDecorator`.

use crate::theme::Theme;
use crate::tokens::{loading, typography};
use crate::ui::AsyncProcessIcon;
use egui::{Id, InnerResponse, Rect, Sense, Ui, UiBuilder, pos2, vec2};

/// What the panel remembers between frames, kept in the `egui::Context`.
#[derive(Clone, Copy, Debug, Default)]
struct PanelState {
    /// When the current loading started, in seconds of `egui` time.
    loading_since: Option<f64>,
    /// When the veil started to fade out (loading ended while it was visible).
    fading_since: Option<f64>,
}

/// Content that is being loaded: a veil over it with a spinner and a text, shown after a start
/// delay so quick loads do not flicker (IntelliJ's `JBLoadingPanel`).
///
/// The caller owns the `loading` flag. The veil appears once `loading` has been true for the
/// [start delay](Self::start_delay_ms) (default 0: at once), blocks the pointer from reaching
/// the content, and fades out over [`tokens::loading::FADE_OUT`](crate::tokens::loading::FADE_OUT)
/// when loading ends. A load that ends before the delay never shows the veil.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::JBLoadingPanel;
///
/// let loading = true;
/// JBLoadingPanel::new("results", loading)
///     .start_delay_ms(300)
///     .text("Loading results…")
///     .show(ui, |ui| {
///         ui.label("results");
///     });
/// # });
/// ```
#[must_use = "show it with `panel.show(ui, content)`"]
pub struct JBLoadingPanel {
    id: Id,
    loading: bool,
    start_delay_ms: u32,
    text: Option<String>,
}

impl JBLoadingPanel {
    /// A panel with a unique `id_salt`; `loading` says whether the content is loading.
    pub fn new(id_salt: impl std::hash::Hash, loading: bool) -> Self {
        Self {
            id: Id::new(id_salt),
            loading,
            start_delay_ms: 0,
            text: None,
        }
    }

    /// How long loading has to last before the veil shows, in milliseconds (the
    /// `startDelayMs` of `JBLoadingPanel`; default 0, no delay).
    pub fn start_delay_ms(mut self, delay: u32) -> Self {
        self.start_delay_ms = delay;
        self
    }

    /// A text under the spinner (`setLoadingText`).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Draws the content, and the veil over it while it is visible.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let inner = ui.scope(add_contents);
        let rect = inner.response.rect;
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let key = self.id.with("loading-panel");
        let mut state: PanelState = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
        let delay = f64::from(self.start_delay_ms) / 1000.0;
        let alpha = if self.loading {
            state.fading_since = None;
            let since = *state.loading_since.get_or_insert(now);
            let remaining = since + delay - now;
            if remaining <= 0.0 {
                1.0
            } else {
                // The veil is not due yet: look again when it is.
                ctx.request_repaint_after_secs(remaining as f32);
                0.0
            }
        } else {
            let was_visible = state
                .loading_since
                .take()
                .is_some_and(|since| now >= since + delay);
            if was_visible {
                state.fading_since = Some(now);
            }
            match state.fading_since {
                Some(since) => {
                    let t = ((now - since) as f32 / loading::FADE_OUT).clamp(0.0, 1.0);
                    if t >= 1.0 {
                        state.fading_since = None;
                    } else {
                        ctx.request_repaint();
                    }
                    1.0 - t
                }
                None => 0.0,
            }
        };
        ctx.data_mut(|d| d.insert_temp(key, state));

        if alpha > 0.0 && rect.is_positive() {
            let colors = Theme::colors(&ctx);
            // The veil takes the pointer: the content under it is not reachable while it shows.
            if self.loading {
                let veil = ui.interact(rect, self.id.with("veil"), Sense::click_and_drag());
                let name = self.text.clone().unwrap_or_else(|| "Loading".to_owned());
                veil.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Other, false, name.clone())
                });
            }
            ui.painter()
                .rect_filled(rect, 0.0, colors.loading_veil.gamma_multiply(alpha));
            let mut overlay = ui.new_child(UiBuilder::new().max_rect(rect));
            overlay.set_opacity(alpha);
            let spinner =
                AsyncProcessIcon::new().label(self.text.as_deref().unwrap_or("Loading").to_owned());
            let text_height = self
                .text
                .as_ref()
                .map_or(0.0, |_| typography::BODY + loading::TEXT_GAP);
            let top = rect.center().y - (spinner_size() + text_height) / 2.0;
            let spinner_rect = Rect::from_center_size(
                pos2(rect.center().x, top + spinner_size() / 2.0),
                vec2(spinner_size(), spinner_size()),
            );
            overlay.scope_builder(UiBuilder::new().max_rect(spinner_rect), |ui| {
                ui.add(spinner);
            });
            if let Some(text) = &self.text {
                let galley = crate::common::galley(
                    &overlay,
                    text.as_str(),
                    typography::body(typography::BODY),
                );
                let pos = pos2(
                    rect.center().x - galley.size().x / 2.0,
                    spinner_rect.bottom() + loading::TEXT_GAP,
                );
                crate::common::text(
                    overlay.painter(),
                    pos,
                    galley,
                    colors.text_secondary.gamma_multiply(alpha),
                );
            }
        }
        inner
    }
}

/// Edge of the spinner in the veil.
fn spinner_size() -> f32 {
    16.0
}
