//! The widgets. Most implement [`egui::Widget`] (add them with `ui.add(...)`); the ones
//! that report more than a [`egui::Response`] have a `show` method.
//!
//! | Group | Items |
//! |---|---|
//! | Actions | [`Button`], [`ActionLink`] |
//! | Text input | [`JBTextField`] ([`JBTextFieldOutput`]), [`JBIntSpinner`] ([`Integer`]), [`Completer`], [`Completion`], [`Completions`] |
//! | Choices | [`JBCheckBox`], [`JBRadioButton`], [`OnOffButton`], [`SegmentedButton`] |
//! | Lists and tables | [`JBList`], [`Tree`] ([`TreeNode`]), [`JBTable`] ([`ColumnInfo`], [`Cell`], [`SortOrder`]), [`TreeTable`], [`CollectionResponse`], [`CellEdit`] |
//! | Rows | [`ColoredListCellRenderer`] ([`CellRendererResponse`]), [`KeyValueTable`] ([`KeyValueRow`], [`KeyValueEntry`], [`Change`], [`ValueCell`], [`KeyValueTableResponse`]), [`PropertyGrid`] |
//! | Feedback | [`InlineBanner`] ([`Status`], [`MessageResponse`]), [`AsyncProcessIcon`], [`ProgressBar`], [`StatusText`] |
//! | Navigation | [`BreadcrumbsComponent`] ([`BreadcrumbsComponentResponse`]); [`tabs`]: [`JBTabs`](tabs::JBTabs) |
//! | Data | [`Counter`], [`Tag`], [`Kbd`], [`Avatar`], [`Histogram`], [`Sparkline`], [`Bucket`], [`ChartColor`] |
//! | Layout | [`Form`] ([`FormUi`], [`FieldLabel`]), [`TitledSeparator`], [`SeparatorComponent`], [`DropZone`] |
//! | Popups | [`popup`]: [`JBPopup`](popup::JBPopup), [`ListSeparator`](popup::ListSeparator) |

mod badges;
mod breadcrumbs;
mod button;
mod chart;
mod choice;
mod collections;
pub(crate) mod completion;
mod drag;
mod feedback;
mod form;
mod key_value;
mod link;
mod list;
mod number_field;
pub mod popup;
mod progress;
mod property_grid;
mod segmented;
mod separator;
mod status_text;
pub mod tabs;
mod text_field;

pub use badges::{Avatar, Counter, Kbd, Tag, initials};
pub use breadcrumbs::{BreadcrumbsComponent, BreadcrumbsComponentResponse};
pub use button::{Button, ButtonVariant};
pub use chart::{Bucket, ChartColor, Histogram, HistogramResponse, Sparkline};
pub use choice::{JBCheckBox, JBRadioButton, OnOffButton};
pub use collections::{
    Cell, CellEdit, CollectionResponse, ColumnInfo, JBList, JBTable, SortOrder, Tree, TreeNode,
    TreeTable,
};
pub use completion::{Completer, Completion, Completions};
pub use drag::{DropZone, DropZoneResponse};
pub use feedback::{InlineBanner, MessageResponse, Status};
pub use form::{FieldLabel, Form, FormUi};
pub use key_value::{
    Change, KeyValueEntry, KeyValueRow, KeyValueTable, KeyValueTableResponse, ValueCell,
};
pub use link::ActionLink;
pub use list::{CellRendererResponse, ColoredListCellRenderer};
pub use number_field::{Integer, JBIntSpinner};
pub use progress::{AsyncProcessIcon, ProgressBar};
pub use property_grid::PropertyGrid;
pub use segmented::SegmentedButton;
pub use separator::{SeparatorComponent, TitledSeparator};
pub use status_text::StatusText;
pub use text_field::{JBTextField, JBTextFieldOutput};
