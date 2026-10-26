//! Controls and containers of the platform layer.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`ComboBox`] | Picking one value from a list. |
//! | [`Splitter`] ([`SplitterResponse`], [`Orientation`], [`Side`]) | Two panes with a draggable divider. |
//! | [`DialogWrapper`] ([`ExitCode`]) | A modal dialog with its buttons. |

mod combo_box;
mod dialog_wrapper;
mod splitter;

pub use combo_box::ComboBox;
pub use dialog_wrapper::{DialogWrapper, ExitCode};
pub use splitter::{Orientation, Side, Splitter, SplitterResponse};
