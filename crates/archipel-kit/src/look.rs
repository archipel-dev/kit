//! UI looks: the three IntelliJ UI generations the kit renders, switchable at runtime.
//!
//! A *look* changes geometry and structure, a [`Theme`](crate::Theme) changes colors; the two
//! are independent and both live in the [`egui::Context`] (no globals), so two windows can
//! show different looks at once:
//!
//! ```
//! use archipel_kit::{Theme, look::{self, UiLook}};
//!
//! # let ctx = egui::Context::default();
//! archipel_kit::install(&ctx, Theme::dark());
//! assert_eq!(look::look(&ctx), UiLook::Islands); // the default
//! look::set_look(&ctx, UiLook::Classic);
//! look::set_compact(&ctx, true); // ignored by Classic, like the IDE
//! assert_eq!(archipel_kit::tokens::metrics(&ctx).row_height, 20.0);
//! ```
//!
//! | Look | What it is |
//! |---|---|
//! | [`UiLook::Islands`] | The default: rounded islands on the window frame, square icon stripes, pill tabs. |
//! | [`UiLook::NewUi`] | New UI without islands: flush panels with 1 px separators, underline tabs, a status bar with a top line. |
//! | [`UiLook::Classic`] | The pre-2022 UI: flat full-row selection, text stripes around the tool windows, 3 px underline tabs with always-visible close buttons, a menu bar. |
//!
//! [`Metrics`] resolves every value that differs between looks (and compact mode) into one
//! struct: components read it with [`crate::tokens::metrics`] where a [`tokens`](crate::tokens)
//! constant would be the Islands value only. The `tokens::*` constants stay the Islands
//! (normal density) values.
//!
//! Themes *suggest* a look ([`Theme::suggested_look`](crate::Theme::suggested_look)); the caller
//! decides. See `docs/looks.md`.

use crate::tokens::{popup, radius, size, stripe, tab, toolbar};
use egui::Context;

/// Which IntelliJ UI generation components render as.
///
/// IntelliJ counterpart: `isNewUI()` plus the `Islands` theme key.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[non_exhaustive]
pub enum UiLook {
    /// New UI with islands (Android Studio's and IntelliJ's default).
    #[default]
    Islands,
    /// New UI without islands: flush panels separated by 1 px lines.
    NewUi,
    /// The classic (pre New UI) look.
    Classic,
}

impl UiLook {
    /// Every look, in picker order.
    pub const ALL: [UiLook; 3] = [UiLook::Islands, UiLook::NewUi, UiLook::Classic];

    /// Display name ("Islands", "New UI", "Classic").
    pub fn name(self) -> &'static str {
        match self {
            UiLook::Islands => "Islands",
            UiLook::NewUi => "New UI",
            UiLook::Classic => "Classic",
        }
    }

    /// Stable id for file names: `islands`, `new_ui`, `classic`.
    pub fn id(self) -> &'static str {
        match self {
            UiLook::Islands => "islands",
            UiLook::NewUi => "new_ui",
            UiLook::Classic => "classic",
        }
    }

    /// Whether panels are rounded islands on the frame (only [`UiLook::Islands`]).
    pub fn is_islands(self) -> bool {
        self == UiLook::Islands
    }

    /// Whether compact density exists in this look (not in [`UiLook::Classic`]).
    pub fn supports_compact(self) -> bool {
        self != UiLook::Classic
    }
}

/// Orthogonal view modes: runtime flags, not themes (IntelliJ `UISettings.presentationMode`,
/// the Distraction Free registry flag, full screen).
///
/// Components honor them through the `hides_*` methods and [`Metrics`]; the scale of
/// presentation mode is applied to the context by [`set_view_mode`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub struct ViewMode {
    /// Presentation mode: UI scale 1.75, hides stripes, status bar, navigation bar, editor
    /// tabs and tool windows.
    pub presentation: bool,
    /// Distraction Free mode: centered editor, hides toolbars, status bar, stripes, tabs
    /// and gutter extras; the window header shrinks to 30.
    pub distraction_free: bool,
    /// Full screen: changes the frame decoration only (the host's job); components do
    /// not change.
    pub full_screen: bool,
}

impl ViewMode {
    /// IDE scale of presentation mode (`presentationModeIdeScale`).
    pub const PRESENTATION_SCALE: f32 = 1.75;

    /// Normal mode: nothing set.
    pub const NORMAL: ViewMode = ViewMode {
        presentation: false,
        distraction_free: false,
        full_screen: false,
    };

    /// Presentation mode.
    pub const fn presentation() -> Self {
        Self {
            presentation: true,
            ..Self::NORMAL
        }
    }

    /// Distraction Free mode.
    pub const fn distraction_free() -> Self {
        Self {
            distraction_free: true,
            ..Self::NORMAL
        }
    }

    /// Zen mode: Distraction Free in full screen.
    pub const fn zen() -> Self {
        Self {
            presentation: false,
            distraction_free: true,
            full_screen: true,
        }
    }

    /// Whether this is Zen mode (`distraction_free && full_screen`).
    pub fn is_zen(self) -> bool {
        self.distraction_free && self.full_screen
    }

    /// UI scale this mode asks for: 1.75 in presentation mode, else 1.
    pub fn scale(self) -> f32 {
        if self.presentation {
            Self::PRESENTATION_SCALE
        } else {
            1.0
        }
    }

    /// Tool window stripes are hidden.
    pub fn hides_stripes(self) -> bool {
        self.presentation || self.distraction_free
    }

    /// The status bar is hidden.
    pub fn hides_status_bar(self) -> bool {
        self.presentation || self.distraction_free
    }

    /// The navigation bar (breadcrumb navbar) is hidden.
    pub fn hides_navbar(self) -> bool {
        self.presentation || self.distraction_free
    }

    /// Editor tabs are hidden (one editor, no strip).
    pub fn hides_tabs(self) -> bool {
        self.presentation || self.distraction_free
    }

    /// Tool windows are hidden (stored layout kept; they come back when the mode ends).
    pub fn hides_tool_windows(self) -> bool {
        self.presentation
    }

    /// The main toolbar content is hidden; the 30 pt header with the window controls stays.
    pub fn hides_toolbar(self) -> bool {
        self.distraction_free
    }

    /// The window header is the short one (30).
    pub fn compact_header(self) -> bool {
        self.distraction_free
    }

    /// The editor's text is centered (a left margin, see
    /// [`editor::distraction_free_margin`](crate::editor::distraction_free_margin)).
    pub fn centers_editor(self) -> bool {
        self.distraction_free || self.presentation
    }

    /// The editor gutter shows only line numbers' column (no icons, no folding).
    pub fn hides_gutter_extras(self) -> bool {
        self.distraction_free || self.presentation
    }
}

/// The look settings of one [`egui::Context`], stored next to the theme.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct LookSettings {
    /// The UI generation.
    pub look: UiLook,
    /// Compact density (`UIDensity.COMPACT`). Ignored while the look is [`UiLook::Classic`].
    pub compact: bool,
    /// View modes.
    pub view: ViewMode,
}

impl LookSettings {
    /// `look` at normal density in normal mode.
    pub const fn new(look: UiLook) -> Self {
        Self {
            look,
            compact: false,
            view: ViewMode::NORMAL,
        }
    }

    /// The same with compact density.
    pub const fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// The same in `view` mode.
    pub const fn view(mut self, view: ViewMode) -> Self {
        self.view = view;
        self
    }

    /// Whether compact density is in effect (it is not in Classic).
    pub fn is_compact(&self) -> bool {
        self.compact && self.look.supports_compact()
    }
}

fn settings_id() -> egui::Id {
    egui::Id::new("archipel_kit::look")
}

fn zoom_id() -> egui::Id {
    egui::Id::new("archipel_kit::look::base-zoom")
}

/// The look settings installed in `ctx`: Islands, not compact, normal mode if none were set.
pub fn settings(ctx: &Context) -> LookSettings {
    ctx.data(|d| d.get_temp::<LookSettings>(settings_id()))
        .unwrap_or_default()
}

/// The UI look of `ctx`.
pub fn look(ctx: &Context) -> UiLook {
    settings(ctx).look
}

/// Whether compact density is in effect in `ctx`.
pub fn is_compact(ctx: &Context) -> bool {
    settings(ctx).is_compact()
}

/// The view modes of `ctx`.
pub fn view_mode(ctx: &Context) -> ViewMode {
    settings(ctx).view
}

/// Switches `ctx` to `look` at runtime. The next frame draws every component in it.
pub fn set_look(ctx: &Context, look: UiLook) {
    let mut now = settings(ctx);
    now.look = look;
    set_settings(ctx, now);
}

/// Switches compact density on or off (no effect on the Classic look).
pub fn set_compact(ctx: &Context, compact: bool) {
    let mut now = settings(ctx);
    now.compact = compact;
    set_settings(ctx, now);
}

/// Switches the view modes. Entering presentation mode zooms the context to
/// [`ViewMode::PRESENTATION_SCALE`] times its current zoom (editor text included); leaving
/// restores the zoom it had.
pub fn set_view_mode(ctx: &Context, view: ViewMode) {
    let mut now = settings(ctx);
    now.view = view;
    set_settings(ctx, now);
}

/// Replaces all look settings of `ctx`.
pub fn set_settings(ctx: &Context, next: LookSettings) {
    let before = settings(ctx);
    if before != next {
        ctx.data_mut(|d| d.insert_temp(settings_id(), next));
        if before.view.presentation != next.view.presentation {
            if next.view.presentation {
                let base = ctx.zoom_factor();
                ctx.data_mut(|d| d.insert_temp(zoom_id(), base));
                ctx.set_zoom_factor(base * ViewMode::PRESENTATION_SCALE);
            } else if let Some(base) = ctx.data(|d| d.get_temp::<f32>(zoom_id())) {
                ctx.set_zoom_factor(base);
            }
        }
        if before.look != next.look {
            crate::style::apply_look(ctx);
        }
        ctx.request_repaint();
    }
}

/// How a selected row is shaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionShape {
    /// A rounded pill inset from the sides (New UI, Islands).
    Pill,
    /// A flat rectangle over the whole row (Classic).
    FullRow,
}

/// How tabs are painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabStyle {
    /// Rounded chips with a fill and a border (Islands).
    Pill,
    /// Flat tabs; the selected one has an underline (New UI: 4 px, radius 2; Classic: 3 px).
    Underline,
}

/// How tool window stripe buttons look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripeStyle {
    /// Square icon tiles (New UI, Islands).
    Icons,
    /// Rotated text buttons around the tool windows, with borders (Classic).
    Text,
}

/// How a tree's expander is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpanderStyle {
    /// A 16 px chevron (New UI, Islands).
    Chevron,
    /// A small triangle (Classic).
    Triangle,
}

/// How the editor gutter folds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldingStyle {
    /// Chevrons shown while the pointer is over the gutter (New UI, Islands).
    HoverChevrons,
    /// A vertical outline with square +/- anchors, always visible (Classic).
    Outline,
}

/// How panels are laid out on the window frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelStyle {
    /// Rounded islands with gaps (Islands).
    Islands,
    /// Flush panels separated by 1 px lines (New UI, Classic).
    Flush,
}

/// Every value that depends on the [`UiLook`] and on compact density, resolved. Get it with
/// [`crate::tokens::metrics`]; the `tokens::*` constants equal its Islands, normal-density
/// values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// The look these values are for.
    pub look: UiLook,
    /// Whether compact density is in effect (never in Classic).
    pub compact: bool,
    /// The view modes in effect.
    pub view: ViewMode,
    /// How panels sit on the frame.
    pub panels: PanelStyle,

    /// List, tree and table row height.
    pub row_height: f32,
    /// Indent per tree level.
    pub tree_indent: f32,
    /// Inset of a list or tree row's selection from the island's sides.
    pub row_inset: f32,
    /// Distance from a tree row's left edge to its expander column at depth 0 (Islands:
    /// the selection inset; Classic: the tree's 4 pt left border).
    pub tree_start: f32,
    /// Horizontal inset of the selection pill inside a row.
    pub row_pill_inset: f32,
    /// Shape of the selection.
    pub selection: SelectionShape,
    /// Corner radius of the selection pill (0 for full rows).
    pub row_radius: u8,
    /// Tree expander style.
    pub expander: ExpanderStyle,
    /// Table header height.
    pub table_header_height: f32,

    /// A tool window's title bar or an editor's tab strip.
    pub header_height: f32,
    /// The main toolbar / window header (the row with the project widgets).
    pub window_toolbar_height: f32,
    /// Classic menu bar row (0 where the menu lives in the header).
    pub menu_bar_height: f32,
    /// Toolbar button edge (window toolbar).
    pub toolbar_button: f32,
    /// Toolbar icon edge.
    pub toolbar_icon: f32,

    /// Button, text field, combo box height.
    pub control_height: f32,
    /// Small control height (speed search, small fields).
    pub control_height_small: f32,
    /// Corner radius of buttons, fields, combo boxes.
    pub control_radius: u8,
    /// Corner radius of icon buttons.
    pub icon_button_radius: u8,
    /// Corner radius of small icon buttons.
    pub icon_button_small_radius: u8,
    /// Buttons are filled with a vertical gradient (Classic) instead of flat.
    pub button_gradient: bool,
    /// Banner corner radius.
    pub banner_radius: u8,

    /// Gap between islands (0 in flush looks).
    pub island_gap: f32,
    /// Island corner radius (0 in flush looks).
    pub island_radius: u8,
    /// Width of a splitter's divider.
    pub split_divider: f32,
    /// Width of the line between flush panels (0 in Islands).
    pub panel_line: f32,

    /// Status bar height (including its top line).
    pub status_bar_height: f32,
    /// Height of the line on top of the status bar (0 in Islands).
    pub status_top_line: f32,
    /// Inset of the status bar content from its sides.
    pub status_bar_inset: f32,
    /// Horizontal padding inside a status bar widget.
    pub status_item_padding: f32,
    /// Height of a status bar widget's hover shape.
    pub status_item_height: f32,
    /// Corner radius of a status bar widget's hover shape.
    pub status_hover_radius: u8,
    /// The breadcrumb navigation bar sits in the status bar (New UI, Islands) instead of
    /// above the editor (Classic).
    pub navbar_in_status_bar: bool,

    /// Padding at the top and bottom of a popup or menu.
    pub popup_padding: f32,
    /// Inset of a popup row's highlight from the popup's sides.
    pub popup_item_inset: f32,
    /// Padding between the highlight and a popup row's content.
    pub popup_item_padding: f32,
    /// Popup row height.
    pub popup_row_height: f32,
    /// Corner radius of popups.
    pub popup_radius: u8,
    /// Menu rows highlight as full-width flat rectangles (Classic).
    pub popup_flat_rows: bool,

    /// How tabs are painted.
    pub tab_style: TabStyle,
    /// Height of a tab (the pill; the whole strip for underline tabs).
    pub tab_height: f32,
    /// Gap between tabs.
    pub tab_gap: f32,
    /// Space before a tab's content.
    pub tab_padding_left: f32,
    /// Space after a tab's content.
    pub tab_padding_right: f32,
    /// Space before the first tab.
    pub tab_strip_inset: f32,
    /// Height of the selected tab's underline (0 for pills).
    pub tab_underline: f32,
    /// Corner radius of the underline.
    pub tab_underline_radius: u8,
    /// Close buttons show on every tab (Classic), not only on hover and selection.
    pub tab_close_always: bool,

    /// How stripe buttons look.
    pub stripe_style: StripeStyle,
    /// Width of a side stripe, including the gap to its island.
    pub stripe_width: f32,
    /// Height of the classic bottom stripe.
    pub stripe_height: f32,
    /// Stripe button tile edge.
    pub stripe_button: f32,
    /// Distance between stripe buttons.
    pub stripe_pitch: f32,
    /// Stripe icon edge.
    pub stripe_icon: f32,

    /// Tool window header actions are always visible (Classic) instead of fading in on hover.
    pub header_actions_always: bool,
    /// The active tool window's header is highlighted (Classic).
    pub active_header_highlight: bool,
    /// Tool windows have a 1 px line under their header even without content tabs.
    pub header_line_always: bool,
    /// The tool window options button is a gear (Classic) instead of the kebab-style "more".
    pub header_gear: bool,

    /// Tooltip horizontal padding.
    pub tooltip_padding_x: f32,
    /// Tooltip vertical padding.
    pub tooltip_padding_y: f32,

    /// How the editor gutter folds.
    pub folding: FoldingStyle,
    /// Gap between gutter areas (line numbers, icons, folding).
    pub gutter_gap: f32,
}

impl Metrics {
    /// The values for `look` at normal (`compact == false`) or compact density, in normal
    /// view mode. `compact` is ignored for [`UiLook::Classic`].
    pub fn resolve(look: UiLook, compact: bool) -> Self {
        Self::of_settings(LookSettings::new(look).compact(compact))
    }

    /// The resolved values of `settings`, including its view mode.
    pub fn of_settings(settings: LookSettings) -> Self {
        let look = settings.look;
        let compact = settings.is_compact();
        let islands = look == UiLook::Islands;
        let classic = look == UiLook::Classic;
        // Pick by look: (islands, new ui, classic), and a compact override for the first two.
        let pick = |i: f32, n: f32, c: f32| match look {
            UiLook::Islands => i,
            UiLook::NewUi => n,
            UiLook::Classic => c,
        };
        let dense = |normal: f32, compact_value: f32| if compact { compact_value } else { normal };

        let header_height = if classic {
            28.0
        } else {
            dense(size::HEADER_HEIGHT, 30.0)
        };
        // The short header of Distraction Free mode, and the Classic toolbar row.
        let window_toolbar_height = if settings.view.compact_header() || classic {
            30.0
        } else {
            dense(size::WINDOW_TOOLBAR_HEIGHT, 32.0)
        };
        let stripe_icon = if classic {
            13.0
        } else {
            dense(stripe::ICON, 16.0)
        };
        // A stripe's icon tile is its icon plus 5 pt (4 compact) of padding on each side.
        let stripe_button = if classic {
            0.0
        } else {
            stripe_icon + dense(10.0, 8.0)
        };
        let stripe_pitch = if classic {
            0.0
        } else {
            dense(stripe::PITCH, 32.0)
        };
        let status_bar_height = match look {
            UiLook::Islands => dense(size::STATUS_BAR_HEIGHT, 25.0),
            UiLook::NewUi => dense(30.0, 25.0),
            UiLook::Classic => 23.0,
        };
        let island_gap = if islands {
            dense(size::ISLAND_GAP, 3.0)
        } else {
            0.0
        };
        let (tab_style, tab_underline, tab_underline_radius) = match look {
            UiLook::Islands => (TabStyle::Pill, 0.0, 0),
            UiLook::NewUi => (TabStyle::Underline, 4.0, radius::UNDERLINE),
            UiLook::Classic => (TabStyle::Underline, 3.0, 0),
        };
        let control_height = if classic {
            24.0
        } else {
            dense(size::CONTROL_HEIGHT, 24.0)
        };
        Metrics {
            look,
            compact,
            view: settings.view,
            panels: if islands {
                PanelStyle::Islands
            } else {
                PanelStyle::Flush
            },

            row_height: if classic {
                20.0
            } else {
                dense(size::ROW_HEIGHT, 22.0)
            },
            tree_indent: pick(size::TREE_INDENT, size::TREE_INDENT, 18.0),
            row_inset: if classic {
                0.0
            } else {
                dense(size::ROW_INSET, 8.0)
            },
            tree_start: if classic {
                4.0
            } else {
                dense(size::CONTENT_INSET, 8.0)
            },
            row_pill_inset: if classic { 0.0 } else { size::ROW_PILL_INSET },
            selection: if classic {
                SelectionShape::FullRow
            } else {
                SelectionShape::Pill
            },
            row_radius: if classic { 0 } else { radius::ROW },
            expander: if classic {
                ExpanderStyle::Triangle
            } else {
                ExpanderStyle::Chevron
            },
            table_header_height: if classic { 25.0 } else { 28.0 },

            header_height,
            window_toolbar_height,
            menu_bar_height: if classic { 24.0 } else { 0.0 },
            toolbar_button: if classic {
                size::ICON_BUTTON
            } else {
                dense(toolbar::BUTTON, 26.0)
            },
            toolbar_icon: if classic {
                size::ICON
            } else {
                dense(toolbar::ICON, 16.0)
            },

            control_height,
            control_height_small: if classic || compact {
                20.0
            } else {
                size::CONTROL_HEIGHT_SMALL
            },
            control_radius: if classic || compact {
                3
            } else {
                radius::CONTROL
            },
            icon_button_radius: if classic { 3 } else { radius::ICON_BUTTON },
            icon_button_small_radius: if classic {
                3
            } else {
                radius::ICON_BUTTON_SMALL
            },
            button_gradient: classic,
            banner_radius: if classic {
                0
            } else if compact {
                4
            } else {
                radius::BANNER
            },

            island_gap,
            island_radius: if islands {
                if compact { 8 } else { radius::ISLAND }
            } else {
                0
            },
            split_divider: if islands { island_gap } else { 1.0 },
            panel_line: if islands { 0.0 } else { 1.0 },

            status_bar_height,
            status_top_line: pick(0.0, 1.0, 1.0),
            status_bar_inset: pick(size::STATUS_BAR_INSET, 10.0, 2.0),
            status_item_padding: pick(size::STATUS_ITEM_PADDING, 8.0, size::STATUS_ITEM_PADDING),
            status_item_height: if classic {
                status_bar_height - 1.0
            } else {
                dense(size::STATUS_ITEM_HEIGHT, 17.0)
            },
            status_hover_radius: if classic {
                0
            } else if islands {
                radius::CONTROL
            } else {
                2
            },
            navbar_in_status_bar: !classic,

            popup_padding: if classic { 4.0 } else { popup::PADDING },
            popup_item_inset: if classic { 0.0 } else { popup::ITEM_INSET },
            popup_item_padding: if classic { 12.0 } else { popup::ITEM_PADDING },
            popup_row_height: if classic {
                22.0
            } else {
                dense(size::ROW_HEIGHT, 22.0)
            },
            popup_radius: if classic { 0 } else { radius::POPUP },
            popup_flat_rows: classic,

            tab_style,
            tab_height: if islands {
                dense(tab::HEIGHT, 24.0)
            } else {
                header_height
            },
            tab_gap: if islands { dense(tab::GAP, 4.0) } else { 0.0 },
            tab_padding_left: if classic { 6.0 } else { tab::PADDING_LEFT },
            tab_padding_right: pick(tab::PADDING_RIGHT, 6.0, 4.0),
            tab_strip_inset: if islands { tab::STRIP_INSET } else { 0.0 },
            tab_underline,
            tab_underline_radius,
            tab_close_always: classic,

            stripe_style: if classic {
                StripeStyle::Text
            } else {
                StripeStyle::Icons
            },
            stripe_width: match look {
                UiLook::Islands => dense(stripe::WIDTH, 35.0),
                UiLook::NewUi => dense(41.0, 33.0),
                UiLook::Classic => 27.0,
            },
            stripe_height: 27.0,
            stripe_button,
            stripe_pitch,
            stripe_icon,

            header_actions_always: classic,
            active_header_highlight: classic,
            header_line_always: classic,
            header_gear: classic,

            tooltip_padding_x: if classic {
                10.0
            } else {
                size::TOOLTIP_PADDING_X
            },
            tooltip_padding_y: if classic {
                7.0
            } else {
                size::TOOLTIP_PADDING_Y
            },

            folding: if classic {
                FoldingStyle::Outline
            } else {
                FoldingStyle::HoverChevrons
            },
            gutter_gap: if classic {
                5.0
            } else {
                dense(crate::tokens::editor::GUTTER_GAP, 2.0)
            },
        }
    }

    /// The metrics of `ctx`'s current look settings.
    pub fn of(ctx: &Context) -> Self {
        Self::of_settings(settings(ctx))
    }

    /// Distance from a tree row's left edge to its expander column at `depth` (the look's
    /// [`size::indent`]).
    pub fn indent(&self, depth: usize) -> f32 {
        self.tree_start + depth as f32 * self.tree_indent
    }

    /// Width of the stripe column without its gap to the island (where the buttons sit).
    pub fn stripe_column(&self) -> f32 {
        self.stripe_width - self.island_gap
    }

    /// Whether panels are rounded islands.
    pub fn is_islands(&self) -> bool {
        self.panels == PanelStyle::Islands
    }

    /// Whether the look has square icon stripe buttons.
    pub fn has_icon_stripes(&self) -> bool {
        self.stripe_style == StripeStyle::Icons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_islands_normal() {
        let ctx = Context::default();
        let s = settings(&ctx);
        assert_eq!(s.look, UiLook::Islands);
        assert!(!s.compact);
        assert_eq!(s.view, ViewMode::NORMAL);
    }

    #[test]
    fn islands_metrics_equal_the_tokens() {
        let m = Metrics::resolve(UiLook::Islands, false);
        assert_eq!(m.row_height, size::ROW_HEIGHT);
        assert_eq!(m.tree_indent, size::TREE_INDENT);
        assert_eq!(m.row_inset, size::ROW_INSET);
        assert_eq!(m.header_height, size::HEADER_HEIGHT);
        assert_eq!(m.window_toolbar_height, size::WINDOW_TOOLBAR_HEIGHT);
        assert_eq!(m.control_height, size::CONTROL_HEIGHT);
        assert_eq!(m.control_height_small, size::CONTROL_HEIGHT_SMALL);
        assert_eq!(m.island_gap, size::ISLAND_GAP);
        assert_eq!(m.split_divider, size::SPLIT_DIVIDER);
        assert_eq!(m.status_bar_height, size::STATUS_BAR_HEIGHT);
        assert_eq!(m.status_bar_inset, size::STATUS_BAR_INSET);
        assert_eq!(m.status_item_padding, size::STATUS_ITEM_PADDING);
        assert_eq!(m.status_item_height, size::STATUS_ITEM_HEIGHT);
        assert_eq!(m.popup_padding, popup::PADDING);
        assert_eq!(m.popup_item_inset, popup::ITEM_INSET);
        assert_eq!(m.popup_item_padding, popup::ITEM_PADDING);
        assert_eq!(m.tab_height, tab::HEIGHT);
        assert_eq!(m.tab_gap, tab::GAP);
        assert_eq!(m.tab_padding_left, tab::PADDING_LEFT);
        assert_eq!(m.tab_padding_right, tab::PADDING_RIGHT);
        assert_eq!(m.tab_strip_inset, tab::STRIP_INSET);
        assert_eq!(m.stripe_width, stripe::WIDTH);
        assert_eq!(m.stripe_button, stripe::BUTTON);
        assert_eq!(m.stripe_pitch, stripe::PITCH);
        assert_eq!(m.stripe_icon, stripe::ICON);
        assert_eq!(m.toolbar_button, toolbar::BUTTON);
        assert_eq!(m.toolbar_icon, toolbar::ICON);
        assert_eq!(m.island_radius, radius::ISLAND);
        assert_eq!(m.control_radius, radius::CONTROL);
        assert_eq!(m.icon_button_radius, radius::ICON_BUTTON);
        assert_eq!(m.popup_radius, radius::POPUP);
        assert_eq!(m.tooltip_padding_x, size::TOOLTIP_PADDING_X);
        assert_eq!(m.tooltip_padding_y, size::TOOLTIP_PADDING_Y);
    }

    #[test]
    fn compact_applies_to_new_ui_and_islands_only() {
        for look in [UiLook::Islands, UiLook::NewUi] {
            let m = Metrics::resolve(look, true);
            assert_eq!(m.row_height, 22.0);
            assert_eq!(m.header_height, 30.0);
            assert_eq!(m.window_toolbar_height, 32.0);
            assert_eq!(m.control_height, 24.0);
            assert_eq!(m.stripe_pitch, 32.0);
            assert_eq!(m.stripe_icon, 16.0);
            assert!(m.compact);
        }
        assert_eq!(Metrics::resolve(UiLook::Islands, true).island_radius, 8);
        let classic = Metrics::resolve(UiLook::Classic, true);
        assert!(!classic.compact);
        assert_eq!(classic, Metrics::resolve(UiLook::Classic, false));
    }

    #[test]
    fn view_modes_follow_the_spec() {
        assert_eq!(ViewMode::presentation().scale(), 1.75);
        assert_eq!(ViewMode::NORMAL.scale(), 1.0);
        assert!(ViewMode::zen().is_zen());
        assert!(!ViewMode::distraction_free().is_zen());
        let presentation = ViewMode::presentation();
        assert!(presentation.hides_stripes() && presentation.hides_status_bar());
        assert!(presentation.hides_tabs() && presentation.hides_tool_windows());
        assert!(!presentation.hides_toolbar());
        let dfm = ViewMode::distraction_free();
        assert!(dfm.hides_toolbar() && dfm.compact_header() && dfm.hides_stripes());
        assert!(!dfm.hides_tool_windows());
        let m = Metrics::of_settings(LookSettings::new(UiLook::Islands).view(dfm));
        assert_eq!(m.window_toolbar_height, 30.0);
    }

    #[test]
    fn switching_at_runtime_changes_metrics() {
        let ctx = Context::default();
        assert_eq!(Metrics::of(&ctx).row_height, 24.0);
        set_look(&ctx, UiLook::Classic);
        assert_eq!(Metrics::of(&ctx).row_height, 20.0);
        set_look(&ctx, UiLook::NewUi);
        set_compact(&ctx, true);
        assert_eq!(Metrics::of(&ctx).row_height, 22.0);
        set_look(&ctx, UiLook::Islands);
        assert_eq!(Metrics::of(&ctx).island_radius, 8);
    }

    #[test]
    fn presentation_mode_zooms_and_restores() {
        let ctx = Context::default();
        // A new zoom factor takes effect with the next frame.
        let frame = |ctx: &Context| {
            let _ = ctx.run(egui::RawInput::default(), |_| {});
        };
        ctx.set_zoom_factor(1.25);
        frame(&ctx);
        set_view_mode(&ctx, ViewMode::presentation());
        frame(&ctx);
        assert!((ctx.zoom_factor() - 1.25 * 1.75).abs() < 1e-4);
        set_view_mode(&ctx, ViewMode::NORMAL);
        frame(&ctx);
        assert!((ctx.zoom_factor() - 1.25).abs() < 1e-4);
    }
}
