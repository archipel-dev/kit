//! The outline of props: a flat list of what a user could see and do, in order. Two
//! renderers that show the same props must produce the same outline; the parity tests of
//! the IDE compare them (`archipel_kit::testing::outline`,
//! `archipel_kit_tui::testing::outline`) and the outline computed directly from the props
//! (this module).

use crate::{
    Breadcrumbs, MenuBar, MenuEntry, Notation, PanelHeader, StatusBar, TabBar, Tone, ToolStripe,
    Toolbar, ToolbarItem,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// One thing in an outline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineEntry {
    /// What it is: `menu`, `menuitem`, `separator`, `button`, `dropdown`, `stripe`, `tab`,
    /// `status`, `crumb`, `title`.
    pub role: String,
    /// The stable id (empty for a separator).
    pub id: String,
    /// The visible text or accessible name.
    pub label: String,
    /// The shortcut as text.
    pub shortcut: Option<String>,
    /// The tooltip as one line.
    pub tooltip: Option<String>,
    /// Whether it reacts to clicks.
    pub enabled: bool,
    /// The checked or selected state, when it has one.
    pub checked: Option<bool>,
    /// Whether it is expanded, when it can be (a submenu).
    pub expanded: Option<bool>,
    /// The badge tone, as text.
    pub badge: Option<String>,
    /// Nesting depth: 0 for a top level entry.
    pub depth: usize,
}

impl OutlineEntry {
    /// An enabled entry with the given role, id and label.
    pub fn new(role: &str, id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            role: role.to_owned(),
            id: id.into(),
            label: label.into(),
            shortcut: None,
            tooltip: None,
            enabled: true,
            checked: None,
            expanded: None,
            badge: None,
            depth: 0,
        }
    }
}

/// An ordered list of [`OutlineEntry`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Outline {
    /// The entries in order.
    pub entries: Vec<OutlineEntry>,
}

impl Outline {
    /// The outline as text, one entry per line, for diffs in test failures.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (order, e) in self.entries.iter().enumerate() {
            let _ = write!(
                out,
                "{order:3} {}{} {} [{}] {:?}",
                "  ".repeat(e.depth),
                e.role,
                e.id,
                if e.enabled { "on" } else { "off" },
                e.label
            );
            if let Some(shortcut) = &e.shortcut {
                let _ = write!(out, " <{shortcut}>");
            }
            if let Some(checked) = e.checked {
                let _ = write!(out, " checked={checked}");
            }
            if let Some(expanded) = e.expanded {
                let _ = write!(out, " expanded={expanded}");
            }
            if let Some(badge) = &e.badge {
                let _ = write!(out, " badge={badge}");
            }
            if let Some(tooltip) = &e.tooltip {
                let _ = write!(out, " tip={tooltip:?}");
            }
            out.push('\n');
        }
        out
    }

    /// A line by line diff of two outlines, or `None` when they are equal.
    pub fn diff(&self, other: &Outline) -> Option<String> {
        if self == other {
            return None;
        }
        let (a, b) = (self.text(), other.text());
        let (a, b): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
        let mut out = String::new();
        for i in 0..a.len().max(b.len()) {
            let (left, right) = (
                a.get(i).copied().unwrap_or("-"),
                b.get(i).copied().unwrap_or("-"),
            );
            if left != right {
                let _ = writeln!(out, "line {i}:\n  left : {left}\n  right: {right}");
            }
        }
        Some(out)
    }
}

fn tone_name(tone: Tone) -> String {
    format!("{tone:?}").to_ascii_lowercase()
}

fn menu_entries(
    entries: &[MenuEntry],
    depth: usize,
    notation: Notation,
    out: &mut Vec<OutlineEntry>,
) {
    for entry in entries {
        match entry {
            MenuEntry::Separator { title } => {
                let mut e = OutlineEntry::new("separator", "", title.clone().unwrap_or_default());
                e.depth = depth;
                out.push(e);
            }
            MenuEntry::Item(item) => {
                let mut e = OutlineEntry::new("menuitem", item.id.clone(), item.label.clone());
                e.depth = depth;
                e.shortcut = item.shortcut.as_ref().map(|s| s.label(notation));
                e.enabled = item.enabled;
                e.checked = item.checked;
                e.expanded = item.submenu.as_ref().map(|_| false);
                e.tooltip = item.disabled_reason.clone();
                out.push(e);
                if let Some(sub) = &item.submenu {
                    menu_entries(sub, depth + 1, notation, out);
                }
            }
        }
    }
}

/// The outline of a menu bar with every menu expanded to its deepest submenu.
pub fn menu_bar(bar: &MenuBar, notation: Notation) -> Outline {
    let mut entries = Vec::new();
    for header in &bar.headers {
        let mut e = OutlineEntry::new("menu", header.id.clone(), header.label.clone());
        e.expanded = Some(false);
        entries.push(e);
        menu_entries(&header.entries, 1, notation, &mut entries);
    }
    Outline { entries }
}

/// The outline of a menu's rows.
pub fn menu_entries_outline(items: &[MenuEntry], notation: Notation) -> Outline {
    let mut entries = Vec::new();
    menu_entries(items, 0, notation, &mut entries);
    Outline { entries }
}

/// The outline of a toolbar.
pub fn toolbar(bar: &Toolbar, notation: Notation) -> Outline {
    let mut entries = Vec::new();
    for item in bar.items() {
        entries.push(match item {
            ToolbarItem::Button(b) => {
                let mut e = OutlineEntry::new("button", b.id.clone(), b.name.clone());
                e.enabled = b.enabled;
                e.checked = b.selected.then_some(true);
                e.badge = b.badge.map(tone_name);
                e.tooltip = b.tooltip.as_ref().map(|t| t.text(notation));
                e.shortcut = b
                    .tooltip
                    .as_ref()
                    .and_then(|t| t.shortcut.as_ref())
                    .map(|s| s.label(notation));
                e
            }
            ToolbarItem::Dropdown(d) => {
                let mut e = OutlineEntry::new("dropdown", d.id.clone(), d.label.clone());
                e.enabled = d.enabled;
                e.tooltip = d.tooltip.as_ref().map(|t| t.text(notation));
                e
            }
            ToolbarItem::Segmented {
                id,
                options,
                selected,
            } => {
                let mut e = OutlineEntry::new("segmented", id.clone(), options.join("|"));
                e.checked = Some(*selected < options.len());
                e
            }
            ToolbarItem::Separator => OutlineEntry::new("separator", "", ""),
        });
    }
    Outline { entries }
}

/// The outline of a stripe: top group, then bottom group.
pub fn stripe(stripe: &ToolStripe, notation: Notation) -> Outline {
    let mut entries = Vec::new();
    for button in stripe.buttons() {
        let mut e = OutlineEntry::new("stripe", button.id.clone(), button.tooltip.title.clone());
        e.checked = Some(button.selected);
        e.badge = button.badge.map(tone_name);
        e.tooltip = Some(button.tooltip.text(notation));
        e.shortcut = button.tooltip.shortcut.as_ref().map(|s| s.label(notation));
        entries.push(e);
    }
    Outline { entries }
}

/// The outline of a panel header.
pub fn panel_header(header: &PanelHeader, notation: Notation) -> Outline {
    let mut entries = vec![OutlineEntry::new(
        "title",
        header.id.clone(),
        header.title.clone(),
    )];
    entries[0].expanded = header.view_selector.then_some(false);
    for tab in &header.tabs {
        let label = match tab.count {
            Some(count) => format!("{} {count}", tab.title),
            None => tab.title.clone(),
        };
        let mut e = OutlineEntry::new("tab", tab.id.clone(), label);
        e.checked = Some(tab.selected);
        entries.push(e);
    }
    for action in &header.actions {
        let mut e = OutlineEntry::new("button", action.id.clone(), action.name.clone());
        e.enabled = action.enabled;
        e.tooltip = action.tooltip.as_ref().map(|t| t.text(notation));
        entries.push(e);
    }
    if header.hide {
        let mut hide = OutlineEntry::new("button", "hide", "Hide");
        hide.tooltip = Some("Hide".to_owned());
        entries.push(hide);
    }
    Outline { entries }
}

/// The outline of a tab strip.
pub fn tab_bar(bar: &TabBar, notation: Notation) -> Outline {
    let mut entries = Vec::new();
    for tab in &bar.tabs {
        let mut e = OutlineEntry::new("tab", tab.id.clone(), tab.title.clone());
        e.checked = Some(tab.selected);
        e.badge = if tab.dirty {
            Some("modified".to_owned())
        } else if tab.pinned {
            Some("pinned".to_owned())
        } else if tab.error {
            Some("error".to_owned())
        } else {
            None
        };
        e.tooltip = tab.tooltip.as_ref().map(|t| t.text(notation));
        entries.push(e);
    }
    Outline { entries }
}

/// The outline of breadcrumbs.
pub fn breadcrumbs(crumbs: &Breadcrumbs, notation: Notation) -> Outline {
    let entries = crumbs
        .crumbs
        .iter()
        .map(|c| {
            let mut e = OutlineEntry::new("crumb", c.id.clone(), c.text.clone());
            e.enabled = c.clickable;
            e.tooltip = c.tooltip.as_ref().map(|t| t.text(notation));
            e
        })
        .collect();
    Outline { entries }
}

/// The outline of a status bar: breadcrumbs first, then the widgets.
pub fn status_bar(bar: &StatusBar, notation: Notation) -> Outline {
    let mut outline = breadcrumbs(&bar.breadcrumbs, notation);
    for item in &bar.items {
        let mut e = OutlineEntry::new("status", item.id.clone(), item.text.clone());
        e.enabled = item.clickable;
        e.tooltip = item.tooltip.as_ref().map(|t| t.text(notation));
        outline.entries.push(e);
    }
    outline
}
