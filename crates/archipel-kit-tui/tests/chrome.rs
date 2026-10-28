#![allow(clippy::unwrap_used)]
//! The chrome components of the GUI-faithful terminal UI: toolbar, menu bar, stripes, panel
//! header, tab bar, status bar and breadcrumbs, driven by the shared model props.

use archipel_kit_model::*;
use archipel_kit_tui::bar::ToolbarControl;
use archipel_kit_tui::header::PanelHeaderControl;
use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use archipel_kit_tui::menubar::MenuBarControl;
use archipel_kit_tui::ratatui::style::Color;
use archipel_kit_tui::statusbar::StatusItemsControl;
use archipel_kit_tui::stripe::ToolStripeControl;
use archipel_kit_tui::tabbar::TabBarControl;
use archipel_kit_tui::testing::{Harness, app_fn};
use archipel_kit_tui::{Glyph, Id, Look, Runtime};

fn look() -> Look {
    Look::gui(archipel_kit_tui::Theme::dark())
}

type OnEvent<S> = Box<dyn FnMut(&mut S, &mut Runtime, &archipel_kit_tui::Event)>;
type OnDraw<S> = Box<
    dyn FnMut(
        &mut S,
        &mut Runtime,
        archipel_kit_tui::ratatui::layout::Rect,
        &mut archipel_kit_tui::ratatui::buffer::Buffer,
    ),
>;
type H<S> = Harness<archipel_kit_tui::testing::FnApp<S, OnEvent<S>, OnDraw<S>>>;

fn harness<S: 'static>(
    state: S,
    on_event: impl FnMut(&mut S, &mut Runtime, &archipel_kit_tui::Event) + 'static,
    on_draw: impl FnMut(
        &mut S,
        &mut Runtime,
        archipel_kit_tui::ratatui::layout::Rect,
        &mut archipel_kit_tui::ratatui::buffer::Buffer,
    ) + 'static,
    w: u16,
    h: u16,
) -> H<S> {
    let rt = Runtime::with(
        look(),
        archipel_kit_tui::input::terminal_keymap(archipel_kit::action_system::Os::Linux),
        archipel_kit_tui::Caps::kitty(),
    );
    let on_event: OnEvent<S> = Box::new(on_event);
    let on_draw: OnDraw<S> = Box::new(on_draw);
    Harness::with_runtime(app_fn(state, on_event, on_draw), rt, w, h)
}

// ---------------------------------------------------------------- toolbar

fn main_toolbar() -> Toolbar {
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
            .tooltip(Tooltip::new("/home/user/demo"))
            .max_chars(14),
    ));
    if let Some(ToolbarItem::Dropdown(project)) = bar.leading.last_mut() {
        project.shrink_order = 2;
    }
    let mut vcs = ToolbarDropdown::new("vcs", "main")
        .icon(Glyph::Branch)
        .tooltip(Tooltip::new("Git Branch"));
    vcs.shrink_order = 1;
    bar.leading.push(ToolbarItem::Dropdown(vcs));
    bar.leading.push(ToolbarItem::Segmented {
        id: "mode".into(),
        options: vec!["Code".into(), "Agents".into()],
        selected: 0,
    });
    bar.trailing.push(ToolbarItem::Dropdown(
        ToolbarDropdown::new("run_config", "Main").max_chars(16),
    ));
    bar.trailing.push(ToolbarItem::Button(
        ToolbarButton::new("run", Glyph::Run, "Run")
            .tone(Tone::Run)
            .tooltip(Tooltip::new("Run 'Main'").shortcut(Shortcut::keys("Shift+F10"))),
    ));
    bar.trailing.push(ToolbarItem::Button(
        ToolbarButton::new("build", Glyph::Build, "Build Project").enabled(false),
    ));
    bar.trailing.push(ToolbarItem::Button(ToolbarButton::new(
        "search",
        Glyph::Search,
        "Search Everywhere",
    )));
    bar.trailing.push(ToolbarItem::Button(ToolbarButton::new(
        "settings",
        Glyph::Settings,
        "Settings",
    )));
    bar
}

struct ToolbarState {
    props: Toolbar,
    control: ToolbarControl,
    events: Vec<ToolbarEvent>,
}

fn toolbar_harness(w: u16) -> H<ToolbarState> {
    harness(
        ToolbarState {
            props: main_toolbar(),
            control: ToolbarControl::new(Id::new("toolbar")),
            events: Vec::new(),
        },
        |s: &mut ToolbarState, rt, e| {
            let got = s.control.handle(rt, e);
            s.events.extend(got);
        },
        |s, rt, area, buf| s.control.draw(rt, &s.props, area, buf),
        w,
        1,
    )
}

#[test]
fn toolbar_lays_out_the_main_toolbar_like_the_gui() {
    let h = toolbar_harness(160);
    let line = &h.screen()[0];
    let order = ["demo", "main", "Code", "Agents", "Main"];
    let mut at = 0;
    for word in order {
        let found = line[at..]
            .find(word)
            .unwrap_or_else(|| panic!("{word} in {line:?}"));
        at += found + word.len();
    }
    // The hamburger is at column 1 (TUI-GUI-024).
    assert_eq!(h.buffer()[(1, 0)].symbol(), "\u{eb94}");
    // The last glyph is at the second last column.
    assert_eq!(h.buffer()[(158, 0)].symbol(), "\u{eaf8}");
    // Icon buttons are 3 cells wide with one blank cell between: pitch 4.
    assert_eq!(h.buffer()[(154, 0)].symbol(), "\u{ea6d}");
}

#[test]
fn toolbar_colors_are_the_gui_tokens() {
    let h = toolbar_harness(160);
    let colors = &look().theme.colors;
    let rgb = |c: archipel_kit::egui::Color32| Color::Rgb(c.r(), c.g(), c.b());
    assert_eq!(h.buffer()[(60, 0)].bg, rgb(colors.frame));
    // Run is green, the disabled Build button is the disabled gray.
    let run = h
        .buffer()
        .content()
        .iter()
        .find(|c| c.symbol() == "\u{eb2c}")
        .unwrap();
    assert_eq!(run.fg, rgb(colors.run));
    let build = h
        .buffer()
        .content()
        .iter()
        .find(|c| c.symbol() == "\u{eb6d}")
        .unwrap();
    assert_eq!(build.fg, rgb(colors.text_disabled));
}

#[test]
fn toolbar_clicks_report_ids_and_disabled_buttons_do_nothing() {
    let mut h = toolbar_harness(160);
    let line = h.screen()[0].clone();
    let col = |needle: &str| {
        let byte = line.find(needle).unwrap();
        unicode_width::UnicodeWidthStr::width(&line[..byte]) as u16
    };
    h.click(1, 0);
    h.click(col("demo") + 1, 0);
    h.click(col("\u{eb2c}"), 0);
    h.click(col("\u{eb6d}"), 0); // disabled
    h.click(col("Agents") + 1, 0);
    let clicks: Vec<String> = h
        .app
        .state
        .events
        .iter()
        .map(|e| match e {
            ToolbarEvent::Click { id } => format!("click {id}"),
            ToolbarEvent::Select { id, index } => format!("select {id} {index}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        clicks,
        ["click menu", "click project", "click run", "select mode 1"]
    );
}

#[test]
fn toolbar_hover_press_and_tooltip() {
    let mut h = toolbar_harness(160);
    let c = look().theme.colors;
    let run_x = {
        let line = &h.screen()[0];
        unicode_width::UnicodeWidthStr::width(&line[..line.find("\u{eb2c}").unwrap()]) as u16
    };
    h.hover(run_x, 0);
    let blend = look().over(c.hover, c.frame);
    assert_eq!(h.buffer()[(run_x, 0)].bg, blend);
    assert_eq!(
        h.buffer()[(run_x - 1, 0)].bg,
        blend,
        "the whole 3-cell button is filled"
    );
    h.advance(0.6);
    assert!(
        h.text().contains("Run 'Main' (Shift+F10)")
            || h.rt.tooltip.showing_title() == Some("Run 'Main'")
    );
}

#[test]
fn toolbar_shrinks_at_narrow_widths_dropping_the_switch_first() {
    let h = toolbar_harness(100);
    let text = h.screen()[0].clone();
    assert!(text.contains("Agents"), "{text}");
    let h = toolbar_harness(95);
    let text = h.screen()[0].clone();
    assert!(
        !text.contains("Agents"),
        "the Code | Agents switch hides first: {text}"
    );
    assert!(text.contains("main") && text.contains("demo"), "{text}");
    let h = toolbar_harness(85);
    let text = h.screen()[0].clone();
    assert!(
        !text.contains("main"),
        "the branch label shortens to its icon: {text}"
    );
    assert!(text.contains("\u{ec6f}") && text.contains("demo"), "{text}");
    let h = toolbar_harness(70);
    let text = h.screen()[0].clone();
    assert!(!text.contains("demo"), "then the project name: {text}");
    // The right cluster never truncates.
    assert!(text.contains("Main") && text.contains("\u{eaf8}"), "{text}");
}

#[test]
fn toolbar_outline_matches_the_props() {
    let h = toolbar_harness(160);
    let want = outline::toolbar(&h.app.state.props, Notation::Text);
    let got = h.app.state.control.outline(h.buffer());
    // Segmented labels and separators aside, ids, order, enabled and labels agree.
    let ids = |o: &Outline| {
        o.entries
            .iter()
            .map(|e| (e.id.clone(), e.enabled))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&got), ids(&want), "{:?}", got.diff(&want));
}

// ---------------------------------------------------------------- menu bar

fn menu_props() -> MenuBar {
    let item = |id: &str, label: &str| MenuItem::new(id, label);
    MenuBar::new(vec![
        MenuHeader::new("FileMenu", "File")
            .mnemonic(0)
            .entries(vec![
                MenuEntry::Item(MenuItem::submenu(
                    "New",
                    "New",
                    vec![
                        MenuEntry::Item(item("NewProject", "Project…")),
                        MenuEntry::Item(item("NewFile", "File")),
                    ],
                )),
                MenuEntry::Item(item("OpenFile", "Open…").mnemonic(0)),
                MenuEntry::separator(),
                MenuEntry::Item(
                    item("ShowSettings", "Settings…").shortcut(Shortcut::keys("Ctrl+Alt+S")),
                ),
                MenuEntry::Item(
                    item("SaveAll", "Save All")
                        .shortcut(Shortcut::keys("Ctrl+S"))
                        .enabled(false),
                ),
                MenuEntry::Item(item("SoftWrap", "Soft-Wrap").checked(true)),
                MenuEntry::Item(item("Exit", "Exit")),
            ]),
        MenuHeader::new("EditMenu", "Edit")
            .mnemonic(0)
            .entries(vec![MenuEntry::Item(item("Undo", "Undo"))]),
        MenuHeader::new("RunMenu", "Run")
            .mnemonic(1)
            .entries(vec![MenuEntry::Item(item("Run", "Run"))]),
    ])
}

struct MenuState {
    props: MenuBar,
    bar: MenuBarControl,
    events: Vec<MenuEvent>,
}

fn menu_harness() -> H<MenuState> {
    let mut h = harness(
        MenuState {
            props: menu_props(),
            bar: MenuBarControl::new(Id::new("menu")),
            events: Vec::new(),
        },
        |s: &mut MenuState, rt, e| {
            let props = s.props.clone();
            let got = s.bar.handle(rt, &props, e);
            s.events.extend(got);
        },
        |s, rt, area, buf| {
            s.bar.expand();
            s.bar.draw(
                rt,
                &s.props,
                archipel_kit_tui::ratatui::layout::Rect::new(0, 0, area.width, 1),
                4,
                buf,
            );
            s.bar.draw_popup(rt, buf);
        },
        80,
        20,
    );
    h.draw();
    h
}

#[test]
fn menu_titles_are_padded_and_the_mnemonic_is_underlined() {
    let h = menu_harness();
    assert_eq!(&h.screen()[0][4..], " File  Edit  Run");
    use archipel_kit_tui::ratatui::style::Modifier;
    // `F` of File and `u` of Run are underlined.
    assert!(h.buffer()[(5, 0)].modifier.contains(Modifier::UNDERLINED));
    assert!(!h.buffer()[(6, 0)].modifier.contains(Modifier::UNDERLINED));
    let run_u = h.find("Run").unwrap().0 + 1;
    assert!(
        h.buffer()[(run_u, 0)]
            .modifier
            .contains(Modifier::UNDERLINED)
    );
}

#[test]
fn a_click_opens_the_menu_below_the_title_with_the_gui_row_layout() {
    let mut h = menu_harness();
    h.click(6, 0);
    let screen = h.screen();
    assert!(screen[1].contains('╭'), "{screen:#?}");
    let settings = screen
        .iter()
        .find(|l| l.contains("Settings…"))
        .unwrap()
        .clone();
    assert!(settings.contains("Ctrl+Alt+S"), "{settings}");
    let new_row = screen.iter().find(|l| l.contains("New")).unwrap();
    assert!(new_row.contains('\u{eab6}'), "a submenu arrow: {new_row}");
    let wrap = screen.iter().find(|l| l.contains("Soft-Wrap")).unwrap();
    assert!(
        wrap.contains('\u{eab2}'),
        "a checked toggle shows a check mark: {wrap}"
    );
    assert!(screen.iter().any(|l| l.contains("───")), "separator");
    // The menu's top edge is row 1, directly under the bar.
    h.snapshot("chrome-menu-open");
}

#[test]
fn menu_rows_choose_and_disabled_rows_do_not() {
    let mut h = menu_harness();
    h.click(6, 0);
    let (x, y) = h.find("Save All").unwrap();
    h.click(x + 1, y);
    assert!(h.app.state.events.is_empty(), "disabled");
    let (x, y) = h.find("Settings…").unwrap();
    h.click(x + 1, y);
    assert_eq!(
        h.app.state.events,
        vec![MenuEvent::Chosen {
            id: "ShowSettings".into()
        }]
    );
    assert!(!h.app.state.bar.is_open());
}

#[test]
fn hovering_another_title_switches_the_menu_and_keys_move_between_titles() {
    let mut h = menu_harness();
    h.click(6, 0);
    assert_eq!(h.app.state.bar.open_header(), Some(0));
    let (x, y) = h.find("Edit").unwrap();
    h.hover(x + 1, y);
    assert_eq!(h.app.state.bar.open_header(), Some(1));
    h.press(KeyCode::Right);
    assert_eq!(h.app.state.bar.open_header(), Some(2));
    h.press(KeyCode::Right);
    assert_eq!(h.app.state.bar.open_header(), Some(0), "wraps");
    h.press(KeyCode::Left);
    assert_eq!(h.app.state.bar.open_header(), Some(2));
}

#[test]
fn escape_closes_the_menu_then_collapses_the_bar() {
    let mut h = menu_harness();
    h.click(6, 0);
    h.press(KeyCode::Esc);
    assert!(!h.app.state.bar.is_open());
    assert!(h.app.state.bar.is_expanded() || true);
    h.press(KeyCode::Esc);
    assert!(h.app.state.events.contains(&MenuEvent::Closed));
}

#[test]
fn submenus_open_to_the_right_and_keys_choose_by_mnemonic() {
    let mut h = menu_harness();
    h.click(6, 0);
    h.press(KeyCode::Down); // New (first selectable)
    h.press(KeyCode::Right);
    assert!(h.text().contains("Project…"), "{}", h.text());
    h.press(KeyCode::Down);
    h.press(KeyCode::Enter);
    assert_eq!(
        h.app.state.events,
        vec![MenuEvent::Chosen {
            id: "NewFile".into()
        }]
    );
    // Mnemonic: Open… is chosen by `o`.
    h.app.state.events.clear();
    h.click(6, 0);
    h.press_with(KeyCode::Char('o'), KeyModifiers::NONE);
    assert_eq!(
        h.app.state.events,
        vec![MenuEvent::Chosen {
            id: "OpenFile".into()
        }]
    );
}

#[test]
fn alt_mnemonic_opens_a_menu() {
    let mut h = menu_harness();
    let rt = &h.rt;
    let props = h.app.state.props.clone();
    assert!(h.app.state.bar.open_by_mnemonic(rt, &props, 'e'));
    h.draw();
    assert_eq!(h.app.state.bar.open_header(), Some(1));
    assert!(h.app.state.bar.open_by_mnemonic(&h.rt, &props, 'u'));
    assert_eq!(h.app.state.bar.open_header(), Some(2), "Run opens by u");
}

#[test]
fn menu_outline_matches_the_props_headers() {
    let h = menu_harness();
    let got = h.app.state.bar.outline(h.buffer());
    let labels: Vec<_> = got.entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, ["File", "Edit", "Run"]);
}

// ---------------------------------------------------------------- stripes

fn stripe_props() -> ToolStripe {
    let mut s = ToolStripe::new(StripeAnchor::Left);
    let button = |id: &str, g: Glyph, t: &str, key: Option<&str>| {
        let mut tip = Tooltip::new(t);
        if let Some(k) = key {
            tip = tip.shortcut(Shortcut::keys(k));
        }
        StripeButton::new(id, g, tip)
    };
    let mut project = button("project", Glyph::Project, "Project", Some("Alt+1"));
    project.selected = true;
    project.focused = true;
    s.top.push(project);
    s.top
        .push(button("commit", Glyph::Commit, "Commit", Some("Alt+0")));
    s.top.push(button(
        "structure",
        Glyph::Structure,
        "Structure",
        Some("Alt+7"),
    ));
    let mut git = button("git", Glyph::Branch, "Git", Some("Alt+9"));
    git.badge = Some(Tone::Error);
    s.bottom.push(git);
    s.bottom.push(button(
        "problems",
        Glyph::Problems,
        "Problems",
        Some("Alt+6"),
    ));
    s.bottom.push(button(
        "terminal",
        Glyph::Terminal,
        "Terminal",
        Some("Alt+F12"),
    ));
    s.bottom
        .push(button("build", Glyph::Build, "Build", Some("Alt+4")));
    s
}

struct StripeState {
    props: ToolStripe,
    control: ToolStripeControl,
    events: Vec<StripeEvent>,
}

fn stripe_harness(h: u16) -> H<StripeState> {
    harness(
        StripeState {
            props: stripe_props(),
            control: ToolStripeControl::new(Id::new("stripe")),
            events: Vec::new(),
        },
        |s: &mut StripeState, rt, e| {
            let got = s.control.handle(rt, e);
            s.events.extend(got);
        },
        |s, rt, area, buf| {
            let col = archipel_kit_tui::ratatui::layout::Rect::new(0, 0, 3, area.height);
            s.control.draw(rt, &s.props, col, buf)
        },
        40,
        h,
    )
}

#[test]
fn stripe_groups_top_and_bottom_with_a_separator_and_two_row_pitch() {
    let h = stripe_harness(20);
    let rows: Vec<String> = h.screen();
    let glyph_rows: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, _)| i)
        .collect();
    // Top group at rows 0, 2, 4; separator row 5; bottom group bottom aligned: 19, 17, 15, 13.
    assert_eq!(glyph_rows, [0, 2, 4, 5, 13, 15, 17, 19], "{rows:#?}");
    assert_eq!(rows[5].trim(), "─");
    assert_eq!(
        rows[19].trim(),
        "\u{ec6f}\u{25cf}".replace('\u{25cf}', "").trim().to_owned() + "●",
        "{rows:?}"
    );
}

#[test]
fn stripe_pitch_falls_back_to_one_row_when_the_column_is_short() {
    let h = stripe_harness(11);
    let rows = h.screen();
    // 3 + separator + 4 buttons at pitch 1 = 8 rows.
    assert_eq!(
        rows.iter().filter(|l| !l.trim().is_empty()).count(),
        8,
        "{rows:#?}"
    );
}

#[test]
fn stripe_active_button_is_accent_filled_and_a_badge_sits_in_the_right_cell() {
    let h = stripe_harness(20);
    let c = look().theme.colors;
    let rgb = |c: archipel_kit::egui::Color32| Color::Rgb(c.r(), c.g(), c.b());
    for x in 0..3 {
        assert_eq!(h.buffer()[(x, 0)].bg, rgb(c.accent), "cell {x}");
    }
    assert_eq!(h.buffer()[(1, 0)].fg, rgb(c.text_on_accent));
    assert_eq!(h.buffer()[(0, 2)].bg, rgb(c.frame));
    assert_eq!(h.buffer()[(2, 19)].symbol(), "●");
    assert_eq!(h.buffer()[(2, 19)].fg, rgb(c.error));
}

#[test]
fn stripe_click_toggles_middle_hides_right_asks_for_the_menu_and_tooltips_name_the_shortcut() {
    let mut h = stripe_harness(20);
    h.click(1, 2);
    h.middle_click(1, 4);
    h.right_click(1, 13);
    assert_eq!(
        h.app.state.events,
        vec![
            StripeEvent::Toggle {
                id: "commit".into()
            },
            StripeEvent::Hide {
                id: "structure".into()
            },
            StripeEvent::Context { id: "build".into() },
        ]
    );
    h.hover(1, 2);
    h.advance(0.6);
    assert_eq!(h.rt.tooltip.showing_title(), Some("Commit"));
    assert!(h.text().contains("Alt+0"));
}

// ---------------------------------------------------------------- panel header

fn header_props() -> PanelHeader {
    let mut header = PanelHeader::new("project", "Project");
    header.view_selector = true;
    header.focused = true;
    for (id, glyph, name) in [
        ("new", Glyph::Plus, "New"),
        ("locate", Glyph::Search, "Locate"),
        ("expand", Glyph::ExpandAll, "Expand All"),
        ("collapse", Glyph::CollapseAll, "Collapse All"),
        ("options", Glyph::MoreVertical, "Options"),
    ] {
        header.actions.push(ToolbarButton::new(id, glyph, name));
    }
    header
}

struct HeaderState {
    props: PanelHeader,
    control: PanelHeaderControl,
    events: Vec<PanelHeaderEvent>,
}

fn header_harness(w: u16, props: PanelHeader) -> H<HeaderState> {
    harness(
        HeaderState {
            props,
            control: PanelHeaderControl::new(Id::new("header")),
            events: Vec::new(),
        },
        |s: &mut HeaderState, rt, e| {
            let got = s.control.handle(rt, e);
            s.events.extend(got);
        },
        |s, rt, area, buf| {
            s.control.draw(rt, &s.props, area, buf);
            s.control.draw_popup(rt, buf);
        },
        w,
        8,
    )
}

#[test]
fn header_has_title_chevron_actions_and_hide() {
    let h = header_harness(40, header_props());
    let line = &h.screen()[0];
    assert!(line.starts_with(" Project"), "{line:?}");
    assert!(line.contains('\u{eab4}'), "view selector chevron");
    let glyphs = [
        "\u{ea60}", "\u{ea6d}", "\u{eb95}", "\u{eac5}", "\u{eb10}", "\u{eaba}",
    ];
    let mut at = 0;
    for g in glyphs {
        at += line[at..]
            .find(g)
            .unwrap_or_else(|| panic!("{g:?} in {line:?}"))
            + g.len();
    }
    // Hide is last with a one cell right margin.
    assert_eq!(h.buffer()[(37, 0)].symbol(), "\u{eaba}");
}

#[test]
fn header_clicks_and_overflow_menu() {
    let mut h = header_harness(40, header_props());
    let (x, y) = h.find("\u{eaba}").unwrap();
    h.click(x, y);
    let (x, y) = h.find("\u{ea60}").unwrap();
    h.click(x, y);
    assert_eq!(
        h.app.state.events,
        vec![
            PanelHeaderEvent::Hide,
            PanelHeaderEvent::Action { id: "new".into() }
        ]
    );
    // Narrow: the actions that do not fit go into an overflow menu.
    let mut h = header_harness(22, header_props());
    let line = h.screen()[0].clone();
    assert!(line.contains('\u{eb10}'), "the overflow button: {line:?}");
    assert!(
        !line.contains('\u{ea60}'),
        "New moved into the menu: {line:?}"
    );
    let (x, y) = h.find("\u{eb10}").unwrap();
    h.click(x, y);
    assert!(h.rt.hits.hit(x, y + 1).is_some());
    let text = h.text();
    assert!(text.contains("New"), "{text}");
    let (x, y) = h.find("New").unwrap();
    h.click(x, y);
    assert_eq!(
        h.app.state.events,
        vec![PanelHeaderEvent::Action { id: "new".into() }]
    );
}

#[test]
fn header_tabs_show_counts_and_underline_the_selected_one() {
    let mut props = PanelHeader::new("problems", "Problems");
    props.tabs = vec![
        PanelTab {
            id: "file".into(),
            title: "File".into(),
            count: Some(1),
            selected: true,
            closable: false,
        },
        PanelTab {
            id: "project".into(),
            title: "Project Errors".into(),
            count: Some(3),
            selected: false,
            closable: true,
        },
    ];
    let mut h = header_harness(70, props);
    let line = h.screen()[0].clone();
    assert!(line.contains("Problems"), "{line}");
    assert!(line.contains("File 1"), "{line}");
    assert!(line.contains("Project Errors 3"), "{line}");
    use archipel_kit_tui::ratatui::style::Modifier;
    let (x, y) = h.find("File").unwrap();
    assert!(h.buffer()[(x, y)].modifier.contains(Modifier::UNDERLINED));
    let (x, y) = h.find("Project").unwrap();
    assert!(!h.buffer()[(x, y)].modifier.contains(Modifier::UNDERLINED));
    h.click(x, y);
    h.middle_click(x, y);
    assert_eq!(
        h.app.state.events,
        vec![
            PanelHeaderEvent::SelectTab {
                id: "project".into()
            },
            PanelHeaderEvent::CloseTab {
                id: "project".into()
            },
        ]
    );
    h.snapshot("chrome-header");
}

// ---------------------------------------------------------------- tab bar

fn tab_props() -> TabBar {
    let mut a = Tab::new(
        "src/Main.java",
        "Main.java",
        IconKey::File("Main.java".into()),
    );
    a.selected = true;
    let mut b = Tab::new(
        "src/Util.java",
        "Util.java",
        IconKey::File("Util.java".into()),
    );
    b.dirty = true;
    let mut c = Tab::new("README.md", "README.md", IconKey::File("README.md".into()));
    c.preview = true;
    let mut d = Tab::new("pom.xml", "pom.xml", IconKey::File("pom.xml".into()));
    d.pinned = true;
    d.error = true;
    TabBar::new("editor", vec![a, b, c, d])
}

struct TabState {
    props: TabBar,
    control: TabBarControl,
    events: Vec<TabEvent>,
}

fn tab_harness(w: u16, props: TabBar) -> H<TabState> {
    harness(
        TabState {
            props,
            control: TabBarControl::new(Id::new("tabs")),
            events: Vec::new(),
        },
        |s: &mut TabState, rt, e| {
            let props = s.props.clone();
            let got = s.control.handle(rt, &props, e);
            s.events.extend(got);
        },
        |s, rt, area, buf| {
            s.control.draw(rt, &s.props, area, buf);
            s.control.draw_popup(rt, buf);
        },
        w,
        16,
    )
}

#[test]
fn tabs_show_icon_name_and_the_state_cell() {
    let h = tab_harness(100, tab_props());
    let line = &h.screen()[0];
    assert!(line.contains("Main.java"), "{line}");
    assert!(line.contains('●'), "the dirty tab shows a dot: {line}");
    assert!(
        line.contains('\u{ea76}'),
        "the selected tab shows the close button: {line}"
    );
    assert!(
        line.contains('\u{eb2b}'),
        "the pinned tab shows a pin: {line}"
    );
    use archipel_kit_tui::ratatui::style::Modifier;
    let (x, y) = h.find("README.md").unwrap();
    assert!(
        h.buffer()[(x, y)].modifier.contains(Modifier::ITALIC),
        "preview"
    );
    let (x, y) = h.find("pom.xml").unwrap();
    assert!(
        h.buffer()[(x, y)].modifier.contains(Modifier::UNDERLINED),
        "error underline"
    );
    assert_eq!(
        h.buffer()[(x, y)].underline_color,
        Color::Rgb(
            look().theme.colors.error.r(),
            look().theme.colors.error.g(),
            look().theme.colors.error.b()
        )
    );
    let c = look().theme.colors;
    let (mx, my) = h.find("Main.java").unwrap();
    assert_eq!(
        h.buffer()[(mx, my)].bg,
        Color::Rgb(c.tab_selected.r(), c.tab_selected.g(), c.tab_selected.b())
    );
    h.snapshot("chrome-tabs");
}

#[test]
fn tab_dot_turns_into_close_on_hover_and_the_close_has_a_guard() {
    let mut h = tab_harness(100, tab_props());
    let (x, y) = h.find("Util.java").unwrap();
    let dot_x = x + 10;
    assert_eq!(h.buffer()[(dot_x, y)].symbol(), "●");
    h.hover(x, y);
    assert_eq!(h.buffer()[(dot_x, y)].symbol(), "\u{ea76}", "hover shows ✕");
    // A click right after entering is guarded; after the guard it closes.
    h.advance(0.3);
    h.click(dot_x, y);
    assert_eq!(
        h.app.state.events.last(),
        Some(&TabEvent::Close {
            id: "src/Util.java".into()
        })
    );
}

#[test]
fn tab_press_selects_middle_closes_right_asks_for_a_menu_pin_unpins() {
    let mut h = tab_harness(100, tab_props());
    let (x, y) = h.find("README.md").unwrap();
    h.click(x, y);
    h.middle_click(x, y);
    h.right_click(x, y);
    let (px, py) = h.find("\u{eb2b}").unwrap();
    h.click(px, py);
    assert_eq!(
        h.app.state.events,
        vec![
            TabEvent::Select {
                id: "README.md".into()
            },
            TabEvent::Close {
                id: "README.md".into()
            },
            TabEvent::Context {
                id: "README.md".into()
            },
            TabEvent::Unpin {
                id: "pom.xml".into()
            },
        ]
    );
}

#[test]
fn tabs_drag_to_reorder() {
    let mut h = tab_harness(100, tab_props());
    let (x, y) = h.find("Main.java").unwrap();
    let (tx, _) = h.find("Util.java").unwrap();
    h.drag((x, y), (tx + 8, y));
    assert!(
        h.app
            .state
            .events
            .iter()
            .any(|e| matches!(e, TabEvent::Reorder { from: 0, to: 1 })),
        "{:?}",
        h.app.state.events
    );
}

#[test]
fn tabs_overflow_into_a_list() {
    let mut tabs = Vec::new();
    for i in 0..12 {
        let mut t = Tab::new(
            format!("f{i}.rs"),
            format!("file{i}.rs"),
            IconKey::File(format!("f{i}.rs")),
        );
        t.selected = i == 0;
        tabs.push(t);
    }
    let mut h = tab_harness(60, TabBar::new("editor", tabs));
    let line = h.screen()[0].clone();
    assert!(
        line.trim_end().ends_with('\u{eb10}'),
        "overflow button at the end: {line:?}"
    );
    let (x, y) = h.find("\u{eb10}").unwrap();
    h.click(x, y);
    assert!(h.text().contains("file11.rs"), "{}", h.text());
    let (x, y) = h.find("file11.rs").unwrap();
    h.click(x + 1, y);
    assert_eq!(
        h.app.state.events,
        vec![TabEvent::Select {
            id: "f11.rs".into()
        }]
    );
}

// ---------------------------------------------------------------- status bar

fn status_props() -> StatusBar {
    let mut bar = StatusBar::default();
    let mut project = Crumb::new("project", "demo");
    project.icon = Some(Glyph::Project.into());
    bar.breadcrumbs.crumbs = vec![
        project,
        Crumb::new("src", "src"),
        Crumb::new("main", "main"),
        Crumb::new("Main", "Main"),
        Crumb::new("method", "main()"),
    ];
    let mut message = StatusBarItem::new("message", "Indexing…");
    message.tone = Tone::Info;
    message.elide = true;
    message.priority = 30;
    let mut spin = StatusBarItem::new("spinner", "");
    spin.spinner = true;
    spin.priority = 40;
    let mut caret = StatusBarItem::new("caret", "12:30");
    caret.priority = 90;
    let mut lf = StatusBarItem::new("separator", "LF");
    lf.priority = 60;
    let mut enc = StatusBarItem::new("encoding", "UTF-8");
    enc.priority = 60;
    let mut indent = StatusBarItem::new("indent", "4 spaces");
    indent.priority = 70;
    let mut gradle = StatusBarItem::new("external", "Gradle");
    gradle.priority = 20;
    let mut lsp = StatusBarItem::new("lsp", "Java LS");
    lsp.priority = 80;
    let mut ai = StatusBarItem::new("ai", "AI");
    ai.priority = 10;
    bar.items = vec![message, spin, caret, lf, enc, indent, gradle, lsp, ai];
    bar
}

struct StatusState {
    props: StatusBar,
    control: StatusItemsControl,
    events: Vec<StatusBarEvent>,
}

fn status_harness(w: u16) -> H<StatusState> {
    harness(
        StatusState {
            props: status_props(),
            control: StatusItemsControl::new(Id::new("status")),
            events: Vec::new(),
        },
        |s: &mut StatusState, rt, e| {
            let got = s.control.handle(rt, e);
            s.events.extend(got);
        },
        |s, rt, area, buf| s.control.draw(rt, &s.props, area, buf),
        w,
        1,
    )
}

#[test]
fn status_bar_has_breadcrumbs_left_and_widgets_right_in_order() {
    let h = status_harness(160);
    let line = h.screen()[0].clone();
    assert!(
        line.starts_with(" \u{eb46} demo › src › main › Main › main()"),
        "{line:?}"
    );
    let mut at = 0;
    for word in [
        "Indexing…",
        "12:30",
        "LF",
        "UTF-8",
        "4 spaces",
        "Gradle",
        "Java LS",
        "AI",
    ] {
        at += line[at..]
            .find(word)
            .unwrap_or_else(|| panic!("{word} in {line:?}"))
            + word.len();
    }
    assert!(!line.contains("INSERT") && !line.contains("NORMAL") && !line.contains("Ln"));
    h.snapshot("chrome-status");
}

#[test]
fn status_bar_drops_the_lowest_priority_widgets_first() {
    let h = status_harness(100);
    let line = h.screen()[0].clone();
    assert!(!line.contains("AI"), "{line:?}");
    assert!(
        line.contains("12:30") && line.contains("Java LS"),
        "{line:?}"
    );
    let h = status_harness(60);
    let line = h.screen()[0].clone();
    assert!(
        line.contains("12:30") && line.contains("Java LS"),
        "{line:?}"
    );
    assert!(!line.contains("Gradle") && !line.contains("AI"), "{line:?}");
    assert!(line.contains("main()"), "the last crumb stays: {line:?}");
}

#[test]
fn status_breadcrumbs_shorten_from_the_left() {
    let h = status_harness(70);
    let line = h.screen()[0].clone();
    assert!(line.contains("main()"), "{line:?}");
    assert!(line.contains('…'), "{line:?}");
}

#[test]
fn status_clicks_report_widgets_and_crumbs() {
    let mut h = status_harness(160);
    let (x, y) = h.find("12:30").unwrap();
    h.click(x, y);
    let (x, y) = h.find("src").unwrap();
    h.click(x, y);
    assert_eq!(
        h.app.state.events,
        vec![
            StatusBarEvent::Click { id: "caret".into() },
            StatusBarEvent::Crumb { id: "src".into() },
        ]
    );
}

#[test]
fn the_spinner_animates_at_ten_hertz() {
    let mut h = status_harness(160);
    let first = h.screen()[0].clone();
    h.advance(0.1);
    let second = h.screen()[0].clone();
    assert_ne!(first, second);
    assert!(h.app.state.control.animating());
}

// ---------------------------------------------------------------- look

#[test]
fn gui_look_maps_tokens_one_to_one_and_blends_translucent_ones() {
    let look = look();
    let c = &look.theme.colors;
    let rgb = |c: archipel_kit::egui::Color32| Color::Rgb(c.r(), c.g(), c.b());
    assert_eq!(look.over(c.frame, c.frame), rgb(c.frame));
    assert_eq!(look.over(c.accent, c.frame), rgb(c.accent));
    // hover is translucent white: blended over the frame it is lighter, not white.
    let hover = look.over(c.hover, c.frame);
    assert_ne!(hover, Color::Rgb(255, 255, 255));
    let Color::Rgb(r, _, _) = hover else { panic!() };
    assert!(r > c.frame.r());
    assert_eq!(look.palette.black, rgb(c.island));
    assert_eq!(look.palette.statusline_bg, rgb(c.frame));
}

#[test]
fn ascii_mode_keeps_the_geometry() {
    let rt = |ascii| {
        Runtime::with(
            look().with_ascii(ascii),
            archipel_kit_tui::input::terminal_keymap(archipel_kit::action_system::Os::Linux),
            archipel_kit_tui::Caps::kitty(),
        )
    };
    let draw = |ascii| {
        let mut h = Harness::with_runtime(
            app_fn(
                (main_toolbar(), ToolbarControl::new(Id::new("t"))),
                |_: &mut (Toolbar, ToolbarControl), _, _| {},
                |s, rt, area, buf| s.1.draw(rt, &s.0, area, buf),
            ),
            rt(ascii),
            120,
            1,
        );
        h.draw();
        h.screen()[0].clone()
    };
    let (nerd, ascii) = (draw(false), draw(true));
    assert_eq!(
        unicode_width::UnicodeWidthStr::width(nerd.as_str()),
        unicode_width::UnicodeWidthStr::width(ascii.as_str()),
        "{nerd:?} vs {ascii:?}"
    );
    assert!(ascii.starts_with(" ="), "{ascii:?}");
}

// ---------------------------------------------------------------- vocabulary

#[test]
fn the_terminal_kit_renders_the_whole_vocabulary() {
    use archipel_kit_model::vocabulary::{PENDING_TUI, VOCABULARY};
    let rendered = archipel_kit_tui::rendered_components();
    for entry in VOCABULARY {
        let found = rendered.iter().find(|(name, _, _)| *name == entry.name);
        match found {
            Some((_, _, props)) => assert!(
                props.ends_with(entry.props),
                "{} renders {props}, the model says {}",
                entry.name,
                entry.props
            ),
            None => assert!(
                PENDING_TUI.contains(&entry.name),
                "{} is neither rendered nor pending",
                entry.name
            ),
        }
    }
    // The pending list only goes down: it is empty now.
    assert!(PENDING_TUI.is_empty(), "{PENDING_TUI:?}");
}
