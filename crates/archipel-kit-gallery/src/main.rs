//! Archipel Kit gallery: every component, in every state, live.
//!
//! ```bash
//! cargo run -p archipel-kit-gallery
//! ```

use archipel_kit::components::{
    Avatar, Banner, Button, Checkbox, Counter, Dropdown, EmptyState, IconButton, Kbd, Link,
    ListRow, MenuItem, Panel, ProgressBar, PropertyGrid, Radio, SectionHeader, SegmentedControl,
    Separator, Severity, Spinner, Tab, TabBar, Tag, TextField, Toast, Toggle,
};
use archipel_kit::{Icon, Theme, egui, icons, tokens};

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
    Icons,
}

impl Page {
    const ALL: [(Page, &'static str, Icon); 9] = [
        (Page::Buttons, "Buttons", Icon::Run),
        (Page::Inputs, "Text fields", Icon::Search),
        (Page::Choices, "Choices", Icon::Success),
        (Page::Navigation, "Tabs & segments", Icon::Structure),
        (Page::Lists, "Lists & trees", Icon::Project),
        (Page::Feedback, "Feedback", Icon::Warning),
        (Page::Data, "Badges & data", Icon::Model),
        (Page::Layout, "Panels & menus", Icon::Variants),
        (Page::Icons, "Icons", Icon::Ai),
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
                            if dark { Theme::dark() } else { Theme::light() }.apply(ui.ctx());
                        }
                        ui.add(Separator);
                        ui.add(Toggle::new(&mut self.enabled, "Enabled"));
                    });
                });
            });

        egui::SidePanel::left("pages")
            .exact_width(220.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(colors.frame)
                    .inner_margin(egui::Margin {
                        left: 6,
                        right: 2,
                        top: 2,
                        bottom: 6,
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
                        left: 2,
                        right: 6,
                        top: 2,
                        bottom: 6,
                    }),
            )
            .show(ctx, |ui| {
                let title = Page::ALL
                    .iter()
                    .find(|(p, ..)| *p == self.page)
                    .map_or("", |(_, l, _)| l);
                Panel::new(title).show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    egui::ScrollArea::vertical()
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
                    ui.add(Button::secondary("Run").icon(Icon::Run).enabled(enabled));
                    ui.add(
                        Button::secondary("Debug")
                            .icon(Icon::Debug)
                            .enabled(enabled),
                    );
                    ui.add(Button::ghost("Small").small(true).enabled(enabled));
                });
                ui.add(SectionHeader::new("Icon buttons"));
                ui.horizontal(|ui| {
                    let colors = Theme::colors(ui.ctx());
                    ui.add(IconButton::new(Icon::Refresh, "Reload").enabled(enabled));
                    ui.add(
                        IconButton::new(Icon::Run, "Run")
                            .color(colors.run)
                            .enabled(enabled),
                    );
                    ui.add(
                        IconButton::new(Icon::Stop, "Stop")
                            .color(colors.stop)
                            .enabled(enabled),
                    );
                    let mut pinned = self.toggles[1];
                    if ui
                        .add(
                            IconButton::new(Icon::Pin, "Pin")
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
                let colors = Theme::colors(ui.ctx());
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
                        .icon(Icon::Project)
                        .expanded(Some(self.expanded[0])),
                ) {
                    self.expanded[0] = !self.expanded[0];
                }
                if self.expanded[0] {
                    if row(
                        ui,
                        1,
                        ListRow::new("src")
                            .icon(Icon::Folder)
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
                                .label_color(colors.vcs_modified),
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
            }
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
                    .icon(Icon::Send)
                    .hint("Create a request to start testing your API.")
                    .action("New request")
                    .show(ui);
            }
            Page::Data => {
                ui.add(SectionHeader::new("Counters & tags"));
                ui.horizontal(|ui| {
                    let colors = Theme::colors(ui.ctx());
                    ui.add(Counter::new(3));
                    ui.add(Counter::new(12).accent(true));
                    ui.add(Counter::new(1234));
                    ui.add(Separator);
                    ui.add(Tag::new("beta"));
                    ui.add(Tag::new("GET").color(colors.success));
                    ui.add(Tag::new("DELETE").color(colors.error));
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
                    ui.add(MenuItem::new("Copy").icon(Icon::Copy).shortcut("Ctrl+C"));
                    ui.add(
                        MenuItem::new("Find in Files…")
                            .icon(Icon::Find)
                            .shortcut("Ctrl+Shift+F"),
                    );
                    let mut wrap = self.toggles[0];
                    if ui.add(MenuItem::new("Soft-Wrap").checked(wrap)).clicked() {
                        wrap = !wrap;
                        self.toggles[0] = wrap;
                    }
                    ui.add(MenuItem::new("Run").icon(Icon::Run).enabled(false));
                });
                ui.add(SectionHeader::new("Context menu"));
                ui.add(Button::secondary("Right-click me"))
                    .context_menu(|ui| {
                        ui.add(MenuItem::new("Rename…").shortcut("Shift+F6"));
                        ui.add(MenuItem::new("Delete").shortcut("Delete"));
                    });
            }
            Page::Icons => {
                ui.add(TextField::search(&mut self.icon_filter).placeholder("Filter icons"));
                let filter = self.icon_filter.to_lowercase();
                let color = Theme::colors(ui.ctx()).icon;
                egui::Grid::new("icons")
                    .num_columns(4)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        for (index, (icon, name)) in Icon::ALL
                            .iter()
                            .filter(|(_, name)| name.to_lowercase().contains(&filter))
                            .enumerate()
                        {
                            ui.horizontal(|ui| {
                                icons::show(ui, *icon, 16.0, color);
                                ui.label(*name);
                            });
                            if index % 4 == 3 {
                                ui.end_row();
                            }
                        }
                    });
            }
        }
    }
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
}
