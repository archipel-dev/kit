//! # Archipel Kit
//!
//! Native UI components for desktop developer tools, built on [egui].
//!
//! ```no_run
//! use archipel_kit::{Theme, ui::Button};
//!
//! # fn ui(ctx: &egui::Context, ui: &mut egui::Ui) {
//! archipel_kit::install(ctx, Theme::dark()); // once, at startup
//!
//! if ui.add(Button::primary("Run")).clicked() {
//!     // ...
//! }
//! # }
//! ```
//!
//! - [`theme`]: colors of the dark and light looks, stored per [`egui::Context`].
//! - [`tokens`]: sizes, radii and typography shared by every component.
//! - [`icons`]: the [`Glyph`]s of the `archipel-icons` kit and file-type [`Icon`]s.
//!
//! The widgets are grouped by the IntelliJ Platform area their names come from:
//!
//! | Module | Contents |
//! |---|---|
//! | [`ui`] | Buttons, text fields, choices, lists, trees, tables, banners, progress, charts, forms. |
//! | [`ui::tabs`] | [`JBTabs`](ui::tabs::JBTabs), [`TabInfo`](ui::tabs::TabInfo). |
//! | [`ui::popup`] | [`JBPopup`](ui::popup::JBPopup), [`ListSeparator`](ui::popup::ListSeparator). |
//! | [`openapi::ui`] | [`ComboBox`](openapi::ui::ComboBox), [`Splitter`](openapi::ui::Splitter), [`DialogWrapper`](openapi::ui::DialogWrapper). |
//! | [`ide`] | [`HelpTooltip`](ide::HelpTooltip). |
//! | [`action_system`] | [`ActionButton`](action_system::ActionButton), [`ActionMenuItem`](action_system::ActionMenuItem). |
//! | [`wm`] | The tool window shell: [`ToolWindowPane`](wm::ToolWindowPane), panels, stripes, status bar, main toolbar. |
//! | [`notification`] | [`Notification`](notification::Notification). |
//! | [`editor`] | [`CodeView`](editor::CodeView), [`CodeEditor`](editor::CodeEditor) and the highlighters. |
//!
//! - [`parts`]: the building blocks of the components, for building your own.
//! - [`behaviour`]: tooltips, context menus, cursors and drag and drop on any widget, with
//!   `.with(...)`.
//! - `testing` (feature `testing`): themed test harnesses and dark + light snapshots, the
//!   same helpers the kit's own tests use.
//!
//! Every component reports its role and label to AccessKit, so screen readers and the
//! testing harness find it by its visible text.

pub mod action_system;
pub mod behaviour;
pub(crate) mod common;
pub mod cursor;
pub mod editor;
pub mod icons;
pub mod ide;
pub mod notification;
pub mod openapi;
pub mod parts;
mod style;
pub mod theme;
pub mod tokens;
pub mod ui;
pub mod wm;

#[cfg(feature = "testing")]
pub mod testing;

pub use egui;
pub use icons::{Glyph, Icon};
pub use parts::{Tone, ValueKind};
pub use style::install;
pub use theme::Theme;

/// The code blocks of the guides and component pages in `docs/`, compiled and run as
/// doctests so the documentation cannot drift from the API.
#[cfg(doctest)]
mod docs {
    macro_rules! doc_pages {
        ($($name:ident = $path:literal;)+) => {
            $(
                #[doc = include_str!(concat!("../../../docs/", $path))]
                struct $name;
            )+
        };
    }

    doc_pages! {
        GettingStarted = "getting-started.md";
        Theming = "theming.md";
        Testing = "testing.md";
        Cursors = "cursors.md";
        Behaviours = "behaviours.md";
        BuildingComponents = "building-components.md";
        Collections = "components/collections.md";
        TreeTable = "components/tree-table.md";
        HelpTooltip = "components/help-tooltip.md";
        DialogWrapper = "components/dialog-wrapper.md";
        IconKit = "icons/README.md";
        Badges = "components/badges.md";
        InlineBanner = "components/inline-banner.md";
        Button = "components/button.md";
        Chart = "components/chart.md";
        Choice = "components/choice.md";
        Code = "components/code.md";
        ComboBox = "components/combo-box.md";
        StatusText = "components/status-text.md";
        ActionButton = "components/action-button.md";
        KeyValueTable = "components/key-value-table.md";
        Icons = "components/icons.md";
        ColoredListCellRenderer = "components/colored-list-cell-renderer.md";
        ActionMenuItem = "components/action-menu-item.md";
        SimpleToolWindowPanel = "components/simple-tool-window-panel.md";
        Progress = "components/progress.md";
        SegmentedButton = "components/segmented-button.md";
        StatusBarWidget = "components/status-bar-widget.md";
        Splitter = "components/splitter.md";
        MainToolbar = "components/window.md";
        ToolWindowPane = "components/tool-window-pane.md";
        JBTabs = "components/jb-tabs.md";
        JBTextField = "components/jb-text-field.md";
        Notification = "components/notification.md";
    }
}
