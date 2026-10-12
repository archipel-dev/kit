use super::*;

pub(super) struct State {
    clicks: usize,
    toggles: [bool; 2],
}

impl Default for State {
    fn default() -> Self {
        Self {
            clicks: 0,
            toggles: [true, false],
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(SectionHeader::new("Variants"));
        ui.horizontal(|ui| {
            for button in [
                Button::primary("Create"),
                Button::secondary("Cancel"),
                Button::ghost("Skip"),
                Button::danger("Delete"),
            ] {
                if ui.add(button.enabled(enabled)).clicked() {
                    self.clicks += 1;
                }
            }
        });
        ui.label(format!("Clicked {} times", self.clicks));
        ui.add(SectionHeader::new("With icon, small"));
        ui.horizontal(|ui| {
            ui.add(Button::secondary("Run").icon(Glyph::Run).enabled(enabled));
            ui.add(
                Button::secondary("Debug")
                    .icon(Glyph::Debug)
                    .enabled(enabled),
            );
            ui.add(Button::ghost("Small").small(true).enabled(enabled));
        });
        ui.add(SectionHeader::new("Icon buttons"));
        ui.horizontal(|ui| {
            ui.add(IconButton::new(Glyph::Refresh, "Reload").enabled(enabled))
                .tooltip_with(Tooltip::new("Reload from Disk").shortcut("Ctrl+Alt+Y"));
            ui.add(
                IconButton::new(Glyph::Run, "Run")
                    .tone(Tone::Run)
                    .enabled(enabled),
            );
            ui.add(
                IconButton::new(Glyph::Stop, "Stop")
                    .tone(Tone::Stop)
                    .enabled(enabled),
            );
            let mut pinned = self.toggles[1];
            if ui
                .add(
                    IconButton::new(Glyph::Pin, "Pin")
                        .selected(pinned)
                        .enabled(enabled),
                )
                .clicked()
            {
                pinned = !pinned;
                self.toggles[1] = pinned;
            }
        });
        ui.add(SectionHeader::new("Links"));
        ui.horizontal(|ui| {
            ui.label("Gradle JVM is not set.");
            ui.add(Link::new("Configure…"));
        });
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
