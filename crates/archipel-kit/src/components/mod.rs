//! The widgets. Most implement [`egui::Widget`] (add them with `ui.add(...)`); the ones
//! that report more than a [`egui::Response`] have a `show` method.
//!
//! | Group | Components |
//! |---|---|
//! | Actions | [`Button`], [`IconButton`], [`Link`], [`MenuItem`], [`MenuSeparator`], [`MenuHeader`] |
//! | Inputs | [`TextField`], [`Checkbox`], [`Radio`], [`Toggle`], [`Dropdown`], [`SegmentedControl`] |
//! | Navigation | [`TabBar`], [`ListRow`] |
//! | Feedback | [`Banner`], [`Toast`], [`Spinner`], [`ProgressBar`], [`EmptyState`] |
//! | Collections | [`VirtualList`], [`Tree`], [`Table`], [`TreeTable`], [`KeyValueTable`] |
//! | Data | [`Counter`], [`Tag`], [`Kbd`], [`Avatar`], [`PropertyGrid`], [`Histogram`], [`Sparkline`] |
//! | Code | [`CodeView`], [`CodeEditor`], and the [`code::Highlighter`]s |
//! | Layout | [`Panel`], [`Form`], [`FieldLabel`], [`SectionHeader`], [`Separator`], [`SplitPane`], [`StatusBarItem`] |
//! | Overlays | [`Tooltip`] ([`TooltipExt`]), [`Dialog`] |

mod badges;
mod button;
mod chart;
mod choice;
pub mod code;
mod collections;
pub(crate) mod common;
mod feedback;
mod form;
mod icon_button;
mod key_value;
mod layout;
mod list;
mod misc;
mod overlay;
mod progress;
mod segmented;
mod split_pane;
mod status_bar;
mod stripe;
mod text_field;
mod toolbar;
mod window;
mod workbench;

pub use badges::{Avatar, Counter, Kbd, Tag, initials};
pub use button::{Button, ButtonVariant};
pub use chart::{Bucket, ChartColor, Histogram, HistogramResponse, Sparkline};
pub use choice::{Checkbox, Radio, Toggle};
pub use code::{CodeEditor, CodeView};
pub use collections::{
    Cell, CellEdit, CollectionResponse, Column, SortOrder, Table, Tree, TreeRow, TreeTable,
    VirtualList,
};
pub use feedback::{Banner, MessageResponse, Severity, Toast};
pub use form::{FieldLabel, Form, FormUi};
pub use icon_button::IconButton;
pub use key_value::{
    Change, KeyValueEntry, KeyValueRow, KeyValueTable, KeyValueTableResponse, ValueCell,
};
pub use layout::{Panel, SectionHeader, Separator};
pub use list::{ListRow, RowResponse};
pub use misc::{Dropdown, EmptyState, Link, MenuItem, PropertyGrid};
pub use overlay::{Dialog, DialogAction, MenuHeader, MenuSeparator, Tooltip, TooltipExt};
pub use progress::{ProgressBar, Spinner};
pub use segmented::{SegmentedControl, Tab, TabBar, TabBarResponse};
pub use split_pane::{Orientation, Side, SplitPane, SplitResponse};
pub use status_bar::{Breadcrumbs, BreadcrumbsResponse, StatusBarItem};
pub use stripe::{StripeButton, ToolStripe};
pub use text_field::{TextField, TextFieldOutput};
pub use toolbar::ToolbarButton;
pub use window::{WindowChrome, WindowToolbar};
pub use workbench::{Workbench, WorkbenchRegion, WorkbenchState};
