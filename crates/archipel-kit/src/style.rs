//! Fonts, spacing and egui visuals.

use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke};
use std::sync::Arc;

/// Sets up a context for Archipel Kit: bundled fonts (Inter, JetBrains Mono), spacing and
/// the visuals of `theme`. Call it once at startup, before the first frame (in eframe:
/// from the app's creation closure), since fonts take effect on the next frame. Switch
/// themes later with [`Theme::apply`].
pub fn install(ctx: &egui::Context, theme: Theme) {
    install_fonts(ctx);
    ctx.style_mut(|style| {
        use egui::{FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(typography::SMALL)),
            (TextStyle::Body, FontId::proportional(typography::BODY)),
            (TextStyle::Button, FontId::proportional(typography::BODY)),
            (
                TextStyle::Heading,
                FontId::new(
                    typography::TITLE,
                    FontFamily::Name(typography::SEMIBOLD.into()),
                ),
            ),
            (TextStyle::Monospace, FontId::monospace(typography::CODE)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        // Popups and menus: a small vertical inset, rows reach the sides (their pills inset
        // themselves), like the reference IDE's popup lists.
        style.spacing.menu_margin = egui::Margin::symmetric(0, 6);
        style.spacing.window_margin = egui::Margin::same(size::DIALOG_PADDING as i8);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        style.spacing.interact_size.y = size::ROW_HEIGHT;
        style.spacing.scroll = egui::style::ScrollStyle::floating();
        style.interaction.selectable_labels = false;
    });
    theme.apply(ctx);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(
        "inter",
        include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice(),
    );
    add(
        "inter-semibold",
        include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice(),
    );
    add(
        "jetbrains-mono",
        include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice(),
    );
    add(
        "jetbrains-mono-bold",
        include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice(),
    );
    // egui's bundled fonts stay as fallbacks for symbols such as ⌘ and ⇧.
    let proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    let family = |first: &str, rest: &[String]| {
        std::iter::once(first.to_owned())
            .chain(rest.iter().cloned())
            .collect::<Vec<_>>()
    };
    fonts
        .families
        .insert(FontFamily::Proportional, family("inter", &proportional));
    fonts
        .families
        .insert(FontFamily::Monospace, family("jetbrains-mono", &monospace));
    fonts.families.insert(
        FontFamily::Name(typography::SEMIBOLD.into()),
        family("inter-semibold", &proportional),
    );
    fonts.families.insert(
        FontFamily::Name(typography::MONO_BOLD.into()),
        family("jetbrains-mono-bold", &monospace),
    );
    ctx.set_fonts(fonts);
}

/// Rebuilds egui's visuals from `theme`, so plain egui widgets match the kit's components.
pub(crate) fn apply_visuals(ctx: &egui::Context, theme: &Theme) {
    let c = &theme.colors;
    ctx.style_mut(|style| {
        let visuals = &mut style.visuals;
        *visuals = if theme.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = Some(c.text);
        visuals.panel_fill = c.frame;
        visuals.window_fill = c.popup;
        visuals.window_stroke = Stroke::new(1.0_f32, c.popup_border);
        visuals.window_corner_radius = CornerRadius::same(radius::POPUP);
        visuals.menu_corner_radius = CornerRadius::same(radius::POPUP);
        visuals.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(if theme.dark { 90 } else { 40 }),
        };
        visuals.popup_shadow = visuals.window_shadow;
        visuals.extreme_bg_color = c.field;
        visuals.faint_bg_color = c.hover;
        visuals.code_bg_color = c.island;
        visuals.hyperlink_color = c.link;
        visuals.selection.bg_fill = c.selection;
        visuals.selection.stroke = Stroke::new(1.0_f32, c.text_strong);
        visuals.text_cursor.stroke = Stroke::new(1.5_f32, theme.editor.caret);
        visuals.error_fg_color = c.error;
        visuals.warn_fg_color = c.warning;
        visuals.resize_corner_size = 0.0;

        let radius = CornerRadius::same(radius::CONTROL);
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = c.frame;
        widgets.noninteractive.weak_bg_fill = c.frame;
        widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, c.popup_border);
        widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, c.text);
        widgets.noninteractive.corner_radius = radius;
        widgets.inactive.bg_fill = c.field;
        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        widgets.inactive.bg_stroke = Stroke::new(1.0_f32, c.border);
        widgets.inactive.fg_stroke = Stroke::new(1.0_f32, c.text);
        widgets.inactive.corner_radius = radius;
        widgets.hovered.bg_fill = c.field;
        widgets.hovered.weak_bg_fill = c.hover;
        widgets.hovered.bg_stroke = Stroke::new(1.0_f32, c.border);
        widgets.hovered.fg_stroke = Stroke::new(1.0_f32, c.text_strong);
        widgets.hovered.corner_radius = radius;
        widgets.hovered.expansion = 0.0;
        widgets.active.bg_fill = c.field;
        widgets.active.weak_bg_fill = c.pressed;
        widgets.active.bg_stroke = Stroke::new(size::FOCUS_RING, c.accent);
        widgets.active.fg_stroke = Stroke::new(1.0_f32, c.text_strong);
        widgets.active.corner_radius = radius;
        widgets.active.expansion = 0.0;
        widgets.open = widgets.active;
    });
}
