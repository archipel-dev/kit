//! Controls and containers of the platform layer.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`ComboBox`] | Picking one value from a list. |
//! | [`Splitter`] ([`SplitterResponse`], [`Orientation`], [`Side`]) | Two panes with a draggable divider. |
//! | [`DialogWrapper`] ([`ExitCode`], [`MessageIcon`]) | A modal dialog with its buttons. |
//! | [`Messages`] | Ready-made info, error, OK/cancel and yes/no dialogs. |
//! | [`ValidationInfo`] ([`DialogValidation`]) | What is wrong with a dialog's input, debounced. |

mod combo_box;
mod dialog_wrapper;
mod messages;
mod splitter;
mod validation;

pub use combo_box::ComboBox;
pub use dialog_wrapper::{
    CANCEL_EXIT_CODE, CLOSE_EXIT_CODE, DialogWrapper, ExitCode, MessageIcon, NEXT_USER_EXIT_CODE,
    OK_EXIT_CODE,
};
pub use messages::Messages;
pub use splitter::{Orientation, Side, Splitter, SplitterResponse};
pub use validation::{DialogValidation, ValidationInfo};
