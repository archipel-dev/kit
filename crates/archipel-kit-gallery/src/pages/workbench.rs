use super::*;
use archipel_kit::wm::{ToolWindowAnchor, ToolWindowPane, ToolWindowPaneState};

#[derive(Default)]
pub(super) struct State {
    layout: ToolWindowPaneState,
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, _enabled: bool) {
    ui.horizontal(|ui| {
        for (hidden, title) in [
            (&mut state.layout.left_hidden, "Browser"),
            (&mut state.layout.right_hidden, "Details"),
            (&mut state.layout.bottom_hidden, "Messages"),
        ] {
            let mut visible = !*hidden;
            if ui.add(OnOffButton::new(&mut visible, title)).changed() {
                *hidden = !visible;
            }
        }
    });
    ui.label(
        "Hover an island to reveal its actions. Drag the gaps, or Tab to one and use arrows / Enter.",
    );
    ui.allocate_ui(egui::vec2(ui.available_width(), 400.0), |ui| {
        ToolWindowPane::new("preview", &mut state.layout)
            .left("Browser", |ui| {
                ColoredListCellRenderer::new("Examples")
                    .icon(Glyph::Folder)
                    .show(ui);
            })
            .icon(ToolWindowAnchor::Left, Glyph::Folder)
            .right("Details", |ui| {
                ui.label("Format: JSON");
            })
            .padded(ToolWindowAnchor::Right, true)
            .icon(ToolWindowAnchor::Right, Glyph::Structure)
            .bottom("Messages", |ui| {
                ui.label("Ready");
            })
            .padded(ToolWindowAnchor::Bottom, true)
            .icon(ToolWindowAnchor::Bottom, Glyph::Terminal)
            .show_inside(ui, |ui| {
                ui.add(Button::primary("Run request"));
                ui.label("Your application content");
            });
    });
}
