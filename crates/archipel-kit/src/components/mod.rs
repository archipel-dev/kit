//! The widgets. Most implement [`egui::Widget`] (add them with `ui.add(...)`); the ones
//! that report more than a [`egui::Response`] have a `show` method.
//!
//! | Group | Components |
//! |---|---|
//! | Actions | [`Button`], [`IconButton`], [`Link`], [`MenuItem`], [`MenuSeparator`] |
//! | Inputs | [`TextField`], [`Checkbox`], [`Radio`], [`Toggle`], [`Dropdown`], [`SegmentedControl`] |
//! | Navigation | [`TabBar`], [`ListRow`] |
//! | Feedback | [`Banner`], [`Toast`], [`Spinner`], [`ProgressBar`], [`EmptyState`] |
//! | Collections | [`VirtualList`], [`Tree`], [`Table`], [`TreeTable`], [`KeyValueTable`] |
//! | Data | [`Counter`], [`Tag`], [`Kbd`], [`Avatar`], [`PropertyGrid`] |
//! | Layout | [`Panel`], [`SectionHeader`], [`Separator`] |
//! | Overlays | [`Tooltip`] ([`TooltipExt`]), [`Dialog`] |

mod badges;
mod button;
mod choice;
mod collections;
pub(crate) mod common;
mod feedback;
mod icon_button;
mod key_value;
mod layout;
mod list;
mod misc;
mod overlay;
mod progress;
mod segmented;
mod text_field;

pub use badges::{Avatar, Counter, Kbd, Tag, initials};
pub use button::{Button, ButtonVariant};
pub use choice::{Checkbox, Radio, Toggle};
pub use collections::{
    Cell, CellEdit, CollectionResponse, Column, SortOrder, Table, Tree, TreeRow, TreeTable,
    VirtualList,
};
pub use feedback::{Banner, MessageResponse, Severity, Toast};
pub use icon_button::IconButton;
pub use key_value::{
    Change, KeyValueEntry, KeyValueRow, KeyValueTable, KeyValueTableResponse, ValueCell,
};
pub use layout::{Panel, SectionHeader, Separator};
pub use list::{ListRow, RowResponse};
pub use misc::{Dropdown, EmptyState, Link, MenuItem, PropertyGrid};
pub use overlay::{Dialog, DialogAction, MenuSeparator, Tooltip, TooltipExt};
pub use progress::{ProgressBar, Spinner};
pub use segmented::{SegmentedControl, Tab, TabBar, TabBarResponse};
pub use text_field::TextField;
