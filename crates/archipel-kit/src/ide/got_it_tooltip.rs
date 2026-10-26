//! The got-it tooltip.

use crate::host::PropertiesComponent;
use crate::theme::Theme;
use crate::tokens::{motion, radius, size, typography};
use crate::ui::{ActionLink, Button};
use egui::{Align2, Color32, Id, Key, Margin, Modifiers, Pos2, Rect, Shape, Stroke, vec2};

/// Where a [`GotItTooltip`] sits relative to what it points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GotItPosition {
    /// Above the target, centered.
    TopMiddle,
    /// Below the target, centered (the default).
    #[default]
    BottomMiddle,
    /// Left of the target, centered.
    LeftMiddle,
    /// Right of the target, centered.
    RightMiddle,
    /// Below the target, with the bubble's left edge near the target's middle: the arrow
    /// stands 15 pt plus the corner radius from the left corner.
    BottomLeft,
}

/// Why a [`GotItTooltip`] went away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GotItDismissal {
    /// The user pressed the button.
    Button,
    /// The user pressed <kbd>Esc</kbd> (IntelliJ action `CloseGotItTooltip`).
    Escape,
    /// The timeout of [`GotItTooltip::with_timeout`] ran out.
    Timeout,
}

/// What a [`GotItTooltip`] did this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GotItResponse {
    /// The tooltip is on screen.
    pub visible: bool,
    /// The tooltip appeared in this frame: count it with [`GotItTooltip::record_shown`].
    pub newly_shown: bool,
    /// The tooltip went away in this frame: store it with [`GotItTooltip::got_it`], stop
    /// passing `should_show`.
    pub dismissed: Option<GotItDismissal>,
    /// The user clicked the link ([`GotItTooltip::link`]).
    pub link_clicked: bool,
    /// The user clicked the secondary button ([`GotItTooltip::secondary_button`]). The
    /// tooltip stays; dismiss it yourself by no longer showing it.
    pub secondary_clicked: bool,
}

/// An onboarding bubble that points at a control, once: "Got it" tells the application
/// to never show it again. IntelliJ's `GotItTooltip` (IntelliJ spec section 11.2).
///
/// The tooltip is stateless; whether it has been seen is the application's. Ask
/// [`GotItTooltip::can_show`] (backed by a [`PropertiesComponent`], so it survives restarts)
/// or decide yourself, pass the answer as `should_show`, and store the result when
/// [`GotItResponse::dismissed`] arrives:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::host::InMemoryProperties;
/// use archipel_kit::ide::GotItTooltip;
///
/// let mut props = InMemoryProperties::new(); // yours lives in the application state
/// // The rect of the control the tooltip points at.
/// let target = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(60.0, 20.0));
/// let tip = GotItTooltip::new("run.config", "Pick a run configuration here.")
///     .header("Run configurations");
/// let response = tip.show(ui.ctx(), target, tip.can_show(&props));
/// if response.newly_shown { tip.record_shown(&mut props); }
/// if response.dismissed.is_some() { tip.got_it(&mut props); }
/// # });
/// ```
///
/// Or let [`GotItTooltip::show_with`] do the bookkeeping.
///
/// Rules: only one got-it tooltip shows at a time, the others wait for it to go away; it
/// does not close on a click elsewhere; <kbd>Esc</kbd> closes it; with a timeout it has no
/// button and closes by itself (5 s); with [`GotItTooltip::show_count`] it may show a few
/// times before it counts as seen.
#[derive(Debug, Clone)]
pub struct GotItTooltip {
    id: String,
    text: String,
    header: Option<String>,
    step: Option<u32>,
    link: Option<String>,
    button: String,
    secondary: Option<String>,
    timeout: Option<f32>,
    position: GotItPosition,
    max_width: f32,
    show_count: u32,
    focus: bool,
}

/// Per tooltip, kept in the context.
#[derive(Debug, Clone, Copy, Default)]
struct State {
    shown_since: Option<f64>,
    size: Option<egui::Vec2>,
}

/// The tooltip that is on screen, and the pass it was last drawn in.
#[derive(Debug, Clone, Default)]
struct Active {
    id: String,
    pass: u64,
}

impl GotItTooltip {
    /// A tooltip with `text`, identified by `id` (the key of its state in the
    /// [`PropertiesComponent`]).
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            header: None,
            step: None,
            link: None,
            button: "Got it".into(),
            secondary: None,
            timeout: None,
            position: GotItPosition::default(),
            max_width: size::GOT_IT_MAX_WIDTH,
            show_count: 1,
            focus: false,
        }
    }

    /// A bold line above the text.
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// A step number before the text ("01"), for a tour of several tooltips.
    pub fn step(mut self, step: u32) -> Self {
        self.step = Some(step);
        self
    }

    /// A link below the text ([`GotItResponse::link_clicked`]).
    pub fn link(mut self, text: impl Into<String>) -> Self {
        self.link = Some(text.into());
        self
    }

    /// The label of the button (default "Got it").
    pub fn button_label(mut self, label: impl Into<String>) -> Self {
        self.button = label.into();
        self
    }

    /// A second button after the first ([`GotItResponse::secondary_clicked`]).
    pub fn secondary_button(mut self, label: impl Into<String>) -> Self {
        self.secondary = Some(label.into());
        self
    }

    /// Closes the tooltip by itself after [`motion::GOT_IT_TIMEOUT`] (5 s); it then has no
    /// button (IntelliJ `withTimeout`).
    pub fn with_timeout(self) -> Self {
        self.timeout(Some(motion::GOT_IT_TIMEOUT))
    }

    /// Closes the tooltip by itself after `seconds`, or never (`None`, the default).
    pub fn timeout(mut self, seconds: Option<f32>) -> Self {
        self.timeout = seconds;
        self
    }

    /// Where the tooltip sits relative to its target (default: below it).
    pub fn position(mut self, position: GotItPosition) -> Self {
        self.position = position;
        self
    }

    /// Widest the text may be (default 280; IntelliJ `withMaxWidth`).
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// How many times the tooltip may show before it counts as seen when nobody dismisses
    /// it (default 1; IntelliJ `withShowCount`).
    pub fn show_count(mut self, count: u32) -> Self {
        self.show_count = count.max(1);
        self
    }

    /// Moves the keyboard focus to the button when the tooltip appears (IntelliJ
    /// `withFocus`).
    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    fn shown_key(&self) -> String {
        format!("gotit.{}.shown", self.id)
    }

    fn done_key(&self) -> String {
        format!("gotit.{}.done", self.id)
    }

    /// Whether the tooltip may still show: it was neither dismissed nor shown
    /// [`show_count`](GotItTooltip::show_count) times (IntelliJ `canShow`).
    pub fn can_show(&self, props: &dyn PropertiesComponent) -> bool {
        !props.get_bool(&self.done_key(), false)
            && props.get_int(&self.shown_key(), 0) < i64::from(self.show_count)
    }

    /// Counts one showing (see [`GotItResponse::newly_shown`]).
    pub fn record_shown(&self, props: &mut dyn PropertiesComponent) {
        let shown = props.get_int(&self.shown_key(), 0);
        props.set_int(&self.shown_key(), shown + 1);
    }

    /// Remembers that the user has seen the tooltip: [`can_show`](GotItTooltip::can_show)
    /// is false from now on (IntelliJ `gotIt`).
    pub fn got_it(&self, props: &mut dyn PropertiesComponent) {
        props.set_bool(&self.done_key(), true);
    }

    /// Forgets that the tooltip was seen, e.g. for a "reset tips" command.
    pub fn reset(&self, props: &mut dyn PropertiesComponent) {
        props.unset(&self.shown_key());
        props.unset(&self.done_key());
    }

    /// Whether this tooltip is on screen now (it was shown in the last frames and not
    /// dismissed).
    pub fn is_showing(&self, ctx: &egui::Context) -> bool {
        let pass = ctx.cumulative_pass_nr();
        ctx.data(|d| d.get_temp::<State>(Id::new(("archipel-got-it", &self.id))))
            .is_some()
            && ctx
                .data(|d| d.get_temp::<Active>(Id::new("archipel-got-it-active")))
                .is_some_and(|a| a.id == self.id && pass.saturating_sub(a.pass) <= 2)
    }

    /// Shows the tooltip next to `target` while `should_show` is true, and does the
    /// bookkeeping in `props`: counts showings and remembers a dismissal.
    pub fn show_with(
        &self,
        ctx: &egui::Context,
        target: Rect,
        props: &mut dyn PropertiesComponent,
    ) -> GotItResponse {
        // A tooltip already on screen stays until it is dismissed, though showing it has
        // used up its count.
        let allowed = self.can_show(props) || self.is_showing(ctx);
        let response = self.show(ctx, target, allowed);
        if response.newly_shown {
            self.record_shown(props);
        }
        if response.dismissed.is_some() {
            self.got_it(props);
        }
        response
    }

    /// Shows the tooltip next to `target` while `should_show` is true. Call it every frame.
    pub fn show(&self, ctx: &egui::Context, target: Rect, should_show: bool) -> GotItResponse {
        let mut out = GotItResponse::default();
        let state_key = Id::new(("archipel-got-it", &self.id));
        let active_key = Id::new("archipel-got-it-active");
        let pass = ctx.cumulative_pass_nr();
        if !should_show {
            ctx.data_mut(|d| d.remove::<State>(state_key));
            return out;
        }
        // One at a time: another tooltip drawn in the last passes goes first.
        let other_active = ctx
            .data(|d| d.get_temp::<Active>(active_key))
            .is_some_and(|a| a.id != self.id && pass.saturating_sub(a.pass) <= 2);
        if other_active {
            ctx.request_repaint();
            return out;
        }
        ctx.data_mut(|d| {
            d.insert_temp(
                active_key,
                Active {
                    id: self.id.clone(),
                    pass,
                },
            )
        });

        let mut state = ctx
            .data(|d| d.get_temp::<State>(state_key))
            .unwrap_or_default();
        let now = ctx.input(|i| i.time);
        if state.shown_since.is_none() {
            state.shown_since = Some(now);
            out.newly_shown = true;
        }
        let newly_shown = out.newly_shown;
        let colors = Theme::colors(ctx);
        let arrow = size::GOT_IT_ARROW_LENGTH;
        let (point, pivot) = match self.position {
            GotItPosition::TopMiddle => (
                Pos2::new(target.center().x, target.top() - arrow),
                Align2::CENTER_BOTTOM,
            ),
            GotItPosition::BottomMiddle => (
                Pos2::new(target.center().x, target.bottom() + arrow),
                Align2::CENTER_TOP,
            ),
            GotItPosition::LeftMiddle => (
                Pos2::new(target.left() - arrow, target.center().y),
                Align2::RIGHT_CENTER,
            ),
            GotItPosition::RightMiddle => (
                Pos2::new(target.right() + arrow, target.center().y),
                Align2::LEFT_CENTER,
            ),
            GotItPosition::BottomLeft => (
                Pos2::new(
                    target.center().x - (size::GOT_IT_ARROW_SHIFT + f32::from(radius::POPUP)),
                    target.bottom() + arrow,
                ),
                Align2::LEFT_TOP,
            ),
        };
        let frame = egui::Frame::new()
            .fill(colors.got_it)
            .stroke(Stroke::new(1.0_f32, colors.got_it_border))
            .corner_radius(radius::POPUP)
            .shadow(ctx.style().visuals.popup_shadow)
            .inner_margin(Margin::symmetric(
                size::GOT_IT_INSET_X as i8,
                size::GOT_IT_INSET_Y as i8,
            ));
        let area = egui::Area::new(Id::new(("archipel-got-it-area", &self.id)))
            .order(egui::Order::Foreground)
            // The bubble is not a target itself; its buttons and links are.
            .sense(egui::Sense::hover())
            .pivot(pivot)
            .fixed_pos(point)
            .show(ctx, |ui| {
                frame
                    .show(ui, |ui| self.content(ui, &colors, &mut out, newly_shown))
                    .response
            });
        let rect = area.response.rect;
        state.size = Some(rect.size());
        self.paint_arrow(ctx, area.response.layer_id, rect, target, &colors);

        // Escape closes the tooltip; so does a timeout.
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            out.dismissed.get_or_insert(GotItDismissal::Escape);
        }
        if let (Some(timeout), Some(since)) = (self.timeout, state.shown_since) {
            if now - since >= f64::from(timeout) {
                out.dismissed.get_or_insert(GotItDismissal::Timeout);
            } else {
                ctx.request_repaint_after_secs((f64::from(timeout) - (now - since)) as f32);
            }
        }
        out.visible = out.dismissed.is_none();
        if out.dismissed.is_some() {
            ctx.data_mut(|d| {
                d.remove::<State>(state_key);
                d.remove::<Active>(active_key);
            });
        } else {
            ctx.data_mut(|d| d.insert_temp(state_key, state));
        }
        out
    }

    /// The body: step, header, text, link and buttons.
    fn content(
        &self,
        ui: &mut egui::Ui,
        colors: &crate::theme::Colors,
        out: &mut GotItResponse,
        newly_shown: bool,
    ) {
        ui.set_max_width(self.max_width + self.step.map_or(0.0, |_| 24.0));
        ui.spacing_mut().item_spacing = vec2(size::GOT_IT_STEP_GAP, size::GOT_IT_STEP_GAP / 2.0);
        ui.horizontal_top(|ui| {
            if let Some(step) = self.step {
                ui.label(
                    egui::RichText::new(format!("{step:02}"))
                        .font(typography::semibold(ui.ctx(), typography::BODY))
                        .color(colors.text_secondary),
                );
            }
            ui.vertical(|ui| {
                ui.set_max_width(self.max_width);
                if let Some(header) = &self.header {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(header)
                                .font(typography::semibold(ui.ctx(), typography::BODY))
                                .color(colors.text_strong),
                        )
                        .wrap(),
                    );
                }
                ui.add(egui::Label::new(egui::RichText::new(&self.text).color(colors.text)).wrap());
                if let Some(link) = &self.link {
                    let link = ui.add(ActionLink::new(link.as_str()));
                    out.link_clicked |= link.clicked();
                }
            });
        });
        // With a timeout the tooltip has no button.
        if self.timeout.is_some() {
            return;
        }
        ui.add_space(size::GOT_IT_BUTTON_TOP - ui.spacing().item_spacing.y);
        ui.horizontal(|ui| {
            let button = ui.add(Button::secondary(self.button.as_str()).small(true));
            if self.focus && newly_shown {
                button.request_focus();
            }
            if button.clicked() {
                out.dismissed = Some(GotItDismissal::Button);
            }
            if let Some(secondary) = &self.secondary {
                let secondary = ui.add(Button::ghost(secondary.as_str()).small(true));
                out.secondary_clicked |= secondary.clicked();
            }
        });
        ui.add_space(size::GOT_IT_BUTTON_BOTTOM);
    }

    /// Draws the arrow from the bubble to the target.
    fn paint_arrow(
        &self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        rect: Rect,
        target: Rect,
        colors: &crate::theme::Colors,
    ) {
        let painter = ctx.layer_painter(layer);
        let half = size::GOT_IT_ARROW_WIDTH / 2.0;
        let length = size::GOT_IT_ARROW_LENGTH;
        let corner = f32::from(radius::POPUP);
        let along = |value: f32, low: f32, high: f32| {
            value.clamp(
                low + corner + half,
                (high - corner - half).max(low + corner + half),
            )
        };
        // Base midpoint on the bubble's edge, and the tip pointing away from it.
        let (base, direction) = match self.position {
            GotItPosition::TopMiddle => (
                Pos2::new(
                    along(target.center().x, rect.left(), rect.right()),
                    rect.bottom(),
                ),
                vec2(0.0, 1.0),
            ),
            GotItPosition::BottomMiddle | GotItPosition::BottomLeft => (
                Pos2::new(
                    along(target.center().x, rect.left(), rect.right()),
                    rect.top(),
                ),
                vec2(0.0, -1.0),
            ),
            GotItPosition::LeftMiddle => (
                Pos2::new(
                    rect.right(),
                    along(target.center().y, rect.top(), rect.bottom()),
                ),
                vec2(1.0, 0.0),
            ),
            GotItPosition::RightMiddle => (
                Pos2::new(
                    rect.left(),
                    along(target.center().y, rect.top(), rect.bottom()),
                ),
                vec2(-1.0, 0.0),
            ),
        };
        let side = vec2(-direction.y, direction.x) * half;
        let tip = base + direction * length;
        let (a, b) = (base - side, base + side);
        painter.add(Shape::convex_polygon(
            vec![a, tip, b],
            colors.got_it,
            Stroke::NONE,
        ));
        let edge = Stroke::new(1.0_f32, colors.got_it_border);
        painter.line_segment([a, tip], edge);
        painter.line_segment([tip, b], edge);
        // Hide the bubble's border where the arrow joins it.
        let cover: Color32 = colors.got_it;
        painter.line_segment([a, b], Stroke::new(2.0_f32, cover));
    }
}
