//! The IntelliJ-style action system: actions, their presentation and data context, groups,
//! places and keymaps, and the widgets that render them.
//!
//! The model code has no painting. Renderers take the [`Presentation`]s and [`ActionRow`]s it
//! computes. See `docs/action-system.md`.
//!
//! | Widget | Use it for |
//! |---|---|
//! | [`ActionButton`] | The flat icon-only button of toolbars and headers. |
//! | [`ActionMenuItem`] | A row of a menu or popup list (check, icon, shortcut, submenu). |

mod action;
mod action_button;
mod action_menu_item;
mod data_context;
mod group;
mod keymap;
mod places;
mod presentation;

pub use action::{
    ActionUiKind, AnAction, AnActionEvent, InputKind, Toggle, ToggleAction, perform,
    perform_with_modifiers, update_presentation,
};
pub use action_button::ActionButton;
pub use action_menu_item::ActionMenuItem;
pub use data_context::{CommonDataKeys, DataContext, DataContextBuilder, DataKey};
pub use group::{
    ActionGroup, ActionItem, ActionRow, DefaultActionGroup, MAX_GROUP_DEPTH, Separator, flatten,
    flatten_with_shortcuts,
};
pub use keymap::{
    KeyResolution, KeyStroke, KeyboardShortcut, Keymap, KeymapUtil, MouseShortcut, Os,
    PendingStroke, SECOND_KEYSTROKE_TIMEOUT, Shortcut,
};
pub use places::{ActionPlace, IdeActions};
pub use presentation::Presentation;
