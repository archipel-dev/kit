//! `TextField::completer`: the kit draws and operates the completion list.

use archipel_kit::Theme;
use archipel_kit::components::{Completion, Completions, TextField};
use archipel_kit::egui::Key;
use archipel_kit::testing::{self, Queryable};

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
        items: VARIABLES
            .iter()
            .filter(|v| v.starts_with(typed))
            .map(|v| Completion::new(*v, format!("{{{{{v}}}}}")).detail("variable"))
            .collect(),
    })
}

fn harness() -> testing::Harness<'static, String> {
    testing::harness_state(Theme::dark(), String::new(), |ui, url| {
        ui.add(TextField::new(url).name("URL").completer(&complete));
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
    assert!(h.query_by_label_contains("base_url").is_some());
    h.key_press(Key::Escape);
    h.run();
    assert!(h.query_by_label_contains("base_url").is_none());
    h.get_by_label("URL").type_text("b");
    h.run();
    assert_eq!(h.state(), "{{b", "the field kept the focus");
    h.get_by_label_contains("base_url").click();
    h.run();
    h.run();
    assert_eq!(h.state(), "{{base_url}}");
}

#[test]
fn enter_without_a_list_still_submits() {
    let mut h = testing::harness_state(Theme::dark(), (String::new(), false), |ui, (url, sent)| {
        let out = TextField::new(url)
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
        "text_field_completion",
        common::IMG,
        |ui| {
            ui.set_width(360.0);
            let id = archipel_kit::egui::Id::new("completion-snapshot");
            let mut url = ui.data_mut(|d| d.get_temp_mut_or_default::<String>(id).clone());
            ui.add(
                TextField::new(&mut url)
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
