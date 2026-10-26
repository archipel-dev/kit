//! The status bar container (IntelliJ spec section 9): anchor ordering, presentations,
//! hover and press effects, click reporting, keyboard.

use archipel_kit::egui::{self, Event, Key, Modifiers, PointerButton, pos2};
use archipel_kit::testing::{Harness, NodeT, Queryable};
use archipel_kit::wm::{
    IconPresentation, MultipleTextValuesPresentation, StatusBar, StatusBarAnchor,
    StatusBarWidgetItem, TextPresentation,
};
use archipel_kit::{Glyph, Theme};

#[derive(Default)]
struct State {
    position: String,
    alignment: f32,
    encoding: usize,
    activated: Vec<String>,
    selected: Vec<(String, usize)>,
    menu: usize,
    escaped: usize,
    info: String,
}

fn harness_with(items: fn(&State) -> Vec<(bool, StatusBarWidgetItem)>) -> Harness<'static, State> {
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_state(
            move |ctx, state: &mut State| {
                if !installed {
                    archipel_kit::install(ctx, Theme::dark());
                    installed = true;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Editor");
                });
                egui::TopBottomPanel::bottom("status")
                    .exact_height(30.0)
                    .show(ctx, |ui| {
                        let mut bar = StatusBar::new("bar").info(state.info.clone());
                        for (left, item) in items(state) {
                            bar = if left {
                                bar.left_widget(item)
                            } else {
                                bar.widget(item)
                            };
                        }
                        let out = bar.show(ui);
                        state.activated.extend(out.activated);
                        state.selected.extend(out.selected_value);
                        state.menu += usize::from(out.widgets_menu.is_some());
                        state.escaped += usize::from(out.escaped);
                    });
            },
            State {
                position: "12:8".into(),
                ..State::default()
            },
        );
    harness.run();
    harness
}

fn x_of(harness: &Harness<'_, State>, label: &str) -> f32 {
    harness.get_by_label(label).rect().left()
}

fn default_items(state: &State) -> Vec<(bool, StatusBarWidgetItem)> {
    vec![
        (
            true,
            StatusBarWidgetItem::new("vcs", TextPresentation::new("main")),
        ),
        (
            false,
            StatusBarWidgetItem::new(
                "position",
                TextPresentation::new(state.position.clone())
                    .max_value("9999:99")
                    .alignment(state.alignment),
            ),
        ),
        (
            false,
            StatusBarWidgetItem::new(
                "encoding",
                MultipleTextValuesPresentation::new(
                    vec!["UTF-8".into(), "UTF-16".into(), "ISO-8859-1".into()],
                    state.encoding,
                ),
            ),
        ),
        (
            false,
            StatusBarWidgetItem::new("bell", IconPresentation::new(Glyph::Bell, "Notifications")),
        ),
    ]
}

fn press(harness: &mut Harness<'_, State>, pos: egui::Pos2, button: PointerButton, release: bool) {
    harness.hover_at(pos);
    harness.step();
    harness.event(Event::PointerButton {
        pos,
        button,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    if release {
        harness.event(Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
}

#[test]
fn anchors_order_the_widgets_of_a_panel() {
    fn items(_: &State) -> Vec<(bool, StatusBarWidgetItem)> {
        let text = |id: &str| TextPresentation::new(id.to_uppercase());
        vec![
            (false, StatusBarWidgetItem::new("c", text("c"))),
            (
                false,
                StatusBarWidgetItem::new("a", text("a")).anchor(StatusBarAnchor::First),
            ),
            (
                false,
                StatusBarWidgetItem::new("b", text("b"))
                    .anchor(StatusBarAnchor::Before("c".into())),
            ),
            (
                false,
                StatusBarWidgetItem::new("d", text("d")).anchor(StatusBarAnchor::After("c".into())),
            ),
            (
                false,
                StatusBarWidgetItem::new("e", text("e"))
                    .anchor(StatusBarAnchor::After("missing".into())),
            ),
            // Named before the widget it follows is added.
            (
                false,
                StatusBarWidgetItem::new("x", text("x"))
                    .anchor(StatusBarAnchor::Before("z".into())),
            ),
            (false, StatusBarWidgetItem::new("z", text("z"))),
        ]
    }
    let harness = harness_with(items);
    let order: Vec<&str> = ["A", "B", "C", "D", "X", "Z", "E"].into_iter().collect();
    let xs: Vec<f32> = order.iter().map(|label| x_of(&harness, label)).collect();
    assert!(
        xs.windows(2).all(|pair| pair[0] < pair[1]),
        "order {order:?} at {xs:?}"
    );
}

#[test]
fn the_left_panel_is_at_the_left_and_the_right_panel_at_the_right_edge() {
    let harness = harness_with(default_items);
    assert!(x_of(&harness, "main") < 40.0);
    let bell = harness.get_by_label("Notifications").rect();
    assert!(bell.right() > 480.0, "{bell:?}");
}

#[test]
fn an_icon_widget_is_a_button_named_by_its_tooltip() {
    let harness = harness_with(default_items);
    let bell = harness.get_by_label("Notifications");
    assert_eq!(bell.accesskit_node().role(), egui::accesskit::Role::Button);
}

#[test]
fn a_prototype_reserves_the_width_of_a_changing_text() {
    let mut harness = harness_with(default_items);
    let before = harness.get_by_label("12:8").rect();
    harness.state_mut().position = "1234:56".into();
    harness.run();
    let after = harness.get_by_label("1234:56").rect();
    assert!(
        (before.width() - after.width()).abs() < 0.5,
        "{before:?} vs {after:?}"
    );
    assert!(
        (before.left() - after.left()).abs() < 0.5,
        "the widgets beside it do not move"
    );
}

#[test]
fn the_alignment_places_the_text_in_the_reserved_width() {
    let text_x = |harness: &Harness<'_, State>, text: &str| {
        harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape.pos.x),
                _ => None,
            })
            .expect("the text is painted")
    };
    let mut harness = harness_with(default_items);
    let left = text_x(&harness, "12:8");
    harness.state_mut().alignment = 1.0;
    harness.run();
    let right = text_x(&harness, "12:8");
    assert!(right > left + 10.0, "left {left}, right {right}");
}

fn hover_fill_at(harness: &Harness<'_, State>, rect: egui::Rect) -> bool {
    let hover = Theme::dark().colors.hover;
    harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) => {
                shape.fill == hover && (shape.rect.center() - rect.center()).length() < 2.0
            }
            _ => false,
        })
}

#[test]
fn only_right_panel_widgets_have_the_hover_effect() {
    let mut harness = harness_with(default_items);
    let right = harness.get_by_label("UTF-8").rect();
    let left = harness.get_by_label("main").rect();
    harness.hover_at(right.center());
    harness.run();
    assert!(
        hover_fill_at(&harness, right),
        "a right widget fills on hover"
    );
    harness.hover_at(left.center());
    harness.run();
    assert!(!hover_fill_at(&harness, left), "a left widget does not");
    assert!(!hover_fill_at(&harness, right), "leaving clears it");
}

#[test]
fn a_press_shows_the_same_fill_as_hover() {
    let mut harness = harness_with(default_items);
    let rect = harness.get_by_label("12:8").rect();
    press(&mut harness, rect.center(), PointerButton::Primary, false);
    assert!(hover_fill_at(&harness, rect));
}

#[test]
fn a_left_press_activates_a_widget_before_the_release() {
    let mut harness = harness_with(default_items);
    let rect = harness.get_by_label("12:8").rect();
    press(&mut harness, rect.center(), PointerButton::Primary, false);
    assert_eq!(harness.state().activated, ["position"]);
    harness.event(Event::PointerButton {
        pos: rect.center(),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    assert_eq!(
        harness.state().activated,
        ["position"],
        "the release adds nothing"
    );
}

#[test]
fn a_right_click_is_not_a_click_but_asks_for_the_widgets_menu_on_the_right_panel_only() {
    let mut harness = harness_with(default_items);
    let right = harness.get_by_label("12:8").rect();
    press(&mut harness, right.center(), PointerButton::Secondary, true);
    assert_eq!(harness.state().menu, 1);
    assert!(harness.state().activated.is_empty());
    let left = harness.get_by_label("main").rect();
    press(&mut harness, left.center(), PointerButton::Secondary, true);
    assert_eq!(harness.state().menu, 1, "the left panel has no menu");
}

#[test]
fn enter_and_space_activate_a_focused_widget() {
    let mut harness = harness_with(default_items);
    harness.get_by_label("12:8").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().activated, ["position"]);
}

#[test]
fn a_widget_with_values_opens_a_list_above_it_and_reports_the_pick() {
    let mut harness = harness_with(default_items);
    let widget = harness.get_by_label("UTF-8").rect();
    press(&mut harness, widget.center(), PointerButton::Primary, true);
    harness.run();
    let item = harness.get_by_label("UTF-16").rect();
    assert!(
        item.bottom() <= widget.top(),
        "above: {item:?} vs {widget:?}"
    );
    harness.get_by_label("UTF-16").click();
    harness.run();
    assert_eq!(harness.state().selected, [("encoding".to_owned(), 1usize)]);
    assert!(
        harness.query_by_label("ISO-8859-1").is_none(),
        "the list closes"
    );
}

#[test]
fn escape_closes_the_list_of_values() {
    let mut harness = harness_with(default_items);
    let widget = harness.get_by_label("UTF-8").rect();
    press(&mut harness, widget.center(), PointerButton::Primary, true);
    harness.run();
    assert!(harness.query_by_label("ISO-8859-1").is_some());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("ISO-8859-1").is_none());
}

#[test]
fn arrow_keys_move_the_focus_between_widgets_and_escape_leaves_the_bar() {
    let mut harness = harness_with(default_items);
    harness.get_by_label("main").focus();
    harness.run();
    for label in ["12:8", "UTF-8", "Notifications"] {
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(
            harness.get_by_label(label).accesskit_node().is_focused(),
            "{label} has the focus"
        );
    }
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(
        harness
            .get_by_label("Notifications")
            .accesskit_node()
            .is_focused(),
        "no wrapping at the end"
    );
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(harness.get_by_label("UTF-8").accesskit_node().is_focused());
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().escaped, 1);
    assert!(!harness.get_by_label("UTF-8").accesskit_node().is_focused());
}

#[test]
fn the_info_text_sits_between_the_panels() {
    let mut harness = harness_with(default_items);
    harness.state_mut().info = "Indexing finished".into();
    harness.run();
    let info = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == "Indexing finished" => {
                Some(shape.pos.x)
            }
            _ => None,
        })
        .expect("the info text is painted");
    let main = harness.get_by_label("main").rect();
    let position = harness.get_by_label("12:8").rect();
    assert!(info > main.right() && info < position.left());
}

#[test]
fn a_disabled_widget_ignores_the_press() {
    fn items(_: &State) -> Vec<(bool, StatusBarWidgetItem)> {
        vec![(
            false,
            StatusBarWidgetItem::new("lock", TextPresentation::new("Read-only")).enabled(false),
        )]
    }
    let mut harness = harness_with(items);
    let rect = harness.get_by_label("Read-only").rect();
    press(
        &mut harness,
        pos2(rect.center().x, rect.center().y),
        PointerButton::Primary,
        true,
    );
    assert!(harness.state().activated.is_empty());
}
