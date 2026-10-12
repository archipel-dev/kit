//! Components never panic when a layout runs out of room: negative widths, tiny areas.

use archipel_kit::components::{
    Button, Checkbox, CodeEditor, Dropdown, Form, KeyValueEntry, KeyValueTable, TextField,
};
use archipel_kit::testing::{self};
use archipel_kit::{Theme, egui};

#[test]
fn negative_widths_and_tiny_areas_do_not_panic() {
    let mut h = testing::harness_state_sized(
        Theme::dark(),
        egui::vec2(40.0, 40.0),
        (
            String::from("text"),
            "GET",
            vec![KeyValueEntry::new("a", "1")],
            true,
        ),
        |ui, (text, method, rows, on)| {
            ui.add(TextField::new(text).width(-25.0));
            ui.add(Dropdown::new("m", method, ["GET", "POST"]).width(-25.0));
            ui.add(Button::primary("Send").min_width(-10.0));
            KeyValueTable::new("kv", rows).show(ui);
            Form::new().show(ui, |form| {
                form.add(Checkbox::new(on, "A long label").comment("And a comment that wraps"));
                form.row("Label:", |ui| ui.add(TextField::new(text)));
            });
            ui.add(CodeEditor::new(text).height(-5.0));
        },
    );
    h.run();
}
