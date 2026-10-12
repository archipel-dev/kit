//! Drop zones: areas that take what a [`DragSource`](crate::behaviour::DragSource) carries.
//! A [`DropTarget`](crate::behaviour::DropTarget) makes an existing widget (a tree row) a
//! target; a [`DropZone`] is an area of its own with a hint (the Query, Projection and Sort
//! areas of a query builder).
//!
//! Dragging is a pointer gesture: an app that offers it also offers a keyboard way to the
//! same result (a double-click or Enter on the row, a menu item), as Android Studio does.

use super::common;
use crate::theme::Theme;
use crate::tokens::{radius, size, typography};
use egui::{
    Align2, CornerRadius, DragAndDrop, Id, InnerResponse, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, vec2,
};
use std::any::Any;
use std::sync::Arc;

/// A place that takes what a [`DragSource`](crate::behaviour::DragSource) of payload type
/// `P` carries: the Query,
/// Projection and Sort areas of a query builder, a folder in a tree.
///
/// While a matching payload is dragged it draws an accent outline, so the user sees where
/// it can go; under the pointer a 2 pt focus ring and a tint mark the target. Payloads of
/// other types leave it alone.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::behaviour::DragSource;
/// use archipel_kit::components::{DropZone, ListRow};
///
/// #[derive(Clone)]
/// struct Field(String);
///
/// ListRow::new("price").with(DragSource(Field("price".into()))).show(ui);
///
/// let mut filter: Vec<String> = Vec::new();
/// let out = DropZone::new("filter")
///     .hint("Drop fields here to filter")
///     .show::<Field, _>(ui, |ui| {
///         for name in &filter {
///             ui.label(name.as_str());
///         }
///     });
/// if let Some(field) = out.dropped {
///     filter.push(field.0.clone());
/// }
/// # });
/// ```
#[must_use = "show it with `zone.show::<Payload, _>(ui, |ui| …)`"]
pub struct DropZone {
    id_salt: Id,
    hint: Option<String>,
    show_hint: bool,
}

/// What a [`DropZone`] reports.
#[non_exhaustive]
pub struct DropZoneResponse<P, R> {
    /// What the content returned.
    pub inner: R,
    /// The zone's response, its rect and accessible node.
    pub response: Response,
    /// A matching payload is being dragged over the zone this frame.
    pub hovered: bool,
    /// The payload dropped on the zone this frame.
    pub dropped: Option<Arc<P>>,
}

impl DropZone {
    /// A zone unique by `id_salt` within its `Ui`.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            hint: None,
            show_hint: true,
        }
    }

    /// The gray text in the zone while it shows no content of its own ("Drop fields here
    /// to filter"); also its accessible name.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Whether to show the hint (default true). Turn it off once the zone holds content.
    pub fn show_hint(mut self, show: bool) -> Self {
        self.show_hint = show;
        self
    }

    /// Lays out the zone around what `add` draws and reports a drop of payload type `P`.
    pub fn show<P: Any + Send + Sync, R>(
        self,
        ui: &mut Ui,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> DropZoneResponse<P, R> {
        let ctx = ui.ctx().clone();
        let colors = Theme::colors(&ctx);
        let id = ui.id().with(self.id_salt);
        let background = ui.painter().add(egui::Shape::Noop);
        let width = ui.available_width();
        let InnerResponse { inner, response } = ui.scope(|ui| {
            ui.set_min_size(vec2(width, size::DROP_ZONE_MIN_HEIGHT));
            let mut content = ui.new_child(
                UiBuilder::new()
                    .max_rect(ui.max_rect().shrink(size::DROP_ZONE_PADDING))
                    .layout(*ui.layout()),
            );
            let inner = add(&mut content);
            let used = content.min_rect().expand(size::DROP_ZONE_PADDING);
            ui.allocate_rect(used, Sense::hover());
            inner
        });
        let rect = response.rect;
        let response = ui.interact(rect, id, Sense::hover());
        let accepting = DragAndDrop::has_payload_of_type::<P>(&ctx);
        // `hovered()` is false for everything while something is dragged; the pointer's
        // latest position decides.
        let hovered = accepting
            && ctx
                .input(|i| i.pointer.latest_pos())
                .is_some_and(|pos| rect.contains(pos) && ui.clip_rect().contains(pos));
        let released = ctx.input(|i| i.pointer.any_released());
        let dropped = if hovered && released {
            DragAndDrop::take_payload::<P>(&ctx)
        } else {
            None
        };

        let label = self
            .hint
            .clone()
            .unwrap_or_else(|| "Drop target".to_owned());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &label));

        // Under a dragged item the outline says enough; the hint would show through its card.
        if self.show_hint
            && !hovered
            && let Some(hint) = &self.hint
        {
            let galley = common::galley(ui, hint.as_str(), typography::body(typography::BODY));
            let pos = Align2::CENTER_CENTER
                .align_size_within_rect(galley.size(), rect)
                .min;
            common::text(ui.painter(), pos, galley, colors.text_placeholder);
        }
        let (stroke, fill) = if hovered {
            (
                Stroke::new(size::FOCUS_RING, colors.focus_ring),
                colors.selection_inactive,
            )
        } else if accepting {
            (
                Stroke::new(1.0_f32, colors.focus_ring),
                egui::Color32::TRANSPARENT,
            )
        } else {
            (
                Stroke::new(1.0_f32, colors.border),
                egui::Color32::TRANSPARENT,
            )
        };
        ui.painter().set(
            background,
            egui::epaint::RectShape::new(
                rect,
                CornerRadius::same(radius::CONTROL),
                fill,
                stroke,
                StrokeKind::Inside,
            ),
        );
        DropZoneResponse {
            inner,
            response,
            hovered,
            dropped,
        }
    }
}
