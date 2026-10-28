//! [`ActionToolbar`] and [`ActionButton`]: an action group as a row of icon buttons that
//! perform their actions through the action system, with the GUI's states (hover, pressed,
//! toggled, disabled), tooltips with the shortcut, the description in the status bar while
//! hovered, drop-down groups and an overflow button.
//!
//! The toolbar is not in the focus order (as in IntelliJ): the keyboard reaches its actions
//! through their shortcuts in the [`Keymap`](archipel_kit::action_system::Keymap).

use crate::components::{fill, put, text_width};
use crate::hit::Id;
use crate::input::{Event, MouseButton};
use crate::pointer::PointerKind;
use crate::popup::{ActionPopupMenu, Placement};
use crate::runtime::Runtime;
use crate::tooltip::Tooltip;
use archipel_kit::action_system::{
    ActionGroup, ActionPlace, ActionRow, AnAction, DataContext, DefaultActionGroup, InputKind,
    flatten_with_shortcuts, perform_with_modifiers,
};
use archipel_kit::egui::Modifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::rc::Rc;

pub use crate::bar::ToolbarControl;

/// What a toolbar did.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ToolbarEvent {
    /// Nothing for the caller.
    None,
    /// An action ran; its id.
    Performed(String),
    /// A drop-down group was clicked: open a popup menu below `anchor` (see
    /// [`ActionToolbar::popup_for`]).
    OpenGroup {
        /// The index of the group among the toolbar's rows.
        index: usize,
        /// The button.
        anchor: Rect,
    },
    /// The overflow button was clicked: open [`ActionToolbar::overflow_menu`] below `anchor`.
    OpenOverflow {
        /// The button.
        anchor: Rect,
    },
}

/// The hover and press state of a toolbar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ToolbarState {
    /// The button the left mouse button is down on.
    pub pressed: Option<usize>,
}

/// How a toolbar lays out its buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ToolbarOrientation {
    /// Left to right.
    #[default]
    Horizontal,
    /// Top to bottom.
    Vertical,
}

/// An action group shown as a toolbar.
#[derive(Clone)]
pub struct ActionToolbar {
    /// The identity; buttons use child ids.
    pub id: Id,
    group: Rc<dyn ActionGroup>,
    data: Rc<DataContext>,
    place: ActionPlace,
    /// Show the action text next to the icon.
    pub show_text: bool,
    /// The layout direction.
    pub orientation: ToolbarOrientation,
    /// The background the toolbar sits on.
    pub on: Option<ratatui::style::Color>,
    rows: Vec<ActionRow>,
    rects: Vec<(usize, Rect)>,
    overflow: Option<(Rect, usize)>,
}

impl ActionToolbar {
    /// A toolbar of `group` for `place` with the data of `data`.
    pub fn new(
        id: Id,
        group: Rc<dyn ActionGroup>,
        data: Rc<DataContext>,
        place: ActionPlace,
    ) -> Self {
        Self {
            id,
            group,
            data,
            place,
            show_text: false,
            orientation: ToolbarOrientation::Horizontal,
            on: None,
            rows: Vec::new(),
            rects: Vec::new(),
            overflow: None,
        }
    }

    /// Shows the action text next to the icons.
    #[must_use]
    pub fn show_text(mut self, show: bool) -> Self {
        self.show_text = show;
        self
    }

    /// Sets the orientation.
    #[must_use]
    pub fn orientation(mut self, orientation: ToolbarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the background it sits on.
    #[must_use]
    pub fn on(mut self, color: ratatui::style::Color) -> Self {
        self.on = Some(color);
        self
    }

    fn button_id(&self, index: usize) -> Id {
        self.id.with(index as u64)
    }

    fn overflow_id(&self) -> Id {
        self.id.with(0xF10)
    }

    /// The rows as of the last [`show`](Self::show) (updated actions).
    pub fn rows(&self) -> &[ActionRow] {
        &self.rows
    }

    fn button_text(&self, row: &ActionRow) -> Option<(String, Option<crate::components::RowIcon>)> {
        let presentation = row.presentation()?;
        let icon = presentation.icon.as_ref().and_then(icon_of);
        Some((presentation.text.clone(), icon))
    }

    fn width_of(&self, row: &ActionRow) -> u16 {
        match row {
            ActionRow::Separator(_) => 1,
            _ => {
                let Some((text, icon)) = self.button_text(row) else {
                    return 0;
                };
                let group = matches!(row, ActionRow::Group { .. });
                let has_text = self.show_text || icon.is_none();
                let icon_w = match (icon.is_some(), has_text) {
                    (true, true) => 2,
                    (true, false) => 1,
                    _ => 0,
                };
                let label_w = if has_text { text_width(&text) } else { 0 };
                2 + icon_w + label_w + if group { 2 } else { 0 }
            }
        }
    }

    /// Draws the toolbar in `area` (one row for horizontal, one column's rows for vertical)
    /// and registers its buttons.
    pub fn show(&mut self, rt: &mut Runtime, state: &ToolbarState, area: Rect, buf: &mut Buffer) {
        let mac = rt.caps.os.is_mac();
        self.rows = flatten_with_shortcuts(
            self.group.as_ref(),
            &self.data,
            &self.place,
            &rt.keymap,
            mac,
        );
        self.rects.clear();
        self.overflow = None;
        let p = rt.look.palette;
        let under = self.on.unwrap_or(p.black);
        let horizontal = self.orientation == ToolbarOrientation::Horizontal;
        fill_rect_area(buf, area, under);
        let more_w = 3u16;
        let total: u16 = self.rows.iter().map(|r| self.width_of(r)).sum();
        let limit = if horizontal { area.width } else { area.height };
        let overflowing = total > limit;
        let budget = if overflowing {
            limit.saturating_sub(more_w)
        } else {
            limit
        };
        let mut used = 0u16;
        let mut first_hidden = None;
        for (i, row) in self.rows.iter().enumerate() {
            let w = if horizontal { self.width_of(row) } else { 1 };
            if used + w > budget {
                first_hidden = Some(i);
                break;
            }
            let rect = if horizontal {
                Rect::new(area.x + used, area.y, w, 1)
            } else {
                Rect::new(
                    area.x,
                    area.y + used,
                    area.width.min(self.width_of(row).max(3)),
                    1,
                )
            };
            used += w;
            self.rects.push((i, rect));
        }
        let hovered = rt.hovered();
        let rows = self.rows.clone();
        for (i, rect) in self.rects.clone() {
            let id = self.button_id(i);
            match &rows[i] {
                ActionRow::Separator(_) => {
                    let (x, y) = (rect.x, rect.y);
                    put(
                        buf,
                        rect,
                        x,
                        y,
                        if horizontal { "│" } else { "─" },
                        Style::default().fg(p.line).bg(under),
                    );
                }
                row => {
                    let Some(presentation) = row.presentation() else {
                        continue;
                    };
                    let hover = hovered == Some(id) && presentation.enabled;
                    let pressed = state.pressed == Some(i) && hover;
                    let toggled = presentation.is_selected();
                    let (bg, fg) = if !presentation.enabled {
                        (under, p.grey_fg)
                    } else if pressed {
                        (p.one_bg3, p.white)
                    } else if toggled {
                        (p.one_bg2, p.nord_blue)
                    } else if hover {
                        (p.one_bg, p.white)
                    } else {
                        (under, p.light_grey)
                    };
                    fill(
                        buf,
                        rect,
                        rect.x,
                        rect.y,
                        rect.width,
                        Style::default().bg(bg),
                    );
                    let mut x = rect.x + 1;
                    let icon = presentation.icon.as_ref().and_then(icon_of);
                    if let Some(icon) = icon {
                        let (ch, color) = icon.char_and_color(&rt.look);
                        let color = if presentation.enabled && !toggled && !hover {
                            color
                        } else {
                            fg
                        };
                        put(
                            buf,
                            rect,
                            x,
                            rect.y,
                            &ch.to_string(),
                            Style::default().fg(color).bg(bg),
                        );
                        x += if self.show_text { 2 } else { 1 };
                    }
                    if self.show_text || icon.is_none() {
                        x = put(
                            buf,
                            rect,
                            x,
                            rect.y,
                            &presentation.text,
                            Style::default().fg(fg).bg(bg),
                        ) + 1;
                    }
                    if matches!(row, ActionRow::Group { .. }) {
                        put(
                            buf,
                            rect,
                            rect.right().saturating_sub(2),
                            rect.y,
                            &rt.look.glyph(crate::Glyph::ChevronDown),
                            Style::default()
                                .fg(fg)
                                .bg(bg)
                                .add_modifier(Modifier::empty()),
                        );
                    }
                    let _ = x;
                    if presentation.enabled {
                        rt.clickable(id, rect);
                        let mut tip =
                            Tooltip::new(presentation.text.trim_end_matches('…').to_owned());
                        if let ActionRow::Action { action, .. } = row
                            && let Some(sc) = rt.shortcut_text(action.id())
                        {
                            tip = tip.shortcut(sc);
                        }
                        if !presentation.description.is_empty() {
                            tip = tip.description(presentation.description.clone());
                        }
                        rt.tooltip.offer(id, rect, tip);
                        if hover && !presentation.description.is_empty() {
                            rt.hint = Some(presentation.description.clone());
                        }
                    } else if let Some(reason) = &presentation.disabled_reason {
                        rt.tooltip.offer(id, rect, Tooltip::new(reason.clone()));
                        rt.clickable(id, rect);
                    }
                }
            }
        }
        if let Some(first) = first_hidden {
            let rect = if horizontal {
                Rect::new(area.x + used, area.y, more_w, 1)
            } else {
                Rect::new(area.x, area.y + used, 3.min(area.width), 1)
            };
            let hover = hovered == Some(self.overflow_id());
            let bg = if hover { p.one_bg } else { under };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            put(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                &rt.look.glyph(crate::Glyph::More),
                Style::default().fg(p.light_grey).bg(bg),
            );
            rt.clickable(self.overflow_id(), rect);
            self.overflow = Some((rect, first));
        }
    }

    /// Applies `event`: a click on a button performs its action (on release inside the
    /// button, left button only).
    pub fn handle(
        &mut self,
        rt: &mut Runtime,
        state: &mut ToolbarState,
        event: &Event,
    ) -> ToolbarEvent {
        let Event::Mouse(_) = event else {
            return ToolbarEvent::None;
        };
        let Some(p) = rt.pointer().copied() else {
            return ToolbarEvent::None;
        };
        if self.overflow.is_some() && p.clicked(self.overflow_id()) {
            let anchor = self.overflow.map(|(r, _)| r).unwrap_or_default();
            return ToolbarEvent::OpenOverflow { anchor };
        }
        match p.kind {
            PointerKind::Down(MouseButton::Left) => {
                state.pressed = self
                    .rects
                    .iter()
                    .find(|(i, _)| p.over(self.button_id(*i)))
                    .map(|(i, _)| *i);
                ToolbarEvent::None
            }
            PointerKind::Up(MouseButton::Left) => {
                state.pressed = None;
                let Some((index, rect)) = self
                    .rects
                    .iter()
                    .find(|(i, _)| p.clicked(self.button_id(*i)))
                    .copied()
                else {
                    return ToolbarEvent::None;
                };
                match &self.rows[index] {
                    ActionRow::Action {
                        action,
                        presentation,
                    } if presentation.enabled => {
                        let mods = Modifiers {
                            shift: p.shift(),
                            ctrl: p.command(),
                            ..Modifiers::NONE
                        };
                        if perform_with_modifiers(
                            action.as_ref(),
                            &self.data,
                            &self.place,
                            InputKind::Mouse,
                            mods,
                        ) {
                            return ToolbarEvent::Performed(action.id().to_owned());
                        }
                        ToolbarEvent::None
                    }
                    ActionRow::Group { presentation, .. } if presentation.enabled => {
                        ToolbarEvent::OpenGroup {
                            index,
                            anchor: rect,
                        }
                    }
                    _ => ToolbarEvent::None,
                }
            }
            PointerKind::Drag(MouseButton::Left) => {
                if state.pressed.is_some_and(|i| !p.over(self.button_id(i))) {
                    // Dragged off: the press look goes (the render checks hover too).
                }
                ToolbarEvent::None
            }
            _ => ToolbarEvent::None,
        }
    }

    /// The popup menu of the drop-down group at row `index` (from [`ToolbarEvent::OpenGroup`]).
    pub fn popup_for(&self, rt: &Runtime, index: usize, anchor: Rect) -> Option<ActionPopupMenu> {
        let ActionRow::Group { group, .. } = self.rows.get(index)? else {
            return None;
        };
        Some(ActionPopupMenu::actions(
            group.clone(),
            self.data.clone(),
            self.place.clone(),
            rt.keymap.clone(),
            rt.caps,
            Placement::Below(anchor),
        ))
    }

    /// A menu of the rows that did not fit (from [`ToolbarEvent::OpenOverflow`]).
    pub fn overflow_menu(&self, rt: &Runtime, anchor: Rect) -> Option<ActionPopupMenu> {
        let (_, first) = self.overflow?;
        let mut group = DefaultActionGroup::new("");
        for row in self.rows.iter().skip(first) {
            match row {
                ActionRow::Action { action, .. } => {
                    group.add_rc(action.clone());
                }
                ActionRow::Group { group: g, .. } => {
                    group.add_group_rc(g.clone());
                }
                ActionRow::Separator(_) => {
                    group.add_separator();
                }
                _ => {}
            }
        }
        Some(ActionPopupMenu::actions(
            Rc::new(group),
            self.data.clone(),
            self.place.clone(),
            rt.keymap.clone(),
            rt.caps,
            Placement::Below(anchor),
        ))
    }
}

fn fill_rect_area(buf: &mut Buffer, area: Rect, bg: ratatui::style::Color) {
    crate::components::fill_rect(buf, area, Style::default().bg(bg));
}

fn icon_of(icon: &archipel_kit::icons::Icon) -> Option<crate::components::RowIcon> {
    use archipel_kit::icons::Icon;
    Some(match icon {
        Icon::Glyph(g) => crate::components::RowIcon::Glyph(*g),
        Icon::Language(l) => crate::components::RowIcon::Language(*l),
        Icon::Symbol(s) => crate::components::RowIcon::Symbol(*s),
        Icon::FolderKind(_) => crate::components::RowIcon::Folder(false),
        Icon::FileImage => crate::components::RowIcon::Glyph(crate::Glyph::Image),
        Icon::FileArchive => crate::components::RowIcon::Glyph(crate::Glyph::Archive),
    })
}

/// A single flat action button (IntelliJ `ActionButton`): a toolbar of one action.
pub struct ActionButton {
    toolbar: ActionToolbar,
    /// Its state.
    pub state: ToolbarState,
}

impl ActionButton {
    /// A button for `action` in `place`.
    pub fn new(
        id: Id,
        action: Rc<dyn AnAction>,
        data: Rc<DataContext>,
        place: ActionPlace,
    ) -> Self {
        let mut group = DefaultActionGroup::new("");
        group.add_rc(action);
        Self {
            toolbar: ActionToolbar::new(id, Rc::new(group), data, place),
            state: ToolbarState::default(),
        }
    }

    /// Draws the button at `area` (3 cells wide).
    pub fn show(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        self.toolbar.show(rt, &self.state, area, buf);
    }

    /// Applies `event`; returns the id of the action that ran.
    pub fn handle(&mut self, rt: &mut Runtime, event: &Event) -> Option<String> {
        match self.toolbar.handle(rt, &mut self.state, event) {
            ToolbarEvent::Performed(id) => Some(id),
            _ => None,
        }
    }
}
