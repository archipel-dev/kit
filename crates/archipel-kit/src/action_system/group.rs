//! Groups of actions: [`ActionGroup`], [`DefaultActionGroup`], [`Separator`], and
//! [`flatten`], which turns a group into the rows a menu or toolbar draws.

use std::fmt;
use std::rc::Rc;

use super::action::{ActionUiKind, AnAction, AnActionEvent, update_presentation};
use super::data_context::DataContext;
use super::keymap::{Keymap, KeymapUtil};
use super::places::ActionPlace;
use super::presentation::Presentation;

/// Groups nested deeper than this are ignored, so a group that contains itself ends the
/// recursion instead of the process.
pub const MAX_GROUP_DEPTH: usize = 16;

/// A divider between actions, optionally with a title.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.Separator`. A separator with text
/// renders as a titled divider in menus and popups.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Separator {
    /// The title, if any (IntelliJ `Separator.getText`).
    pub text: Option<String>,
}

impl Separator {
    /// A plain divider (IntelliJ `Separator.getInstance()`).
    pub fn new() -> Self {
        Self { text: None }
    }

    /// A titled divider (IntelliJ `Separator.create(text)`).
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
        }
    }
}

/// One child of a group.
///
/// IntelliJ keeps all three as `AnAction` subclasses (`AnAction`, `ActionGroup`,
/// `Separator`); an enum makes the case analysis exhaustive for renderers.
#[derive(Clone)]
#[non_exhaustive]
pub enum ActionItem {
    /// A command.
    Action(Rc<dyn AnAction>),
    /// A nested group: flattened into its parent, or a submenu when it is a popup group.
    Group(Rc<dyn ActionGroup>),
    /// A divider.
    Separator(Separator),
}

impl ActionItem {
    /// An action item (IntelliJ `DefaultActionGroup.add(AnAction)`).
    pub fn action(action: impl AnAction + 'static) -> Self {
        Self::Action(Rc::new(action))
    }

    /// A group item.
    pub fn group(group: impl ActionGroup + 'static) -> Self {
        Self::Group(Rc::new(group))
    }

    /// A plain separator item.
    pub fn separator() -> Self {
        Self::Separator(Separator::new())
    }
}

impl fmt::Debug for ActionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(action) => write!(f, "Action({})", action.id()),
            Self::Group(group) => write!(f, "Group({})", group.name()),
            Self::Separator(separator) => write!(f, "{separator:?}"),
        }
    }
}

/// An action that stands for a list of children: a menu, a toolbar, a drop-down.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.ActionGroup` (which extends
/// `AnAction`; here groups are a separate trait because they are never performed, except
/// through [`Presentation::perform_group`]). `children` may compute the list from the event,
/// like IntelliJ's `getChildren(AnActionEvent?)`.
pub trait ActionGroup {
    /// The id the group is registered under, if any (IntelliJ `ActionManager.getId`).
    fn id(&self) -> Option<&str> {
        None
    }

    /// The group's display name, the text of its popup row (IntelliJ
    /// `getTemplatePresentation().getText()`).
    fn name(&self) -> &str {
        ""
    }

    /// The children in order (IntelliJ `getChildren(e)`). `e` is `None` when the list is
    /// asked for outside an update, like IntelliJ's null event.
    fn children(&self, e: Option<&AnActionEvent<'_>>) -> Vec<ActionItem>;

    /// Whether the group opens as a submenu or drop-down instead of flattening into its
    /// parent (IntelliJ `isPopup`).
    fn is_popup(&self) -> bool {
        false
    }

    /// The presentation the group starts from.
    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.name()).popup_group(self.is_popup())
    }

    /// Updates the group's own presentation, for example to hide it (IntelliJ
    /// `ActionGroup.update`). The default is a no-op.
    fn update(&self, e: &mut AnActionEvent<'_>) {
        let _ = e;
    }
}

/// An ordered, caller-built list of children.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.DefaultActionGroup`. Positioning
/// constraints (`Constraints.before` ...) and the modification stamp are left out: build the
/// group in the order you want, every frame or once.
///
/// ```
/// use archipel_kit::action_system::{
///     ActionGroup, AnAction, AnActionEvent, DefaultActionGroup, Presentation,
/// };
///
/// struct Open;
/// impl AnAction for Open {
///     fn id(&self) -> &str { "Open" }
///     fn template_presentation(&self) -> Presentation { Presentation::new("Open") }
///     fn action_performed(&self, _e: &AnActionEvent<'_>) {}
/// }
///
/// let mut file = DefaultActionGroup::popup("File");
/// file.add(Open).add_separator();
/// assert_eq!(file.len(), 2);
/// assert!(file.is_popup());
/// ```
#[derive(Debug, Clone, Default)]
pub struct DefaultActionGroup {
    name: String,
    id: Option<String>,
    popup: bool,
    items: Vec<ActionItem>,
}

impl DefaultActionGroup {
    /// A flat group: its children appear in the parent (IntelliJ `createFlatGroup`).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// A popup group: a submenu or drop-down (IntelliJ `createPopupGroup`).
    pub fn popup(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            popup: true,
            ..Self::default()
        }
    }

    /// Sets the id the group is registered under.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Makes the group a popup or flat group (IntelliJ `setPopup`).
    #[must_use]
    pub fn with_popup(mut self, popup: bool) -> Self {
        self.popup = popup;
        self
    }

    /// Appends an action (IntelliJ `add(AnAction)`).
    pub fn add(&mut self, action: impl AnAction + 'static) -> &mut Self {
        self.items.push(ActionItem::action(action));
        self
    }

    /// Appends a shared action (the same instance can sit in several groups).
    pub fn add_rc(&mut self, action: Rc<dyn AnAction>) -> &mut Self {
        self.items.push(ActionItem::Action(action));
        self
    }

    /// Appends a plain separator (IntelliJ `addSeparator()`).
    pub fn add_separator(&mut self) -> &mut Self {
        self.items.push(ActionItem::separator());
        self
    }

    /// Appends a titled separator (IntelliJ `addSeparator(text)`).
    pub fn add_separator_with_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.items
            .push(ActionItem::Separator(Separator::with_text(text)));
        self
    }

    /// Appends a nested group (IntelliJ `add(ActionGroup)`).
    pub fn add_group(&mut self, group: impl ActionGroup + 'static) -> &mut Self {
        self.items.push(ActionItem::group(group));
        self
    }

    /// Appends a shared nested group.
    pub fn add_group_rc(&mut self, group: Rc<dyn ActionGroup>) -> &mut Self {
        self.items.push(ActionItem::Group(group));
        self
    }

    /// Appends any item.
    pub fn add_item(&mut self, item: ActionItem) -> &mut Self {
        self.items.push(item);
        self
    }

    /// Removes every action with `id` (IntelliJ `remove(AnAction)`). Returns how many.
    pub fn remove(&mut self, id: &str) -> usize {
        let before = self.items.len();
        self.items
            .retain(|item| !matches!(item, ActionItem::Action(action) if action.id() == id));
        before - self.items.len()
    }

    /// Removes all children (IntelliJ `removeAll`).
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// The number of children, separators included (IntelliJ `getChildrenCount`).
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no children.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl ActionGroup for DefaultActionGroup {
    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn children(&self, _e: Option<&AnActionEvent<'_>>) -> Vec<ActionItem> {
        self.items.clone()
    }

    fn is_popup(&self) -> bool {
        self.popup
    }
}

/// One row of a flattened group, ready to draw.
///
/// IntelliJ builds the same list inside `Utils.expandActionGroup` for menus and toolbars.
#[derive(Clone)]
#[non_exhaustive]
pub enum ActionRow {
    /// A command with its presentation for this frame.
    Action {
        /// The action, to perform on click.
        action: Rc<dyn AnAction>,
        /// How it looks now.
        presentation: Presentation,
    },
    /// A popup group: draw a submenu row, and [`flatten`] the group when it opens.
    Group {
        /// The group.
        group: Rc<dyn ActionGroup>,
        /// How its row looks now.
        presentation: Presentation,
    },
    /// A divider.
    Separator(Separator),
}

impl ActionRow {
    /// The row's presentation; `None` for separators.
    pub fn presentation(&self) -> Option<&Presentation> {
        match self {
            Self::Action { presentation, .. } | Self::Group { presentation, .. } => {
                Some(presentation)
            }
            Self::Separator(_) => None,
        }
    }

    /// Whether the row is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator(_))
    }
}

impl fmt::Debug for ActionRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action {
                action,
                presentation,
            } => write!(f, "Action({}, {:?})", action.id(), presentation.text),
            Self::Group {
                group,
                presentation,
            } => write!(f, "Group({}, {:?})", group.name(), presentation.text),
            Self::Separator(separator) => write!(f, "{separator:?}"),
        }
    }
}

/// Updates every child of `group` for `place` and returns the rows to draw.
///
/// The rules, the ones IntelliJ's menus and toolbars apply:
///
/// - invisible actions and groups are dropped, disabled ones stay;
/// - a flat group is inlined into its parent; a popup group stays one row;
/// - a popup group without visible actions is dropped when its presentation says
///   `hide_group_if_empty`, and disabled when it says `disable_group_if_empty`;
/// - separators never lead, never trail and never repeat; a titled separator replaces a
///   plain one next to it, and a titled separator may lead.
///
/// ```
/// use archipel_kit::action_system::{
///     flatten, ActionPlace, AnAction, AnActionEvent, DataContext, DefaultActionGroup, Presentation,
/// };
///
/// struct Item(&'static str);
/// impl AnAction for Item {
///     fn id(&self) -> &str { self.0 }
///     fn template_presentation(&self) -> Presentation { Presentation::new(self.0) }
///     fn action_performed(&self, _e: &AnActionEvent<'_>) {}
/// }
///
/// let mut group = DefaultActionGroup::new("Edit");
/// group.add_separator().add(Item("Cut")).add_separator().add_separator().add(Item("Paste")).add_separator();
/// let rows = flatten(&group, &DataContext::empty(), &ActionPlace::EDITOR_POPUP);
/// assert_eq!(rows.len(), 3); // Cut, separator, Paste
/// ```
pub fn flatten(
    group: &dyn ActionGroup,
    data_context: &DataContext,
    place: &ActionPlace,
) -> Vec<ActionRow> {
    flatten_impl(group, data_context, place, None)
}

/// Like [`flatten`], and fills each row's `shortcut_text` from `keymap` when the action has
/// none (IntelliJ menus show `KeymapUtil.getFirstKeyboardShortcutText`). `mac` picks the
/// macOS symbol style.
pub fn flatten_with_shortcuts(
    group: &dyn ActionGroup,
    data_context: &DataContext,
    place: &ActionPlace,
    keymap: &Keymap,
    mac: bool,
) -> Vec<ActionRow> {
    flatten_impl(group, data_context, place, Some((keymap, mac)))
}

type Shortcuts<'a> = Option<(&'a Keymap, bool)>;

fn flatten_impl(
    group: &dyn ActionGroup,
    data_context: &DataContext,
    place: &ActionPlace,
    shortcuts: Shortcuts<'_>,
) -> Vec<ActionRow> {
    let mut rows = Vec::new();
    let mut event = AnActionEvent::new(data_context, place.clone())
        .with_presentation(group.template_presentation());
    group.update(&mut event);
    collect(group, &event, shortcuts, 0, &mut rows);
    while rows.last().is_some_and(ActionRow::is_separator) {
        rows.pop();
    }
    rows
}

fn collect(
    group: &dyn ActionGroup,
    event: &AnActionEvent<'_>,
    shortcuts: Shortcuts<'_>,
    depth: usize,
    rows: &mut Vec<ActionRow>,
) {
    if depth > MAX_GROUP_DEPTH {
        return;
    }
    let (data_context, place) = (event.data_context, &event.place);
    for item in group.children(Some(event)) {
        match item {
            ActionItem::Action(action) => {
                let mut presentation = update_presentation(action.as_ref(), data_context, place);
                if !presentation.visible {
                    continue;
                }
                fill_shortcut(&mut presentation, action.id(), shortcuts);
                rows.push(ActionRow::Action {
                    action,
                    presentation,
                });
            }
            ActionItem::Separator(separator) => push_separator(rows, separator),
            ActionItem::Group(sub) => {
                let mut sub_event = AnActionEvent::new(data_context, place.clone())
                    .with_presentation(sub.template_presentation())
                    .with_ui_kind(ActionUiKind::for_place(place));
                sub.update(&mut sub_event);
                let mut presentation = sub_event.presentation.clone();
                if !presentation.visible {
                    continue;
                }
                if sub.is_popup() || presentation.popup_group {
                    presentation.popup_group = true;
                    let mut inner = Vec::new();
                    collect(sub.as_ref(), &sub_event, shortcuts, depth + 1, &mut inner);
                    if !inner.iter().any(|row| !row.is_separator()) {
                        if presentation.hide_group_if_empty {
                            continue;
                        }
                        if presentation.disable_group_if_empty {
                            presentation.enabled = false;
                        }
                    }
                    rows.push(ActionRow::Group {
                        group: sub,
                        presentation,
                    });
                } else {
                    collect(sub.as_ref(), &sub_event, shortcuts, depth + 1, rows);
                }
            }
        }
    }
}

fn fill_shortcut(presentation: &mut Presentation, id: &str, shortcuts: Shortcuts<'_>) {
    if presentation.shortcut_text.is_some() {
        return;
    }
    if let Some((keymap, mac)) = shortcuts {
        presentation.shortcut_text = KeymapUtil::first_keyboard_shortcut_text(keymap, id, mac);
    }
}

fn push_separator(rows: &mut Vec<ActionRow>, separator: Separator) {
    match rows.last() {
        None if separator.text.is_none() => {}
        Some(ActionRow::Separator(_)) => {
            if separator.text.is_some() {
                rows.pop();
                rows.push(ActionRow::Separator(separator));
            }
        }
        _ => rows.push(ActionRow::Separator(separator)),
    }
}
