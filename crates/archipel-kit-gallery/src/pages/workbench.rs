use super::*;
use archipel_kit::components::{Workbench, WorkbenchRegion, WorkbenchState};

#[derive(Default)]
pub(super) struct State {
    layout: WorkbenchState,
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, _enabled: bool) {
    ui.horizontal(|ui| {
        for (hidden, title) in [
            (&mut state.layout.left_hidden, "Browser"),
            (&mut state.layout.right_hidden, "Details"),
            (&mut state.layout.bottom_hidden, "Messages"),
        ] {
            let mut visible = !*hidden;
            if ui.add(Toggle::new(&mut visible, title)).changed() {
                *hidden = !visible;
            }
        }
    });
    ui.label(
        "Hover an island to reveal its actions. Drag the gaps, or Tab to one and use arrows / Enter.",
    );
    ui.allocate_ui(egui::vec2(ui.available_width(), 400.0), |ui| {
        Workbench::new("preview", &mut state.layout)
            .left("Browser", |ui| {
                ListRow::new("Examples").icon(Glyph::Folder).show(ui);
            })
            .right("Details", |ui| {
                ui.label("Format: JSON");
            })
            .padded(WorkbenchRegion::Right, true)
            .bottom("Messages", |ui| {
                ui.label("Ready");
            })
            .padded(WorkbenchRegion::Bottom, true)
            .show_inside(ui, |ui| {
                ui.add(Button::primary("Run request"));
                ui.label("Your application content");
            });
    });
}
