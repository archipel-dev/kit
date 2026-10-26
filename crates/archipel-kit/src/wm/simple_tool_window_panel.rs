//! The tool window panel: an island with a header.

use crate::action_system::ActionButton;
use crate::behaviour::WidgetExt;
use crate::common;
use crate::icons::Glyph;
use crate::ide::HelpTooltip;
use crate::theme::Theme;
use crate::tokens::{motion, radius, size, tab, typography};
use egui::{CornerRadius, InnerResponse, Sense, Stroke, Ui, vec2};

/// An island: a rounded panel with a title bar, the building block of a workbench layout
/// (tool windows, sidebars, inspectors).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, action_system::ActionButton, wm::SimpleToolWindowPanel};
///
/// SimpleToolWindowPanel::new("Project")
///     .actions(|ui| {
///         ui.add(ActionButton::new(Glyph::CollapseAll, "Collapse all"));
///     })
///     .show(ui, |ui| {
///         ui.label("src/");
///     });
/// # });
/// ```
pub struct SimpleToolWindowPanel<'a> {
    title: String,
    actions: Option<Actions<'a>>,
    on_hide: Option<Box<dyn FnOnce() + 'a>>,
    header: Option<(f32, Actions<'a>)>,
    hover_actions: bool,
    padded: bool,
    header_separator: bool,
    tabs: Option<(&'a [String], &'a mut usize)>,
    views: Option<(&'a [String], &'a mut usize)>,
}

type Actions<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl<'a> SimpleToolWindowPanel<'a> {
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
            tabs: None,
            views: None,
        }
    }

    /// Makes the title a view switcher, like Android Studio's "Project ▾": the title reads the
    /// selected view with a chevron after it; a click (or <kbd>Enter</kbd>) opens a list of
    /// the views right below the header, the current one highlighted, and picking one
    /// selects it. The caller owns the index.
    pub fn views(mut self, views: &'a [String], selected: &'a mut usize) -> Self {
        self.views = Some((views, selected));
        self
    }

    /// Content tabs after the title, like Android Studio's Problems tool window ("File",
    /// "Project Errors"): the caller owns the selected index, a click selects. The selected
    /// tab is blue while the tool window has the focus and outlined gray otherwise; others
    /// show a hover fill. Adds the header hairline.
    pub fn tabs(mut self, tabs: &'a [String], selected: &'a mut usize) -> Self {
        self.tabs = Some((tabs, selected));
        self.header_separator = true;
        self
    }

    /// Toolbar buttons on the right of the title bar (usually [`crate::action_system::ActionButton`]s),
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

    /// Draw a hairline below the header, edge to edge in the frame color (Android Studio's
    /// tool windows with content tabs, like Problems).
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
            actions_opacity(ui.ctx(), panel_id.with("actions"), hovered || focused)
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
                            .max_rect(header.shrink2(vec2(size::HEADER_ACTIONS_INSET, 0.0)))
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    toolbar.set_clip_rect(header.intersect(ui.clip_rect()));
                    toolbar.multiply_opacity(opacity);
                    toolbar.spacing_mut().item_spacing.x = size::TOOLBAR_GAP;
                    if let Some(hide) = self.on_hide {
                        let button =
                            ActionButton::new(Glyph::Minimize, format!("Hide {}", self.title))
                                .with(HelpTooltip::new("Hide"));
                        if toolbar.add(button).clicked() {
                            hide();
                        }
                    }
                    if let Some(actions) = self.actions {
                        actions(&mut toolbar);
                    }
                    // Hidden actions leave the whole header to the title.
                    if opacity > 0.0 {
                        content_right = toolbar.min_rect().left() - size::TOOLBAR_GAP;
                    }
                }
                // Shown actions win over the title: as in the Islands UI, a title that no
                // longer fits beside them is hidden whole rather than cut, and the actions,
                // anchored right, are clipped at the island's left edge.
                let actions_shown =
                    opacity > 0.0 && content_right < header.right() - size::CONTENT_INSET;
                let left = (header.left() + size::CONTENT_INSET).min(header.right());
                content_right = content_right.max(left).min(header.right());
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(left, header.top()),
                    egui::pos2(content_right, header.bottom()),
                );
                if let Some((width, content)) = self.header {
                    if actions_shown && width > content_rect.width() {
                        // Squeezed out by the actions; it comes back when they hide.
                    } else {
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
                    }
                } else {
                    let shown = self
                        .views
                        .as_ref()
                        .and_then(|(views, selected)| views.get(**selected))
                        .unwrap_or(&self.title)
                        .clone();
                    let galley = common::galley(
                        ui,
                        shown.as_str(),
                        typography::semibold(ui.ctx(), typography::TITLE),
                    );
                    let mut title_width = galley.size().x;
                    if let Some((views, selected)) = self.views {
                        title_width += size::TITLE_CHEVRON_GAP + size::ICON;
                        let title_rect = egui::Rect::from_min_size(
                            egui::pos2(left, header.top()),
                            vec2(title_width, header.height()),
                        )
                        .intersect(content_rect);
                        let switcher =
                            ui.interact(title_rect, panel_id.with("views"), Sense::click());
                        crate::cursor::on_hover_action(&switcher);
                        switcher.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &shown)
                        });
                        let chevron = egui::Rect::from_min_size(
                            egui::pos2(
                                left + galley.size().x + size::TITLE_CHEVRON_GAP,
                                header.center().y - size::ICON / 2.0,
                            ),
                            vec2(size::ICON, size::ICON),
                        );
                        crate::icons::paint(
                            &ui.painter().with_clip_rect(content_rect),
                            chevron,
                            Glyph::ChevronDown,
                            colors.text_secondary,
                        );
                        if switcher.has_focus() {
                            crate::parts::focus_ring(
                                ui.painter(),
                                title_rect,
                                radius::CONTROL,
                                &colors,
                            );
                        }
                        let open_id = panel_id.with("views-open");
                        let mut open = ui
                            .ctx()
                            .data(|d| d.get_temp::<bool>(open_id))
                            .unwrap_or(false);
                        if switcher.clicked() {
                            open = !open;
                        }
                        // The list opens right below the header, at the island's left edge.
                        let anchor = egui::Rect::from_min_max(
                            egui::pos2(header.left() + 2.0, title_rect.top()),
                            egui::pos2(
                                title_rect.right().max(header.left() + 3.0),
                                header.bottom() - size::POPUP_PADDING,
                            ),
                        );
                        let mut picked = None;
                        crate::ui::popup::JBPopup::new(panel_id.with("views-popup"), anchor)
                            .width(
                                views
                                    .iter()
                                    .map(|view| {
                                        common::galley(
                                            ui,
                                            view.as_str(),
                                            typography::body(typography::BODY),
                                        )
                                        .size()
                                        .x
                                    })
                                    .fold(0.0, f32::max)
                                    + 2.0
                                        * (crate::tokens::popup::ITEM_INSET
                                            + crate::tokens::popup::ITEM_PADDING),
                            )
                            .show(ui.ctx(), &mut open, |ui| {
                                for (index, view) in views.iter().enumerate() {
                                    if ui
                                        .add(
                                            crate::action_system::ActionMenuItem::new(
                                                view.as_str(),
                                            )
                                            .highlighted(index == *selected),
                                        )
                                        .clicked()
                                    {
                                        picked = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = picked {
                            *selected = index;
                            open = false;
                        }
                        ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
                    }
                    if !(actions_shown && title_width > content_rect.width()) {
                        common::text_left(
                            &ui.painter()
                                .with_clip_rect(content_rect.intersect(ui.clip_rect())),
                            header,
                            left,
                            galley,
                            colors.text_strong,
                        );
                    }
                    if let Some((tabs, selected)) = self.tabs {
                        let clip = content_rect.intersect(ui.clip_rect());
                        let start = left + title_width + tab::CONTENT_TITLE_GAP;
                        let measured: Vec<_> = tabs
                            .iter()
                            .map(|label| {
                                let galley = common::galley(
                                    ui,
                                    label.as_str(),
                                    typography::body(typography::BODY),
                                );
                                let width = galley.size().x + 2.0 * tab::CONTENT_PADDING;
                                (galley, width)
                            })
                            .collect();
                        let visible = visible_tabs(
                            &measured.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
                            *selected,
                            content_rect.right() - start,
                        );
                        let mut x = start;
                        for &index in &visible {
                            let (galley, width) = measured[index].clone();
                            let label = &tabs[index];
                            let rect = egui::Rect::from_center_size(
                                egui::pos2(x + width / 2.0, header.center().y),
                                vec2(width, tab::HEIGHT),
                            );
                            x += width + tab::CONTENT_GAP;
                            let response = ui.interact(
                                rect.intersect(clip),
                                panel_id.with(("content-tab", index)),
                                Sense::click(),
                            );
                            let is_selected = *selected == index;
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    true,
                                    is_selected,
                                    label,
                                )
                            });
                            if response.clicked() {
                                *selected = index;
                            }
                            let painter = ui.painter().with_clip_rect(clip);
                            let corner = CornerRadius::same(radius::TAB);
                            if is_selected {
                                // Blue while the tool window is active, gray otherwise.
                                let (fill, border) = if focused {
                                    (colors.tab_selected, colors.tab_selected_border)
                                } else {
                                    (colors.frame, colors.border)
                                };
                                painter.rect_filled(rect, corner, fill);
                                painter.rect_stroke(
                                    rect,
                                    corner,
                                    Stroke::new(1.0_f32, border),
                                    egui::StrokeKind::Inside,
                                );
                            } else if response.hovered() {
                                painter.rect_filled(rect, corner, colors.hover);
                            }
                            if response.has_focus() {
                                crate::parts::focus_ring(&painter, rect, radius::TAB, &colors);
                            }
                            common::text_left(
                                &painter,
                                rect,
                                rect.left() + tab::CONTENT_PADDING,
                                galley,
                                colors.text,
                            );
                        }
                        // Tabs that don't fit are listed behind a chevron after the last one,
                        // as in Android Studio.
                        if visible.len() < tabs.len() {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(x, header.center().y - size::ICON_BUTTON / 2.0),
                                vec2(size::ICON_BUTTON, size::ICON_BUTTON),
                            );
                            let more = ui.put(
                                rect,
                                crate::action_system::ActionButton::new(
                                    Glyph::ChevronDown,
                                    "More tabs",
                                )
                                .with(HelpTooltip::new("More tabs")),
                            );
                            let open_id = panel_id.with("more-tabs");
                            let mut open: bool =
                                ui.ctx().data(|d| d.get_temp(open_id)).unwrap_or(false);
                            if more.clicked() {
                                open = !open;
                            }
                            let mut picked = None;
                            crate::ui::popup::JBPopup::below(open_id.with("popup"), &more).show(
                                ui.ctx(),
                                &mut open,
                                |ui| {
                                    for (index, label) in tabs.iter().enumerate() {
                                        if visible.contains(&index) {
                                            continue;
                                        }
                                        if ui
                                            .add(crate::action_system::ActionMenuItem::new(
                                                label.as_str(),
                                            ))
                                            .clicked()
                                        {
                                            picked = Some(index);
                                        }
                                    }
                                },
                            );
                            if let Some(index) = picked {
                                *selected = index;
                                open = false;
                            }
                            ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
                        }
                    }
                }
                if self.header_separator {
                    // As in Android Studio: a 1 pt line in the frame color, from edge to edge,
                    // right below the 40 pt header.
                    ui.painter().hline(
                        header.x_range(),
                        header.bottom() + 0.5,
                        Stroke::new(1.0_f32, colors.frame),
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

/// Opacity of an island's hover actions: 1 at once while `revealed`, then after
/// [`motion::ISLAND_ACTIONS_HIDE_DELAY`] a short fade to 0 (as measured in Android Studio).
fn actions_opacity(ctx: &egui::Context, id: egui::Id, revealed: bool) -> f32 {
    let now = ctx.input(|i| i.time);
    if revealed {
        ctx.data_mut(|d| d.insert_temp(id, now));
        return 1.0;
    }
    let Some(last) = ctx.data(|d| d.get_temp::<f64>(id)) else {
        return 0.0;
    };
    let since = (now - last) as f32;
    let fade = (since - motion::ISLAND_ACTIONS_HIDE_DELAY) / motion::ISLAND_ACTIONS_FADE_OUT;
    if fade >= 1.0 {
        0.0
    } else if fade < 0.0 {
        ctx.request_repaint_after_secs(motion::ISLAND_ACTIONS_HIDE_DELAY - since);
        1.0
    } else {
        ctx.request_repaint();
        1.0 - fade
    }
}

/// The content tabs that fit in `room` (their `widths` are known), in order, with room for
/// the overflow chevron when some don't; the selected tab is always among them, in the
/// place of the last one that fits.
fn visible_tabs(widths: &[f32], selected: usize, room: f32) -> Vec<usize> {
    let total: f32 =
        widths.iter().sum::<f32>() + tab::CONTENT_GAP * widths.len().saturating_sub(1) as f32;
    if total <= room {
        return (0..widths.len()).collect();
    }
    let room = room - size::ICON_BUTTON - tab::CONTENT_GAP;
    let mut shown = Vec::new();
    let mut used = 0.0;
    for (index, width) in widths.iter().enumerate() {
        let next = used
            + width
            + if shown.is_empty() {
                0.0
            } else {
                tab::CONTENT_GAP
            };
        if next > room {
            break;
        }
        used = next;
        shown.push(index);
    }
    if !shown.contains(&selected) && selected < widths.len() {
        // Make room for the selected tab at the end.
        while !shown.is_empty() {
            let extra = widths[selected]
                + if shown.is_empty() {
                    0.0
                } else {
                    tab::CONTENT_GAP
                };
            if used + extra <= room {
                break;
            }
            if let Some(last) = shown.pop() {
                used -= widths[last]
                    + if shown.is_empty() {
                        0.0
                    } else {
                        tab::CONTENT_GAP
                    };
            }
        }
        shown.push(selected);
    }
    shown
}

#[cfg(test)]
mod visible_tab_tests {
    use super::*;

    #[test]
    fn all_fit_or_the_selected_one_stays() {
        let widths = [60.0, 60.0, 60.0, 60.0];
        assert_eq!(visible_tabs(&widths, 0, 1000.0), vec![0, 1, 2, 3]);
        let few = visible_tabs(&widths, 0, 200.0);
        assert!(few.len() < 4 && few[0] == 0);
        let with_selected = visible_tabs(&widths, 3, 200.0);
        assert_eq!(with_selected.last(), Some(&3));
        assert!(with_selected.len() < 4);
    }
}
