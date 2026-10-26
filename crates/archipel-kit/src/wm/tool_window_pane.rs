//! A shared docked islands layout, independent of application content and policy.

use crate::Icon;
use crate::action_system::ActionButton;
use crate::behaviour::WidgetExt;
use crate::ide::HelpTooltip;
use crate::openapi::ui::{Side, Splitter};
use crate::wm::{
    IdeFrameDecorator, MainToolbar, SimpleToolWindowPanel, SquareStripeButton, ToolWindowToolbar,
};
use crate::{Glyph, Theme, egui, tokens};
use egui::{Id, Response, Ui};
use std::cell::Cell;

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
}

impl ToolWindowPaneState {
    /// The selected content tab of `region`'s tool window.
    pub fn tool_tab(&self, region: ToolWindowAnchor) -> usize {
        self.tool_tabs[region_index(region)]
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
}
struct Region<'a> {
    title: String,
    content: Option<Content<'a>>,
    actions: Option<Content<'a>>,
    tabs: Option<Vec<String>>,
    padded: bool,
    icon: Option<Icon>,
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
        }
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
            toolbar.show(ctx, |ui| {
                if let Some(content) = content {
                    content(ui);
                }
            });
        }
        // The glow continues from the toolbar: anchor it to the window, not the islands.
        self.glow_anchor = Some(ctx.input(|i| i.viewport_rect()));
        if let Some(content) = self.status.take() {
            // Android Studio's status bar: 30 pt on the frame, items 21 pt high.
            egui::TopBottomPanel::bottom(self.id.with("status"))
                .show_separator_line(false)
                .exact_height(tokens::size::STATUS_BAR_HEIGHT)
                .frame(
                    egui::Frame::new()
                        .fill(Theme::colors(ctx).frame)
                        .inner_margin(egui::Margin::symmetric(
                            tokens::size::STATUS_BAR_INSET as i8,
                            0,
                        )),
                )
                .show(ctx, |ui| {
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
                    .inner_margin(tokens::size::ISLAND_GAP as i8),
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
        };
        let full = ui.available_rect_before_wrap();
        if frame_background {
            ui.painter()
                .rect_filled(full, 0, Theme::colors(ui.ctx()).frame);
        }
        if let Some(color) = project_color {
            crate::parts::project_glow(ui.painter(), glow_anchor.unwrap_or(full), color);
        }
        // Tool window stripes on the frame, beside the islands.
        let mut rect = full;
        let stripe_width = tokens::stripe::WIDTH - tokens::size::ISLAND_GAP;
        let ctx = ui.ctx().clone();
        let button = |region: &Option<Region<'_>>, kind: ToolWindowAnchor, hidden: bool| {
            region.as_ref().and_then(|r| {
                r.icon.map(|icon| {
                    SquareStripeButton::new(icon, r.title.clone())
                        .open(!hidden)
                        .active(!hidden && region_active(&ctx, id, kind))
                })
            })
        };
        let left_top = button(&left, ToolWindowAnchor::Left, state.left_hidden);
        let left_bottom = button(&bottom, ToolWindowAnchor::Bottom, state.bottom_hidden);
        let right_top = button(&right, ToolWindowAnchor::Right, state.right_hidden);
        let mut toggled = None;
        if left_top.is_some() || left_bottom.is_some() {
            let stripe = egui::Rect::from_min_max(
                full.min,
                egui::pos2(full.left() + stripe_width, full.bottom()),
            );
            rect.min.x = stripe.right() + tokens::size::ISLAND_GAP;
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(stripe));
            let has_top = left_top.is_some();
            let mut bar = ToolWindowToolbar::new(id.with("left-stripe"));
            if let Some(b) = left_top {
                bar = bar.top(b);
            }
            if let Some(b) = left_bottom {
                bar = bar.bottom(b);
            }
            toggled = match bar.show(&mut column) {
                Some(0) if has_top => Some(ToolWindowAnchor::Left),
                Some(_) => Some(ToolWindowAnchor::Bottom),
                None => None,
            };
        }
        if let Some(b) = right_top {
            let stripe = egui::Rect::from_min_max(
                egui::pos2(full.right() - stripe_width, full.top()),
                full.max,
            );
            rect.max.x = stripe.left() - tokens::size::ISLAND_GAP;
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(stripe));
            if ToolWindowToolbar::new(id.with("right-stripe"))
                .top(b)
                .show(&mut column)
                .is_some()
            {
                toggled = Some(ToolWindowAnchor::Right);
            }
        }
        if let Some(region) = toggled {
            let hidden = match region {
                ToolWindowAnchor::Left => &mut state.left_hidden,
                ToolWindowAnchor::Right => &mut state.right_hidden,
                ToolWindowAnchor::Bottom => &mut state.bottom_hidden,
            };
            *hidden = !*hidden;
            if *hidden && state.maximized == Some(region) {
                state.maximized = None;
            }
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
                let gap = tokens::size::SPLIT_DIVIDER;
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
            match region {
                ToolWindowAnchor::Left => state.left_hidden = true,
                ToolWindowAnchor::Right => state.right_hidden = true,
                ToolWindowAnchor::Bottom => state.bottom_hidden = true,
            }
            state.maximized = None;
        } else if let Some(region) = signals.maximize.get() {
            state.maximized = if maximized == Some(region) {
                None
            } else {
                Some(region)
            };
        }
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
        let mut selected = signals.tabs[region_index(region)];
        let before = selected;
        let tabs = self.tabs.unwrap_or_default();
        // Every island header is separated from its content by a hairline (the suite's
        // choice; Android Studio draws it only under content tabs).
        let mut panel = SimpleToolWindowPanel::new(self.title).header_separator(true);
        if !tabs.is_empty() {
            panel = panel.tabs(&tabs, &mut selected);
        }
        panel
            .hover_actions(true)
            .padded(self.padded)
            .on_hide(|| hide.set(Some(region)))
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
            signals.tab.set(Some((region, selected)));
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
