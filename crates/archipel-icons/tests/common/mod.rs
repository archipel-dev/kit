//! Shared by the icon sheet tests.

#![allow(dead_code)]

use egui::Color32;
use egui_kittest::Harness;

/// (name, background, caption text, icon ink): the Archipel Kit island colors.
pub const LOOKS: [(&str, Color32, Color32, Color32); 2] = [
    (
        "dark",
        Color32::from_rgb(0x19, 0x1A, 0x1C),
        Color32::from_rgb(0x9F, 0xA2, 0xA8),
        Color32::from_rgb(0xCE, 0xD0, 0xD6),
    ),
    (
        "light",
        Color32::from_rgb(0xFF, 0xFF, 0xFF),
        Color32::from_rgb(0x5F, 0x62, 0x69),
        Color32::from_rgb(0x73, 0x76, 0x7C),
    ),
];

/// Registers the bold label font the way Archipel Kit does (family `semibold`), so the
/// sheets show the labels users see.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "inter-semibold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../archipel-kit/assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.families.insert(
        egui::FontFamily::Name("semibold".into()),
        vec!["inter-semibold".into()],
    );
    ctx.set_fonts(fonts);
}

pub fn sheet<'a>(
    size: egui::Vec2,
    scale: f32,
    background: Color32,
    mut draw: impl FnMut(&egui::Painter) + 'a,
) -> Harness<'a> {
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(scale)
        .wgpu()
        .build(move |ctx| {
            if !installed {
                installed = true;
                install_fonts(ctx);
                ctx.request_repaint();
                return;
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(background))
                .show(ctx, |ui| draw(ui.painter()));
        });
    harness.run();
    harness.run();
    harness
}
