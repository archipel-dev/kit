//! An action group shown as a popup menu (IntelliJ `ActionPopupMenu`).

use super::{
    ActionGroup, ActionPlace, ActionRow, AnAction, DataContext, InputKind, Keymap,
    flatten_with_shortcuts, perform_with_modifiers,
};
use crate::ui::popup::{
    Chosen, LevelSpec, Options, Output, Placement, PopupChain, PopupSource, RowSpec, drive,
};
use egui::{Context, Id, Pos2, Rect};
use std::rc::Rc;

/// What a frame of an [`ActionPopupMenu`] reported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ActionPopupMenuResponse {
    /// The id of the action performed this frame, if one was chosen and could run.
    pub performed: Option<String>,
    /// The menu closed without choosing an action (Esc, a click outside).
    pub cancelled: bool,
}

/// An [`ActionGroup`] shown as a popup menu: rows for actions (icon, text, shortcut, check
/// mark for toggles), titled and plain separators, and submenus for popup groups. Choosing an
/// action [`perform`](super::perform)s it, closes the menu (unless its presentation says
/// `keep_popup_on_perform`) and reports its id.
///
/// The menu follows the list popup rules (IntelliJ spec §5): arrows move one shared
/// highlight and wrap, <kbd>Home</kbd>/<kbd>End</kbd>/<kbd>PgUp</kbd>/<kbd>PgDn</kbd>,
/// <kbd>Enter</kbd> chooses, <kbd>→</kbd> opens a submenu and <kbd>←</kbd> or <kbd>Esc</kbd>
/// closes it, <kbd>Shift</kbd>+<kbd>Esc</kbd> closes every level. The first pointer move after
/// opening is ignored, hovering highlights, a submenu opens after the pointer rests on its row
/// for 250 ms, the highlight stays on the last row when the pointer leaves, a press outside
/// closes the menu and it cannot reopen for 200 ms. Presentations are updated every frame, so
/// rows follow the action state live.
///
/// The caller owns the [`PopupChain`] (open, selection, which submenu is open). Open it with
/// [`PopupChain::open`] on the click, show it every frame while it is open.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::action_system::*;
/// use archipel_kit::ui::popup::PopupChain;
///
/// struct Copy;
/// impl AnAction for Copy {
///     fn id(&self) -> &str { "Copy" }
///     fn template_presentation(&self) -> Presentation { Presentation::new("Copy") }
///     fn action_performed(&self, _e: &AnActionEvent<'_>) {}
/// }
/// let mut group = DefaultActionGroup::new("Edit");
/// group.add(Copy);
/// let context = DataContext::empty();
/// let mut chain = PopupChain::default();
/// let now = ui.ctx().input(|i| i.time);
/// chain.open(now, false);
/// let out = ActionPopupMenu::new("edit-menu", &group, &context, ActionPlace::EDITOR_POPUP)
///     .show_at(ui.ctx(), &mut chain, egui::pos2(10.0, 10.0));
/// if let Some(id) = out.performed { let _ = id; }
/// # });
/// ```
#[must_use = "show it with `.show(ctx, &mut chain, anchor)`"]
pub struct ActionPopupMenu<'a> {
    id: Id,
    group: &'a dyn ActionGroup,
    context: &'a DataContext,
    place: ActionPlace,
    keymap: Option<(&'a Keymap, bool)>,
    submenu_on_hover: bool,
    speed_search: bool,
    numbering: bool,
}

impl<'a> ActionPopupMenu<'a> {
    /// A menu of `group` for `place`, with `context` as the data actions see.
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
            keymap: None,
            submenu_on_hover: true,
            speed_search: false,
            numbering: false,
        }
    }

    /// Shows each action's first keyboard shortcut from `keymap`; `mac` picks the macOS
    /// symbol style.
    pub fn keymap(mut self, keymap: &'a Keymap, mac: bool) -> Self {
        self.keymap = Some((keymap, mac));
        self
    }

    /// Whether submenus open after 250 ms of the pointer resting on their row (default on;
    /// they always open on <kbd>→</kbd>, <kbd>Enter</kbd> and click).
    pub fn show_submenu_on_hover(mut self, on: bool) -> Self {
        self.submenu_on_hover = on;
        self
    }

    /// Typing filters the rows (IntelliJ `ActionSelectionAid.SPEEDSEARCH`).
    pub fn speed_search(mut self, on: bool) -> Self {
        self.speed_search = on;
        self
    }

    /// Numbers the rows 1-9 (then letters) and chooses a row when its number is typed
    /// (IntelliJ `ActionSelectionAid.NUMBERING`).
    pub fn numbering(mut self, on: bool) -> Self {
        self.numbering = on;
        self
    }

    /// Shows the menu below `anchor` (above it when there is no room) while `chain` is open.
    /// A menu opened for a toolbar drop-down selects its first row; pass `preselect` to
    /// [`PopupChain::open`] for that.
    pub fn show(
        self,
        ctx: &Context,
        chain: &mut PopupChain,
        anchor: Rect,
    ) -> ActionPopupMenuResponse {
        self.run(ctx, chain, Placement::Below(anchor))
    }

    /// Shows the menu with its top-left corner at `pos` (a context menu).
    pub fn show_at(
        self,
        ctx: &Context,
        chain: &mut PopupChain,
        pos: Pos2,
    ) -> ActionPopupMenuResponse {
        self.run(ctx, chain, Placement::At(pos))
    }

    fn run(
        self,
        ctx: &Context,
        chain: &mut PopupChain,
        placement: Placement,
    ) -> ActionPopupMenuResponse {
        let mut source = ActionSource {
            group: self.group,
            context: self.context,
            place: &self.place,
            keymap: self.keymap,
            speed_search: self.speed_search,
            numbering: self.numbering,
            levels: Vec::new(),
            performed: None,
            pointer_input: ctx.input(|i| i.pointer.any_released()),
            modifiers: ctx.input(|i| i.modifiers),
        };
        let options = Options {
            show_submenu_on_hover: self.submenu_on_hover,
            cancel_on_click_outside: true,
        };
        let Output { cancelled, .. } = drive(ctx, self.id, placement, options, chain, &mut source);
        ActionPopupMenuResponse {
            performed: source.performed,
            cancelled,
        }
    }
}

/// The rows of one level: the actions (separators are folded into the rows after them).
struct LevelRows {
    rows: Vec<ActionRow>,
    spec: LevelSpec,
}

struct ActionSource<'a> {
    group: &'a dyn ActionGroup,
    context: &'a DataContext,
    place: &'a ActionPlace,
    keymap: Option<(&'a Keymap, bool)>,
    speed_search: bool,
    numbering: bool,
    levels: Vec<Option<LevelRows>>,
    performed: Option<String>,
    pointer_input: bool,
    modifiers: egui::Modifiers,
}

impl ActionSource<'_> {
    fn flatten(&self, group: &dyn ActionGroup) -> Vec<ActionRow> {
        let default = Keymap::new("none");
        let (keymap, mac) = self.keymap.unwrap_or((&default, false));
        flatten_with_shortcuts(group, self.context, self.place, keymap, mac)
    }

    /// The group whose rows level `depth` shows, found through the chosen rows `path`.
    fn group_at(&mut self, depth: usize, path: &[usize]) -> Option<Rc<dyn ActionGroup>> {
        if depth == 0 {
            return None;
        }
        let parent = self.level_rows(depth - 1, &path[..depth - 1])?;
        match parent.rows.get(*path.get(depth - 1)?)? {
            ActionRow::Group { group, .. } => Some(group.clone()),
            _ => None,
        }
    }

    fn level_rows(&mut self, depth: usize, path: &[usize]) -> Option<&LevelRows> {
        while self.levels.len() <= depth {
            self.levels.push(None);
        }
        if self.levels[depth].is_none() {
            let rows = if depth == 0 {
                self.flatten(self.group)
            } else {
                let group = self.group_at(depth, path)?;
                self.flatten(group.as_ref())
            };
            let spec = self.spec_of(&rows);
            // Selectable rows only: separators fold into the next row.
            let rows: Vec<ActionRow> = rows.into_iter().filter(|r| !r.is_separator()).collect();
            self.levels[depth] = Some(LevelRows { rows, spec });
        }
        self.levels[depth].as_ref()
    }

    fn spec_of(&self, rows: &[ActionRow]) -> LevelSpec {
        let mut specs: Vec<RowSpec> = Vec::new();
        let mut separator: Option<Option<String>> = None;
        let mut number = 0usize;
        for row in rows {
            match row {
                ActionRow::Separator(s) => separator = Some(s.text.clone()),
                ActionRow::Action { presentation, .. } | ActionRow::Group { presentation, .. } => {
                    let sub = matches!(row, ActionRow::Group { .. });
                    let mnemonic = (self.numbering && presentation.enabled)
                        .then(|| {
                            let c = match number {
                                0..=8 => char::from_digit(number as u32 + 1, 10),
                                9 => Some('0'),
                                n => char::from_u32('A' as u32 + (n as u32 - 10)),
                            };
                            number += 1;
                            c.filter(char::is_ascii_alphanumeric)
                        })
                        .flatten();
                    specs.push(RowSpec {
                        text: presentation.text.clone(),
                        icon: presentation.icon,
                        shortcut: presentation.shortcut_text.clone(),
                        checked: presentation.selected,
                        selectable: presentation.enabled,
                        separator: separator.take(),
                        sub,
                        mnemonic,
                        keep_open: presentation.keep_popup_on_perform,
                        ..RowSpec::default()
                    });
                }
            }
        }
        LevelSpec {
            rows: specs,
            title: None,
            speed_search: self.speed_search,
            auto_select: false,
            mnemonics: self.numbering,
            default_index: None,
        }
    }
}

impl PopupSource for ActionSource<'_> {
    fn level(&mut self, depth: usize, path: &[usize]) -> Option<LevelSpec> {
        let spec = self.level_rows(depth, path)?.spec.clone();
        (!spec.rows.is_empty()).then_some(spec)
    }

    fn choose(&mut self, depth: usize, path: &[usize], row: usize, final_choice: bool) -> Chosen {
        let Some(level) = self.level_rows(depth, path) else {
            return Chosen::Stay;
        };
        match level.rows.get(row) {
            Some(ActionRow::Group { .. }) if !final_choice => Chosen::Opened,
            Some(ActionRow::Action { action, .. }) if final_choice => {
                let action: Rc<dyn AnAction> = action.clone();
                let input = if self.pointer_input {
                    InputKind::Mouse
                } else {
                    InputKind::Keyboard
                };
                if perform_with_modifiers(
                    action.as_ref(),
                    self.context,
                    self.place,
                    input,
                    self.modifiers,
                ) {
                    self.performed = Some(action.id().to_owned());
                }
                // The state the rows were read from changed: read it again next frame.
                self.levels.clear();
                Chosen::Final
            }
            _ => Chosen::Stay,
        }
    }
}
