//! The widgets. Most implement [`egui::Widget`] (add them with `ui.add(...)`); the ones
//! that report more than a [`egui::Response`] have a `show` method.
//!
//! | Group | Components |
//! |---|---|
//! | Actions | [`Button`], [`IconButton`], [`Link`], [`MenuItem`] |
//! | Inputs | [`TextField`], [`Checkbox`], [`Radio`], [`Toggle`], [`Dropdown`], [`SegmentedControl`] |
//! | Navigation | [`TabBar`], [`ListRow`] |
//! | Feedback | [`Banner`], [`Toast`], [`Spinner`], [`ProgressBar`], [`EmptyState`] |
//! | Data | [`Counter`], [`Tag`], [`Kbd`], [`Avatar`], [`PropertyGrid`] |
//! | Layout | [`Panel`], [`SectionHeader`], [`Separator`] |

mod badges;
mod button;
mod choice;
mod common;
mod feedback;
mod icon_button;
mod layout;
mod list;
mod misc;
mod progress;
mod segmented;
mod text_field;

pub use badges::{Avatar, Counter, Kbd, Tag, initials};
pub use button::{Button, ButtonVariant};
pub use choice::{Checkbox, Radio, Toggle};
pub use feedback::{Banner, MessageResponse, Severity, Toast};
pub use icon_button::IconButton;
pub use layout::{Panel, SectionHeader, Separator};
pub use list::{ListRow, RowResponse};
pub use misc::{Dropdown, EmptyState, Link, MenuItem, PropertyGrid};
pub use progress::{ProgressBar, Spinner};
pub use segmented::{SegmentedControl, Tab, TabBar, TabBarResponse};
pub use text_field::TextField;
