use super::*;

/// The JSON of the Code page.
const CODE_SAMPLE: &str = r#"{
  "id": 7,
  "name": "archipel",
  "active": true,
  "tags": ["ui", "rust"],
  "owner": {
    "login": "henry",
    "email": null
  },
  "score": 4.5e2
}"#;

pub(super) struct State {
    last_action: String,
    code: String,
    url_bar: UrlBar,
    header: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_action: String::new(),
            code: CODE_SAMPLE.into(),
            url_bar: UrlBar {
                url: "{{base_url}}/users/{{user_id}}?key={{token}}&q={{missing}}".into(),
                ..UrlBar::default()
            },
            header: "Bearer {{token}}".into(),
        }
    }
}

impl State {
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(SectionHeader::new("Template variables"));
        let submitted = self.url_bar.show(ui, enabled);
        if submitted {
            self.last_action = format!("Sent {}", self.url_bar.url);
        }
        ui.label("Type {{ for suggestions: Up/Down move, Enter or a click completes, Esc closes.");
        let highlighter = TemplateHighlighter::new(resolve_variable);
        ui.add(
            TextField::new(&mut self.header)
                .label("Header value")
                .highlighter(&highlighter)
                .clearable(true)
                .enabled(enabled),
        );
        ui.add(SectionHeader::new("Code view"));
        CodeView::new(CODE_SAMPLE)
            .label("Response body")
            .highlighter(&Json)
            .line_numbers(true)
            .folding(true)
            .height(236.0)
            .show(ui);
        ui.add(SectionHeader::new("Code editor"));
        ui.add(
            CodeEditor::new(&mut self.code)
                .label("Request body")
                .highlighter(&Json)
                .line_numbers(true)
                .enabled(enabled)
                .height(236.0),
        );
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}

const VARIABLES: [&str; 4] = ["base_url", "user_id", "user_name", "token"];

fn resolve_variable(name: &str) -> TemplateToken {
    match name {
        "token" => TemplateToken::Secret,
        name if VARIABLES.contains(&name) => TemplateToken::Resolved,
        _ => TemplateToken::Unresolved,
    }
}

/// A URL bar with `{{variable}}` highlighting and autocomplete, as documented on the code
/// page. The state is the gallery's; the kit keeps none.
#[derive(Default)]
struct UrlBar {
    url: String,
    open: bool,
    selected: usize,
    matches: Vec<&'static str>,
}

impl UrlBar {
    /// Returns `true` when Enter was pressed with no list open.
    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) -> bool {
        let highlighter = TemplateHighlighter::new(resolve_variable);
        // The list was open last frame: it owns the navigation keys.
        let listing = self.open && !self.matches.is_empty();
        let (mut down, mut up, mut accept) = (false, false, false);
        if listing {
            ui.input_mut(|i| {
                down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
                up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                accept = i.consume_key(Modifiers::NONE, Key::Enter);
                if i.consume_key(Modifiers::NONE, Key::Escape) {
                    self.open = false;
                }
            });
        }
        let out = TextField::new(&mut self.url)
            .label("URL")
            .placeholder("https://example.com")
            .monospace(true)
            .clearable(true)
            .enabled(enabled)
            .highlighter(&highlighter)
            .show(ui);
        self.matches = match &out.template {
            Some(token) => VARIABLES
                .into_iter()
                .filter(|v| v.starts_with(token.partial.as_str()))
                .collect(),
            None => Vec::new(),
        };
        if out.response.changed() {
            self.open = true;
            self.selected = 0;
        }
        let clicking = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
        if self.matches.is_empty() || out.response.lost_focus() && !clicking {
            self.open = false;
        }
        let last = self.matches.len().saturating_sub(1);
        if down {
            self.selected = (self.selected + 1).min(last);
        }
        if up {
            self.selected = self.selected.saturating_sub(1);
        }
        self.selected = self.selected.min(last);
        if accept && let Some(name) = self.matches.get(self.selected) {
            out.insert_completion(ui.ctx(), &mut self.url, name);
            self.open = false;
        }
        if let Some(caret) = out.caret_rect {
            let (matches, selected) = (self.matches.clone(), self.selected);
            let mut picked = None;
            Popover::new("gallery-url-completions", caret)
                .width(220.0)
                .show(ui.ctx(), &mut self.open, |ui| {
                    for (i, name) in matches.iter().enumerate() {
                        if ui
                            .add(MenuItem::new(*name).highlighted(i == selected))
                            .clicked()
                        {
                            picked = Some(*name);
                        }
                    }
                });
            if let Some(name) = picked {
                out.insert_completion(ui.ctx(), &mut self.url, name);
                self.open = false;
            }
        }
        out.submitted && !accept
    }
}
