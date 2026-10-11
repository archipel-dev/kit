//! Which mouse cursor shows where.
//!
//! In egui the last cursor set during a frame wins. Children are drawn after their parent,
//! so **the innermost widget decides**: every kit component sets its own cursor on hover
//! (see the table in `docs/cursors.md`). A parent that must decide for everything inside it
//! (a busy panel, a drag in progress) wraps its content in [`override_in`], which sets its
//! cursor after the children have drawn, so it wins over them. A parent that only wants a
//! default for its empty space (a canvas, a drag handle area) uses [`fallback_in`]: its
//! cursor is set first, so every child that sets one wins.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use archipel_kit::{components::Button, cursor, egui::CursorIcon};
//!
//! let loading = true;
//! cursor::override_in(ui, loading.then_some(CursorIcon::Progress), |ui| {
//!     ui.add(Button::primary("Run"));
//! });
//! # });
//! ```

use egui::{CursorIcon, InnerResponse, Response, Ui};

/// Sets `icon` while `response` is hovered. What kit components call; use it in your own
/// widgets so nested overrides keep working.
pub fn on_hover(response: &Response, icon: CursorIcon) {
    if response.hovered() {
        response.ctx.set_cursor_icon(icon);
    }
}

/// Runs `add` and, when `icon` is `Some` and the pointer is over the area it used, sets
/// `icon` afterwards so it wins over every cursor set inside. `None` leaves the children's
/// cursors alone, so the override can be switched by state.
pub fn override_in<R>(
    ui: &mut Ui,
    icon: Option<CursorIcon>,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let inner = ui.scope(add);
    if let Some(icon) = icon
        && inner.response.contains_pointer()
    {
        ui.ctx().set_cursor_icon(icon);
    }
    inner
}

/// Runs `add` with `icon` as the cursor over the area it covers, unless a child sets its own.
/// The area is the one `add` used last frame (immediate mode learns sizes by drawing), so
/// the fallback starts on the second frame.
pub fn fallback_in<R>(
    ui: &mut Ui,
    icon: CursorIcon,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let id = ui.next_auto_id().with("archipel-cursor-fallback");
    let last: Option<egui::Rect> = ui.ctx().data(|d| d.get_temp(id));
    if let Some(rect) = last
        && ui.rect_contains_pointer(rect)
    {
        ui.ctx().set_cursor_icon(icon);
    }
    let inner = ui.scope(add);
    let rect = inner.response.rect;
    ui.ctx().data_mut(|d| d.insert_temp(id, rect));
    inner
}

/// The cursor set so far this frame (for tests).
pub fn current(ctx: &egui::Context) -> CursorIcon {
    ctx.output(|o| o.cursor_icon)
}
