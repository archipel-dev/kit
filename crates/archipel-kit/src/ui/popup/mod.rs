//! Popups and their lists.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`JBPopup`] | A popup attached to a widget that closes on Escape and outside clicks. |
//! | [`ListSeparator`] | The line (or titled group heading) between the items of a popup list. |
//! | [`ListPopup`], [`PopupStep`], [`BaseListPopupStep`] | A list that chooses a value, with sub-steps, keyboard, speed search and timings. |

mod engine;
mod jb_popup;
mod list_popup;
mod list_separator;

pub use engine::PopupChain;
pub(crate) use engine::{
    Chosen, LevelSpec, Options, Output, Placement, PopupSource, RowSpec, drive,
};
pub use list_popup::{BaseListPopupStep, ListPopup, ListPopupResponse, PopupStep, PopupStepResult};

pub use jb_popup::JBPopup;
pub use list_separator::ListSeparator;
