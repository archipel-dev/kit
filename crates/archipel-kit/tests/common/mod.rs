//! Shared by the component tests.

/// Snapshots double as the images of `docs/components/*.md`.
#[allow(dead_code)]
pub const IMG: &str = "../../docs/components/img";

/// Fills the canvas with the window frame color, like the area around islands (popups
/// float over it in snapshots).
#[allow(dead_code)]
pub fn backdrop(ctx: &egui::Context) {
    let colors = archipel_kit::Theme::colors(ctx);
    ctx.layer_painter(egui::LayerId::background()).rect_filled(
        ctx.content_rect(),
        0.0,
        colors.frame,
    );
}
