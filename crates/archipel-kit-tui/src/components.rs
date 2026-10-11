//! The components, as ratatui [`Widget`]s. Names and options follow the GUI kit, so a
//! screen can be built twice from the same description.
//!
//! Icons are Nerd Font characters followed by a space: Nerd Font icons are wider than one
//! cell and draw into it.

use crate::glyphs::{self, CAP_LEFT, CAP_RIGHT};
use crate::{Glyph, Language, Look, Symbol};
use archipel_kit::components::{ButtonVariant, Severity};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

/// Writes `text` at (`x`, `y`) clipped to `area`, returning the column after it.
fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, text: &str, style: Style) -> u16 {
    if y < area.top() || y >= area.bottom() || x >= area.right() || x < area.left() {
        return x;
    }
    let width = (area.right() - x) as usize;
    let (end, _) = buf.set_stringn(x, y, text, width, style);
    end
}

/// Fills cells `x..x + width` of row `y` with blanks in `style`, clipped to `area`.
fn fill(buf: &mut Buffer, area: Rect, x: u16, y: u16, width: u16, style: Style) {
    if y < area.top() || y >= area.bottom() {
        return;
    }
    for cx in x.max(area.left())..(x.saturating_add(width)).min(area.right()) {
        if let Some(cell) = buf.cell_mut((cx, y)) {
            cell.set_symbol(" ").set_style(style);
        }
    }
}

/// A pill: `width` cells of `bg` with half-circle caps at both ends, on `under`.
/// Returns the first column inside the left cap.
fn pill(buf: &mut Buffer, area: Rect, x: u16, y: u16, width: u16, bg: Color, under: Color) -> u16 {
    if width < 2 {
        return x;
    }
    let cap = Style::default().fg(bg).bg(under);
    put(buf, area, x, y, &CAP_LEFT.to_string(), cap);
    fill(buf, area, x + 1, y, width - 2, Style::default().bg(bg));
    put(buf, area, x + width - 1, y, &CAP_RIGHT.to_string(), cap);
    x + 1
}

/// An island: an area filled with the island color with notched corners and an optional
/// bold title row. Place islands one cell apart on a [`Frame`].
pub struct Island<'a> {
    look: &'a Look,
    title: Option<&'a str>,
}

impl<'a> Island<'a> {
    /// An untitled island.
    pub fn new(look: &'a Look) -> Self {
        Self { look, title: None }
    }

    /// A title on the first row.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// The content area: inset by two columns, below the title row.
    pub fn inner(&self, area: Rect) -> Rect {
        let top = if self.title.is_some() { 2 } else { 1 };
        Rect::new(
            area.x + 2,
            area.y + top,
            area.width.saturating_sub(4),
            area.height.saturating_sub(top + 1),
        )
    }
}

impl Widget for Island<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        for y in area.top()..area.bottom() {
            fill(buf, area, area.x, y, area.width, self.look.body());
        }
        if area.width >= 2 && area.height >= 2 {
            let corner = Style::default()
                .fg(self.look.color(c.island))
                .bg(self.look.color_over(c.frame, c.frame));
            let (right, bottom) = (area.right() - 1, area.bottom() - 1);
            for (x, y, quadrants) in [
                (area.x, area.y, "▟"),
                (right, area.y, "▙"),
                (area.x, bottom, "▜"),
                (right, bottom, "▛"),
            ] {
                put(buf, area, x, y, quadrants, corner);
            }
        }
        if let Some(title) = self.title {
            put(buf, area, area.x + 2, area.y, title, self.look.strong());
        }
    }
}

/// Fills `area` with the frame color: the background islands sit on.
pub struct Frame<'a> {
    look: &'a Look,
}

impl<'a> Frame<'a> {
    /// The frame of `look`.
    pub fn new(look: &'a Look) -> Self {
        Self { look }
    }
}

impl Widget for Frame<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let style = Style::default()
            .bg(self.look.color_over(c.frame, c.frame))
            .fg(self.look.color(c.text));
        for y in area.top()..area.bottom() {
            fill(buf, area, area.x, y, area.width, style);
        }
    }
}

/// A push button: a pill with an optional icon and the label.
pub struct Button<'a> {
    look: &'a Look,
    label: &'a str,
    variant: ButtonVariant,
    icon: Option<Glyph>,
    focused: bool,
    enabled: bool,
}

impl<'a> Button<'a> {
    /// A button of `variant`.
    pub fn new(look: &'a Look, label: &'a str, variant: ButtonVariant) -> Self {
        Self {
            look,
            label,
            variant,
            icon: None,
            focused: false,
            enabled: true,
        }
    }

    /// The accent-filled default action.
    pub fn primary(look: &'a Look, label: &'a str) -> Self {
        Self::new(look, label, ButtonVariant::Primary)
    }

    /// A filled neutral button.
    pub fn secondary(look: &'a Look, label: &'a str) -> Self {
        Self::new(look, label, ButtonVariant::Secondary)
    }

    /// A button without fill.
    pub fn ghost(look: &'a Look, label: &'a str) -> Self {
        Self::new(look, label, ButtonVariant::Ghost)
    }

    /// A destructive action.
    pub fn danger(look: &'a Look, label: &'a str) -> Self {
        Self::new(look, label, ButtonVariant::Danger)
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Keyboard focus: bold, underlined label.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Disabled buttons are dimmed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Width in cells.
    pub fn width(&self) -> u16 {
        let icon = if self.icon.is_some() { 2 } else { 0 };
        (self.label.width() + icon + 4) as u16
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let island = look.color(c.island);
        let (bg, fg) = match (self.variant, self.enabled) {
            (ButtonVariant::Primary, true) => (look.color(c.accent), look.color(c.text_on_accent)),
            (ButtonVariant::Danger, true) => (look.color(c.stop), look.color(c.text_on_accent)),
            // Disabled fills are neutral with dim text that stays legible in cells.
            (ButtonVariant::Primary | ButtonVariant::Danger | ButtonVariant::Secondary, false) => {
                (look.color(c.border), look.color(c.text_placeholder))
            }
            (ButtonVariant::Secondary, true) => (look.color(c.border), look.color(c.text_strong)),
            (ButtonVariant::Ghost, enabled) => (
                island,
                look.color(if enabled { c.text } else { c.text_disabled }),
            ),
        };
        let width = self.width();
        let x = pill(buf, area, area.x, area.y, width, bg, island) + 1;
        let style = Style::default().bg(bg).fg(fg);
        let x = match self.icon {
            Some(icon) => {
                let color = if self.enabled && self.variant == ButtonVariant::Ghost {
                    look.glyph_color(icon)
                } else {
                    fg
                };
                put(buf, area, x, area.y, &look.glyph(icon), style.fg(color)) + 1
            }
            None => x,
        };
        let mut label = style;
        if self.focused {
            label = label.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        }
        put(buf, area, x, area.y, self.label, label);
    }
}

/// A checkbox: the box icon in the accent when checked, then the label.
pub struct Checkbox<'a> {
    look: &'a Look,
    checked: bool,
    mixed: bool,
    label: &'a str,
    focused: bool,
    enabled: bool,
}

impl<'a> Checkbox<'a> {
    /// A checkbox showing `checked`.
    pub fn new(look: &'a Look, checked: bool, label: &'a str) -> Self {
        Self {
            look,
            checked,
            mixed: false,
            label,
            focused: false,
            enabled: true,
        }
    }

    /// Some, not all, children checked: a dash.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// Keyboard focus: bold label.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Disabled checkboxes are dimmed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for Checkbox<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (mark, color) = match (self.mixed, self.checked) {
            (true, _) => (glyphs::CHECKBOX_MIXED, c.accent),
            (false, true) => (glyphs::CHECKBOX_ON, c.accent),
            (false, false) => (glyphs::CHECKBOX_OFF, c.control_border),
        };
        let color = if self.enabled { color } else { c.text_disabled };
        put(
            buf,
            area,
            area.x,
            area.y,
            &mark.to_string(),
            look.text(color),
        );
        let mut label = look.text(if self.enabled {
            c.text
        } else {
            c.text_disabled
        });
        if self.focused {
            label = label.add_modifier(Modifier::BOLD);
        }
        put(buf, area, area.x + 2, area.y, self.label, label);
    }
}

/// A radio button: a filled ring when selected.
pub struct Radio<'a> {
    look: &'a Look,
    selected: bool,
    label: &'a str,
    enabled: bool,
}

impl<'a> Radio<'a> {
    /// An option showing `selected`.
    pub fn new(look: &'a Look, selected: bool, label: &'a str) -> Self {
        Self {
            look,
            selected,
            label,
            enabled: true,
        }
    }

    /// Disabled options are dimmed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for Radio<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (mark, color) = if self.selected {
            (glyphs::RADIO_ON, c.accent)
        } else {
            (glyphs::RADIO_OFF, c.control_border)
        };
        let color = if self.enabled { color } else { c.text_disabled };
        put(
            buf,
            area,
            area.x,
            area.y,
            &mark.to_string(),
            look.text(color),
        );
        let text = if self.enabled {
            c.text
        } else {
            c.text_disabled
        };
        put(buf, area, area.x + 2, area.y, self.label, look.text(text));
    }
}

/// An on/off switch: a pill track with the knob at the right when on.
pub struct Toggle<'a> {
    look: &'a Look,
    on: bool,
    label: &'a str,
}

impl<'a> Toggle<'a> {
    /// A switch showing `on`.
    pub fn new(look: &'a Look, on: bool, label: &'a str) -> Self {
        Self { look, on, label }
    }
}

impl Widget for Toggle<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (track, knob, text) = if self.on {
            (c.toggle_on, archipel_kit::egui::Color32::WHITE, " ●")
        } else {
            (c.control_border, c.toggle_off_knob, "● ")
        };
        let track = look.color(track);
        let x = pill(buf, area, area.x, area.y, 4, track, look.color(c.island));
        put(
            buf,
            area,
            x,
            area.y,
            text,
            Style::default().bg(track).fg(look.color(knob)),
        );
        put(buf, area, area.x + 5, area.y, self.label, look.body());
    }
}

/// A single-line input: the value in a pill, a placeholder while empty, a cursor and
/// accent caps while focused, red caps and a message below on error.
pub struct TextField<'a> {
    look: &'a Look,
    value: &'a str,
    placeholder: &'a str,
    label: Option<&'a str>,
    error: Option<&'a str>,
    focused: bool,
}

impl<'a> TextField<'a> {
    /// A field showing `value`.
    pub fn new(look: &'a Look, value: &'a str) -> Self {
        Self {
            look,
            value,
            placeholder: "",
            label: None,
            error: None,
            focused: false,
        }
    }

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// A label row above the field.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// A validation message below the field.
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    /// Keyboard focus: accent caps and a cursor.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Rows the field needs (label, field, error).
    pub fn height(&self) -> u16 {
        1 + u16::from(self.label.is_some()) + u16::from(self.error.is_some())
    }
}

impl Widget for TextField<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let mut y = area.y;
        if let Some(label) = self.label {
            put(buf, area, area.x, y, label, look.body());
            y += 1;
        }
        // The combo fill, or the frame color where the combo fill is the island color.
        let field = look.color(if c.combo == c.island {
            c.frame
        } else {
            c.combo
        });
        pill(
            buf,
            area,
            area.x,
            y,
            area.width,
            field,
            look.color(c.island),
        );
        let edge = if self.error.is_some() {
            Some(c.error)
        } else if self.focused {
            Some(c.accent)
        } else {
            None
        };
        if let Some(edge) = edge {
            let caps = look.text(edge);
            put(buf, area, area.x, y, &CAP_LEFT.to_string(), caps);
            if area.width > 1 {
                put(buf, area, area.right() - 1, y, &CAP_RIGHT.to_string(), caps);
            }
        }
        let inner = Rect::new(
            area.x + 1,
            area.y,
            area.width.saturating_sub(2),
            area.height,
        );
        let text = Style::default().bg(field).fg(look.color(c.text_strong));
        let x = area.x + 2;
        if self.value.is_empty() && !self.focused {
            put(
                buf,
                inner,
                x,
                y,
                self.placeholder,
                text.fg(look.color(c.text_placeholder)),
            );
        } else {
            let end = put(buf, inner, x, y, self.value, text);
            if self.focused {
                put(buf, inner, end, y, "▏", text.fg(look.color(c.accent)));
            }
        }
        if let Some(error) = self.error {
            put(buf, area, area.x + 1, y + 1, error, look.text(c.error));
        }
    }
}

/// An icon in a row: a glyph, a language logo or a symbol.
#[derive(Debug, Clone, Copy)]
pub enum RowIcon {
    /// A kit glyph in its GUI color.
    Glyph(Glyph),
    /// A language logo in the language color.
    Language(Language),
    /// A symbol icon in the symbol color.
    Symbol(Symbol),
}

impl From<Glyph> for RowIcon {
    fn from(glyph: Glyph) -> Self {
        RowIcon::Glyph(glyph)
    }
}

impl From<Language> for RowIcon {
    fn from(language: Language) -> Self {
        RowIcon::Language(language)
    }
}

impl From<Symbol> for RowIcon {
    fn from(symbol: Symbol) -> Self {
        RowIcon::Symbol(symbol)
    }
}

impl RowIcon {
    /// The character and its color in `look`.
    pub fn char_and_color(self, look: &Look) -> (char, Color) {
        match self {
            RowIcon::Glyph(glyph) => (glyphs::nerd_font(glyph), look.glyph_color(glyph)),
            RowIcon::Language(language) => {
                (glyphs::language(language), look.readable(language.color()))
            }
            RowIcon::Symbol(symbol) => (
                glyphs::symbol(symbol.kind),
                look.readable(symbol.kind.color()),
            ),
        }
    }
}

/// Draws `icon` and the space after it at (`x`, `y`) on `bg`, returning the column after.
fn draw_icon(
    look: &Look,
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    icon: RowIcon,
    bg: Color,
) -> u16 {
    let (ch, color) = icon.char_and_color(look);
    let x = put(
        buf,
        area,
        x,
        y,
        &ch.to_string(),
        Style::default().fg(color).bg(bg),
    );
    put(buf, area, x, y, " ", Style::default().bg(bg))
}

/// One row of a list or tree; the selected row is an inset pill.
pub struct ListRow<'a> {
    look: &'a Look,
    label: &'a str,
    secondary: Option<&'a str>,
    icon: Option<RowIcon>,
    depth: u16,
    expanded: Option<bool>,
    selected: bool,
}

impl<'a> ListRow<'a> {
    /// A row showing `label`.
    pub fn new(look: &'a Look, label: &'a str) -> Self {
        Self {
            look,
            label,
            secondary: None,
            icon: None,
            depth: 0,
            expanded: None,
            selected: false,
        }
    }

    /// Dimmed text after the label.
    pub fn secondary(mut self, text: &'a str) -> Self {
        self.secondary = Some(text);
        self
    }

    /// Icon before the label.
    pub fn icon(mut self, icon: impl Into<RowIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Tree depth (two cells per level).
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// `Some` draws an expander chevron.
    pub fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }

    /// Highlights the row.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Widget for ListRow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let island = look.color(c.island);
        let bg = if self.selected {
            pill(
                buf,
                area,
                area.x,
                area.y,
                area.width,
                look.color(c.selection),
                island,
            );
            look.color(c.selection)
        } else {
            fill(
                buf,
                area,
                area.x,
                area.y,
                area.width,
                Style::default().bg(island),
            );
            island
        };
        let base = Style::default().bg(bg);
        let mut x = area.x.saturating_add(1 + self.depth * 2);
        if let Some(expanded) = self.expanded {
            let chevron = if expanded {
                Glyph::ChevronDown
            } else {
                Glyph::ChevronRight
            };
            put(
                buf,
                area,
                x,
                area.y,
                &look.glyph(chevron),
                base.fg(look.color(c.text_secondary)),
            );
        }
        x = x.saturating_add(2);
        if let Some(icon) = self.icon {
            x = draw_icon(look, buf, area, x, area.y, icon, bg);
        }
        let label = base.fg(look.color(if self.selected { c.text_strong } else { c.text }));
        x = put(buf, area, x, area.y, self.label, label);
        if let Some(secondary) = self.secondary {
            put(
                buf,
                area,
                x + 1,
                area.y,
                secondary,
                base.fg(look.color(c.text_placeholder)),
            );
        }
    }
}

/// Editor tabs on one row; the selected tab is a tinted pill with a close button.
pub struct TabBar<'a> {
    look: &'a Look,
    tabs: &'a [(Option<RowIcon>, &'a str, bool)],
    selected: usize,
}

impl<'a> TabBar<'a> {
    /// `tabs` as (icon, label, modified), with `selected` active.
    pub fn new(
        look: &'a Look,
        tabs: &'a [(Option<RowIcon>, &'a str, bool)],
        selected: usize,
    ) -> Self {
        Self {
            look,
            tabs,
            selected,
        }
    }

    fn tab_width(icon: Option<RowIcon>, label: &str, modified: bool, selected: bool) -> u16 {
        let icon = if icon.is_some() { 2 } else { 0 };
        let marks = if modified { 2 } else { 0 } + if selected { 2 } else { 0 };
        (4 + icon + label.width() + marks) as u16
    }
}

impl Widget for TabBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let island = look.color(c.island);
        let mut x = area.x;
        for (index, (icon, label, modified)) in self.tabs.iter().enumerate() {
            let selected = index == self.selected;
            let width = Self::tab_width(*icon, label, *modified, selected);
            let bg = if selected {
                pill(
                    buf,
                    area,
                    x,
                    area.y,
                    width,
                    look.color(c.tab_selected),
                    island,
                );
                look.color(c.tab_selected)
            } else {
                island
            };
            let mut cx = x + 2;
            if let Some(icon) = icon {
                cx = draw_icon(look, buf, area, cx, area.y, *icon, bg);
            }
            let mut text = Style::default().bg(bg).fg(look.color(if selected {
                c.text_strong
            } else {
                c.text_secondary
            }));
            if selected {
                text = text.add_modifier(Modifier::BOLD);
            }
            cx = put(buf, area, cx, area.y, label, text);
            let dim = Style::default().bg(bg).fg(look.color(c.text_secondary));
            if *modified {
                cx = put(buf, area, cx + 1, area.y, "●", dim);
            }
            if selected {
                put(buf, area, cx + 1, area.y, &look.glyph(Glyph::Close), dim);
            }
            x += width + 1;
        }
    }
}

/// A full-width message pill with an icon, text and action pills on the right.
pub struct Banner<'a> {
    look: &'a Look,
    severity: Severity,
    text: &'a str,
    actions: &'a [&'a str],
}

impl<'a> Banner<'a> {
    /// A banner of `severity` saying `text`, with `actions`.
    pub fn new(look: &'a Look, severity: Severity, text: &'a str, actions: &'a [&'a str]) -> Self {
        Self {
            look,
            severity,
            text,
            actions,
        }
    }
}

impl Widget for Banner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (tint, glyph) = match self.severity {
            Severity::Info => (c.banner_info, Glyph::Info),
            Severity::Success => (c.banner_success, Glyph::Success),
            Severity::Warning => (c.banner_warning, Glyph::Warning),
            Severity::Error => (c.banner_error, Glyph::Error),
        };
        let bg = look.color(tint.background);
        pill(
            buf,
            area,
            area.x,
            area.y,
            area.width,
            bg,
            look.color(c.island),
        );
        let base = Style::default().bg(bg).fg(look.color(c.text_strong));
        let x = put(
            buf,
            area,
            area.x + 2,
            area.y,
            &look.glyph(glyph),
            base.fg(look.glyph_color(glyph)),
        );
        put(buf, area, x + 1, area.y, self.text, base);
        let mut right = area.right().saturating_sub(2);
        for action in self.actions.iter().rev() {
            let width = action.width() as u16 + 4;
            right = right.saturating_sub(width);
            let x = pill(buf, area, right, area.y, width, look.color(tint.border), bg);
            put(
                buf,
                area,
                x + 1,
                area.y,
                action,
                Style::default()
                    .bg(look.color(tint.border))
                    .fg(look.color(c.text_strong)),
            );
            right = right.saturating_sub(1);
        }
    }
}

/// Options in a row; the selected one is a pill.
pub struct SegmentedControl<'a> {
    look: &'a Look,
    options: &'a [&'a str],
    selected: usize,
}

impl<'a> SegmentedControl<'a> {
    /// `options` with `selected` active.
    pub fn new(look: &'a Look, options: &'a [&'a str], selected: usize) -> Self {
        Self {
            look,
            options,
            selected,
        }
    }
}

impl Widget for SegmentedControl<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let island = look.color(c.island);
        let mut x = area.x;
        for (index, option) in self.options.iter().enumerate() {
            let width = option.width() as u16 + 4;
            let style = if index == self.selected {
                let bg = look.color(c.segment_selected_border);
                pill(buf, area, x, area.y, width, bg, island);
                Style::default()
                    .bg(bg)
                    .fg(look.color(c.text_strong))
                    .add_modifier(Modifier::BOLD)
            } else {
                look.text(c.text_secondary)
            };
            put(buf, area, x + 2, area.y, option, style);
            x += width;
        }
    }
}

/// A thin progress bar: filled in the accent, the rest in the track color.
pub struct ProgressBar<'a> {
    look: &'a Look,
    fraction: f32,
}

impl<'a> ProgressBar<'a> {
    /// A bar at `fraction` (0..=1).
    pub fn new(look: &'a Look, fraction: f32) -> Self {
        Self {
            look,
            fraction: fraction.clamp(0.0, 1.0),
        }
    }
}

impl Widget for ProgressBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let filled = (area.width as f32 * self.fraction).round() as u16;
        let bar = "━".repeat(filled as usize);
        let rest = "━".repeat((area.width - filled) as usize);
        let x = put(buf, area, area.x, area.y, &bar, self.look.text(c.accent));
        put(
            buf,
            area,
            x,
            area.y,
            &rest,
            self.look.text(c.progress_track),
        );
    }
}

/// Keyboard shortcut caps.
pub struct Kbd<'a> {
    look: &'a Look,
    keys: &'a [&'a str],
}

impl<'a> Kbd<'a> {
    /// Caps for `keys`.
    pub fn new(look: &'a Look, keys: &'a [&'a str]) -> Self {
        Self { look, keys }
    }

    /// Width in cells.
    pub fn width(&self) -> u16 {
        self.keys
            .iter()
            .map(|key| key.width() as u16 + 3)
            .sum::<u16>()
            .saturating_sub(1)
    }
}

impl Widget for Kbd<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let bg = look.color(c.border);
        let mut x = area.x;
        for key in self.keys {
            let width = key.width() as u16 + 2;
            let inner = pill(buf, area, x, area.y, width, bg, look.color(c.island));
            put(
                buf,
                area,
                inner,
                area.y,
                key,
                Style::default().bg(bg).fg(look.color(c.text_strong)),
            );
            x += width + 1;
        }
    }
}

/// A chip of text in a pill: tags and counters.
pub struct Chip<'a> {
    look: &'a Look,
    text: &'a str,
    color: Option<archipel_kit::egui::Color32>,
    accent: bool,
}

impl<'a> Chip<'a> {
    /// A neutral tag.
    pub fn tag(look: &'a Look, text: &'a str) -> Self {
        Self {
            look,
            text,
            color: None,
            accent: false,
        }
    }

    /// A tag tinted with `color`.
    pub fn colored(look: &'a Look, text: &'a str, color: archipel_kit::egui::Color32) -> Self {
        Self {
            look,
            text,
            color: Some(color),
            accent: false,
        }
    }

    /// A counter (accent-filled when `accent`).
    pub fn counter(look: &'a Look, text: &'a str, accent: bool) -> Self {
        Self {
            look,
            text,
            color: None,
            accent,
        }
    }

    /// Width in cells.
    pub fn width(&self) -> u16 {
        self.text.width() as u16 + 2
    }
}

impl Widget for Chip<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (bg, fg) = match (self.color, self.accent) {
            (Some(color), _) => (look.color(color.gamma_multiply(0.22)), look.color(color)),
            (None, true) => (look.color(c.accent), look.color(c.text_on_accent)),
            (None, false) => (look.color(c.border), look.color(c.text_secondary)),
        };
        let width = self.width();
        let x = pill(buf, area, area.x, area.y, width, bg, look.color(c.island));
        put(
            buf,
            area,
            x,
            area.y,
            self.text,
            Style::default().bg(bg).fg(fg),
        );
    }
}

/// A language logo in the language color.
pub struct LanguageIcon<'a> {
    look: &'a Look,
    language: Language,
}

impl<'a> LanguageIcon<'a> {
    /// The icon for `language`.
    pub fn new(look: &'a Look, language: Language) -> Self {
        Self { look, language }
    }
}

impl Widget for LanguageIcon<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = self.look.color(self.look.theme.colors.island);
        draw_icon(
            self.look,
            buf,
            area,
            area.x,
            area.y,
            self.language.into(),
            bg,
        );
    }
}

/// A symbol icon (class, method, field …) in the symbol color.
pub struct SymbolIcon<'a> {
    look: &'a Look,
    symbol: Symbol,
}

impl<'a> SymbolIcon<'a> {
    /// The icon for `symbol`.
    pub fn new(look: &'a Look, symbol: Symbol) -> Self {
        Self { look, symbol }
    }
}

impl Widget for SymbolIcon<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = self.look.color(self.look.theme.colors.island);
        draw_icon(self.look, buf, area, area.x, area.y, self.symbol.into(), bg);
    }
}
