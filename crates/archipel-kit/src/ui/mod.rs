//! The widgets. Most implement [`egui::Widget`] (add them with `ui.add(...)`); the ones
//! that report more than a [`egui::Response`] have a `show` method.
//!
//! | Group | Items |
//! |---|---|
//! | Actions | [`Button`], [`ActionLink`] |
//! | Text input | [`JBTextField`] ([`JBTextFieldOutput`]), [`SearchTextField`] ([`SearchHistory`]), [`Extension`], [`JBIntSpinner`] ([`Integer`]), [`Completer`], [`Completion`], [`Completions`] |
//! | Choices | [`JBCheckBox`], [`JBRadioButton`], [`OnOffButton`], [`SegmentedButton`] |
//! | Lists and tables | [`JBList`], [`Tree`] ([`TreeNode`]), [`JBTable`] ([`ColumnInfo`], [`Cell`], [`SortOrder`]), [`TreeTable`], [`CollectionResponse`], [`CellEdit`], [`ListSelectionModel`] ([`SelectionMode`], [`SelectionTarget`]), [`SpeedSearch`] ([`SpeedSearchState`], [`SpeedSearchComparator`]) |
//! | Rows | [`ColoredListCellRenderer`] ([`CellRendererResponse`]), [`KeyValueTable`] ([`KeyValueRow`], [`KeyValueEntry`], [`Change`], [`ValueCell`], [`KeyValueTableResponse`]), [`PropertyGrid`] |
//! | Feedback | [`InlineBanner`] ([`Status`], [`MessageResponse`]), [`EditorNotificationPanel`], [`AsyncProcessIcon`], [`ProgressBar`] ([`ProgressState`]), [`StatusText`] ([`StatusTextResponse`]), [`JBLoadingPanel`] |
//! | Navigation | [`BreadcrumbsComponent`] ([`BreadcrumbsComponentResponse`]); [`tabs`]: [`JBTabs`](tabs::JBTabs) |
//! | Data | [`Counter`], [`Tag`], [`Kbd`], [`Avatar`], [`Histogram`], [`Sparkline`], [`Bucket`], [`ChartColor`] |
//! | Layout | [`dsl`]: [`panel`](dsl::panel) ([`Panel`](dsl::Panel), [`Row`](dsl::Row), [`Cell`](dsl::Cell)), [`TitledSeparator`], [`CollapsiblePanel`] ([`CollapsiblePanelResponse`]), [`SeparatorComponent`], [`DropZone`] |
//! | Popups | [`popup`]: [`JBPopup`](popup::JBPopup), [`ListSeparator`](popup::ListSeparator) |

pub(crate) mod activation;
mod badges;
mod breadcrumbs;
mod button;
mod chart;
mod choice;
mod collapsible_panel;
mod collections;
pub(crate) mod completion;
mod drag;
pub mod dsl;
mod editor_notification_panel;
mod feedback;
mod key_value;
mod link;
mod list;
mod loading_panel;
mod number_field;
pub mod popup;
pub(crate) mod popup_rows;
mod progress;
mod property_grid;
mod search_text_field;
mod segmented;
mod selection;
mod separator;
mod speed_search;
mod status_text;
pub mod tabs;
mod text_extensions;
mod text_field;

pub use badges::{Avatar, Counter, Kbd, Tag, initials};
pub use breadcrumbs::{BreadcrumbsComponent, BreadcrumbsComponentResponse};
pub use button::{Button, ButtonVariant};
pub use chart::{Bucket, ChartColor, Histogram, HistogramResponse, Sparkline};
pub use choice::{JBCheckBox, JBRadioButton, OnOffButton, ThreeState, ThreeStateCheckBox};
pub use collapsible_panel::{CollapsiblePanel, CollapsiblePanelResponse};
pub use collections::{
    Cell, CellEdit, CollectionResponse, ColumnInfo, JBList, JBTable, SelectionTarget, SortOrder,
    Tree, TreeNode, TreeTable,
};
pub use completion::{Completer, Completion, Completions};
pub use drag::{DropZone, DropZoneResponse};
pub use editor_notification_panel::EditorNotificationPanel;
pub use feedback::{InlineBanner, MessageResponse, Status};
pub use key_value::{
    Change, KeyValueEntry, KeyValueRow, KeyValueTable, KeyValueTableResponse, ValueCell,
};
pub use link::ActionLink;
pub use list::{CellRendererResponse, ColoredListCellRenderer};
pub use loading_panel::JBLoadingPanel;
pub use number_field::{Integer, JBIntSpinner};
pub use progress::{AsyncProcessIcon, ProgressBar, ProgressState};
pub use property_grid::PropertyGrid;
pub use search_text_field::SearchTextField;
pub use segmented::SegmentedButton;
pub use selection::{ListSelectionModel, SelectionMode};
pub use separator::{SeparatorComponent, TitledSeparator};
pub use speed_search::{PUNCTUATION_MARKS, SpeedSearch, SpeedSearchComparator, SpeedSearchState};
pub use status_text::{StatusText, StatusTextResponse};
pub use text_extensions::{Extension, SearchHistory};
pub use text_field::{JBTextField, JBTextFieldOutput};
