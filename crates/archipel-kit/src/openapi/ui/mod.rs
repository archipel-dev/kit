//! Controls and containers of the platform layer.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`ComboBox`] | Picking one value from a list. |
//! | [`Splitter`] ([`SplitterResponse`], [`Orientation`], [`Side`], [`DividerStyle`], [`LackOfSpaceStrategy`]) | Two panes with a draggable divider. |
//! | [`OnePixelSplitter`], [`JBSplitter`] | A splitter with a 1 pt line (and one that remembers its share). |
//! | [`DialogWrapper`] ([`ExitCode`], [`MessageIcon`]) | A modal dialog with its buttons. |
//! | [`Messages`] | Ready-made info, error, OK/cancel and yes/no dialogs. |
//! | [`ValidationInfo`] ([`DialogValidation`]) | What is wrong with a dialog's input, debounced. |

mod combo_box;
mod dialog_wrapper;
mod messages;
mod one_pixel_splitter;
mod splitter;
mod validation;

pub use combo_box::ComboBox;
pub use dialog_wrapper::{
    CANCEL_EXIT_CODE, CLOSE_EXIT_CODE, DialogWrapper, ExitCode, MessageIcon, NEXT_USER_EXIT_CODE,
    OK_EXIT_CODE,
};
pub use messages::Messages;
pub use one_pixel_splitter::{JBSplitter, OnePixelSplitter};
pub use splitter::{
    DividerStyle, LackOfSpaceStrategy, Orientation, Side, Splitter, SplitterResponse,
};
pub use validation::{DialogValidation, ValidationInfo};
