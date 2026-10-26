//! Popups and their lists.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`JBPopup`] | A popup attached to a widget that closes on Escape and outside clicks. |
//! | [`ListSeparator`] | The line (or titled group heading) between the items of a popup list. |

mod jb_popup;
mod list_separator;

pub use jb_popup::JBPopup;
pub use list_separator::ListSeparator;
