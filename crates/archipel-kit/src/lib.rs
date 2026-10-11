//! # Archipel Kit
//!
//! Native UI components for desktop developer tools, built on [egui].
//!
//! ```no_run
//! use archipel_kit::{Theme, components::Button};
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
//! - [`components`]: the widgets.
//! - [`parts`]: the building blocks of the components, for building your own.
//! - [`behaviour`]: tooltips, context menus, cursors and drag and drop on any widget, with
//!   `.with(...)`.
//! - `testing` (feature `testing`): themed test harnesses and dark + light snapshots, the
//!   same helpers the kit's own tests use.
//!
//! Every component reports its role and label to AccessKit, so screen readers and the
//! testing harness find it by its visible text.

pub mod behaviour;
pub mod components;
pub mod cursor;
pub mod icons;
pub mod parts;
mod style;
pub mod theme;
pub mod tokens;

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
        Tooltip = "components/tooltip.md";
        DialogPage = "components/dialog.md";
        IconKit = "icons/README.md";
        Badges = "components/badges.md";
        Banner = "components/banner.md";
        Button = "components/button.md";
        Choice = "components/choice.md";
        Dropdown = "components/dropdown.md";
        EmptyState = "components/empty-state.md";
        IconButton = "components/icon-button.md";
        Icons = "components/icons.md";
        ListRow = "components/list-row.md";
        MenuItem = "components/menu-item.md";
        Panel = "components/panel.md";
        Progress = "components/progress.md";
        SegmentedControl = "components/segmented-control.md";
        StatusBarItem = "components/status-bar-item.md";
        SplitPane = "components/split-pane.md";
        TabBar = "components/tab-bar.md";
        TextField = "components/text-field.md";
        Toast = "components/toast.md";
    }
}
