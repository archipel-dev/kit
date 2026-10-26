//! Where an action is shown: [`ActionPlace`], and the ids of the standard groups and
//! actions: [`IdeActions`].

use std::borrow::Cow;
use std::fmt;

/// The name of the place an action is shown or invoked from: a toolbar, a context menu, the
/// status bar.
///
/// IntelliJ counterpart: the string constants of `com.intellij.openapi.actionSystem.ActionPlaces`
/// carried by `AnActionEvent.getPlace`. A place is a plain string; actions branch on it in
/// `update` (hide a label in toolbars, show it in menus) and renderers pick looks by it. The
/// string values are the kit's own: only equality matters, never the spelling.
///
/// The IntelliJ convention is kept: `<Feature>Popup` is a context menu and `<Feature>Toolbar`
/// a toolbar, which [`is_popup_place`](Self::is_popup_place) and
/// [`is_toolbar_place`](Self::is_toolbar_place) rely on. Use [`ActionPlace::new`] for places of
/// your own.
///
/// ```
/// use archipel_kit::action_system::ActionPlace;
///
/// assert!(ActionPlace::EDITOR_POPUP.is_popup_place());
/// assert!(!ActionPlace::MAIN_TOOLBAR.is_popup_place());
/// assert!(ActionPlace::new("MyPluginPopup").is_popup_place());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionPlace(Cow<'static, str>);

macro_rules! places {
    ($($name:ident = $value:literal;)+) => {
        impl ActionPlace {
            $(
                #[doc = concat!("IntelliJ `ActionPlaces.", stringify!($name), "`.")]
                pub const $name: ActionPlace = ActionPlace(Cow::Borrowed($value));
            )+
        }
    };
}

places! {
    UNKNOWN = "unknown";
    TOOLBAR = "toolbar";
    POPUP = "popup";
    MAIN_MENU = "MainMenu";
    MAIN_TOOLBAR = "MainToolbar";
    EDITOR_POPUP = "EditorPopup";
    EDITOR_TOOLBAR = "EditorToolbar";
    EDITOR_TAB = "EditorTab";
    EDITOR_TAB_POPUP = "EditorTabPopup";
    TABS_MORE_TOOLBAR = "TabsMoreToolbar";
    EDITOR_GUTTER = "ICON_NAVIGATION";
    EDITOR_GUTTER_POPUP = "EditorGutterPopup";
    EDITOR_FLOATING_TOOLBAR = "EditorFloatingToolbar";
    EDITOR_INLAY = "EditorInlay";
    EDITOR_HINT = "EditorHint";
    CONTEXT_TOOLBAR = "ContextToolbar";
    TOOLWINDOW_POPUP = "ToolwindowPopup";
    TOOLWINDOW_TITLE = "ToolwindowTitle";
    TOOLWINDOW_CONTENT = "ToolwindowContent";
    TOOLWINDOW_TOOLBAR_BAR = "ToolwindowToolbarBar";
    STATUS_BAR_PLACE = "StatusBarPlace";
    ACTION_SEARCH = "GoToAction";
    KEYBOARD_SHORTCUT = "KeyboardShortcut";
    MOUSE_SHORTCUT = "MouseShortcut";
    TOOLBAR_DECORATOR_TOOLBAR = "ToolbarDecorator";
    NAVIGATION_BAR_TOOLBAR = "NavBarToolbar";
    PROJECT_VIEW_TOOLBAR = "ProjectViewToolbar";
    PROJECT_VIEW_POPUP = "ProjectViewPopup";
    RUN_CONFIGURATIONS_COMBOBOX = "RunConfigurationsComboBox";
    WELCOME_SCREEN = "WelcomeScreen";
}

impl ActionPlace {
    /// A place of your own (IntelliJ: any string passed as the place). Name context menus
    /// `...Popup` and toolbars `...Toolbar` so the `is_*_place` helpers keep working.
    pub fn new(name: impl Into<String>) -> Self {
        Self(Cow::Owned(name.into()))
    }

    /// The place's name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the place is a context menu: [`POPUP`](Self::POPUP) or any name ending in
    /// `Popup` (IntelliJ `ActionPlaces.isPopupPlace`).
    pub fn is_popup_place(&self) -> bool {
        let name = self.as_str();
        name.eq_ignore_ascii_case("popup") || name.ends_with("Popup")
    }

    /// Whether the place is a toolbar: [`TOOLBAR`](Self::TOOLBAR) or any name ending in
    /// `Toolbar` (IntelliJ `AnActionEvent.isFromActionToolbar` is true for these).
    pub fn is_toolbar_place(&self) -> bool {
        let name = self.as_str();
        name.eq_ignore_ascii_case("toolbar") || name.ends_with("Toolbar")
    }
}

impl Default for ActionPlace {
    fn default() -> Self {
        Self::UNKNOWN
    }
}

impl fmt::Display for ActionPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ActionPlace {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&'static str> for ActionPlace {
    fn from(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }
}

impl From<String> for ActionPlace {
    fn from(name: String) -> Self {
        Self(Cow::Owned(name))
    }
}

/// Ids of the standard groups and actions the kit refers to.
///
/// IntelliJ counterpart: `com.intellij.openapi.actionSystem.IdeActions` (group ids) and the
/// action ids of `platform-resources` keymaps (`$Copy`, `Find`, ...). Register your action
/// under one of these ids to get its default shortcut from [`Keymap::default_for`].
///
/// [`Keymap::default_for`]: crate::action_system::Keymap::default_for
pub struct IdeActions;

macro_rules! ids {
    ($($name:ident = $value:literal;)+) => {
        impl IdeActions {
            $(
                #[doc = concat!("The id `", $value, "` (IntelliJ `IdeActions.", stringify!($name), "` or the keymap action id).")]
                pub const $name: &'static str = $value;
            )+
        }
    };
}

ids! {
    GROUP_MAIN_MENU = "MainMenu";
    GROUP_MAIN_TOOLBAR = "MainToolBar";
    GROUP_EDITOR_POPUP = "EditorPopupMenu";
    GROUP_NAVBAR_POPUP = "NavbarPopupMenu";
    GROUP_NAVBAR_TOOLBAR = "NavBarToolBar";
    ACTION_CUT = "$Cut";
    ACTION_COPY = "$Copy";
    ACTION_PASTE = "$Paste";
    ACTION_PASTE_HISTORY = "PasteMultiple";
    ACTION_UNDO = "$Undo";
    ACTION_REDO = "$Redo";
    ACTION_SELECT_ALL = "$SelectAll";
    ACTION_DELETE = "$Delete";
    ACTION_FIND = "Find";
    ACTION_REPLACE = "Replace";
    ACTION_FIND_NEXT = "FindNext";
    ACTION_FIND_PREVIOUS = "FindPrevious";
    ACTION_FIND_IN_PATH = "FindInPath";
    ACTION_REPLACE_IN_PATH = "ReplaceInPath";
    ACTION_GOTO_LINE = "GotoLine";
    ACTION_GOTO_ACTION = "GotoAction";
    ACTION_SELECT_NEXT_OCCURRENCE = "SelectNextOccurrence";
    ACTION_UNSELECT_PREVIOUS_OCCURRENCE = "UnselectPreviousOccurrence";
    ACTION_SELECT_ALL_OCCURRENCES = "SelectAllOccurrences";
    ACTION_EDITOR_ESCAPE = "EditorEscape";
    ACTION_EDITOR_ENTER = "EditorEnter";
    ACTION_EDITOR_TAB = "EditorTab";
    ACTION_EDITOR_UNINDENT_SELECTION = "EditorUnindentSelection";
    ACTION_EDITOR_DUPLICATE = "EditorDuplicate";
    ACTION_EDITOR_DELETE_LINE = "EditorDeleteLine";
    ACTION_EDITOR_PREVIOUS_WORD = "EditorPreviousWord";
    ACTION_EDITOR_NEXT_WORD = "EditorNextWord";
    ACTION_COMMENT_BY_LINE = "CommentByLineComment";
    ACTION_SHOW_INTENTION_ACTIONS = "ShowIntentionActions";
    ACTION_QUICK_JAVA_DOC = "QuickJavaDoc";
    ACTION_NEXT_TAB = "NextTab";
    ACTION_PREVIOUS_TAB = "PreviousTab";
    ACTION_SWITCHER = "Switcher";
    ACTION_CLOSE_CONTENT = "CloseContent";
    ACTION_CLOSE_ACTIVE_TAB = "CloseActiveTab";
    ACTION_HIDE_ACTIVE_WINDOW = "HideActiveWindow";
    ACTION_HIDE_ALL_WINDOWS = "HideAllWindows";
    ACTION_JUMP_TO_LAST_WINDOW = "JumpToLastWindow";
    ACTION_FOCUS_EDITOR = "FocusEditor";
    ACTION_SHOW_SEARCH_HISTORY = "ShowSearchHistory";
    ACTION_MAXIMIZE_TOOL_WINDOW = "MaximizeToolWindow";
}
