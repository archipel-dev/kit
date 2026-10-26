# Window toolbar

`IdeFrameDecorator` configures the host viewport; `MainToolbar` draws the matching themed
toolbar. On macOS, `Integrated` keeps the native traffic lights alongside your content,
without an extra native title strip. Empty toolbar space drags the window; double-click
maximizes or restores it. Child controls retain their own input.

```rust
use archipel_kit::{egui, ui::Button, wm::{IdeFrameDecorator, MainToolbar}};
let viewport = IdeFrameDecorator::Integrated.viewport(
    egui::ViewportBuilder::default().with_title("My application")
);
# egui::__run_test_ctx(|ctx| {
MainToolbar::new("main-toolbar").chrome(IdeFrameDecorator::Integrated).show(ctx, |ui| {
    ui.label("My application");
    ui.add(Button::secondary("Open"));
});
# });
```

Pass `viewport` to your host's window options before creating the window. Use the same
chrome mode when rendering its toolbar. In the host's update callback, also call
`IdeFrameDecorator::Integrated.align_native_controls("My application", ctx)`.
This centers the actual macOS traffic lights in the full toolbar height, including after
native resize relayout, and applies the theme's frame color to the native backing window.
It also makes the green button zoom the
window (fill the screen) instead of moving it into a macOS full-screen space: in a
full-screen space macOS hides the traffic lights and slides its own empty title bar over
the toolbar when the pointer reaches the top edge. Zoomed, the toolbar and the traffic
lights stay where they are. The title must
uniquely identify a window in your process. The helper returns false if the window or UI
thread is unavailable; it uses safe AppKit bindings and does not create replacement icons.
`Native` is the default and retains the normal
native title bar. `Integrated` retains native decorations on Windows and Linux;
their title-bar integration has not been verified here. `Borderless` supplies named,
keyboard-accessible minimize, maximize/restore and close buttons using the existing kit
icons. Its host must provide border resizing where custom hit testing is required.

The toolbar reserves space for macOS native controls and clips content to its available
width. Hosts should set a sensible minimum window size for their toolbar actions.

![Dark toolbar](img/main_toolbar.dark.png)
![Light toolbar](img/main_toolbar.light.png)

The snapshots show the portable `Borderless` mode. Native traffic lights belong to the
operating system and are checked in the running macOS gallery.

## Toolbar controls

Measured on Android Studio's main toolbar (`tokens::toolbar`):

- `ToolbarComboButton::new(text)` is a flat text widget like Android Studio's project and version
  control widgets: 30 pt high, radius 6, 10 pt before the text, optional 20 pt `.icon(..)` or
  `.leading(slot)` (e.g. an `Avatar` as the project tile) 8 pt in, and a chevron with
  `.dropdown(true)`. Hover and press fill it; `.open(true)` keeps it filled while its menu
  (a `JBPopup` anchored to the response) is open.
- `ActionButton::large(true)` is the 30 pt toolbar icon button with a 20 pt icon.
- The toolbar keeps 6 pt from the window's edges, so the main menu button sits right above
  the stripe buttons, and puts 10 pt between controls.
- `.project_color(color)` paints Android Studio's soft glow of the project's color behind the
  project widget (`parts::project_glow`): the color itself at 26 % in dark themes, a pastel of
  it in light ones. `ToolWindowPane::project_color` continues it over the top of the stripes.

```rust
# egui::__run_test_ctx(|ctx| {
use archipel_kit::{ui::Avatar, action_system::ActionButton, wm::{ToolbarComboButton, MainToolbar}};
use archipel_kit::{Glyph, egui, parts::Slot};

MainToolbar::new("main-toolbar")
    .project_color(egui::Color32::from_rgb(0x4C, 0xA0, 0x74))
    .show(ctx, |ui| {
        ui.add(ActionButton::new(Glyph::Menu, "Main menu").large(true));
        let _ = ToolbarComboButton::new("archipel")
            .leading(Slot::ui_width(20.0, |ui| { ui.add(Avatar::new("archipel").size(20.0)); }))
            .dropdown(true)
            .show(ui);
        let _ = ToolbarComboButton::new("main").icon(Glyph::Branch).dropdown(true).show(ui);
    });
# });
```
