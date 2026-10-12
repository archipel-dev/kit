use super::*;

#[derive(Default)]
pub(super) struct State {}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, _enabled: bool) {
        ui.add(SectionHeader::new("Counters & tags"));
        ui.horizontal(|ui| {
            ui.add(Counter::new(3));
            ui.add(Counter::new(12).accent(true));
            ui.add(Counter::new(1234));
            ui.add(Separator);
            ui.add(Tag::new("beta"));
            ui.add(Tag::new("GET").tone(Tone::Success));
            ui.add(Tag::new("DELETE").tone(Tone::Error));
        });
        ui.add(SectionHeader::new("Keyboard shortcuts"));
        ui.horizontal(|ui| {
            ui.add(Kbd::parse("Ctrl+Shift+F"));
            ui.add(Kbd::new(["⌘", "K"]));
        });
        ui.add(SectionHeader::new("Avatars"));
        ui.horizontal(|ui| {
            for name in ["archipel-ide", "nexus-http", "mongo-raabit"] {
                ui.add(Avatar::new(name).size(24.0));
            }
            ui.add(Avatar::new("Ada Lovelace").size(24.0).round(true));
        });
        data_colors(ui);
        ui.add(SectionHeader::new("Property grid"));
        PropertyGrid::new("props")
            .row("Status", "200 OK")
            .row("Time", "142 ms")
            .mono_row("Content-Type", "application/json")
            .show(ui);
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}

/// Typed-value, chart, ramp and method/status colors.
fn data_colors(ui: &mut egui::Ui) {
    ui.add(SectionHeader::new("Data colors"));
    let theme = Theme::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        for kind in ValueKind::ALL {
            ui.label(
                egui::RichText::new(kind.label())
                    .monospace()
                    .color(kind.color(&theme)),
            );
        }
    });
    ui.horizontal(|ui| {
        for index in 0..8 {
            swatch(ui, theme.chart(index));
        }
    });
    ui.horizontal(|ui| {
        for step in 0..=10 {
            swatch(ui, theme.colors.ramp(step as f32 / 10.0));
        }
    });
    ui.horizontal(|ui| {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            ui.add(Tag::new(method).tone(Tone::method(method)));
        }
        for code in [200, 301, 404, 503] {
            ui.add(Tag::new(code.to_string()).tone(Tone::status(code)));
        }
    });
}

fn swatch(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, tokens::radius::CONTROL, color);
}
