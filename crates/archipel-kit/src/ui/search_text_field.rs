//! `SearchTextField`: a text field for filtering and searching.

use super::text_extensions::{Extension, SearchHistory};
use super::text_field::{JBTextField, JBTextFieldOutput};
use crate::icons::Glyph;
use egui::{Id, Response, Ui, Widget};

/// A search input: IntelliJ's `SearchTextField`.
///
/// A magnifier at the left, a placeholder, and a clear button (a cross that appears while the
/// field has text). With a [history](Self::history) the magnifier becomes a button that
/// lists the earlier searches.
///
/// | Input | |
/// |---|---|
/// | typing | Edits the text; `changed()` after every edit. |
/// | <kbd>Esc</kbd> | Clears the text while there is any ([`clear_on_escape`](Self::clear_on_escape), default on); on an empty field it is left for the window. |
/// | click on the cross | Clears the text and keeps the focus. |
/// | click on the magnifier, <kbd>Alt</kbd>+<kbd>Down</kbd> | Lists the history (needs [`history`](Self::history) with entries). The first 28 pt of the field are the button. |
/// | <kbd>Alt</kbd>+<kbd>Down</kbd> / <kbd>Alt</kbd>+<kbd>Up</kbd> | With the list closed they open it / step to the previous entry; with it open, <kbd>Alt</kbd>+<kbd>Down</kbd> steps to the next. Both wrap around. |
/// | <kbd>Enter</kbd> | Submits: the text is added to the history. |
/// | in the list | <kbd>Up</kbd>/<kbd>Down</kbd> move, <kbd>Enter</kbd> or a click chooses (it goes to the front of the history), <kbd>Esc</kbd> closes. |
///
/// The history keeps five entries by default and is yours: persist it where the app keeps its
/// settings (the kit has no storage).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::ui::{SearchHistory, SearchTextField};
///
/// let mut query = String::new();
/// let mut history = SearchHistory::new();
/// let out = SearchTextField::new(&mut query).history(&mut history).show(ui);
/// if out.submitted { /* run the search */ }
/// # });
/// ```
#[must_use = "add it with `ui.add(field)` or `field.show(ui)`"]
pub struct SearchTextField<'t> {
    text: &'t mut String,
    history: Option<&'t mut SearchHistory>,
    placeholder: String,
    name: Option<String>,
    width: Option<f32>,
    enabled: bool,
    clear_on_escape: bool,
    extensions: Vec<Extension>,
    id: Option<Id>,
}

impl<'t> SearchTextField<'t> {
    /// A search field editing `text`.
    pub fn new(text: &'t mut String) -> Self {
        Self {
            text,
            history: None,
            placeholder: "Search".to_owned(),
            name: None,
            width: None,
            enabled: true,
            clear_on_escape: true,
            extensions: Vec::new(),
            id: None,
        }
    }

    /// Remembers submitted searches and offers them from the magnifier.
    pub fn history(mut self, history: &'t mut SearchHistory) -> Self {
        self.history = Some(history);
        self
    }

    /// The hint shown while the field is empty (default "Search").
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// The accessible name (default: the placeholder).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Total width in points (default: the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Disabled fields are dimmed and read-only.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether <kbd>Esc</kbd> clears the text (default true, IntelliJ `toClearTextOnEscape`).
    pub fn clear_on_escape(mut self, clear: bool) -> Self {
        self.clear_on_escape = clear;
        self
    }

    /// An [extension](Extension) icon inside the field, next to the clear button.
    pub fn extension(mut self, extension: Extension) -> Self {
        self.extensions.push(extension);
        self
    }

    /// The id of the text edit, to focus it from outside.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Adds the field and reports what [`JBTextField::show`] does.
    pub fn show(self, ui: &mut Ui) -> JBTextFieldOutput {
        let mut field = JBTextField::new(self.text)
            .leading_icon(Glyph::Search)
            .placeholder(self.placeholder)
            .clearable(true)
            .clear_on_escape(self.clear_on_escape)
            .enabled(self.enabled);
        for extension in self.extensions {
            field = field.extension(extension);
        }
        if let Some(history) = self.history {
            field = field.history(history);
        }
        if let Some(name) = self.name {
            field = field.name(name);
        }
        if let Some(width) = self.width {
            field = field.width(width);
        }
        if let Some(id) = self.id {
            field = field.id(id);
        }
        field.show(ui)
    }
}

impl Widget for SearchTextField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
