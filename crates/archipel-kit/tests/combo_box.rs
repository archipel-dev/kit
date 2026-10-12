//! `TextField::suggestions`: an editable combo box.

use archipel_kit::Theme;
use archipel_kit::components::TextField;
use archipel_kit::egui::Key;
use archipel_kit::testing::{self, Queryable};

fn fields() -> Vec<String> {
    ["address.city", "email", "name", "tier"]
        .map(String::from)
        .to_vec()
}

fn harness(text: &str) -> testing::Harness<'static, String> {
    let fields = fields();
    testing::harness_state_sized(
        Theme::dark(),
        archipel_kit::egui::vec2(320.0, 240.0),
        text.to_owned(),
        move |ui, field| {
            ui.add(TextField::new(field).name("Field").suggestions(&fields));
        },
    )
}

#[test]
fn the_chevron_lists_everything_and_a_click_picks() {
    let mut h = harness("");
    assert!(h.query_by_label("email").is_none(), "closed at first");
    h.get_by_label("Show suggestions").click();
    h.run();
    h.run();
    for field in fields() {
        assert!(h.query_by_label(&field).is_some(), "{field} is listed");
    }
    h.get_by_label("tier").click();
    h.run();
    h.run();
    assert_eq!(h.state(), "tier");
    assert!(h.query_by_label("email").is_none(), "closed after the pick");
}

#[test]
fn typing_filters_ignoring_case_and_enter_picks() {
    let mut h = harness("");
    h.get_by_label("Field").focus();
    h.get_by_label("Field").type_text("AM");
    h.run();
    assert!(h.query_by_label("name").is_some());
    assert!(h.query_by_label("email").is_none(), "filtered out");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state(), "name", "replaces the whole text");
}

#[test]
fn free_text_stays_and_escape_closes() {
    let mut h = harness("");
    h.get_by_label("Field").focus();
    h.get_by_label("Field").type_text("e");
    h.run();
    assert!(h.query_by_label("email").is_some());
    h.key_press(Key::Escape);
    h.run();
    assert!(h.query_by_label("email").is_none());
    h.get_by_label("Field").type_text("xtra");
    h.run();
    assert_eq!(h.state(), "extra", "not a suggestion, still kept");
}

#[test]
fn alt_down_opens_the_full_list_on_a_chosen_value() {
    let mut h = harness("");
    h.get_by_label("Field").focus();
    h.get_by_label("Field").type_text("tier");
    h.run();
    assert!(
        h.query_by_label("email").is_none(),
        "a full match lists nothing"
    );
    h.key_press_modifiers(archipel_kit::egui::Modifiers::ALT, Key::ArrowDown);
    h.run();
    h.run();
    assert!(h.query_by_label("email").is_some(), "everything on request");
}

#[test]
fn a_second_click_on_the_chevron_closes() {
    let mut h = harness("");
    h.get_by_label("Show suggestions").click();
    h.run();
    h.run();
    assert!(h.query_by_label("email").is_some());
    h.get_by_label("Show suggestions").click();
    h.run();
    h.run();
    assert!(h.query_by_label("email").is_none());
}

#[test]
fn snapshot() {
    const IMG: &str = "../../docs/components/img";
    testing::snapshot_themes_after(
        "text_field_suggestions",
        IMG,
        |ui| {
            ui.set_width(240.0);
            let id = archipel_kit::egui::Id::new("combo-snapshot");
            let mut field = ui.data_mut(|d| d.get_temp_mut_or_default::<String>(id).clone());
            let fields = fields();
            ui.add(
                TextField::new(&mut field)
                    .name("Field")
                    .suggestions(&fields),
            );
            ui.data_mut(|d| d.insert_temp(id, field));
            ui.add_space(120.0);
        },
        |h| {
            h.get_by_label("Field").focus();
            h.get_by_label("Field").type_text("e");
            h.run();
        },
    );
}
