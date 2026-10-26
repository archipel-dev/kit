//! The status bar: a container of [`StatusBarWidget`]s with anchor ordering, widget
//! presentations, hover and press effects and click reporting.

use crate::action_system::ActionMenuItem;
use crate::common;
use crate::icons::Icon;
use crate::parts::{Surface, Tone};
use crate::theme::Theme;
use crate::tokens::{size, typography};
use crate::wm::StatusBarWidget;
use egui::{Align, Id, Key, Layout, Pos2, Rect, Sense, Ui, UiBuilder, pos2, vec2};

/// Where a widget goes among the others of its panel (IntelliJ `StatusBar.Anchors`:
/// `before(id)`, `after(id)`, `DEFAULT_ANCHOR` = last, `first`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum StatusBarAnchor {
    /// At the start of the panel.
    First,
    /// At the end of the panel (the default).
    #[default]
    Last,
    /// Right before the widget with this id (at the end if there is none).
    Before(String),
    /// Right after the widget with this id (at the end if there is none).
    After(String),
}

/// A text widget (IntelliJ `StatusBarWidget.TextPresentation`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextPresentation {
    /// The text. Longer than the room it has, it is cut off with an ellipsis and shown in
    /// full as the tooltip.
    pub text: String,
    /// The tooltip (`getTooltipText`).
    pub tooltip: Option<String>,
    /// Where the text sits in the width the widget has beyond it, `0.0` (left) to `1.0`
    /// (right) (`getAlignment`).
    pub alignment: f32,
    /// A prototype that reserves the width (`getMaxValue`), so a changing value such as a
    /// line and column does not move the widgets beside it.
    pub max_value: Option<String>,
}

impl TextPresentation {
    /// A widget reading `text`, aligned left.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tooltip: None,
            alignment: 0.0,
            max_value: None,
        }
    }

    /// Sets the tooltip.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Sets the alignment (0 to 1).
    #[must_use]
    pub fn alignment(mut self, alignment: f32) -> Self {
        self.alignment = alignment.clamp(0.0, 1.0);
        self
    }

    /// Reserves the width of `prototype`.
    #[must_use]
    pub fn max_value(mut self, prototype: impl Into<String>) -> Self {
        self.max_value = Some(prototype.into());
        self
    }
}

/// An icon widget (IntelliJ `StatusBarWidget.IconPresentation`).
#[derive(Debug, Clone, PartialEq)]
pub struct IconPresentation {
    /// The icon.
    pub icon: Icon,
    /// The tooltip, which also names the widget for screen readers.
    pub tooltip: String,
}

impl IconPresentation {
    /// An icon widget named `tooltip`.
    pub fn new(icon: impl Into<Icon>, tooltip: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            tooltip: tooltip.into(),
        }
    }
}

/// A text widget that opens a list of values (IntelliJ
/// `StatusBarWidget.MultipleTextValuesPresentation`): the line separator, the encoding. The
/// list opens above it; picking a value is reported in [`StatusBarResponse::selected_value`].
#[derive(Debug, Clone, PartialEq)]
pub struct MultipleTextValuesPresentation {
    /// The values, in list order.
    pub values: Vec<String>,
    /// The index of the current value, shown as the widget's text and highlighted in the
    /// list.
    pub selected: usize,
    /// The tooltip (`getTooltipText`).
    pub tooltip: Option<String>,
}

impl MultipleTextValuesPresentation {
    /// A widget showing `values[selected]`.
    pub fn new(values: Vec<String>, selected: usize) -> Self {
        Self {
            values,
            selected,
            tooltip: None,
        }
    }

    /// Sets the tooltip.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// How a widget looks and what a click on it does.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WidgetPresentation {
    /// Text.
    Text(TextPresentation),
    /// An icon.
    Icon(IconPresentation),
    /// Text that opens a list of values.
    MultipleTextValues(MultipleTextValuesPresentation),
}

impl From<TextPresentation> for WidgetPresentation {
    fn from(presentation: TextPresentation) -> Self {
        Self::Text(presentation)
    }
}

impl From<IconPresentation> for WidgetPresentation {
    fn from(presentation: IconPresentation) -> Self {
        Self::Icon(presentation)
    }
}

impl From<MultipleTextValuesPresentation> for WidgetPresentation {
    fn from(presentation: MultipleTextValuesPresentation) -> Self {
        Self::MultipleTextValues(presentation)
    }
}

/// One widget of a [`StatusBar`]: an id, its presentation and its place among the others.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusBarWidgetItem {
    id: String,
    presentation: WidgetPresentation,
    anchor: StatusBarAnchor,
    enabled: bool,
    tone: Tone,
}

impl StatusBarWidgetItem {
    /// A widget `id` (IntelliJ `StatusBarWidget.ID`) with `presentation`.
    pub fn new(id: impl Into<String>, presentation: impl Into<WidgetPresentation>) -> Self {
        Self {
            id: id.into(),
            presentation: presentation.into(),
            anchor: StatusBarAnchor::Last,
            enabled: true,
            tone: Tone::Default,
        }
    }

    /// Where it goes among the widgets of its panel.
    #[must_use]
    pub fn anchor(mut self, anchor: StatusBarAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Disabled widgets are dimmed and ignore clicks.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Meaning of the text and icon color, e.g. [`Tone::Error`].
    #[must_use]
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// What the user did in a [`StatusBar`] this frame.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StatusBarResponse {
    /// The bar's area.
    pub rect: Rect,
    /// The id of the widget the user activated: a press of the left button on it, or
    /// <kbd>Enter</kbd> or <kbd>Space</kbd> on it when focused. Widgets with a list of
    /// values open it too.
    pub activated: Option<String>,
    /// The id of a [`MultipleTextValuesPresentation`] widget and the index of the value
    /// picked from its list.
    pub selected_value: Option<(String, usize)>,
    /// A right click on the right panel, and where: show the menu that enables and disables
    /// widgets (IntelliJ's status bar widgets popup).
    pub widgets_menu: Option<Pos2>,
    /// <kbd>Esc</kbd> was pressed with a widget focused: the bar let go of the focus, give
    /// it back to the component that had it before.
    pub escaped: bool,
}

/// The status bar: info text and widgets of the left panel, then (at the right end) the
/// widgets of the right panel in anchor order.
///
/// IntelliJ counterpart: `com.intellij.openapi.wm.StatusBar` with its widget panels (spec
/// section 9). The bar is stateless: the caller owns the widgets and their values and gets
/// clicks back.
///
/// - Only the right panel's widgets have the hover and press effect (a rounded fill, arc
///   4, the same for both); left widgets have none.
/// - A left click acts on **press**, not on release.
/// - A right click on the right panel is reported as a request for the widgets menu.
/// - <kbd>←</kbd> and <kbd>→</kbd> move the focus between the widgets; <kbd>Esc</kbd>
///   leaves the bar (reported, and the bar lets go of the focus).
/// - The bar takes the available width and no outer padding: the window frame supplies it
///   ([`ToolWindowPane::status`](crate::wm::ToolWindowPane::status) insets 8 pt).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::wm::{
///     StatusBar, StatusBarAnchor, StatusBarWidgetItem, TextPresentation,
///     MultipleTextValuesPresentation, IconPresentation,
/// };
/// use archipel_kit::Glyph;
///
/// let out = StatusBar::new("status")
///     .info("Indexing finished")
///     .widget(StatusBarWidgetItem::new(
///         "position",
///         TextPresentation::new("12:8").max_value("9999:99").alignment(1.0),
///     ))
///     .widget(StatusBarWidgetItem::new(
///         "encoding",
///         MultipleTextValuesPresentation::new(vec!["UTF-8".into(), "UTF-16".into()], 0),
///     ).anchor(StatusBarAnchor::Last))
///     .widget(StatusBarWidgetItem::new(
///         "notifications",
///         IconPresentation::new(Glyph::Bell, "Notifications"),
///     ).anchor(StatusBarAnchor::Before("encoding".into())))
///     .show(ui);
/// if let Some((id, index)) = out.selected_value { /* switch `id` to value `index` */ }
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct StatusBar {
    id: Id,
    info: Option<String>,
    left: Vec<StatusBarWidgetItem>,
    right: Vec<StatusBarWidgetItem>,
}

impl StatusBar {
    /// A bar; `id_salt` tells bars apart.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(id_salt),
            info: None,
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    /// The info text of the center (IntelliJ `StatusBar.setInfo`): what the pointer is on,
    /// the last message.
    pub fn info(mut self, text: impl Into<String>) -> Self {
        self.info = Some(text.into());
        self
    }

    /// Adds a widget to the right panel (IntelliJ `addWidget` with an anchor): the panel
    /// with the hover effect.
    pub fn widget(mut self, widget: StatusBarWidgetItem) -> Self {
        self.right.push(widget);
        self
    }

    /// Adds a widget to the left panel (IntelliJ `addWidget(widget)` without a panel):
    /// no hover effect.
    pub fn left_widget(mut self, widget: StatusBarWidgetItem) -> Self {
        self.left.push(widget);
        self
    }

    /// Draws the bar in the width of `ui`, as high as `ui` allows (at most
    /// [`size::STATUS_BAR_HEIGHT`]).
    pub fn show(self, ui: &mut Ui) -> StatusBarResponse {
        let colors = Theme::colors(ui.ctx());
        let width = if ui.available_width().is_finite() {
            ui.available_width()
        } else {
            size::STATUS_BAR_HEIGHT * 20.0
        };
        let height = ui
            .available_height()
            .min(crate::tokens::metrics(ui.ctx()).status_bar_height);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let mut out = StatusBarResponse {
            rect,
            activated: None,
            selected_value: None,
            widgets_menu: None,
            escaped: false,
        };
        let left = order(self.left);
        let right = order(self.right);
        let mut ids: Vec<Id> = Vec::new();
        let (primary_pressed, secondary_pressed) = ui.input(|i| {
            (
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.button_pressed(egui::PointerButton::Secondary),
            )
        });
        let enter_or_space = ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        let mut pending: Vec<(String, Rect, Vec<String>, usize)> = Vec::new();

        // The left panel first, so Tab visits it first. It leaves room for the right panel
        // as it was last frame.
        let right_key = self.id.with("right-width");
        let right_last: f32 = ui.data(|d| d.get_temp(right_key)).unwrap_or(0.0);
        let left_rect = Rect::from_min_max(
            rect.min,
            pos2((rect.right() - right_last).max(rect.left()), rect.max.y),
        );
        let mut left_ui = ui.new_child(
            UiBuilder::new()
                .id_salt(self.id.with("left"))
                .max_rect(left_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        left_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        left_ui.spacing_mut().item_spacing.x = 0.0;
        let mut draw = |ui: &mut Ui, item: &StatusBarWidgetItem, hover_effect: bool| {
            let (widget, values, selected) = widget_of(item, hover_effect);
            let response = ui.add(widget);
            ids.push(response.id);
            let acted = item.enabled
                && ((primary_pressed && response.hovered())
                    || (response.has_focus() && enter_or_space));
            if acted {
                out.activated = Some(item.id.clone());
                if values.is_some() {
                    let key = self.id.with(("open", &item.id));
                    let open: bool = ui.data(|d| d.get_temp(key)).unwrap_or(false);
                    ui.data_mut(|d| d.insert_temp(key, !open));
                }
            }
            if let Some(values) = values {
                let key = self.id.with(("open", &item.id));
                if ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false) {
                    pending.push((item.id.clone(), response.rect, values, selected));
                }
            }
        };
        for item in &left {
            draw(&mut left_ui, item, false);
        }
        let left_end = left_ui.min_rect().right();

        // The right panel, right to left.
        let mut right_ui = ui.new_child(
            UiBuilder::new()
                .id_salt(self.id.with("right"))
                .max_rect(Rect::from_min_max(
                    pos2(left_end.min(rect.right()), rect.top()),
                    rect.max,
                ))
                .layout(Layout::right_to_left(Align::Center)),
        );
        right_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        right_ui.spacing_mut().item_spacing.x = 0.0;
        // Right to left: the last widget is added first. Their ids are collected in
        // reading order afterwards.
        let before = left.len();
        for item in right.iter().rev() {
            draw(&mut right_ui, item, true);
        }
        ids[before..].reverse();
        let right_rect = right_ui.min_rect();
        ui.data_mut(|d| d.insert_temp(right_key, right_rect.width()));

        // The info text fills what is left between the panels.
        if let Some(info) = &self.info {
            let from = left_end + size::STATUS_ITEM_PADDING;
            let to = right_rect.left() - size::STATUS_ITEM_PADDING;
            if to > from {
                let font = typography::body(typography::BODY);
                let galley = common::galley_elided(ui, info, font, to - from);
                let area = Rect::from_min_max(pos2(from, rect.top()), pos2(to, rect.bottom()));
                common::text_left(
                    &ui.painter().with_clip_rect(area),
                    area,
                    from,
                    galley,
                    colors.text_secondary,
                );
            }
        }

        // A right click anywhere on the right panel asks for the widgets menu.
        if secondary_pressed
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            && right_rect
                .expand2(vec2(size::STATUS_ITEM_PADDING, 0.0))
                .contains(pos)
            && ui.rect_contains_pointer(right_rect)
        {
            out.widgets_menu = Some(pos);
        }

        // Arrow keys move the focus between the widgets; Esc leaves the bar. egui drops the
        // focus at the start of the frame an Esc arrives in, so that frame uses the widget
        // that had it in the last one.
        let memo = self.id.with("focused-widget");
        let escape_now = ui.input(|i| i.key_pressed(Key::Escape));
        let focused = ui.memory(|m| m.focused());
        let position = if escape_now {
            ui.data(|d| d.get_temp::<Option<usize>>(memo)).flatten()
        } else {
            let position = focused.and_then(|focus| ids.iter().position(|id| *id == focus));
            ui.data_mut(|d| d.insert_temp(memo, position));
            position
        };
        if let Some(position) = position.filter(|&position| position < ids.len()) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    ids[position],
                    egui::EventFilter {
                        horizontal_arrows: true,
                        ..egui::EventFilter::default()
                    },
                );
            });
            let press = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
            let step = if press(Key::ArrowRight) {
                Some(1isize)
            } else if press(Key::ArrowLeft) {
                Some(-1)
            } else {
                None
            };
            if let Some(step) = step {
                let next = position as isize + step;
                if (0..ids.len() as isize).contains(&next) {
                    ui.memory_mut(|m| m.request_focus(ids[next as usize]));
                }
            }
            if escape_now {
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
                ui.memory_mut(|m| m.surrender_focus(ids[position]));
                ui.data_mut(|d| d.insert_temp(memo, None::<usize>));
                out.escaped = true;
            }
        }

        // The list of a widget with several values, above it.
        for (id, anchor, values, selected) in pending {
            let key = self.id.with(("open", &id));
            let mut open = true;
            let width = values
                .iter()
                .map(|value| {
                    common::galley(ui, value.as_str(), typography::body(typography::BODY))
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
                + 2.0
                    * (crate::tokens::metrics(ui.ctx()).popup_item_inset
                        + crate::tokens::metrics(ui.ctx()).popup_item_padding);
            let mut picked = None;
            above_popup(
                ui.ctx(),
                self.id.with(("popup", &id)),
                anchor,
                width.max(anchor.width()),
                &mut open,
                |ui| {
                    for (index, value) in values.iter().enumerate() {
                        if ui
                            .add(ActionMenuItem::new(value.as_str()).highlighted(index == selected))
                            .clicked()
                        {
                            picked = Some(index);
                        }
                    }
                },
            );
            if let Some(index) = picked {
                out.selected_value = Some((id, index));
                open = false;
            }
            if !open {
                ui.data_mut(|d| d.insert_temp(key, false));
            }
        }
        out
    }
}

/// The widget for `item`, and its values and selected index when it opens a list.
fn widget_of(
    item: &StatusBarWidgetItem,
    hover_effect: bool,
) -> (StatusBarWidget<'static>, Option<Vec<String>>, usize) {
    let base = |text: &str| {
        StatusBarWidget::new(text)
            .tone(item.tone)
            .enabled(item.enabled)
            .hover_effect(hover_effect)
    };
    match &item.presentation {
        WidgetPresentation::Text(text) => {
            let mut widget = base(&text.text).text_alignment(text.alignment);
            if let Some(prototype) = &text.max_value {
                widget = widget.reserve_width_of(prototype.clone());
            }
            if let Some(tooltip) = &text.tooltip {
                widget = widget.with(crate::ide::HelpTooltip::new(tooltip.clone()));
            }
            (widget, None, 0)
        }
        WidgetPresentation::Icon(icon) => {
            (base("").icon(icon.icon).name(icon.tooltip.clone()), None, 0)
        }
        WidgetPresentation::MultipleTextValues(values) => {
            let current = values
                .values
                .get(values.selected)
                .map_or("", String::as_str);
            let mut widget = base(current).chevron(true);
            if let Some(tooltip) = &values.tooltip {
                widget = widget.with(crate::ide::HelpTooltip::new(tooltip.clone()));
            }
            (widget, Some(values.values.clone()), values.selected)
        }
    }
}

/// The widgets in the order their anchors ask for. A widget whose anchor names an id that is
/// not there (yet) waits for the others, and goes last when none turns up.
fn order(items: Vec<StatusBarWidgetItem>) -> Vec<StatusBarWidgetItem> {
    let mut placed: Vec<StatusBarWidgetItem> = Vec::new();
    let mut waiting = items;
    loop {
        let mut progress = false;
        let mut still_waiting = Vec::new();
        for item in waiting {
            let position = match &item.anchor {
                StatusBarAnchor::First => Some(0),
                StatusBarAnchor::Last => Some(placed.len()),
                StatusBarAnchor::Before(id) => placed.iter().position(|other| &other.id == id),
                StatusBarAnchor::After(id) => placed
                    .iter()
                    .position(|other| &other.id == id)
                    .map(|index| index + 1),
            };
            match position {
                Some(index) => {
                    placed.insert(index, item);
                    progress = true;
                }
                None => still_waiting.push(item),
            }
        }
        waiting = still_waiting;
        if waiting.is_empty() || !progress {
            break;
        }
    }
    placed.extend(waiting);
    placed
}

/// A popup that opens above `anchor` (a status bar sits at the window's bottom edge).
fn above_popup<R>(
    ctx: &egui::Context,
    id: Id,
    anchor: Rect,
    width: f32,
    open: &mut bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if !*open {
        return None;
    }
    let last_height: f32 = ctx.data(|d| d.get_temp(id)).unwrap_or(0.0);
    let top = (anchor.top() - last_height - size::POPUP_PADDING).max(ctx.content_rect().top());
    let shown = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(anchor.left(), top))
        .show(ctx, |ui| {
            Surface::Popup.frame(ctx).show(ui, |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                add(ui)
            })
        });
    let rect = shown.response.rect;
    ctx.data_mut(|d| d.insert_temp(id, rect.height()));
    let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
    let clicked_outside = ctx.input(|i| {
        i.pointer.any_click()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !rect.contains(p) && !anchor.contains(p))
    });
    if escape || clicked_outside {
        *open = false;
    }
    Some(shown.inner.inner)
}
