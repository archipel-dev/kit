//! # Archipel Kit model
//!
//! The component vocabulary of the Archipel Kit as plain data: for each component a props
//! struct (what to show) and an event enum (what the user did). The model has no `egui`, no
//! `ratatui` and no I/O, so an application can build its whole UI once, as props, and let
//! the egui kit (`archipel-kit`) or the terminal kit (`archipel-kit-tui`) render it. Both
//! kits re-export this crate as `model`.
//!
//! Rules for every type here:
//!
//! - `Clone + PartialEq + Debug + Serialize`, so a test can compare two builds of the same
//!   UI and a snapshot can be written as JSON.
//! - No color, rectangle, pixel or toolkit id. Color is a semantic [`Tone`]; an icon is an
//!   [`IconKey`]; a shortcut is a [`Shortcut`] that the renderer formats for its platform;
//!   sizes are abstract (characters, items), the renderer maps them to pixels or cells.
//! - Each interactive part has a stable string `id`. A renderer reports interaction as an
//!   event naming that id, so the application decides what it means.
//!
//! The components are [`ToolbarButton`], [`ToolbarDropdown`], [`Toolbar`], [`MenuBar`] with
//! [`MenuHeader`], [`MenuItem`] and [`MenuEntry`], [`ToolStripe`] with [`StripeButton`],
//! [`PanelHeader`], [`TabBar`] with [`Tab`], [`StatusBar`] with [`StatusBarItem`] and
//! [`Breadcrumbs`]. [`vocabulary`] lists them with the kit types that render them, and
//! [`outline`] reduces props to the flat list that the render parity tests compare.
//!
//! ```
//! use archipel_kit_model::{MenuBar, MenuEntry, MenuHeader, MenuItem, Shortcut};
//!
//! let bar = MenuBar::new(vec![MenuHeader::new("file", "File").mnemonic(0).entries(vec![
//!     MenuEntry::Item(MenuItem::new("SaveAll", "Save All").shortcut(Shortcut::keys("Ctrl+S"))),
//!     MenuEntry::separator(),
//!     MenuEntry::Item(MenuItem::new("Exit", "Exit")),
//! ])]);
//! let json = serde_json::to_string(&bar).unwrap();
//! assert_eq!(serde_json::from_str::<MenuBar>(&json).unwrap(), bar);
//! ```

mod breadcrumbs;
mod icon;
mod menu;
pub mod outline;
mod panel;
mod shortcut;
mod status;
mod stripe;
mod tabs;
mod tone;
mod toolbar;
pub mod vocabulary;

pub use archipel_icons::Glyph;
pub use breadcrumbs::{BreadcrumbEvent, Breadcrumbs, Crumb};
pub use icon::{Avatar, IconKey, avatar_rgb};
pub use menu::{MenuBar, MenuEntry, MenuEvent, MenuHeader, MenuItem};
pub use outline::{Outline, OutlineEntry};
pub use panel::{PanelHeader, PanelHeaderEvent, PanelTab};
pub use shortcut::{Chord, Notation, Shortcut};
pub use status::{StatusBar, StatusBarEvent, StatusBarItem};
pub use stripe::{StripeAnchor, StripeButton, StripeEvent, ToolStripe};
pub use tabs::{Tab, TabBar, TabEvent};
pub use tone::{Tone, Tooltip};
pub use toolbar::{Toolbar, ToolbarButton, ToolbarDropdown, ToolbarEvent, ToolbarItem};
