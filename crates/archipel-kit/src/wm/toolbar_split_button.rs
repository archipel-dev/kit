//! The split drop-down of the main toolbar: the run widget.

use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::{State, Tone};
use crate::theme::Theme;
use crate::tokens::{radius, size, toolbar, typography};
use egui::{Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// What [`ToolbarSplitButton::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ToolbarSplitResponse {
    /// The main part: the action itself (run the selected configuration).
    pub main: Response,
    /// The arrow part: opens the list (choose the configuration).
    pub arrow: Response,
}

/// A main toolbar button in two parts: an action with an icon and text, and a chevron that
/// opens a list, divided by a short line (IntelliJ `MainToolbar.SplitDropdown`, the run
/// configuration widget of the center zone). Each part has its own hover and press fill.
/// Open the list with a [`ListPopup`](crate::ui::popup::ListPopup) or an
/// [`ActionPopupMenu`](crate::action_system::ActionPopupMenu) anchored to the arrow's response.
///
/// Measured values are IntelliJ spec §14: left part margin 7, separator margin 3 and 20 pt
/// long, right part 26 pt (`tokens::toolbar::SPLIT_*`).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, wm::ToolbarSplitButton};
///
/// let out = ToolbarSplitButton::new("app").icon(Glyph::Run).show(ui);
/// if out.main.clicked() { /* run */ }
/// if out.arrow.clicked() { /* open the configuration list */ }
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
pub struct ToolbarSplitButton {
    label: String,
    icon: Option<Icon>,
    enabled: bool,
    open: bool,
}

impl ToolbarSplitButton {
    /// A split button reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            enabled: true,
            open: false,
        }
    }

    /// The 20-point icon before the text.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Disabled buttons are dimmed and ignore input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the list is open: the arrow part stays filled.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Draws the button.
    pub fn show(self, ui: &mut Ui) -> ToolbarSplitResponse {
        let colors = Theme::colors(ui.ctx());
        let font = typography::body(typography::BODY);
        let natural = common::galley(ui, self.label.as_str(), font.clone());
        let lead = if self.icon.is_some() {
            toolbar::ICON + toolbar::ICON_GAP
        } else {
            0.0
        };
        let pad = 7.0;
        let main_width = (pad + lead + natural.size().x + pad)
            .min(toolbar::DROPDOWN_MAX_WIDTH - toolbar::SPLIT_ARROW_WIDTH);
        let room = (main_width - 2.0 * pad - lead).max(0.0);
        let galley = if natural.size().x > room {
            common::galley_elided(ui, self.label.as_str(), font, room)
        } else {
            natural
        };
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let separator = 2.0 * toolbar::SPLIT_SEPARATOR_MARGIN + 1.0;
        let (rect, _) = ui.allocate_exact_size(
            vec2(
                main_width + separator + toolbar::SPLIT_ARROW_WIDTH,
                toolbar::BUTTON,
            ),
            Sense::hover(),
        );
        let main_rect = Rect::from_min_size(rect.min, vec2(main_width, rect.height()));
        let arrow_rect = Rect::from_min_size(
            pos2(rect.right() - toolbar::SPLIT_ARROW_WIDTH, rect.top()),
            vec2(toolbar::SPLIT_ARROW_WIDTH, rect.height()),
        );
        let main = ui.interact(main_rect, ui.id().with((&self.label, "main")), sense);
        let arrow = ui.interact(arrow_rect, ui.id().with((&self.label, "arrow")), sense);
        main.widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, &self.label));
        let arrow_name = format!("{} options", self.label);
        arrow.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, self.enabled, &arrow_name));
        crate::cursor::on_hover_action(&main);
        crate::cursor::on_hover_action(&arrow);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let corner = radius::ICON_BUTTON;
            for (part, response, open) in
                [(main_rect, &main, false), (arrow_rect, &arrow, self.open)]
            {
                let fill = if !self.enabled {
                    egui::Color32::TRANSPARENT
                } else if open {
                    colors.pressed
                } else {
                    common::flat_fill(response, &colors)
                };
                painter.rect_filled(part, corner, fill);
                if response.has_focus() {
                    common::focus_ring(painter, part, corner, &colors);
                }
            }
            let state = State::of(&main);
            let mut x = main_rect.left() + pad;
            if let Some(icon) = self.icon {
                let icon_rect = Rect::from_center_size(
                    pos2(x + toolbar::ICON / 2.0, rect.center().y),
                    vec2(toolbar::ICON, toolbar::ICON),
                );
                icons::paint(painter, icon_rect, icon, state.icon(Tone::Icon, &colors));
                x += lead;
            }
            let text_color = if self.enabled {
                colors.text
            } else {
                colors.text_disabled
            };
            common::text_left(painter, main_rect, x, galley, text_color);
            let line_x = main_rect.right() + toolbar::SPLIT_SEPARATOR_MARGIN + 0.5;
            painter.vline(
                line_x,
                (rect.center().y - toolbar::SPLIT_SEPARATOR_LENGTH / 2.0)
                    ..=(rect.center().y + toolbar::SPLIT_SEPARATOR_LENGTH / 2.0),
                Stroke::new(1.0_f32, colors.separator),
            );
            let chevron = Rect::from_center_size(arrow_rect.center(), vec2(size::ICON, size::ICON));
            icons::paint(
                painter,
                chevron,
                Glyph::ChevronDown,
                State::of(&arrow).icon(Tone::Icon, &colors),
            );
        }
        ToolbarSplitResponse { main, arrow }
    }
}
