//! One composite page (tool window, editor, controls) in every bundled theme: the images of
//! `docs/theming.md`. They are also the visual check that no theme paints a missing color
//! (magenta, transparent or unreadable text).

mod common;

use archipel_kit::editor::model::Editor;
use archipel_kit::editor::{EditorComponent, JavaScript};
use archipel_kit::testing::{self, Harness, SnapshotOptions, SnapshotResults};
use archipel_kit::ui::{
    Button, ColoredListCellRenderer, Counter, InlineBanner, JBCheckBox, JBTextField, OnOffButton,
    ProgressBar, SeparatorComponent, Status, Tag,
};
use archipel_kit::wm::SimpleToolWindowPanel;
use archipel_kit::{Glyph, Theme, egui, tokens};

const CODE: &str = r#"// Load the open requests
const limit = 25;

async function load(name) {
  const res = await fetch(`/api/${name}?limit=${limit}`);
  if (!res.ok) throw new Error("HTTP " + res.status);
  return res.json();
}"#;

/// The page: a Project tool window and an editor island on the window frame, laid out like
/// the gallery.
fn page(ctx: &egui::Context) {
    let colors = Theme::colors(ctx);
    let gap = tokens::size::ISLAND_GAP as i8;
    egui::SidePanel::left("project")
        .show_separator_line(false)
        .exact_width(200.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(colors.frame)
                .inner_margin(egui::Margin {
                    left: gap,
                    right: gap / 2,
                    top: gap,
                    bottom: gap,
                }),
        )
        .show(ctx, |ui| {
            SimpleToolWindowPanel::new("Project").show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                let rows = [
                    ("archipel", Glyph::Project, true, false),
                    ("crates", Glyph::Folder, true, false),
                    ("theme.rs", Glyph::File, false, true),
                    ("lib.rs", Glyph::File, false, false),
                    ("Cargo.toml", Glyph::File, false, false),
                ];
                for (name, icon, expanded, selected) in rows {
                    let mut row = ColoredListCellRenderer::new(name)
                        .icon(icon)
                        .selected(selected);
                    if expanded {
                        row = row.expanded(Some(true));
                    }
                    row.show(ui);
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
                    top: gap,
                    bottom: gap,
                }),
        )
        .show(ctx, |ui| {
            SimpleToolWindowPanel::new("theme.rs").show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                let mut editor = Editor::new(CODE);
                EditorComponent::new(&mut editor)
                    .viewer(true)
                    .label("Editor")
                    .highlighter(&JavaScript)
                    .line_numbers(true)
                    .height(176.0)
                    .show(ui);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add(Button::primary("Run"));
                    ui.add(Button::secondary("Cancel"));
                    ui.add(Button::danger("Delete"));
                    ui.add(Tag::new("beta"));
                    ui.add(Counter::new(12).accent(true));
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let (mut checked, mut on) = (true, true);
                    ui.add(JBCheckBox::new(&mut checked, "Follow redirects"));
                    ui.add(SeparatorComponent);
                    ui.add(OnOffButton::new(&mut on, "Verbose"));
                });
                ui.add_space(4.0);
                let mut url = String::from("https://example.com/api");
                ui.add(JBTextField::new(&mut url).label("Request URL"));
                ui.add_space(4.0);
                InlineBanner::new(Status::Warning, "Project JDK is not defined").show(ui);
                ui.add_space(4.0);
                ui.add(ProgressBar::new(Some(0.6)));
            });
        });
}

/// Renders the page in `theme` and compares it with `docs/components/img/theme_<id>.png`.
fn snapshot(id: &str, theme: Theme, results: &mut SnapshotResults) {
    let mut pending = Some(theme);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 400.0))
        .with_pixels_per_point(testing::SNAPSHOT_SCALE)
        .with_max_steps(30)
        .wgpu()
        .build(move |ctx| {
            // Fonts installed during a frame take effect on the next one.
            if let Some(theme) = pending.take() {
                archipel_kit::install(ctx, theme);
                ctx.request_repaint();
                return;
            }
            page(ctx);
        });
    harness.run_ok();
    results.add(harness.try_snapshot_options(
        format!("theme_{id}"),
        &SnapshotOptions::new().output_path(common::IMG),
    ));
}

#[test]
fn every_theme_renders_the_composite_page() {
    let mut results = SnapshotResults::new();
    for info in Theme::all() {
        snapshot(info.id, (info.constructor)(), &mut results);
    }
    results.unwrap();
}

#[test]
fn schemes_render_in_the_editor() {
    // Any scheme on any theme: the editor island takes the scheme's colors.
    let mut results = SnapshotResults::new();
    let scheme = archipel_kit::EditorScheme::by_id("monokai").expect("monokai");
    snapshot(
        "islands_dark_monokai",
        Theme::islands_dark().with_editor_scheme(scheme),
        &mut results,
    );
    results.unwrap();
}
