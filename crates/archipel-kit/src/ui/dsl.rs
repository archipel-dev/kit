//! A UI DSL for settings pages and dialogs, after IntelliJ's Kotlin UI DSL v2
//! (`com.intellij.ui.dsl.builder`): [`panel`] with [`Row`]s of [`Cell`]s, groups, comments,
//! gaps and a label column that lines up across rows.
//!
//! | IntelliJ | Here |
//! |---|---|
//! | `panel { }` | [`panel`]`(ui, \|p\| ..)`, a [`Panel`] |
//! | `row("Label:") { }`, `row { }` | [`Panel::row`], [`Panel::unlabeled_row`], a [`Row`] |
//! | `checkBox`, `textField`, `comboBox`, `intTextField`, `radioButton` … | [`Row::check_box`], [`Row::text_field`], [`Row::combo_box`], [`Row::int_spinner`], [`Row::radio_button`] with a [`RadioGroup`] … |
//! | `cell(component)` | [`Row::cell`] with any [`egui::Widget`], [`Row::cell_ui`] with a closure |
//! | `.comment()`, `.contextHelp()`, `.enabledIf()`, `.visibleIf()`, `.align()`, `.gap()` | [`Cell::comment`], [`Cell::context_help`], [`Cell::enabled_if`], [`Cell::visible_if`], [`Cell::align`], [`Cell::gap`] |
//! | `group(title)`, `collapsibleGroup`, `separator`, `indent`, `buttonsGroup` | [`Panel::group`], [`Panel::collapsible_group`], [`Panel::separator`], [`Panel::indent`], [`Panel::buttons_group`] |
//! | `TopGap`, `BottomGap`, `RightGap` | [`Gap`], [`RightGap`] |
//! | `validationOnInput { }` (`ValidationInfo`) | [`Cell::validation`] with a [`ValidationInfo`] |
//!
//! # How it differs from the Kotlin DSL
//!
//! The kit is immediate mode and stateless. Nothing is bound: a cell's widget borrows the
//! caller's value (`r.text_field(&mut name)`) like every other kit widget, and what a cell
//! reports is a [`Response`] (see [`Cell::on_response`]). A row is described first, with its
//! cells' options chained after the cell (`r.check_box(&mut on, "Use proxy").comment("...")`),
//! and laid out when the row closure returns.
//!
//! # Columns line up
//!
//! The label of every [labelled row](Panel::row) is padded to the widest label of the panel
//! (or of its [group](Panel::group): a group has a column of its own) and the cells start in one
//! column, as in IntelliJ's grid. The kit measures instead of
//! solving a grid: the panel remembers the widest label, the cells' heights and (for
//! [`RowLayout::ParentGrid`] rows) the widest cell of each column from the frame before, in
//! the `egui::Context`, and asks egui to lay the frame out again when one changed
//! (`request_discard`), so no frame is shown ragged. The first frame of a panel is laid out
//! twice; after that, one pass. Panels in one `Ui` are told apart by their position, as
//! egui's own auto ids.
//!
//! # Spacing (IntelliJ `SpacingConfiguration`, kept to what Android Studio measured)
//!
//! | | |
//! |---|---|
//! | between rows | 5 pt, plus 4 pt above and below a row with a 28 pt control (checkbox rows are 29 pt apart, a field is 9 pt from a checkbox box) |
//! | label to its cell, `RightGap::Small` | 6 pt (`horizontalSmallGap`) |
//! | between cells, `RightGap::Default` | 16 pt (`horizontalDefaultGap`) |
//! | `RightGap::Columns` | 60 pt (`horizontalColumnsGap`) |
//! | comment to the right of a cell | 12 pt (`horizontalCommentGap`) |
//! | `Gap::Small` / `Gap::Medium` | 8 / 20 pt (`verticalSmallGap`, `verticalMediumGap`) |
//! | group title | 20 pt above (except the first), rows indented 24 pt |
//! | comment under a control | 3 pt below it, 12 pt text |
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use archipel_kit::ui::dsl::{RightGap, panel};
//!
//! let (mut name, mut proxy, mut port) = (String::from("kit"), false, 8080_u16);
//! panel(ui, |p| {
//!     p.row("Name:", |r| {
//!         r.text_field(&mut name).comment("Shown in the project switcher");
//!     });
//!     p.group("Network", |p| {
//!         // A cell borrows its value until its row is laid out: read it first.
//!         let use_proxy = proxy;
//!         p.unlabeled_row(|r| {
//!             r.check_box(&mut proxy, "Use proxy").gap(RightGap::Small);
//!             r.int_spinner(&mut port).enabled_if(use_proxy);
//!         });
//!     });
//! });
//! # });
//! ```

use crate::common;
use crate::openapi::ui::{ComboBox, ValidationInfo};
use crate::parts::Tone;
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use crate::ui::{
    Button, Integer, JBCheckBox, JBIntSpinner, JBRadioButton, JBTextField, SegmentedButton,
    SeparatorComponent, TitledSeparator,
};
use egui::{
    Align, Layout, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType, vec2,
};
use std::cell::{Cell as Counter, RefCell};
use std::rc::Rc;

/// Extra vertical space before or after a row (IntelliJ `TopGap` / `BottomGap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Gap {
    /// No extra space.
    #[default]
    None,
    /// [`size::DSL_GAP_SMALL`] (8 pt).
    Small,
    /// [`size::DSL_GAP_MEDIUM`] (20 pt).
    Medium,
}

impl Gap {
    fn points(self) -> f32 {
        match self {
            Gap::None => 0.0,
            Gap::Small => size::DSL_GAP_SMALL,
            Gap::Medium => size::DSL_GAP_MEDIUM,
        }
    }
}

/// The space after a cell, before the next one (IntelliJ `RightGap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RightGap {
    /// 16 pt, between independent controls.
    #[default]
    Default,
    /// 6 pt, between a control and what belongs to it: a checkbox and its dropdown.
    Small,
    /// 60 pt, between columns of a two-column page.
    Columns,
}

impl RightGap {
    fn points(self) -> f32 {
        match self {
            RightGap::Default => size::FORM_CONTROL_GAP,
            RightGap::Small => size::FORM_LABEL_GAP,
            RightGap::Columns => size::DSL_COLUMNS_GAP,
        }
    }
}

/// Where a cell's label goes (IntelliJ `LabelPosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LabelPosition {
    /// Before the cell.
    Left,
    /// Above the cell.
    Top,
}

/// How a cell uses the room of its row (IntelliJ `AlignX`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CellAlign {
    /// At the left, as wide as it is (default).
    #[default]
    Left,
    /// At the right edge of the row.
    Right,
    /// Takes the room the cells after it leave. Widgets that size themselves to the room
    /// (text fields) grow with it.
    Fill,
}

/// How the cells of a row line up with other rows (IntelliJ `RowLayout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RowLayout {
    /// The label is padded to the panel's label column and the cells follow it: the default
    /// of a [labelled row](Panel::row).
    #[default]
    LabelAligned,
    /// The cells start at the left, whatever other rows do: the default of an
    /// [unlabelled row](Panel::unlabeled_row).
    Independent,
    /// Cell *n* of the row is as wide as cell *n* of the widest row with this layout, so
    /// columns of cells line up across rows (a table of fields).
    ParentGrid,
}

/// What the panel remembers from the previous frame.
#[derive(Debug, Clone, Default, PartialEq)]
struct Measured {
    /// Width of the widest label of each group (scope).
    labels: Vec<f32>,
    /// Widest cell of each [`RowLayout::ParentGrid`] column.
    columns: Vec<f32>,
    /// Height of the controls of each row.
    heights: Vec<f32>,
    /// Width of each cell, for [`CellAlign::Right`] and [`CellAlign::Fill`].
    widths: Vec<f32>,
}

impl Measured {
    fn close_to(&self, other: &Measured) -> bool {
        let same = |a: &[f32], b: &[f32]| {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.5)
        };
        same(&self.labels, &other.labels)
            && same(&self.columns, &other.columns)
            && same(&self.heights, &other.heights)
            && same(&self.widths, &other.widths)
    }
}

struct Shared {
    prev: Measured,
    now: RefCell<Measured>,
    row: Counter<usize>,
    cell: Counter<usize>,
    /// Scopes handed out so far; scope 0 is the panel itself.
    scopes: Counter<usize>,
}

/// Lays out the rows `build` adds, top to bottom, with the label column aligned: IntelliJ's
/// `panel { }`.
pub fn panel<R>(ui: &mut Ui, build: impl FnOnce(&mut Panel<'_>) -> R) -> R {
    let key = ui.next_auto_id().with("archipel-dsl-panel");
    let shared = Shared {
        prev: ui.data(|d| d.get_temp::<Measured>(key)).unwrap_or_default(),
        now: RefCell::new(Measured::default()),
        row: Counter::new(0),
        cell: Counter::new(0),
        scopes: Counter::new(1),
    };
    let inner = ui
        .vertical(|ui| {
            ui.spacing_mut().item_spacing.y = size::FORM_ROW_GAP;
            build(&mut Panel {
                ui,
                shared: &shared,
                scope: 0,
                enabled: true,
                visible: true,
            })
        })
        .inner;
    let now = shared.now.into_inner();
    // The columns follow this frame's measurements; when they moved, lay the frame out again
    // so no frame shows them ragged.
    if !now.close_to(&shared.prev) {
        ui.data_mut(|d| d.insert_temp(key, now));
        ui.ctx().request_discard("dsl panel columns changed");
    }
    inner
}

/// A vertical stack of rows, groups and separators: IntelliJ's `Panel`.
pub struct Panel<'u> {
    ui: &'u mut Ui,
    shared: &'u Shared,
    /// The label column this panel's rows align to: the panel itself, or one of its groups.
    scope: usize,
    enabled: bool,
    visible: bool,
}

impl Panel<'_> {
    /// A row with a label (`Name:`) followed by the cells `build` adds. The labels of all
    /// labelled rows of the panel are padded to the widest, so the cells start in one column
    /// (IntelliJ `row(label) { }`).
    pub fn row<'a>(&mut self, label: &str, build: impl FnOnce(&mut Row<'a>)) {
        let mut row = Row::new(Some(label.to_owned()), RowLayout::LabelAligned);
        build(&mut row);
        self.show_row(row);
    }

    /// A row without a label: its cells start at the left (IntelliJ `row { }`).
    pub fn unlabeled_row<'a>(&mut self, build: impl FnOnce(&mut Row<'a>)) {
        let mut row = Row::new(None, RowLayout::Independent);
        build(&mut row);
        self.show_row(row);
    }

    /// A titled group: the title with a line to the right edge, then the rows `build` adds,
    /// indented 24 pt under it (IntelliJ `group(title) { }`). A group has a label column of
    /// its own.
    pub fn group<R>(&mut self, title: &str, build: impl FnOnce(&mut Panel<'_>) -> R) -> R {
        if self.visible {
            self.ui.add(TitledSeparator::new(title));
        }
        let scope = self.new_scope();
        self.indented(scope, build)
    }

    fn new_scope(&self) -> usize {
        let scope = self.shared.scopes.get();
        self.shared.scopes.set(scope + 1);
        scope
    }

    /// A group the user shows and hides ("Advanced"): a collapsible title, then, while
    /// `*open`, the rows `build` adds. Returns what `build` returned, `None` while closed
    /// (IntelliJ `collapsibleGroup`). The open state is the caller's.
    pub fn collapsible_group<R>(
        &mut self,
        title: &str,
        open: &mut bool,
        build: impl FnOnce(&mut Panel<'_>) -> R,
    ) -> Option<R> {
        if !self.visible {
            return None;
        }
        self.ui.add(TitledSeparator::new(title).collapsible(open));
        let scope = self.new_scope();
        open.then(|| self.indented(scope, build))
    }

    /// A group of radio buttons or toggles: a `title` line (if any) with the rows indented
    /// under it (IntelliJ `buttonsGroup(title) { }`).
    pub fn buttons_group<R>(
        &mut self,
        title: Option<&str>,
        build: impl FnOnce(&mut Panel<'_>) -> R,
    ) -> R {
        let Some(title) = title else {
            return build(&mut Panel {
                ui: &mut *self.ui,
                shared: self.shared,
                scope: self.scope,
                enabled: self.enabled,
                visible: self.visible,
            });
        };
        if self.visible {
            let enabled = self.enabled;
            label_line(self.ui, title, enabled);
            self.ui
                .add_space(size::DSL_BUTTONS_GROUP_HEADER_GAP - size::FORM_ROW_GAP);
        }
        let scope = self.new_scope();
        self.indented(scope, build)
    }

    /// Rows that belong to the row above (they only apply when it is checked), indented
    /// 24 pt (IntelliJ `indent { }`).
    pub fn indent<R>(&mut self, build: impl FnOnce(&mut Panel<'_>) -> R) -> R {
        self.indented(self.scope, build)
    }

    fn indented<R>(&mut self, scope: usize, build: impl FnOnce(&mut Panel<'_>) -> R) -> R {
        let (shared, enabled, visible) = (self.shared, self.enabled, self.visible);
        self.ui
            .horizontal_top(|ui| {
                ui.add_space(size::FORM_GROUP_INDENT);
                ui.vertical(|ui| {
                    build(&mut Panel {
                        ui,
                        shared,
                        scope,
                        enabled,
                        visible,
                    })
                })
                .inner
            })
            .inner
    }

    /// A line across the panel (IntelliJ `separator()`).
    pub fn separator(&mut self) {
        if self.visible {
            let width = self.ui.available_width();
            self.ui.add(SeparatorComponent::length(width));
        }
    }

    /// Extra space before the next row.
    pub fn gap(&mut self, gap: Gap) {
        if self.visible {
            self.ui.add_space(gap.points());
        }
    }

    /// The rows `build` adds are disabled unless `enabled` (IntelliJ `enabledIf`).
    pub fn enabled_if<R>(&mut self, enabled: bool, build: impl FnOnce(&mut Panel<'_>) -> R) -> R {
        build(&mut Panel {
            ui: &mut *self.ui,
            shared: self.shared,
            scope: self.scope,
            enabled: self.enabled && enabled,
            visible: self.visible,
        })
    }

    /// The rows `build` adds are not shown unless `visible` (IntelliJ `visibleIf`). They
    /// still run, so the closure's state changes do not depend on visibility.
    pub fn visible_if<R>(&mut self, visible: bool, build: impl FnOnce(&mut Panel<'_>) -> R) -> R {
        build(&mut Panel {
            ui: &mut *self.ui,
            shared: self.shared,
            scope: self.scope,
            enabled: self.enabled,
            visible: self.visible && visible,
        })
    }

    /// The underlying `Ui`, for content that is not a row (a table, a preview).
    pub fn ui(&mut self) -> &mut Ui {
        self.ui
    }

    fn show_row(&mut self, row: Row<'_>) {
        if !self.visible || !row.visible {
            // Keep the measurements of later rows in step.
            self.shared.row.set(self.shared.row.get() + 1);
            self.shared
                .cell
                .set(self.shared.cell.get() + row.cells.len());
            let mut now = self.shared.now.borrow_mut();
            let (rows, cells) = (self.shared.row.get(), self.shared.cell.get());
            now.heights.resize(rows, size::CONTROL_HEIGHT);
            now.widths.resize(cells, 0.0);
            return;
        }
        row.show(self.ui, self.shared, self.scope, self.enabled);
    }
}

/// A line of text in the label style (a group header, a row's label), dimmed when disabled.
fn label_line(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let galley = common::galley(ui, text, typography::body(typography::BODY));
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x, size::ROW_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, text));
    let colors = Theme::colors(ui.ctx());
    let color = common::enabled_text(enabled, colors.text, &colors);
    common::text_left(ui.painter(), rect, rect.left(), galley, color);
    response
}

/// A comment (12 pt, gray), `indent` points from the left, wrapped to `wrap` (the room after
/// the indent).
fn comment_line(ui: &mut Ui, text: &str, indent: f32, wrap: f32, enabled: bool) -> Response {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: typography::body(typography::COMMENT),
            line_height: Some(size::COMMENT_LINE_HEIGHT),
            color: egui::Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap.max(size::ICON);
    let galley = ui.painter().layout_job(job);
    let lines = galley.rows.len().max(1) as f32;
    // The text starts COMMENT_GAP below what is above it and the box leaves room below.
    let (rect, response) = ui.allocate_exact_size(
        vec2(
            indent + galley.size().x,
            size::COMMENT_SPACE + lines * size::COMMENT_LINE_HEIGHT,
        ),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, text));
    let colors = Theme::colors(ui.ctx());
    common::text(
        ui.painter(),
        rect.min + vec2(indent, size::COMMENT_GAP),
        galley,
        common::enabled_text(enabled, colors.text_placeholder, &colors),
    );
    response
}

/// What a cell's content is told about its cell.
pub struct CellCtx<'c> {
    /// False when the cell, its row or its panel is disabled.
    pub enabled: bool,
    /// The cell's [validation](Cell::validation) message, for content that shows it itself
    /// (text fields do).
    pub validation: Option<&'c ValidationInfo>,
    /// The cell's [width](Cell::width), for content that takes one.
    pub width: Option<f32>,
    /// The row's label without its colon: the accessible name of a field.
    pub name: Option<&'c str>,
}

type Content<'a> = Box<dyn FnOnce(&mut Ui, &CellCtx<'_>) -> Response + 'a>;
type OnResponse<'a> = Box<dyn FnOnce(&Response) + 'a>;

/// One cell of a [`Row`]: a widget and how it sits in the row (IntelliJ `Cell`). The
/// methods chain: `r.text_field(&mut s).comment("...").enabled_if(on)`.
pub struct Cell<'a> {
    content: Option<Content<'a>>,
    comment: Option<String>,
    comment_right: Option<String>,
    context_help: Option<String>,
    label: Option<(String, LabelPosition)>,
    enabled: bool,
    visible: bool,
    gap: RightGap,
    align: CellAlign,
    width: Option<f32>,
    validation: Option<ValidationInfo>,
    /// The content shows `validation` itself (a text field).
    shows_validation: bool,
    /// Indent of the comment under the cell (a checkbox's label starts after its box).
    comment_indent: f32,
    on_response: Option<OnResponse<'a>>,
}

impl<'a> Cell<'a> {
    fn new(content: Content<'a>) -> Self {
        Self {
            content: Some(content),
            comment: None,
            comment_right: None,
            context_help: None,
            label: None,
            enabled: true,
            visible: true,
            gap: RightGap::Default,
            align: CellAlign::Left,
            width: None,
            validation: None,
            shows_validation: false,
            comment_indent: 0.0,
            on_response: None,
        }
    }

    /// An explanation under the cell, in smaller gray text, wrapped to the room of the row
    /// (IntelliJ `comment`). It is the control's accessible description where the control
    /// supports one.
    pub fn comment(&mut self, text: impl Into<String>) -> &mut Self {
        self.comment = Some(text.into());
        self
    }

    /// Indents the [comment](Self::comment) under the cell by `points`: a checkbox's comment
    /// lines up with its label (the [`Row::check_box`] cell does this already).
    pub fn comment_indent(&mut self, points: f32) -> &mut Self {
        self.comment_indent = points;
        self
    }

    /// A short note after the cell, 12 pt away (IntelliJ `commentRight`).
    pub fn comment_right(&mut self, text: impl Into<String>) -> &mut Self {
        self.comment_right = Some(text.into());
        self
    }

    /// A help icon after the cell whose tooltip explains it (IntelliJ `contextHelp`).
    pub fn context_help(&mut self, text: impl Into<String>) -> &mut Self {
        self.context_help = Some(text.into());
        self
    }

    /// A label for the cell, before it or above it (IntelliJ `Cell.label`).
    pub fn label(&mut self, text: impl Into<String>, position: LabelPosition) -> &mut Self {
        self.label = Some((text.into(), position));
        self
    }

    /// The cell is disabled unless `enabled` (IntelliJ `enabledIf`). The widget of a cell
    /// borrows its value until the row is laid out, so read the value this depends on before
    /// the row: `let on = checked; p.unlabeled_row(|r| { r.check_box(&mut checked, "Use
    /// proxy"); r.text_field(&mut host).enabled_if(on); })`.
    pub fn enabled_if(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    /// The cell is not shown unless `visible` (IntelliJ `visibleIf`).
    pub fn visible_if(&mut self, visible: bool) -> &mut Self {
        self.visible = visible;
        self
    }

    /// The space after the cell (IntelliJ `gap(RightGap)`).
    pub fn gap(&mut self, gap: RightGap) -> &mut Self {
        self.gap = gap;
        self
    }

    /// Where the cell sits in the row's room (IntelliJ `align`).
    pub fn align(&mut self, align: CellAlign) -> &mut Self {
        self.align = align;
        self
    }

    /// The width of a field cell in points, where the content takes one (text fields,
    /// spinners, dropdowns). The default is the kit's.
    pub fn width(&mut self, width: f32) -> &mut Self {
        self.width = Some(width);
        self
    }

    /// Shows a [`ValidationInfo`] on the cell: an error gets a red outline, a warning a yellow
    /// one, and the message appears under the cell (IntelliJ `validationOnInput`). `None`
    /// clears it. Text fields draw it themselves; other widgets get the outline around them.
    pub fn validation(&mut self, info: Option<&ValidationInfo>) -> &mut Self {
        self.validation = info.cloned();
        self
    }

    /// Calls `f` with the widget's [`Response`] when the row is laid out: how a cell reports
    /// `changed()` or `clicked()`. It is not called for a cell that is not visible.
    pub fn on_response(&mut self, f: impl FnOnce(&Response) + 'a) -> &mut Self {
        self.on_response = Some(Box::new(f));
        self
    }
}

/// The selection shared by the radio buttons of a group, which may sit in different cells and
/// rows: `let group = RadioGroup::new(&mut indent);` then `r.radio_button(&group, 0, "Tabs")`
/// (IntelliJ `buttonsGroup` with a bound property).
#[derive(Clone)]
pub struct RadioGroup<'a, T>(Rc<RefCell<&'a mut T>>);

impl<'a, T> RadioGroup<'a, T> {
    /// A group whose selected value is `current`.
    pub fn new(current: &'a mut T) -> Self {
        Self(Rc::new(RefCell::new(current)))
    }
}

/// A horizontal line of cells: IntelliJ's `Row`.
pub struct Row<'a> {
    label: Option<String>,
    layout: RowLayout,
    cells: Vec<Cell<'a>>,
    top_gap: Gap,
    bottom_gap: Gap,
    comment: Option<String>,
    enabled: bool,
    visible: bool,
}

impl<'a> Row<'a> {
    fn new(label: Option<String>, layout: RowLayout) -> Self {
        Self {
            label,
            layout,
            cells: Vec::new(),
            top_gap: Gap::None,
            bottom_gap: Gap::None,
            comment: None,
            enabled: true,
            visible: true,
        }
    }

    /// Changes how the row lines up with others (IntelliJ `layout`).
    pub fn layout(&mut self, layout: RowLayout) -> &mut Self {
        self.layout = layout;
        self
    }

    /// Extra space above the row (IntelliJ `topGap`).
    pub fn top_gap(&mut self, gap: Gap) -> &mut Self {
        self.top_gap = gap;
        self
    }

    /// Extra space below the row (IntelliJ `bottomGap`).
    pub fn bottom_gap(&mut self, gap: Gap) -> &mut Self {
        self.bottom_gap = gap;
        self
    }

    /// An explanation under the row, starting where its cells start (IntelliJ `rowComment`).
    pub fn row_comment(&mut self, text: impl Into<String>) -> &mut Self {
        self.comment = Some(text.into());
        self
    }

    /// Disables every cell of the row unless `enabled` (IntelliJ `Row.enabledIf`).
    pub fn enabled_if(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    /// Hides the row unless `visible` (IntelliJ `Row.visibleIf`).
    pub fn visible_if(&mut self, visible: bool) -> &mut Self {
        self.visible = visible;
        self
    }

    /// Adds any widget as a cell (IntelliJ `cell(component)`).
    pub fn cell(&mut self, widget: impl Widget + 'a) -> &mut Cell<'a> {
        self.cell_ui(move |ui| ui.add(widget))
    }

    /// Adds a cell drawn by `add`, which returns the widget's response (for widgets with a
    /// `show` method, or several widgets in one cell).
    pub fn cell_ui(&mut self, add: impl FnOnce(&mut Ui) -> Response + 'a) -> &mut Cell<'a> {
        self.push(Box::new(move |ui, _| add(ui)))
    }

    fn push(&mut self, content: Content<'a>) -> &mut Cell<'a> {
        self.cells.push(Cell::new(content));
        self.cells.last_mut().expect("just pushed")
    }

    /// A text (IntelliJ `label`), followed by a small gap.
    pub fn label(&mut self, text: impl Into<String>) -> &mut Cell<'a> {
        let text = text.into();
        let cell = self.push(Box::new(move |ui, ctx| label_line(ui, &text, ctx.enabled)));
        cell.gap = RightGap::Small;
        cell
    }

    /// A gray comment as a cell of its own (IntelliJ `comment`), wrapped to the room left.
    pub fn comment(&mut self, text: impl Into<String>) -> &mut Cell<'a> {
        let text = text.into();
        self.push(Box::new(move |ui, ctx| {
            let wrap = ui.available_width();
            comment_line(ui, &text, 0.0, wrap, ctx.enabled)
        }))
    }

    /// A [`JBCheckBox`] (IntelliJ `checkBox`). A [comment](Cell::comment) lines up with its
    /// label.
    pub fn check_box(&mut self, checked: &'a mut bool, text: impl Into<String>) -> &mut Cell<'a> {
        let text = text.into();
        let cell = self.push(Box::new(move |ui, ctx| {
            ui.add(JBCheckBox::new(checked, text).enabled(ctx.enabled))
        }));
        cell.comment_indent = size::ICON + size::CHECK_LABEL_GAP;
        cell
    }

    /// A [`JBRadioButton`] of `group`, selected when the group holds `value` (IntelliJ
    /// `radioButton` in a `buttonsGroup`).
    pub fn radio_button<T: PartialEq + 'a>(
        &mut self,
        group: &RadioGroup<'a, T>,
        value: T,
        text: impl Into<String>,
    ) -> &mut Cell<'a> {
        let text = text.into();
        let group = group.0.clone();
        let cell = self.push(Box::new(move |ui, ctx| {
            let mut current = group.borrow_mut();
            ui.add(JBRadioButton::new(&mut **current, value, text).enabled(ctx.enabled))
        }));
        cell.comment_indent = size::ICON + size::CHECK_LABEL_GAP;
        cell
    }

    /// A secondary [`Button`] (IntelliJ `button`). For another variant use
    /// [`cell`](Self::cell) with a [`Button`].
    pub fn button(&mut self, text: impl Into<String>) -> &mut Cell<'a> {
        let text = text.into();
        self.push(Box::new(move |ui, ctx| {
            ui.add(Button::secondary(text).enabled(ctx.enabled))
        }))
    }

    /// A [`JBTextField`] (IntelliJ `textField`), named after the row's label.
    pub fn text_field(&mut self, text: &'a mut String) -> &mut Cell<'a> {
        self.field(text, false)
    }

    /// A masked [`JBTextField`] (IntelliJ `passwordField`).
    pub fn password_field(&mut self, text: &'a mut String) -> &mut Cell<'a> {
        self.field(text, true)
    }

    fn field(&mut self, text: &'a mut String, password: bool) -> &mut Cell<'a> {
        let cell = self.push(Box::new(move |ui, ctx| {
            let mut field = JBTextField::new(text)
                .password(password)
                .enabled(ctx.enabled)
                .validation(ctx.validation);
            if let Some(name) = ctx.name {
                field = field.name(name);
            }
            if let Some(width) = ctx.width {
                field = field.width(width);
            }
            let output = field.show(ui);
            // The cell is the field's frame, not its text edit.
            let mut response = output.response;
            response.rect = output.frame;
            response
        }));
        cell.shows_validation = true;
        cell
    }

    /// A [`JBIntSpinner`] (IntelliJ `spinner` / `intTextField`), named after the row's
    /// label. Set the range with a closure: `r.int_spinner_with(&mut n, |s| s.range(1..=9))`.
    pub fn int_spinner<T: Integer + 'a>(&mut self, value: &'a mut T) -> &mut Cell<'a> {
        self.int_spinner_with(value, |spinner| spinner)
    }

    /// Like [`int_spinner`](Self::int_spinner), configuring the spinner (range, step).
    pub fn int_spinner_with<T: Integer + 'a>(
        &mut self,
        value: &'a mut T,
        configure: impl FnOnce(JBIntSpinner<'a, T>) -> JBIntSpinner<'a, T> + 'a,
    ) -> &mut Cell<'a> {
        self.push(Box::new(move |ui, ctx| {
            let mut spinner = configure(JBIntSpinner::new(value)).enabled(ctx.enabled);
            if let Some(name) = ctx.name {
                spinner = spinner.name(name);
            }
            if let Some(width) = ctx.width {
                spinner = spinner.width(width);
            }
            ui.add(spinner)
        }))
    }

    /// A [`ComboBox`] (IntelliJ `comboBox`), named after the row's label.
    pub fn combo_box<T: PartialEq + Clone + ToString + 'a>(
        &mut self,
        id_salt: impl std::hash::Hash + 'a,
        selected: &'a mut T,
        options: impl IntoIterator<Item = T> + 'a,
    ) -> &mut Cell<'a> {
        self.push(Box::new(move |ui, ctx| {
            let mut combo = ComboBox::new(id_salt, selected, options).enabled(ctx.enabled);
            if let Some(name) = ctx.name {
                combo = combo.label(name);
            }
            if let Some(width) = ctx.width {
                combo = combo.width(width);
            }
            ui.add(combo)
        }))
    }

    /// A [`SegmentedButton`] (IntelliJ `segmentedButton`).
    pub fn segmented_button<T: PartialEq + Clone + 'a>(
        &mut self,
        selected: &'a mut T,
        options: impl IntoIterator<Item = (T, String)> + 'a,
    ) -> &mut Cell<'a> {
        self.push(Box::new(move |ui, ctx| {
            ui.add(SegmentedButton::new(selected, options).enabled(ctx.enabled))
        }))
    }

    fn show(self, ui: &mut Ui, shared: &Shared, scope: usize, panel_enabled: bool) {
        let label_column = shared.prev.labels.get(scope).copied().unwrap_or(0.0);
        let index = shared.row.get();
        shared.row.set(index + 1);
        let first_cell = shared.cell.get();
        shared.cell.set(first_cell + self.cells.len());
        let measured = shared.prev.heights.get(index).copied();
        let height = measured.unwrap_or(size::CONTROL_HEIGHT);
        let row_enabled = panel_enabled && self.enabled;
        let row_name = self
            .label
            .as_deref()
            .map(|label| label.trim_end_matches(':').to_owned());
        let has_comment = self.cells.iter().any(|cell| cell.comment.is_some());
        let inset = if height > size::ROW_HEIGHT + 0.5 {
            size::FORM_FIELD_INSET
        } else {
            0.0
        };
        ui.add_space(self.top_gap.points() + inset);

        let mut tallest = 0.0_f32;
        let mut label_width = 0.0_f32;
        let mut columns: Vec<f32> = Vec::new();
        let mut widths: Vec<f32> = vec![0.0; self.cells.len()];
        let layout = self.layout;
        let label = self.label.clone();
        let visible_after: Vec<usize> = (0..self.cells.len())
            .filter(|&i| self.cells[i].visible)
            .collect();
        let line = ui
            .horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if let Some(label) = &label {
                    let response = ui
                        .allocate_ui_with_layout(
                            vec2(ui.available_width(), height),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.set_min_height(height);
                                label_line(ui, label, row_enabled)
                            },
                        )
                        .inner;
                    label_width = response.rect.width();
                    if layout == RowLayout::LabelAligned {
                        ui.add_space((label_column - label_width).max(0.0));
                    }
                    ui.add_space(size::FORM_LABEL_GAP);
                }
                let mut previous_gap: Option<f32> = None;
                for (position, cell) in self.cells.into_iter().enumerate() {
                    if !cell.visible {
                        continue;
                    }
                    if let Some(gap) = previous_gap {
                        ui.add_space(gap);
                    }
                    let cell_index = first_cell + position;
                    let own = shared.prev.widths.get(cell_index).copied().unwrap_or(0.0);
                    // Room the cells after this one will take.
                    let after: f32 = visible_after
                        .iter()
                        .filter(|&&i| i > position)
                        .map(|&i| {
                            shared
                                .prev
                                .widths
                                .get(first_cell + i)
                                .copied()
                                .unwrap_or(0.0)
                                + self_gap_estimate()
                        })
                        .sum();
                    let column = columns.len();
                    let grid = if layout == RowLayout::ParentGrid {
                        shared.prev.columns.get(column).copied().unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    let align = cell.align;
                    if align == CellAlign::Right {
                        ui.add_space((ui.available_width() - own).max(0.0));
                    }
                    let fill =
                        (align == CellAlign::Fill).then(|| (ui.available_width() - after).max(0.0));
                    let (width, tall, gap) =
                        cell.show(ui, height, row_enabled, row_name.as_deref(), grid, fill);
                    columns.push(width);
                    widths[position] = width;
                    tallest = tallest.max(tall);
                    previous_gap = Some(gap);
                }
            })
            .response;
        let _ = line;
        // The row's own comment lines up with the first cell.
        if let Some(comment) = &self.comment {
            let indent = if self.label.is_some() && layout == RowLayout::LabelAligned {
                label_column + size::FORM_LABEL_GAP
            } else {
                0.0
            };
            let enabled = row_enabled;
            ui.add_space(size::COMMENT_GAP - inset - size::FORM_ROW_GAP);
            let wrap = ui.available_width() - indent;
            comment_line(ui, comment, indent, wrap, enabled);
        } else if !has_comment {
            ui.add_space(inset);
        }
        ui.add_space(self.bottom_gap.points());

        let mut now = shared.now.borrow_mut();
        if now.labels.len() <= scope {
            now.labels.resize(scope + 1, 0.0);
        }
        now.labels[scope] = now.labels[scope].max(label_width);
        now.heights.resize(index + 1, size::CONTROL_HEIGHT);
        now.heights[index] = if tallest > 0.0 { tallest } else { height };
        if layout == RowLayout::ParentGrid {
            if now.columns.len() < columns.len() {
                now.columns.resize(columns.len(), 0.0);
            }
            for (slot, width) in now.columns.iter_mut().zip(&columns) {
                *slot = slot.max(*width);
            }
        }
        let total = first_cell + widths.len();
        now.widths.resize(total, 0.0);
        now.widths[first_cell..total].copy_from_slice(&widths);
    }
}

/// The gap assumed between a cell and the next one when the next one's own is not known yet.
fn self_gap_estimate() -> f32 {
    size::FORM_CONTROL_GAP
}

impl Cell<'_> {
    /// Draws the cell; returns its measured width (without its gap), the height of its
    /// control, and the gap after it.
    fn show(
        mut self,
        ui: &mut Ui,
        row_height: f32,
        row_enabled: bool,
        name: Option<&str>,
        min_width: f32,
        fill: Option<f32>,
    ) -> (f32, f32, f32) {
        let enabled = row_enabled && self.enabled;
        let validation = self.validation.take();
        let content = self.content.take();
        let on_response = self.on_response.take();
        let mut control_height = 0.0_f32;
        let cell = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                if let Some((text, LabelPosition::Top)) = &self.label {
                    label_line(ui, text, enabled);
                }
                let line = ui.allocate_ui_with_layout(
                    vec2(fill.unwrap_or(ui.available_width()), row_height),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.set_min_height(row_height);
                        if let Some(width) = fill {
                            ui.set_max_width(width);
                            ui.set_min_width(width);
                        }
                        if let Some((text, LabelPosition::Left)) = &self.label {
                            label_line(ui, text, enabled);
                            ui.add_space(size::FORM_LABEL_GAP);
                        }
                        let context = CellCtx {
                            enabled,
                            validation: validation.as_ref(),
                            width: self.width,
                            name,
                        };
                        let response = ui
                            .add_enabled_ui(enabled, |ui| match content {
                                Some(content) => content(ui, &context),
                                None => ui.label(""),
                            })
                            .inner;
                        if let Some(info) = &validation
                            && !self.shows_validation
                        {
                            paint_outline(ui, &response, info);
                        }
                        if let Some(text) = self.comment_right.as_deref() {
                            ui.add_space(size::INLINE_COMMENT_GAP);
                            let wrap = ui.available_width();
                            comment_line(ui, text, 0.0, wrap, enabled);
                        }
                        if let Some(help) = self.context_help.as_deref() {
                            ui.add_space(size::FORM_LABEL_GAP);
                            context_help(ui, help);
                        }
                        response
                    },
                );
                let response = line.inner;
                control_height = response.rect.height();
                if let Some(on_response) = on_response {
                    on_response(&response);
                }
                if let Some(info) = &validation
                    && !self.shows_validation
                {
                    validation_message(ui, &response, info);
                }
                if let Some(text) = &self.comment {
                    let wrap = ui.available_width() - self.comment_indent;
                    comment_line(ui, text, self.comment_indent, wrap, enabled);
                }
                response.rect.width()
            })
            .inner;
        let mut width = cell;
        if let Some(width_now) = fill {
            width = width_now;
        }
        // Pad to the grid column.
        if width < min_width {
            ui.add_space(min_width - width);
            width = min_width;
        }
        (width, control_height, self.gap.points())
    }
}

/// The outline of a validated widget that does not draw its own: red for an error, yellow for
/// a warning, 1 pt (the field's own error outline is the same).
fn paint_outline(ui: &Ui, response: &Response, info: &ValidationInfo) {
    let colors = Theme::colors(ui.ctx());
    let color = if info.warning {
        colors.warning
    } else {
        colors.error
    };
    ui.painter().rect_stroke(
        response.rect.expand(1.0),
        egui::CornerRadius::same(radius::CONTROL + 1),
        Stroke::new(1.0_f32, color),
        StrokeKind::Inside,
    );
}

/// The message of a validated widget, under it and as its tooltip.
fn validation_message(ui: &mut Ui, response: &Response, info: &ValidationInfo) {
    let colors = Theme::colors(ui.ctx());
    let color = if info.warning {
        colors.warning
    } else {
        colors.error
    };
    ui.add(
        egui::Label::new(
            egui::RichText::new(&info.message)
                .color(color)
                .font(typography::body(typography::SMALL + 1.0)),
        )
        .extend(),
    );
    crate::ide::HelpTooltip::new(info.message.clone()).show_for(response);
}

/// The help icon after a cell: its tooltip is the explanation.
fn context_help(ui: &mut Ui, text: &str) {
    let (rect, response) = ui.allocate_exact_size(vec2(size::ICON, size::ICON), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
    let colors = Theme::colors(ui.ctx());
    crate::icons::paint(
        ui.painter(),
        rect,
        crate::icons::Glyph::Help,
        Tone::Secondary.color(&colors),
    );
    crate::ide::HelpTooltip::new(text.to_owned()).show_for(&response);
}
