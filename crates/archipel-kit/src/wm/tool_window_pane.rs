//! A shared docked islands layout, independent of application content and policy.

use crate::Icon;
use crate::action_system::{ActionButton, ActionGroup, DataContext};
use crate::behaviour::WidgetExt;
use crate::ide::HelpTooltip;
use crate::openapi::ui::{Side, Splitter};
use crate::wm::{
    ContentManager, IdeFrameDecorator, MainToolbar, SimpleToolWindowPanel, SquareStripeButton,
    ToolWindow, ToolWindowEvent, ToolWindowToolbar,
};
use crate::{Glyph, Theme, egui, tokens};
use egui::{Id, Response, Ui};
use std::cell::{Cell, RefCell};

/// A docked tool-window region that can occupy the entire work area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ToolWindowAnchor {
    /// The left tool window.
    Left,
    /// The right tool window.
    Right,
    /// The bottom tool window.
    Bottom,
}

/// Where [`ToolWindowPane::show_with`] asks for content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToolWindowPlace {
    /// The center island.
    Center,
    /// A tool window declared with [`ToolWindowPane::tool`].
    Tool(ToolWindowAnchor),
    /// The header buttons of that tool window ([`ActionButton`]s, laid out right to left
    /// before its Maximize and Hide buttons; they show while the pointer is over it).
    ToolActions(ToolWindowAnchor),
    /// The window toolbar declared with [`ToolWindowPane::with_toolbar`].
    Toolbar,
    /// The status bar declared with [`ToolWindowPane::with_status`].
    Status,
}

/// Caller-owned layout state. Persist it in the host's settings if desired.
/// Ratios retain their preferred values when a smaller viewport clamps the layout.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ToolWindowPaneState {
    /// Left pane's share of the horizontal work area.
    pub left_ratio: f32,
    /// Center's share of the space remaining beside the right pane.
    pub center_ratio: f32,
    /// Main area's share of the vertical work area above the bottom pane.
    pub top_ratio: f32,
    /// Whether the left pane is hidden.
    pub left_hidden: bool,
    /// Whether the right pane is hidden.
    pub right_hidden: bool,
    /// Whether the bottom pane is hidden.
    pub bottom_hidden: bool,
    /// A maximized tool window, or the regular docked layout.
    pub maximized: Option<ToolWindowAnchor>,
    /// The selected content tab of the left, right and bottom tool windows (see
    /// [`ToolWindowPane::tool_tabs`]).
    pub tool_tabs: [usize; 3],
    /// What the tool windows reported since the application last called
    /// [`take_events`](Self::take_events). Not part of the saved layout.
    #[serde(skip)]
    pub events: Vec<ToolWindowEvent>,
}

impl ToolWindowPaneState {
    /// The selected content tab of `region`'s tool window.
    pub fn tool_tab(&self, region: ToolWindowAnchor) -> usize {
        self.tool_tabs[region_index(region)]
    }

    /// Whether `region`'s tool window is shown (it may still be squeezed out of a small
    /// window; see [`ToolWindowPane`]).
    pub fn is_visible(&self, region: ToolWindowAnchor) -> bool {
        !*self.hidden_flag(region)
    }

    /// Shows `region`'s tool window (IntelliJ `ToolWindow.show`). Hidden windows come back
    /// from their stripe button too.
    pub fn show(&mut self, region: ToolWindowAnchor) {
        *self.hidden_flag_mut(region) = false;
    }

    /// Hides `region`'s tool window (IntelliJ `ToolWindow.hide`).
    pub fn hide(&mut self, region: ToolWindowAnchor) {
        *self.hidden_flag_mut(region) = true;
        if self.maximized == Some(region) {
            self.maximized = None;
        }
    }

    /// Hides every tool window (IntelliJ `HideAllWindows`, `Ctrl+Shift+F12`).
    pub fn hide_all(&mut self) {
        self.left_hidden = true;
        self.right_hidden = true;
        self.bottom_hidden = true;
        self.maximized = None;
    }

    /// Returns what the tool windows reported since the last call (activations, hides,
    /// `Esc` to the editor, drops on another stripe, ...) and clears it.
    pub fn take_events(&mut self) -> Vec<ToolWindowEvent> {
        std::mem::take(&mut self.events)
    }

    fn hidden_flag(&self, region: ToolWindowAnchor) -> &bool {
        match region {
            ToolWindowAnchor::Left => &self.left_hidden,
            ToolWindowAnchor::Right => &self.right_hidden,
            ToolWindowAnchor::Bottom => &self.bottom_hidden,
        }
    }

    fn hidden_flag_mut(&mut self, region: ToolWindowAnchor) -> &mut bool {
        match region {
            ToolWindowAnchor::Left => &mut self.left_hidden,
            ToolWindowAnchor::Right => &mut self.right_hidden,
            ToolWindowAnchor::Bottom => &mut self.bottom_hidden,
        }
    }
}

fn region_index(region: ToolWindowAnchor) -> usize {
    match region {
        ToolWindowAnchor::Left => 0,
        ToolWindowAnchor::Right => 1,
        ToolWindowAnchor::Bottom => 2,
    }
}

impl Default for ToolWindowPaneState {
    fn default() -> Self {
        Self {
            left_ratio: 0.22,
            center_ratio: 0.75,
            top_ratio: 0.7,
            left_hidden: false,
            right_hidden: false,
            bottom_hidden: false,
            maximized: None,
            tool_tabs: [0; 3],
            events: Vec::new(),
        }
    }
}

type Content<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// What the tool windows report while they draw, applied to the state afterwards, and the
/// tab selection they draw with.
struct Signals {
    hide: Cell<Option<ToolWindowAnchor>>,
    maximize: Cell<Option<ToolWindowAnchor>>,
    tab: Cell<Option<(ToolWindowAnchor, usize)>>,
    tabs: [usize; 3],
    /// Events the headers report, appended to the state's.
    events: RefCell<Vec<ToolWindowEvent>>,
}
struct Region<'a> {
    title: String,
    content: Option<Content<'a>>,
    actions: Option<Content<'a>>,
    tabs: Option<Vec<String>>,
    padded: bool,
    icon: Option<Icon>,
    contents: Option<&'a mut ContentManager>,
    activate_number: Option<u8>,
    title_group: Option<(&'a dyn ActionGroup, &'a DataContext)>,
    options: Option<&'a dyn ActionGroup>,
}

/// A reusable window shell: optional toolbar and status bar, a center island, and
/// optional left, right and bottom tool windows with keyboard-operable splitters.
/// The caller supplies each region's content and owns all layout state.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// use archipel_kit::wm::{ToolWindowPane, ToolWindowPaneState};
/// let mut state = ToolWindowPaneState::default();
/// ToolWindowPane::new("workspace", &mut state)
///     .left("Browser", |ui| { ui.label("Collections"); })
///     .bottom("Messages", |ui| { ui.label("Ready"); })
///     .show(ctx, |ui| { ui.label("Document"); });
/// # });
/// ```
#[must_use = "show the workbench with .show or .show_inside"]
pub struct ToolWindowPane<'a> {
    id: Id,
    state: &'a mut ToolWindowPaneState,
    left: Option<Region<'a>>,
    right: Option<Region<'a>>,
    bottom: Option<Region<'a>>,
    toolbar: Option<(IdeFrameDecorator, Option<Content<'a>>)>,
    status: Option<Option<Content<'a>>>,
    project_color: Option<egui::Color32>,
    glow_anchor: Option<egui::Rect>,
    center_padded: bool,
    frame_background: bool,
    main_menu: Option<(&'a dyn ActionGroup, &'a DataContext)>,
}

impl<'a> ToolWindowPane<'a> {
    /// Construct a shell with a stable identity and caller-owned layout state.
    pub fn new(id_salt: impl std::hash::Hash, state: &'a mut ToolWindowPaneState) -> Self {
        Self {
            id: Id::new(id_salt),
            state,
            left: None,
            right: None,
            bottom: None,
            toolbar: None,
            status: None,
            project_color: None,
            glow_anchor: None,
            center_padded: true,
            frame_background: true,
            main_menu: None,
        }
    }

    /// The main menu of the window toolbar (see [`MainToolbar::main_menu`]): a menu bar row
    /// in the Classic look, a hamburger button in New UI and Islands.
    pub fn main_menu(mut self, group: &'a dyn ActionGroup, data_context: &'a DataContext) -> Self {
        self.main_menu = Some((group, data_context));
        self
    }

    /// A titled left tool window. Its header provides Hide and Maximize/Restore actions.
    pub fn left(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.left = Some(Region::new(title, content));
        self
    }
    /// A titled right tool window.
    pub fn right(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.right = Some(Region::new(title, content));
        self
    }
    /// A titled bottom tool window below the main area.
    pub fn bottom(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom = Some(Region::new(title, content));
        self
    }
    /// Gives `region`'s body the panel content inset (default: edge to edge, for trees,
    /// lists and consoles whose rows bring their own inset). Use it for forms and text.
    /// Call it after adding the region.
    pub fn padded(mut self, region: ToolWindowAnchor, padded: bool) -> Self {
        let slot = match region {
            ToolWindowAnchor::Left => &mut self.left,
            ToolWindowAnchor::Right => &mut self.right,
            ToolWindowAnchor::Bottom => &mut self.bottom,
        };
        if let Some(slot) = slot {
            slot.padded = padded;
        }
        self
    }

    /// Gives `region` a button with `icon` on a tool window stripe beside the islands, as in
    /// Android Studio: the left stripe holds the left (top group) and bottom (bottom group)
    /// tool windows, the right stripe the right one. The button is filled while the tool
    /// window is open and opens or closes it. Call it after adding the region.
    pub fn icon(mut self, region: ToolWindowAnchor, icon: impl Into<Icon>) -> Self {
        let slot = match region {
            ToolWindowAnchor::Left => &mut self.left,
            ToolWindowAnchor::Right => &mut self.right,
            ToolWindowAnchor::Bottom => &mut self.bottom,
        };
        if let Some(slot) = slot {
            slot.icon = Some(icon.into());
        }
        self
    }

    /// Gives the center island the content inset (default `true`). Pass `false` for an
    /// editor: its tab bar sits at the island's top and its text runs to the edges.
    pub fn center_padded(mut self, padded: bool) -> Self {
        self.center_padded = padded;
        self
    }

    /// Paint the window frame behind the islands and stripes (default `true`). A plugin
    /// view embedded with [`show_inside`](Self::show_inside) in a host that already drew
    /// the frame (and its project glow) passes `false`, so the frame stays one surface and
    /// the glow is not cut off at the embedded workbench's edge.
    pub fn frame_background(mut self, paint: bool) -> Self {
        self.frame_background = paint;
        self
    }

    /// The project's color: a soft glow of it on the frame behind the toolbar's project
    /// widget and over the top of the stripes, as in Android Studio's Islands UI.
    pub fn project_color(mut self, color: egui::Color32) -> Self {
        self.project_color = Some(color);
        self
    }

    /// Window toolbar; use the same chrome when configuring the host viewport.
    pub fn toolbar(
        mut self,
        chrome: IdeFrameDecorator,
        content: impl FnOnce(&mut Ui) + 'a,
    ) -> Self {
        self.toolbar = Some((chrome, Some(Box::new(content))));
        self
    }
    /// A status area on the window frame, typically containing `StatusBarWidget`s.
    pub fn status(mut self, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.status = Some(Some(Box::new(content)));
        self
    }

    /// Declares a tool window in `region` titled `title`, with its stripe button showing
    /// `icon`. Its content comes from the closure given to [`show_with`](Self::show_with),
    /// which lets one closure fill every region from the same application state.
    pub fn tool(
        mut self,
        region: ToolWindowAnchor,
        title: impl Into<String>,
        icon: impl Into<Icon>,
    ) -> Self {
        let mut declared = Region::declared(title);
        declared.icon = Some(icon.into());
        *self.slot(region) = Some(declared);
        self
    }

    /// Content tabs in `region`'s tool window header, after its title, like Android
    /// Studio's Problems tool window ("File", "Project Errors"). A click selects one; the
    /// selection is [`ToolWindowPaneState::tool_tab`] (kept with the layout), and the next frame
    /// shows it. Call it after declaring the region.
    pub fn tool_tabs(mut self, region: ToolWindowAnchor, tabs: Vec<String>) -> Self {
        if let Some(slot) = self.slot(region) {
            slot.tabs = Some(tabs);
        }
        self
    }

    /// Declares a tool window from a caller-owned [`ToolWindow`]: its anchor, title, icon
    /// and `Alt+digit` shortcut, and its [`ContentManager`] as header tabs. A tab the user
    /// selects becomes the manager's selection; a closed one is removed from it (and
    /// reported as [`ToolWindowEvent::ContentCloseRequested`]). The content itself still
    /// comes from [`show_with`](Self::show_with) as [`ToolWindowPlace::Tool`].
    pub fn tool_window(mut self, window: &'a mut ToolWindow) -> Self {
        let mut declared = Region::declared(window.title.clone());
        declared.icon = Some(window.icon);
        declared.activate_number = window.activate_number;
        let anchor = window.anchor;
        declared.contents = Some(&mut window.contents);
        *self.slot(anchor) = Some(declared);
        self
    }

    /// Binds `Alt+digit` (1 to 9, IntelliJ `ActivateXToolWindow`) to `region`'s tool window:
    /// it opens a hidden one, hides the active one and activates a visible one (reported
    /// as [`ToolWindowEvent`]s). The stripe button's tooltip shows the shortcut. Call it
    /// after declaring the region.
    pub fn activate_with(mut self, region: ToolWindowAnchor, digit: u8) -> Self {
        if let Some(slot) = self.slot(region) {
            slot.activate_number = Some(digit.clamp(1, 9));
        }
        self
    }

    /// The actions of `region`'s tool window header, from an [`ActionGroup`] (IntelliJ place
    /// `TOOLWINDOW_TITLE`), drawn as buttons next to the Maximize and Hide buttons under the
    /// header's show-on-hover rules. Call it after declaring the region.
    pub fn header_actions(
        mut self,
        region: ToolWindowAnchor,
        group: &'a dyn ActionGroup,
        data_context: &'a DataContext,
    ) -> Self {
        if let Some(slot) = self.slot(region) {
            slot.title_group = Some((group, data_context));
        }
        self
    }

    /// The options menu of `region`'s tool window (place `TOOLWINDOW_POPUP`): the gear
    /// button of its header and a right click on the header open it. Without one, a right
    /// click is reported as [`ToolWindowEvent::HeaderMenuRequested`]. The actions get the
    /// data context given to [`header_actions`](Self::header_actions) (or an empty one).
    pub fn options_menu(mut self, region: ToolWindowAnchor, group: &'a dyn ActionGroup) -> Self {
        if let Some(slot) = self.slot(region) {
            slot.options = Some(group);
        }
        self
    }

    /// Declares the window toolbar; its content comes from [`show_with`](Self::show_with)
    /// as [`ToolWindowPlace::Toolbar`].
    pub fn with_toolbar(mut self, chrome: IdeFrameDecorator) -> Self {
        self.toolbar = Some((chrome, None));
        self
    }

    /// Declares the status bar; its content comes from [`show_with`](Self::show_with) as
    /// [`ToolWindowPlace::Status`].
    pub fn with_status(mut self) -> Self {
        self.status = Some(None);
        self
    }

    fn slot(&mut self, region: ToolWindowAnchor) -> &mut Option<Region<'a>> {
        match region {
            ToolWindowAnchor::Left => &mut self.left,
            ToolWindowAnchor::Right => &mut self.right,
            ToolWindowAnchor::Bottom => &mut self.bottom,
        }
    }

    /// Renders a whole window, asking `content` for every declared place (center, tool
    /// windows, toolbar, status bar) with the same `state`. One closure and one borrow of
    /// the application state, instead of a closure per region:
    ///
    /// ```
    /// # egui::__run_test_ctx(|ctx| {
    /// use archipel_kit::wm::{StatusBarWidget, ToolWindowPane, ToolWindowPlace, ToolWindowAnchor, ToolWindowPaneState};
    /// use archipel_kit::Glyph;
    /// struct App { layout: ToolWindowPaneState, requests: Vec<String> }
    /// let mut app = App { layout: ToolWindowPaneState::default(), requests: vec![] };
    /// let App { layout, requests } = &mut app;
    /// ToolWindowPane::new("app", layout)
    ///     .tool(ToolWindowAnchor::Left, "Collections", Glyph::Folder)
    ///     .with_status()
    ///     .show_with(ctx, requests, |place, ui, requests| match place {
    ///         ToolWindowPlace::Tool(ToolWindowAnchor::Left) => {
    ///             ui.label(format!("{} requests", requests.len()));
    ///         }
    ///         ToolWindowPlace::Status => {
    ///             ui.add(StatusBarWidget::new("Ready"));
    ///         }
    ///         _ => {}
    ///     });
    /// # });
    /// ```
    pub fn show_with<S>(
        self,
        ctx: &egui::Context,
        state: &mut S,
        content: impl FnMut(ToolWindowPlace, &mut Ui, &mut S),
    ) -> Response {
        let shared = std::cell::RefCell::new((state, content));
        // Each place is drawn one after the other, so the borrows never overlap.
        let fill = |place: ToolWindowPlace, ui: &mut Ui| {
            let mut guard = shared.borrow_mut();
            let (state, content) = &mut *guard;
            content(place, ui, state);
        };
        self.dispatch(&fill)
            .show(ctx, |ui| fill(ToolWindowPlace::Center, ui))
    }

    /// Like [`show_with`](Self::show_with), into a bounded area (no toolbar or status bar).
    pub fn show_inside_with<S>(
        self,
        ui: &mut Ui,
        state: &mut S,
        content: impl FnMut(ToolWindowPlace, &mut Ui, &mut S),
    ) -> Response {
        let shared = std::cell::RefCell::new((state, content));
        let fill = |place: ToolWindowPlace, ui: &mut Ui| {
            let mut guard = shared.borrow_mut();
            let (state, content) = &mut *guard;
            content(place, ui, state);
        };
        self.dispatch(&fill)
            .show_inside(ui, |ui| fill(ToolWindowPlace::Center, ui))
    }

    /// Fills every declared place without content from `fill`.
    fn dispatch<'b>(self, fill: &'b (dyn Fn(ToolWindowPlace, &mut Ui) + 'b)) -> ToolWindowPane<'b>
    where
        'a: 'b,
    {
        let region = |region: Option<Region<'a>>, place: ToolWindowAnchor| {
            region.map(|r| Region {
                content: Some(r.content.unwrap_or_else(|| {
                    Box::new(move |ui: &mut Ui| fill(ToolWindowPlace::Tool(place), ui))
                })),
                actions: Some(r.actions.unwrap_or_else(|| {
                    Box::new(move |ui: &mut Ui| fill(ToolWindowPlace::ToolActions(place), ui))
                })),
                title: r.title,
                tabs: r.tabs,
                padded: r.padded,
                icon: r.icon,
                contents: r.contents,
                activate_number: r.activate_number,
                title_group: r.title_group,
                options: r.options,
            })
        };
        ToolWindowPane {
            id: self.id,
            state: self.state,
            left: region(self.left, ToolWindowAnchor::Left),
            right: region(self.right, ToolWindowAnchor::Right),
            bottom: region(self.bottom, ToolWindowAnchor::Bottom),
            toolbar: self.toolbar.map(|(chrome, content)| {
                let content: Content<'b> = content.unwrap_or_else(|| {
                    Box::new(move |ui: &mut Ui| fill(ToolWindowPlace::Toolbar, ui))
                });
                (chrome, Some(content))
            }),
            status: self.status.map(|content| {
                let content: Content<'b> = content.unwrap_or_else(|| {
                    Box::new(move |ui: &mut Ui| fill(ToolWindowPlace::Status, ui))
                });
                Some(content)
            }),
            project_color: self.project_color,
            glow_anchor: self.glow_anchor,
            center_padded: self.center_padded,
            frame_background: self.frame_background,
            main_menu: self.main_menu,
        }
    }

    /// Render a whole window. Hidden or temporarily constrained region callbacks are
    /// not invoked. The host can restore hidden panes by changing the corresponding flag.
    pub fn show(mut self, ctx: &egui::Context, center: impl FnOnce(&mut Ui)) -> Response {
        if let Some((chrome, content)) = self.toolbar.take() {
            let mut toolbar = MainToolbar::new(self.id.with("toolbar")).chrome(chrome);
            if let Some(color) = self.project_color {
                toolbar = toolbar.project_color(color);
            }
            if let Some((group, context)) = self.main_menu {
                toolbar = toolbar.main_menu(group, context);
            }
            toolbar.show(ctx, |ui| {
                if let Some(content) = content {
                    content(ui);
                }
            });
        }
        // The glow continues from the toolbar: anchor it to the window, not the islands.
        self.glow_anchor = Some(ctx.input(|i| i.viewport_rect()));
        let metrics = tokens::metrics(ctx);
        // Presentation and Distraction Free modes hide the status bar.
        let status = self
            .status
            .take()
            .filter(|_| !metrics.view.hides_status_bar());
        if let Some(content) = status {
            // Android Studio's status bar: 30 pt on the frame, items 21 pt high. New UI and
            // Classic draw a 1 pt line on top of it.
            let line = metrics.status_top_line;
            egui::TopBottomPanel::bottom(self.id.with("status"))
                .show_separator_line(false)
                .exact_height(metrics.status_bar_height)
                .frame(
                    egui::Frame::new()
                        .fill(Theme::colors(ctx).frame)
                        .inner_margin(egui::Margin {
                            left: metrics.status_bar_inset as i8,
                            right: metrics.status_bar_inset as i8,
                            top: line as i8,
                            bottom: 0,
                        }),
                )
                .show(ctx, |ui| {
                    if line > 0.0 {
                        let rect = ui.max_rect();
                        let x = (rect.left() - metrics.status_bar_inset)
                            ..=(rect.right() + metrics.status_bar_inset);
                        ui.painter().hline(
                            x,
                            rect.top() - line / 2.0,
                            egui::Stroke::new(line, Theme::colors(ctx).divider),
                        );
                    }
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        if let Some(content) = content {
                            content(ui);
                        }
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Theme::colors(ctx).frame)
                    .inner_margin(metrics.island_gap as i8),
            )
            .show(ctx, |ui| self.show_inside(ui, center))
            .response
    }

    /// Render only the islands into a bounded area, for embedding or previews.
    /// Toolbar and status callbacks are used only by `show`.
    pub fn show_inside(self, ui: &mut Ui, center: impl FnOnce(&mut Ui)) -> Response {
        let Self {
            id,
            state,
            mut left,
            mut right,
            mut bottom,
            project_color,
            glow_anchor,
            center_padded,
            frame_background,
            ..
        } = self;
        let signals = Signals {
            hide: Cell::new(None),
            maximize: Cell::new(None),
            tab: Cell::new(None),
            tabs: state.tool_tabs,
            events: RefCell::new(Vec::new()),
        };
        let present = [left.is_some(), right.is_some(), bottom.is_some()];
        let numbers = [
            left.as_ref().and_then(|r| r.activate_number),
            right.as_ref().and_then(|r| r.activate_number),
            bottom.as_ref().and_then(|r| r.activate_number),
        ];
        let full = ui.available_rect_before_wrap();
        if frame_background {
            ui.painter()
                .rect_filled(full, 0, Theme::colors(ui.ctx()).frame);
        }
        // The project glow reaches over the stripes only in Islands; New UI tints just the
        // toolbar, Classic not at all (looks spec §8.2 `glow`).
        if let Some(color) = project_color.filter(|_| tokens::metrics(ui.ctx()).is_islands()) {
            crate::parts::project_glow(ui.painter(), glow_anchor.unwrap_or(full), color);
        }
        let ctx = ui.ctx().clone();
        let metrics = tokens::metrics(&ctx);
        // Presentation mode hides the tool windows (the layout comes back after it).
        if metrics.view.hides_tool_windows() {
            left = None;
            right = None;
            bottom = None;
        }
        // Tool window stripes on the frame, beside the islands.
        let mut rect = full;
        let gap = metrics.island_gap;
        let stripe_width = metrics.stripe_column();
        let show_stripes = !metrics.view.hides_stripes();
        let mut toggled = None;
        let mut closed = None;
        let mut events: Vec<ToolWindowEvent> = Vec::new();
        if show_stripes && !metrics.has_icon_stripes() {
            rect = classic_stripes(
                ui,
                id,
                full,
                [&left, &right, &bottom],
                state,
                &mut toggled,
                &mut closed,
                &mut events,
            );
        }
        let icon_stripes = show_stripes && metrics.has_icon_stripes();
        let button = |region: &Option<Region<'_>>, kind: ToolWindowAnchor, hidden: bool| {
            region.as_ref().and_then(|r| {
                r.icon.map(|icon| {
                    let mut button = SquareStripeButton::new(icon, r.title.clone())
                        .open(!hidden)
                        .active(!hidden && region_active(&ctx, id, kind));
                    if let Some(digit) = r.activate_number {
                        button = button.shortcut(format!("Alt+{digit}"));
                    }
                    button
                })
            })
        };
        let left_top =
            button(&left, ToolWindowAnchor::Left, state.left_hidden).filter(|_| icon_stripes);
        let left_bottom =
            button(&bottom, ToolWindowAnchor::Bottom, state.bottom_hidden).filter(|_| icon_stripes);
        let right_top =
            button(&right, ToolWindowAnchor::Right, state.right_hidden).filter(|_| icon_stripes);
        if left_top.is_some() || left_bottom.is_some() {
            let stripe = egui::Rect::from_min_max(
                full.min,
                egui::pos2(full.left() + stripe_width, full.bottom()),
            );
            rect.min.x = stripe.right() + gap;
            // New UI: a 1 pt line between the stripe and the tool windows.
            crate::parts::Surface::island_lines(ui.painter(), stripe, [false, false, true, false]);
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(stripe));
            let has_top = left_top.is_some();
            let mut bar = ToolWindowToolbar::new(id.with("left-stripe"));
            if let Some(b) = left_top {
                bar = bar.top(b);
            }
            if let Some(b) = left_bottom {
                bar = bar.bottom(b);
            }
            let anchor_of = |index: usize| {
                if index == 0 && has_top {
                    ToolWindowAnchor::Left
                } else {
                    ToolWindowAnchor::Bottom
                }
            };
            let response = bar.show_with_events(&mut column);
            toggled = response.clicked.map(anchor_of);
            closed = response.close_clicked.map(anchor_of);
            if let Some((index, pos)) = response.popup {
                events.push(ToolWindowEvent::StripeMenuRequested(anchor_of(index), pos));
            }
            if let Some((index, pos)) = response.dropped {
                // The top half of the left stripe is the Left group, the bottom half the
                // Bottom group.
                let to = if pos.y < response.rect.center().y {
                    ToolWindowAnchor::Left
                } else {
                    ToolWindowAnchor::Bottom
                };
                if to != anchor_of(index) {
                    events.push(ToolWindowEvent::MovedToAnchor {
                        window: anchor_of(index),
                        to,
                    });
                }
            }
        }
        if let Some(b) = right_top {
            let stripe = egui::Rect::from_min_max(
                egui::pos2(full.right() - stripe_width, full.top()),
                full.max,
            );
            rect.max.x = stripe.left() - gap;
            crate::parts::Surface::island_lines(ui.painter(), stripe, [true, false, false, false]);
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(stripe));
            let response = ToolWindowToolbar::new(id.with("right-stripe"))
                .top(b)
                .show_with_events(&mut column);
            if response.clicked.is_some() {
                toggled = Some(ToolWindowAnchor::Right);
            }
            if response.close_clicked.is_some() {
                closed = Some(ToolWindowAnchor::Right);
            }
            if let Some((_, pos)) = response.popup {
                events.push(ToolWindowEvent::StripeMenuRequested(
                    ToolWindowAnchor::Right,
                    pos,
                ));
            }
            if let Some((_, _)) = response.dropped {
                // Only the left stripe has two groups: a drop on the left stripe moves it.
                if let Some(pos) = ui.input(|i| i.pointer.latest_pos())
                    && pos.x < full.center().x
                {
                    events.push(ToolWindowEvent::MovedToAnchor {
                        window: ToolWindowAnchor::Right,
                        to: if pos.y < full.center().y {
                            ToolWindowAnchor::Left
                        } else {
                            ToolWindowAnchor::Bottom
                        },
                    });
                }
            }
        }
        if let Some(region) = toggled {
            if state.is_visible(region) {
                state.hide(region);
                events.push(ToolWindowEvent::Hidden(region));
            } else {
                state.show(region);
                events.push(ToolWindowEvent::Activated(region));
            }
        }
        // A close click (middle button, or Shift and left) on a stripe button hides.
        if let Some(region) = closed
            && state.is_visible(region)
        {
            state.hide(region);
            events.push(ToolWindowEvent::Hidden(region));
        }
        let maximized = state.maximized;
        let response = ui
            .scope_builder(egui::UiBuilder::new().id_salt(id).max_rect(rect), |ui| {
                let maximized_region = match maximized {
                    Some(ToolWindowAnchor::Left) if !state.left_hidden => left.take(),
                    Some(ToolWindowAnchor::Right) if !state.right_hidden => right.take(),
                    Some(ToolWindowAnchor::Bottom) if !state.bottom_hidden => bottom.take(),
                    _ => None,
                };
                if let Some(region) = maximized_region {
                    region.show(
                        ui,
                        id,
                        maximized.unwrap_or(ToolWindowAnchor::Left),
                        true,
                        &signals,
                    );
                    return;
                }
                // Missing/hidden maximized panes must never hide the center.
                state.maximized = None;
                let min = tokens::size::SPLIT_MIN_PANE;
                let center_min = tokens::size::WORKBENCH_CENTER_MIN;
                let gap = metrics.split_divider;
                let left_open = left.is_some() && !state.left_hidden;
                let right_open = right.is_some() && !state.right_hidden;
                let auto_right = right_open
                    && rect.width()
                        < center_min
                            + min * (1 + usize::from(left_open)) as f32
                            + gap * (1 + usize::from(left_open)) as f32;
                let auto_left = left_open && rect.width() < center_min + min + gap;
                let auto_bottom = bottom.is_some()
                    && !state.bottom_hidden
                    && rect.height() < center_min + min + gap;
                let mut left_hidden = state.left_hidden || auto_left;
                let mut right_hidden = state.right_hidden || auto_right;
                let mut bottom_hidden = state.bottom_hidden || auto_bottom;
                let main = |ui: &mut Ui| {
                    let middle = |ui: &mut Ui| {
                        if let Some(region) = right {
                            let split =
                                Splitter::horizontal(id.with("right"), &mut state.center_ratio)
                                    .default_ratio(0.75)
                                    .min_sizes(center_min, min)
                                    .label("Resize right tool window")
                                    .line(false)
                                    .collapsible(Side::Second, &mut right_hidden)
                                    .enabled(!auto_right)
                                    .show(
                                        ui,
                                        |ui| center_island(ui, center_padded, center),
                                        |ui| {
                                            if ui.is_visible() {
                                                region.show(
                                                    ui,
                                                    id,
                                                    ToolWindowAnchor::Right,
                                                    false,
                                                    &signals,
                                                );
                                            }
                                        },
                                    );
                            if split.divider.changed() {
                                state.right_hidden = right_hidden;
                            }
                        } else {
                            center_island(ui, center_padded, center);
                        }
                    };
                    if let Some(region) = left {
                        let split = Splitter::horizontal(id.with("left"), &mut state.left_ratio)
                            .default_ratio(0.22)
                            .min_sizes(
                                min,
                                center_min
                                    + if right_open && !auto_right {
                                        min + gap
                                    } else {
                                        0.0
                                    },
                            )
                            .label("Resize left tool window")
                            .line(false)
                            .collapsible(Side::First, &mut left_hidden)
                            .enabled(!auto_left)
                            .show(
                                ui,
                                |ui| {
                                    if ui.is_visible() {
                                        region.show(
                                            ui,
                                            id,
                                            ToolWindowAnchor::Left,
                                            false,
                                            &signals,
                                        );
                                    }
                                },
                                middle,
                            );
                        if split.divider.changed() {
                            state.left_hidden = left_hidden;
                        }
                    } else {
                        middle(ui);
                    }
                };
                if let Some(region) = bottom {
                    let split = Splitter::vertical(id.with("bottom"), &mut state.top_ratio)
                        .default_ratio(0.7)
                        .min_sizes(center_min, min)
                        .label("Resize bottom tool window")
                        .line(false)
                        .collapsible(Side::Second, &mut bottom_hidden)
                        .enabled(!auto_bottom)
                        .show(ui, main, |ui| {
                            if ui.is_visible() {
                                region.show(ui, id, ToolWindowAnchor::Bottom, false, &signals);
                            }
                        });
                    if split.divider.changed() {
                        state.bottom_hidden = bottom_hidden;
                    }
                } else {
                    main(ui);
                }
            })
            .response;
        if let Some((region, tab)) = signals.tab.get() {
            state.tool_tabs[region_index(region)] = tab;
        }
        if let Some(region) = signals.hide.get() {
            state.hide(region);
            events.push(ToolWindowEvent::Hidden(region));
        } else if let Some(region) = signals.maximize.get() {
            state.maximized = if maximized == Some(region) {
                None
            } else {
                Some(region)
            };
            events.push(ToolWindowEvent::MaximizeToggled(
                region,
                state.maximized == Some(region),
            ));
        }
        events.extend(signals.events.take());
        handle_keys(&ctx, id, state, &Keys { present, numbers }, &mut events);
        state.events.extend(events);
        response
    }
}

impl<'a> Region<'a> {
    fn new(title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            title: title.into(),
            content: Some(Box::new(content)),
            actions: None,
            tabs: None,
            padded: false,
            icon: None,
            contents: None,
            activate_number: None,
            title_group: None,
            options: None,
        }
    }

    fn declared(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: None,
            actions: None,
            tabs: None,
            padded: false,
            icon: None,
            contents: None,
            activate_number: None,
            title_group: None,
            options: None,
        }
    }

    fn show(
        self,
        ui: &mut Ui,
        workbench: Id,
        region: ToolWindowAnchor,
        maximized: bool,
        signals: &Signals,
    ) {
        let (hide, maximize) = (&signals.hide, &signals.maximize);
        let label = format!(
            "{} {}",
            if maximized { "Restore" } else { "Maximize" },
            self.title
        );
        // As in the IntelliJ Islands UI, the header actions fade in while the pointer is
        // anywhere over the tool window (or focus is inside it) and fade out when it leaves.
        let mut contents = self.contents;
        let (tabs, mut selected, closable) = match contents.as_deref().filter(|c| !c.is_empty()) {
            Some(manager) => manager.tabs(),
            None => (
                self.tabs.unwrap_or_default(),
                signals.tabs[region_index(region)],
                Vec::new(),
            ),
        };
        let before = selected;
        let managed = contents.as_deref().is_some_and(|c| !c.is_empty());
        let closed_tab = Cell::new(None);
        // Every island header is separated from its content by a hairline (the suite's
        // choice; Android Studio draws it only under content tabs).
        let mut panel = SimpleToolWindowPanel::new(self.title).header_separator(true);
        if !tabs.is_empty() {
            panel = panel.tabs(&tabs, &mut selected).tab_closable(&closable);
        }
        let empty_context = DataContext::empty();
        let (title_group, data_context) = match self.title_group {
            Some((group, context)) => (Some(group), context),
            None => (None, &empty_context),
        };
        if let Some(group) = title_group {
            panel = panel.title_actions(group);
        }
        if let Some(group) = self.options {
            panel = panel.popup_group(group);
        }
        panel
            .data_context(data_context)
            .hover_actions(true)
            .padded(self.padded)
            .on_hide(|| hide.set(Some(region)))
            .on_activate(|| {
                signals
                    .events
                    .borrow_mut()
                    .push(ToolWindowEvent::Activated(region));
            })
            .on_toggle_maximize(|| maximize.set(Some(region)))
            .on_popup(|pos| {
                signals
                    .events
                    .borrow_mut()
                    .push(ToolWindowEvent::HeaderMenuRequested(region, pos));
            })
            .on_close_tab(|index| closed_tab.set(Some(index)))
            .actions(|ui| {
                let custom = self.actions;
                let button = ActionButton::new(Glyph::Fullscreen, label).with(HelpTooltip::new(
                    if maximized { "Restore" } else { "Maximize" },
                ));
                if ui.add(button).clicked() {
                    maximize.set(Some(region));
                }
                if let Some(custom) = custom {
                    custom(ui);
                }
            })
            .show(ui, |ui| {
                ui.set_min_height((ui.available_height() - tokens::size::SECTION_SPACING).max(0.0));
                crate::parts::desktop(egui::ScrollArea::both())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(content) = self.content {
                            content(ui);
                        }
                    });
            });
        if selected != before {
            if managed {
                if let Some(manager) = contents.as_deref_mut() {
                    manager.select_index(selected);
                }
            } else {
                signals.tab.set(Some((region, selected)));
            }
            ui.ctx().request_repaint();
        }
        if let Some(index) = closed_tab.get() {
            signals
                .events
                .borrow_mut()
                .push(ToolWindowEvent::ContentCloseRequested(region, index));
            if let Some(manager) = contents {
                manager.remove_content(index);
            }
            ui.ctx().request_repaint();
        }
        // Where the island is, to tell next frame whether the focus is in it.
        let rect = ui.min_rect();
        ui.ctx()
            .data_mut(|d| d.insert_temp(region_key(workbench, region), rect));
    }
}

fn region_key(workbench: Id, region: ToolWindowAnchor) -> Id {
    workbench.with(("kit-region-rect", region))
}

/// Whether the keyboard focus is inside `region`'s island (as drawn last frame): that tool
/// window is the active one.
fn region_active(ctx: &egui::Context, workbench: Id, region: ToolWindowAnchor) -> bool {
    let Some(rect) = ctx.data(|d| d.get_temp::<egui::Rect>(region_key(workbench, region))) else {
        return false;
    };
    ctx.memory(|m| m.focused())
        .and_then(|id| ctx.read_response(id))
        .is_some_and(|response| rect.contains_rect(response.rect))
}

/// The Classic look's three text stripes around the tool windows (looks spec §2.3, §3.2):
/// left and right along the sides, and one along the bottom for the bottom tool window. Each
/// is drawn even when empty, as in the classic IDE. Returns the area left for the islands;
/// clicks are reported like the icon stripes'.
#[allow(clippy::too_many_arguments)]
fn classic_stripes(
    ui: &mut Ui,
    id: Id,
    full: egui::Rect,
    regions: [&Option<Region<'_>>; 3],
    state: &ToolWindowPaneState,
    toggled: &mut Option<ToolWindowAnchor>,
    closed: &mut Option<ToolWindowAnchor>,
    events: &mut Vec<ToolWindowEvent>,
) -> egui::Rect {
    use crate::wm::{Stripe, StripeButton, StripeSide};
    let metrics = tokens::metrics(ui.ctx());
    let (side, low) = (metrics.stripe_width, metrics.stripe_height);
    let rects = [
        egui::Rect::from_min_max(
            full.min,
            egui::pos2(full.left() + side, full.bottom() - low),
        ),
        egui::Rect::from_min_max(
            egui::pos2(full.right() - side, full.top()),
            egui::pos2(full.right(), full.bottom() - low),
        ),
        egui::Rect::from_min_max(egui::pos2(full.left(), full.bottom() - low), full.max),
    ];
    let sides = [StripeSide::Left, StripeSide::Right, StripeSide::Bottom];
    for ((region, anchor), (rect, stripe_side)) in regions
        .into_iter()
        .zip(ANCHORS)
        .zip(rects.into_iter().zip(sides))
    {
        let mut stripe = Stripe::new(id.with(("classic-stripe", anchor)), stripe_side);
        if let Some(r) = region.as_ref().filter(|r| r.icon.is_some()) {
            let open = state.is_visible(anchor);
            let mut button = StripeButton::new(r.title.clone())
                .open(open)
                .active(open && region_active(ui.ctx(), id, anchor));
            if let Some(icon) = r.icon {
                button = button.icon(icon);
            }
            if let Some(digit) = r.activate_number {
                button = button.shortcut(format!("Alt+{digit}"));
            }
            stripe = stripe.button(button);
        }
        let mut column = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let response = stripe.show(&mut column);
        if response.clicked.is_some() {
            *toggled = Some(anchor);
        }
        if response.close_clicked.is_some() {
            *closed = Some(anchor);
        }
        if let Some((_, pos)) = response.popup {
            events.push(ToolWindowEvent::StripeMenuRequested(anchor, pos));
        }
    }
    egui::Rect::from_min_max(
        egui::pos2(rects[0].right(), full.top()),
        egui::pos2(rects[1].left(), rects[2].top()),
    )
}

fn center_island(ui: &mut Ui, padded: bool, content: impl FnOnce(&mut Ui)) {
    if !ui.is_visible() {
        return;
    }
    let mut frame = crate::parts::Surface::Island.frame(ui.ctx());
    if !padded {
        frame = frame.inner_margin(0);
    }
    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        content(ui);
    });
}

/// What [`handle_keys`] needs to know about the tool windows of this frame.
struct Keys {
    /// Which of left, right and bottom are declared.
    present: [bool; 3],
    /// Their `Alt+digit` shortcuts.
    numbers: [Option<u8>; 3],
}

const ANCHORS: [ToolWindowAnchor; 3] = [
    ToolWindowAnchor::Left,
    ToolWindowAnchor::Right,
    ToolWindowAnchor::Bottom,
];

fn digit_key(digit: u8) -> Option<egui::Key> {
    use egui::Key::*;
    Some(match digit {
        1 => Num1,
        2 => Num2,
        3 => Num3,
        4 => Num4,
        5 => Num5,
        6 => Num6,
        7 => Num7,
        8 => Num8,
        9 => Num9,
        _ => return None,
    })
}

/// The keyboard of the tool windows (IntelliJ spec section 8.3): `Alt+digit` activates,
/// `Shift+Esc` hides the active tool window, `Ctrl+Shift+F12` hides all, `Ctrl+Shift+'`
/// maximizes the active one, and `Esc` in a tool window whose content did not use it asks
/// to return to the editor. The layout changes are applied to `state`; everything is also
/// pushed to `events`.
fn handle_keys(
    ctx: &egui::Context,
    id: Id,
    state: &mut ToolWindowPaneState,
    keys: &Keys,
    events: &mut Vec<ToolWindowEvent>,
) {
    use egui::{Key, Modifiers};
    // Which tool window has the focus. egui drops the focus at the start of the frame an Esc
    // arrives in (Shift+Esc too), so that frame uses the window that had it in the last one.
    let memo = id.with("kit-active-region");
    let escape_now = ctx.input(|i| i.key_pressed(Key::Escape));
    let current = ANCHORS
        .into_iter()
        .zip(keys.present)
        .find(|&(anchor, present)| {
            present && state.is_visible(anchor) && region_active(ctx, id, anchor)
        })
        .map(|(anchor, _)| anchor);
    // A text field that has the focus uses Esc itself (it publishes its IME area while
    // focused), so Esc does not leave the tool window; same rule for the Esc frame.
    let text_key = id.with("kit-text-focus");
    let text_focused = if escape_now {
        ctx.data(|d| d.get_temp::<bool>(text_key)).unwrap_or(false)
    } else {
        let now = ctx.output(|o| o.ime.is_some());
        ctx.data_mut(|d| d.insert_temp(text_key, now));
        now
    };
    let active = if escape_now {
        ctx.data(|d| d.get_temp::<Option<ToolWindowAnchor>>(memo))
            .flatten()
            .filter(|&anchor| state.is_visible(anchor))
    } else {
        ctx.data_mut(|d| d.insert_temp(memo, current));
        current
    };
    let consume = |modifiers: Modifiers, key: Key| ctx.input_mut(|i| i.consume_key(modifiers, key));

    if consume(Modifiers::COMMAND | Modifiers::SHIFT, Key::F12) {
        state.hide_all();
        events.push(ToolWindowEvent::AllHidden);
        return;
    }
    for (anchor, number) in ANCHORS.into_iter().zip(keys.numbers) {
        let Some(key) = number.and_then(digit_key) else {
            continue;
        };
        if !consume(Modifiers::ALT, key) {
            continue;
        }
        if !state.is_visible(anchor) {
            state.show(anchor);
            events.push(ToolWindowEvent::Activated(anchor));
        } else if active == Some(anchor) {
            state.hide(anchor);
            events.push(ToolWindowEvent::Hidden(anchor));
        } else {
            events.push(ToolWindowEvent::Activated(anchor));
        }
        return;
    }
    let Some(active) = active else {
        return;
    };
    if consume(Modifiers::SHIFT, Key::Escape) {
        state.hide(active);
        events.push(ToolWindowEvent::Hidden(active));
    } else if consume(Modifiers::COMMAND | Modifiers::SHIFT, Key::Quote) {
        state.maximized = if state.maximized == Some(active) {
            None
        } else {
            Some(active)
        };
        events.push(ToolWindowEvent::MaximizeToggled(
            active,
            state.maximized == Some(active),
        ));
    } else if !text_focused && ctx.input(|i| i.key_pressed(Key::Escape)) {
        events.push(ToolWindowEvent::FocusEditor(active));
    }
}
