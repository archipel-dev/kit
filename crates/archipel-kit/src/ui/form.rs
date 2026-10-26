//! Settings forms: rows of controls in titled groups, spaced like Android Studio's Settings
//! pages.

use crate::common;
use crate::theme::Theme;
use crate::tokens::{size, typography};
use crate::ui::TitledSeparator;
use egui::{Align, Layout, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, vec2};
use std::cell::Cell;

/// A settings page: rows of controls, optionally in titled groups.
///
/// The form owns the vertical rhythm, so pages built with it look like Android Studio's
/// Settings: checkbox rows 29 pt apart, fields 9 pt apart, a group title with a line and
/// 20 pt above it, the group's rows indented under the title. Put explanations in the
/// controls ([`JBCheckBox::comment`](crate::ui::JBCheckBox::comment),
/// [`JBCheckBox::inline_comment`](crate::ui::JBCheckBox::inline_comment)) instead of extra labels.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{JBCheckBox, Form, JBTextField};
///
/// let (mut lighter, mut readers, mut zoom) = (false, false, String::from("100%"));
/// Form::new().show(ui, |form| {
///     form.add(
///         JBCheckBox::new(&mut lighter, "Different tool window background")
///             .comment("Use a lighter color in the dark theme"),
///     );
///     form.group("Accessibility", |form| {
///         form.row("Zoom:", |ui| ui.add(JBTextField::new(&mut zoom).width(60.0)));
///         form.add(
///             JBCheckBox::new(&mut readers, "Support screen readers")
///                 .inline_comment("Requires restart"),
///         );
///     });
/// });
/// # });
/// ```
#[derive(Debug, Default)]
#[must_use = "show it with `form.show(ui, |form| …)`"]
pub struct Form {}

impl Form {
    /// An empty form.
    pub fn new() -> Self {
        Self {}
    }

    /// Lays out the rows `add` adds, top to bottom. The fields of labelled
    /// [rows](FormUi::row) start in one column, after the widest label, as in Android
    /// Studio's dialogs.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut FormUi<'_>) -> R) -> R {
        let key = ui.next_auto_id().with("archipel-form-labels");
        let labels = Labels {
            width: ui.data(|d| d.get_temp::<f32>(key)).unwrap_or(0.0),
            widest: Cell::new(0.0),
        };
        let inner = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = size::FORM_ROW_GAP;
                add(&mut FormUi {
                    ui,
                    labels: &labels,
                })
            })
            .inner;
        // The column follows the labels of this frame; when it moved, lay the frame out
        // again so no frame shows ragged fields.
        let widest = labels.widest.get();
        if (widest - labels.width).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(key, widest));
            ui.ctx().request_discard("form label column changed");
        }
        inner
    }
}

/// The label column of a [`Form`]: its width as of the last frame, and the widest label
/// seen this frame.
struct Labels {
    width: f32,
    widest: Cell<f32>,
}

/// The rows of a [`Form`].
pub struct FormUi<'u> {
    ui: &'u mut Ui,
    labels: &'u Labels,
}

impl FormUi<'_> {
    /// A row with a control that names itself: a checkbox, radio, toggle or button.
    pub fn add(&mut self, widget: impl Widget) -> Response {
        self.ui.add(widget)
    }

    /// A row with a label (`Theme:`) followed by the fields `add` adds, left to right,
    /// [`FORM_LABEL_GAP`](size::FORM_LABEL_GAP) apart. The label names the row; give
    /// fields an accessible name of their own ([`JBTextField::name`](crate::ui::JBTextField::name)).
    /// Label further fields with [`FieldLabel`] and set independent ones apart with
    /// `ui.add_space(size::FORM_CONTROL_SPACE)`.
    ///
    /// The fields of all labelled rows of a form start in one column, after its widest
    /// label (rows indented in a [group](Self::group) by the group's indent more).
    pub fn row<R>(&mut self, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
        let labels = self.labels;
        self.fields(|ui| {
            let width = ui.add(FieldLabel::new(label)).rect.width();
            labels.widest.set(labels.widest.get().max(width));
            ui.add_space((labels.width - width).max(0.0));
            add(ui)
        })
    }

    /// A row of fields without a leading label, e.g. a checkbox that enables a dropdown
    /// (`[x] Use custom font: [Inter]  Size: [13]`). Like [`row`](Self::row) otherwise.
    pub fn fields<R>(&mut self, add: impl FnOnce(&mut Ui) -> R) -> R {
        // Fields are 28 pt; a few points above and below put their edges where the boxes
        // of neighbouring checkbox rows are.
        self.ui.add_space(size::FORM_FIELD_INSET);
        let inner = self
            .ui
            .allocate_ui_with_layout(
                vec2(self.ui.available_width(), size::CONTROL_HEIGHT),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = size::FORM_LABEL_GAP;
                    add(ui)
                },
            )
            .inner;
        self.ui.add_space(size::FORM_FIELD_INSET);
        inner
    }

    /// An explanation under the labelled [row](Self::row) above, in the smaller gray comment
    /// text, starting where the row's fields start and wrapping at the form's width; as the
    /// comments under Android Studio's settings fields. Checkboxes and radios carry their own
    /// ([`JBCheckBox::comment`](crate::ui::JBCheckBox::comment)).
    pub fn comment(&mut self, text: &str) {
        let indent = self.labels.width + size::FORM_LABEL_GAP;
        let wrap = (self.ui.available_width() - indent).max(size::ICON);
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
        job.wrap.max_width = wrap;
        let galley = self.ui.painter().layout_job(job);
        let lines = galley.rows.len().max(1) as f32;
        let colors = Theme::colors(self.ui.ctx());
        // Close under the field, as a checkbox comment is under its label: undo the row's
        // inset and the gap between rows.
        self.ui
            .add_space(size::COMMENT_GAP - size::FORM_FIELD_INSET - size::FORM_ROW_GAP);
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(indent);
            let (rect, response) = ui.allocate_exact_size(
                vec2(galley.size().x, lines * size::COMMENT_LINE_HEIGHT),
                Sense::hover(),
            );
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
            common::text(ui.painter(), rect.min, galley, colors.text_placeholder);
        });
    }

    /// A row with a label followed by radio buttons (`Open project in (•) New window
    /// ( ) Current window`), [`FORM_CONTROL_GAP`](size::FORM_CONTROL_GAP) apart.
    pub fn radios<R>(&mut self, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
        self.ui
            .allocate_ui_with_layout(
                vec2(self.ui.available_width(), size::ROW_HEIGHT),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = size::FORM_CONTROL_GAP;
                    ui.add(FieldLabel::new(label));
                    add(ui)
                },
            )
            .inner
    }

    /// Extra space before the next row, between rows that are only loosely related. Use a
    /// [group](Self::group) for unrelated ones.
    pub fn space(&mut self) {
        self.ui.add_space(size::FORM_ROWS_GAP);
    }

    /// A titled group: the title with a line to the right edge, then the rows `add` adds,
    /// indented under it.
    pub fn group<R>(&mut self, title: &str, add: impl FnOnce(&mut FormUi<'_>) -> R) -> R {
        self.ui.add(TitledSeparator::new(title));
        self.indent(add)
    }

    /// A group the user shows and hides ("Advanced"): a [collapsible](TitledSeparator::collapsible)
    /// header, then, while `*open`, the rows `add` adds, indented under it. Returns what `add`
    /// returned, `None` while closed.
    pub fn collapsible<R>(
        &mut self,
        title: &str,
        open: &mut bool,
        add: impl FnOnce(&mut FormUi<'_>) -> R,
    ) -> Option<R> {
        self.ui.add(TitledSeparator::new(title).collapsible(open));
        open.then(|| self.indent(add))
    }

    /// Rows that belong to the row above (they only apply when it is checked), indented
    /// under it.
    pub fn indent<R>(&mut self, add: impl FnOnce(&mut FormUi<'_>) -> R) -> R {
        self.ui
            .horizontal_top(|ui| {
                ui.add_space(size::FORM_GROUP_INDENT);
                ui.vertical(|ui| {
                    add(&mut FormUi {
                        ui,
                        labels: self.labels,
                    })
                })
                .inner
            })
            .inner
    }

    /// The underlying `Ui`, for content that is not a row (a table, a preview).
    pub fn ui(&mut self) -> &mut Ui {
        self.ui
    }
}

/// The label of a field in a [`Form`] row (`Size:`), dimmed with the field when it is
/// disabled.
#[must_use = "add it with `ui.add(label)`"]
pub struct FieldLabel {
    text: String,
    enabled: bool,
}

impl FieldLabel {
    /// A label reading `text`, usually ending in a colon.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            enabled: true,
        }
    }

    /// Dims the label along with a disabled field.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for FieldLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let galley = common::galley(ui, self.text.as_str(), typography::body(typography::BODY));
        let (rect, response) =
            ui.allocate_exact_size(vec2(galley.size().x, size::ROW_HEIGHT), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, self.enabled, &self.text));
        let colors = Theme::colors(ui.ctx());
        let color = common::enabled_text(self.enabled, colors.text, &colors);
        common::text_left(ui.painter(), rect, rect.left(), galley, color);
        response
    }
}
