//! Archipel Kit gallery: every component, in every state, live.
//!
//! ```bash
//! cargo run -p archipel-kit-gallery
//! ```

mod pages;
use archipel_kit::{EditorScheme, Theme, egui, tokens};
use archipel_kit::{
    openapi::ui::ComboBox,
    ui::{Avatar, ColoredListCellRenderer, OnOffButton, SeparatorComponent, Tag},
    wm::{IdeFrameDecorator, MainToolbar, SimpleToolWindowPanel},
};
use pages::{Page, Pages};

/// The view modes of the picker, by name.
const VIEW_MODES: [(&str, archipel_kit::ViewMode); 4] = [
    ("Normal", archipel_kit::ViewMode::NORMAL),
    ("Presentation", archipel_kit::ViewMode::presentation()),
    (
        "Distraction free",
        archipel_kit::ViewMode::distraction_free(),
    ),
    ("Zen", archipel_kit::ViewMode::zen()),
];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: IdeFrameDecorator::Integrated.viewport(
            egui::ViewportBuilder::default()
                .with_title("Archipel Kit Gallery")
                .with_inner_size([1100.0, 760.0])
                .with_min_inner_size([800.0, 500.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "Archipel Kit Gallery",
        options,
        Box::new(|cc| {
            archipel_kit::install(&cc.egui_ctx, Theme::dark());
            Ok(Box::new(Gallery::default()))
        }),
    )
}

/// First entry of the editor scheme picker: the scheme the UI theme selects itself.
const THEME_SCHEME: &str = "Theme's own scheme";

struct Gallery {
    page: Page,
    /// Name of the picked UI theme (`Theme::all()`).
    theme: String,
    /// Name of the picked editor scheme, or [`THEME_SCHEME`].
    scheme: String,
    enabled: bool,
    pages: Pages,
    chrome: IdeFrameDecorator,
    /// Name of the picked UI look (`UiLook::name`).
    look: String,
    /// Compact density.
    compact: bool,
    /// Name of the picked view mode (`VIEW_MODES`).
    view: String,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            page: Page::Buttons,
            theme: Theme::dark().name,
            scheme: THEME_SCHEME.into(),
            enabled: true,
            pages: Pages::default(),
            chrome: IdeFrameDecorator::Integrated,
            look: archipel_kit::UiLook::Islands.name().into(),
            compact: false,
            view: VIEW_MODES[0].0.into(),
        }
    }
}

impl eframe::App for Gallery {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.chrome
            .align_native_controls("Archipel Kit Gallery", ctx);
        self.show(ctx);
    }
}

impl Gallery {
    /// The theme of the pickers: the UI theme, with the picked editor scheme if any.
    fn picked(&self) -> Theme {
        let mut theme = Theme::all()
            .iter()
            .find(|info| info.name == self.theme)
            .map_or_else(Theme::dark, |info| (info.constructor)());
        if let Some(scheme) = EditorScheme::all()
            .iter()
            .find(|scheme| scheme.name() == self.scheme)
        {
            theme = theme.with_editor_scheme(*scheme);
        }
        theme
    }

    /// Applies the pickers' theme, keeping the app's language icon style.
    fn pick(&self, ctx: &egui::Context) {
        let mut theme = self.picked();
        theme.language_icons = Theme::current(ctx).language_icons;
        theme.apply(ctx);
    }

    /// The look settings of the look pickers.
    fn picked_look(&self) -> archipel_kit::LookSettings {
        let look = archipel_kit::UiLook::ALL
            .into_iter()
            .find(|look| look.name() == self.look)
            .unwrap_or_default();
        let view = VIEW_MODES
            .iter()
            .find(|(name, _)| *name == self.view)
            .map_or(archipel_kit::ViewMode::NORMAL, |(_, mode)| *mode);
        archipel_kit::LookSettings::new(look)
            .compact(self.compact)
            .view(view)
    }

    fn show(&mut self, ctx: &egui::Context) {
        let colors = Theme::colors(ctx);
        // Gap between islands (none in the flush looks), in whole points for `egui::Margin`.
        let gap = tokens::metrics(ctx).island_gap as i8;
        MainToolbar::new("toolbar")
            .chrome(self.chrome)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(Avatar::new("Archipel Kit"));
                    ui.label(
                        egui::RichText::new("Archipel Kit")
                            .font(tokens::typography::semibold(ui.ctx(), 14.0))
                            .color(colors.text_strong),
                    );
                    // Not in tests: the snapshots would change with every release.
                    if !cfg!(test) {
                        ui.add(Tag::new(concat!("v", env!("CARGO_PKG_VERSION"))));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(OnOffButton::new(&mut self.enabled, "Enabled"));
                        ui.add(SeparatorComponent);
                        // The UI look, its density and the view mode, next to the themes.
                        let views = VIEW_MODES.iter().map(|(name, _)| (*name).to_owned());
                        let view = ui.add(
                            ComboBox::new("view-mode", &mut self.view, views)
                                .label("View mode")
                                .width(130.0),
                        );
                        let compact = ui.add(OnOffButton::new(&mut self.compact, "Compact"));
                        let looks = archipel_kit::UiLook::ALL
                            .iter()
                            .map(|look| look.name().to_owned());
                        let look = ui.add(
                            ComboBox::new("ui-look", &mut self.look, looks)
                                .label("UI look")
                                .width(100.0),
                        );
                        if look.changed() || compact.changed() || view.changed() {
                            archipel_kit::look::set_settings(ui.ctx(), self.picked_look());
                        }
                        ui.add(SeparatorComponent);
                        let schemes = std::iter::once(THEME_SCHEME.to_owned())
                            .chain(EditorScheme::all().iter().map(|s| s.name().to_owned()));
                        let scheme = ui.add(
                            ComboBox::new("editor-scheme", &mut self.scheme, schemes)
                                .label("Editor scheme")
                                .separators([1])
                                .width(140.0),
                        );
                        let themes = Theme::all().iter().map(|info| info.name.to_owned());
                        let theme = ui.add(
                            ComboBox::new("ui-theme", &mut self.theme, themes)
                                .label("UI theme")
                                .separators([2])
                                .width(140.0),
                        );
                        if theme.changed() || scheme.changed() {
                            if theme.changed() {
                                self.scheme = THEME_SCHEME.into();
                            }
                            self.pick(ui.ctx());
                        }
                    });
                });
            });

        egui::SidePanel::left("pages")
            .show_separator_line(false)
            .exact_width(220.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin {
                        left: gap,
                        right: gap / 2,
                        top: 0,
                        bottom: gap,
                    }),
            )
            .show(ctx, |ui| {
                SimpleToolWindowPanel::new("Components").show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    for (page, label, icon) in Page::ALL {
                        let row = ColoredListCellRenderer::new(label)
                            .icon(icon)
                            .selected(self.page == page)
                            .show(ui);
                        if row.response.clicked() {
                            self.page = page;
                        }
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin {
                        left: gap / 2,
                        right: gap,
                        top: 0,
                        bottom: gap,
                    }),
            )
            .show(ctx, |ui| {
                let title = Page::ALL
                    .iter()
                    .find(|(p, ..)| *p == self.page)
                    .map_or("", |(_, l, _)| l);
                SimpleToolWindowPanel::new(title).show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    archipel_kit::parts::scroll_vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Frame::new()
                                .inner_margin(egui::Margin::symmetric(
                                    tokens::size::CONTENT_INSET as i8,
                                    4,
                                ))
                                .show(ui, |ui| {
                                    ui.set_max_width(640.0);
                                    ui.add_enabled_ui(true, |ui| {
                                        self.pages.show(self.page, ui, self.enabled)
                                    });
                                });
                        });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_kit::testing::{Harness, Queryable, SnapshotOptions, SnapshotResults};

    /// Renders every page in both themes: the gallery must not panic, and the images show
    /// up in review when a component changes. The Buttons page is the README screenshot.
    #[test]
    fn every_page_renders() {
        let mut results = SnapshotResults::new();
        for dark in [true, false] {
            for (page, label, _) in Page::ALL {
                let mut gallery = Gallery {
                    page,
                    theme: if dark { Theme::dark() } else { Theme::light() }.name,
                    chrome: IdeFrameDecorator::Native,
                    ..Gallery::default()
                };
                let mut installed = false;
                let mut harness = Harness::builder()
                    .with_size(egui::vec2(1000.0, 640.0))
                    .with_pixels_per_point(2.0)
                    .with_max_steps(30)
                    .wgpu()
                    .build(move |ctx| {
                        if !installed {
                            installed = true;
                            archipel_kit::install(
                                ctx,
                                if dark { Theme::dark() } else { Theme::light() },
                            );
                            ctx.request_repaint();
                            return;
                        }
                        gallery.show(ctx);
                    });
                harness.run_ok();
                if page == Page::Buttons {
                    let theme = if dark { "dark" } else { "light" };
                    results.add(harness.try_snapshot_options(
                        format!("gallery.{theme}"),
                        &SnapshotOptions::new().output_path("../../docs/img"),
                    ));
                }
                assert!(
                    harness.query_by_label(label).is_some(),
                    "page {label} lists itself in the sidebar"
                );
            }
        }
        results.unwrap();
    }

    /// The look pickers resolve to look settings: look, density and view mode.
    #[test]
    fn look_pickers_pick_the_look_density_and_view_mode() {
        let mut gallery = Gallery::default();
        assert_eq!(gallery.picked_look(), archipel_kit::LookSettings::default());
        gallery.look = "Classic".into();
        gallery.compact = true;
        gallery.view = "Zen".into();
        let picked = gallery.picked_look();
        assert_eq!(picked.look, archipel_kit::UiLook::Classic);
        assert!(
            picked.compact && !picked.is_compact(),
            "Classic ignores compact"
        );
        assert!(picked.view.is_zen());
    }

    /// The two pickers combine any UI theme with any editor scheme.
    #[test]
    fn pickers_combine_a_theme_with_a_scheme() {
        let mut gallery = Gallery::default();
        assert_eq!(
            gallery.picked(),
            Theme::dark(),
            "the default is Archipel Dark"
        );
        gallery.theme = "Darcula".into();
        assert_eq!(
            gallery.picked(),
            Theme::darcula(),
            "THEME_SCHEME keeps the theme's own"
        );
        gallery.scheme = "Monokai".into();
        let theme = gallery.picked();
        assert_eq!(theme.name, "Darcula");
        assert_eq!(theme.editor_scheme().map(|s| s.name()), Some("Monokai"));
        assert_eq!(theme.colors, Theme::darcula().colors);
    }

    /// Every page passes the kit's UX audit: named controls, big enough targets, no
    /// overlaps, everything reachable by Tab.
    #[test]
    fn every_page_passes_the_ux_audit() {
        let mut failures = Vec::new();
        for (page, label, _) in Page::ALL {
            let mut gallery = Gallery {
                page,
                ..Gallery::default()
            };
            let mut installed = false;
            let mut harness = Harness::builder()
                .with_size(egui::vec2(1000.0, 640.0))
                .with_max_steps(30)
                .build(move |ctx| {
                    if !installed {
                        installed = true;
                        archipel_kit::install(ctx, Theme::dark());
                        ctx.request_repaint();
                        return;
                    }
                    gallery.show(ctx);
                });
            harness.run_ok();
            let audit = archipel_kit::testing::audit(&mut harness);
            for finding in audit.findings {
                failures.push(format!("{label}: {finding}"));
            }
        }
        assert!(
            failures.is_empty(),
            "UX audit:\n  {}",
            failures.join("\n  ")
        );
    }
}
