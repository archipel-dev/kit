use super::*;
use archipel_kit::components::{Workbench, WorkbenchState};

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
        "Drag splitters or Tab to one and use arrows / Enter. Headers hide, maximize and restore.",
    );
    ui.allocate_ui(egui::vec2(ui.available_width(), 400.0), |ui| {
        Workbench::new("preview", &mut state.layout)
            .left("Browser", |ui| {
                ListRow::new("Examples").icon(Glyph::Folder).show(ui);
            })
            .right("Details", |ui| {
                ui.label("Format: JSON");
            })
            .bottom("Messages", |ui| {
                ui.label("Ready");
            })
            .show_inside(ui, |ui| {
                ui.add(Button::primary("Run request"));
                ui.label("Your application content");
            });
    });
}
