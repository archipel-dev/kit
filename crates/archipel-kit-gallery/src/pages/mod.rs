use archipel_kit::behaviour::ContextMenu;
use archipel_kit::components::code::{Json, TemplateHighlighter, TemplateToken};
use archipel_kit::components::{
    Avatar, Banner, Bucket, Button, Cell, ChartColor, Checkbox, CodeEditor, CodeView, Column,
    Counter, Dialog, Dropdown, EmptyState, Histogram, IconButton, Kbd, KeyValueEntry,
    KeyValueTable, Link, ListRow, MenuItem, MenuSeparator, Panel, ProgressBar, PropertyGrid, Radio,
    SectionHeader, SegmentedControl, Separator, Severity, Side, SortOrder, Sparkline, Spinner,
    SplitPane, StatusBarItem, Tab, TabBar, Table, Tag, TextField, Toast, Toggle, Tooltip,
    TooltipExt, Tree, TreeRow, TreeTable,
};
use archipel_kit::egui::{Key, Modifiers};
use archipel_kit::icons::{Category, Language, LanguageStyle, SymbolKind};
use archipel_kit::parts::{Popover, Slot, Surface};
use archipel_kit::{Glyph, Icon, Theme, Tone, ValueKind, cursor, egui, tokens};

mod buttons;
mod charts;
mod choices;
mod code;
mod data;
mod feedback;
mod icons;
mod inputs;
mod layout;
mod lists;
mod navigation;
mod overlays;
mod workbench;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Page {
    Buttons,
    Inputs,
    Choices,
    Navigation,
    Lists,
    Feedback,
    Data,
    Charts,
    Layout,
    Overlays,
    Code,
    Icons,
    Workbench,
}

impl Page {
    pub(super) const ALL: [(Page, &'static str, Glyph); 13] = [
        (Page::Buttons, "Buttons", Glyph::Run),
        (Page::Inputs, "Text fields", Glyph::Search),
        (Page::Choices, "Choices", Glyph::Success),
        (Page::Navigation, "Tabs & segments", Glyph::Structure),
        (Page::Lists, "Lists & trees", Glyph::Project),
        (Page::Feedback, "Feedback", Glyph::Warning),
        (Page::Data, "Badges & data", Glyph::Package),
        (Page::Charts, "Charts", Glyph::Structure),
        (Page::Layout, "Panels & menus", Glyph::Layers),
        (Page::Overlays, "Tooltips & dialogs", Glyph::Info),
        (Page::Code, "Code", Glyph::Code),
        (Page::Icons, "Icons", Glyph::Sparkle),
        (Page::Workbench, "Workbench", Glyph::Layers),
    ];
}

#[derive(Default)]
pub(super) struct Pages {
    buttons: buttons::State,
    inputs: inputs::State,
    choices: choices::State,
    navigation: navigation::State,
    lists: lists::State,
    feedback: feedback::State,
    data: data::State,
    layout: layout::State,
    overlays: overlays::State,
    code: code::State,
    icons: icons::State,
    charts: charts::State,
    workbench: workbench::State,
}

impl Pages {
    pub(super) fn show(&mut self, page: Page, ui: &mut egui::Ui, enabled: bool) {
        ui.add_enabled_ui(enabled, |ui| match page {
            Page::Buttons => buttons::show(ui, &mut self.buttons, enabled),
            Page::Inputs => inputs::show(ui, &mut self.inputs, enabled),
            Page::Choices => choices::show(ui, &mut self.choices, enabled),
            Page::Navigation => navigation::show(ui, &mut self.navigation, enabled),
            Page::Lists => lists::show(ui, &mut self.lists, enabled),
            Page::Feedback => feedback::show(ui, &mut self.feedback, enabled),
            Page::Data => data::show(ui, &mut self.data, enabled),
            Page::Layout => layout::show(ui, &mut self.layout, enabled),
            Page::Overlays => overlays::show(ui, &mut self.overlays, enabled),
            Page::Code => code::show(ui, &mut self.code, enabled),
            Page::Icons => icons::show(ui, &mut self.icons, enabled),
            Page::Charts => charts::show(ui, &mut self.charts, enabled),
            Page::Workbench => workbench::show(ui, &mut self.workbench, enabled),
        });
    }
}
