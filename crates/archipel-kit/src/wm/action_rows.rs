//! Renders an [`ActionGroup`] as header buttons and menu rows, performing the action that was
//! clicked. Shared by the tool window header, the tab strip and the status bar.

use crate::action_system::{
    ActionButton, ActionGroup, ActionMenuItem, ActionPlace, ActionRow, DataContext, InputKind,
    flatten, perform_with_modifiers,
};
use crate::icons::{Glyph, Icon};
use crate::ui::popup::{JBPopup, ListSeparator};
use egui::{Id, Ui};
use std::cell::RefCell;

/// How big the buttons of [`show_action_buttons`] are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonSize {
    /// 22 pt, as in island headers.
    Regular,
    /// 20 pt, inside tabs and rows.
    Small,
}

/// What [`show_action_buttons`] does besides drawing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ButtonOptions {
    /// Whether a click performs the action (false while a dead zone is running).
    pub perform: bool,
    /// Whether separators of the group are drawn.
    pub separators: bool,
    /// Draw the actions in reverse order (for a right-to-left layout, so they read in the
    /// group's order).
    pub reverse: bool,
}

impl Default for ButtonOptions {
    fn default() -> Self {
        Self {
            perform: true,
            separators: true,
            reverse: false,
        }
    }
}

/// Draws the visible actions of `group` for `place` as [`ActionButton`]s in the layout of
/// `ui` (separators become group separators, popup groups a button that opens a menu below
/// it). A click performs the action through [`perform_with_modifiers`]; returns the id of the
/// action that ran.
pub(crate) fn show_action_buttons(
    ui: &mut Ui,
    id: Id,
    group: &dyn ActionGroup,
    data_context: &DataContext,
    place: &ActionPlace,
    size: ButtonSize,
    options: ButtonOptions,
) -> Option<String> {
    let mut performed = None;
    let rows = flatten(group, data_context, place);
    let rows: Vec<ActionRow> = if options.reverse {
        rows.into_iter().rev().collect()
    } else {
        rows
    };
    for (index, row) in rows.into_iter().enumerate() {
        match row {
            ActionRow::Action {
                action,
                presentation,
            } => {
                let icon: Icon = presentation.icon.unwrap_or(Glyph::More.into());
                let mut button = ActionButton::new(icon, presentation.text.clone())
                    .enabled(presentation.enabled)
                    .small(size == ButtonSize::Small);
                if let Some(selected) = presentation.selected {
                    button = button.selected(selected);
                }
                let response = ui.add(button);
                if response.clicked() && options.perform {
                    let modifiers = ui.input(|i| i.modifiers);
                    if perform_with_modifiers(
                        action.as_ref(),
                        data_context,
                        place,
                        InputKind::Mouse,
                        modifiers,
                    ) {
                        performed = Some(action.id().to_owned());
                    }
                }
            }
            ActionRow::Group {
                group: sub,
                presentation,
            } => {
                let icon: Icon = presentation.icon.unwrap_or(Glyph::MoreVertical.into());
                let button = ActionButton::new(icon, presentation.text.clone())
                    .enabled(presentation.enabled)
                    .small(size == ButtonSize::Small);
                let response = ui.add(button);
                let open_id = id.with(("group", index));
                let mut open: bool = ui.ctx().data(|d| d.get_temp(open_id)).unwrap_or(false);
                if response.clicked() && options.perform {
                    open = !open;
                }
                JBPopup::below(open_id.with("popup"), &response)
                    .width(crate::tokens::size::SUBMENU_WIDTH)
                    .show(ui.ctx(), &mut open, |ui| {
                        let rows = flatten(sub.as_ref(), data_context, place);
                        if let Some(done) = show_menu_rows(ui, &rows, data_context, place) {
                            performed = Some(done);
                            true
                        } else {
                            false
                        }
                    })
                    .inspect(|&done| {
                        if done {
                            open = false;
                        }
                    });
                ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
            }
            ActionRow::Separator(_) => {
                if options.separators {
                    ui.add(crate::ui::SeparatorComponent::group());
                }
            }
        }
    }
    performed
}

/// Draws `rows` as menu items in `ui` (a popup's content), performing the one clicked and
/// returning its id. Popup groups become submenus.
pub(crate) fn show_menu_rows(
    ui: &mut Ui,
    rows: &[ActionRow],
    data_context: &DataContext,
    place: &ActionPlace,
) -> Option<String> {
    let performed: RefCell<Option<String>> = RefCell::new(None);
    for row in rows {
        match row {
            ActionRow::Action {
                action,
                presentation,
            } => {
                let mut item =
                    ActionMenuItem::new(presentation.text.as_str()).enabled(presentation.enabled);
                if let Some(icon) = presentation.icon {
                    item = item.icon(icon);
                }
                if let Some(selected) = presentation.selected {
                    item = item.checked(selected);
                }
                if let Some(shortcut) = &presentation.shortcut_text {
                    item = item.shortcut(shortcut.as_str());
                }
                if ui.add(item).clicked() {
                    let modifiers = ui.input(|i| i.modifiers);
                    if perform_with_modifiers(
                        action.as_ref(),
                        data_context,
                        place,
                        InputKind::Mouse,
                        modifiers,
                    ) {
                        *performed.borrow_mut() = Some(action.id().to_owned());
                    }
                }
            }
            ActionRow::Group {
                group,
                presentation,
            } => {
                let mut item =
                    ActionMenuItem::new(presentation.text.as_str()).enabled(presentation.enabled);
                if let Some(icon) = presentation.icon {
                    item = item.icon(icon);
                }
                let sub = flatten(group.as_ref(), data_context, place);
                let performed = &performed;
                ui.add(item.submenu(move |ui| {
                    if let Some(done) = show_menu_rows(ui, &sub, data_context, place) {
                        *performed.borrow_mut() = Some(done);
                    }
                }));
            }
            ActionRow::Separator(separator) => {
                match &separator.text {
                    Some(text) => ui.add(ListSeparator::with_text(text.as_str())),
                    None => ui.add(ListSeparator::new()),
                };
            }
        }
    }
    performed.into_inner()
}

/// The modifier keys held at the pointer events of this frame: the frame's own modifiers
/// plus those the pointer button events carry (they agree in a real window; a test harness
/// may only set the latter).
pub(crate) fn pointer_event_modifiers(ui: &Ui) -> egui::Modifiers {
    ui.input(|i| {
        let mut modifiers = i.modifiers;
        for event in &i.events {
            if let egui::Event::PointerButton { modifiers: m, .. } = event {
                modifiers.alt |= m.alt;
                modifiers.ctrl |= m.ctrl;
                modifiers.shift |= m.shift;
                modifiers.mac_cmd |= m.mac_cmd;
                modifiers.command |= m.command;
            }
        }
        modifiers
    })
}
