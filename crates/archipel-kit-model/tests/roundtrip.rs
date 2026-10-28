//! Every props type builds and round-trips through serde (TUI-CMP-001), and the outline of
//! props is stable.

use archipel_icons::Glyph;
use archipel_kit_model::outline;
use archipel_kit_model::*;
use serde::{Serialize, de::DeserializeOwned};

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: T) {
    let json = serde_json::to_string(&value).expect("serializes");
    let back: T = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, value, "{json}");
}

fn button(id: &str) -> ToolbarButton {
    ToolbarButton::new(id, Glyph::Run, "Run")
        .tone(Tone::Run)
        .tooltip(Tooltip::new("Run 'Main'").shortcut(Shortcut::keys("Shift+F10")))
}

#[test]
fn toolbar_props_round_trip() {
    let mut bar = Toolbar::new("main");
    bar.leading.push(ToolbarItem::Button(ToolbarButton::new(
        "menu",
        Glyph::Menu,
        "Main Menu",
    )));
    bar.leading.push(ToolbarItem::Dropdown(
        ToolbarDropdown::new("project", "demo")
            .avatar(Avatar {
                initials: "DM".into(),
                rgb: 0x3574F0,
            })
            .max_chars(14),
    ));
    bar.leading.push(ToolbarItem::Segmented {
        id: "mode".into(),
        options: vec!["Code".into(), "Agents".into()],
        selected: 0,
    });
    bar.trailing.push(ToolbarItem::Button(button("run")));
    round_trip(bar.clone());
    round_trip(ToolbarEvent::Click { id: "run".into() });
    let text = outline::toolbar(&bar, Notation::Text).text();
    assert!(text.contains("Run 'Main' (Shift+F10)"), "{text}");
}

#[test]
fn menu_props_round_trip_and_outline_expands_submenus() {
    let bar = MenuBar::new(vec![
        MenuHeader::new("FileMenu", "File")
            .mnemonic(0)
            .entries(vec![
                MenuEntry::Item(MenuItem::submenu(
                    "New",
                    "New",
                    vec![MenuEntry::Item(MenuItem::new("NewFile", "File"))],
                )),
                MenuEntry::separator(),
                MenuEntry::Item(
                    MenuItem::new("SaveAll", "Save All")
                        .shortcut(Shortcut::keys("Ctrl+S"))
                        .enabled(false),
                ),
                MenuEntry::Item(MenuItem::new("SoftWrap", "Soft-Wrap").checked(true)),
            ]),
    ]);
    round_trip(bar.clone());
    round_trip(MenuEvent::Chosen {
        id: "SaveAll".into(),
    });
    let outline = outline::menu_bar(&bar, Notation::Text);
    let labels: Vec<_> = outline.entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, ["File", "New", "File", "", "Save All", "Soft-Wrap"]);
    assert_eq!(outline.entries[4].shortcut.as_deref(), Some("Ctrl+S"));
    assert!(!outline.entries[4].enabled);
    assert_eq!(outline.entries[5].checked, Some(true));
    assert_eq!(outline.entries[2].depth, 2);
    assert!(outline.diff(&outline).is_none());
}

#[test]
fn stripe_header_tabs_status_round_trip() {
    let mut stripe = ToolStripe::new(StripeAnchor::Left);
    let mut project = StripeButton::new(
        "project",
        Glyph::Project,
        Tooltip::new("Project").shortcut(Shortcut::keys("Alt+1")),
    );
    project.selected = true;
    project.badge = Some(Tone::Error);
    stripe.top.push(project);
    stripe
        .bottom
        .push(StripeButton::new("git", Glyph::Branch, Tooltip::new("Git")));
    round_trip(stripe.clone());
    round_trip(StripeEvent::Toggle {
        id: "project".into(),
    });
    assert_eq!(outline::stripe(&stripe, Notation::Text).entries.len(), 2);

    let mut header = PanelHeader::new("project", "Project");
    header.view_selector = true;
    header
        .actions
        .push(ToolbarButton::new("new", Glyph::Plus, "New"));
    header.tabs.push(PanelTab {
        id: "t".into(),
        title: "File".into(),
        count: Some(1),
        selected: true,
        closable: false,
    });
    round_trip(header.clone());
    round_trip(PanelHeaderEvent::Hide);
    assert_eq!(
        outline::panel_header(&header, Notation::Text).entries.len(),
        4
    );

    let mut tab = Tab::new(
        "src/Main.java",
        "Main.java",
        IconKey::File("Main.java".into()),
    );
    tab.dirty = true;
    let tabs = TabBar::new("editor", vec![tab]);
    round_trip(tabs.clone());
    round_trip(TabEvent::Reorder { from: 0, to: 1 });
    assert_eq!(
        outline::tab_bar(&tabs, Notation::Text).entries[0]
            .badge
            .as_deref(),
        Some("modified")
    );

    let status = StatusBar {
        breadcrumbs: Breadcrumbs {
            crumbs: vec![Crumb::new("p", "demo"), Crumb::new("f", "Main.java")],
        },
        items: vec![StatusBarItem::new("caret", "1:1")],
    };
    round_trip(status.clone());
    round_trip(StatusBarEvent::Click { id: "caret".into() });
    round_trip(BreadcrumbEvent::Click { id: "p".into() });
    assert_eq!(
        outline::status_bar(&status, Notation::Text).entries.len(),
        3
    );
}

#[test]
fn icon_keys_round_trip_by_name() {
    round_trip(IconKey::Glyph(Glyph::ChevronDown));
    round_trip(IconKey::File("a/b.rs".into()));
    round_trip(IconKey::Folder { open: true });
    assert!(
        serde_json::to_string(&IconKey::Glyph(Glyph::ChevronDown))
            .expect("serializes")
            .contains("chevron-down")
    );
}

#[test]
fn every_vocabulary_entry_names_a_pending_or_rendered_component() {
    let names: Vec<_> = vocabulary::VOCABULARY.iter().map(|e| e.name).collect();
    for pending in vocabulary::PENDING_EGUI
        .iter()
        .chain(vocabulary::PENDING_TUI)
    {
        assert!(
            names.contains(pending),
            "{pending} is not in the vocabulary"
        );
    }
}
