//! Editor tabs.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`JBTabs`] ([`JBTabsResponse`]) | A strip of tabs with scrolling, a hidden-tabs list and a more menu. |
//! | [`TabInfo`] | The description of one tab: label, icon, closable, modified. |

mod jb_tabs;

pub use jb_tabs::{JBTabs, JBTabsResponse, TabInfo};
