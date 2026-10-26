//! `ListPopup`, `PopupStep` and `BaseListPopupStep`: a list that chooses a value.

use super::ListSeparator;
use super::engine::{
    Chosen, LevelSpec, Options, Output, Placement, PopupChain, PopupSource, RowSpec, drive,
};
use crate::icons::Icon;
use egui::{Context, Id, Pos2, Rect};
use std::fmt::Display;

/// What choosing a value leads to (IntelliJ: `onChosen` returns the next `PopupStep`, or
/// `PopupStep.FINAL_CHOICE`).
#[non_exhaustive]
pub enum PopupStepResult<T> {
    /// The choice is final: the popup closes and reports the value.
    FinalChoice,
    /// The next step, shown as a child list beside the row.
    Next(Box<dyn PopupStep<T>>),
    /// Nothing more happens; the popup stays open (a toggle in a list).
    Stay,
}

/// One list of a [`ListPopup`]: its values, how they look and what choosing one does.
///
/// IntelliJ counterpart: `com.intellij.openapi.ui.popup.PopupStep` together with
/// `ListPopupStep`. Rows without a title text are not selectable separators of the previous
/// row via [`separator_for`](Self::separator_for).
pub trait PopupStep<T: Clone> {
    /// The title shown above the rows (IntelliJ `getTitle`).
    fn title(&self) -> Option<&str> {
        None
    }

    /// The values, one per row (IntelliJ `getValues`).
    fn values(&self) -> &[T];

    /// The row text (IntelliJ `getTextFor`).
    fn text_for(&self, value: &T) -> String;

    /// The row icon (IntelliJ `getIconFor`).
    fn icon_for(&self, _value: &T) -> Option<Icon> {
        None
    }

    /// A shortcut shown at the row's end (the kit's counterpart of `getShortcut` of
    /// `ListPopupStepEx`).
    fn shortcut_for(&self, _value: &T) -> Option<String> {
        None
    }

    /// A separator drawn before the row (IntelliJ `getSeparatorAbove`).
    fn separator_for(&self, _value: &T) -> Option<ListSeparator> {
        None
    }

    /// Whether the row can be selected (IntelliJ `isSelectable`).
    fn is_selectable(&self, _value: &T) -> bool {
        true
    }

    /// Whether the row opens a sub-step: a chevron, and Right/Enter open it (IntelliJ
    /// `hasSubstep`).
    fn has_substep(&self, _value: &T) -> bool {
        false
    }

    /// The value was chosen. `final_choice` is false for a row with a sub-step (and for
    /// auto-selection): return the next step (IntelliJ `onChosen`).
    fn on_chosen(&mut self, value: &T, final_choice: bool) -> PopupStepResult<T> {
        let _ = (value, final_choice);
        PopupStepResult::FinalChoice
    }

    /// The popup was dismissed without a choice (IntelliJ `canceled`).
    fn canceled(&mut self) {}

    /// Whether a digit or letter chosen as a row's mnemonic chooses it at once (IntelliJ
    /// `isMnemonicsNavigationEnabled`).
    fn is_mnemonics_navigation_enabled(&self) -> bool {
        false
    }

    /// The row's mnemonic character.
    fn mnemonic_for(&self, _value: &T) -> Option<char> {
        None
    }

    /// Whether typing starts speed search, which filters the rows (IntelliJ
    /// `isSpeedSearchEnabled`).
    fn is_speed_search_enabled(&self) -> bool {
        false
    }

    /// Whether the selected row is chosen (opening its sub-step) after 500 ms without
    /// moving (IntelliJ `isAutoSelectionEnabled`, `ide.popup.auto.delay`).
    fn is_auto_selection_enabled(&self) -> bool {
        false
    }

    /// The row selected when the list opens (IntelliJ `getDefaultOptionIndex`).
    fn default_option_index(&self) -> usize {
        0
    }
}

type TextFn<T> = Box<dyn Fn(&T) -> String>;
type IconFn<T> = Box<dyn Fn(&T) -> Option<Icon>>;
type SubstepFn<T> = Box<dyn Fn(&T) -> bool>;
type ChosenFn<T> = Box<dyn FnMut(&T, bool) -> PopupStepResult<T>>;

/// A ready-made [`PopupStep`]: a list of values with optional title, icons, separators and
/// a choice handler (IntelliJ `BaseListPopupStep`).
///
/// ```
/// use archipel_kit::ui::popup::{BaseListPopupStep, ListPopup};
///
/// let step = BaseListPopupStep::new("Branch", vec!["main", "dev", "release"])
///     .speed_search(true);
/// let popup = ListPopup::new("branches", step);
/// assert!(!popup.is_open());
/// ```
pub struct BaseListPopupStep<T> {
    title: Option<String>,
    values: Vec<T>,
    text: TextFn<T>,
    icon: Option<IconFn<T>>,
    separators: Vec<usize>,
    unselectable: Vec<usize>,
    substep: Option<SubstepFn<T>>,
    on_chosen: Option<ChosenFn<T>>,
    mnemonics: bool,
    speed_search: bool,
    auto_selection: bool,
    default_index: usize,
}

impl<T: Clone + Display + 'static> BaseListPopupStep<T> {
    /// A step titled `title` (may be empty) listing `values`, shown with `Display`.
    pub fn new(title: impl Into<String>, values: Vec<T>) -> Self {
        Self::with_text(title, values, |value| value.to_string())
    }
}

impl<T: Clone + 'static> BaseListPopupStep<T> {
    /// A step whose row text is computed by `text`.
    pub fn with_text(
        title: impl Into<String>,
        values: Vec<T>,
        text: impl Fn(&T) -> String + 'static,
    ) -> Self {
        let title = title.into();
        Self {
            title: (!title.is_empty()).then_some(title),
            values,
            text: Box::new(text),
            icon: None,
            separators: Vec::new(),
            unselectable: Vec::new(),
            substep: None,
            on_chosen: None,
            mnemonics: false,
            speed_search: false,
            auto_selection: false,
            default_index: 0,
        }
    }

    /// Row icons.
    #[must_use]
    pub fn icons(mut self, icon: impl Fn(&T) -> Option<Icon> + 'static) -> Self {
        self.icon = Some(Box::new(icon));
        self
    }

    /// A line before row `index`.
    #[must_use]
    pub fn separator_before(mut self, index: usize) -> Self {
        self.separators.push(index);
        self
    }

    /// Row `index` cannot be selected.
    #[must_use]
    pub fn unselectable(mut self, index: usize) -> Self {
        self.unselectable.push(index);
        self
    }

    /// Which values open a sub-step; `on_chosen` must then return the next step.
    #[must_use]
    pub fn has_substep(mut self, has: impl Fn(&T) -> bool + 'static) -> Self {
        self.substep = Some(Box::new(has));
        self
    }

    /// What choosing does. The default is a final choice.
    #[must_use]
    pub fn on_chosen(
        mut self,
        handler: impl FnMut(&T, bool) -> PopupStepResult<T> + 'static,
    ) -> Self {
        self.on_chosen = Some(Box::new(handler));
        self
    }

    /// Turns mnemonic navigation on: rows show their number (1-9, then letters) and typing
    /// it chooses the row.
    #[must_use]
    pub fn mnemonics(mut self, on: bool) -> Self {
        self.mnemonics = on;
        self
    }

    /// Typing filters the rows.
    #[must_use]
    pub fn speed_search(mut self, on: bool) -> Self {
        self.speed_search = on;
        self
    }

    /// Auto-selection after 500 ms idle.
    #[must_use]
    pub fn auto_selection(mut self, on: bool) -> Self {
        self.auto_selection = on;
        self
    }

    /// The row selected when the list opens.
    #[must_use]
    pub fn default_index(mut self, index: usize) -> Self {
        self.default_index = index;
        self
    }

    fn index_of(&self, value: &T) -> Option<usize> {
        // Rows are found by their text: values need not be comparable.
        let text = (self.text)(value);
        self.values.iter().position(|v| (self.text)(v) == text)
    }
}

impl<T: Clone + 'static> PopupStep<T> for BaseListPopupStep<T> {
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn values(&self) -> &[T] {
        &self.values
    }

    fn text_for(&self, value: &T) -> String {
        (self.text)(value)
    }

    fn icon_for(&self, value: &T) -> Option<Icon> {
        self.icon.as_ref().and_then(|icon| icon(value))
    }

    fn separator_for(&self, value: &T) -> Option<ListSeparator> {
        let index = self.index_of(value)?;
        self.separators.contains(&index).then(ListSeparator::new)
    }

    fn is_selectable(&self, value: &T) -> bool {
        self.index_of(value)
            .is_none_or(|index| !self.unselectable.contains(&index))
    }

    fn has_substep(&self, value: &T) -> bool {
        self.substep.as_ref().is_some_and(|has| has(value))
    }

    fn on_chosen(&mut self, value: &T, final_choice: bool) -> PopupStepResult<T> {
        match &mut self.on_chosen {
            Some(handler) => handler(value, final_choice),
            None => PopupStepResult::FinalChoice,
        }
    }

    fn is_mnemonics_navigation_enabled(&self) -> bool {
        self.mnemonics
    }

    fn mnemonic_for(&self, value: &T) -> Option<char> {
        if !self.mnemonics {
            return None;
        }
        let index = self.index_of(value)?;
        // 1-9, then 0, then A-Z, like the numbering of IntelliJ's action popups.
        let c = match index {
            0..=8 => char::from_digit(index as u32 + 1, 10)?,
            9 => '0',
            _ => char::from_u32('A' as u32 + (index as u32 - 10))?,
        };
        c.is_ascii_alphanumeric().then_some(c)
    }

    fn is_speed_search_enabled(&self) -> bool {
        self.speed_search
    }

    fn is_auto_selection_enabled(&self) -> bool {
        self.auto_selection
    }

    fn default_option_index(&self) -> usize {
        self.default_index
    }
}

/// What a frame of a [`ListPopup`] reported.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListPopupResponse<T> {
    /// The value chosen finally this frame.
    pub chosen: Option<T>,
    /// The popup was dismissed without a choice this frame (Esc, a click outside).
    pub cancelled: bool,
}

/// A list popup that chooses a value, built from [`PopupStep`]s.
///
/// IntelliJ counterpart: `ListPopup` / `ListPopupImpl` (with `WizardPopup`). It is a plain
/// value the caller keeps while the popup is wanted; each frame call [`show`](Self::show).
///
/// | Keys | |
/// |---|---|
/// | <kbd>↑</kbd> <kbd>↓</kbd> | Move the selection, skipping separators and disabled rows; wraps around. |
/// | <kbd>Home</kbd> <kbd>End</kbd> <kbd>PgUp</kbd> <kbd>PgDn</kbd> | First, last, a page up or down. |
/// | <kbd>Enter</kbd> | Choose; a row with a sub-step opens it. |
/// | <kbd>→</kbd> / <kbd>←</kbd> | Open the sub-step / go back to the parent list. |
/// | <kbd>Esc</kbd> / <kbd>Shift</kbd>+<kbd>Esc</kbd> | Back one level (clears speed search first) / close the whole chain. |
/// | mnemonic | With mnemonics on, the row's number or letter chooses it. |
/// | typing | With speed search on, filters the rows. |
///
/// Pointer: hovering selects the row (the first move after opening is ignored); a row with a
/// sub-step opens it after 250 ms when [`show_submenu_on_hover`](Self::show_submenu_on_hover)
/// is on, and a popup with auto-selection chooses after 500 ms idle; a click chooses on
/// release; a press on the chevron opens the sub-step without choosing; a press outside
/// closes the popup, and it cannot reopen for 200 ms.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{Button, popup::{BaseListPopupStep, ListPopup}};
///
/// let mut popup: Option<ListPopup<&str>> = None;
/// let button = ui.add(Button::secondary("Branch"));
/// if button.clicked() {
///     let mut p = ListPopup::new("branches", BaseListPopupStep::new("", vec!["main", "dev"]));
///     p.open(ui.ctx());
///     popup = Some(p);
/// }
/// if let Some(p) = popup.as_mut() {
///     let response = p.show(ui.ctx(), button.rect);
///     if let Some(branch) = response.chosen { let _ = branch; }
/// }
/// # });
/// ```
pub struct ListPopup<T: Clone> {
    id: Id,
    steps: Vec<Box<dyn PopupStep<T>>>,
    chain: PopupChain,
    submenu_on_hover: bool,
    cancel_outside: bool,
}

impl<T: Clone + 'static> ListPopup<T> {
    /// A closed popup with `step` as its first list; `id_salt` identifies it.
    pub fn new(id_salt: impl std::hash::Hash, step: impl PopupStep<T> + 'static) -> Self {
        Self {
            id: Id::new(id_salt),
            steps: vec![Box::new(step)],
            chain: PopupChain::default(),
            submenu_on_hover: false,
            cancel_outside: true,
        }
    }

    /// Row sub-steps open when the pointer rests on them for 250 ms (IntelliJ
    /// `setShowSubmenuOnHover`; action button popups set it). Default off.
    #[must_use]
    pub fn show_submenu_on_hover(mut self, on: bool) -> Self {
        self.submenu_on_hover = on;
        self
    }

    /// Whether a press outside closes the popup (default true).
    #[must_use]
    pub fn cancel_on_click_outside(mut self, on: bool) -> Self {
        self.cancel_outside = on;
        self
    }

    /// Opens the popup with the default row selected. Returns false when it closed less than
    /// 200 ms ago (the reopen guard) or is already open.
    pub fn open(&mut self, ctx: &Context) -> bool {
        let now = ctx.input(|i| i.time);
        if self.chain.is_open() {
            return false;
        }
        self.steps.truncate(1);
        self.chain.open(now, true)
    }

    /// Whether the popup is open.
    pub fn is_open(&self) -> bool {
        self.chain.is_open()
    }

    /// Closes the popup without a choice (no `canceled` callback).
    pub fn close(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        self.chain.close(now);
    }

    /// The state of the chain (selection, open sub-steps), for inspection.
    pub fn chain(&self) -> &PopupChain {
        &self.chain
    }

    /// Shows the popup below `anchor` (above it when there is no room) while open.
    pub fn show(&mut self, ctx: &Context, anchor: Rect) -> ListPopupResponse<T> {
        self.show_placed(ctx, Placement::Below(anchor))
    }

    /// Shows the popup with its top-left corner at `pos`.
    pub fn show_at(&mut self, ctx: &Context, pos: Pos2) -> ListPopupResponse<T> {
        self.show_placed(ctx, Placement::At(pos))
    }

    fn show_placed(&mut self, ctx: &Context, placement: Placement) -> ListPopupResponse<T> {
        let mut source = StepSource {
            steps: &mut self.steps,
            chosen: None,
        };
        let options = Options {
            show_submenu_on_hover: self.submenu_on_hover,
            cancel_on_click_outside: self.cancel_outside,
        };
        let Output { chosen, cancelled } = drive(
            ctx,
            self.id,
            placement,
            options,
            &mut self.chain,
            &mut source,
        );
        let value = if chosen.is_some() {
            source.chosen.take()
        } else {
            None
        };
        if cancelled {
            for step in &mut self.steps {
                step.canceled();
            }
        }
        // Steps of levels that closed are dropped.
        let depth = self.chain.depth().max(1);
        self.steps.truncate(depth.max(1));
        ListPopupResponse {
            chosen: value,
            cancelled,
        }
    }
}

struct StepSource<'a, T: Clone> {
    steps: &'a mut Vec<Box<dyn PopupStep<T>>>,
    chosen: Option<T>,
}

impl<T: Clone> PopupSource for StepSource<'_, T> {
    fn level(&mut self, depth: usize, _path: &[usize]) -> Option<LevelSpec> {
        let step = self.steps.get(depth)?;
        let mnemonics = step.is_mnemonics_navigation_enabled();
        let rows = step
            .values()
            .iter()
            .map(|value| RowSpec {
                text: step.text_for(value),
                icon: step.icon_for(value),
                shortcut: step.shortcut_for(value),
                selectable: step.is_selectable(value),
                separator: step
                    .separator_for(value)
                    .map(|separator| separator.title().map(str::to_owned)),
                sub: step.has_substep(value),
                mnemonic: if mnemonics {
                    step.mnemonic_for(value)
                } else {
                    None
                },
                ..RowSpec::default()
            })
            .collect();
        Some(LevelSpec {
            rows,
            title: step.title().map(str::to_owned),
            speed_search: step.is_speed_search_enabled(),
            auto_select: step.is_auto_selection_enabled(),
            mnemonics,
            default_index: Some(step.default_option_index()),
        })
    }

    fn choose(&mut self, depth: usize, _path: &[usize], row: usize, final_choice: bool) -> Chosen {
        let Some(step) = self.steps.get_mut(depth) else {
            return Chosen::Stay;
        };
        let Some(value) = step.values().get(row).cloned() else {
            return Chosen::Stay;
        };
        match step.on_chosen(&value, final_choice) {
            PopupStepResult::FinalChoice => {
                if final_choice {
                    self.chosen = Some(value);
                    Chosen::Final
                } else {
                    Chosen::Stay
                }
            }
            PopupStepResult::Next(next) => {
                self.steps.truncate(depth + 1);
                self.steps.push(next);
                Chosen::Opened
            }
            PopupStepResult::Stay => Chosen::Stay,
        }
    }
}
