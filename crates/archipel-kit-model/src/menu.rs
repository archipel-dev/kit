use crate::{IconKey, Shortcut};
use serde::{Deserialize, Serialize};

/// One row of a menu: an action, or a submenu when `submenu` is set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItem {
    /// The action id; a choice reports it.
    pub id: String,
    /// The text, without mnemonic markers.
    pub label: String,
    /// The index (in characters) of the label's mnemonic letter, which is underlined.
    pub mnemonic: Option<usize>,
    /// The icon in the icon column.
    pub icon: Option<IconKey>,
    /// The shortcut, right aligned.
    pub shortcut: Option<Shortcut>,
    /// Whether the row can be chosen. A disabled row is greyed and shows `disabled_reason`
    /// as tooltip.
    pub enabled: bool,
    /// `Some` for a toggle: whether it is on (a check mark in the icon column).
    pub checked: Option<bool>,
    /// Why the row is disabled.
    pub disabled_reason: Option<String>,
    /// The rows of the submenu (an arrow at the right); `None` for an ordinary row.
    pub submenu: Option<Vec<MenuEntry>>,
}

impl MenuItem {
    /// An enabled row.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            mnemonic: None,
            icon: None,
            shortcut: None,
            enabled: true,
            checked: None,
            disabled_reason: None,
            submenu: None,
        }
    }

    /// A row that opens `entries`.
    pub fn submenu(
        id: impl Into<String>,
        label: impl Into<String>,
        entries: Vec<MenuEntry>,
    ) -> Self {
        let mut item = Self::new(id, label);
        item.submenu = Some(entries);
        item
    }

    /// Sets the shortcut.
    #[must_use]
    pub fn shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Sets the mnemonic character index.
    #[must_use]
    pub fn mnemonic(mut self, index: usize) -> Self {
        self.mnemonic = Some(index);
        self
    }

    /// Sets the enabled state.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Makes it a toggle in the given state.
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Sets the icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<IconKey>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// A row of a menu: an item or a separator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MenuEntry {
    /// An action or a submenu.
    Item(MenuItem),
    /// A line, with an optional caption.
    Separator {
        /// The caption.
        title: Option<String>,
    },
}

impl MenuEntry {
    /// A plain separator.
    pub fn separator() -> Self {
        MenuEntry::Separator { title: None }
    }
}

/// The title of one menu of a [`MenuBar`] and its rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuHeader {
    /// Stable id (the reference group id, `FileMenu`).
    pub id: String,
    /// The title.
    pub label: String,
    /// The index of the mnemonic letter in `label`, underlined; `Alt` plus the letter opens
    /// the menu.
    pub mnemonic: Option<usize>,
    /// The rows.
    pub entries: Vec<MenuEntry>,
}

impl MenuHeader {
    /// A menu without rows.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            mnemonic: None,
            entries: Vec::new(),
        }
    }

    /// Sets the mnemonic index.
    #[must_use]
    pub fn mnemonic(mut self, index: usize) -> Self {
        self.mnemonic = Some(index);
        self
    }

    /// Sets the rows.
    #[must_use]
    pub fn entries(mut self, entries: Vec<MenuEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// The mnemonic letter, lowercased.
    pub fn mnemonic_char(&self) -> Option<char> {
        self.mnemonic
            .and_then(|at| self.label.chars().nth(at))
            .map(|c| c.to_ascii_lowercase())
    }
}

/// A row of titles, each with its menu.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MenuBar {
    /// The titles in order.
    pub headers: Vec<MenuHeader>,
}

impl MenuBar {
    /// A bar of `headers`.
    pub fn new(headers: Vec<MenuHeader>) -> Self {
        Self { headers }
    }
}

/// What the user did in a menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MenuEvent {
    /// A row was chosen; the menu closes.
    Chosen {
        /// The row's id.
        id: String,
    },
    /// The menu closed without a choice.
    Closed,
}
