use archipel_kit::behaviour::PopupHandler;
use archipel_kit::editor::{Json, TemplateHighlighter, TemplateToken};
use archipel_kit::egui::{Key, Modifiers};
use archipel_kit::icons::{Category, Language, LanguageStyle, SymbolKind};
use archipel_kit::parts::{Slot, Surface};
use archipel_kit::ui::popup::JBPopup;
use archipel_kit::{Glyph, Icon, Theme, Tone, ValueKind, cursor, egui, tokens};
use archipel_kit::{
    action_system::{ActionButton, ActionMenuItem},
    editor::{CodeEditor, CodeView},
    ide::{HelpTooltip, HelpTooltipExt},
    notification::Notification,
    openapi::ui::{ComboBox, DialogWrapper, Side, Splitter},
    ui::popup::ListSeparator,
    ui::tabs::{JBTabs, TabInfo},
    ui::{
        ActionLink, AsyncProcessIcon, Avatar, Bucket, Button, Cell, ChartColor,
        ColoredListCellRenderer, ColumnInfo, Counter, Extension, Histogram, InlineBanner,
        JBCheckBox, JBIntSpinner, JBTable, JBTextField, Kbd, KeyValueEntry, KeyValueTable,
        OnOffButton, ProgressBar, PropertyGrid, SearchHistory, SearchTextField, SegmentedButton,
        SeparatorComponent, SortOrder, Sparkline, Status, StatusText, Tag, TitledSeparator, Tree,
        TreeNode, TreeTable,
    },
    wm::{SimpleToolWindowPanel, StatusBarWidget},
};

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
    ToolWindowPane,
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
        (Page::ToolWindowPane, "Workbench", Glyph::Layers),
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
            Page::ToolWindowPane => workbench::show(ui, &mut self.workbench, enabled),
        });
    }
}
