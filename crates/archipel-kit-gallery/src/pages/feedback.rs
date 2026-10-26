use super::*;
use archipel_kit::notification::{
    NotificationAction, NotificationEventKind, NotificationGroup, NotificationType,
    NotificationsManager,
};
use archipel_kit::ui::{EditorNotificationPanel, ProgressState};

pub(super) struct State {
    banners: [bool; 4],
    progress: f32,
    editor_panel: bool,
    notifications: NotificationsManager,
    last_event: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            banners: [true; 4],
            progress: 0.4,
            editor_panel: true,
            notifications: NotificationsManager::new(),
            last_event: String::new(),
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
        InlineBanner::new(Status::Promo, "Try the new debugger tool window")
            .action("Show me")
            .gear_action("Banner settings")
            .dismissible(true)
            .show(ui);
        ui.add(TitledSeparator::new("Editor notification panel"));
        if self.editor_panel {
            let out = EditorNotificationPanel::new("Project JDK is not defined")
                .status(Status::Warning)
                .action("Setup JDK")
                .dismissible(true)
                .show(ui);
            self.editor_panel = !out.dismissed;
        } else if ui.add(ActionLink::new("Show the panel again")).clicked() {
            self.editor_panel = true;
        }
        ui.add(TitledSeparator::new("Notifications"));
        Notification::new(Status::Success, "Build finished")
            .body("3 modules compiled in 4.2 s")
            .action("Open log")
            .show(ui);
        ui.label("Balloons: 10 s, 300 s when sticky; they wait while the pointer is over them.");
        ui.horizontal(|ui| {
            if ui.add(Button::secondary("Notify")).clicked() {
                self.notifications.notify(
                    Notification::new(NotificationType::Information, "Indexing finished")
                        .body("1,204 files were indexed.")
                        .action("Show"),
                );
            }
            if ui.add(Button::secondary("Notify (sticky)")).clicked() {
                self.notifications.notify(
                    Notification::new(NotificationType::Warning, "Project JDK is not defined")
                        .group(NotificationGroup::sticky_balloon_group("Gallery"))
                        .notification_action(
                            NotificationAction::new("Remind me").keep_notification(true),
                        )
                        .action("Setup JDK"),
                );
            }
        });
        if !self.last_event.is_empty() {
            ui.label(format!("Last notification event: {}", self.last_event));
        }
        for event in self.notifications.show(ui.ctx()) {
            self.last_event = match event.kind {
                NotificationEventKind::Action(index) => format!("action {index}"),
                NotificationEventKind::Dismissed => "dismissed".into(),
                NotificationEventKind::Hidden => "hidden".into(),
                _ => "other".into(),
            };
        }
        ui.add(TitledSeparator::new("Progress"));
        ui.horizontal(|ui| {
            ui.add(AsyncProcessIcon::new());
            ui.label("Indexing…");
        });
        ui.add(ProgressBar::new(Some(self.progress)));
        ui.add(egui::Slider::new(&mut self.progress, 0.0..=1.0).text("fraction"));
        ui.add(ProgressBar::new(None));
        ui.add(ProgressBar::new(Some(1.0)).state(ProgressState::Passed));
        ui.add(ProgressBar::new(Some(0.7)).state(ProgressState::Warning));
        ui.add(ProgressBar::new(Some(0.3)).state(ProgressState::Failed));
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
