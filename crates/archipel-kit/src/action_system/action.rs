//! Actions: [`AnAction`], [`ToggleAction`], [`AnActionEvent`] and the update/perform helpers.

use egui::Modifiers;

use super::data_context::{DataContext, DataKey};
use super::places::ActionPlace;
use super::presentation::Presentation;

/// Where an action's event came from (IntelliJ `AnActionEvent.getInputEvent`, reduced to its
/// kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum InputKind {
    /// No input event: an update pass, or an action invoked from code.
    #[default]
    None,
    /// A pointer click or press.
    Mouse,
    /// A key press, from a shortcut or from activating a focused item.
    Keyboard,
}

/// The kind of surface an action is shown on.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.ActionUiKind` (`NONE`, `TOOLBAR`,
/// `POPUP`, `MAIN_MENU`, `SEARCH_POPUP`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ActionUiKind {
    /// No surface.
    #[default]
    None,
    /// A toolbar.
    Toolbar,
    /// A context menu or popup list.
    Popup,
    /// The main menu.
    MainMenu,
    /// The action search popup.
    SearchPopup,
}

impl ActionUiKind {
    /// The kind implied by `place`: popup places are [`Popup`](Self::Popup), toolbar places
    /// [`Toolbar`](Self::Toolbar), plus the main menu and the action search.
    pub fn for_place(place: &ActionPlace) -> Self {
        if *place == ActionPlace::MAIN_MENU {
            Self::MainMenu
        } else if *place == ActionPlace::ACTION_SEARCH {
            Self::SearchPopup
        } else if place.is_popup_place() {
            Self::Popup
        } else if place.is_toolbar_place() {
            Self::Toolbar
        } else {
            Self::None
        }
    }
}

/// What an action sees when it updates or runs: the presentation to fill, the data, the place
/// and the input.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.AnActionEvent`. During `update`
/// the presentation is the one to change; during `action_performed` it holds the state the
/// action had after its last update.
#[derive(Debug)]
#[non_exhaustive]
pub struct AnActionEvent<'a> {
    /// The presentation to fill during `update` (IntelliJ `getPresentation`).
    pub presentation: Presentation,
    /// The data to read (IntelliJ `getDataContext`).
    pub data_context: &'a DataContext,
    /// Where the action is shown (IntelliJ `getPlace`).
    pub place: ActionPlace,
    /// The surface kind (IntelliJ `getUiKind`).
    pub ui_kind: ActionUiKind,
    /// The kind of input that triggered the event (IntelliJ `getInputEvent`).
    pub input: InputKind,
    /// The modifier keys held (IntelliJ `getModifiers`).
    pub modifiers: Modifiers,
}

impl<'a> AnActionEvent<'a> {
    /// An event for an update pass: default presentation, no input, no modifiers. The UI
    /// kind is derived from the place.
    pub fn new(data_context: &'a DataContext, place: ActionPlace) -> Self {
        Self {
            presentation: Presentation::default(),
            data_context,
            ui_kind: ActionUiKind::for_place(&place),
            place,
            input: InputKind::None,
            modifiers: Modifiers::NONE,
        }
    }

    /// Sets the presentation the event starts with.
    #[must_use]
    pub fn with_presentation(mut self, presentation: Presentation) -> Self {
        self.presentation = presentation;
        self
    }

    /// Sets the input kind.
    #[must_use]
    pub fn with_input(mut self, input: InputKind) -> Self {
        self.input = input;
        self
    }

    /// Sets the held modifiers.
    #[must_use]
    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Sets the UI kind, overriding the one derived from the place.
    #[must_use]
    pub fn with_ui_kind(mut self, ui_kind: ActionUiKind) -> Self {
        self.ui_kind = ui_kind;
        self
    }

    /// The value for `key` in the data context (IntelliJ `AnActionEvent.getData`).
    pub fn data<T: 'static>(&self, key: DataKey<T>) -> Option<&T> {
        self.data_context.get(key)
    }

    /// Whether the event comes from a toolbar (IntelliJ `isFromActionToolbar`).
    pub fn is_from_action_toolbar(&self) -> bool {
        self.ui_kind == ActionUiKind::Toolbar
    }

    /// Whether the event comes from a context menu (IntelliJ `isFromContextMenu`).
    pub fn is_from_context_menu(&self) -> bool {
        self.ui_kind == ActionUiKind::Popup && self.place.is_popup_place()
    }

    /// Whether the event comes from the main menu (IntelliJ `isFromMainMenu`).
    pub fn is_from_main_menu(&self) -> bool {
        self.ui_kind == ActionUiKind::MainMenu
    }

    /// Whether the event comes from the action search (IntelliJ `isFromSearchPopup`).
    pub fn is_from_search_popup(&self) -> bool {
        self.ui_kind == ActionUiKind::SearchPopup
    }
}

/// A command: it decides in `update` whether it is available and how it looks, and does its
/// work in `action_performed`.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.AnAction`. Like IntelliJ's, an
/// action is stateless by contract and one instance serves every place; its state lives in
/// the caller (use `Cell`, `RefCell` or channels in the implementor, since methods take
/// `&self`). Differences:
///
/// - The template presentation is a method that returns a value, not a mutable field.
/// - There is no `getActionUpdateThread`: `update` runs on the UI thread every frame, so keep
///   it cheap, or cache in the host and invalidate.
/// - There is no `DumbAwareAction`: the marker means "usable while indexing", and the kit has
///   no indexing; a host that has it keeps the flag on its own action type.
///
/// ```
/// use archipel_kit::action_system::{AnAction, AnActionEvent, Presentation};
///
/// struct Reload;
/// impl AnAction for Reload {
///     fn id(&self) -> &str { "Reload" }
///     fn template_presentation(&self) -> Presentation { Presentation::new("Reload") }
///     fn action_performed(&self, _e: &AnActionEvent<'_>) { /* reload */ }
/// }
/// ```
pub trait AnAction {
    /// The id the action is registered and bound to shortcuts under (IntelliJ
    /// `ActionManager.getId`; the id is a property of the action here because there is no
    /// global registry to ask).
    fn id(&self) -> &str;

    /// The presentation every place starts from (IntelliJ `getTemplatePresentation`).
    fn template_presentation(&self) -> Presentation;

    /// Decides enabled, visible, text and icon for this frame by changing
    /// `e.presentation` (IntelliJ `update`). The default leaves the template as is.
    fn update(&self, e: &mut AnActionEvent<'_>) {
        let _ = e;
    }

    /// Runs the action (IntelliJ `actionPerformed`). Called only when the action is enabled
    /// and visible after `update`, by [`perform`].
    fn action_performed(&self, e: &AnActionEvent<'_>);
}

/// An action with an on/off state: a checkmark menu item, a pressed toolbar button, a
/// checkbox row.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.ToggleAction`. Wrap it in
/// [`Toggle`] to get an [`AnAction`]: the wrapper copies `is_selected` into the presentation
/// on every update and flips the state on perform.
///
/// ```
/// use std::cell::Cell;
/// use archipel_kit::action_system::{
///     AnActionEvent, Presentation, Toggle, ToggleAction, DataContext, ActionPlace, InputKind,
///     perform, update_presentation,
/// };
///
/// #[derive(Default)]
/// struct WordWrap(Cell<bool>);
/// impl ToggleAction for WordWrap {
///     fn id(&self) -> &str { "WordWrap" }
///     fn template_presentation(&self) -> Presentation { Presentation::new("Soft-Wrap") }
///     fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool { self.0.get() }
///     fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) { self.0.set(selected) }
/// }
///
/// let action = Toggle::new(WordWrap::default());
/// let ctx = DataContext::empty();
/// assert_eq!(update_presentation(&action, &ctx, &ActionPlace::POPUP).selected, Some(false));
/// perform(&action, &ctx, &ActionPlace::POPUP, InputKind::Mouse);
/// assert_eq!(update_presentation(&action, &ctx, &ActionPlace::POPUP).selected, Some(true));
/// ```
pub trait ToggleAction {
    /// The action id (see [`AnAction::id`]).
    fn id(&self) -> &str;

    /// The template presentation; [`Toggle`] marks it as a toggle.
    fn template_presentation(&self) -> Presentation;

    /// Whether the action is on (IntelliJ `ToggleAction.isSelected`).
    fn is_selected(&self, e: &AnActionEvent<'_>) -> bool;

    /// Turns the action on or off (IntelliJ `ToggleAction.setSelected`).
    fn set_selected(&self, e: &AnActionEvent<'_>, selected: bool);

    /// Extra availability logic, run before the selected state is copied in. The default is
    /// a no-op.
    fn update(&self, e: &mut AnActionEvent<'_>) {
        let _ = e;
    }
}

/// Adapts a [`ToggleAction`] to an [`AnAction`] (IntelliJ `ToggleAction extends AnAction`).
///
/// A blanket `impl<T: ToggleAction> AnAction for T` would forbid every other `AnAction`
/// implementation in downstream crates, so the adapter is explicit.
#[derive(Debug, Clone, Default)]
pub struct Toggle<T>(pub T);

impl<T: ToggleAction> Toggle<T> {
    /// Wraps `action`.
    pub fn new(action: T) -> Self {
        Self(action)
    }
}

impl<T: ToggleAction> AnAction for Toggle<T> {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn template_presentation(&self) -> Presentation {
        let mut presentation = self.0.template_presentation();
        presentation.selected.get_or_insert(false);
        presentation
    }

    fn update(&self, e: &mut AnActionEvent<'_>) {
        self.0.update(e);
        e.presentation.selected = Some(self.0.is_selected(e));
    }

    fn action_performed(&self, e: &AnActionEvent<'_>) {
        let selected = self.0.is_selected(e);
        self.0.set_selected(e, !selected);
    }
}

/// Runs `action.update` for `place` and returns the resulting presentation (IntelliJ
/// `ActionUtil.updateAction` / `performDumbAwareUpdate`). The result starts from the
/// action's template; nothing is cached.
pub fn update_presentation(
    action: &dyn AnAction,
    data_context: &DataContext,
    place: &ActionPlace,
) -> Presentation {
    let mut event = AnActionEvent::new(data_context, place.clone())
        .with_presentation(action.template_presentation());
    action.update(&mut event);
    event.presentation
}

/// Updates `action`, and runs it only if it is enabled and visible afterwards. Returns
/// whether it ran (IntelliJ `ActionUtil.performAction` after `ActionUtil.lastUpdateAndCheckDumb`).
pub fn perform(
    action: &dyn AnAction,
    data_context: &DataContext,
    place: &ActionPlace,
    input: InputKind,
) -> bool {
    perform_with_modifiers(action, data_context, place, input, Modifiers::NONE)
}

/// Like [`perform`], with the modifier keys held at the time (IntelliJ `getModifiers`).
pub fn perform_with_modifiers(
    action: &dyn AnAction,
    data_context: &DataContext,
    place: &ActionPlace,
    input: InputKind,
    modifiers: Modifiers,
) -> bool {
    let mut event = AnActionEvent::new(data_context, place.clone())
        .with_presentation(action.template_presentation())
        .with_input(input)
        .with_modifiers(modifiers);
    action.update(&mut event);
    if !event.presentation.is_actionable() {
        return false;
    }
    action.action_performed(&event);
    true
}
