//! The kit theme must remain authoritative when native system appearance differs.
use archipel_kit::{Theme, egui, install};
#[test]
fn explicit_theme_and_typography_survive_native_appearance_and_switches() {
    let ctx = egui::Context::default();
    ctx.set_theme(egui::ThemePreference::System);
    install(&ctx, Theme::dark());
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    assert_eq!(ctx.style().visuals.window_fill, Theme::dark().colors.popup);
    let fonts = ctx.style().text_styles.clone();
    let spacing = ctx.style().spacing.item_spacing;
    Theme::light().apply(&ctx);
    assert_eq!(ctx.theme(), egui::Theme::Light);
    assert_eq!(ctx.style().text_styles, fonts);
    assert_eq!(ctx.style().spacing.item_spacing, spacing);
    assert_eq!(ctx.style().visuals.window_fill, Theme::light().colors.popup);
    Theme::dark().apply(&ctx);
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    assert_eq!(ctx.style().text_styles, fonts);
}
