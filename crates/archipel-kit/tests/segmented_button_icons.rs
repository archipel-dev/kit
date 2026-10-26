//! Segmented icons keep names, selection, keyboard control and disabled state.
use archipel_kit::{
    Glyph, Theme,
    testing::{self, Queryable},
    ui::SegmentedButton,
};

#[test]
fn icon_only_segments_are_named_and_arrow_keys_select() {
    let mut h = testing::harness_state(Theme::dark(), 0, |ui, selected| {
        ui.add(
            SegmentedButton::new(selected, [(0, "Pretty"), (1, "Raw"), (2, "Preview")])
                .icon(0, Glyph::Code)
                .icon(1, Glyph::File)
                .icon_only(true),
        );
    });
    h.get_by_label("Pretty").focus();
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_eq!(*h.state(), 1);
    assert!(h.get_by_label("Raw").is_focused());
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    assert_eq!(*h.state(), 0);
}

#[test]
fn snapshots() {
    testing::snapshot_themes(
        "segmented_button_icons",
        "../../docs/components/img",
        |ui| {
            let mut selected = 0;
            ui.add(
                SegmentedButton::new(&mut selected, [(0, "Pretty"), (1, "Raw"), (2, "Preview")])
                    .icon(0, Glyph::Code)
                    .icon(1, Glyph::File),
            );
            ui.add(
                SegmentedButton::new(&mut selected, [(0, "Pretty"), (1, "Raw"), (2, "Preview")])
                    .icon(0, Glyph::Code)
                    .icon(1, Glyph::File)
                    .icon_only(true),
            );
            ui.add(
                SegmentedButton::new(&mut selected, [(0, "Pretty"), (1, "Raw"), (2, "Preview")])
                    .icon(0, Glyph::Code)
                    .icon(1, Glyph::File)
                    .icon_only(true)
                    .enabled(false),
            );
        },
    );
}
