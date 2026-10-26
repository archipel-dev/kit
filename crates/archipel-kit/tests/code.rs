//! `EditorComponent` as a code view (`viewer(true)`) and as a code editor: behavior,
//! accessibility, virtualization and dark/light snapshots. The rules of the IntelliJ editor
//! (keys, mouse, painting) are in `tests/editor_component.rs`.

mod common;

use archipel_kit::editor::EditorComponent;
use archipel_kit::editor::model::Editor;
use archipel_kit::editor::{
    ExtendedJson, GraphQl, Highlighter, JavaScript, Json, PlainText, Span, TokenKind, Xml,
};
use archipel_kit::parts::ValueKind;
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
    let mut viewer = Editor::new(BODY);
    testing::snapshot_themes("code_view", common::IMG, |ui| {
        EditorComponent::new(&mut viewer)
            .label("Response body")
            .highlighter(&Json)
            .viewer(true)
            .line_numbers(true)
            .bracket_folding(true)
            .height(236.0)
            .show(ui);
    });
}

#[test]
fn view_find_snapshot() {
    let mut viewer = Editor::new(BODY);
    testing::snapshot_themes_after(
        "code_view_find",
        common::IMG,
        |ui| {
            EditorComponent::new(&mut viewer)
                .label("Response body")
                .highlighter(&Json)
                .viewer(true)
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
    let samples: [(&str, &dyn Highlighter, &str); 6] = [
        ("JSON", &Json, "{ \"ok\": true, \"n\": [1, -2.5e3, null] }"),
        (
            "Extended JSON",
            &ExtendedJson,
            "{ _id: ObjectId(\"65f0\"), at: ISODate(\"2026\") }",
        ),
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
    let mut editors: Vec<Editor> = samples
        .iter()
        .map(|(_, _, code)| Editor::new(code))
        .collect();
    testing::snapshot_themes("code_highlighting", common::IMG, |ui| {
        for ((name, highlighter, _), editor) in samples.iter().zip(editors.iter_mut()) {
            EditorComponent::new(editor)
                .label(*name)
                .highlighter(*highlighter)
                .viewer(true)
                .height(30.0)
                .id(egui::Id::new(name))
                .show(ui);
        }
    });
}

fn kinds(highlighter: &dyn Highlighter, line: &str) -> Vec<(String, TokenKind)> {
    let mut spans: Vec<Span> = Vec::new();
    highlighter.highlight_line(line, &mut 0, &mut spans);
    spans
        .into_iter()
        .filter(|s| s.kind != TokenKind::Punctuation)
        .map(|s| (line[s.start..s.end].to_owned(), s.kind))
        .collect()
}

#[test]
fn extended_json_colors_constructors_as_their_values() {
    let line = r#"{ _id: ObjectId("65f0"), "at": new Date('2026-10-12'), n: NumberLong("7"), x: -1.5e3, b: BinData(0, "AA=="), s: 'it\'s', ok: false, z: null } // last"#;
    assert_eq!(
        kinds(&ExtendedJson, line),
        [
            ("_id", TokenKind::Property),
            ("ObjectId(\"65f0\")", TokenKind::Value(ValueKind::Id)),
            ("\"at\"", TokenKind::Property),
            ("new Date('2026-10-12')", TokenKind::Value(ValueKind::Date)),
            ("n", TokenKind::Property),
            ("NumberLong(\"7\")", TokenKind::Value(ValueKind::Number)),
            ("x", TokenKind::Property),
            ("-1.5e3", TokenKind::Number),
            ("b", TokenKind::Property),
            ("BinData(0, \"AA==\")", TokenKind::Value(ValueKind::Binary)),
            ("s", TokenKind::Property),
            ("'it\\'s'", TokenKind::String),
            ("ok", TokenKind::Property),
            ("false", TokenKind::Value(ValueKind::Boolean)),
            ("z", TokenKind::Property),
            ("null", TokenKind::Value(ValueKind::Null)),
            ("// last", TokenKind::Comment),
        ]
        .map(|(text, kind)| (text.to_owned(), kind))
    );
}

#[test]
fn extended_json_survives_broken_input() {
    for line in [
        "ObjectId(\"65f",
        "{ a: 'open",
        "new",
        "new Foo(1)",
        "é: ünïcode",
        "-",
        "{{{",
    ] {
        let mut spans = Vec::new();
        ExtendedJson.highlight_line(line, &mut 0, &mut spans);
        for span in &spans {
            assert!(
                span.start < span.end && span.end <= line.len(),
                "{line}: {span:?}"
            );
            assert!(line.is_char_boundary(span.start) && line.is_char_boundary(span.end));
        }
    }
    assert_eq!(
        kinds(&ExtendedJson, "ObjectId(\"65f"),
        [("\"65f".to_owned(), TokenKind::String)],
        "an unclosed call is no value; its argument is still a string"
    );
}

#[test]
fn editor_snapshot() {
    let mut editor = Editor::new(BODY);
    testing::snapshot_themes_after(
        "code_editor",
        common::IMG,
        |ui| {
            ui.add(
                EditorComponent::new(&mut editor)
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
fn viewer_exposes_a_document_with_its_text() {
    let mut viewer = Editor::new("let x = 1;");
    let mut harness = testing::harness(Theme::dark(), |ui| {
        EditorComponent::new(&mut viewer)
            .label("Script")
            .viewer(true)
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
        (Editor::new(&text), Report::default()),
        |ui, (editor, Report(jump, report))| {
            let r = EditorComponent::new(editor)
                .label("Big")
                .highlighter(&Json)
                .viewer(true)
                .line_numbers(true)
                .scroll_to_line(jump.take())
                .height(300.0)
                .show(ui);
            *report = (r.line_count, r.visible_lines, r.laid_out_rows);
        },
    );
    harness.run_ok();
    harness.state_mut().1.0 = Some(150_000);
    harness.run_ok();
    harness.run_ok();
    let report = harness.state().1.1.clone();
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
    let mut harness = testing::harness_state(
        Theme::dark(),
        (Editor::new(BODY), None),
        |ui, (editor, copied)| {
            EditorComponent::new(editor)
                .label("Body")
                .viewer(true)
                .height(200.0)
                .show(ui);
            record_copy(ui.ctx(), copied);
        },
    );
    harness.get_by_label("Body").focus();
    harness.run_ok();
    ctrl(&harness, Key::A);
    harness.run_ok();
    ctrl(&harness, Key::C);
    harness.run_ok();
    assert_eq!(harness.state().1.as_deref(), Some(BODY));
}

#[test]
fn dragging_selects_text() {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (Editor::new("hello world\nsecond"), None),
        |ui, (editor, copied)| {
            EditorComponent::new(editor)
                .label("Body")
                .viewer(true)
                .height(80.0)
                .show(ui);
            record_copy(ui.ctx(), copied);
        },
    );
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
    assert_eq!(harness.state().1.as_deref(), Some("hello"));
}

#[test]
fn find_highlights_counts_and_closes() {
    let mut matches = 0;
    let mut viewer = Editor::new(BODY);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        matches = EditorComponent::new(&mut viewer)
            .label("Body")
            .highlighter(&Json)
            .viewer(true)
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
    let mut viewer = Editor::new("Foo foo\nFOO bar");
    let mut harness = testing::harness(Theme::dark(), |ui| {
        matches = EditorComponent::new(&mut viewer)
            .label("Body")
            .viewer(true)
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
    let mut viewer = Editor::new(BODY);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let r = EditorComponent::new(&mut viewer)
            .label("Body")
            .highlighter(&Json)
            .viewer(true)
            .line_numbers(true)
            .bracket_folding(true)
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
    let mut viewer = Editor::new(BODY);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        EditorComponent::new(&mut viewer)
            .label("Body")
            .viewer(true)
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
    let mut viewer = Editor::new(&"x".repeat(400));
    let mut rows = 0;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        rows = EditorComponent::new(&mut viewer)
            .label("Body")
            .viewer(true)
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
    use archipel_kit::editor::{
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

fn editor_harness<'a>(
    text: &str,
    configure: impl Fn(&mut Editor) + 'a,
) -> testing::Harness<'a, Editor> {
    let mut editor = Editor::new(text);
    configure(&mut editor);
    testing::harness_state(Theme::dark(), editor, |ui, editor| {
        ui.add(EditorComponent::new(editor).label("Body").height(100.0));
    })
}

#[test]
fn typing_updates_the_document() {
    let mut harness = editor_harness("", |_| {});
    let editor = harness.get_by_label("Body");
    assert_eq!(editor.role(), Role::TextInput);
    editor.focus();
    editor.type_text("let a");
    harness.run_ok();
    assert_eq!(harness.state().document.text(), "let a");
}

#[test]
fn tab_inserts_spaces_and_enter_keeps_the_indent() {
    let mut harness = editor_harness("", |e| {
        e.settings.tab_size = 2;
        e.settings.indent_size = 2;
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
    assert_eq!(harness.state().document.text(), "  a\n  b");
    assert!(
        harness.get_by_label("Body").accesskit_node().is_focused(),
        "Tab stays in the editor"
    );
}

#[test]
fn brackets_pair_and_the_closer_is_skipped() {
    let mut harness = editor_harness("", |_| {});
    harness.get_by_label("Body").focus();
    harness.run_ok();
    harness.get_by_label("Body").type_text("{");
    harness.run_ok();
    assert_eq!(harness.state().document.text(), "{}");
    harness.key_press(Key::Enter);
    harness.run_ok();
    assert_eq!(harness.state().document.text(), "{\n    \n}");
    harness.get_by_label("Body").type_text("x");
    harness.run_ok();
    assert_eq!(harness.state().document.text(), "{\n    x\n}");
}

#[test]
fn bracket_pairing_can_be_turned_off() {
    let mut harness = editor_harness("", |e| {
        e.settings.auto_pair_brackets = false;
        e.settings.auto_pair_quotes = false;
    });
    harness.get_by_label("Body").focus();
    harness.run_ok();
    harness.get_by_label("Body").type_text("(");
    harness.run_ok();
    assert_eq!(harness.state().document.text(), "(");
}

#[test]
fn editor_find_opens_and_closes() {
    let mut harness =
        testing::harness_state(Theme::dark(), Editor::new("one two one"), |ui, editor| {
            ui.add(
                EditorComponent::new(editor)
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
    assert_eq!(
        harness.state().document.text(),
        "one two one",
        "find doesn't edit the text"
    );
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Close find").click();
    harness.run_ok();
    assert!(harness.query_by_label("Next match").is_none());
}

#[test]
fn big_editor_text_stays_responsive() {
    let text = big_json(25_000);
    assert!(text.len() > 900_000, "{} bytes", text.len());
    let mut harness = testing::harness_state(Theme::dark(), Editor::new(&text), |ui, editor| {
        ui.add(
            EditorComponent::new(editor)
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

#[test]
fn the_component_reports_its_selection_as_a_byte_range() {
    let mut harness = testing::harness_state(
        Theme::dark(),
        (Editor::new("db.café.find()\nnext"), None),
        |ui, (editor, selection): &mut (Editor, Option<std::ops::Range<usize>>)| {
            *selection = EditorComponent::new(editor)
                .label("Script")
                .height(80.0)
                .show(ui)
                .selection;
        },
    );
    harness.get_by_label("Script").focus();
    harness.run();
    assert_eq!(harness.state().1, None, "a caret is no selection");
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::Home);
    harness.run();
    // Select "db.café" (7 chars, 8 bytes) with Shift+Right.
    for _ in 0..7 {
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    }
    harness.run();
    let (editor, selection) = harness.state();
    let range = selection.clone().expect("a selection");
    assert_eq!(&editor.document.text()[range], "db.café");
}

#[test]
fn a_console_view_follows_the_end_as_it_grows() {
    let log = |n: usize| {
        (0..n)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut harness = testing::harness_state(
        Theme::dark(),
        (Editor::new(&log(100)), 0..0),
        move |ui, (editor, visible): &mut (Editor, std::ops::Range<usize>)| {
            *visible = EditorComponent::new(editor)
                .label("Output")
                .viewer(true)
                .height(120.0)
                .stick_to_bottom(true)
                .show(ui)
                .visible_lines;
        },
    );
    harness.run();
    assert_eq!(harness.state().1.end, 100, "starts at the end");
    harness.state_mut().0.set_text(&log(150));
    harness.run();
    harness.run();
    assert_eq!(harness.state().1.end, 150, "follows new lines");
}
