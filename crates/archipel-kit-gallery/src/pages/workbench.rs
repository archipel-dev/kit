use super::*;
use archipel_kit::wm::{ToolWindowAnchor, ToolWindowPane, ToolWindowPaneState};

#[derive(Default)]
pub(super) struct State {
    layout: ToolWindowPaneState,
    /// The last events the tool windows reported (Alt+1 and Shift+Esc work too).
    events: Vec<String>,
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
        "Hover an island to reveal its actions. Drag the gaps, or Tab to one and use arrows / Enter. \
         Click, double-click or middle-click a header; Alt+1/2/3, Shift+Esc, Ctrl+Shift+F12; \
         drag a stripe button.",
    );
    if let Some(last) = state.events.last() {
        ui.label(format!("Last event: {last}"));
    }
    ui.allocate_ui(egui::vec2(ui.available_width(), 400.0), |ui| {
        ToolWindowPane::new("preview", &mut state.layout)
            .left("Browser", |ui| {
                ColoredListCellRenderer::new("Examples")
                    .icon(Glyph::Folder)
                    .show(ui);
            })
            .icon(ToolWindowAnchor::Left, Glyph::Folder)
            .activate_with(ToolWindowAnchor::Left, 1)
            .right("Details", |ui| {
                ui.label("Format: JSON");
            })
            .padded(ToolWindowAnchor::Right, true)
            .icon(ToolWindowAnchor::Right, Glyph::Structure)
            .activate_with(ToolWindowAnchor::Right, 2)
            .bottom("Messages", |ui| {
                ui.label("Ready");
            })
            .padded(ToolWindowAnchor::Bottom, true)
            .icon(ToolWindowAnchor::Bottom, Glyph::Terminal)
            .activate_with(ToolWindowAnchor::Bottom, 3)
            .show_inside(ui, |ui| {
                ui.add(Button::primary("Run request"));
                ui.label("Your application content");
            });
    });
    let events = state.layout.take_events();
    state
        .events
        .extend(events.into_iter().map(|event| format!("{event:?}")));
}
