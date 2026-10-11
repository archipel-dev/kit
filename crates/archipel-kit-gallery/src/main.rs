//! Archipel Kit gallery: every component, in every state, live.
//!
//! ```bash
//! cargo run -p archipel-kit-gallery
//! ```

use archipel_kit::behaviour::ContextMenu;
use archipel_kit::components::{
    Avatar, Banner, Button, Cell, Checkbox, Column, Counter, Dialog, Dropdown, EmptyState,
    IconButton, Kbd, Link, ListRow, MenuItem, MenuSeparator, Panel, ProgressBar, PropertyGrid,
    Radio, SectionHeader, SegmentedControl, Separator, Severity, SortOrder, Spinner, Tab, TabBar,
    Table, Tag, TextField, Toast, Toggle, Tooltip, TooltipExt,
};
use archipel_kit::icons::{Category, Language, LanguageStyle, SymbolKind};
use archipel_kit::parts::Slot;
use archipel_kit::{Glyph, Icon, Theme, Tone, ValueKind, cursor, egui, tokens};

/// Gap between islands, in whole points for `egui::Margin`.
const GAP: i8 = tokens::size::ISLAND_GAP as i8;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Archipel Kit Gallery")
            .with_inner_size([1100.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Archipel Kit Gallery",
        options,
        Box::new(|cc| {
            archipel_kit::install(&cc.egui_ctx, Theme::dark());
            Ok(Box::new(Gallery::default()))
        }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Buttons,
    Inputs,
    Choices,
    Navigation,
    Lists,
    Feedback,
    Data,
    Layout,
    Overlays,
    Icons,
}

impl Page {
    const ALL: [(Page, &'static str, Glyph); 10] = [
        (Page::Buttons, "Buttons", Glyph::Run),
        (Page::Inputs, "Text fields", Glyph::Search),
        (Page::Choices, "Choices", Glyph::Success),
        (Page::Navigation, "Tabs & segments", Glyph::Structure),
        (Page::Lists, "Lists & trees", Glyph::Project),
        (Page::Feedback, "Feedback", Glyph::Warning),
        (Page::Data, "Badges & data", Glyph::Package),
        (Page::Layout, "Panels & menus", Glyph::Layers),
        (Page::Overlays, "Tooltips & dialogs", Glyph::Info),
        (Page::Icons, "Icons", Glyph::Sparkle),
    ];
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Pretty,
    Raw,
    Preview,
}

struct Gallery {
    page: Page,
    dark: bool,
    enabled: bool,
    clicks: usize,
    name: String,
    query: String,
    port: String,
    password: String,
    checks: [bool; 3],
    indent: usize,
    toggles: [bool; 2],
    view: View,
    tab: usize,
    tabs: Vec<Tab>,
    expanded: [bool; 2],
    selected: usize,
    method: &'static str,
    banners: [bool; 4],
    progress: f32,
    icon_filter: String,
    dialog: Option<GalleryDialog>,
    branch: String,
    busy: bool,
    last_action: String,
    columns: Vec<Column>,
    row: Option<usize>,
    sort: Option<(usize, SortOrder)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryDialog {
    Delete,
    NewBranch,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            page: Page::Buttons,
            dark: true,
            enabled: true,
            clicks: 0,
            name: String::new(),
            query: String::new(),
            port: "8080".into(),
            password: "hunter2".into(),
            checks: [true, false, false],
            indent: 1,
            toggles: [true, false],
            view: View::Pretty,
            tab: 0,
            tabs: ["main.rs", "theme.rs", "Cargo.toml", "README.md"]
                .into_iter()
                .map(|name| Tab::new(name).icon(Icon::for_file(name)).closable(true))
                .collect(),
            expanded: [true, true],
            selected: 2,
            method: "GET",
            banners: [true; 4],
            progress: 0.4,
            icon_filter: String::new(),
            dialog: None,
            branch: "feature/islands".into(),
            busy: false,
            last_action: String::new(),
            columns: vec![
                Column::new("#", 70.0).right(),
                Column::new("Request", 200.0).mono(),
                Column::new("Status", 64.0).right(),
                Column::new("Time", 80.0).right(),
            ],
            row: None,
            sort: None,
        }
    }
}

impl eframe::App for Gallery {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show(ctx);
    }
}

impl Gallery {
    fn show(&mut self, ctx: &egui::Context) {
        let colors = Theme::colors(ctx);
        egui::TopBottomPanel::top("toolbar")
            .show_separator_line(false)
            .exact_height(40.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin::symmetric(12, 0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add(Avatar::new("Archipel Kit"));
                    ui.label(
                        egui::RichText::new("Archipel Kit")
                            .font(tokens::typography::semibold(ui.ctx(), 14.0))
                            .color(colors.text_strong),
                    );
                    ui.add(Tag::new(concat!("v", env!("CARGO_PKG_VERSION"))));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut dark = self.dark;
                        if ui.add(Toggle::new(&mut dark, "Dark")).changed() {
                            self.dark = dark;
                            let mut theme = if dark { Theme::dark() } else { Theme::light() };
                            theme.language_icons = Theme::current(ui.ctx()).language_icons;
                            theme.apply(ui.ctx());
                        }
                        ui.add(Separator);
                        ui.add(Toggle::new(&mut self.enabled, "Enabled"));
                    });
                });
            });

        egui::SidePanel::left("pages")
            .show_separator_line(false)
            .exact_width(220.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin {
                        left: GAP,
                        right: GAP / 2,
                        top: 0,
                        bottom: GAP,
                    }),
            )
            .show(ctx, |ui| {
                Panel::new("Components").show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    for (page, label, icon) in Page::ALL {
                        let row = ListRow::new(label)
                            .icon(icon)
                            .selected(self.page == page)
                            .show(ui);
                        if row.response.clicked() {
                            self.page = page;
                        }
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin {
                        left: GAP / 2,
                        right: GAP,
                        top: 0,
                        bottom: GAP,
                    }),
            )
            .show(ctx, |ui| {
                let title = Page::ALL
                    .iter()
                    .find(|(p, ..)| *p == self.page)
                    .map_or("", |(_, l, _)| l);
                Panel::new(title).show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    archipel_kit::parts::scroll_vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Frame::new()
                                .inner_margin(egui::Margin::symmetric(
                                    tokens::size::CONTENT_INSET as i8,
                                    4,
                                ))
                                .show(ui, |ui| {
                                    ui.set_max_width(640.0);
                                    ui.add_enabled_ui(true, |ui| self.page(ui));
                                });
                        });
                });
            });
    }

    fn show_dialog(&mut self, ctx: &egui::Context) {
        let action = match self.dialog {
            None => return,
            Some(GalleryDialog::Delete) => Dialog::new("Delete “main.rs”?")
                .icon(Glyph::Warning)
                .message("The file will be moved to the trash. You can restore it from there.")
                .primary("Delete")
                .danger(true)
                .secondary("Cancel")
                .show(ctx),
            Some(GalleryDialog::NewBranch) => {
                let valid = !self.branch.trim().is_empty();
                let branch = &mut self.branch;
                Dialog::new("New Branch")
                    .content(|ui| {
                        ui.add(TextField::new(branch).label("Branch name"));
                    })
                    .primary("Create")
                    .primary_enabled(valid)
                    .secondary("Cancel")
                    .width(360.0)
                    .show(ctx)
            }
        };
        if let Some(action) = action {
            self.last_action = format!("{action:?}");
            self.dialog = None;
        }
    }

    fn page(&mut self, ui: &mut egui::Ui) {
        let enabled = self.enabled;
        match self.page {
            Page::Buttons => {
                ui.add(SectionHeader::new("Variants"));
                ui.horizontal(|ui| {
                    for button in [
                        Button::primary("Create"),
                        Button::secondary("Cancel"),
                        Button::ghost("Skip"),
                        Button::danger("Delete"),
                    ] {
                        if ui.add(button.enabled(enabled)).clicked() {
                            self.clicks += 1;
                        }
                    }
                });
                ui.label(format!("Clicked {} times", self.clicks));
                ui.add(SectionHeader::new("With icon, small"));
                ui.horizontal(|ui| {
                    ui.add(Button::secondary("Run").icon(Glyph::Run).enabled(enabled));
                    ui.add(
                        Button::secondary("Debug")
                            .icon(Glyph::Debug)
                            .enabled(enabled),
                    );
                    ui.add(Button::ghost("Small").small(true).enabled(enabled));
                });
                ui.add(SectionHeader::new("Icon buttons"));
                ui.horizontal(|ui| {
                    ui.add(IconButton::new(Glyph::Refresh, "Reload").enabled(enabled))
                        .tooltip_with(Tooltip::new("Reload from Disk").shortcut("Ctrl+Alt+Y"));
                    ui.add(
                        IconButton::new(Glyph::Run, "Run")
                            .tone(Tone::Run)
                            .enabled(enabled),
                    );
                    ui.add(
                        IconButton::new(Glyph::Stop, "Stop")
                            .tone(Tone::Stop)
                            .enabled(enabled),
                    );
                    let mut pinned = self.toggles[1];
                    if ui
                        .add(
                            IconButton::new(Glyph::Pin, "Pin")
                                .selected(pinned)
                                .enabled(enabled),
                        )
                        .clicked()
                    {
                        pinned = !pinned;
                        self.toggles[1] = pinned;
                    }
                });
                ui.add(SectionHeader::new("Links"));
                ui.horizontal(|ui| {
                    ui.label("Gradle JVM is not set.");
                    ui.add(Link::new("Configure…"));
                });
            }
            Page::Inputs => {
                ui.add(
                    TextField::new(&mut self.name)
                        .label("Project name")
                        .placeholder("my-app")
                        .enabled(enabled),
                );
                ui.add(TextField::search(&mut self.query).enabled(enabled));
                let error = self
                    .port
                    .parse::<u16>()
                    .is_err()
                    .then_some("Enter a number from 0 to 65535");
                ui.add(
                    TextField::new(&mut self.port)
                        .label("Port")
                        .error(error)
                        .enabled(enabled),
                );
                ui.add(
                    TextField::new(&mut self.password)
                        .label("Password")
                        .password(true)
                        .enabled(enabled),
                );
                ui.add(SectionHeader::new("Dropdown"));
                ui.add(
                    Dropdown::new(
                        "method",
                        &mut self.method,
                        ["GET", "POST", "PUT", "PATCH", "DELETE"],
                    )
                    .label("HTTP method")
                    .enabled(enabled),
                );
            }
            Page::Choices => {
                ui.add(SectionHeader::new("Checkboxes"));
                let [a, b, c] = &mut self.checks;
                let all = *a && *b && *c;
                let some = *a || *b || *c;
                let mut parent = all;
                if ui
                    .add(
                        Checkbox::new(&mut parent, "All inspections")
                            .mixed(some && !all)
                            .enabled(enabled),
                    )
                    .changed()
                {
                    self.checks = [parent; 3];
                }
                let [a, b, c] = &mut self.checks;
                ui.indent("children", |ui| {
                    ui.add(Checkbox::new(a, "Unused imports").enabled(enabled));
                    ui.add(Checkbox::new(b, "Spelling").enabled(enabled));
                    ui.add(Checkbox::new(c, "Deprecated API").enabled(enabled));
                });
                ui.add(SectionHeader::new("Radio buttons"));
                ui.add(Radio::new(&mut self.indent, 0, "Tabs").enabled(enabled));
                ui.add(Radio::new(&mut self.indent, 1, "Spaces").enabled(enabled));
                ui.add(SectionHeader::new("Toggles"));
                ui.add(
                    Toggle::new(&mut self.toggles[0], "Save files automatically").enabled(enabled),
                );
            }
            Page::Navigation => {
                ui.add(SectionHeader::new("Segmented control"));
                ui.add(
                    SegmentedControl::new(
                        &mut self.view,
                        [
                            (View::Pretty, "Pretty"),
                            (View::Raw, "Raw"),
                            (View::Preview, "Preview"),
                        ],
                    )
                    .enabled(enabled),
                );
                ui.add(SectionHeader::new("Tab bar"));
                let out = TabBar::new(&self.tabs, self.tab).show(ui);
                if let Some(index) = out.selected {
                    self.tab = index;
                }
                if let Some(index) = out.closed {
                    self.tabs.remove(index);
                    self.tab = self.tab.min(self.tabs.len().saturating_sub(1));
                }
                if self.tabs.is_empty() && ui.add(Link::new("Reopen tabs")).clicked() {
                    self.tabs = Gallery::default().tabs;
                }
            }
            Page::Lists => {
                ui.add(SectionHeader::new("Tree"));
                let mut row = |ui: &mut egui::Ui, index: usize, row: ListRow| {
                    let out = row
                        .selected(self.selected == index)
                        .enabled(enabled)
                        .show(ui);
                    if out.response.clicked() {
                        self.selected = index;
                    }
                    out.toggled
                };
                if row(
                    ui,
                    0,
                    ListRow::new("archipel")
                        .icon(Glyph::Project)
                        .expanded(Some(self.expanded[0])),
                ) {
                    self.expanded[0] = !self.expanded[0];
                }
                if self.expanded[0] {
                    if row(
                        ui,
                        1,
                        ListRow::new("src")
                            .icon(Glyph::Folder)
                            .depth(1)
                            .expanded(Some(self.expanded[1])),
                    ) {
                        self.expanded[1] = !self.expanded[1];
                    }
                    if self.expanded[1] {
                        row(
                            ui,
                            2,
                            ListRow::new("main.rs")
                                .icon(Icon::for_file("main.rs"))
                                .depth(2),
                        );
                        row(
                            ui,
                            3,
                            ListRow::new("theme.rs")
                                .icon(Icon::for_file("theme.rs"))
                                .depth(2)
                                .label_tone(Tone::Modified),
                        );
                    }
                    row(
                        ui,
                        4,
                        ListRow::new("Cargo.toml")
                            .icon(Icon::for_file("Cargo.toml"))
                            .depth(1)
                            .secondary("1 KB"),
                    );
                }
                ui.add(SectionHeader::new("Slots and behaviours"));
                ListRow::new("Problems")
                    .icon(Glyph::Warning)
                    .trailing(Slot::text("12"))
                    .enabled(enabled)
                    .with(Tooltip::new("Problems").hint("12 in 3 files"))
                    .show(ui);
                ListRow::new("app")
                    .icon(Glyph::Folder)
                    .enabled(enabled)
                    .trailing_on_hover(true)
                    .trailing(Slot::ui(|ui| {
                        ui.add(
                            IconButton::new(Glyph::Close, "Remove app")
                                .small(true)
                                .enabled(enabled),
                        );
                    }))
                    .with(ContextMenu::new(|ui| {
                        ui.add(MenuItem::new("Open Module Settings"));
                        ui.add(MenuSeparator);
                        ui.add(MenuItem::new("Remove"));
                    }))
                    .show(ui);
            }
            Page::Feedback => {}
            _ => {}
        }
        if self.page == Page::Lists {
            ui.add(SectionHeader::new("Virtualized table: 100 000 rows"));
            let len = 100_000;
            let descending = self
                .sort
                .is_some_and(|(_, order)| order == SortOrder::Descending);
            let out = Table::new("gallery-table", &mut self.columns, len, &mut self.row)
                .sort(self.sort)
                .height(220.0)
                .show(ui, |row, col| {
                    let n = if descending { len - 1 - row } else { row };
                    let status = [200, 201, 404, 200, 500, 304][n % 6];
                    match col {
                        0 => Cell::new(n.to_string()).muted(),
                        1 => Cell::new(format!("GET /api/users/{n}")),
                        2 => Cell::new(status.to_string()).tone(match status {
                            500.. => Tone::Error,
                            400.. => Tone::Warning,
                            _ => Tone::Success,
                        }),
                        _ => Cell::new(format!("{} ms", 12 + n * 37 % 300)).muted(),
                    }
                });
            if let Some(col) = out.sort_clicked {
                self.sort = Some(match self.sort {
                    Some((c, order)) if c == col => (col, order.reversed()),
                    _ => (col, SortOrder::Ascending),
                });
            }
        }
        match self.page {
            Page::Feedback => {
                ui.add(SectionHeader::new("Banners"));
                let banners = [
                    (Severity::Info, "Indexing finished: 1,204 files", None),
                    (Severity::Success, "Gradle sync succeeded", None),
                    (
                        Severity::Warning,
                        "Project JDK is not defined",
                        Some("Set up JDK"),
                    ),
                    (Severity::Error, "Build failed with 3 errors", Some("Retry")),
                ];
                for (index, (severity, text, action)) in banners.into_iter().enumerate() {
                    if self.banners[index] {
                        let mut banner = Banner::new(severity, text).dismissible(true);
                        if let Some(action) = action {
                            banner = banner.action(action);
                        }
                        if banner.show(ui).dismissed {
                            self.banners[index] = false;
                        }
                    }
                }
                if self.banners.iter().any(|shown| !shown)
                    && ui.add(Link::new("Show all banners")).clicked()
                {
                    self.banners = [true; 4];
                }
                ui.add(SectionHeader::new("Toast"));
                Toast::new(Severity::Success, "Build finished")
                    .body("3 modules compiled in 4.2 s")
                    .action("Open log")
                    .show(ui);
                ui.add(SectionHeader::new("Progress"));
                ui.horizontal(|ui| {
                    ui.add(Spinner::new());
                    ui.label("Indexing…");
                });
                ui.add(ProgressBar::new(Some(self.progress)));
                ui.add(egui::Slider::new(&mut self.progress, 0.0..=1.0).text("fraction"));
                ui.add(ProgressBar::new(None));
                ui.add(SectionHeader::new("Empty state"));
                EmptyState::new("No requests yet")
                    .icon(Glyph::Send)
                    .hint("Create a request to start testing your API.")
                    .action("New request")
                    .show(ui);
            }
            Page::Data => {
                ui.add(SectionHeader::new("Counters & tags"));
                ui.horizontal(|ui| {
                    ui.add(Counter::new(3));
                    ui.add(Counter::new(12).accent(true));
                    ui.add(Counter::new(1234));
                    ui.add(Separator);
                    ui.add(Tag::new("beta"));
                    ui.add(Tag::new("GET").tone(Tone::Success));
                    ui.add(Tag::new("DELETE").tone(Tone::Error));
                });
                ui.add(SectionHeader::new("Keyboard shortcuts"));
                ui.horizontal(|ui| {
                    ui.add(Kbd::parse("Ctrl+Shift+F"));
                    ui.add(Kbd::new(["⌘", "K"]));
                });
                ui.add(SectionHeader::new("Avatars"));
                ui.horizontal(|ui| {
                    for name in ["archipel-ide", "nexus-http", "mongo-raabit"] {
                        ui.add(Avatar::new(name).size(24.0));
                    }
                    ui.add(Avatar::new("Ada Lovelace").size(24.0).round(true));
                });
                data_colors(ui);
                ui.add(SectionHeader::new("Property grid"));
                PropertyGrid::new("props")
                    .row("Status", "200 OK")
                    .row("Time", "142 ms")
                    .mono_row("Content-Type", "application/json")
                    .show(ui);
            }
            Page::Layout => {
                ui.add(SectionHeader::new("Menu"));
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(260.0);
                    ui.add(MenuItem::new("Copy").icon(Glyph::Copy).shortcut("Ctrl+C"));
                    ui.add(
                        MenuItem::new("Find in Files…")
                            .icon(Glyph::FindInFiles)
                            .shortcut("Ctrl+Shift+F"),
                    );
                    ui.add(MenuSeparator);
                    let mut wrap = self.toggles[0];
                    if ui.add(MenuItem::new("Soft-Wrap").checked(wrap)).clicked() {
                        wrap = !wrap;
                        self.toggles[0] = wrap;
                    }
                    ui.add(MenuItem::new("Run").icon(Glyph::Run).enabled(false));
                });
                ui.add(SectionHeader::new("Context menu"));
                ui.add(Button::secondary("Right-click me"))
                    .context_menu(|ui| {
                        ui.add(MenuItem::new("Rename…").shortcut("Shift+F6"));
                        ui.add(MenuItem::new("Delete").shortcut("Delete"));
                    });
            }
            Page::Overlays => {
                ui.add(SectionHeader::new("Tooltips"));
                ui.horizontal(|ui| {
                    ui.add(Button::secondary("Hover me"))
                        .tooltip_with(Tooltip::new("Run 'gallery'").shortcut("Shift+F10"));
                    ui.add(Button::secondary("With hint")).tooltip_with(
                        Tooltip::new("Search Everywhere")
                            .shortcut("Double Shift")
                            .hint("Find files, actions, classes and symbols in one place."),
                    );
                    ui.add(Button::primary("Deploy").enabled(false))
                        .tooltip("Connect a cluster first");
                });
                ui.add(SectionHeader::new("Dialogs"));
                ui.horizontal(|ui| {
                    if ui.add(Button::danger("Delete file…")).clicked() {
                        self.dialog = Some(GalleryDialog::Delete);
                    }
                    if ui.add(Button::secondary("New branch…")).clicked() {
                        self.dialog = Some(GalleryDialog::NewBranch);
                    }
                });
                if !self.last_action.is_empty() {
                    ui.label(format!("Last dialog: {}", self.last_action));
                }
                ui.add(SectionHeader::new("Context menu"));
                ui.add(Button::secondary("Right-click me"))
                    .context_menu(|ui| {
                        ui.add(MenuItem::new("Rename…").shortcut("Shift+F6"));
                        ui.add(MenuItem::new("Copy Path").icon(Glyph::Copy));
                        ui.add(MenuSeparator);
                        ui.add(
                            MenuItem::new("Delete")
                                .icon(Glyph::Delete)
                                .shortcut("Delete"),
                        );
                    });
                ui.add(SectionHeader::new("Cursors"));
                ui.add(Checkbox::new(
                    &mut self.busy,
                    "Busy: override every cursor below",
                ));
                cursor::override_in(ui, self.busy.then_some(egui::CursorIcon::Progress), |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Button::secondary("Arrow"));
                        ui.add(Link::new("Hand"));
                        ui.add(TextField::new(&mut self.query).placeholder("Text beam"));
                    });
                });
                self.show_dialog(ui.ctx());
            }
            Page::Icons => {
                ui.add(TextField::search(&mut self.icon_filter).placeholder("Search icons"));
                ui.add(SectionHeader::new("Language icon style"));
                let mut style = Theme::current(ui.ctx()).language_icons;
                let styles = LanguageStyle::ALL.map(|style| (style, style.name()));
                if ui.add(SegmentedControl::new(&mut style, styles)).changed() {
                    let mut theme = (*Theme::current(ui.ctx())).clone();
                    theme.language_icons = style;
                    theme.apply(ui.ctx());
                }
                let color = Theme::colors(ui.ctx()).icon;
                let matches: Vec<Glyph> = Glyph::search(&self.icon_filter).collect();
                for category in Category::ALL {
                    let glyphs: Vec<Glyph> = matches
                        .iter()
                        .copied()
                        .filter(|g| g.category() == category)
                        .collect();
                    if glyphs.is_empty() {
                        continue;
                    }
                    ui.add(SectionHeader::new(category.label()));
                    egui::Grid::new(category.label())
                        .num_columns(4)
                        .spacing([24.0, 8.0])
                        .show(ui, |ui| {
                            for (index, glyph) in glyphs.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    archipel_kit::icons::show(ui, *glyph, 16.0, color);
                                    ui.label(glyph.name());
                                })
                                .response
                                .on_hover_text(glyph.keywords().join(", "));
                                if index % 4 == 3 {
                                    ui.end_row();
                                }
                            }
                        });
                }
                let query = self.icon_filter.to_lowercase();
                let languages: Vec<Language> = Language::ALL
                    .iter()
                    .copied()
                    .filter(|l| {
                        l.name().to_lowercase().contains(&query)
                            || l.extensions().iter().any(|e| e.contains(&query))
                    })
                    .collect();
                if !languages.is_empty() {
                    ui.add(SectionHeader::new("Languages"));
                    egui::Grid::new("languages")
                        .num_columns(4)
                        .spacing([24.0, 8.0])
                        .show(ui, |ui| {
                            for (index, language) in languages.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    archipel_kit::icons::show(ui, *language, 16.0, color);
                                    ui.label(language.name());
                                })
                                .response
                                .on_hover_text(
                                    language
                                        .extensions()
                                        .iter()
                                        .map(|e| format!(".{e}"))
                                        .collect::<Vec<_>>()
                                        .join(" "),
                                );
                                if index % 4 == 3 {
                                    ui.end_row();
                                }
                            }
                        });
                }
                if query.is_empty() || "symbol".contains(&query) {
                    ui.add(SectionHeader::new("Symbols"));
                    ui.horizontal_wrapped(|ui| {
                        for kind in SymbolKind::ALL {
                            archipel_kit::icons::show(ui, kind, 16.0, color)
                                .on_hover_text(kind.name());
                        }
                    });
                }
                if matches.is_empty() && languages.is_empty() {
                    EmptyState::new("No icons match")
                        .hint("Try a keyword such as \"http\" or \"trash\".")
                        .show(ui);
                }
            }
            _ => {}
        }
    }
}

/// Typed-value, chart, ramp and method/status colors.
fn data_colors(ui: &mut egui::Ui) {
    ui.add(SectionHeader::new("Data colors"));
    let theme = Theme::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        for kind in ValueKind::ALL {
            ui.label(
                egui::RichText::new(kind.label())
                    .monospace()
                    .color(kind.color(&theme)),
            );
        }
    });
    ui.horizontal(|ui| {
        for index in 0..8 {
            swatch(ui, theme.chart(index));
        }
    });
    ui.horizontal(|ui| {
        for step in 0..=10 {
            swatch(ui, theme.colors.ramp(step as f32 / 10.0));
        }
    });
    ui.horizontal(|ui| {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            ui.add(Tag::new(method).tone(Tone::method(method)));
        }
        for code in [200, 301, 404, 503] {
            ui.add(Tag::new(code.to_string()).tone(Tone::status(code)));
        }
    });
}

fn swatch(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, tokens::radius::CONTROL, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_kit::testing::{Harness, Queryable, SnapshotOptions, SnapshotResults};

    /// Renders every page in both themes: the gallery must not panic, and the images show
    /// up in review when a component changes. The Buttons page is the README screenshot.
    #[test]
    fn every_page_renders() {
        let mut results = SnapshotResults::new();
        for dark in [true, false] {
            for (page, label, _) in Page::ALL {
                let mut gallery = Gallery {
                    page,
                    dark,
                    ..Gallery::default()
                };
                let mut installed = false;
                let mut harness = Harness::builder()
                    .with_size(egui::vec2(1000.0, 640.0))
                    .with_pixels_per_point(2.0)
                    .with_max_steps(30)
                    .wgpu()
                    .build(move |ctx| {
                        if !installed {
                            installed = true;
                            archipel_kit::install(
                                ctx,
                                if dark { Theme::dark() } else { Theme::light() },
                            );
                            ctx.request_repaint();
                            return;
                        }
                        gallery.show(ctx);
                    });
                harness.run_ok();
                if page == Page::Buttons {
                    let theme = if dark { "dark" } else { "light" };
                    results.add(harness.try_snapshot_options(
                        format!("gallery.{theme}"),
                        &SnapshotOptions::new().output_path("../../docs/img"),
                    ));
                }
                assert!(
                    harness.query_by_label(label).is_some(),
                    "page {label} lists itself in the sidebar"
                );
            }
        }
        results.unwrap();
    }

    /// Every page passes the kit's UX audit: named controls, big enough targets, no
    /// overlaps, everything reachable by Tab.
    #[test]
    fn every_page_passes_the_ux_audit() {
        let mut failures = Vec::new();
        for (page, label, _) in Page::ALL {
            let mut gallery = Gallery {
                page,
                ..Gallery::default()
            };
            let mut installed = false;
            let mut harness = Harness::builder()
                .with_size(egui::vec2(1000.0, 640.0))
                .with_max_steps(30)
                .build(move |ctx| {
                    if !installed {
                        installed = true;
                        archipel_kit::install(ctx, Theme::dark());
                        ctx.request_repaint();
                        return;
                    }
                    gallery.show(ctx);
                });
            harness.run_ok();
            let audit = archipel_kit::testing::audit(&mut harness);
            for finding in audit.findings {
                failures.push(format!("{label}: {finding}"));
            }
        }
        assert!(
            failures.is_empty(),
            "UX audit:\n  {}",
            failures.join("\n  ")
        );
    }
}
