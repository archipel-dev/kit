//! How a focused control is activated from the keyboard (IntelliJ spec sections 13.2 and
//! 13.3): <kbd>Space</kbd> on key release, like Swing's button model, and <kbd>Enter</kbd>
//! on press for push buttons only (in a checkbox or radio <kbd>Enter</kbd> belongs to the
//! dialog's default button).

use egui::{Key, Response, Ui, response::Flags};

/// Makes `response.clicked()` follow Swing's key rules for a focused control: <kbd>Space</kbd>
/// activates when it is released (egui reports a click when it is pressed), <kbd>Enter</kbd>
/// activates on press when `enter` is true and not at all otherwise. Pointer and screen
/// reader clicks are left as they are.
pub(crate) fn key_activation(ui: &Ui, response: &mut Response, enabled: bool, enter: bool) {
    if !enabled || !response.has_focus() {
        return;
    }
    let (space_down, space_up, enter_down) = ui.input(|i| {
        (
            i.key_pressed(Key::Space),
            i.key_released(Key::Space),
            i.key_pressed(Key::Enter),
        )
    });
    if space_down || (enter_down && !enter) {
        response.flags.set(Flags::FAKE_PRIMARY_CLICKED, false);
    }
    if space_up || (enter_down && enter) {
        response.flags.set(Flags::FAKE_PRIMARY_CLICKED, true);
    }
}
