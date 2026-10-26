//! List and tree rows.

use crate::behaviour::{Behaviour, Behaviours};
use crate::common;
use crate::icons::Icon;
use crate::ide::HelpTooltip;
use crate::parts::{
    IconPainter, SimpleColoredComponent, SimpleTextAttributes, Slot, State, TextFragment, Tone,
};
use crate::theme::Theme;
use crate::tokens::{opacity, size};
use crate::ui::SpeedSearchComparator;
use egui::{
    Color32, CornerRadius, Painter, Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2,
};

/// What happened to a [`ColoredListCellRenderer`] this frame.
pub struct CellRendererResponse {
    /// The row's response: `clicked()`, `double_clicked()`, `secondary_clicked()` for a
    /// context menu, `context_menu(...)`.
    pub response: Response,
    /// The expander chevron was clicked (or the row was double-clicked, or Left/Right was
    /// pressed while it had focus); the caller flips the node's expanded state.
    pub toggled: bool,
}

/// A custom icon painter: see [`ColoredListCellRenderer::icon_painter`].
type IconPaint<'a> = Box<dyn Fn(&Painter, Rect, Color32) + 'a>;

/// One row of a list or tree: chevron, leading slot, icon, label, secondary text and a
/// trailing slot.
/// The whole row is the click target; selection and hover draw a rounded pill.
///
/// Rows are stateless: the caller owns selection and expansion, so the same row works for
/// a flat list, a lazy tree or a virtualized table.
///
/// The [`trailing`](Self::trailing) slot takes a count, a status, or actions such as a
/// remove button ([`trailing_on_hover`](Self::trailing_on_hover) shows them only while the
/// row is hovered or selected); the [`leading`](Self::leading) slot sits before the icon (a
/// checkbox). A label longer than the room is elided with `…` and shown in full as the
/// tooltip. Tooltips, context menus and drag and drop are added with
/// [`with`](Self::with).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::{Glyph, Icon, ui::ColoredListCellRenderer};
///
/// let mut expanded = true;
/// let mut selected = 1;
/// let row = ColoredListCellRenderer::new("src")
///     .icon(Glyph::Folder)
///     .expanded(Some(expanded))
///     .selected(selected == 0)
///     .show(ui);
/// if row.toggled { expanded = !expanded; }
/// if row.response.clicked() { selected = 0; }
///
/// if expanded {
///     let row = ColoredListCellRenderer::new("main.rs")
///         .icon(Icon::for_file("main.rs"))
///         .depth(1)
///         .secondary("12 KB")
///         .selected(selected == 1)
///         .show(ui);
///     if row.response.clicked() { selected = 1; }
/// }
/// # });
/// ```
pub struct ColoredListCellRenderer<'a> {
    label: String,
    secondary: Option<String>,
    icon: Option<Icon>,
    icon_tone: Option<Tone>,
    label_tone: Tone,
    strong: bool,
    tint: Option<Tone>,
    icon_painter: Option<IconPaint<'a>>,
    leading: Option<Slot<'a>>,
    depth: usize,
    expanded: Option<bool>,
    flat: bool,
    selected: bool,
    enabled: bool,
    focusable: bool,
    inactive: bool,
    trailing: Option<Slot<'a>>,
    trailing_on_hover: bool,
    behaviours: Behaviours<'a>,
    fragments: Vec<TextFragment>,
    search: Option<&'a SpeedSearchComparator>,
    joined_above: bool,
    joined_below: bool,
    lead: bool,
}

impl<'a> ColoredListCellRenderer<'a> {
    /// A row showing `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            secondary: None,
            icon: None,
            icon_tone: None,
            label_tone: Tone::Default,
            strong: false,
            tint: None,
            icon_painter: None,
            leading: None,
            depth: 0,
            expanded: None,
            flat: false,
            selected: false,
            enabled: true,
            focusable: true,
            inactive: false,
            trailing: None,
            trailing_on_hover: false,
            behaviours: Behaviours::default(),
            fragments: Vec::new(),
            search: None,
            joined_above: false,
            joined_below: false,
            lead: false,
        }
    }

    /// Adds a fragment of text after the label with its own [`SimpleTextAttributes`]: regular,
    /// bold, grayed, link, error … (`SimpleColoredComponent.append`). Fragments are
    /// searched and highlighted by a speed search together with the label.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use archipel_kit::parts::SimpleTextAttributes;
    /// use archipel_kit::ui::ColoredListCellRenderer;
    ///
    /// ColoredListCellRenderer::new("main.rs")
    ///     .append(" src/bin", SimpleTextAttributes::GRAYED_ATTRIBUTES)
    ///     .append(" 2 errors", SimpleTextAttributes::ERROR_ATTRIBUTES)
    ///     .show(ui);
    /// # });
    /// ```
    pub fn append(mut self, text: impl Into<String>, attributes: SimpleTextAttributes) -> Self {
        self.fragments.push(TextFragment::new(text, attributes));
        self
    }

    /// The label, as the text a speed search matches against together with the fragments.
    pub fn search_text(&self) -> String {
        let mut text = self.label.clone();
        for fragment in &self.fragments {
            text.push_str(&fragment.text);
        }
        text
    }

    /// Highlights what `search` matches in the label and the fragments.
    pub(crate) fn search(mut self, search: Option<&'a SpeedSearchComparator>) -> Self {
        self.search = search;
        self
    }

    /// Squares the corners of the selection pill that touch a selected neighbor row.
    pub(crate) fn joined(mut self, above: bool, below: bool) -> Self {
        self.joined_above = above;
        self.joined_below = below;
        self
    }

    /// The keyboard's lead row: outlined when it is not selected.
    pub(crate) fn lead(mut self, lead: bool) -> Self {
        self.lead = lead;
        self
    }

    /// Content at the right end of the row: a count, a status, an icon, or widgets such as
    /// action buttons.
    pub fn trailing(mut self, slot: Slot<'a>) -> Self {
        self.trailing = Some(slot);
        self
    }

    /// Shows the trailing slot only while the row is hovered, focused or selected (row
    /// actions that would clutter a long list).
    pub fn trailing_on_hover(mut self, on_hover: bool) -> Self {
        self.trailing_on_hover = on_hover;
        self
    }

    /// Adds `behaviour` (a tooltip, context menu, drag source …) to the row's response.
    pub fn with(mut self, behaviour: impl Behaviour + 'a) -> Self {
        self.behaviours.push(behaviour);
        self
    }

    /// Dimmed text after the label: a path, a size, a type.
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Icon before the label.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Meaning of the icon's color, e.g. [`Tone::Run`] (default: the icon gray).
    pub fn icon_tone(mut self, tone: Tone) -> Self {
        self.icon_tone = Some(tone);
        self
    }

    /// Meaning of the label's color, e.g. [`Tone::Modified`] for a changed file.
    pub fn label_tone(mut self, tone: Tone) -> Self {
        self.label_tone = tone;
        self
    }

    /// A semibold label: group titles, the running configuration.
    pub fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// Tints the row's background by meaning, e.g. [`Tone::Warning`] for a search hit or
    /// [`Tone::Success`] for a running configuration. The tint stays visible on a selected or hovered row.
    pub fn tint(mut self, tone: Tone) -> Self {
        self.tint = Some(tone);
        self
    }

    /// Paints the icon yourself, for icons that are not data in the kit (a file icon with a
    /// badge): `paint(painter, rect, color)` gets the 16×16 icon rect and the color of the
    /// icon tone in the row's state (dimmed when disabled). Replaces [`icon`](Self::icon).
    pub fn icon_painter(mut self, paint: impl Fn(&Painter, Rect, Color32) + 'a) -> Self {
        self.icon_painter = Some(Box::new(paint));
        self
    }

    /// Content before the icon, after the chevron: a checkbox. Give widgets room with
    /// [`Slot::ui_width`].
    pub fn leading(mut self, slot: Slot<'a>) -> Self {
        self.leading = Some(slot);
        self
    }

    /// Tree depth: indents the row by one level per step.
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// `Some(expanded)` draws an expander chevron (a tree node with children); `None`
    /// reserves its space so leaves align with their siblings.
    pub fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }

    /// A row of a flat list (popup lists, results): no expander column, so the text
    /// starts at the content inset.
    pub fn flat(mut self, flat: bool) -> Self {
        self.flat = flat;
        self
    }

    /// Highlights the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disabled rows are dimmed and ignore clicks.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// `Some(expanded)` if this row is a tree node.
    pub(crate) fn is_expanded(&self) -> Option<bool> {
        self.expanded
    }

    /// Rows inside a list widget are not focusable themselves: the list takes focus and
    /// moves the selection with the arrow keys.
    pub(crate) fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// A selection in a list without focus: the quieter inactive selection color.
    pub(crate) fn inactive(mut self, inactive: bool) -> Self {
        self.inactive = inactive;
        self
    }

    /// The copy of the row that follows the pointer while it is dragged: its icon and label
    /// on a popup card, as wide as the row.
    fn drag_image(&self, ui: &mut Ui, rect: Rect) {
        let ctx = &ui.ctx().clone();
        let (Some(pointer), Some(press)) =
            ctx.input(|i| (i.pointer.latest_pos(), i.pointer.press_origin()))
        else {
            return;
        };
        // Laid out over the row and then only its paint moved to the pointer, as egui does:
        // its hit areas stay on the row, so the targets under the pointer still see it.
        let id = ui.id().with("drag-image");
        let layer = egui::LayerId::new(egui::Order::Tooltip, id);
        // A Ui of its own, not a child: a child would take an auto id from the row's Ui and
        // shift the ids of everything after it (the drop targets) while the drag lasts.
        let mut card = Ui::new(
            ctx.clone(),
            id,
            egui::UiBuilder::new().layer_id(layer).max_rect(rect),
        );
        card.set_opacity(opacity::DRAG_IMAGE);
        crate::parts::Surface::Popup
            .frame(ctx)
            .inner_margin(0)
            .show(&mut card, |ui| {
                let (row, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                let content = SimpleColoredComponent {
                    icon: self.icon,
                    icon_tone: self.icon_tone,
                    icon_painter: self.icon_painter.as_deref().map(IconPainter),
                    label: &self.label,
                    label_tone: self.label_tone,
                    strong: self.strong,
                    secondary: self.secondary.as_deref(),
                    ..SimpleColoredComponent::default()
                };
                let pill = row.shrink2(vec2(crate::tokens::metrics(ui.ctx()).row_inset, 0.0));
                // Enabled, unselected: the plain row.
                let state = State {
                    enabled: true,
                    ..State::default()
                };
                content.paint(
                    ui,
                    pill,
                    pill.left() + crate::tokens::metrics(ui.ctx()).popup_item_padding,
                    state,
                );
            });
        // Held where it was grabbed.
        ctx.transform_layer_shapes(
            layer,
            egui::emath::TSTransform::from_translation(pointer - press),
        );
    }

    /// Draws the row across the available width.
    pub fn show(self, ui: &mut Ui) -> CellRendererResponse {
        let colors = Theme::colors(ui.ctx());
        let metrics = crate::tokens::metrics(ui.ctx());
        // Behaviours that need more sense (a drag source) are part of the one registration, so
        // the row keeps its place in the Tab order.
        let sense = if self.enabled && self.focusable {
            Sense::click() | self.behaviours.sense()
        } else if self.enabled {
            Sense::CLICK | self.behaviours.sense()
        } else {
            Sense::hover()
        };
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), metrics.row_height), sense);
        crate::cursor::on_hover(&response, egui::CursorIcon::Default);
        response.widget_info(|| {
            let mut info = WidgetInfo::selected(
                WidgetType::SelectableLabel,
                self.enabled,
                self.selected,
                self.search_text(),
            );
            if let Some(secondary) = &self.secondary {
                info.label = Some(format!("{} {secondary}", self.search_text()));
            }
            info
        });

        let search_text = self.search_text();
        let mut toggled = false;
        let mut elided = false;
        // Measured on Android Studio: the pill is inset 12 pt (popup lists: as menu items);
        // the expander column starts at the pill's edge and moves 19 pt per level.
        // Classic: the selection is the whole row.
        let pill = if self.flat {
            rect.shrink2(vec2(metrics.popup_item_inset, 0.0))
        } else {
            rect.shrink2(vec2(metrics.row_inset, 0.0))
        };
        let chevron = Rect::from_min_size(
            pos2(
                if self.flat {
                    pill.left() + metrics.popup_item_padding
                } else {
                    rect.left() + metrics.indent(self.depth)
                },
                rect.center().y - size::CHEVRON_COLUMN / 2.0,
            ),
            vec2(
                if self.flat { 0.0 } else { size::CHEVRON_COLUMN },
                size::CHEVRON_COLUMN,
            ),
        );
        if let Some(expanded) = self.expanded.filter(|_| !self.flat) {
            let chevron_response =
                ui.interact(chevron.expand(2.0), response.id.with("chevron"), sense);
            let action = if expanded { "Collapse" } else { "Expand" };
            chevron_response.widget_info(|| {
                WidgetInfo::labeled(
                    WidgetType::Button,
                    self.enabled,
                    format!("{action} {}", self.label),
                )
            });
            toggled = chevron_response.clicked() || response.double_clicked();
            if response.has_focus() {
                let key = if expanded {
                    egui::Key::ArrowLeft
                } else {
                    egui::Key::ArrowRight
                };
                toggled |= ui.input(|input| input.key_pressed(key));
            }
        }

        let state = State::of(&response)
            .selected(self.selected)
            .inactive(self.inactive);
        // Dragged by a drag behaviour: the row stays in place, dimmed, and a card with its
        // icon and label follows the pointer, held where it was grabbed.
        let dragged = self.behaviours.sense().senses_drag()
            && response.dragged()
            && egui::DragAndDrop::has_any_payload(ui.ctx());
        let opacity = ui.opacity();
        if dragged {
            // The card first, at full strength; then the row in place, dimmed.
            self.drag_image(ui, rect);
            ui.multiply_opacity(opacity::DRAG_SOURCE);
        }
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            // A popup list highlights the row under the pointer, like a menu; island lists
            // and trees have no hover fill.
            let fill = if self.flat && state.enabled && state.hovered && !state.selected {
                colors.selection
            } else {
                state.row_fill(&colors)
            };
            // Neighbors that are selected too join into one block: only its outer corners
            // are round.
            let (top, bottom) = (
                if self.joined_above {
                    0
                } else {
                    metrics.row_radius
                },
                if self.joined_below {
                    0
                } else {
                    metrics.row_radius
                },
            );
            let corner = CornerRadius {
                nw: top,
                ne: top,
                sw: bottom,
                se: bottom,
            };
            if fill != Color32::TRANSPARENT {
                painter.rect_filled(pill, corner, fill);
            }
            // Over the selection fill, so a tinted row keeps its meaning selected.
            if let Some(tone) = self.tint {
                painter.rect_filled(
                    pill,
                    corner,
                    tone.color(&colors).gamma_multiply(opacity::ROW_TINT),
                );
            }
            if (response.has_focus() || self.lead) && !self.selected {
                common::focus_ring(painter, pill.shrink(1.0), metrics.row_radius, &colors);
            }
            if let Some(expanded) = self.expanded {
                crate::parts::paint_expander(painter, chevron, expanded, colors.text_secondary);
            }
            let mut x = if self.flat {
                chevron.right()
            } else {
                chevron.right() + size::CHEVRON_GAP
            };
            if let Some(slot) = self.leading {
                let width = slot.width(ui);
                let slot_rect =
                    Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height()));
                slot.show(ui, slot_rect, state);
                x += width + size::ROW_LEADING_GAP;
            }
            let reserve_right = self
                .trailing
                .as_ref()
                .map_or(0.0, |slot| slot.width(ui) + size::ROW_TRAILING_GAP);
            let matches = self
                .search
                .and_then(|search| search.matching_fragments(&search_text))
                .unwrap_or_default();
            let content = SimpleColoredComponent {
                icon: self.icon,
                icon_tone: self.icon_tone,
                icon_painter: self.icon_painter.as_deref().map(IconPainter),
                label: &self.label,
                label_tone: self.label_tone,
                strong: self.strong,
                secondary: self.secondary.as_deref(),
                reserve_right,
                fragments: &self.fragments,
                matches: &matches,
                ..SimpleColoredComponent::default()
            };
            elided = content.paint(ui, pill, x, state);
        }
        if let Some(slot) = self.trailing {
            let reveal = !self.trailing_on_hover
                || state.selected
                || state.focused
                || ui.rect_contains_pointer(rect);
            if reveal && ui.is_rect_visible(rect) {
                let inset = rect.right() - pill.right() + 4.0;
                let width = slot.width(ui);
                let slot_rect = Rect::from_min_max(
                    pos2(rect.right() - inset - width, rect.top()),
                    pos2(rect.right() - inset, rect.bottom()),
                );
                slot.show(ui, slot_rect, state);
            }
        }
        ui.set_opacity(opacity);
        // The full label, unless the caller brought a tooltip of their own.
        if elided && !self.behaviours.has_tooltip() {
            HelpTooltip::new(self.label.clone()).show_for(&response);
        }
        let response = self.behaviours.attach(ui, response);
        CellRendererResponse { response, toggled }
    }
}
