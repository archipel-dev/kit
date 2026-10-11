//! Containers: islands (panels) with a header, section headers and separators.

use super::{IconButton, Tooltip, common};
use crate::behaviour::WidgetExt;
use crate::icons::Glyph;
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{CornerRadius, InnerResponse, Sense, Stroke, Ui, Widget, vec2};

/// An island: a rounded panel with a title bar, the building block of a workbench layout
/// (tool windows, sidebars, inspectors).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, components::{IconButton, Panel}};
///
/// Panel::new("Project")
///     .actions(|ui| {
///         ui.add(IconButton::new(Glyph::CollapseAll, "Collapse all"));
///     })
///     .show(ui, |ui| {
///         ui.label("src/");
///     });
/// # });
/// ```
pub struct Panel<'a> {
    title: String,
    actions: Option<Actions<'a>>,
    on_hide: Option<Box<dyn FnOnce() + 'a>>,
    header: Option<(f32, Actions<'a>)>,
    hover_actions: bool,
    padded: bool,
    header_separator: bool,
}

type Actions<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl<'a> Panel<'a> {
    /// A panel titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            actions: None,
            on_hide: None,
            header: None,
            hover_actions: false,
            padded: false,
            header_separator: false,
        }
    }

    /// Toolbar buttons on the right of the title bar (usually [`super::IconButton`]s),
    /// laid out right to left: the first one added is the rightmost.
    pub fn actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    /// Replace the title's painted content with a bounded, left-aligned header slot.
    /// The title remains the panel's identity; give interactive header widgets names.
    pub fn header(mut self, width: f32, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.header = Some((
            if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            },
            Box::new(content),
        ));
        self
    }

    /// Fade header actions on island hover or keyboard focus (default: always visible).
    /// Their space stays reserved and Tab can reach them while visually hidden.
    pub fn hover_actions(mut self, enabled: bool) -> Self {
        self.hover_actions = enabled;
        self
    }

    /// Apply the common content inset to the body (default: edge-to-edge for trees).
    pub fn padded(mut self, enabled: bool) -> Self {
        self.padded = enabled;
        self
    }

    /// Draw an inset horizontal hairline below the header.
    pub fn header_separator(mut self, enabled: bool) -> Self {
        self.header_separator = enabled;
        self
    }

    /// Adds the tool window's hide button (a minimize icon, named "Hide <title>") as the
    /// rightmost button of the title bar; `hide` runs when it is clicked. With
    /// [`actions`](Self::actions) the panel is an IntelliJ-style tool window header: title,
    /// action buttons, hide.
    pub fn on_hide(mut self, hide: impl FnOnce() + 'a) -> Self {
        self.on_hide = Some(Box::new(hide));
        self
    }

    /// Draws the panel with `body` below the title bar.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let colors = Theme::colors(ui.ctx());
        let panel_id = ui.make_persistent_id(("island", &self.title, ui.next_auto_id()));
        let previous = ui.ctx().data(|d| d.get_temp::<egui::Rect>(panel_id));
        let hovered = previous.is_some_and(|r| ui.rect_contains_pointer(r));
        let focused = ui
            .memory(|m| m.focused())
            .and_then(|id| ui.ctx().read_response(id))
            .is_some_and(|r| previous.is_some_and(|p| p.contains_rect(r.rect)));
        let opacity = if self.hover_actions {
            ui.ctx().animate_bool_with_time(
                panel_id.with("actions"),
                hovered || focused,
                size::ISLAND_ACTION_FADE,
            )
        } else {
            1.0
        };
        let result = egui::Frame::new()
            .fill(colors.island)
            .corner_radius(CornerRadius::same(radius::ISLAND))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let (header, _) = ui.allocate_exact_size(
                    vec2(ui.available_width(), size::HEADER_HEIGHT),
                    Sense::hover(),
                );
                let mut content_right = header.right() - size::CONTENT_INSET;
                if self.actions.is_some() || self.on_hide.is_some() {
                    let mut toolbar = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(header.shrink2(vec2(size::CONTENT_INSET - 6.0, 2.0)))
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    toolbar.set_clip_rect(header.intersect(ui.clip_rect()));
                    toolbar.multiply_opacity(opacity);
                    toolbar.spacing_mut().item_spacing.x = if self.hover_actions {
                        size::TOOLBAR_GAP
                    } else {
                        2.0
                    };
                    if let Some(hide) = self.on_hide {
                        let button =
                            IconButton::new(Glyph::Minimize, format!("Hide {}", self.title))
                                .with(Tooltip::new("Hide"));
                        if toolbar.add(button).clicked() {
                            hide();
                        }
                    }
                    if let Some(actions) = self.actions {
                        actions(&mut toolbar);
                    }
                    content_right = toolbar.min_rect().left() - size::TOOLBAR_GAP;
                }
                let left = (header.left() + size::CONTENT_INSET).min(header.right());
                content_right = content_right.max(left).min(header.right());
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(left, header.top()),
                    egui::pos2(content_right, header.bottom()),
                );
                if let Some((width, content)) = self.header {
                    let mut slot = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(egui::Rect::from_min_size(
                                content_rect.min,
                                vec2(width.min(content_rect.width()), content_rect.height()),
                            ))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    slot.set_clip_rect(slot.max_rect().intersect(ui.clip_rect()));
                    content(&mut slot);
                } else {
                    let galley = common::galley(
                        ui,
                        self.title.as_str(),
                        typography::semibold(ui.ctx(), typography::TITLE),
                    );
                    common::text_left(
                        &ui.painter()
                            .with_clip_rect(content_rect.intersect(ui.clip_rect())),
                        header,
                        left,
                        galley,
                        colors.text_strong,
                    );
                }
                if self.header_separator {
                    ui.painter().hline(
                        (header.left() + size::CONTENT_INSET)
                            ..=(header.right() - size::CONTENT_INSET)
                                .max(header.left() + size::CONTENT_INSET),
                        header.bottom(),
                        Stroke::new(1.0_f32, colors.border),
                    );
                }
                ui.spacing_mut().item_spacing.y = ui.ctx().style().spacing.item_spacing.y;
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: if self.padded {
                            size::CONTENT_INSET as i8
                        } else {
                            0
                        },
                        right: if self.padded {
                            size::CONTENT_INSET as i8
                        } else {
                            0
                        },
                        top: if self.padded {
                            size::SECTION_SPACING as i8
                        } else {
                            0
                        },
                        bottom: size::SECTION_SPACING as i8,
                    })
                    .show(ui, body)
                    .inner
            });
        ui.ctx()
            .data_mut(|d| d.insert_temp(panel_id, result.response.rect));
        result
    }
}

/// A section title with a hairline, separating groups inside a panel or
/// a settings page.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::components::SectionHeader;
///
/// ui.add(SectionHeader::new("Code style"));
/// # });
/// ```
#[must_use = "add it with `ui.add(header)`"]
pub struct SectionHeader {
    title: String,
}

impl SectionHeader {
    /// A header titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl Widget for SectionHeader {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let colors = Theme::colors(ui.ctx());
        let galley = common::galley(
            ui,
            self.title.as_str(),
            typography::semibold(ui.ctx(), typography::BODY),
        );
        // Space above separates the section from the previous one.
        let (outer, response) = ui.allocate_exact_size(
            vec2(
                ui.available_width(),
                size::ROW_HEIGHT + size::SECTION_SPACING,
            ),
            Sense::hover(),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(outer.left(), outer.bottom() - size::ROW_HEIGHT),
            outer.max,
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.title));
        let text_right = rect.left() + galley.size().x;
        common::text_left(
            ui.painter(),
            rect,
            rect.left(),
            galley,
            colors.text_secondary,
        );
        let y = rect.center().y.round() + 0.5;
        if text_right + 8.0 < rect.right() {
            ui.painter().hline(
                text_right + 8.0..=rect.right(),
                y,
                Stroke::new(1.0_f32, colors.separator),
            );
        }
        response
    }
}

/// A 1 px separator line across the available width (or height in a horizontal layout).
#[must_use = "add it with `ui.add(separator)`"]
#[derive(Default)]
pub struct Separator;

impl Separator {
    /// A separator with an explicit line length in points, centered in its layout.
    /// Useful for action groups inside panel headers. Invalid lengths become zero.
    pub fn length(points: f32) -> impl Widget {
        SizedSeparator(if points.is_finite() {
            points.max(0.0)
        } else {
            0.0
        })
    }
}

struct SizedSeparator(f32);
impl Widget for SizedSeparator {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let vertical = ui.layout().main_dir().is_horizontal();
        let length = self
            .0
            .min(if vertical {
                ui.available_height()
            } else {
                ui.available_width()
            })
            .max(0.0);
        let (rect, response) = ui.allocate_exact_size(
            if vertical {
                vec2(9.0, length)
            } else {
                vec2(length, 9.0)
            },
            Sense::hover(),
        );
        let stroke = Stroke::new(1.0_f32, Theme::colors(ui.ctx()).separator);
        if vertical {
            ui.painter()
                .vline(rect.center().x.round() + 0.5, rect.y_range(), stroke);
        } else {
            ui.painter()
                .hline(rect.x_range(), rect.center().y.round() + 0.5, stroke);
        }
        response
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let colors = Theme::colors(ui.ctx());
        let horizontal = ui.layout().main_dir().is_horizontal();
        let size = if horizontal {
            vec2(9.0, ui.available_height().min(size::ICON_BUTTON))
        } else {
            vec2(ui.available_width(), 9.0)
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let stroke = Stroke::new(1.0_f32, colors.separator);
        if horizontal {
            ui.painter()
                .vline(rect.center().x.round() + 0.5, rect.y_range(), stroke);
        } else {
            ui.painter()
                .hline(rect.x_range(), rect.center().y.round() + 0.5, stroke);
        }
        response
    }
}
