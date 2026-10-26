//! A terminal IDE built from the kit: project tree, editor tabs, run window, toolbar, status
//! line, Search Everywhere, context menus, dialogs, notifications and a theme switcher. It
//! looks like NvChad and behaves like an IntelliJ app.
//!
//! `cargo run -p archipel-kit-tui --example ide`
//!
//! Use a terminal with 24-bit color and a Nerd Font (JetBrainsMono Nerd Font). Keys (the GUI
//! kit's keymap; fallbacks for terminals that cannot send a combo are shown in tooltips):
//!
//! | Key | Action |
//! |---|---|
//! | Alt+1 / Alt+4 / Alt+3, or Ctrl+W then 1 / 4 / 3 | Project, Run, Notifications |
//! | Esc, Shift+Esc, F12, Ctrl+Shift+F12 | back to the editor, hide, jump, hide all |
//! | Shift Shift, Ctrl+Shift+A, Ctrl+P | Search Everywhere / Go to Action |
//! | Shift+F10, Ctrl+F2 | Run, Stop |
//! | Alt+Left / Alt+Right | previous / next tab |
//! | Ctrl+F4, Ctrl+K then W | close the tab |
//! | Ctrl+T | next theme |
//! | Ctrl+K then Q | quit |
//! | Ctrl+N | New File dialog |
//!
//! The mouse works everywhere: click, double click, drag splitters and tabs, wheel, right
//! click for context menus, middle click on a tab to close it.

#![allow(missing_docs)]

use archipel_kit::action_system::{
    ActionPlace, AnAction, AnActionEvent, DataContext, DefaultActionGroup, Presentation, Shortcut,
    Toggle, ToggleAction,
};
use archipel_kit::notification::{Notification, NotificationGroup};
use archipel_kit::openapi::ui::{ExitCode, ValidationInfo};
use archipel_kit::ui::Status;
use archipel_kit::ui::popup::PopupStep;
use archipel_kit::wm::{Content, ToolWindow, ToolWindowAnchor};
use archipel_kit_tui::balloon::Balloons;
use archipel_kit_tui::collections::{JBList, JBTree, RowData, TreeEvent, TreeNode};
use archipel_kit_tui::combo::{ComboBoxControl, ComboBoxState, ComboEvent};
use archipel_kit_tui::components::{
    Diagnostics, GitStatus, Pane, PaneKind, ProgressBar, RowIcon, StatusBar, StatusSlot, TabInfo,
};
use archipel_kit_tui::controls::{FieldEvent, TextFieldControl, TextFieldState};
use archipel_kit_tui::dashboard::{Dashboard, DashboardButton};
use archipel_kit_tui::dialog::{Dialog, DialogContent, Messages};
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::Event;
use archipel_kit_tui::overlay::{Handled, Layers, Overlay, PopupOverlay, draw_which_key};
use archipel_kit_tui::popup::{
    ActionPopupMenu, ListPopup, Placement, PopupHost, PopupState, PopupStyle,
};
use archipel_kit_tui::statusbar::{StatusBarControl, StatusEvent};
use archipel_kit_tui::tabs::{TabsControl, TabsEvent, TabsState};
use archipel_kit_tui::textarea::{TextArea, TextAreaState, TextLanguage};
use archipel_kit_tui::toolbar::{ActionToolbar, ToolbarEvent, ToolbarState};
use archipel_kit_tui::tooltip::Tooltip;
use archipel_kit_tui::workbench::{Workbench, WorkbenchEvent, WorkbenchFocus};
use archipel_kit_tui::{Glyph, Language, Look, Mode, Runtime, Theme, runtime};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const TREE: Id = Id::new("ide.tree");
const EDITOR: Id = Id::new("ide.editor");
const TABS: Id = Id::new("ide.tabs");
const TOOLBAR: Id = Id::new("ide.toolbar");
const COMBO: Id = Id::new("ide.configs");
const RUN_LIST: Id = Id::new("ide.run");
const STATUS: Id = Id::new("ide.status");
const DASH: Id = Id::new("ide.dash");
const NOTES: Id = Id::new("ide.notes");
const TAB_BUTTONS: Id = Id::new("ide.tabbuttons");

/// What actions ask the app to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Run,
    Stop,
    NextTheme,
    SearchEverywhere,
    CloseTab,
    NewFile,
    About,
    Quit,
    OpenFile(String),
    ToggleWindow(&'static str),
    DeleteFile(String),
    Notify(Status, &'static str, &'static str),
}

type Queue = Rc<RefCell<Vec<Command>>>;

/// An action that queues a command.
struct Cmd {
    id: &'static str,
    text: &'static str,
    icon: Glyph,
    description: &'static str,
    command: Command,
    queue: Queue,
    enabled: Option<Rc<Cell<bool>>>,
}

impl AnAction for Cmd {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new(self.text)
            .icon(self.icon)
            .description(self.description)
    }

    fn update(&self, e: &mut AnActionEvent<'_>) {
        if let Some(enabled) = &self.enabled {
            e.presentation.enabled = enabled.get();
        }
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        self.queue.borrow_mut().push(self.command.clone());
    }
}

struct SyntaxToggle(Rc<Cell<bool>>);

impl ToggleAction for SyntaxToggle {
    fn id(&self) -> &str {
        "ToggleSyntax"
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Syntax Highlighting")
            .icon(Glyph::Code)
            .description("Tint keywords, strings and comments")
    }

    fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool {
        self.0.get()
    }

    fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) {
        self.0.set(selected);
    }
}

/// What layers tell the app.
pub enum Msg {
    Command(Command),
    NewFile(String),
    Confirmed(String, ExitCode),
    Config(usize),
}

struct Files(HashMap<String, String>);

fn files() -> Files {
    let mut m = HashMap::new();
    m.insert(
        "Cargo.toml".to_owned(),
        include_str!("../Cargo.toml").to_owned(),
    );
    m.insert(
        "README.md".to_owned(),
        include_str!("../../../docs/tui/README.md").to_owned(),
    );
    for (name, text) in [
        ("lib.rs", include_str!("../src/lib.rs")),
        ("palette.rs", include_str!("../src/palette.rs")),
        ("hit.rs", include_str!("../src/hit.rs")),
        ("focus.rs", include_str!("../src/focus.rs")),
    ] {
        m.insert(
            format!("crates/archipel-kit-tui/src/{name}"),
            text.to_owned(),
        );
    }
    Files(m)
}

fn language_of(path: &str) -> RowIcon {
    match path.rsplit('.').next() {
        Some("rs") => Language::Rust.into(),
        Some("toml") => Language::Toml.into(),
        Some("md") => Language::Markdown.into(),
        _ => Glyph::File.into(),
    }
}

fn tree_nodes(files: &Files) -> Vec<TreeNode> {
    let mut src: Vec<&String> = files.0.keys().filter(|k| k.contains("/src/")).collect();
    src.sort();
    let leaf = |path: &str| {
        let name = path.rsplit('/').next().unwrap_or(path).to_owned();
        let mut node = TreeNode::new(name.clone()).key(name);
        node.icon = Some(language_of(path));
        node
    };
    vec![
        TreeNode::new("kit")
            .key("kit")
            .icon(RowIcon::Folder(true))
            .secondary("~/archipel")
            .children(vec![
                TreeNode::new("crates")
                    .key("crates")
                    .icon(RowIcon::Folder(false))
                    .children(vec![
                        TreeNode::new("archipel-kit-tui")
                            .key("tui")
                            .icon(RowIcon::Folder(false))
                            .children(vec![
                                leaf("Cargo.toml"),
                                TreeNode::new("src")
                                    .key("src")
                                    .icon(RowIcon::Folder(false))
                                    .children(src.iter().map(|p| leaf(p)).collect()),
                            ]),
                    ]),
                leaf("README.md"),
            ]),
    ]
}

/// A Search Everywhere entry.
#[derive(Clone)]
pub struct SearchItem {
    text: String,
    secondary: String,
    icon: RowIcon,
    command: Command,
    shortcut: Option<String>,
}

struct SearchStep {
    items: Vec<SearchItem>,
}

impl PopupStep<SearchItem> for SearchStep {
    fn title(&self) -> Option<&str> {
        Some("Search Everywhere")
    }

    fn values(&self) -> &[SearchItem] {
        &self.items
    }

    fn text_for(&self, value: &SearchItem) -> String {
        value.text.clone()
    }

    fn icon_for(&self, value: &SearchItem) -> Option<archipel_kit::icons::Icon> {
        Some(match value.icon {
            RowIcon::Glyph(g) => g.into(),
            RowIcon::Language(l) => archipel_kit::icons::Icon::Language(l),
            RowIcon::Symbol(s) => archipel_kit::icons::Icon::Symbol(s),
            RowIcon::Folder(_) => Glyph::Folder.into(),
        })
    }

    fn shortcut_for(&self, value: &SearchItem) -> Option<String> {
        value
            .shortcut
            .clone()
            .or_else(|| Some(value.secondary.clone()))
    }

    fn is_speed_search_enabled(&self) -> bool {
        true
    }
}

struct NameContent {
    field: TextFieldState,
    existing: Vec<String>,
    shared: Rc<RefCell<String>>,
}

const NAME_FIELD: Id = Id::new("ide.newfile.name");

impl DialogContent for NameContent {
    fn height(&self, _width: u16) -> u16 {
        1
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        TextFieldControl::new(NAME_FIELD)
            .label("Name")
            .placeholder("main.rs")
            .on(rt.look.palette.darker_black)
            .show(rt, &self.field, area, buf);
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> bool {
        let used = matches!(
            TextFieldControl::new(NAME_FIELD).handle(rt, &mut self.field, event),
            FieldEvent::Changed | FieldEvent::Consumed | FieldEvent::Cleared
        );
        *self.shared.borrow_mut() = self.field.text().trim().to_owned();
        used
    }

    fn revision(&self) -> u64 {
        self.field.revision
    }

    fn validate(&self) -> Vec<ValidationInfo> {
        let name = self.field.text().trim();
        if name.is_empty() {
            vec![ValidationInfo::error("Enter a file name")]
        } else if self.existing.iter().any(|e| e.ends_with(name)) {
            vec![ValidationInfo::error("A file with this name exists")]
        } else if name.contains(' ') {
            vec![ValidationInfo::warning(
                "Spaces in file names are inconvenient",
            )]
        } else {
            Vec::new()
        }
    }

    fn preferred_focus(&self) -> Option<Id> {
        Some(NAME_FIELD)
    }
}

/// An ambient layer that shares the balloon stack with the app.
struct BalloonLayer(Rc<RefCell<Balloons>>);

impl Overlay<Msg> for BalloonLayer {
    fn modal(&self) -> bool {
        false
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<Msg> {
        <Balloons as Overlay<Msg>>::event(&mut self.0.borrow_mut(), rt, event)
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        <Balloons as Overlay<Msg>>::draw(&mut self.0.borrow_mut(), rt, area, buf);
    }

    fn tick(&mut self, rt: &mut Runtime) -> Vec<Msg> {
        <Balloons as Overlay<Msg>>::tick(&mut self.0.borrow_mut(), rt)
    }

    fn deadline(&self) -> Option<f64> {
        self.0.borrow().deadline()
    }
}

/// The IDE.
pub struct IdeApp {
    queue: Queue,
    layers: Layers<Msg>,
    wb: Workbench,
    files: Files,
    tree: JBTree,
    nodes: Vec<TreeNode>,
    open: Vec<String>,
    tabs: Vec<TabInfo>,
    tab_state: TabsState,
    tab_control: TabsControl,
    areas: Vec<TextAreaState>,
    lines: HashMap<String, Vec<String>>,
    toolbar: ActionToolbar,
    toolbar_state: ToolbarState,
    status: StatusBarControl,
    configs: Vec<String>,
    config: ComboBoxState,
    balloons: Rc<RefCell<Balloons>>,
    run_log: Vec<RowData>,
    run_list: JBList,
    running: Rc<Cell<bool>>,
    run_started: Option<f64>,
    run_lines_shown: usize,
    syntax: Rc<Cell<bool>>,
    have_tabs: Rc<Cell<bool>>,
    theme_index: usize,
    git: GitStatus,
    dashboard: Dashboard,
    tabs_rect: Rect,
    buttons_rect: Rect,
    search_items: Vec<SearchItem>,
    now: f64,
}

const RUN_OUTPUT: [&str; 7] = [
    "   Compiling archipel-icons v0.3.1",
    "   Compiling archipel-kit v0.3.1",
    "   Compiling archipel-kit-tui v0.3.1",
    "warning: unused import: `Mode`",
    "    Finished `dev` profile [unoptimized] target(s) in 4.21s",
    "     Running `target/debug/examples/ide`",
    "Process finished with exit code 0",
];

impl IdeApp {
    /// A new IDE in the theme `theme_index` of [`Theme::all`].
    pub fn new(theme_index: usize) -> Self {
        let queue: Queue = Rc::default();
        let running = Rc::new(Cell::new(false));
        let have_tabs = Rc::new(Cell::new(false));
        let syntax = Rc::new(Cell::new(true));
        let cmd = |id, text, icon, description, command, enabled: Option<Rc<Cell<bool>>>| Cmd {
            id,
            text,
            icon,
            description,
            command,
            queue: queue.clone(),
            enabled,
        };
        let mut group = DefaultActionGroup::new("Main Toolbar");
        group.add(cmd(
            "Run",
            "Run",
            Glyph::Run,
            "Run the current configuration",
            Command::Run,
            None,
        ));
        group.add(cmd(
            "Stop",
            "Stop",
            Glyph::Stop,
            "Stop the running process",
            Command::Stop,
            Some(running.clone()),
        ));
        group.add_separator();
        group.add(Toggle::new(SyntaxToggle(syntax.clone())));
        group.add(cmd(
            "CloseActiveTab",
            "Close Tab",
            Glyph::Close,
            "Close the active editor tab",
            Command::CloseTab,
            Some(have_tabs.clone()),
        ));
        group.add(cmd(
            "GotoAction",
            "Search Everywhere",
            Glyph::Search,
            "Search files and actions",
            Command::SearchEverywhere,
            None,
        ));
        group.add(cmd(
            "NextTheme",
            "Switch Theme",
            Glyph::Sun,
            "Switch to the next theme",
            Command::NextTheme,
            None,
        ));
        let mut more = DefaultActionGroup::popup("Tools");
        more.add(cmd(
            "NewFile",
            "New File…",
            Glyph::NewFile,
            "Create a file",
            Command::NewFile,
            None,
        ));
        more.add(cmd(
            "About",
            "About",
            Glyph::Info,
            "About the terminal kit",
            Command::About,
            None,
        ));
        more.add_separator();
        more.add(cmd(
            "Exit",
            "Exit",
            Glyph::Close,
            "Leave the IDE",
            Command::Quit,
            None,
        ));
        group.add_group(more);

        let mut run =
            ToolWindow::new("run", "Run", Glyph::Run, ToolWindowAnchor::Bottom).activate_number(4);
        run.contents
            .add_content(Content::new("build", "cargo build").closeable(false));
        run.contents
            .add_content(Content::new("tests", "tests").closeable(false));
        let wb = Workbench::new(vec![
            ToolWindow::new("project", "Project", Glyph::Project, ToolWindowAnchor::Left)
                .activate_number(1),
            ToolWindow::new(
                "notifications",
                "Notifications",
                Glyph::Bell,
                ToolWindowAnchor::Right,
            )
            .activate_number(3),
            run,
        ]);
        let files = files();
        let nodes = tree_nodes(&files);
        let mut tree = JBTree::new();
        tree.expand_path("kit/crates/tui/src");
        tree.expand_path("kit/crates/tui");
        let balloons = Rc::new(RefCell::new(Balloons::new()));
        let mut layers = Layers::new();
        layers.push_ambient(BalloonLayer(balloons.clone()));
        let mut app = Self {
            queue,
            layers,
            wb,
            files,
            tree,
            nodes,
            open: Vec::new(),
            tabs: Vec::new(),
            tab_state: TabsState::new(0),
            tab_control: TabsControl::new(TABS),
            areas: Vec::new(),
            lines: HashMap::new(),
            toolbar: ActionToolbar::new(
                TOOLBAR,
                Rc::new(group),
                Rc::new(DataContext::empty()),
                ActionPlace::MAIN_TOOLBAR,
            ),
            toolbar_state: ToolbarState::default(),
            status: StatusBarControl::new(STATUS)
                .tooltip(
                    StatusSlot::Git,
                    Tooltip::new("Git Branch").description("Click to switch branches"),
                )
                .tooltip(StatusSlot::Cursor, Tooltip::new("Caret Position"))
                .tooltip(
                    StatusSlot::Cwd,
                    Tooltip::new("Toggle Project").shortcut("Alt+1"),
                )
                .tooltip(
                    StatusSlot::Lsp,
                    Tooltip::new("Language Server").description("rust-analyzer is running"),
                ),
            configs: vec![
                "ide (example)".into(),
                "gallery (example)".into(),
                "cargo test".into(),
            ],
            config: ComboBoxState::new(0),
            balloons,
            run_log: Vec::new(),
            run_list: JBList::single(),
            running,
            run_started: None,
            run_lines_shown: 0,
            syntax,
            have_tabs,
            theme_index,
            git: GitStatus {
                branch: "tui-interactive".into(),
                added: 3,
                changed: 2,
                removed: 1,
            },
            dashboard: Dashboard::new(
                DASH,
                vec![
                    "  ▄▀█ █▀█ █▀▀ █ █ █ █▀▀ █   ".into(),
                    "  █▀█ █▀▄ █▄▄ █▀█ █ █▀▀ █▄▄ ".into(),
                    "".into(),
                    "      the terminal kit".into(),
                ],
                vec![
                    DashboardButton::new(Glyph::Search, "Search Everywhere", "Shift Shift", 'e'),
                    DashboardButton::new(Glyph::NewFile, "New File", "Ctrl N", 'n'),
                    DashboardButton::new(Glyph::Project, "Project", "Alt 1", 'p'),
                    DashboardButton::new(Glyph::Sun, "Switch Theme", "Ctrl T", 't'),
                    DashboardButton::new(Glyph::Close, "Quit", "Ctrl K Q", 'q'),
                ],
            ),
            tabs_rect: Rect::default(),
            buttons_rect: Rect::default(),
            search_items: Vec::new(),
            now: 0.0,
        };
        app.open_file("README.md");
        app.wb.state.hide_all();
        app.wb.show("project");
        app
    }

    /// The look of theme `index` of [`Theme::all`].
    pub fn look_for(index: usize) -> Look {
        let all = Theme::all();
        Look::new((all[index % all.len()].constructor)())
    }

    /// The keymap of the IDE: the kit's with the terminal fallbacks and the IDE's own keys.
    pub fn keymap(os: archipel_kit::action_system::Os) -> archipel_kit::action_system::Keymap {
        let mut keymap = archipel_kit_tui::input::terminal_keymap(os);
        for (id, key) in [
            ("Run", "shift F10"),
            ("Stop", "ctrl F2"),
            ("NextTheme", "ctrl T"),
            ("Exit", "ctrl K > Q"),
            ("NewFile", "ctrl N"),
            ("About", "F1"),
        ] {
            if let Some(shortcut) = Shortcut::parse(key) {
                keymap.add_shortcut(id, shortcut);
            }
        }
        keymap
    }

    fn open_file(&mut self, path: &str) {
        if let Some(i) = self.open.iter().position(|p| p == path) {
            self.tab_state.selected = i;
            return;
        }
        let Some(text) = self.files.0.get(path) else {
            return;
        };
        self.lines
            .insert(path.to_owned(), text.lines().map(str::to_owned).collect());
        let name = path.rsplit('/').next().unwrap_or(path);
        let mut tab = TabInfo::new(name);
        tab.icon = match language_of(path) {
            RowIcon::Language(l) => Some(l.into()),
            _ => Some(Glyph::FileText.into()),
        };
        self.tabs.push(tab);
        self.open.push(path.to_owned());
        self.areas.push(TextAreaState::new());
        self.tab_state.selected = self.open.len() - 1;
        self.have_tabs.set(true);
    }

    fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        let path = self.open.remove(index);
        self.lines.remove(&path);
        self.tabs.remove(index);
        self.areas.remove(index);
        self.tab_state.removed(index, self.tabs.len());
        self.have_tabs.set(!self.tabs.is_empty());
    }

    fn notify(&mut self, status: Status, title: &str, body: &str) {
        let n = Notification::new(status, title)
            .group(NotificationGroup::balloon_group("ide"))
            .action("Open log");
        self.balloons.borrow_mut().notify(n, Some(body));
    }

    /// The file a tree path (ending in the file name) stands for.
    fn file_for(&self, tree_path: &str) -> Option<String> {
        let name = tree_path.rsplit('/').next()?;
        self.files
            .0
            .keys()
            .find(|k| *k == name || k.ends_with(&format!("/{name}")))
            .cloned()
    }

    fn current_path(&self) -> Option<&str> {
        self.open.get(self.tab_state.selected).map(String::as_str)
    }

    fn run(&mut self, rt: &Runtime) {
        self.running.set(true);
        self.run_started = Some(rt.now);
        self.run_lines_shown = 0;
        self.run_log.clear();
        self.wb.activate("run");
    }

    fn finish_run(&mut self, ok: bool) {
        self.running.set(false);
        self.run_started = None;
        if ok {
            self.notify(
                Status::Success,
                "Build succeeded",
                "ide (example) finished with exit code 0.",
            );
        } else {
            self.notify(Status::Warning, "Run stopped", "The process was stopped.");
        }
    }

    fn run_command(&mut self, rt: &mut Runtime, command: Command) {
        match command {
            Command::Run => self.run(rt),
            Command::Stop => {
                if self.running.get() {
                    self.finish_run(false);
                }
            }
            Command::NextTheme => {
                self.theme_index = (self.theme_index + 1) % Theme::all().len();
                rt.look = Self::look_for(self.theme_index);
                let name = Theme::all()[self.theme_index].name;
                self.notify(Status::Info, "Theme", name);
            }
            Command::SearchEverywhere => self.search(rt),
            Command::CloseTab => self.close_tab(self.tab_state.selected),
            Command::NewFile => self.new_file(rt),
            Command::About => {
                let dialog = Messages::info_dialog(
                    "About",
                    "Archipel Kit TUI: NvChad looks, IntelliJ behavior, the same actions and keymap as the GUI kit.",
                );
                self.layers.push(rt, dialog);
            }
            Command::Quit => {
                let dialog = Messages::yes_no_dialog("Exit", "Leave the IDE?")
                    .on_exit(|code| vec![Msg::Confirmed("quit".into(), code)]);
                self.layers.push(rt, dialog);
            }
            Command::OpenFile(path) => self.open_file(&path),
            Command::ToggleWindow(id) => {
                self.wb.toggle(id);
            }
            Command::DeleteFile(path) => {
                let dialog = Messages::yes_no_dialog("Delete", format!("Delete {path}?"))
                    .danger(true)
                    .on_exit(move |code| vec![Msg::Confirmed(path.clone(), code)]);
                self.layers.push(rt, dialog);
            }
            Command::Notify(status, title, body) => self.notify(status, title, body),
        }
    }

    fn new_file(&mut self, rt: &mut Runtime) {
        let shared: Rc<RefCell<String>> = Rc::default();
        let name = shared.clone();
        let dialog = Dialog::new("New File")
            .primary("Create")
            .secondary("Cancel")
            .message("Enter the name of the new file")
            .content(NameContent {
                field: TextFieldState::new(),
                existing: self.files.0.keys().cloned().collect(),
                shared,
            })
            .on_exit(move |code| {
                if code == ExitCode::Primary {
                    vec![Msg::NewFile(name.borrow().clone())]
                } else {
                    Vec::new()
                }
            });
        self.layers.push(rt, dialog);
    }

    fn search(&mut self, rt: &mut Runtime) {
        let mut items = Vec::new();
        let actions: [(&str, &str, Glyph, Command); 8] = [
            (
                "Run",
                "Run the current configuration",
                Glyph::Run,
                Command::Run,
            ),
            (
                "Stop",
                "Stop the running process",
                Glyph::Stop,
                Command::Stop,
            ),
            ("Switch Theme", "Next theme", Glyph::Sun, Command::NextTheme),
            (
                "New File…",
                "Create a file",
                Glyph::NewFile,
                Command::NewFile,
            ),
            (
                "Close Tab",
                "Close the active tab",
                Glyph::Close,
                Command::CloseTab,
            ),
            (
                "Toggle Project",
                "Show or hide the Project window",
                Glyph::Project,
                Command::ToggleWindow("project"),
            ),
            (
                "Toggle Run Window",
                "Show or hide Run",
                Glyph::Terminal,
                Command::ToggleWindow("run"),
            ),
            (
                "About",
                "About the terminal kit",
                Glyph::Info,
                Command::About,
            ),
        ];
        for (text, secondary, icon, command) in actions {
            let id = match &command {
                Command::Run => "Run",
                Command::Stop => "Stop",
                Command::NextTheme => "NextTheme",
                Command::NewFile => "NewFile",
                Command::CloseTab => "CloseActiveTab",
                Command::About => "About",
                _ => "",
            };
            items.push(SearchItem {
                text: text.into(),
                secondary: secondary.into(),
                icon: icon.into(),
                command,
                shortcut: rt.shortcut_text(id),
            });
        }
        let mut paths: Vec<&String> = self.files.0.keys().collect();
        paths.sort();
        for path in paths {
            items.push(SearchItem {
                text: path.rsplit('/').next().unwrap_or(path).to_owned(),
                secondary: path.clone(),
                icon: language_of(path),
                command: Command::OpenFile(path.clone()),
                shortcut: None,
            });
        }
        self.search_items = items.clone();
        let mut host: ListPopup<SearchItem> =
            ListPopup::list(SearchStep { items }).style(PopupStyle::Telescope);
        host.open(rt, true);
        let overlay = PopupOverlay::new(host, |source, state| match state {
            PopupState::Chosen => source
                .take_chosen()
                .map(|item| vec![Msg::Command(item.command)])
                .unwrap_or_default(),
            _ => Vec::new(),
        });
        self.layers.push(rt, overlay);
    }

    fn menu(
        &mut self,
        rt: &mut Runtime,
        group: DefaultActionGroup,
        place: ActionPlace,
        x: u16,
        y: u16,
    ) {
        let mut menu = ActionPopupMenu::actions(
            Rc::new(group),
            Rc::new(DataContext::empty()),
            place,
            rt.keymap.clone(),
            rt.caps,
            Placement::At(x, y),
        );
        menu.open(rt, false);
        let overlay = PopupOverlay::new(menu, |_, _| Vec::<Msg>::new());
        self.layers.push(rt, overlay);
    }

    fn cmd(&self, id: &'static str, text: &'static str, icon: Glyph, command: Command) -> Cmd {
        Cmd {
            id,
            text,
            icon,
            description: "",
            command,
            queue: self.queue.clone(),
            enabled: None,
        }
    }

    fn tree_menu(&mut self, rt: &mut Runtime, path: Option<String>, x: u16, y: u16) {
        let mut g = DefaultActionGroup::new("Project View");
        if let Some(path) = path.filter(|p| self.files.0.contains_key(p)) {
            g.add(self.cmd("Open", "Open", Glyph::File, Command::OpenFile(path.clone())));
            g.add(self.cmd("RunFile", "Run", Glyph::Run, Command::Run));
            g.add_separator();
            g.add(self.cmd(
                "DeleteFile",
                "Delete…",
                Glyph::Delete,
                Command::DeleteFile(path),
            ));
        }
        g.add(self.cmd("NewFile", "New File…", Glyph::NewFile, Command::NewFile));
        self.menu(rt, g, ActionPlace::PROJECT_VIEW_POPUP, x, y);
    }

    fn tab_menu(&mut self, rt: &mut Runtime, x: u16, y: u16) {
        let mut g = DefaultActionGroup::new("Editor Tab");
        g.add(self.cmd("CloseActiveTab", "Close", Glyph::Close, Command::CloseTab));
        g.add(self.cmd("NextTheme", "Switch Theme", Glyph::Sun, Command::NextTheme));
        self.menu(rt, g, ActionPlace::EDITOR_TAB_POPUP, x, y);
    }

    fn editor_menu(&mut self, rt: &mut Runtime, x: u16, y: u16) {
        let mut g = DefaultActionGroup::new("Editor");
        g.add(self.cmd(
            "GotoAction",
            "Search Everywhere",
            Glyph::Search,
            Command::SearchEverywhere,
        ));
        g.add(self.cmd("Run", "Run", Glyph::Run, Command::Run));
        g.add_separator();
        g.add(self.cmd(
            "CloseActiveTab",
            "Close Tab",
            Glyph::Close,
            Command::CloseTab,
        ));
        self.menu(rt, g, ActionPlace::EDITOR_POPUP, x, y);
    }

    fn status_menu(&mut self, rt: &mut Runtime, x: u16, y: u16) {
        let mut g = DefaultActionGroup::new("Status Bar");
        g.add(self.cmd(
            "ToggleProject",
            "Project",
            Glyph::Project,
            Command::ToggleWindow("project"),
        ));
        g.add(self.cmd("ToggleRun", "Run", Glyph::Run, Command::ToggleWindow("run")));
        g.add(self.cmd(
            "ToggleNotes",
            "Notifications",
            Glyph::Bell,
            Command::ToggleWindow("notifications"),
        ));
        self.menu(rt, g, ActionPlace::STATUS_BAR_PLACE, x, y);
    }

    fn branch_popup(&mut self, rt: &mut Runtime, anchor: Rect) {
        let branches = vec![
            "tui-interactive".to_owned(),
            "main".to_owned(),
            "feat/intellij-platform-alignment".to_owned(),
        ];
        let step = archipel_kit::ui::popup::BaseListPopupStep::new("Branches", branches)
            .icons(|_| Some(Glyph::Branch.into()));
        let host = PopupHost::menu_below(step, anchor)
            .at(Placement::At(anchor.x, anchor.y.saturating_sub(5)));
        let mut host = host;
        host.open(rt, true);
        let overlay = PopupOverlay::new(host, |source, state| match state {
            PopupState::Chosen => source
                .take_chosen()
                .map(|b| vec![Msg::Confirmed(format!("branch:{b}"), ExitCode::Primary)])
                .unwrap_or_default(),
            _ => Vec::new(),
        });
        self.layers.push(rt, overlay);
    }

    fn apply(&mut self, rt: &mut Runtime, msg: Msg) {
        match msg {
            Msg::Command(c) => self.run_command(rt, c),
            Msg::NewFile(name) => {
                let path = format!("crates/archipel-kit-tui/src/{name}");
                self.files.0.insert(path.clone(), String::new());
                self.nodes = tree_nodes(&self.files);
                self.tree.expand_path("kit/crates/tui/src");
                self.open_file(&path);
                self.notify(Status::Success, "File created", &name);
            }
            Msg::Confirmed(what, code) => {
                if what == "quit" {
                    if code == ExitCode::Primary {
                        rt.quit = true;
                    }
                } else if let Some(branch) = what.strip_prefix("branch:") {
                    self.git.branch = branch.to_owned();
                } else if code == ExitCode::Primary {
                    self.files.0.remove(&what);
                    if let Some(i) = self.open.iter().position(|p| *p == what) {
                        self.close_tab(i);
                    }
                    self.nodes = tree_nodes(&self.files);
                    self.notify(Status::Info, "File deleted", &what);
                }
            }
            Msg::Config(i) => self.config.selected = i,
        }
    }

    fn drain(&mut self, rt: &mut Runtime) {
        loop {
            let commands: Vec<Command> = std::mem::take(&mut *self.queue.borrow_mut());
            if commands.is_empty() {
                break;
            }
            for command in commands {
                self.run_command(rt, command);
            }
        }
    }

    fn mode(&self) -> Mode {
        if !self.layers.is_empty() {
            return Mode::Command;
        }
        match &self.wb.focus {
            WorkbenchFocus::Editor => Mode::Visual,
            WorkbenchFocus::Tool(id) if id == "run" => Mode::Terminal,
            WorkbenchFocus::Tool(_) => Mode::Normal,
        }
    }

    fn handle_workbench(&mut self, events: Vec<WorkbenchEvent>, rt: &mut Runtime) {
        for e in events {
            match e {
                WorkbenchEvent::StripeMenu(id, x, y) | WorkbenchEvent::HeaderMenu(id, x, y) => {
                    let mut g = DefaultActionGroup::new("Tool Window");
                    let toggle: &'static str = match id.as_str() {
                        "project" => "project",
                        "run" => "run",
                        _ => "notifications",
                    };
                    g.add(self.cmd(
                        "HideActiveWindow",
                        "Hide",
                        Glyph::Minimize,
                        Command::ToggleWindow(toggle),
                    ));
                    self.menu(rt, g, ActionPlace::TOOLWINDOW_POPUP, x, y);
                }
                _ => {}
            }
        }
    }
}

impl archipel_kit_tui::App for IdeApp {
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        self.now = rt.now;
        let (consumed, messages) = self.layers.event(rt, event);
        for m in messages {
            self.apply(rt, m);
        }
        if consumed {
            self.drain(rt);
            return;
        }
        if rt.double_shift() {
            self.search(rt);
            return;
        }
        // Mouse: workbench first (focus, stripes, splitters), then the widgets.
        if let Event::Mouse(_) = event {
            let events = self.wb.handle(rt, event);
            self.handle_workbench(events, rt);
        }
        // Keys: the focused widget first (a speed search or text field uses Esc itself).
        let esc_for_widget = matches!(event, Event::Key(k) if k.code == archipel_kit_tui::input::KeyCode::Esc)
            && self.tree.state.speed.is_active()
            && self.wb.focused_tool() == Some("project");
        if let Event::Key(_) = event
            && !esc_for_widget
        {
            {
                for id in rt.actions().to_vec() {
                    if let Some(events) = self.wb.perform(&id) {
                        self.handle_workbench(events, rt);
                        self.drain(rt);
                        return;
                    }
                    if self.perform_action(rt, &id) {
                        self.drain(rt);
                        return;
                    }
                }
            }
        }
        self.route(rt, event);
        self.drain(rt);
    }

    fn tick(&mut self, rt: &mut Runtime) {
        self.now = rt.now;
        let messages = self.layers.tick(rt);
        for m in messages {
            self.apply(rt, m);
        }
        if let Some(started) = self.run_started {
            let due = ((rt.now - started) / 0.25) as usize;
            while self.run_lines_shown < due.min(RUN_OUTPUT.len()) {
                let line = RUN_OUTPUT[self.run_lines_shown];
                let mut row = RowData::new(line);
                row.muted = line.starts_with("   Compiling");
                self.run_log.push(row);
                self.run_lines_shown += 1;
            }
            if self.run_lines_shown >= RUN_OUTPUT.len() && self.running.get() {
                self.finish_run(true);
            }
        }
        self.drain(rt);
    }

    fn deadline(&self, rt: &Runtime) -> Option<f64> {
        let mut next = self.layers.deadline();
        let mut consider = |t: f64| next = Some(next.map_or(t, |n| n.min(t)));
        if let Some(started) = self.run_started {
            consider(started + (self.run_lines_shown as f64 + 1.0) * 0.25);
        }
        if let Some(t) = rt.tooltip.deadline() {
            consider(t);
        }
        next
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        self.now = rt.now;
        let look = rt.look.clone();
        let p = look.palette;
        Pane::new(&look).kind(PaneKind::Editor).render(area, buf);
        if area.height < 4 || area.width < 20 {
            return;
        }
        // Row 0: the main toolbar.
        let top = Rect::new(area.x, area.y, area.width, 1);
        for x in 0..area.width {
            if let Some(cell) = buf.cell_mut((area.x + x, area.y)) {
                cell.set_symbol(" ").set_bg(p.black2);
            }
        }
        let name = " kit ";
        archipel_kit_tui::testing::put_text(
            buf,
            top,
            1,
            top.y,
            name,
            p.on(p.nord_blue),
            p.nord_blue,
        );
        let toolbar_area = Rect::new(8, top.y, area.width.saturating_sub(40).min(60), 1);
        self.toolbar
            .show(rt, &self.toolbar_state, toolbar_area, buf);
        let combo = ComboBoxControl::new(COMBO, &self.configs).width(22);
        let cx = (area.width / 2)
            .max(toolbar_area.right() + 1)
            .min(area.width.saturating_sub(24));
        combo.show(rt, &self.config, cx, top.y, buf);
        let hint = rt.hint.clone();
        if let Some(h) = hint {
            archipel_kit_tui::testing::put_text(
                buf,
                top,
                cx + 24,
                top.y,
                &h,
                p.light_grey,
                p.black2,
            );
        }
        // Status line and the workbench between.
        let status_row = Rect::new(area.x, area.bottom() - 1, area.width, 1);
        let work = Rect::new(area.x, area.y + 1, area.width, area.height - 2);
        let areas = self.wb.draw(rt, work, buf);
        self.sync_focus(rt);
        // Project tree.
        if let Some(body) = areas.tool("project") {
            self.tree
                .show(rt, TREE, body, &self.nodes, p.darker_black, buf);
        }
        // Run window.
        if let Some(body) = areas.tool("run") {
            let selected = self
                .wb
                .window("run")
                .and_then(|w| w.contents.selected_index())
                .unwrap_or(0);
            if selected == 0 {
                let list_area =
                    Rect::new(body.x, body.y, body.width, body.height.saturating_sub(1));
                self.run_list
                    .show(rt, RUN_LIST, list_area, &self.run_log, p.darker_black, buf);
                if self.running.get() && body.height > 1 {
                    let fraction = self.run_lines_shown as f32 / RUN_OUTPUT.len() as f32;
                    ProgressBar::new(&look, fraction).on(p.darker_black).render(
                        Rect::new(
                            body.x + 1,
                            body.bottom() - 1,
                            body.width.saturating_sub(2).min(40),
                            1,
                        ),
                        buf,
                    );
                }
            } else {
                archipel_kit_tui::testing::put_text(
                    buf,
                    body,
                    body.x + 1,
                    body.y,
                    "$ ",
                    p.green,
                    p.darker_black,
                );
            }
        }
        // Notifications history.
        if let Some(body) = areas.tool("notifications") {
            let rows: Vec<RowData> = self
                .balloons
                .borrow()
                .manager
                .history()
                .iter()
                .map(|n| RowData::new(n.title()).icon(Glyph::Bell))
                .collect();
            let mut list = JBList::single();
            list.show(rt, NOTES, body, &rows, p.darker_black, buf);
            if rows.is_empty() {
                archipel_kit_tui::testing::put_text(
                    buf,
                    body,
                    body.x + 1,
                    body.y,
                    "No notifications",
                    p.grey_fg,
                    p.darker_black,
                );
            }
        }
        // The editor: tabufline and text.
        let editor = areas.editor;
        if editor.height > 1 && editor.width > 4 {
            let bar = Rect::new(editor.x, editor.y, editor.width, 1);
            let buttons_w = 7u16.min(bar.width);
            let tabs_area = Rect::new(bar.x, bar.y, bar.width.saturating_sub(buttons_w), 1);
            self.tabs_rect = tabs_area;
            self.tab_control.show(
                rt,
                &mut self.tab_state,
                &self.tabs,
                tabs_area,
                p.darker_black,
                buf,
            );
            // Right end: theme toggle and close all.
            let bx = bar.right().saturating_sub(buttons_w);
            self.buttons_rect = Rect::new(bx, bar.y, buttons_w, 1);
            let theme_glyph = if p.dark { Glyph::Sun } else { Glyph::Moon };
            for (i, glyph, color) in [(0u16, theme_glyph, p.yellow), (1, Glyph::Close, p.red)] {
                let r = Rect::new(bx + i * 3 + 1, bar.y, 3, 1).intersection(bar);
                archipel_kit_tui::testing::put_text(
                    buf,
                    bar,
                    r.x,
                    r.y,
                    &look.glyph(glyph),
                    color,
                    p.darker_black,
                );
                rt.clickable(TAB_BUTTONS.with(u64::from(i)), r);
            }
            let body = Rect::new(editor.x, editor.y + 1, editor.width, editor.height - 1);
            if self.tabs.is_empty() {
                self.dashboard.show(rt, &look, body, buf);
            } else {
                let path = self.open[self.tab_state.selected.min(self.open.len() - 1)].clone();
                let idx = self.tab_state.selected.min(self.open.len() - 1);
                let lines = self.lines.get(&path).cloned().unwrap_or_default();
                let mut view = TextArea::new(EDITOR).language(TextLanguage::of_file(&path));
                view.syntax = self.syntax.get();
                view.show(rt, &mut self.areas[idx], &lines, body, buf);
            }
        }
        // Status line.
        let (pos, file) = match self.current_path() {
            Some(path) => {
                let idx = self
                    .tab_state
                    .selected
                    .min(self.areas.len().saturating_sub(1));
                let a = &self.areas[idx];
                (
                    format!("Ln {}, Col {}", a.line + 1, a.col + 1),
                    Some((
                        language_of(path),
                        path.rsplit('/').next().unwrap_or(path).to_owned(),
                    )),
                )
            }
            None => ("Ln -, Col -".to_owned(), None),
        };
        let center = if self.running.get() {
            format!(
                "Running… {}%",
                self.run_lines_shown * 100 / RUN_OUTPUT.len()
            )
        } else {
            rt.hint.clone().unwrap_or_default()
        };
        let mode = self.mode();
        let mut bar = StatusBar::new(&look)
            .mode(mode.label(), mode.color(&p))
            .git(&self.git)
            .center(&center)
            .diagnostics(Diagnostics {
                errors: 0,
                warnings: 1,
                hints: 2,
                info: 0,
            })
            .lsp("rust-analyzer")
            .cwd("kit")
            .cursor(&pos);
        if let Some((icon, name)) = &file {
            bar = bar.file(*icon, name);
        }
        self.status.show(rt, bar, status_row, buf);
        // Layers above everything, then which-key.
        self.layers.draw(rt, area, buf);
        draw_which_key(rt, area, buf, &|id| id.to_owned());
    }
}

impl IdeApp {
    /// The id of the focused tool window, if a tool window has focus.
    pub fn workbench_focus_tool(&self) -> Option<String> {
        self.wb.focused_tool().map(str::to_owned)
    }

    /// The paths of the open files.
    pub fn open_files(&self) -> &[String] {
        &self.open
    }

    /// Whether a run is in progress.
    pub fn is_running(&self) -> bool {
        self.running.get()
    }

    /// Whether the project has the file `path`.
    pub fn has_file(&self, path: &str) -> bool {
        self.files.0.contains_key(path)
    }

    fn perform_action(&mut self, rt: &mut Runtime, id: &str) -> bool {
        let command = match id {
            "Run" => Command::Run,
            "Stop" => Command::Stop,
            "NextTheme" => Command::NextTheme,
            "GotoAction" => Command::SearchEverywhere,
            "CloseActiveTab" | "CloseContent" if self.wb.focused_tool().is_none() => {
                Command::CloseTab
            }
            "NewFile" => Command::NewFile,
            "About" => Command::About,
            "Exit" => Command::Quit,
            "NextTab" | "PreviousTab" if self.wb.focused_tool().is_none() => {
                let delta = if id == "NextTab" { 1 } else { -1 };
                self.tab_state.select_relative(self.tabs.len(), delta);
                return true;
            }
            "Switcher" => Command::SearchEverywhere,
            _ => return false,
        };
        self.run_command(rt, command);
        true
    }

    /// Makes the widget of the focused workbench part the focused widget.
    fn sync_focus(&self, rt: &mut Runtime) {
        if !self.layers.is_empty() {
            return;
        }
        let id = match &self.wb.focus {
            WorkbenchFocus::Editor => EDITOR,
            WorkbenchFocus::Tool(t) if t == "project" => TREE,
            WorkbenchFocus::Tool(t) if t == "run" => RUN_LIST,
            WorkbenchFocus::Tool(_) => NOTES,
        };
        if !rt.focused(id) && !self.focus_is_control(rt) {
            rt.focus.focus(id);
        }
    }

    fn focus_is_control(&self, rt: &Runtime) -> bool {
        [COMBO, TABS].iter().any(|id| rt.focused(*id))
    }

    fn route(&mut self, rt: &mut Runtime, event: &Event) {
        self.sync_focus(rt);
        // Toolbar and combo.
        match self.toolbar.handle(rt, &mut self.toolbar_state, event) {
            ToolbarEvent::OpenGroup { index, anchor } => {
                if let Some(mut menu) = self.toolbar.popup_for(rt, index, anchor) {
                    menu.open(rt, false);
                    self.layers
                        .push(rt, PopupOverlay::new(menu, |_, _| Vec::<Msg>::new()));
                }
            }
            ToolbarEvent::OpenOverflow { anchor } => {
                if let Some(mut menu) = self.toolbar.overflow_menu(rt, anchor) {
                    menu.open(rt, false);
                    self.layers
                        .push(rt, PopupOverlay::new(menu, |_, _| Vec::<Msg>::new()));
                }
            }
            _ => {}
        }
        let combo = ComboBoxControl::new(COMBO, &self.configs).width(22);
        if combo.handle(rt, &mut self.config, event) == ComboEvent::OpenList {
            let anchor = rt.hits.rect_of(COMBO).unwrap_or_default();
            let mut list = combo.popup(&self.config, anchor);
            list.open(rt, true);
            let overlay = PopupOverlay::new(list, |source, state| match state {
                PopupState::Chosen => source.take_chosen().map(Msg::Config).into_iter().collect(),
                _ => Vec::new(),
            });
            self.layers.push(rt, overlay);
        }
        // Status bar.
        match self.status.handle(rt, event) {
            StatusEvent::Clicked(StatusSlot::Cwd, _) => {
                self.run_command(rt, Command::ToggleWindow("project"))
            }
            StatusEvent::Clicked(StatusSlot::Git, rect) => self.branch_popup(rt, rect),
            StatusEvent::Clicked(StatusSlot::Lsp, _) => {
                self.notify(
                    Status::Info,
                    "rust-analyzer",
                    "The language server is idle.",
                );
            }
            StatusEvent::Clicked(StatusSlot::Diagnostics, _) => {
                self.notify(
                    Status::Warning,
                    "Problems",
                    "1 warning, 2 hints in the current file.",
                );
            }
            StatusEvent::Clicked(StatusSlot::Mode, _) => self.run_command(rt, Command::NextTheme),
            StatusEvent::ContextMenu(_, x, y) => self.status_menu(rt, x, y),
            _ => {}
        }
        // Tab buttons.
        if let Event::Mouse(_) = event
            && let Some(p) = rt.pointer().copied()
        {
            if p.clicked(TAB_BUTTONS.with(0)) {
                self.run_command(rt, Command::NextTheme);
            } else if p.clicked(TAB_BUTTONS.with(1)) {
                while !self.tabs.is_empty() {
                    self.close_tab(0);
                }
            }
        }
        // Tabs.
        let before = self.tab_state.selected;
        match self
            .tab_control
            .handle(rt, &mut self.tab_state, &mut self.tabs, event)
        {
            TabsEvent::CloseRequested(i) => self.close_tab(i),
            TabsEvent::ContextMenu { index, x, y } => {
                self.tab_state.selected = index;
                self.tab_menu(rt, x, y);
            }
            TabsEvent::Reordered { from, to } => {
                self.open.swap(from, to);
                self.areas.swap(from, to);
            }
            _ => {}
        }
        let _ = before;
        // Dashboard.
        if self.tabs.is_empty()
            && self.wb.focused_tool().is_none()
            && let Some(i) = self.dashboard.handle(rt, event)
        {
            let command = match i {
                0 => Command::SearchEverywhere,
                1 => Command::NewFile,
                2 => Command::ToggleWindow("project"),
                3 => Command::NextTheme,
                _ => Command::Quit,
            };
            self.run_command(rt, command);
        }
        // Tree.
        match self.tree.handle(rt, TREE, event) {
            TreeEvent::Activated(path) => {
                if let Some(file) = self.file_for(&path) {
                    self.run_command(rt, Command::OpenFile(file));
                }
            }
            TreeEvent::ContextMenu { path, x, y } => {
                let file = path.and_then(|p| self.file_for(&p));
                self.tree_menu(rt, file, x, y);
            }
            _ => {}
        }
        // Run list.
        let rows = self.run_log.clone();
        self.run_list.handle(rt, RUN_LIST, event, &rows);
        // Editor text.
        if !self.tabs.is_empty() {
            let idx = self.tab_state.selected.min(self.open.len() - 1);
            let path = self.open[idx].clone();
            let lines = self.lines.get(&path).cloned().unwrap_or_default();
            let view = TextArea::new(EDITOR);
            view.handle(rt, &mut self.areas[idx], &lines, event);
            if let Event::Mouse(_) = event
                && let Some(p) = rt.pointer().copied()
                && p.right_press()
                && p.over(EDITOR)
            {
                self.editor_menu(rt, p.x, p.y);
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let index = std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<usize>().ok())
        .unwrap_or(0);
    let mut app = IdeApp::new(index);
    let options = runtime::RunOptions {
        report_modifier_keys: true,
        ..runtime::RunOptions::default()
    };
    runtime::run_app_with_keymap(IdeApp::look_for(index), &mut app, options, IdeApp::keymap)
}
