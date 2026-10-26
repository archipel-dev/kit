use super::*;

pub(super) struct State {
    banners: [bool; 4],
    progress: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            banners: [true; 4],
            progress: 0.4,
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, _enabled: bool) {
        ui.add(TitledSeparator::new("Banners"));
        let banners = [
            (Status::Info, "Indexing finished: 1,204 files", None),
            (Status::Success, "Gradle sync succeeded", None),
            (
                Status::Warning,
                "Project JDK is not defined",
                Some("Set up JDK"),
            ),
            (Status::Error, "Build failed with 3 errors", Some("Retry")),
        ];
        for (index, (severity, text, action)) in banners.into_iter().enumerate() {
            if self.banners[index] {
                let mut banner = InlineBanner::new(severity, text).dismissible(true);
                if let Some(action) = action {
                    banner = banner.action(action);
                }
                if banner.show(ui).dismissed {
                    self.banners[index] = false;
                }
            }
        }
        if self.banners.iter().any(|shown| !shown)
            && ui.add(ActionLink::new("Show all banners")).clicked()
        {
            self.banners = [true; 4];
        }
        ui.add(TitledSeparator::new("Toast"));
        Notification::new(Status::Success, "Build finished")
            .body("3 modules compiled in 4.2 s")
            .action("Open log")
            .show(ui);
        ui.add(TitledSeparator::new("Progress"));
        ui.horizontal(|ui| {
            ui.add(AsyncProcessIcon::new());
            ui.label("Indexing…");
        });
        ui.add(ProgressBar::new(Some(self.progress)));
        ui.add(egui::Slider::new(&mut self.progress, 0.0..=1.0).text("fraction"));
        ui.add(ProgressBar::new(None));
        ui.add(TitledSeparator::new("Empty state"));
        StatusText::new("No requests yet")
            .icon(Glyph::Send)
            .hint("Create a request to start testing your API.")
            .action("New request")
            .show(ui);
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
