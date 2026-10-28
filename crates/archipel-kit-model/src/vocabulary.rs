//! The component vocabulary (the product's "one UI layer, two renderers" table): every
//! component with its props type and the kit types that render it. The kits check their
//! side of this list in tests; [`PENDING`] names what a kit does not render yet, and its
//! length only goes down.

/// One component of the vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The vocabulary name.
    pub name: &'static str,
    /// The props type in this crate.
    pub props: &'static str,
    /// The event type in this crate.
    pub event: &'static str,
}

/// The components that have props in the model.
pub const VOCABULARY: &[Entry] = &[
    Entry {
        name: "ToolbarButton",
        props: "ToolbarButton",
        event: "ToolbarEvent",
    },
    Entry {
        name: "ToolbarDropdown",
        props: "ToolbarDropdown",
        event: "ToolbarEvent",
    },
    Entry {
        name: "Toolbar",
        props: "Toolbar",
        event: "ToolbarEvent",
    },
    Entry {
        name: "MenuBar",
        props: "MenuBar",
        event: "MenuEvent",
    },
    Entry {
        name: "MenuHeader",
        props: "MenuHeader",
        event: "MenuEvent",
    },
    Entry {
        name: "MenuItem",
        props: "MenuItem",
        event: "MenuEvent",
    },
    Entry {
        name: "MenuSeparator",
        props: "MenuEntry",
        event: "MenuEvent",
    },
    Entry {
        name: "ToolStripe",
        props: "ToolStripe",
        event: "StripeEvent",
    },
    Entry {
        name: "StripeButton",
        props: "StripeButton",
        event: "StripeEvent",
    },
    Entry {
        name: "PanelHeader",
        props: "PanelHeader",
        event: "PanelHeaderEvent",
    },
    Entry {
        name: "TabBar",
        props: "TabBar",
        event: "TabEvent",
    },
    Entry {
        name: "Tab",
        props: "Tab",
        event: "TabEvent",
    },
    Entry {
        name: "StatusBar",
        props: "StatusBar",
        event: "StatusBarEvent",
    },
    Entry {
        name: "StatusBarItem",
        props: "StatusBarItem",
        event: "StatusBarEvent",
    },
    Entry {
        name: "Breadcrumbs",
        props: "Breadcrumbs",
        event: "BreadcrumbEvent",
    },
];

/// The components the egui kit (`archipel-kit`) does not render from these props yet: its
/// own components cover them with their own builders (`ActionButton`, `JBTabs`, hand
/// built chrome in the IDE). Each entry goes when the egui component takes the props.
pub const PENDING_EGUI: &[&str] = &[
    "ToolbarDropdown",
    "Toolbar",
    "MenuBar",
    "MenuHeader",
    "MenuItem",
    "MenuSeparator",
    "ToolStripe",
    "StripeButton",
    "PanelHeader",
    "TabBar",
    "Tab",
    "StatusBar",
    "StatusBarItem",
    "Breadcrumbs",
    "ToolbarButton",
];

/// The components the terminal kit does not render yet: none.
pub const PENDING_TUI: &[&str] = &[];
