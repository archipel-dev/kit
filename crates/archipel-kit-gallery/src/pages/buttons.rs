use super::*;
use archipel_kit::action_system::{
    ActionGroup, ActionItem, ActionPlace, ActionPopupMenu, ActionToolbar, AnAction, AnActionEvent,
    DataContext, DefaultActionGroup, Presentation, Toggle, ToggleAction,
};
use archipel_kit::ui::popup::{BaseListPopupStep, ListPopup, PopupChain};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A command of the gallery: logs its name when performed.
struct Command {
    id: &'static str,
    icon: Option<Glyph>,
    shortcut: &'static str,
    last: Rc<RefCell<String>>,
}

impl AnAction for Command {
    fn id(&self) -> &str {
        self.id
    }

    fn template_presentation(&self) -> Presentation {
        let mut presentation = Presentation::new(self.id);
        if let Some(icon) = self.icon {
            presentation = presentation.icon(icon);
        }
        if !self.shortcut.is_empty() {
            presentation = presentation.shortcut_text(self.shortcut);
        }
        presentation
    }

    fn action_performed(&self, _e: &AnActionEvent<'_>) {
        *self.last.borrow_mut() = self.id.to_owned();
    }
}

/// A toggle of the gallery.
struct SoftWrap(Rc<Cell<bool>>);

impl ToggleAction for SoftWrap {
    fn id(&self) -> &str {
        "Soft-Wrap"
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Soft-Wrap").icon(Glyph::SoftWrap)
    }

    fn is_selected(&self, _e: &AnActionEvent<'_>) -> bool {
        self.0.get()
    }

    fn set_selected(&self, _e: &AnActionEvent<'_>, selected: bool) {
        self.0.set(selected);
    }
}

/// A popup group with an icon, for a toolbar drop-down.
struct AddGroup(DefaultActionGroup);

impl ActionGroup for AddGroup {
    fn name(&self) -> &str {
        "Add"
    }

    fn children(&self, e: Option<&AnActionEvent<'_>>) -> Vec<ActionItem> {
        self.0.children(e)
    }

    fn is_popup(&self) -> bool {
        true
    }

    fn template_presentation(&self) -> Presentation {
        Presentation::new("Add").icon(Glyph::Plus).popup_group(true)
    }
}

pub(super) struct State {
    clicks: usize,
    toggles: [bool; 2],
    last: Rc<RefCell<String>>,
    wrap: Rc<Cell<bool>>,
    menu: PopupChain,
    menu_at: egui::Pos2,
    branches: Option<ListPopup<String>>,
    branch: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            clicks: 0,
            toggles: [true, false],
            last: Rc::default(),
            wrap: Rc::default(),
            menu: PopupChain::default(),
            menu_at: egui::Pos2::ZERO,
            branches: None,
            branch: "main".to_owned(),
        }
    }
}

impl State {
    fn command(&self, id: &'static str, icon: Option<Glyph>, shortcut: &'static str) -> Command {
        Command {
            id,
            icon,
            shortcut,
            last: self.last.clone(),
        }
    }

    fn group(&self) -> DefaultActionGroup {
        let mut add = DefaultActionGroup::popup("Add");
        add.add(self.command("New File", Some(Glyph::NewFile), "Alt+Insert"))
            .add(self.command("New Folder", Some(Glyph::NewFolder), ""));
        let mut group = DefaultActionGroup::new("Gallery");
        group
            .add(self.command("Reload", Some(Glyph::Refresh), "Ctrl+Alt+Y"))
            .add(self.command("Run", Some(Glyph::Run), "Shift+F10"))
            .add_separator()
            .add(Toggle::new(SoftWrap(self.wrap.clone())))
            .add_group(AddGroup(add));
        group
    }

    fn show(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.add(TitledSeparator::new("Variants"));
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
        ui.add(TitledSeparator::new("With icon, small"));
        ui.horizontal(|ui| {
            ui.add(Button::secondary("Run").icon(Glyph::Run).enabled(enabled));
            ui.add(
                Button::secondary("Debug")
                    .icon(Glyph::Debug)
                    .enabled(enabled),
            );
            ui.add(Button::ghost("Small").small(true).enabled(enabled));
        });
        ui.add(TitledSeparator::new("Icon buttons"));
        ui.horizontal(|ui| {
            ui.add(ActionButton::new(Glyph::Refresh, "Reload").enabled(enabled))
                .tooltip_with(HelpTooltip::new("Reload from Disk").shortcut("Ctrl+Alt+Y"));
            ui.add(
                ActionButton::new(Glyph::Run, "Run")
                    .tone(Tone::Run)
                    .enabled(enabled),
            );
            ui.add(
                ActionButton::new(Glyph::Stop, "Stop")
                    .tone(Tone::Stop)
                    .enabled(enabled),
            );
            let mut pinned = self.toggles[1];
            if ui
                .add(
                    ActionButton::new(Glyph::Pin, "Pin")
                        .selected(pinned)
                        .enabled(enabled),
                )
                .clicked()
            {
                pinned = !pinned;
                self.toggles[1] = pinned;
            }
            ui.add(
                ActionButton::new(Glyph::Branch, "Branch")
                    .dropdown(true)
                    .enabled(enabled),
            );
        });
        ui.add(TitledSeparator::new("Links"));
        ui.horizontal(|ui| {
            ui.label("Gradle JVM is not set.");
            ui.add(ActionLink::new("Configure…").enabled(enabled));
            ui.add(ActionLink::new("Docs").external(true).enabled(enabled));
        });

        // An action group, rendered three ways: a toolbar, a list popup and a context menu.
        ui.add(TitledSeparator::new("Action toolbar"));
        let group = self.group();
        let context = DataContext::empty();
        let keymap = archipel_kit::action_system::Keymap::default_for(
            archipel_kit::action_system::Os::Linux,
        );
        ui.add_enabled_ui(enabled, |ui| {
            ActionToolbar::new("gallery-toolbar", &group, &context, ActionPlace::TOOLBAR)
                .keymap(&keymap, false)
                .show(ui);
        });
        ui.label(format!("Last action: {}", self.last.borrow()));

        ui.add(TitledSeparator::new("List popup"));
        ui.horizontal(|ui| {
            let button =
                ui.add(Button::secondary(format!("Branch: {}", self.branch)).enabled(enabled));
            if button.clicked() && self.branches.is_none() {
                let step = BaseListPopupStep::new(
                    "Branches",
                    vec![
                        "main".to_owned(),
                        "develop".to_owned(),
                        "release/1.2".to_owned(),
                    ],
                )
                .icons(|_: &String| Some(Glyph::Branch.into()))
                .speed_search(true);
                let mut popup = ListPopup::new("gallery-branches", step);
                popup.open(ui.ctx());
                self.branches = Some(popup);
            }
            if let Some(popup) = &mut self.branches {
                if let Some(chosen) = popup.show(ui.ctx(), button.rect).chosen {
                    self.branch = chosen;
                }
                if !popup.is_open() {
                    self.branches = None;
                }
            }
        });

        ui.add(TitledSeparator::new("Action popup menu (right click)"));
        let area = ui.add(ActionLink::new("Right-click here for the context menu"));
        if area.secondary_clicked()
            && let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos())
        {
            self.menu_at = pos;
            let now = ui.ctx().input(|i| i.time);
            self.menu.open(now, false);
        }
        ActionPopupMenu::new("gallery-menu", &group, &context, ActionPlace::EDITOR_POPUP)
            .keymap(&keymap, false)
            .show_at(ui.ctx(), &mut self.menu, self.menu_at);
    }
}

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, enabled: bool) {
    state.show(ui, enabled);
}
