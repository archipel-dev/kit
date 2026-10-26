//! The breadcrumbs path.

use crate::common;
use crate::icons::{self, Glyph, Icon};
use crate::parts::{State, Tone};
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// A path of places, like Android Studio's navigation path at the left of the status bar
/// (`project › src › main.rs`) or a settings page's location. Each crumb is a flat button
/// with an optional icon; dimmed chevrons separate them. Text is dimmed at rest and full
/// strength under the pointer. The response says which crumb was clicked.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, Icon, ui::BreadcrumbsComponent};
///
/// let clicked = BreadcrumbsComponent::new()
///     .crumb("archipel", Some(Glyph::Folder.into()))
///     .crumb("main.rs", Some(Icon::for_file("main.rs")))
///     .show(ui)
///     .clicked;
/// # let _ = clicked;
/// # });
/// ```
#[must_use = "show it with `.show(ui)`"]
#[derive(Default)]
pub struct BreadcrumbsComponent {
    crumbs: Vec<(String, Option<Icon>)>,
}

impl BreadcrumbsComponent {
    /// An empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a crumb reading `label`, with an optional icon.
    pub fn crumb(mut self, label: impl Into<String>, icon: Option<Icon>) -> Self {
        self.crumbs.push((label.into(), icon));
        self
    }

    /// Draws the path in a row, [`size::STATUS_ITEM_HEIGHT`] high.
    pub fn show(self, ui: &mut Ui) -> BreadcrumbsComponentResponse {
        let colors = Theme::colors(ui.ctx());
        let mut clicked = None;
        let mut crumbs = Vec::with_capacity(self.crumbs.len());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::BREADCRUMB_GAP;
            let count = self.crumbs.len();
            for (index, (label, icon)) in self.crumbs.iter().enumerate() {
                let galley = common::galley(ui, label.as_str(), typography::body(typography::BODY));
                let icon_width = if icon.is_some() {
                    size::ICON + size::ICON_TEXT_GAP
                } else {
                    0.0
                };
                let width = 2.0 * size::BREADCRUMB_PADDING + icon_width + galley.size().x;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, size::STATUS_ITEM_HEIGHT), Sense::click());
                crate::cursor::on_hover_action(&response);
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
                if response.clicked() {
                    clicked = Some(index);
                }
                let state = State::of(&response);
                let enabled = response.enabled();
                if ui.is_rect_visible(rect) {
                    let painter = ui.painter();
                    let fill = state.flat_fill(&colors);
                    if fill != egui::Color32::TRANSPARENT {
                        painter.rect_filled(rect, CornerRadius::same(radius::CONTROL), fill);
                    }
                    if response.has_focus() {
                        common::focus_ring(painter, rect, radius::CONTROL, &colors);
                    }
                    let mut x = rect.left() + size::BREADCRUMB_PADDING;
                    if let Some(icon) = *icon {
                        let icon_rect = Rect::from_min_size(
                            pos2(x, rect.center().y - size::ICON / 2.0),
                            vec2(size::ICON, size::ICON),
                        );
                        icons::paint(painter, icon_rect, icon, state.icon(Tone::Icon, &colors));
                        x += icon_width;
                    }
                    let color = if !enabled {
                        colors.text_disabled
                    } else if state.hovered || state.pressed || state.focused {
                        colors.text
                    } else {
                        colors.text_secondary
                    };
                    common::text_left(painter, rect, x, galley, color);
                }
                crumbs.push(response);
                if index + 1 < count {
                    let (chevron, _) = ui.allocate_exact_size(
                        vec2(size::ICON, size::STATUS_ITEM_HEIGHT),
                        Sense::hover(),
                    );
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(chevron.center(), vec2(size::ICON, size::ICON)),
                        Glyph::ChevronRight,
                        colors.text_placeholder,
                    );
                }
            }
        });
        BreadcrumbsComponentResponse { clicked, crumbs }
    }
}

/// What [`BreadcrumbsComponent::show`] drew.
pub struct BreadcrumbsComponentResponse {
    /// The index of the crumb clicked this frame.
    pub clicked: Option<usize>,
    /// Each crumb's response, in order.
    pub crumbs: Vec<Response>,
}
