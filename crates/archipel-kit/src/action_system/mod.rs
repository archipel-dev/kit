//! The IntelliJ-style action system: actions, their presentation and data context, groups,
//! places and keymaps.
//!
//! Everything here is model code with no painting. Renderers (toolbars, menus, buttons) take
//! the [`Presentation`]s and [`ActionRow`]s it computes. See `docs/action-system.md`.

mod action;
mod data_context;
mod group;
mod keymap;
mod places;
mod presentation;

pub use action::{
    ActionUiKind, AnAction, AnActionEvent, InputKind, Toggle, ToggleAction, perform,
    perform_with_modifiers, update_presentation,
};
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
