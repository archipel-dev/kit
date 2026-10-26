//! A toolbar of action buttons (IntelliJ `ActionToolbar`).

use super::{
    ActionButton, ActionGroup, ActionPlace, ActionPopupMenu, ActionRow, DataContext,
    DefaultActionGroup, InputKind, Keymap, flatten_with_shortcuts, perform_with_modifiers,
};
use crate::theme::Theme;
use crate::tokens::{action_toolbar, motion, size};
use crate::ui::popup::PopupChain;
use crate::{Glyph, Icon};
use egui::{Id, Pos2, Rect, Response, Sense, Stroke, Ui, vec2};

/// The direction a toolbar lays its buttons out in (IntelliJ `SwingConstants.HORIZONTAL`
/// and `VERTICAL` passed to `createActionToolbar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ToolbarOrientation {
    /// Left to right.
    #[default]
    Horizontal,
    /// Top to bottom.
    Vertical,
}

/// What a frame of an [`ActionToolbar`] reported.
#[derive(Debug)]
#[non_exhaustive]
pub struct ActionToolbarResponse {
    /// The toolbar's area.
    pub response: Response,
    /// The id of the action performed this frame (a click on its button, or a choice in a
    /// drop-down or the overflow menu).
    pub invoked: Option<String>,
    /// How many rows did not fit and moved into the overflow menu.
    pub overflowed: usize,
    /// Whether a drop-down, the overflow or the secondary menu is open.
    pub popup_open: bool,
}

/// Which button owns the open menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Item(usize),
    Overflow,
    Secondary,
}

/// What the toolbar remembers between frames, in the egui context.
#[derive(Debug, Clone, Default)]
struct Memory {
    chain: PopupChain,
    owner: Option<Owner>,
    anchor: Option<Rect>,
    /// The owner whose menu closed last, and when: the reopen guard is per button.
    closed: Option<(Owner, f64)>,
}

/// Renders an [`ActionGroup`] as a row (or column) of [`ActionButton`]s for an
/// [`ActionPlace`], updating every action each frame with the [`DataContext`].
///
/// - Actions are 22 pt icon buttons; toggle actions show the pressed look while selected;
///   separators are 1 pt lines; popup groups are buttons with a drop-down arrow that open an
///   [`ActionPopupMenu`] below (beside, when vertical) the button, as a list popup with the
///   first row selected and submenus that open on hover.
/// - **Overflow**: rows that do not fit move into a "More actions" menu at the end; give the
///   toolbar the width it should use (the `Ui`'s available width) and it decides. Turn it off
///   with [`overflow`](Self::overflow).
/// - [`secondary`](Self::secondary) adds a drop-down at the far end (IntelliJ's secondary
///   group: the gear or ellipsis menu).
/// - Clicking performs the action (as [`ActionButton::for_action`] does) and
///   [`ActionToolbarResponse::invoked`] says which. The toolbar is not in the Tab order
///   (see [`ActionButton`]).
/// - Plain toolbars have a 2 pt border; [`decorated`](Self::decorated) toolbars (5, 7).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, action_system::*};
///
/// struct Reload;
/// impl AnAction for Reload {
///     fn id(&self) -> &str { "Reload" }
///     fn template_presentation(&self) -> Presentation {
///         Presentation::new("Reload").icon(Glyph::Refresh)
///     }
///     fn action_performed(&self, _e: &AnActionEvent<'_>) {}
/// }
/// let mut group = DefaultActionGroup::new("Tools");
/// group.add(Reload).add_separator();
/// let context = DataContext::empty();
/// let out = ActionToolbar::new("tools", &group, &context, ActionPlace::TOOLBAR).show(ui);
/// if let Some(id) = out.invoked { let _ = id; }
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct ActionToolbar<'a> {
    id: Id,
    group: &'a dyn ActionGroup,
    context: &'a DataContext,
    place: ActionPlace,
    orientation: ToolbarOrientation,
    keymap: Option<(&'a Keymap, bool)>,
    decorated: bool,
    overflow: bool,
    focusable: Option<bool>,
    secondary: Option<&'a dyn ActionGroup>,
    minimum_button_size: f32,
}

impl<'a> ActionToolbar<'a> {
    /// A toolbar of `group` for `place`; `id_salt` identifies it.
    pub fn new(
        id_salt: impl std::hash::Hash,
        group: &'a dyn ActionGroup,
        context: &'a DataContext,
        place: ActionPlace,
    ) -> Self {
        Self {
            id: Id::new(id_salt),
            group,
            context,
            place,
            orientation: ToolbarOrientation::Horizontal,
            keymap: None,
            decorated: false,
            overflow: true,
            focusable: None,
            secondary: None,
            minimum_button_size: size::ICON_BUTTON,
        }
    }

    /// Horizontal (default) or vertical.
    pub fn orientation(mut self, orientation: ToolbarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Shows each action's first keyboard shortcut from `keymap` in tooltips and menus.
    pub fn keymap(mut self, keymap: &'a Keymap, mac: bool) -> Self {
        self.keymap = Some((keymap, mac));
        self
    }

    /// A decorated toolbar has the New UI border insets (5, 7) instead of 2.
    pub fn decorated(mut self, decorated: bool) -> Self {
        self.decorated = decorated;
        self
    }

    /// Moves rows that do not fit into a "More actions" menu (default on).
    pub fn overflow(mut self, overflow: bool) -> Self {
        self.overflow = overflow;
        self
    }

    /// Whether the buttons are in the Tab order (see [`ActionButton::focusable`]).
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = Some(focusable);
        self
    }

    /// A group shown as a drop-down at the far end of the toolbar (IntelliJ's secondary
    /// actions).
    pub fn secondary(mut self, group: &'a dyn ActionGroup) -> Self {
        self.secondary = Some(group);
        self
    }

    /// The smallest button edge (IntelliJ `setMinimumButtonSize`, 22 by default; the
    /// navigation bar uses 20).
    pub fn minimum_button_size(mut self, edge: f32) -> Self {
        self.minimum_button_size = edge;
        self
    }

    /// Draws the toolbar and reports what was invoked.
    pub fn show(self, ui: &mut Ui) -> ActionToolbarResponse {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let mem_id = self.id.with("archipel-action-toolbar");
        let mut mem: Memory = ctx.data(|d| d.get_temp(mem_id)).unwrap_or_default();
        let default = Keymap::new("none");
        let (keymap, mac) = self.keymap.unwrap_or((&default, false));
        let rows = flatten_with_shortcuts(self.group, self.context, &self.place, keymap, mac);
        let horizontal = self.orientation == ToolbarOrientation::Horizontal;
        let colors = Theme::colors(&ctx);

        // Main axis length of each row, and how many fit.
        let button_edge = self.minimum_button_size.max(size::ICON_BUTTON);
        let gap = size::TOOLBAR_GAP;
        let row_len = |row: &ActionRow| match row {
            ActionRow::Separator(_) => 2.0 * action_toolbar::SEPARATOR_GAP + 1.0,
            ActionRow::Action { .. } => button_edge,
            ActionRow::Group { .. } => button_edge + action_toolbar::ARROW_WIDTH,
        };
        let (inset_x, inset_y) = if self.decorated {
            (
                action_toolbar::DECORATED_INSET_X,
                action_toolbar::DECORATED_INSET_Y,
            )
        } else {
            (action_toolbar::INSET, action_toolbar::INSET)
        };
        let (inset_main, inset_cross) = if horizontal {
            (inset_x, inset_y)
        } else {
            (inset_y, inset_x)
        };
        let available = if horizontal {
            ui.available_width()
        } else {
            ui.available_height()
        } - 2.0 * inset_main;
        let secondary_len = if self.secondary.is_some() {
            gap + button_edge + action_toolbar::ARROW_WIDTH
        } else {
            0.0
        };
        let total: f32 = rows.iter().map(row_len).sum::<f32>()
            + gap * rows.len().saturating_sub(1) as f32
            + secondary_len;
        let mut shown = rows.len();
        if self.overflow && total > available && available.is_finite() {
            let more = gap + button_edge;
            let mut used = secondary_len + more;
            shown = 0;
            for row in &rows {
                let next = used + row_len(row) + if shown > 0 { gap } else { 0.0 };
                if next > available {
                    break;
                }
                used = next;
                shown += 1;
            }
            // Never end on a separator.
            while shown > 0 && rows[shown - 1].is_separator() {
                shown -= 1;
            }
        }
        let overflowed_rows: Vec<&ActionRow> = rows[shown..].iter().collect();
        let overflowed = overflowed_rows
            .iter()
            .filter(|row| !row.is_separator())
            .count();

        let mut invoked: Option<String> = None;
        let mut open_request: Option<(Owner, Rect)> = None;
        let focusable = self.focusable;
        let place = self.place.clone();
        let context = self.context;
        let cross = button_edge + 2.0 * inset_cross;
        let inner = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // The row is as high as its buttons, not as a text row.
            ui.spacing_mut().interact_size = vec2(button_edge, button_edge);
            let body = |ui: &mut Ui| {
                ui.spacing_mut().item_spacing = if horizontal {
                    vec2(gap, 0.0)
                } else {
                    vec2(0.0, gap)
                };
                ui.set_min_height(if horizontal {
                    cross - 2.0 * inset_cross
                } else {
                    0.0
                });
                for (index, row) in rows[..shown].iter().enumerate() {
                    match row {
                        ActionRow::Separator(_) => {
                            let (len, thick) = (action_toolbar::SEPARATOR_LENGTH, 1.0);
                            let span = 2.0 * action_toolbar::SEPARATOR_GAP + thick;
                            let (rect, _) = ui.allocate_exact_size(
                                if horizontal {
                                    vec2(span, button_edge)
                                } else {
                                    vec2(button_edge, span)
                                },
                                Sense::hover(),
                            );
                            let c = rect.center();
                            let stroke = Stroke::new(1.0_f32, colors.separator);
                            if horizontal {
                                ui.painter().vline(
                                    c.x.floor() + 0.5,
                                    (c.y - len / 2.0)..=(c.y + len / 2.0),
                                    stroke,
                                );
                            } else {
                                ui.painter().hline(
                                    (c.x - len / 2.0)..=(c.x + len / 2.0),
                                    c.y.floor() + 0.5,
                                    stroke,
                                );
                            }
                        }
                        ActionRow::Action {
                            action,
                            presentation,
                        } => {
                            let mut button = ActionButton::from_presentation(presentation);
                            if let Some(focusable) = focusable {
                                button = button.focusable(focusable);
                            }
                            let out = button.show(ui);
                            if out.activated
                                && let Some(id) = perform_from_toolbar(
                                    ui,
                                    action.as_ref(),
                                    context,
                                    &place,
                                    &out.response,
                                )
                            {
                                invoked = Some(id);
                            }
                        }
                        ActionRow::Group { presentation, .. } => {
                            let mut button =
                                ActionButton::from_presentation(presentation).dropdown(true);
                            if let Some(focusable) = focusable {
                                button = button.focusable(focusable);
                            }
                            let out = button.show(ui);
                            if out.activated && presentation.enabled {
                                open_request = Some((Owner::Item(index), out.response.rect));
                            }
                        }
                    }
                }
                if !overflowed_rows.is_empty() {
                    let mut button = ActionButton::new(Icon::Glyph(Glyph::More), "More actions");
                    if let Some(focusable) = focusable {
                        button = button.focusable(focusable);
                    }
                    let out = button.show(ui);
                    if out.activated {
                        open_request = Some((Owner::Overflow, out.response.rect));
                    }
                }
                if self.secondary.is_some() {
                    let mut button =
                        ActionButton::new(Icon::Glyph(Glyph::Settings), "Options").dropdown(true);
                    if let Some(focusable) = focusable {
                        button = button.focusable(focusable);
                    }
                    let out = button.show(ui);
                    if out.activated {
                        open_request = Some((Owner::Secondary, out.response.rect));
                    }
                }
            };
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(inset_x as i8, inset_y as i8))
                .show(ui, |ui| {
                    if horizontal {
                        ui.horizontal(body);
                    } else {
                        ui.vertical(body);
                    }
                });
        });
        let response = inner.response;

        // Opening a menu: closing the same button's menu just now is not undone by the
        // release of the click that closed it.
        if let Some((owner, anchor)) = open_request {
            let guarded = mem.closed.is_some_and(|(closed_owner, at)| {
                closed_owner == owner && now - at < f64::from(motion::POPUP_REOPEN_GUARD)
            });
            if !guarded && !(mem.chain.is_open() && mem.owner == Some(owner)) {
                mem.chain = PopupChain::default();
                mem.chain.open(now, true);
                mem.owner = Some(owner);
                mem.anchor = Some(anchor);
            }
        }

        // The open menu.
        let was_open = mem.chain.is_open();
        if let (true, Some(owner), Some(anchor)) = (was_open, mem.owner, mem.anchor) {
            let popup_id = mem_id.with("menu");
            let output = match owner {
                Owner::Item(index) => match rows.get(index) {
                    Some(ActionRow::Group { group, .. }) => {
                        let mut menu = ActionPopupMenu::new(
                            popup_id,
                            group.as_ref(),
                            self.context,
                            self.place.clone(),
                        );
                        if let Some((keymap, mac)) = self.keymap {
                            menu = menu.keymap(keymap, mac);
                        }
                        Some(show_menu(menu, &ctx, &mut mem.chain, anchor, horizontal))
                    }
                    _ => {
                        mem.chain.close(now);
                        None
                    }
                },
                Owner::Overflow => {
                    let mut group = DefaultActionGroup::new("More");
                    for row in &overflowed_rows {
                        match row {
                            ActionRow::Action { action, .. } => {
                                group.add_rc(action.clone());
                            }
                            ActionRow::Group { group: sub, .. } => {
                                group.add_group_rc(sub.clone());
                            }
                            ActionRow::Separator(separator) => match &separator.text {
                                Some(text) => {
                                    group.add_separator_with_text(text.clone());
                                }
                                None => {
                                    group.add_separator();
                                }
                            },
                        }
                    }
                    let mut menu =
                        ActionPopupMenu::new(popup_id, &group, self.context, self.place.clone());
                    if let Some((keymap, mac)) = self.keymap {
                        menu = menu.keymap(keymap, mac);
                    }
                    Some(show_menu(menu, &ctx, &mut mem.chain, anchor, horizontal))
                }
                Owner::Secondary => match self.secondary {
                    Some(group) => {
                        let mut menu =
                            ActionPopupMenu::new(popup_id, group, self.context, self.place.clone());
                        if let Some((keymap, mac)) = self.keymap {
                            menu = menu.keymap(keymap, mac);
                        }
                        Some(show_menu(menu, &ctx, &mut mem.chain, anchor, horizontal))
                    }
                    None => {
                        mem.chain.close(now);
                        None
                    }
                },
            };
            if let Some(performed) = output.and_then(|out| out.performed) {
                invoked = Some(performed);
            }
            if !mem.chain.is_open() {
                mem.closed = Some((owner, now));
                mem.owner = None;
            }
        }
        let popup_open = mem.chain.is_open();
        ctx.data_mut(|d| d.insert_temp(mem_id, mem));
        ActionToolbarResponse {
            response,
            invoked,
            overflowed,
            popup_open,
        }
    }
}

/// Performs `action` for a click on its toolbar button; returns its id when it ran.
fn perform_from_toolbar(
    ui: &Ui,
    action: &dyn super::AnAction,
    context: &DataContext,
    place: &ActionPlace,
    _response: &Response,
) -> Option<String> {
    let (input, modifiers) = ui.input(|i| {
        (
            if i.pointer.any_released() {
                InputKind::Mouse
            } else {
                InputKind::Keyboard
            },
            i.modifiers,
        )
    });
    perform_with_modifiers(action, context, place, input, modifiers).then(|| action.id().to_owned())
}

fn show_menu(
    menu: ActionPopupMenu<'_>,
    ctx: &egui::Context,
    chain: &mut PopupChain,
    anchor: Rect,
    horizontal: bool,
) -> super::ActionPopupMenuResponse {
    if horizontal {
        menu.show(ctx, chain, anchor)
    } else {
        let pos: Pos2 = anchor.right_top();
        menu.show_at(ctx, chain, pos)
    }
}
