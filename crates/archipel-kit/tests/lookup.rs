//! The completion popup (IntelliJ `Lookup`): triggering, keys, mouse, matching, sorting,
//! documentation, multi-caret and layout. One test per rule of `docs/components/code.md`.

use archipel_kit::Theme;
use archipel_kit::editor::model::Editor;
use archipel_kit::editor::{EditorComponent, EditorComponentResponse};
use archipel_kit::egui::{Key, Modifiers};
use archipel_kit::testing::{self, Queryable};
use archipel_kit::ui::{Completer, Completion, Completions, LookupStats, MatchKind, PrefixMatcher};
use std::cell::{Cell, RefCell};

/// Everything the completer knows; the list narrows it by the identifier before the caret.
struct Words {
    items: Vec<Completion>,
    /// Extra items of a second Ctrl+Space.
    more: Vec<Completion>,
    delay: f32,
    docs: bool,
    recent: Vec<(&'static str, u64)>,
    advert: Option<&'static str>,
}

impl Words {
    fn new(words: &[&str]) -> Self {
        Self {
            items: words.iter().map(|w| Completion::new(*w, *w)).collect(),
            more: Vec::new(),
            delay: 0.0,
            docs: false,
            recent: Vec::new(),
            advert: None,
        }
    }

    fn at(&self, text: &str, caret: usize, items: &[Completion]) -> Option<Completions> {
        let before: Vec<char> = text.chars().take(caret).collect();
        let run = before
            .iter()
            .rev()
            .take_while(|c| c.is_alphanumeric() || **c == '_')
            .count();
        Some(Completions {
            start: caret - run,
            end: caret,
            items: items.to_vec(),
        })
    }
}

impl Completer for Words {
    fn complete(&self, text: &str, caret: usize) -> Option<Completions> {
        self.at(text, caret, &self.items)
    }

    fn complete_extended(&self, text: &str, caret: usize) -> Option<Completions> {
        let mut all = self.items.clone();
        all.extend(self.more.clone());
        self.at(text, caret, &all)
    }

    fn recency(&self, lookup: &str) -> u64 {
        self.recent
            .iter()
            .find(|(w, _)| *w == lookup)
            .map_or(0, |(_, r)| *r)
    }

    fn auto_popup_delay(&self) -> f32 {
        self.delay
    }

    fn auto_show_documentation(&self) -> bool {
        self.docs
    }

    fn advertisement(&self) -> Option<String> {
        self.advert.map(str::to_owned)
    }
}

type H<'a> = testing::Harness<'a, Editor>;

fn editor_with<'a>(completer: &'a Words, text: &str) -> H<'a> {
    let mut h = testing::harness_state_sized(
        Theme::dark(),
        archipel_kit::egui::vec2(480.0, 420.0),
        Editor::new(text),
        move |ui, editor| {
            ui.add(
                EditorComponent::new(editor)
                    .label("Code")
                    .height(150.0)
                    .completer(completer),
            );
        },
    );
    h.get_by_label("Code").focus();
    h.run();
    h.key_press_modifiers(Modifiers::CTRL, Key::End);
    h.run();
    h
}

fn text(h: &H<'_>) -> String {
    h.state().document.text().to_owned()
}

fn listed(h: &H<'_>, label: &str) -> bool {
    h.query_by_label(label).is_some()
}

fn type_text(h: &mut H<'_>, s: &str) {
    h.get_by_label("Code").type_text(s);
    h.run();
}

fn press(h: &mut H<'_>, key: Key) {
    h.key_press(key);
    h.run();
}

fn words() -> Words {
    Words::new(&["alpha", "alphabet", "alpine", "beta"])
}

#[test]
fn auto_popup_opens_on_an_identifier_character_only() {
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "(");
    assert!(!listed(&h, "alpha"), "`(` does not open the list");
    type_text(&mut h, "al");
    assert!(listed(&h, "alpha"), "a letter does");
    assert!(!listed(&h, "beta"), "narrowed by the prefix");
}

#[test]
fn auto_popup_waits_for_the_completers_delay() {
    let mut w = words();
    w.delay = 0.3;
    let mut h = editor_with(&w, "");
    type_text(&mut h, "a");
    assert!(!listed(&h, "alpha"), "not yet");
    h.run_steps(30);
    assert!(listed(&h, "alpha"), "after the delay");
}

#[test]
fn ctrl_space_opens_at_any_position_and_a_second_press_widens() {
    let mut w = words();
    w.more = vec![Completion::new("zeta", "zeta")];
    let mut h = editor_with(&w, "x = ");
    h.key_press_modifiers(Modifiers::CTRL, Key::Space);
    h.run();
    assert!(listed(&h, "alpha") && listed(&h, "beta"));
    assert!(!listed(&h, "zeta"));
    h.key_press_modifiers(Modifiers::CTRL, Key::Space);
    h.run();
    assert!(listed(&h, "zeta"), "the second invocation shows more");
}

#[test]
fn ctrl_space_with_one_item_inserts_it_at_once() {
    let w = Words::new(&["only_one"]);
    let mut h = editor_with(&w, "on");
    h.key_press_modifiers(Modifiers::CTRL, Key::Space);
    h.run();
    assert_eq!(text(&h), "only_one");
    assert!(!listed(&h, "only_one"), "no list");
}

#[test]
fn down_and_up_wrap_around() {
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    press(&mut h, Key::ArrowUp);
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "alpine", "Up from the first row goes to the last");

    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    for _ in 0..3 {
        press(&mut h, Key::ArrowDown);
    }
    press(&mut h, Key::Enter);
    assert_eq!(
        text(&h),
        "alpha",
        "Down from the last row goes to the first"
    );
}

#[test]
fn page_down_and_page_up_move_by_a_page() {
    let names: Vec<String> = (0..30).map(|i| format!("item{i:02}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let w = Words::new(&refs);
    let mut h = editor_with(&w, "");
    type_text(&mut h, "item");
    press(&mut h, Key::PageDown);
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "item10");

    let mut h = editor_with(&w, "");
    type_text(&mut h, "item");
    for _ in 0..4 {
        press(&mut h, Key::PageDown);
    }
    assert!(listed(&h, "item29"), "scrolled to the end");
    press(&mut h, Key::PageUp);
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "item19");
}

#[test]
fn at_most_eleven_rows_show() {
    let names: Vec<String> = (0..30).map(|i| format!("item{i:02}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let w = Words::new(&refs);
    let mut h = editor_with(&w, "");
    type_text(&mut h, "item");
    let rows = h.query_all_by_label_contains("item").count();
    assert_eq!(rows, 11);
}

#[test]
fn enter_replaces_the_prefix_and_keeps_the_text_after_the_caret() {
    let w = words();
    let mut h = editor_with(&w, "alpxyz");
    {
        let doc = h.state().document.clone();
        h.state_mut().carets.set_caret(&doc, 3);
    }
    h.run();
    h.key_press_modifiers(Modifiers::CTRL, Key::Space);
    h.run();
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "alphaxyz");
}

#[test]
fn tab_replaces_the_identifier_tail() {
    let w = words();
    let mut h = editor_with(&w, "alpxyz");
    {
        let doc = h.state().document.clone();
        h.state_mut().carets.set_caret(&doc, 3);
    }
    h.run();
    h.key_press_modifiers(Modifiers::CTRL, Key::Space);
    h.run();
    press(&mut h, Key::Tab);
    assert_eq!(
        text(&h),
        "alpha",
        "the typed rest of the word is overwritten"
    );
}

#[test]
fn escape_closes_the_list_and_keeps_the_focus() {
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    press(&mut h, Key::Escape);
    assert!(!listed(&h, "alpha"));
    type_text(&mut h, "x");
    assert_eq!(text(&h), "alx", "the editor still has the focus");
}

#[test]
fn context_characters_insert_the_item_and_are_typed() {
    for c in ['.', ',', ';', '=', ' ', ':'] {
        let w = words();
        let mut h = editor_with(&w, "");
        type_text(&mut h, "alph");
        type_text(&mut h, &c.to_string());
        assert_eq!(text(&h), format!("alpha{c}"), "`{c}` commits the selection");
        assert!(!listed(&h, "alphabet"), "closed after `{c}`");
    }
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "alph");
    type_text(&mut h, "(");
    assert!(text(&h).starts_with("alpha("), "got {:?}", text(&h));
}

#[test]
fn ctrl_period_inserts_followed_by_a_dot() {
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "be");
    h.key_press_modifiers(Modifiers::CTRL, Key::Period);
    h.run();
    assert_eq!(text(&h), "beta.");
}

#[test]
fn any_other_character_closes_the_list() {
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    type_text(&mut h, ")");
    assert!(!listed(&h, "alpha"));
    assert_eq!(text(&h), "al)", "the character is typed, nothing inserted");
}

#[test]
fn backspace_widens_the_list_and_closes_it_at_the_start_of_the_prefix() {
    let w = Words::new(&["abc", "axe", "bat"]);
    let mut h = editor_with(&w, "");
    type_text(&mut h, "ab");
    assert!(listed(&h, "abc") && !listed(&h, "axe"));
    press(&mut h, Key::Backspace);
    assert!(listed(&h, "abc") && listed(&h, "axe"), "wider with `a`");
    press(&mut h, Key::Backspace);
    assert!(!listed(&h, "abc"), "closed at the start of the prefix");
}

#[test]
fn caret_keys_leaving_the_prefix_close_the_list() {
    let w = words();
    let mut h = editor_with(&w, "x ");
    type_text(&mut h, "al");
    press(&mut h, Key::Home);
    assert!(!listed(&h, "alpha"));
}

#[test]
fn click_selects_and_double_click_inserts() {
    let w = words();
    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    h.get_by_label("alphabet").click();
    h.run();
    assert_eq!(text(&h), "al", "a single click does not insert");
    assert!(listed(&h, "alphabet"), "the list stays");
    // The click selected the row: Enter inserts it.
    h.run_steps(40);
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "alphabet");

    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    h.get_by_label("alpine").click();
    h.run();
    h.get_by_label("alpine").click();
    h.run();
    h.run();
    assert_eq!(text(&h), "alpine", "a double click inserts");
}

#[test]
fn matching_prefers_prefix_then_camel_hump_then_substring() {
    let m = PrefixMatcher::new("se");
    assert_eq!(
        m.matches("setValue").expect("matches").kind,
        MatchKind::Prefix
    );
    assert_eq!(
        m.matches("getSetValue").expect("matches").kind,
        MatchKind::Word
    );
    assert_eq!(
        m.matches("resent").expect("matches").kind,
        MatchKind::Substring
    );
    assert_eq!(
        PrefixMatcher::new("gSV")
            .matches("getSetValue")
            .expect("matches")
            .kind,
        MatchKind::CamelHump
    );
}

fn first_choice(items: &[&str], typed: &str, recent: Vec<(&'static str, u64)>) -> String {
    let mut w = Words::new(items);
    w.recent = recent;
    let mut h = editor_with(&w, "");
    type_text(&mut h, typed);
    press(&mut h, Key::Enter);
    text(&h)
}

#[test]
fn the_list_sorts_by_match_kind() {
    let items = ["resent", "getSetValue", "setValue"];
    assert_eq!(first_choice(&items, "se", vec![]), "setValue");
    // The second row is the hump match.
    let w = Words::new(&items);
    let mut h = editor_with(&w, "");
    type_text(&mut h, "se");
    press(&mut h, Key::ArrowDown);
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "getSetValue");
}

#[test]
fn exact_case_sorts_first() {
    assert_eq!(first_choice(&["alpha", "Alpha"], "Al", vec![]), "Alpha");
    assert_eq!(first_choice(&["Alpha", "alpha"], "al", vec![]), "alpha");
}

#[test]
fn recently_chosen_items_sort_first_and_ties_keep_the_completers_order() {
    let items = ["alpha", "alphabet", "alpine"];
    assert_eq!(first_choice(&items, "al", vec![]), "alpha");
    assert_eq!(first_choice(&items, "al", vec![("alpine", 3)]), "alpine");
    assert_eq!(
        first_choice(&items, "al", vec![("alpine", 3), ("alphabet", 9)]),
        "alphabet"
    );
}

#[test]
fn lookup_stats_remember_the_latest_choice() {
    let mut stats = LookupStats::default();
    assert_eq!(stats.recency("a"), 0);
    stats.record("a");
    stats.record("b");
    assert!(stats.recency("b") > stats.recency("a"));
}

#[test]
fn presentation_carries_the_matched_fragments_and_texts() {
    let item = Completion::new("getString", "getString")
        .tail_text("(key)")
        .type_text("String")
        .bold()
        .strikeout();
    let p = item.presentation(Some(&PrefixMatcher::new("gS")));
    assert_eq!(p.item_text, "getString");
    assert_eq!(p.tail_text, "(key)");
    assert_eq!(p.type_text, "String");
    assert_eq!(p.fragments, vec![0..1, 3..4]);
    assert!(p.bold && p.strikeout);
}

#[test]
fn completion_inserts_at_every_caret() {
    let w = Words::new(&["user", "users"]);
    let mut h = editor_with(&w, "us\nus");
    {
        let editor = h.state_mut();
        let doc = editor.document.clone();
        editor.carets.set_caret(&doc, 2);
        editor.carets.add_caret(&doc, 5);
    }
    h.run();
    h.key_press_modifiers(Modifiers::CTRL, Key::Space);
    h.run();
    press(&mut h, Key::Enter);
    assert_eq!(text(&h), "user\nuser");
    let offsets: Vec<usize> = h.state().carets.carets().iter().map(|c| c.offset).collect();
    assert_eq!(offsets, vec![4, 9]);
    // One undo step.
    h.key_press_modifiers(Modifiers::CTRL, Key::Z);
    h.run();
    assert_eq!(text(&h), "us\nus");
}

#[test]
fn a_template_caret_offset_is_kept() {
    let mut w = Words::new(&[]);
    w.items = vec![Completion::new("user", "user(id: )").caret(9)];
    w.items.push(Completion::new("users", "users"));
    let mut h = editor_with(&w, "");
    type_text(&mut h, "use");
    press(&mut h, Key::Enter);
    type_text(&mut h, "7");
    assert_eq!(text(&h), "user(id: 7)");
}

#[test]
fn ctrl_q_toggles_the_documentation_beside_the_list() {
    let mut w = words();
    w.items[0] = Completion::new("alpha", "alpha").documentation("The first letter.");
    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    assert!(h.query_by_label_contains("Documentation").is_none());
    h.key_press_modifiers(Modifiers::CTRL, Key::Q);
    h.run();
    assert!(h.query_by_label_contains("The first letter").is_some());
    h.key_press_modifiers(Modifiers::CTRL, Key::Q);
    h.run();
    assert!(h.query_by_label_contains("Documentation").is_none());
}

#[test]
fn documentation_shows_by_itself_after_500_ms_when_asked() {
    let mut w = words();
    w.docs = true;
    w.items[0] = Completion::new("alpha", "alpha").documentation("The first letter.");
    let mut h = editor_with(&w, "");
    type_text(&mut h, "al");
    h.run_steps(10);
    assert!(
        h.query_by_label_contains("Documentation").is_none(),
        "not yet"
    );
    h.run_steps(40);
    assert!(h.query_by_label_contains("The first letter").is_some());
}

#[test]
fn the_popup_opens_above_the_caret_when_there_is_no_room_below() {
    let w = words();
    let completer: &Words = &w;
    let mut h = testing::harness_state_sized(
        Theme::dark(),
        archipel_kit::egui::vec2(480.0, 260.0),
        Editor::new(""),
        move |ui, editor| {
            ui.add_space(110.0);
            ui.add(
                EditorComponent::new(editor)
                    .label("Code")
                    .height(100.0)
                    .completer(completer),
            );
        },
    );
    h.get_by_label("Code").focus();
    h.run();
    h.get_by_label("Code").type_text("a");
    h.run();
    let editor_top = h.get_by_label("Code").rect().top();
    let row = h.get_by_label("alpha").rect();
    assert!(
        row.bottom() < editor_top + 40.0,
        "above the caret line: {row:?}"
    );
    // And below it with room.
    let mut h = editor_with(&w, "");
    type_text(&mut h, "a");
    let row = h.get_by_label("alpha").rect();
    assert!(row.top() > h.get_by_label("Code").rect().top() + 10.0);
}

#[test]
fn the_response_reports_the_completed_item() {
    let w = words();
    let seen = RefCell::new(None);
    let calls = Cell::new(0);
    let mut h = testing::harness_state_sized(
        Theme::dark(),
        archipel_kit::egui::vec2(480.0, 300.0),
        Editor::new(""),
        |ui, editor| {
            let out: EditorComponentResponse = EditorComponent::new(editor)
                .label("Code")
                .height(120.0)
                .completer(&w)
                .show(ui);
            calls.set(calls.get() + 1);
            if let Some(item) = out.completed {
                *seen.borrow_mut() = Some(item.label);
            }
        },
    );
    h.get_by_label("Code").focus();
    h.run();
    h.get_by_label("Code").type_text("be");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(seen.borrow().as_deref(), Some("beta"));
}

fn snap_editor(name: &str, words: &Words, typed: &str, toggle_docs: bool, above: bool) {
    testing::snapshot_themes_after(
        name,
        "../../docs/components/img",
        |ui| {
            ui.set_width(if toggle_docs { 840.0 } else { 420.0 });
            let id = archipel_kit::egui::Id::new(("lookup-snapshot", "state"));
            let mut editor: Editor = ui.data_mut(|d| {
                d.get_temp(id)
                    .unwrap_or_else(|| Editor::new("let value = "))
            });
            if above {
                ui.add_space(190.0);
            }
            ui.add(
                EditorComponent::new(&mut editor)
                    .label("Code")
                    .height(if above { 60.0 } else { 40.0 })
                    .completer(words),
            );
            if !above {
                ui.add_space(if toggle_docs { 210.0 } else { 190.0 });
            }
            ui.data_mut(|d| d.insert_temp(id, editor));
        },
        |h| {
            h.get_by_label("Code").focus();
            h.run();
            h.key_press_modifiers(Modifiers::CTRL, Key::End);
            h.run();
            h.get_by_label("Code").type_text(typed);
            h.run();
            if toggle_docs {
                h.key_press_modifiers(Modifiers::CTRL, Key::Q);
                h.run();
            }
        },
    );
}

fn rich() -> Words {
    use archipel_icons::SymbolKind;
    use archipel_kit::icons::Icon;
    let mut w = Words::new(&[]);
    w.items = vec![
        Completion::new("getString", "getString")
            .icon(Icon::from(SymbolKind::Method))
            .tail_text("(key: &str)")
            .type_text("String")
            .documentation("Returns the string stored under key, or an empty string."),
        Completion::new("getStringArray", "getStringArray")
            .icon(Icon::from(SymbolKind::Method))
            .tail_text("(key: &str)")
            .type_text("Vec<String>"),
        Completion::new("setString", "setString")
            .icon(Icon::from(SymbolKind::Method))
            .tail_text("(key, value)")
            .type_text("()")
            .strikeout(),
        Completion::new("string_count", "string_count")
            .icon(Icon::from(SymbolKind::Field))
            .type_text("usize"),
        Completion::new("GetStringRequest", "GetStringRequest")
            .icon(Icon::from(SymbolKind::Class))
            .type_text("struct")
            .bold(),
    ];
    w.advert = Some("Press Ctrl+Enter to insert and complete the statement");
    w
}

#[test]
fn snapshot_lookup_with_matched_fragments_and_type_text() {
    snap_editor("lookup", &rich(), "gS", false, false);
}

#[test]
fn snapshot_lookup_with_documentation() {
    snap_editor("lookup_docs", &rich(), "get", true, false);
}

#[test]
fn snapshot_lookup_above_the_caret() {
    snap_editor("lookup_above", &rich(), "get", false, true);
}
