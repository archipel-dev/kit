//! Loading rows preserve their names and keyboard navigation.
use archipel_kit::{
    Glyph, Theme, Tone,
    testing::{self, Queryable, Role},
    ui::{Tree, TreeNode},
};

#[test]
fn busy_parent_is_named_and_keyboard_reachable() {
    let rows = [TreeNode::new("collections", 0)
        .icon(Glyph::Folder)
        .icon_tone(Tone::Warning)
        .busy(true)
        .parent(false)];
    let mut h = testing::harness_state(Theme::dark(), (None, None), |ui, (selected, toggled)| {
        let out = Tree::new("tree", &rows, selected).show(ui);
        if out.toggled.is_some() {
            *toggled = out.toggled;
        }
    });
    h.get_by_role(Role::Tree).focus();
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(3);
    h.get_by_label("Loading collections");
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(h.state().0, Some(0));
    assert_eq!(h.state().1, Some(0));
}

#[test]
fn snapshots() {
    testing::snapshot_themes("tree_busy", "../../docs/components/img", |ui| {
        let rows = [
            TreeNode::new("Collections", 0)
                .icon(Glyph::Folder)
                .icon_tone(Tone::Warning)
                .busy(true)
                .parent(false),
            TreeNode::new("users", 1).icon(Glyph::Database),
        ];
        Tree::new("loading", &rows, &mut None).height(80.0).show(ui);
    });
}

#[test]
fn double_clicking_a_parent_keeps_tree_keyboard_focus() {
    let mut h = testing::harness_state(Theme::dark(), (None, false), |ui, (selected, open)| {
        let rows = [TreeNode::new("src", 0).parent(*open)];
        if Tree::new("project", &rows, selected).show(ui).toggled == Some(0) {
            *open = !*open;
        }
    });
    h.get_by_label("src").click();
    h.run();
    h.get_by_label("src").click();
    h.run();
    assert!(h.state().1);
    assert!(h.get_by_role(Role::Tree).is_focused());
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    assert!(!h.state().1);
}
