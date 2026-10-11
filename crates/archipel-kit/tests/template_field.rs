//! `TextField` with a highlighter: `{{variable}}` highlighting, the caret API and the
//! autocomplete wiring documented on the code page.

mod common;

use archipel_kit::Theme;
use archipel_kit::components::code::{TemplateAt, TemplateHighlighter, TemplateToken};
use archipel_kit::components::{MenuItem, TextField};
use archipel_kit::egui::{self, Key, Modifiers};
use archipel_kit::parts::Popover;
use archipel_kit::testing::{self, Queryable};

const VARIABLES: [&str; 4] = ["base_url", "user_id", "user_name", "api_key"];

fn resolve(name: &str) -> TemplateToken {
    match name {
        "api_key" => TemplateToken::Secret,
        n if VARIABLES.contains(&n) => TemplateToken::Resolved,
        _ => TemplateToken::Unresolved,
    }
}

/// The caller's state of one URL bar with autocomplete.
#[derive(Default)]
struct Bar {
    url: String,
    open: bool,
    selected: usize,
    matches: Vec<&'static str>,
    submitted: usize,
    token: Option<TemplateAt>,
    caret: Option<egui::Rect>,
    field: Option<egui::Rect>,
}

impl Bar {
    fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            ..Self::default()
        }
    }
}

/// The wiring of the docs: keys first, then the field, then the popup.
fn url_bar(ui: &mut egui::Ui, bar: &mut Bar, enabled: bool) {
    let highlighter = TemplateHighlighter::new(resolve);
    // The list was open last frame: it owns the navigation keys, so the field does not
    // move its caret on Up/Down or submit on Enter.
    let listing = bar.open && !bar.matches.is_empty();
    let (mut down, mut up, mut accept) = (false, false, false);
    if listing {
        ui.input_mut(|i| {
            down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
            up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
            accept = i.consume_key(Modifiers::NONE, Key::Enter);
            if i.consume_key(Modifiers::NONE, Key::Escape) {
                bar.open = false;
            }
        });
    }
    let out = TextField::new(&mut bar.url)
        .label("URL")
        .placeholder("https://example.com")
        .monospace(true)
        .clearable(true)
        .enabled(enabled)
        .highlighter(&highlighter)
        .show(ui);
    bar.submitted += usize::from(out.submitted);
    bar.caret = out.caret_rect;
    bar.field = Some(out.response.rect);
    bar.token = out.template.clone();

    bar.matches = match &out.template {
        Some(token) => VARIABLES
            .into_iter()
            .filter(|v| v.starts_with(token.partial.as_str()))
            .collect(),
        None => Vec::new(),
    };
    if out.response.changed() {
        bar.open = true;
        bar.selected = 0;
    }
    // Tabbing away closes the list. A click moves the focus too: the popover closes
    // itself on clicks outside it, and a click inside must reach its item.
    let clicking = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released());
    if bar.matches.is_empty() || out.response.lost_focus() && !clicking {
        bar.open = false;
    }
    let last = bar.matches.len().saturating_sub(1);
    if down {
        bar.selected = (bar.selected + 1).min(last);
    }
    if up {
        bar.selected = bar.selected.saturating_sub(1);
    }
    bar.selected = bar.selected.min(last);
    if accept && let Some(name) = bar.matches.get(bar.selected) {
        out.insert_completion(ui.ctx(), &mut bar.url, name);
        bar.open = false;
    }
    if let Some(caret) = out.caret_rect {
        let (matches, selected) = (bar.matches.clone(), bar.selected);
        let mut picked = None;
        Popover::new("url-completions", caret)
            .width(220.0)
            .show(ui.ctx(), &mut bar.open, |ui| {
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
            out.insert_completion(ui.ctx(), &mut bar.url, name);
            bar.open = false;
        }
    }
}

fn bar_harness(url: &str) -> egui_kittest::Harness<'static, Bar> {
    testing::harness_state(Theme::dark(), Bar::new(url), |ui, bar| {
        url_bar(ui, bar, true)
    })
}

#[test]
fn snapshot_variables() {
    testing::snapshot_themes("template_field", common::IMG, |ui| {
        let highlighter = TemplateHighlighter::new(resolve);
        let mut url = "{{base_url}}/users/{{user_id}}".to_owned();
        ui.add(
            TextField::new(&mut url)
                .label("Resolved variables")
                .monospace(true)
                .clearable(true)
                .highlighter(&highlighter),
        );
        let mut missing = "{{base_url}}/{{missing}}".to_owned();
        ui.add(
            TextField::new(&mut missing)
                .label("Unresolved")
                .monospace(true)
                .highlighter(&highlighter),
        );
        let mut header = "Bearer {{api_key}}".to_owned();
        ui.add(
            TextField::new(&mut header)
                .label("Secret in a header value")
                .highlighter(&highlighter),
        );
        let mut empty = String::new();
        ui.add(
            TextField::new(&mut empty)
                .label("Empty")
                .placeholder("https://example.com")
                .highlighter(&highlighter),
        );
        let mut off = "{{base_url}}/{{missing}}".to_owned();
        ui.add(
            TextField::new(&mut off)
                .label("Disabled")
                .monospace(true)
                .enabled(false)
                .highlighter(&highlighter),
        );
    });
}

#[test]
fn snapshot_autocomplete_open() {
    let mut bar = Bar::new("{{base_url}}/users/");
    testing::snapshot_themes_sized(
        "template_autocomplete",
        common::IMG,
        egui::vec2(480.0, 190.0),
        |ui| {
            // Typed by hand once, then the frame repeats with the same state.
            if bar.url.ends_with('/') {
                bar.url.push_str("{{us");
                bar.open = true;
            }
            let highlighter = TemplateHighlighter::new(resolve);
            let out = TextField::new(&mut bar.url)
                .label("URL")
                .monospace(true)
                .highlighter(&highlighter)
                .show(ui);
            out.response.request_focus();
            let anchor = out.caret_rect.unwrap_or(out.response.rect);
            Popover::new("snapshot-completions", anchor)
                .width(220.0)
                .show(ui.ctx(), &mut bar.open, |ui| {
                    ui.add(MenuItem::new("user_id").highlighted(true));
                    ui.add(MenuItem::new("user_name"));
                });
        },
    );
}

#[test]
fn enter_submits_and_never_inserts_a_newline() {
    let mut harness = bar_harness("");
    let field = harness.get_by_label("URL");
    field.focus();
    field.type_text("https://a.test");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().url, "https://a.test");
    assert_eq!(harness.state().submitted, 1);
}

#[test]
fn caret_rect_follows_the_caret_inside_the_field() {
    let mut harness = bar_harness("");
    let field = harness.get_by_label("URL");
    field.focus();
    harness.run();
    let empty = harness.state().caret.expect("a caret once focused");
    harness.get_by_label("URL").type_text("abcdef");
    harness.run();
    let typed = harness.state().caret.expect("a value");
    let rect = harness.state().field.expect("a value");
    assert!(typed.left() > empty.left() + 20.0, "{typed:?} vs {empty:?}");
    assert!(
        rect.contains_rect(typed.shrink(0.5)),
        "{typed:?} in {rect:?}"
    );
}

#[test]
fn typing_a_variable_reports_the_token_and_its_partial_name() {
    let mut harness = bar_harness("");
    harness.get_by_label("URL").focus();
    harness.get_by_label("URL").type_text("https://{{us");
    harness.run();
    let token = harness.state().token.clone().expect("token at the caret");
    assert_eq!(token.partial, "us");
    assert_eq!(token.range, 8..12);
    assert!(!token.closed);
    assert_eq!(harness.state().matches, ["user_id", "user_name"]);
    assert!(harness.state().open);

    harness.get_by_label("URL").type_text("}}/x");
    harness.run();
    assert!(harness.state().token.is_none(), "the caret left the token");
    assert!(!harness.state().open);
}

#[test]
fn autocomplete_keyboard_up_down_enter() {
    let mut harness = bar_harness("");
    harness.get_by_label("URL").focus();
    harness.get_by_label("URL").type_text("a/{{us");
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selected, 1);
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selected, 1);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().url, "a/{{user_name}}");
    assert_eq!(
        harness.state().submitted,
        0,
        "Enter accepted, not submitted"
    );
    assert!(!harness.state().open);

    // The caret sits after the completion: typing continues the URL.
    harness.get_by_label("URL").type_text("/posts");
    harness.run();
    assert_eq!(harness.state().url, "a/{{user_name}}/posts");

    // With the list closed, Enter submits.
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().submitted, 1);
}

#[test]
fn escape_closes_the_list_and_keeps_the_text() {
    let mut harness = bar_harness("");
    harness.get_by_label("URL").focus();
    harness.get_by_label("URL").type_text("{{b");
    harness.run();
    assert!(harness.state().open);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().open);
    assert_eq!(harness.state().url, "{{b");
}

#[test]
fn clicking_a_suggestion_completes_the_token() {
    let mut harness = bar_harness("");
    harness.get_by_label("URL").focus();
    harness.get_by_label("URL").type_text("x {{api");
    harness.run();
    harness.get_by_label("api_key").click();
    harness.run();
    assert_eq!(harness.state().url, "x {{api_key}}");
}

#[test]
fn completion_replaces_an_existing_closing_brace_pair() {
    let mut harness = bar_harness("{{us}}");
    harness.get_by_label("URL").focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness.get_by_label("URL").type_text("e");
    harness.run();
    let token = harness.state().token.clone().expect("a value");
    assert_eq!((token.partial.as_str(), token.closed), ("use", true));
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().url,
        "{{user_id}}",
        "the closing braces are replaced"
    );
}

#[test]
fn disabled_field_reports_no_token_and_ignores_typing() {
    let mut harness = testing::harness_state(Theme::dark(), Bar::new("{{x}}"), |ui, bar| {
        url_bar(ui, bar, false)
    });
    harness.get_by_label("URL").type_text("abc");
    harness.run();
    assert_eq!(harness.state().url, "{{x}}");
    assert!(harness.state().token.is_none());
}

#[test]
fn field_conforms_enabled_and_disabled() {
    let highlighter = TemplateHighlighter::new(resolve);
    let mut text = String::from("{{base_url}}/{{missing}}");
    testing::conformance("URL", move |ui, enabled| {
        ui.add(
            TextField::new(&mut text)
                .label("URL")
                .enabled(enabled)
                .highlighter(&highlighter),
        )
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn highlighted_fields_pass_the_audit() {
    let highlighter = TemplateHighlighter::new(resolve);
    let mut url = String::from("{{base_url}}/{{missing}}?k={{api_key}}");
    let mut header = String::new();
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(
            TextField::new(&mut url)
                .label("URL")
                .clearable(true)
                .highlighter(&highlighter),
        );
        ui.add(
            TextField::new(&mut header)
                .label("Header value")
                .highlighter(&highlighter)
                .error(Some("Unknown variable")),
        );
    });
    testing::audit(&mut harness).assert_clean();
}
