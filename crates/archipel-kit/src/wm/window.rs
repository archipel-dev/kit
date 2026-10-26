//! Native window integration and the shared desktop toolbar.

use crate::action_system::ActionButton;
use crate::{Glyph, Theme, egui, tokens};
use egui::{
    Align, Id, InnerResponse, Layout, Sense, Ui, UiBuilder, ViewportBuilder, ViewportCommand,
};

/// How an application's main toolbar meets its native window chrome.
/// Window creation remains the host application's responsibility.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdeFrameDecorator {
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

impl IdeFrameDecorator {
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

    /// Configure a viewport before creating it. Use the same mode on [`MainToolbar`].
    ///
    /// ```
    /// use archipel_kit::{egui, wm::IdeFrameDecorator};
    /// let viewport = IdeFrameDecorator::Integrated.viewport(
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
/// use archipel_kit::{ui::Button, wm::{IdeFrameDecorator, MainToolbar}};
/// MainToolbar::new("main-toolbar").chrome(IdeFrameDecorator::Integrated).show(ctx, |ui| {
///     ui.label("My application");
///     ui.add(Button::secondary("Open"));
/// });
/// # });
/// ```
#[must_use = "show the toolbar with .show(ctx, content)"]
pub struct MainToolbar<'a> {
    id: Id,
    chrome: IdeFrameDecorator,
    project_color: Option<egui::Color32>,
    menu: Option<(
        &'a dyn crate::action_system::ActionGroup,
        &'a crate::action_system::DataContext,
        crate::action_system::ActionPlace,
    )>,
}

impl<'a> MainToolbar<'a> {
    /// A toolbar with a stable identity. Defaults to normal native chrome.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(id_salt),
            chrome: IdeFrameDecorator::Native,
            project_color: None,
            menu: None,
        }
    }

    /// The context menu of the toolbar's empty background, opened with a right click
    /// (IntelliJ's "Customize Toolbar" menu). `group` is shown as an
    /// [`ActionPopupMenu`](crate::action_system::ActionPopupMenu) for the
    /// [`MAIN_TOOLBAR`](crate::action_system::ActionPlace::MAIN_TOOLBAR) place.
    pub fn context_menu(
        mut self,
        group: &'a dyn crate::action_system::ActionGroup,
        context: &'a crate::action_system::DataContext,
    ) -> Self {
        self.menu = Some((
            group,
            context,
            crate::action_system::ActionPlace::MAIN_TOOLBAR,
        ));
        self
    }

    /// Draws the toolbar in IntelliJ's three zones: `left` (main menu button, project and
    /// version control widgets), `center` (the run widget, centered in the window) and
    /// `right` (search everywhere, settings, account; right-aligned). Each closure lays its
    /// widgets out left to right.
    pub fn show_zones(
        self,
        ctx: &egui::Context,
        left: impl FnOnce(&mut Ui),
        center: impl FnOnce(&mut Ui),
        right: impl FnOnce(&mut Ui),
    ) -> InnerResponse<()> {
        let id = self.id.with("center-width");
        self.show(ctx, |ui| {
            let area = ui.max_rect();
            left(ui);
            let width: f32 = ctx.data(|d| d.get_temp(id)).unwrap_or(0.0);
            let window = ctx.content_rect();
            let center_rect = egui::Rect::from_center_size(
                egui::pos2(window.center().x, area.center().y),
                egui::vec2(width.max(1.0), area.height()),
            );
            let mut middle = ui.new_child(
                UiBuilder::new()
                    .max_rect(center_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            middle.spacing_mut().item_spacing.x = tokens::toolbar::GAP;
            center(&mut middle);
            let used = middle.min_rect().width();
            if (used - width).abs() > 0.5 {
                ctx.data_mut(|d| d.insert_temp(id, used));
                ctx.request_repaint();
            }
            // Like the center zone, the right one is as wide as it was last frame, so it
            // can end at the right edge while its widgets read left to right.
            let end_id = id.with("right");
            let end_width: f32 = ctx.data(|d| d.get_temp(end_id)).unwrap_or(0.0);
            let end_rect = egui::Rect::from_min_max(
                egui::pos2((area.right() - end_width).max(area.left()), area.top()),
                area.right_bottom(),
            );
            let mut end = ui.new_child(
                UiBuilder::new()
                    .max_rect(end_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            end.spacing_mut().item_spacing.x = tokens::toolbar::GAP;
            right(&mut end);
            let used = end.min_rect().width();
            if (used - end_width).abs() > 0.5 {
                ctx.data_mut(|d| d.insert_temp(end_id, used));
                ctx.request_repaint();
            }
        })
    }

    /// Tints the toolbar with a soft glow of the project's color behind the project widget,
    /// as Android Studio's Islands UI does (see [`crate::parts::project_glow`]).
    pub fn project_color(mut self, color: egui::Color32) -> Self {
        self.project_color = Some(color);
        self
    }

    /// Match the mode used when configuring the application's viewport.
    pub fn chrome(mut self, chrome: IdeFrameDecorator) -> Self {
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
                let drag = match self.menu {
                    Some((group, context, place)) => {
                        use crate::behaviour::Behaviour;
                        crate::behaviour::PopupHandler::for_group(group, context, place)
                            .attach(ui, drag)
                    }
                    None => drag,
                };
                let controls = self.chrome == IdeFrameDecorator::Borderless;
                let control_width = if controls {
                    3.0 * tokens::toolbar::TITLE_BUTTON
                } else {
                    0.0
                };
                let left = (rect.left() + self.chrome.left_clearance()).min(rect.right());
                // The window controls reach the window's edge, past the toolbar's padding.
                let edge = if controls {
                    tokens::toolbar::WINDOW_PADDING
                } else {
                    0.0
                };
                let right = (rect.right() + edge
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
                // With focus on one of the toolbar's controls (IntelliJ's `FocusMainToolbar`),
                // Left and Right move it to the neighbor. Text fields keep their arrows.
                if !ctx.wants_keyboard_input()
                    && let Some(focused) = ctx.memory(|m| m.focused())
                    && ctx
                        .read_response(focused)
                        .is_some_and(|r| rect.contains(r.rect.center()))
                {
                    let right = ctx
                        .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
                    let left = ctx
                        .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
                    if right {
                        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
                    } else if left {
                        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::Previous));
                    }
                }
                if controls {
                    let control_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.right() + edge - control_width, rect.top()),
                        egui::pos2(rect.right() + edge, rect.bottom()),
                    );
                    let mut buttons = ui.new_child(
                        UiBuilder::new()
                            .max_rect(control_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    buttons.spacing_mut().item_spacing.x = 0.0;
                    if buttons
                        .add(ActionButton::new(Glyph::Minimize, "Minimize window").title_pane(true))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    if buttons
                        .add(
                            ActionButton::new(
                                Glyph::Fullscreen,
                                if maximized {
                                    "Restore window"
                                } else {
                                    "Maximize window"
                                },
                            )
                            .title_pane(true),
                        )
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    }
                    if buttons
                        .add(ActionButton::new(Glyph::Close, "Close window").title_pane(true))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
                if self.chrome != IdeFrameDecorator::Native {
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
