//! The GUI-faithful chrome on one page: main toolbar with the hamburger menu, tool window
//! stripes, a tool window header, the editor tab strip and the status bar, all drawn from
//! the props of `archipel-kit-model`.
//!
//! `cargo run -p archipel-kit-tui --example chrome`
//!
//! Click the hamburger to expand the menu bar, hover to switch menus, click anything to
//! see its event in the status bar. `Ctrl+Q` quits. Use a Nerd Font and a terminal with
//! 24-bit colors.

#![allow(missing_docs)]

use archipel_kit_model::*;
use archipel_kit_tui::bar::ToolbarControl;
use archipel_kit_tui::header::PanelHeaderControl;
use archipel_kit_tui::input::Event;
use archipel_kit_tui::menubar::MenuBarControl;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::statusbar::StatusItemsControl;
use archipel_kit_tui::stripe::ToolStripeControl;
use archipel_kit_tui::tabbar::TabBarControl;
use archipel_kit_tui::{Glyph, Id, Look, Runtime, Theme, runtime};

struct Chrome {
    toolbar: ToolbarControl,
    menu: MenuBarControl,
    left: ToolStripeControl,
    header: PanelHeaderControl,
    tabs: TabBarControl,
    status: StatusItemsControl,
    toolbar_props: Toolbar,
    menu_props: MenuBar,
    stripe: ToolStripe,
    tab_props: TabBar,
    message: String,
}

fn toolbar() -> Toolbar {
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
    let mut vcs = ToolbarDropdown::new("vcs", "main").icon(Glyph::Branch);
    vcs.shrink_order = 1;
    bar.leading.push(ToolbarItem::Dropdown(vcs));
    bar.leading.push(ToolbarItem::Segmented {
        id: "mode".into(),
        options: vec!["Code".into(), "Agents".into()],
        selected: 0,
    });
    bar.trailing.push(ToolbarItem::Dropdown(
        ToolbarDropdown::new("config", "Main").max_chars(16),
    ));
    for (id, glyph, name, tone) in [
        ("run", Glyph::Run, "Run", Tone::Run),
        ("build", Glyph::Build, "Build Project", Tone::Default),
        ("debug", Glyph::Debug, "Debug", Tone::Run),
        ("search", Glyph::Search, "Search Everywhere", Tone::Default),
        ("settings", Glyph::Settings, "Settings", Tone::Default),
    ] {
        bar.trailing.push(ToolbarItem::Button(
            ToolbarButton::new(id, glyph, name).tone(tone),
        ));
    }
    bar
}

fn menu() -> MenuBar {
    let item = |id: &str, label: &str, keys: Option<&str>| {
        let mut item = MenuItem::new(id, label);
        item.shortcut = keys.map(Shortcut::keys);
        MenuEntry::Item(item)
    };
    let header = |id: &str, label: &str, entries: Vec<MenuEntry>| {
        MenuHeader::new(id, label).mnemonic(0).entries(entries)
    };
    MenuBar::new(vec![
        header(
            "FileMenu",
            "File",
            vec![
                MenuEntry::Item(MenuItem::submenu(
                    "New",
                    "New",
                    vec![
                        item("NewProject", "Project…", None),
                        item("NewFile", "File", None),
                    ],
                )),
                item("OpenFile", "Open…", None),
                MenuEntry::separator(),
                item("ShowSettings", "Settings…", Some("Ctrl+Alt+S")),
                item("SaveAll", "Save All", Some("Ctrl+S")),
                item("Exit", "Exit", None),
            ],
        ),
        header(
            "EditMenu",
            "Edit",
            vec![item("Undo", "Undo", Some("Ctrl+Z"))],
        ),
        header("HelpMenu", "Help", vec![item("About", "About", None)]),
    ])
}

fn stripe() -> ToolStripe {
    let mut s = ToolStripe::new(StripeAnchor::Left);
    let button = |id: &str, glyph: Glyph, title: &str, keys: &str| {
        StripeButton::new(
            id,
            glyph,
            Tooltip::new(title).shortcut(Shortcut::keys(keys)),
        )
    };
    let mut project = button("project", Glyph::Project, "Project", "Alt+1");
    project.selected = true;
    project.focused = true;
    s.top = vec![project, button("commit", Glyph::Commit, "Commit", "Alt+0")];
    let mut problems = button("problems", Glyph::Problems, "Problems", "Alt+6");
    problems.badge = Some(Tone::Error);
    s.bottom = vec![
        button("git", Glyph::Branch, "Git", "Alt+9"),
        problems,
        button("terminal", Glyph::Terminal, "Terminal", "Alt+F12"),
    ];
    s
}

fn tabs() -> TabBar {
    let mut main = Tab::new("Main.java", "Main.java", IconKey::File("Main.java".into()));
    main.selected = true;
    let mut util = Tab::new("Util.java", "Util.java", IconKey::File("Util.java".into()));
    util.dirty = true;
    let mut readme = Tab::new("README.md", "README.md", IconKey::File("README.md".into()));
    readme.preview = true;
    TabBar::new("editor", vec![main, util, readme])
}

fn status(message: &str) -> StatusBar {
    let mut bar = StatusBar::default();
    let mut project = Crumb::new("project", "demo");
    project.icon = Some(Glyph::Project.into());
    bar.breadcrumbs.crumbs = vec![
        project,
        Crumb::new("src", "src"),
        Crumb::new("Main", "Main"),
    ];
    let mut text = StatusBarItem::new("message", message);
    text.tone = Tone::Info;
    text.elide = true;
    text.priority = 30;
    let mut caret = StatusBarItem::new("caret", "12:30");
    caret.priority = 90;
    bar.items = vec![
        text,
        caret,
        StatusBarItem::new("separator", "LF"),
        StatusBarItem::new("encoding", "UTF-8"),
        StatusBarItem::new("indent", "4 spaces"),
    ];
    bar
}

impl archipel_kit_tui::App for Chrome {
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        let props = self.menu_props.clone();
        for e in self.menu.handle(rt, &props, event) {
            self.message = format!("{e:?}");
        }
        if self.menu.is_open() {
            return;
        }
        for e in self.toolbar.handle(rt, event) {
            if matches!(&e, ToolbarEvent::Click { id } if id == "menu") {
                self.menu.expand();
            }
            self.message = format!("{e:?}");
        }
        for e in self.left.handle(rt, event) {
            self.message = format!("{e:?}");
        }
        for e in self.header.handle(rt, event) {
            self.message = format!("{e:?}");
        }
        let tabs = self.tab_props.clone();
        for e in self.tabs.handle(rt, &tabs, event) {
            if let TabEvent::Select { id } = &e {
                for tab in &mut self.tab_props.tabs {
                    tab.selected = tab.id == *id;
                }
            }
            self.message = format!("{e:?}");
        }
        for e in self.status.handle(rt, event) {
            self.message = format!("{e:?}");
        }
        if rt.is_action("Exit") {
            rt.quit = true;
        }
    }

    fn tick(&mut self, rt: &mut Runtime) {
        self.menu.tick(rt);
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        if area.height < 8 || area.width < 40 {
            return;
        }
        let colors = rt.look.theme.colors;
        let island = rt.look.over(colors.island, colors.island);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(island);
            }
        }
        let bar = Rect::new(0, 0, area.width, 1);
        if self.menu.is_expanded() {
            // The hamburger stays; the titles replace the project and VCS widgets.
            self.toolbar.draw(
                rt,
                &Toolbar {
                    leading: vec![self.toolbar_props.leading[0].clone()],
                    ..self.toolbar_props.clone()
                },
                bar,
                buf,
            );
            self.menu.draw(rt, &self.menu_props, bar, 4, buf);
        } else {
            self.toolbar.draw(rt, &self.toolbar_props, bar, buf);
        }
        let body = Rect::new(0, 1, area.width, area.height.saturating_sub(2));
        self.left
            .draw(rt, &self.stripe, Rect::new(0, body.y, 3, body.height), buf);
        let mut header = PanelHeader::new("project", "Project");
        header.view_selector = true;
        header.focused = true;
        for (id, glyph, name) in [
            ("new", Glyph::Plus, "New"),
            ("expand", Glyph::ExpandAll, "Expand All"),
            ("options", Glyph::MoreVertical, "Options"),
        ] {
            header.actions.push(ToolbarButton::new(id, glyph, name));
        }
        let side = 28.min(area.width / 3);
        self.header
            .draw(rt, &header, Rect::new(3, body.y, side, 1), buf);
        self.tabs.draw(
            rt,
            &self.tab_props,
            Rect::new(3 + side, body.y, area.width.saturating_sub(3 + side), 1),
            buf,
        );
        let status_bar = status(&self.message);
        self.status.draw(
            rt,
            &status_bar,
            Rect::new(0, area.bottom() - 1, area.width, 1),
            buf,
        );
        self.menu.draw_popup(rt, buf);
        self.header.draw_popup(rt, buf);
        self.tabs.draw_popup(rt, buf);
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Chrome {
        toolbar: ToolbarControl::new(Id::new("c.toolbar")),
        menu: MenuBarControl::new(Id::new("c.menu")),
        left: ToolStripeControl::new(Id::new("c.stripe")),
        header: PanelHeaderControl::new(Id::new("c.header")),
        tabs: TabBarControl::new(Id::new("c.tabs")),
        status: StatusItemsControl::new(Id::new("c.status")),
        toolbar_props: toolbar(),
        menu_props: menu(),
        stripe: stripe(),
        tab_props: tabs(),
        message: "Click something".into(),
    };
    let look = Look::gui(Theme::dark());
    runtime::run_app_with_keymap(look, &mut app, runtime::RunOptions::default(), |os| {
        archipel_kit_tui::input::terminal_keymap(os)
    })
}
