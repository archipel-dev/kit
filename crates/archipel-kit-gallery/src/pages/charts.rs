use super::*;

pub(super) struct State {
    bucket: Option<usize>,
    city: Option<usize>,
    last_action: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            bucket: None,
            city: Some(0),
            last_action: String::new(),
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, _enabled: bool) {
        let price = [
            Bucket::range(0.0, 20.0, 31),
            Bucket::range(20.0, 40.0, 148),
            Bucket::range(40.0, 60.0, 96),
            Bucket::range(60.0, 80.0, 44),
            Bucket::range(80.0, 100.0, 12),
            Bucket::range(100.0, 980.0, 3),
        ];
        let cities = [
            Bucket::new("Berlin", 482),
            Bucket::new("Hamburg", 301),
            Bucket::new("Munich", 255),
            Bucket::new("Köln", 120),
            Bucket::new("Leipzig", 40),
        ];
        let latency = [
            12.0, 14.5, 11.0, 13.0, 30.0, 18.0, 16.5, 15.0, 14.0, 17.0, 12.5, 13.5,
        ];
        ui.add(SectionHeader::new("Histogram: value distribution"));
        Histogram::new("price", &price).show(ui);
        ui.add(SectionHeader::new("Selectable, on the sequential ramp"));
        let out = Histogram::new("price", &price)
            .selectable(true)
            .ramp(true)
            .selected(self.bucket)
            .show(ui);
        if out.selected.is_some() {
            self.bucket = out.selected;
        }
        if let Some(index) = out.clicked {
            self.last_action = format!("Bucket {} activated", price[index].label());
        }
        ui.add(SectionHeader::new("Top values (horizontal)"));
        let out = Histogram::new("city", &cities)
            .horizontal(true)
            .selectable(true)
            .selected(self.city)
            .show(ui);
        if out.selected.is_some() {
            self.city = out.selected;
        }
        ui.add(SectionHeader::new("Disabled and empty"));
        Histogram::new("price", &price)
            .height(56.0)
            .enabled(false)
            .show(ui);
        Histogram::new("price", &[]).height(40.0).show(ui);
        ui.add(SectionHeader::new("Sparklines"));
        ui.horizontal(|ui| {
            ui.label("p95 latency");
            ui.add(Sparkline::new("p95 latency", &latency).tooltip(true));
            ui.add(
                Sparkline::new("p95 latency", &latency)
                    .last_marker(true)
                    .min_max_band(true)
                    .tooltip(true),
            );
            ui.add(
                Sparkline::new("errors", &latency)
                    .color(ChartColor::Tone(Tone::Error))
                    .last_marker(true),
            );
            ui.add(Sparkline::new("one value", &[4.0]).last_marker(true));
            ui.add(Sparkline::new("no data", &[]));
        });
        if !self.last_action.is_empty() {
            ui.label(&self.last_action);
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
