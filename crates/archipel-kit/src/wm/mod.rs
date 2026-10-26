//! The window manager: the IDE frame, its tool windows and their chrome.
//!
//! | Item | Use it for |
//! |---|---|
//! | [`ToolWindowPane`] ([`ToolWindowPaneState`], [`ToolWindowAnchor`], [`ToolWindowPlace`]) | The docked islands shell. |
//! | [`SimpleToolWindowPanel`] | An island with a header: views, tabs and actions. |
//! | [`ToolWindowToolbar`], [`SquareStripeButton`] | The tool window stripes at the window edges. |
//! | [`StatusBar`] ([`StatusBarWidgetItem`], [`StatusBarAnchor`], [`TextPresentation`], [`IconPresentation`], [`MultipleTextValuesPresentation`]) | The status bar with its anchored widgets. |
//! | [`StatusBarWidget`] | A status bar item. |
//! | [`ToolWindow`] ([`Content`], [`ContentManager`], [`ToolWindowEvent`]) | Caller-owned tool window state. |
//! | [`MainToolbar`], [`IdeFrameDecorator`] | The main toolbar and the window frame. |
//! | [`ToolbarComboButton`] | A main toolbar control that opens a list. |
//! | [`ToolbarSplitButton`] | The run widget: an action and a list in one button. |

pub(crate) mod action_rows;
mod simple_tool_window_panel;
mod status_bar;
mod status_bar_widget;
mod stripe;
mod tool_window;
mod tool_window_pane;
mod toolbar_combo_button;
mod toolbar_split_button;
mod window;

pub use simple_tool_window_panel::SimpleToolWindowPanel;
pub use status_bar::{
    IconPresentation, MultipleTextValuesPresentation, StatusBar, StatusBarAnchor,
    StatusBarResponse, StatusBarWidgetItem, TextPresentation, WidgetPresentation,
};
pub use status_bar_widget::StatusBarWidget;
pub use stripe::{SquareStripeButton, StripeResponse, ToolWindowToolbar};
pub use tool_window::{Content, ContentManager, ToolWindow, ToolWindowEvent};
pub use tool_window_pane::{
    ToolWindowAnchor, ToolWindowPane, ToolWindowPaneState, ToolWindowPlace,
};
pub use toolbar_combo_button::ToolbarComboButton;
pub use toolbar_split_button::{ToolbarSplitButton, ToolbarSplitResponse};
pub use window::{IdeFrameDecorator, MainToolbar};
