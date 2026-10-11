//! `CodeView` and `CodeEditor` tests: behavior, accessibility, virtualization and
//! dark/light snapshots.

mod common;

use archipel_kit::components::code::{GraphQl, JavaScript, Json, PlainText, Xml};
use archipel_kit::components::{CodeEditor, CodeView};
use archipel_kit::testing::{self, NodeExt, NodeT, Queryable, Role};
use archipel_kit::{Theme, egui};
use egui::{Event, Key, Modifiers, OutputCommand};

const BODY: &str = r#"{
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

fn ctrl(harness: &testing::Harness<'_, impl Sized>, key: Key) {
    harness.key_press_modifiers(Modifiers::COMMAND, key);
}

/// Records the text copied during the frame into `slot`.
fn record_copy(ctx: &egui::Context, slot: &mut Option<String>) {
    ctx.output(|o| {
        for command in &o.commands {
            if let OutputCommand::CopyText(text) = command {
                *slot = Some(text.clone());
            }
        }
    });
}
/// A JSON array of `n` objects, one per line.
fn big_json(n: usize) -> String {
    let mut out = String::with_capacity(n * 40);
    out.push_str("[\n");
    for i in 0..n {
        out.push_str(&format!(
            "  {{\"id\": {i}, \"name\": \"item {i}\", \"ok\": true}}{}\n",
            if i + 1 < n { "," } else { "" }
        ));
    }
    out.push(']');
    out
}

#[test]
fn view_snapshot() {
    testing::snapshot_themes("code_view", common::IMG, |ui| {
        CodeView::new(BODY)
            .label("Response body")
            .highlighter(&Json)
            .line_numbers(true)
            .folding(true)
            .height(236.0)
            .show(ui);
    });
}

#[test]
fn view_find_snapshot() {
    testing::snapshot_themes_after(
        "code_view_find",
        common::IMG,
        |ui| {
            CodeView::new(BODY)
                .label("Response body")
                .highlighter(&Json)
                .line_numbers(true)
                .height(236.0)
                .show(ui);
        },
        |harness| {
            harness.get_by_label("Response body").focus();
            harness.run_ok();
            ctrl(harness, Key::F);
            harness.run_ok();
            harness.event(Event::Text("o".into()));
            harness.run_ok();
        },
    );
}

#[test]
fn highlighting_snapshot() {
    testing::snapshot_themes("code_highlighting", common::IMG, |ui| {
        let samples: [(&str, &dyn archipel_kit::components::code::Highlighter, &str); 5] = [
            ("JSON", &Json, "{ \"ok\": true, \"n\": [1, -2.5e3, null] }"),
            (
                "XML",
                &Xml,
                "<a href=\"/x\" id='y'>Tom &amp; Jerry</a> <!-- note -->",
            ),
            (
                "GraphQL",
                &GraphQl,
                "query Q($id: ID!) { user(id: $id) @live { name } }",
            ),
            (
                "JavaScript",
                &JavaScript,
                "const n = items.map(x => x * 2); // double",
            ),
            (
                "Plain text",
                &PlainText,
                "Just some text, 42 and \"quotes\".",
            ),
        ];
        for (name, highlighter, code) in samples {
            CodeView::new(code)
                .label(name)
                .highlighter(highlighter)
                .height(30.0)
                .id(egui::Id::new(name))
                .show(ui);
        }
    });
}

#[test]
fn editor_snapshot() {
    let mut body = BODY.to_owned();
    testing::snapshot_themes_after(
        "code_editor",
        common::IMG,
        |ui| {
            ui.add(
                CodeEditor::new(&mut body)
                    .label("Request body")
                    .highlighter(&Json)
                    .line_numbers(true)
                    .height(236.0),
            );
        },
        |harness| {
            harness.get_by_label("Request body").focus();
            harness.run_ok();
            ctrl(harness, Key::A);
            harness.run_ok();
        },
    );
}

#[test]
fn view_exposes_a_document_with_its_text() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        CodeView::new("let x = 1;")
            .label("Script")
            .height(60.0)
            .show(ui);
    });
    let node = harness.get_by_label("Script");
    assert_eq!(node.role(), Role::Document);
    assert_eq!(node.accesskit_node().value().as_deref(), Some("let x = 1;"));
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn a_big_document_lays_out_only_its_visible_lines() {
    let text = big_json(200_000);
    assert!(text.len() > 8_000_000, "{} bytes", text.len());
    #[derive(Default)]
    struct Report(Option<usize>, (usize, std::ops::Range<usize>, usize));
    let mut harness = testing::harness_state(
        Theme::dark(),
        Report::default(),
        |ui, Report(jump, report)| {
            let r = CodeView::new(&text)
                .label("Big")
                .highlighter(&Json)
                .line_numbers(true)
                .scroll_to_line(jump.take())
                .height(300.0)
                .show(ui);
            *report = (r.line_count, r.visible_lines, r.laid_out_rows);
        },
    );
    harness.run_ok();
    harness.state_mut().0 = Some(150_000);
    harness.run_ok();
    harness.run_ok();
    let report = harness.state().1.clone();
    let (lines, visible, laid_out) = report;
    assert_eq!(lines, 200_002);
    assert!(
        visible.contains(&150_000),
        "scrolled to line 150000, showing {visible:?}"
    );
    assert!(visible.len() <= 30, "{visible:?}");
    assert!(laid_out <= 30, "laid out {laid_out} rows of {lines} lines");
}

#[test]
fn select_all_and_copy() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, copied| {
        CodeView::new(BODY).label("Body").height(200.0).show(ui);
        record_copy(ui.ctx(), copied);
    });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    ctrl(&harness, Key::A);
    harness.run_ok();
    ctrl(&harness, Key::C);
    harness.run_ok();
    assert_eq!(harness.state().as_deref(), Some(BODY));
}

#[test]
fn dragging_selects_text() {
    let mut harness = testing::harness_state(Theme::dark(), None, |ui, copied| {
        CodeView::new("hello world\nsecond")
            .label("Body")
            .height(80.0)
            .show(ui);
        record_copy(ui.ctx(), copied);
    });
    let rect = harness.get_by_label("Body").rect();
    let y = rect.top() + 4.0 + 10.0;
    let start = egui::pos2(rect.left() + 8.0, y);
    harness.hover_at(start);
    harness.event(Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run_ok();
    let end = egui::pos2(rect.left() + 8.0 + 5.0 * 7.9, y);
    harness.event(Event::PointerMoved(end));
    harness.run_ok();
    harness.event(Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_ok();
    ctrl(&harness, Key::C);
    harness.run_ok();
    assert_eq!(harness.state().as_deref(), Some("hello"));
}

#[test]
fn find_highlights_counts_and_closes() {
    let mut matches = 0;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        matches = CodeView::new(BODY)
            .label("Body")
            .highlighter(&Json)
            .height(236.0)
            .show(ui)
            .matches;
    });
    assert!(
        harness.query_by_label("Next match").is_none(),
        "closed at first"
    );
    harness.get_by_label("Body").focus();
    harness.run_ok();
    ctrl(&harness, Key::F);
    harness.run_ok();
    assert!(harness.query_by_label("Next match").is_some());
    harness.event(Event::Text("a".into()));
    harness.run_ok();
    // The query field has the focus.
    assert!(
        harness
            .get_by_role(Role::TextInput)
            .accesskit_node()
            .is_focused()
    );
    harness.get_by_label("Next match").click();
    harness.run_ok();
    testing::audit(&mut harness).assert_clean();
    harness.key_press(Key::Escape);
    harness.run_ok();
    assert!(
        harness.query_by_label("Next match").is_none(),
        "escape closes the bar"
    );
    drop(harness);
    assert_eq!(matches, 0, "no matches while closed");
}

#[test]
fn find_counts_matches_case_insensitively() {
    let mut matches = 0;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        matches = CodeView::new("Foo foo\nFOO bar")
            .label("Body")
            .height(100.0)
            .show(ui)
            .matches;
    });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    ctrl(&harness, Key::F);
    harness.run_ok();
    harness.event(Event::Text("foo".into()));
    harness.run_ok();
    harness.run_ok();
    drop(harness);
    assert_eq!(matches, 3);
}

#[test]
fn folding_hides_the_lines_of_a_region() {
    let mut visible = 0..0;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let r = CodeView::new(BODY)
            .label("Body")
            .highlighter(&Json)
            .line_numbers(true)
            .folding(true)
            .height(260.0)
            .show(ui);
        visible = r.visible_lines;
    });
    // The first fold arrow is the whole document: `{` on line 1 closes on line 11.
    harness.get_by_label("Fold lines 1 to 11").click();
    harness.run_ok();
    assert!(harness.query_by_label("Unfold lines 1 to 11").is_some());
    harness.get_by_label("Unfold lines 1 to 11").click();
    harness.run_ok();
    harness.get_by_label("Fold lines 6 to 9").click();
    harness.run_ok();
    testing::audit(&mut harness).assert_clean();
    drop(harness);
    assert_eq!(visible, 0..11);
}

#[test]
fn cursor_is_a_beam_over_text_and_an_arrow_over_the_gutter() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        CodeView::new(BODY)
            .label("Body")
            .line_numbers(true)
            .height(200.0)
            .show(ui);
    });
    let rect = harness.get_by_label("Body").rect();
    harness.hover_at(egui::pos2(rect.left() + 60.0, rect.top() + 20.0));
    harness.run_ok();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Text
    );
    harness.hover_at(egui::pos2(rect.left() - 14.0, rect.top() + 20.0));
    harness.run_ok();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Default
    );
}

#[test]
fn wrapped_view_breaks_long_lines() {
    let long = "x".repeat(400);
    let mut rows = 0;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        rows = CodeView::new(&long)
            .label("Body")
            .wrap(true)
            .height(200.0)
            .show(ui)
            .laid_out_rows;
    });
    harness.run_ok();
    drop(harness);
    assert!(rows > 3, "400 columns wrap onto several rows, got {rows}");
}

#[test]
fn highlighters_color_the_expected_tokens() {
    use archipel_kit::components::code::{
        Highlighter, Span, TokenKind,
        TokenKind::{Attribute, Comment, Error, Keyword, Number, Property, Tag, Variable},
    };
    use std::string::String as Text;
    fn kinds(h: &dyn Highlighter, line: &str) -> Vec<(Text, TokenKind)> {
        let mut spans: Vec<Span> = Vec::new();
        h.highlight_line(line, &mut 0, &mut spans);
        spans
            .iter()
            .map(|s| (line[s.start..s.end].to_owned(), s.kind))
            .collect()
    }
    let json = kinds(&Json, r#"{"a": [1, true, "s"], "b": nope}"#);
    assert!(json.contains(&("\"a\"".into(), Property)));
    assert!(json.contains(&("1".into(), Number)));
    assert!(json.contains(&("true".into(), Keyword)));
    assert!(json.contains(&("\"s\"".into(), TokenKind::String)));
    assert!(json.contains(&("nope".into(), Error)));
    let xml = kinds(&Xml, r#"<a href="x">hi &amp; bye</a>"#);
    assert!(xml.contains(&("<a".into(), Tag)));
    assert!(xml.contains(&("href".into(), Attribute)));
    assert!(xml.contains(&("\"x\"".into(), TokenKind::String)));
    assert!(xml.contains(&("&amp;".into(), Variable)));
    let gql = kinds(&GraphQl, "query Q($id: ID!) @live { user(id: $id) }");
    assert!(gql.contains(&("query".into(), Keyword)));
    assert!(gql.contains(&("$id".into(), Variable)));
    assert!(gql.contains(&("@live".into(), Attribute)));
    assert!(gql.contains(&("ID".into(), Tag)));
    assert!(gql.contains(&("id".into(), Property)));
    let js = kinds(&JavaScript, "const x = a.b(1); // hi");
    assert!(js.contains(&("const".into(), Keyword)));
    assert!(js.contains(&("b".into(), Property)));
    assert!(js.contains(&("// hi".into(), Comment)));
    // Multi-line constructs carry their state to the next line.
    let mut state = 0;
    let mut spans = Vec::new();
    JavaScript.highlight_line("/* open", &mut state, &mut spans);
    assert_ne!(state, 0);
    spans.clear();
    JavaScript.highlight_line("still */ let y", &mut state, &mut spans);
    assert_eq!(state, 0);
    assert_eq!(
        (spans[0].start, spans[0].end, spans[0].kind),
        (0, 8, Comment)
    );
}

// ---- editor ----

#[test]
fn typing_updates_the_bound_string() {
    let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        ui.add(CodeEditor::new(text).label("Body").height(100.0));
    });
    let editor = harness.get_by_label("Body");
    assert_eq!(editor.role(), Role::TextInput);
    editor.focus();
    editor.type_text("let a");
    harness.run_ok();
    assert_eq!(harness.state(), "let a");
}

#[test]
fn tab_inserts_spaces_and_enter_keeps_the_indent() {
    let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        ui.add(
            CodeEditor::new(text)
                .label("Body")
                .height(100.0)
                .tab_width(2),
        );
    });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    harness.key_press(Key::Tab);
    harness.run_ok();
    harness.get_by_label("Body").type_text("a");
    harness.run_ok();
    harness.key_press(Key::Enter);
    harness.run_ok();
    harness.get_by_label("Body").type_text("b");
    harness.run_ok();
    assert_eq!(harness.state(), "  a\n  b");
    assert!(
        harness.get_by_label("Body").accesskit_node().is_focused(),
        "Tab stays in the editor"
    );
}

#[test]
fn brackets_pair_and_the_closer_is_skipped() {
    let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        ui.add(CodeEditor::new(text).label("Body").height(100.0));
    });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    harness.get_by_label("Body").type_text("{");
    harness.run_ok();
    assert_eq!(harness.state(), "{}");
    harness.key_press(Key::Enter);
    harness.run_ok();
    assert_eq!(harness.state(), "{\n    \n}");
    harness.get_by_label("Body").type_text("x");
    harness.run_ok();
    assert_eq!(harness.state(), "{\n    x\n}");
}

#[test]
fn bracket_pairing_can_be_turned_off() {
    let mut harness = testing::harness_state(Theme::dark(), String::new(), |ui, text| {
        ui.add(
            CodeEditor::new(text)
                .label("Body")
                .bracket_pairing(false)
                .height(100.0),
        );
    });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    harness.get_by_label("Body").type_text("(");
    harness.run_ok();
    assert_eq!(harness.state(), "(");
}

#[test]
fn editor_find_opens_and_closes() {
    let mut harness =
        testing::harness_state(Theme::dark(), "one two one".to_owned(), |ui, text| {
            ui.add(
                CodeEditor::new(text)
                    .label("Body")
                    .tab_indents(false)
                    .height(160.0),
            );
        });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    ctrl(&harness, Key::F);
    harness.run_ok();
    harness.event(Event::Text("one".into()));
    harness.run_ok();
    assert!(harness.query_by_label("Next match").is_some());
    harness.get_by_label("Next match").click();
    harness.run_ok();
    assert_eq!(harness.state(), "one two one", "find doesn't edit the text");
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Close find").click();
    harness.run_ok();
    assert!(harness.query_by_label("Next match").is_none());
}

#[test]
fn big_editor_text_stays_responsive() {
    // About 1 MB: the documented limit.
    let text = big_json(25_000);
    assert!(text.len() > 900_000, "{} bytes", text.len());
    let mut harness = testing::harness_state(Theme::dark(), text, |ui, text| {
        ui.add(
            CodeEditor::new(text)
                .label("Body")
                .highlighter(&Json)
                .line_numbers(true)
                .height(300.0),
        );
    });
    let started = std::time::Instant::now();
    harness.get_by_label("Body").focus();
    harness.get_by_label("Body").type_text("x");
    harness.run_ok();
    // Generous: unoptimized test builds are far slower than release ones.
    assert!(started.elapsed().as_secs() < 30, "{:?}", started.elapsed());
}
