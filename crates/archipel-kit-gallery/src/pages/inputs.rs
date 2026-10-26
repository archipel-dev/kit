use super::*;

pub(super) struct State {
    name: String,
    query: String,
    port: String,
    password: String,
    body: String,
    params: Vec<KeyValueEntry>,
    method: &'static str,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: String::new(),
            query: String::new(),
            port: "8080".into(),
            password: "hunter2".into(),
            body: "{\n  \"name\": \"archipel\"\n}".into(),
            params: vec![
                KeyValueEntry::new("page", "1"),
                KeyValueEntry::new("limit", "50"),
                KeyValueEntry {
                    enabled: false,
                    ..KeyValueEntry::new("debug", "true")
                },
            ],
            method: "GET",
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(
            JBTextField::new(&mut self.name)
                .label("Project name")
                .placeholder("my-app")
                .enabled(enabled),
        );
        ui.add(
            JBTextField::search(&mut self.query)
                .id(egui::Id::new("gallery-search"))
                .enabled(enabled),
        );
        let error = self
            .port
            .parse::<u16>()
            .is_err()
            .then_some("Enter a number from 0 to 65535");
        ui.add(
            JBTextField::new(&mut self.port)
                .label("Port")
                .error(error)
                .enabled(enabled),
        );
        ui.add(
            JBTextField::new(&mut self.password)
                .label("Password")
                .password(true)
                .enabled(enabled),
        );
        ui.add(
            JBTextField::new(&mut self.body)
                .label("Request body")
                .multiline(4)
                .monospace(true)
                .enabled(enabled),
        );
        ui.add(TitledSeparator::new("Key/value table"));
        // Disabling the page makes the table read-only.
        KeyValueTable::new("params", &mut self.params)
            .key_placeholder("Add parameter")
            .value_placeholder("value")
            .read_only(!enabled)
            .show(ui);
        ui.add(TitledSeparator::new("Dropdown"));
        ui.add(
            ComboBox::new(
                "method",
                &mut self.method,
                ["GET", "POST", "PUT", "PATCH", "DELETE"],
            )
            .label("HTTP method")
            .leading(|_| Slot::icon(Glyph::Send))
            .enabled(enabled),
        );
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
