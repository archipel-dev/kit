//! Native window integration and the shared desktop toolbar.

use super::IconButton;
use crate::{Glyph, Theme, egui, tokens};
use egui::{
    Align, Id, InnerResponse, Layout, Sense, Ui, UiBuilder, ViewportBuilder, ViewportCommand,
};

/// How an application's main toolbar meets its native window chrome.
/// Window creation remains the host application's responsibility.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowChrome {
    /// A normal native title bar above the app's toolbar, appropriate for dialog windows.
    #[default]
    Native,
    /// On macOS, a transparent native title area behind the app toolbar, retaining native
    /// traffic lights. Other platforms retain their native decorations.
    Integrated,
    /// No native title bar; the toolbar supplies minimize, maximize/restore and close.
    /// The host is responsible for border resizing on platforms that require custom hit testing.
    Borderless,
}

impl WindowChrome {
    /// Center macOS native controls in the toolbar's full vertical space. Call from the
    /// host's UI thread after creating the window and on updates (AppKit may reposition
    /// controls during resize). `title` must uniquely identify a window in this process;
    /// The context supplies the zoom factor and native backing color. Returns false when
    /// no unique native window is available. Other platforms need no adjustment.
    pub fn align_native_controls(self, title: &str, ctx: &egui::Context) -> bool {
        if self != Self::Integrated {
            return true;
        }
        #[cfg(target_os = "macos")]
        {
            use objc2::MainThreadMarker;
            use objc2_app_kit::{
                NSApplication, NSColor, NSWindowButton, NSWindowCollectionBehavior,
            };
            use objc2_foundation::NSPoint;
            let zoom = ctx.zoom_factor();
            if !zoom.is_finite() || zoom <= 0.0 {
                return false;
            }
            let Some(main_thread) = MainThreadMarker::new() else {
                return false;
            };
            let app = NSApplication::sharedApplication(main_thread);
            let windows = app.windows();
            let mut matching = windows
                .iter()
                .filter(|window| window.title().to_string() == title);
            let Some(window) = matching.next() else {
                return false;
            };
            if matching.next().is_some() {
                return false;
            }
            let [r, g, b, a] = Theme::colors(ctx).frame.to_srgba_unmultiplied();
            let background = NSColor::colorWithSRGBRed_green_blue_alpha(
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0,
                f64::from(a) / 255.0,
            );
            window.setBackgroundColor(Some(&background));
            // The green button zooms the window instead of moving it into a full-screen
            // space: there, macOS hides the traffic lights and slides in its own empty
            // title bar over the toolbar. Zoomed, the toolbar and its controls stay as they
            // are, as in Android Studio's maximized window.
            let behavior = window.collectionBehavior();
            let zooms = (behavior | NSWindowCollectionBehavior::FullScreenNone)
                & !NSWindowCollectionBehavior::FullScreenPrimary;
            if behavior != zooms {
                window.setCollectionBehavior(zooms);
            }
            let target = f64::from(tokens::size::WINDOW_TOOLBAR_HEIGHT * zoom) / 2.0;
            for kind in [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ] {
                let Some(button) = window.standardWindowButton(kind) else {
                    return false;
                };
                let frame = button.frame();
                let in_window = button.convertRect_toView(button.bounds(), None);
                let center_from_top =
                    window.frame().size.height - in_window.origin.y - in_window.size.height / 2.0;
                let delta = center_from_top - target;
                if delta.abs() > 0.01 {
                    // AppKit's native titlebar uses bottom-left coordinates.
                    button.setFrameOrigin(NSPoint::new(frame.origin.x, frame.origin.y + delta));
                }
            }
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (title, ctx);
            true
        }
    }

    /// Configure a viewport before creating it. Use the same mode on [`WindowToolbar`].
    ///
    /// ```
    /// use archipel_kit::{egui, components::WindowChrome};
    /// let viewport = WindowChrome::Integrated.viewport(
    ///     egui::ViewportBuilder::default().with_title("My application")
    /// );
    /// ```
    pub fn viewport(self, builder: ViewportBuilder) -> ViewportBuilder {
        match self {
            Self::Integrated if cfg!(target_os = "macos") => builder
                .with_decorations(true)
                .with_fullsize_content_view(true)
                .with_title_shown(false)
                .with_titlebar_shown(false)
                .with_titlebar_buttons_shown(true)
                .with_movable_by_background(false),
            Self::Borderless => builder.with_decorations(false),
            _ => builder
                .with_decorations(true)
                .with_fullsize_content_view(false)
                .with_title_shown(true)
                .with_titlebar_shown(true)
                .with_titlebar_buttons_shown(true),
        }
    }

    fn left_clearance(self) -> f32 {
        if self == Self::Integrated && cfg!(target_os = "macos") {
            tokens::size::NATIVE_WINDOW_CONTROLS
        } else {
            0.0
        }
    }
}

/// A single themed toolbar/title area. Only its empty background drags or maximizes
/// the window; buttons, text fields and other child widgets retain their usual input.
/// Native traffic lights are kept clear in integrated macOS windows.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// use archipel_kit::components::{Button, WindowChrome, WindowToolbar};
/// WindowToolbar::new("main-toolbar").chrome(WindowChrome::Integrated).show(ctx, |ui| {
///     ui.label("My application");
///     ui.add(Button::secondary("Open"));
/// });
/// # });
/// ```
#[must_use = "show the toolbar with .show(ctx, content)"]
pub struct WindowToolbar {
    id: Id,
    chrome: WindowChrome,
    project_color: Option<egui::Color32>,
}

impl WindowToolbar {
    /// A toolbar with a stable identity. Defaults to normal native chrome.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(id_salt),
            chrome: WindowChrome::Native,
            project_color: None,
        }
    }

    /// Tints the toolbar with a soft glow of the project's color behind the project widget,
    /// as Android Studio's Islands UI does (see [`crate::parts::project_glow`]).
    pub fn project_color(mut self, color: egui::Color32) -> Self {
        self.project_color = Some(color);
        self
    }

    /// Match the mode used when configuring the application's viewport.
    pub fn chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = chrome;
        self
    }

    /// Draw the toolbar above the rest of the app. `content` can use ordinary layout
    /// and kit widgets; right-aligned actions fit with `ui.with_layout(...)`.
    /// Returns the content's result and panel response.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let colors = Theme::colors(ctx);
        let dark = Theme::current(ctx).dark;
        let theme_id = self.id.with((ctx.viewport_id(), "native-theme"));
        let theme_changed = ctx.data_mut(|data| {
            let changed = data.get_temp::<bool>(theme_id) != Some(dark);
            data.insert_temp(theme_id, dark);
            changed
        });
        if theme_changed {
            ctx.send_viewport_cmd(ViewportCommand::SetTheme(if dark {
                egui::SystemTheme::Dark
            } else {
                egui::SystemTheme::Light
            }));
        }
        egui::TopBottomPanel::top(self.id)
            .show_separator_line(false)
            .exact_height(tokens::size::WINDOW_TOOLBAR_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin::symmetric(
                        tokens::toolbar::WINDOW_PADDING as i8,
                        0,
                    )),
            )
            .show(ctx, |ui| {
                if let Some(color) = self.project_color {
                    let window = ctx.input(|i| i.viewport_rect());
                    crate::parts::project_glow(ui.painter(), window, color);
                }
                let rect = ui.max_rect();
                // Register beneath children: their click/drag targets take precedence.
                let drag =
                    ui.interact(rect, self.id.with("background"), Sense::CLICK | Sense::DRAG);
                drag.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Window, true, "Window toolbar")
                });
                let controls = self.chrome == WindowChrome::Borderless;
                let control_width = if controls {
                    3.0 * tokens::size::ICON_BUTTON + 2.0 * tokens::size::TOOLBAR_GAP
                } else {
                    0.0
                };
                let left = (rect.left() + self.chrome.left_clearance()).min(rect.right());
                let right = (rect.right()
                    - control_width
                    - if controls {
                        tokens::size::TOOLBAR_GAP
                    } else {
                        0.0
                    })
                .max(left);
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(left, rect.top()),
                    egui::pos2(right, rect.bottom()),
                );
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(Layout::left_to_right(Align::Center)),
                );
                child.set_clip_rect(content_rect.intersect(ui.clip_rect()));
                child.spacing_mut().item_spacing.x = tokens::toolbar::GAP;
                let result = content(&mut child);
                if controls {
                    let control_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.right() - control_width, rect.top()),
                        rect.right_bottom(),
                    );
                    let mut buttons = ui.new_child(
                        UiBuilder::new()
                            .max_rect(control_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    buttons.spacing_mut().item_spacing.x = tokens::size::TOOLBAR_GAP;
                    if buttons
                        .add(IconButton::new(Glyph::Minimize, "Minimize window"))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    if buttons
                        .add(IconButton::new(
                            Glyph::Fullscreen,
                            if maximized {
                                "Restore window"
                            } else {
                                "Maximize window"
                            },
                        ))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    }
                    if buttons
                        .add(IconButton::new(Glyph::Close, "Close window"))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
                if self.chrome != WindowChrome::Native {
                    if drag.double_clicked() {
                        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    } else if drag.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                }
                result
            })
    }
}
