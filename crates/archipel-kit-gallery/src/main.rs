//! Archipel Kit gallery: every component, in every state, live.
//!
//! ```bash
//! cargo run -p archipel-kit-gallery
//! ```

mod pages;
use archipel_kit::components::{
    Avatar, ListRow, Panel, Separator, Tag, Toggle, WindowChrome, WindowToolbar,
};
use archipel_kit::{Theme, egui, tokens};
use pages::{Page, Pages};

/// Gap between islands, in whole points for `egui::Margin`.
const GAP: i8 = tokens::size::ISLAND_GAP as i8;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: WindowChrome::Integrated.viewport(
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

struct Gallery {
    page: Page,
    dark: bool,
    enabled: bool,
    pages: Pages,
    chrome: WindowChrome,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            page: Page::Buttons,
            dark: true,
            enabled: true,
            pages: Pages::default(),
            chrome: WindowChrome::Integrated,
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
    fn show(&mut self, ctx: &egui::Context) {
        let colors = Theme::colors(ctx);
        WindowToolbar::new("toolbar")
            .chrome(self.chrome)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(Avatar::new("Archipel Kit"));
                    ui.label(
                        egui::RichText::new("Archipel Kit")
                            .font(tokens::typography::semibold(ui.ctx(), 14.0))
                            .color(colors.text_strong),
                    );
                    ui.add(Tag::new(concat!("v", env!("CARGO_PKG_VERSION"))));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut dark = self.dark;
                        if ui.add(Toggle::new(&mut dark, "Dark")).changed() {
                            self.dark = dark;
                            let mut theme = if dark { Theme::dark() } else { Theme::light() };
                            theme.language_icons = Theme::current(ui.ctx()).language_icons;
                            theme.apply(ui.ctx());
                        }
                        ui.add(Separator);
                        ui.add(Toggle::new(&mut self.enabled, "Enabled"));
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
                        left: GAP,
                        right: GAP / 2,
                        top: 0,
                        bottom: GAP,
                    }),
            )
            .show(ctx, |ui| {
                Panel::new("Components").show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    for (page, label, icon) in Page::ALL {
                        let row = ListRow::new(label)
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
                        left: GAP / 2,
                        right: GAP,
                        top: 0,
                        bottom: GAP,
                    }),
            )
            .show(ctx, |ui| {
                let title = Page::ALL
                    .iter()
                    .find(|(p, ..)| *p == self.page)
                    .map_or("", |(_, l, _)| l);
                Panel::new(title).show(ui, |ui| {
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
                    dark,
                    chrome: WindowChrome::Native,
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
