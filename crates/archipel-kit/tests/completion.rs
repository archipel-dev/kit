//! `JBTextField::completer`: the kit draws and operates the completion list.

use archipel_kit::Theme;
use archipel_kit::egui::Key;
use archipel_kit::testing::{self, Queryable};
use archipel_kit::ui::{Completion, Completions, JBTextField};

const VARIABLES: [&str; 3] = ["base_url", "user_id", "user_name"];

/// `{{` up to the caret, then the variables starting with what was typed.
fn complete(text: &str, caret: usize) -> Option<Completions> {
    let before: String = text.chars().take(caret).collect();
    let open = before.rfind("{{")?;
    let typed = &before[open + 2..];
    if typed.contains("}}") {
        return None;
    }
    Some(Completions {
        start: before[..open].chars().count(),
        end: caret,
        items: VARIABLES
            .iter()
            .filter(|v| v.starts_with(typed))
            .map(|v| Completion::new(*v, format!("{{{{{v}}}}}")).detail("variable"))
            .collect(),
    })
}

fn harness() -> testing::Harness<'static, String> {
    testing::harness_state(Theme::dark(), String::new(), |ui, url| {
        ui.add(JBTextField::new(url).name("URL").completer(&complete));
    })
}

#[test]
fn typing_opens_the_list_and_enter_inserts_the_first() {
    let mut h = harness();
    h.get_by_label("URL").focus();
    h.get_by_label("URL").type_text("a/{{us");
    h.run();
    assert!(
        h.query_by_label_contains("user_id").is_some(),
        "list is open"
    );
    assert!(h.query_by_label_contains("base_url").is_none(), "filtered");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state(), "a/{{user_id}}");
    assert!(
        h.query_by_label_contains("user_name").is_none(),
        "list closed"
    );
}

#[test]
fn arrows_move_and_tab_inserts() {
    let mut h = harness();
    h.get_by_label("URL").focus();
    h.get_by_label("URL").type_text("{{us");
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state(), "{{user_name}}");
    // The caret is after the insertion: typing continues there.
    h.get_by_label("URL").type_text("/x");
    h.run();
    assert_eq!(h.state(), "{{user_name}}/x");
}

#[test]
fn escape_closes_and_a_click_inserts() {
    let mut h = harness();
    h.get_by_label("URL").focus();
    h.get_by_label("URL").type_text("{{");
    h.run();
    assert!(
        h.query_by_label_contains("base_url").is_none(),
        "`{{` is not an identifier character: nothing opens"
    );
    h.get_by_label("URL").type_text("b");
    h.run();
    assert!(h.query_by_label_contains("base_url").is_some());
    h.key_press(Key::Escape);
    h.run();
    assert!(h.query_by_label_contains("base_url").is_none());
    h.get_by_label("URL").type_text("a");
    h.run();
    assert_eq!(h.state(), "{{ba", "the field kept the focus");
    // A click selects; a double click inserts.
    h.get_by_label_contains("base_url").click();
    h.run();
    assert_eq!(h.state(), "{{ba", "a single click only selects");
    h.get_by_label_contains("base_url").click();
    h.run();
    h.run();
    assert_eq!(h.state(), "{{base_url}}");
}

#[test]
fn enter_without_a_list_still_submits() {
    let mut h = testing::harness_state(Theme::dark(), (String::new(), false), |ui, (url, sent)| {
        let out = JBTextField::new(url)
            .name("URL")
            .completer(&complete)
            .show(ui);
        *sent |= out.submitted;
    });
    h.get_by_label("URL").focus();
    h.get_by_label("URL").type_text("https://a.test");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().1);
}

#[test]
fn snapshot() {
    mod common {
        pub const IMG: &str = "../../docs/components/img";
    }
    testing::snapshot_themes_after(
        "jb_text_field_completion",
        common::IMG,
        |ui| {
            ui.set_width(360.0);
            let id = archipel_kit::egui::Id::new("completion-snapshot");
            let mut url = ui.data_mut(|d| d.get_temp_mut_or_default::<String>(id).clone());
            ui.add(
                JBTextField::new(&mut url)
                    .name("URL")
                    .monospace(true)
                    .completer(&complete),
            );
            ui.data_mut(|d| d.insert_temp(id, url));
            ui.add_space(110.0);
        },
        |h| {
            h.get_by_label("URL").focus();
            h.get_by_label("URL").type_text("https://{{us");
            h.run();
        },
    );
}

/// Fields of a query: the word before the caret, completed with arguments when they need one.
fn fields(text: &str, caret: usize) -> Option<Completions> {
    let before: String = text.chars().take(caret).collect();
    let word_start = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric())
        .last()
        .map_or(caret, |(i, _)| before[..i].chars().count());
    let typed = &before[before
        .char_indices()
        .nth(word_start)
        .map_or(before.len(), |(i, _)| i)..];
    Some(Completions {
        start: word_start,
        end: caret,
        items: [("user", "user(id: )", Some(9)), ("users", "users", None)]
            .into_iter()
            .filter(|(name, _, _)| name.starts_with(typed))
            .map(|(name, insert, caret)| {
                let item = Completion::new(name, insert).detail("User");
                match caret {
                    Some(c) => item.caret(c),
                    None => item,
                }
            })
            .collect(),
    })
}

#[test]
fn code_editor_completes_and_places_the_caret_inside() {
    use archipel_kit::editor::{EditorComponent, model::Editor};
    let mut h = testing::harness_state(Theme::dark(), Editor::new("query {\n  "), |ui, editor| {
        ui.add(
            EditorComponent::new(editor)
                .label("Query")
                .height(160.0)
                .completer(&fields),
        );
    });
    h.get_by_label("Query").focus();
    h.run();
    h.key_press_modifiers(archipel_kit::egui::Modifiers::CTRL, Key::End);
    h.run();
    h.get_by_label("Query").type_text("us");
    h.run();
    assert!(h.query_by_label_contains("users").is_some(), "list is open");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().document.text(), "query {\n  user(id: )");
    // The caret is between the parentheses.
    h.get_by_label("Query").type_text("7");
    h.run();
    assert_eq!(h.state().document.text(), "query {\n  user(id: 7)");
}

#[test]
fn ctrl_space_opens_the_list_in_a_code_editor() {
    use archipel_kit::editor::{EditorComponent, model::Editor};
    let mut h = testing::harness_state(Theme::dark(), Editor::new(""), |ui, editor| {
        ui.add(
            EditorComponent::new(editor)
                .label("Query")
                .height(160.0)
                .completer(&fields),
        );
    });
    h.get_by_label("Query").focus();
    h.run();
    assert!(h.query_by_label_contains("users").is_none());
    h.key_press_modifiers(archipel_kit::egui::Modifiers::CTRL, Key::Space);
    h.run();
    assert!(h.query_by_label_contains("users").is_some());
}

/// Waits for a typed letter, but lists everything when asked.
struct Patient;

impl archipel_kit::ui::Completer for Patient {
    fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
        let typed = text
            .chars()
            .take(caret)
            .last()
            .is_some_and(char::is_alphanumeric);
        typed
            .then(|| self.complete_on_request(text, caret))
            .flatten()
    }

    fn complete_on_request(&self, _text: &str, caret: usize) -> Option<Completions> {
        Some(Completions {
            start: caret,
            end: caret,
            items: vec![
                Completion::new("users", "users"),
                Completion::new("user", "user"),
            ],
        })
    }
}

#[test]
fn a_patient_completer_opens_on_request_only() {
    use archipel_kit::editor::{EditorComponent, model::Editor};
    let mut h = testing::harness_state(Theme::dark(), Editor::new(""), |ui, editor| {
        ui.add(
            EditorComponent::new(editor)
                .label("Query")
                .height(160.0)
                .completer(&Patient),
        );
    });
    h.get_by_label("Query").focus();
    h.get_by_label("Query").type_text("{ ");
    h.run();
    assert!(
        h.query_by_label_contains("users").is_none(),
        "no list after `{{ `"
    );
    h.key_press_modifiers(archipel_kit::egui::Modifiers::CTRL, Key::Space);
    h.run();
    assert!(
        h.query_by_label_contains("users").is_some(),
        "Ctrl+Space lists everything"
    );
}
