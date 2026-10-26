use super::*;
use archipel_kit::editor::model::{Editor, SearchHistory};
use archipel_kit::editor::{
    ChangeKind, ChangeMarker, GutterIcon, HighlightSeverity, JavaScript, RangeHighlight,
};

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

/// A script for the gutter demo.
const SCRIPT_SAMPLE: &str = "function total(items) {\n  let sum = 0;\n  for (const item of items) {\n    sum += item.price * item.qty;\n  }\n  // TODO: tax\n  return sum;\n}\n";

pub(super) struct State {
    last_action: String,
    viewer: Editor,
    editor: Editor,
    script: Editor,
    icons: Vec<GutterIcon>,
    markers: Vec<ChangeMarker>,
    highlights: Vec<RangeHighlight>,
    clicked: String,
    history: SearchHistory,
    url_bar: UrlBar,
    header: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_action: String::new(),
            viewer: Editor::new(CODE_SAMPLE),
            editor: Editor::new(CODE_SAMPLE),
            script: Editor::new(SCRIPT_SAMPLE),
            icons: vec![
                GutterIcon::new(1, Glyph::Breakpoint, "Breakpoint on line 2").tone(Tone::Error),
                GutterIcon::new(5, Glyph::Bookmark, "Bookmark on line 6").lane(1),
            ],
            markers: vec![
                ChangeMarker::new(1..3, ChangeKind::Added),
                ChangeMarker::new(4..5, ChangeKind::Modified),
            ],
            highlights: vec![
                RangeHighlight::new(44..47, HighlightSeverity::Error)
                    .message("Unresolved variable"),
                RangeHighlight::new(97..104, HighlightSeverity::Warning).message("Unused"),
            ],
            clicked: String::new(),
            history: SearchHistory::new(),
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
        ui.add(TitledSeparator::new("Template variables"));
        let submitted = self.url_bar.show(ui, enabled);
        if submitted {
            self.last_action = format!("Sent {}", self.url_bar.url);
        }
        ui.label("Type {{ for suggestions: Up/Down move, Enter or a click completes, Esc closes.");
        let highlighter = TemplateHighlighter::new(resolve_variable);
        ui.add(
            JBTextField::new(&mut self.header)
                .label("Header value")
                .highlighter(&highlighter)
                .clearable(true)
                .enabled(enabled),
        );
        ui.add(TitledSeparator::new("Viewer"));
        ui.label("Ctrl+F finds in the viewer (Enter and F3 step, Alt+C / Alt+W / Alt+X toggle Match Case, Words and Regex, Esc closes); the editor below also has Ctrl+R to replace.");
        EditorComponent::new(&mut self.viewer)
            .search_history(&mut self.history)
            .label("Response body")
            .highlighter(&Json)
            .viewer(true)
            .line_numbers(true)
            .bracket_folding(true)
            .height(236.0)
            .show(ui);
        ui.add(TitledSeparator::new("Gutter, markers and folding"));
        ui.label("Hover the gutter for fold chevrons; Alt+drag selects a column, Alt+Shift+click adds a caret.");
        let out = EditorComponent::new(&mut self.script)
            .label("Script")
            .highlighter(&JavaScript)
            .viewer(true)
            .line_numbers(true)
            .bracket_folding(true)
            .gutter_icons(&self.icons)
            .change_markers(&self.markers)
            .highlights(&self.highlights)
            .enabled(enabled)
            .height(200.0)
            .show(ui);
        if let Some(click) = out.gutter_clicks.first() {
            self.clicked = format!("Gutter click on line {} ({:?})", click.line + 1, click.area);
        }
        if !self.clicked.is_empty() {
            ui.label(&self.clicked);
        }
        ui.add(TitledSeparator::new("Editor"));
        ui.add(
            EditorComponent::new(&mut self.editor)
                .search_history(&mut self.history)
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
        let out = JBTextField::new(&mut self.url)
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
            JBPopup::new("gallery-url-completions", caret)
                .width(220.0)
                .show(ui.ctx(), &mut self.open, |ui| {
                    for (i, name) in matches.iter().enumerate() {
                        if ui
                            .add(ActionMenuItem::new(*name).highlighted(i == selected))
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
