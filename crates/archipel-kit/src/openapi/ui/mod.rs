//! Controls and containers of the platform layer.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`ComboBox`] | Picking one value from a list. |
//! | [`Splitter`] ([`SplitterResponse`], [`Orientation`], [`Side`], [`DividerStyle`], [`LackOfSpaceStrategy`]) | Two panes with a draggable divider. |
//! | [`OnePixelSplitter`], [`JBSplitter`] | A splitter with a 1 pt line (and one that remembers its share). |
//! | [`DialogWrapper`] ([`ExitCode`]) | A modal dialog with its buttons. |

mod combo_box;
mod dialog_wrapper;
mod one_pixel_splitter;
mod splitter;

pub use combo_box::ComboBox;
pub use dialog_wrapper::{DialogWrapper, ExitCode};
pub use one_pixel_splitter::{JBSplitter, OnePixelSplitter};
pub use splitter::{
    DividerStyle, LackOfSpaceStrategy, Orientation, Side, Splitter, SplitterResponse,
};
