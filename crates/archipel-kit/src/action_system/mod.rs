//! Actions as the user triggers them from toolbars and menus.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`ActionButton`] | The flat icon-only button of toolbars and headers. |
//! | [`ActionMenuItem`] | A row of a menu or popup list (check, icon, shortcut, submenu). |

mod action_button;
mod action_menu_item;

pub use action_button::ActionButton;
pub use action_menu_item::ActionMenuItem;
