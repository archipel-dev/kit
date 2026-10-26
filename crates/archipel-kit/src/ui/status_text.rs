//! Placeholder text for empty content.

use crate::icons::{self, Icon};
use crate::parts::{SimpleTextAttributes, State};
use crate::theme::Theme;
use crate::tokens::{empty_text, typography};
use crate::ui::Button;
use egui::{Align, Id, Layout, Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// One run of text in an IntelliJ-style status text, possibly a link.
#[derive(Debug, Clone)]
struct Fragment {
    text: String,
    attributes: SimpleTextAttributes,
    link: bool,
}

/// What a [`StatusText`] reports: its response and the link that was clicked.
#[derive(Debug)]
pub struct StatusTextResponse {
    /// The action button's response (legacy status texts), or the response of the text block.
    pub response: Response,
    /// The index of the link fragment that was clicked this frame, counting the links of the
    /// text from the first line on (0 is the first [`link`](StatusText::link)).
    pub link: Option<usize>,
}

/// What to show where a list, table or panel has nothing to display (IntelliJ's
/// `StatusText`, "empty text").
///
/// Two ways to build it:
///
/// - **Lines of text** ([`StatusText::text`], [`append`](Self::append),
///   [`link`](Self::link), [`line`](Self::line)): grayed lines centered in the space, with
///   clickable link fragments, like the empty text of an IDE's lists and tool windows. A
///   [`JBList`](crate::ui::JBList), [`Tree`](crate::ui::Tree), [`JBTable`](crate::ui::JBTable)
///   or [`TreeTable`](crate::ui::TreeTable) shows one in its viewport when it has no rows
///   (`.empty_text(..)`); without one it says "Nothing to show".
/// - **A title, a hint and one action** ([`StatusText::new`], `.hint`, `.icon`, `.action`):
///   a larger empty state for a whole panel.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, ui::StatusText};
///
/// let out = StatusText::text("No requests yet. ")
///     .link("Create one")
///     .append(" or ")
///     .link("import a collection")
///     .show_with_links(ui);
/// match out.link {
///     Some(0) => { /* create a request */ }
///     Some(1) => { /* import */ }
///     _ => {}
/// }
///
/// let out = StatusText::new("No requests yet")
///     .icon(Glyph::Send)
///     .hint("Create a request to start testing your API.")
///     .action("New request")
///     .show(ui);
/// if out.clicked() { /* ... */ }
/// # });
/// ```
pub struct StatusText {
    title: String,
    hint: Option<String>,
    icon: Option<Icon>,
    action: Option<String>,
    lines: Vec<Vec<Fragment>>,
    above_center: bool,
}

impl StatusText {
    /// An empty state titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            hint: None,
            icon: None,
            action: None,
            lines: Vec::new(),
            above_center: false,
        }
    }

    /// IntelliJ-style empty text: one grayed line reading `text` (`setText`). Continue it
    /// with [`append`](Self::append) and [`link`](Self::link), or start another line with
    /// [`line`](Self::line).
    pub fn text(text: impl Into<String>) -> Self {
        let mut status = Self::new("");
        status.lines.push(vec![Fragment {
            text: text.into(),
            attributes: SimpleTextAttributes::GRAYED_ATTRIBUTES,
            link: false,
        }]);
        status
    }

    /// Explanation below the title: why it is empty and how to fill it.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Large icon above the title.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// One primary action button below the hint.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    fn push(mut self, fragment: Fragment) -> Self {
        match self.lines.last_mut() {
            Some(line) => line.push(fragment),
            None => self.lines.push(vec![fragment]),
        }
        self
    }

    /// Adds grayed text to the current line (`appendText`).
    pub fn append(self, text: impl Into<String>) -> Self {
        self.append_with(text, SimpleTextAttributes::GRAYED_ATTRIBUTES)
    }

    /// Adds text in `attributes` to the current line.
    pub fn append_with(self, text: impl Into<String>, attributes: SimpleTextAttributes) -> Self {
        self.push(Fragment {
            text: text.into(),
            attributes,
            link: false,
        })
    }

    /// Adds a link to the current line: link colored, underlined under the pointer, and
    /// reported in [`StatusTextResponse::link`] when clicked.
    pub fn link(self, text: impl Into<String>) -> Self {
        self.push(Fragment {
            text: text.into(),
            attributes: SimpleTextAttributes::LINK_PLAIN_ATTRIBUTES,
            link: true,
        })
    }

    /// Starts a new line with grayed `text` (`appendLine`).
    pub fn line(self, text: impl Into<String>) -> Self {
        self.line_with(text, SimpleTextAttributes::GRAYED_ATTRIBUTES)
    }

    /// Starts a new line with `text` in `attributes`.
    pub fn line_with(mut self, text: impl Into<String>, attributes: SimpleTextAttributes) -> Self {
        self.lines.push(vec![Fragment {
            text: text.into(),
            attributes,
            link: false,
        }]);
        self
    }

    /// Places the text block above the center of the space instead of on it
    /// (`setShowAboveCenter`), for panels with something below it.
    pub fn above_center(mut self, above: bool) -> Self {
        self.above_center = above;
        self
    }

    /// Whether this is an IntelliJ-style text (lines) rather than a title and hint.
    pub(crate) fn is_lines(&self) -> bool {
        !self.lines.is_empty()
    }

    /// Draws it centered in the available width; the response is the action button's
    /// (never clicked if there is no action).
    pub fn show(self, ui: &mut Ui) -> Response {
        self.show_with_links(ui).response
    }

    /// Like [`show`](Self::show), and reports the clicked link of a text made with
    /// [`StatusText::text`].
    pub fn show_with_links(self, ui: &mut Ui) -> StatusTextResponse {
        if self.is_lines() {
            let height = self.block_height(ui);
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), height.max(1.0)), Sense::hover());
            let link = self.paint_lines(ui, rect, ui.id().with("status-text"));
            return StatusTextResponse { response, link };
        }
        let colors = Theme::colors(ui.ctx());
        let response = ui
            .with_layout(Layout::top_down(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add_space(12.0);
                if let Some(icon) = self.icon {
                    icons::show(ui, icon, 32.0, colors.text_placeholder);
                    ui.add_space(2.0);
                }
                ui.label(
                    egui::RichText::new(&self.title)
                        .font(typography::semibold(ui.ctx(), typography::BODY + 1.0))
                        .color(colors.text_strong),
                );
                if let Some(hint) = &self.hint {
                    ui.add(
                        egui::Label::new(egui::RichText::new(hint).color(colors.text_secondary))
                            .wrap(),
                    );
                }
                let response = match &self.action {
                    Some(action) => {
                        ui.add_space(4.0);
                        ui.add(Button::primary(action.as_str()))
                    }
                    None => ui.allocate_response(vec2(0.0, 0.0), Sense::hover()),
                };
                ui.add_space(12.0);
                response
            })
            .inner;
        StatusTextResponse {
            response,
            link: None,
        }
    }

    /// Paints the text block of a collection that has no rows, in `rect` (its viewport).
    pub(crate) fn show_in(self, ui: &mut Ui, rect: Rect, id: Id) -> Option<usize> {
        if self.is_lines() {
            self.paint_lines(ui, rect, id)
        } else {
            self.show_legacy_in(ui, rect)
        }
    }

    fn show_legacy_in(self, ui: &mut Ui, rect: Rect) -> Option<usize> {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::top_down(Align::Center)),
        );
        self.show_with_links(&mut child).link
    }

    /// Height of the lines: each line is as high as its tallest fragment.
    fn block_height(&self, ui: &Ui) -> f32 {
        let state = State {
            enabled: true,
            ..State::default()
        };
        let mut height = 0.0;
        for (index, line) in self.lines.iter().enumerate() {
            let line_height = line
                .iter()
                .map(|f| {
                    crate::parts::layout_run(ui, &f.text, f.attributes, state, f32::INFINITY)
                        .size()
                        .y
                })
                .fold(0.0_f32, f32::max);
            height += line_height;
            if index > 0 {
                height += empty_text::LINE_GAP;
            }
        }
        height
    }

    /// Paints the lines centered in `rect`. Returns the clicked link.
    fn paint_lines(&self, ui: &mut Ui, rect: Rect, id: Id) -> Option<usize> {
        let state = State {
            enabled: true,
            ..State::default()
        };
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let height = self.block_height(ui);
        let top = if self.above_center {
            rect.center().y - height
        } else {
            rect.center().y - height / 2.0
        };
        let mut y = top.max(rect.top());
        let mut clicked = None;
        let mut link_index = 0;
        for (line_index, line) in self.lines.iter().enumerate() {
            let galleys: Vec<_> = line
                .iter()
                .map(|f| crate::parts::layout_run(ui, &f.text, f.attributes, state, f32::INFINITY))
                .collect();
            let line_width: f32 = galleys.iter().map(|g| g.size().x).sum();
            let line_height = galleys.iter().map(|g| g.size().y).fold(0.0_f32, f32::max);
            let line_rect = Rect::from_min_size(
                pos2((rect.center().x - line_width / 2.0).max(rect.left()), y),
                vec2(line_width.min(rect.width()), line_height),
            );
            // The line as one named label for screen readers; links sit on top of it.
            let text: String = line.iter().map(|f| f.text.as_str()).collect();
            let label = ui.interact(line_rect, id.with(("line", line_index)), Sense::hover());
            label.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
            let mut x = line_rect.left();
            for (fragment, galley) in line.iter().zip(galleys) {
                let width = galley.size().x;
                let pos = pos2(x, y + (line_height - galley.size().y) / 2.0);
                if fragment.link {
                    let target = Rect::from_min_size(pos, galley.size());
                    let response =
                        ui.interact(target, id.with(("link", link_index)), Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::labeled(WidgetType::Link, true, &fragment.text)
                    });
                    crate::cursor::on_hover_action(&response);
                    let attributes = if response.hovered() || response.has_focus() {
                        fragment.attributes.underline(true)
                    } else {
                        fragment.attributes
                    };
                    let galley = crate::parts::layout_run(
                        ui,
                        &fragment.text,
                        attributes,
                        state,
                        f32::INFINITY,
                    );
                    painter.galley(pos, galley, egui::Color32::PLACEHOLDER);
                    if response.has_focus() {
                        crate::common::focus_ring(
                            &painter,
                            target.expand(1.0),
                            crate::tokens::radius::CONTROL,
                            &Theme::colors(ui.ctx()),
                        );
                    }
                    if response.clicked() {
                        clicked = Some(link_index);
                    }
                    link_index += 1;
                } else {
                    painter.galley(pos, galley, egui::Color32::PLACEHOLDER);
                }
                x += width;
            }
            y += line_height + empty_text::LINE_GAP;
        }
        clicked
    }
}
