//! The components, as ratatui [`Widget`]s. They keep the GUI kit's names, options and
//! colors and take the flat look of modern Neovim setups (NvChad and the like): blocks of
//! color without borders, title pills, a tab line of buffers, a status line with a mode
//! block, prompt-style inputs.
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

/// A pill at (`x`, `y`): `text` on `bg` between half-circle caps, on `under`. Returns the
/// column after it.
fn pill(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
    under: Color,
) -> u16 {
    let bg = style.bg.unwrap_or(under);
    let cap = Style::default().fg(bg).bg(under);
    let x = put(buf, area, x, y, &CAP_LEFT.to_string(), cap);
    let x = put(buf, area, x, y, text, style);
    put(buf, area, x, y, &CAP_RIGHT.to_string(), cap)
}

/// A panel: a flat block of the island color, no border. A title sits in a pill on the
/// first row, in the accent when the panel has focus (like a Telescope title). Place panels
/// one cell apart on a [`Frame`], as in the GUI's Islands layout.
pub struct Island<'a> {
    look: &'a Look,
    title: Option<&'a str>,
    focused: bool,
}

impl<'a> Island<'a> {
    /// An untitled panel.
    pub fn new(look: &'a Look) -> Self {
        Self {
            look,
            title: None,
            focused: false,
        }
    }

    /// A title pill on the first row.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Keyboard focus: an accent title pill.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The content area: one column of padding, below the title row and a blank row.
    pub fn inner(&self, area: Rect) -> Rect {
        let top = if self.title.is_some() { 2 } else { 1 };
        Rect::new(
            area.x + 1,
            area.y + top,
            area.width.saturating_sub(2),
            area.height.saturating_sub(top + 1),
        )
    }
}

impl Widget for Island<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        for y in area.top()..area.bottom() {
            fill(buf, area, area.x, y, area.width, look.body());
        }
        if let Some(title) = self.title {
            let style = if self.focused {
                Style::default()
                    .bg(look.color(c.accent))
                    .fg(look.color(c.text_on_accent))
            } else {
                Style::default()
                    .bg(look.color(c.border))
                    .fg(look.color(c.text_strong))
            };
            let text = format!(" {title} ");
            let island = look.color(c.island);
            pill(
                buf,
                area,
                area.x + 1,
                area.y,
                &text,
                style.add_modifier(Modifier::BOLD),
                island,
            );
        }
    }
}

/// Fills `area` with the frame color: the background behind panels and the status bar.
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

/// A push button: the label on a fill, one cell of padding each side. A `key` is
/// underlined in the label, the way terminal apps show mnemonics.
pub struct Button<'a> {
    look: &'a Look,
    label: &'a str,
    variant: ButtonVariant,
    icon: Option<Glyph>,
    key: Option<char>,
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
            key: None,
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

    /// The shortcut key, underlined where it first appears in the label.
    pub fn key(mut self, key: char) -> Self {
        self.key = Some(key);
        self
    }

    /// Keyboard focus: `❯` markers around the button.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Disabled buttons are dimmed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Width in cells, focus markers included.
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
            (ButtonVariant::Secondary, true) => (look.color(c.border), look.color(c.text_strong)),
            (ButtonVariant::Ghost, true) => (island, look.color(c.link)),
            (ButtonVariant::Ghost, false) => (island, look.color(c.text_disabled)),
            (_, false) => (look.color(c.border), look.color(c.text_placeholder)),
        };
        let marker = look.text(c.accent).add_modifier(Modifier::BOLD);
        let focus = if self.focused { "❯" } else { " " };
        put(buf, area, area.x, area.y, focus, marker);
        let style = Style::default().bg(bg).fg(fg);
        fill(buf, area, area.x + 1, area.y, self.width() - 2, style);
        let mut x = area.x + 2;
        if let Some(icon) = self.icon {
            let color = if self.enabled && self.variant == ButtonVariant::Ghost {
                look.glyph_color(icon)
            } else {
                fg
            };
            x = put(buf, area, x, area.y, &look.glyph(icon), style.fg(color)) + 1;
        }
        let key_at = self.key.and_then(|key| {
            self.label
                .char_indices()
                .find(|(_, ch)| ch.eq_ignore_ascii_case(&key))
                .map(|(i, _)| i)
        });
        for (i, ch) in self.label.char_indices() {
            let mut cell = style;
            if Some(i) == key_at && self.enabled {
                cell = cell.add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
            }
            x = put(buf, area, x, area.y, ch.encode_utf8(&mut [0; 4]), cell);
        }
        let focus = if self.focused { "❮" } else { " " };
        let end = area.x + self.width() - 1;
        put(buf, area, end, area.y, focus, marker);
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

    /// Keyboard focus: highlighted row.
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

/// A mark icon and a label, on the hover color when `focused`.
#[allow(clippy::too_many_arguments)]
fn mark_row(
    look: &Look,
    buf: &mut Buffer,
    area: Rect,
    mark: char,
    color: Color,
    label: &str,
    text: Color,
    focused: bool,
) {
    let c = &look.theme.colors;
    let bg = look.color(if focused { c.row_hover } else { c.island });
    // The switch icon is wider than the box and ring icons.
    let gap = if mark == glyphs::TOGGLE_ON || mark == glyphs::TOGGLE_OFF {
        3
    } else {
        2
    };
    let width = label.width() as u16 + gap + 2;
    fill(buf, area, area.x, area.y, width, Style::default().bg(bg));
    put(
        buf,
        area,
        area.x,
        area.y,
        &mark.to_string(),
        Style::default().fg(color).bg(bg),
    );
    put(
        buf,
        area,
        area.x + gap,
        area.y,
        label,
        Style::default().fg(text).bg(bg),
    );
}

impl Widget for Checkbox<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (mark, color) = match (self.mixed, self.checked) {
            (true, _) => (glyphs::CHECKBOX_MIXED, c.accent),
            (false, true) => (glyphs::CHECKBOX_ON, c.accent),
            (false, false) => (glyphs::CHECKBOX_OFF, c.text_secondary),
        };
        let (color, text) = if self.enabled {
            (color, c.text)
        } else {
            (c.text_disabled, c.text_disabled)
        };
        let (color, text) = (look.color(color), look.color(text));
        mark_row(look, buf, area, mark, color, self.label, text, self.focused);
    }
}

/// A radio button: a filled ring when selected.
pub struct Radio<'a> {
    look: &'a Look,
    selected: bool,
    label: &'a str,
    focused: bool,
    enabled: bool,
}

impl<'a> Radio<'a> {
    /// An option showing `selected`.
    pub fn new(look: &'a Look, selected: bool, label: &'a str) -> Self {
        Self {
            look,
            selected,
            label,
            focused: false,
            enabled: true,
        }
    }

    /// Keyboard focus: highlighted row.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
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
            (glyphs::RADIO_OFF, c.text_secondary)
        };
        let (color, text) = if self.enabled {
            (color, c.text)
        } else {
            (c.text_disabled, c.text_disabled)
        };
        let (color, text) = (look.color(color), look.color(text));
        mark_row(look, buf, area, mark, color, self.label, text, self.focused);
    }
}

/// An on/off switch icon, green when on, then the label.
pub struct Toggle<'a> {
    look: &'a Look,
    on: bool,
    label: &'a str,
    focused: bool,
}

impl<'a> Toggle<'a> {
    /// A switch showing `on`.
    pub fn new(look: &'a Look, on: bool, label: &'a str) -> Self {
        Self {
            look,
            on,
            label,
            focused: false,
        }
    }

    /// Keyboard focus: highlighted row.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for Toggle<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let (mark, color) = if self.on {
            (glyphs::TOGGLE_ON, c.toggle_on)
        } else {
            (glyphs::TOGGLE_OFF, c.text_secondary)
        };
        let (color, text) = (look.color(color), look.color(c.text));
        mark_row(look, buf, area, mark, color, self.label, text, self.focused);
    }
}

/// A prompt-style text input: one row on the field color with the label as a pill at the
/// left (or a search icon), the value, and a cursor while focused. The pill turns accent on
/// focus and red on error; the error message goes on the row below.
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

    /// The label, in a pill at the left.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// A validation message, on the row below.
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    /// Keyboard focus: accent pill and a cursor.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Rows the field needs: the field and, with an error, the message.
    pub fn height(&self) -> u16 {
        1 + u16::from(self.error.is_some())
    }
}

impl Widget for TextField<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let island = look.color(c.island);
        let field = look.color(c.row_hover);
        let y = area.y;
        let row = Rect::new(area.x, y, area.width, 1.min(area.height));
        let mark = if self.error.is_some() {
            c.error
        } else if self.focused {
            c.accent
        } else {
            c.text_secondary
        };
        // The field is a pill of the field color; the label pill sits inside its left end.
        let cap = Style::default().fg(field).bg(island);
        put(buf, row, area.x, y, &CAP_LEFT.to_string(), cap);
        fill(
            buf,
            row,
            area.x + 1,
            y,
            area.width.saturating_sub(2),
            Style::default().bg(field),
        );
        if area.width >= 2 {
            put(buf, row, area.right() - 1, y, &CAP_RIGHT.to_string(), cap);
        }
        let mut x = area.x + 1;
        match self.label {
            Some(label) => {
                let style = if self.error.is_some() || self.focused {
                    Style::default()
                        .bg(look.color(mark))
                        .fg(look.color(c.text_on_accent))
                } else {
                    Style::default()
                        .bg(look.color(c.border))
                        .fg(look.color(c.text_strong))
                };
                let text = format!(" {label} ");
                x = put(buf, row, x, y, &text, style.add_modifier(Modifier::BOLD)) + 1;
            }
            None => {
                let style = Style::default().bg(field).fg(look.color(mark));
                x = put(buf, row, x, y, &look.glyph(Glyph::Search), style) + 1;
            }
        }
        let inner = Rect::new(x, y, area.right().saturating_sub(x + 1), 1.min(area.height));
        let text = Style::default().bg(field);
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
            let end = put(
                buf,
                inner,
                x,
                y,
                self.value,
                text.fg(look.color(c.text_strong)),
            );
            if self.focused {
                put(buf, inner, end, y, "▏", text.fg(look.color(c.accent)));
            }
        }
        if let Some(error) = self.error {
            let x = put(
                buf,
                area,
                area.x + 1,
                y + 1,
                &look.glyph(Glyph::Error),
                look.text(c.error),
            );
            put(buf, area, x + 1, y + 1, error, look.text(c.error));
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

/// One row of a list or tree. The selected row is a full-width bar in the selection color;
/// in an unfocused list it is a quieter tint.
pub struct ListRow<'a> {
    look: &'a Look,
    label: &'a str,
    secondary: Option<&'a str>,
    icon: Option<RowIcon>,
    depth: u16,
    expanded: Option<bool>,
    selected: bool,
    active: bool,
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
            active: true,
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

    /// Whether the list has focus (default `true`).
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl Widget for ListRow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let bg = match (self.selected, self.active) {
            (true, true) => look.color(c.selection),
            (true, false) => look.color(c.row_hover),
            (false, _) => look.color(c.island),
        };
        let base = Style::default().bg(bg);
        fill(buf, area, area.x, area.y, area.width, base);
        if self.selected && self.active {
            put(
                buf,
                area,
                area.x,
                area.y,
                "▌",
                base.fg(look.color(c.accent)),
            );
        }
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

/// A tab line of open buffers, as in NvChad: tabs are blocks on the frame color; the
/// selected one has the island color, so it joins the editor below, bold text and a close
/// icon. Modified buffers show a dot.
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

    fn tab_width(icon: Option<RowIcon>, label: &str) -> u16 {
        let icon = if icon.is_some() { 2 } else { 0 };
        (2 + icon + label.width() + 2 + 1) as u16
    }
}

impl Widget for TabBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let frame = look.color_over(c.frame, c.frame);
        fill(
            buf,
            area,
            area.x,
            area.y,
            area.width,
            Style::default().bg(frame),
        );
        let mut x = area.x;
        for (index, (icon, label, modified)) in self.tabs.iter().enumerate() {
            let selected = index == self.selected;
            let width = Self::tab_width(*icon, label);
            let bg = if selected {
                look.color(c.island)
            } else {
                frame
            };
            fill(buf, area, x, area.y, width, Style::default().bg(bg));
            let mut cx = x + 2;
            if let Some(icon) = icon {
                cx = draw_icon(look, buf, area, cx, area.y, *icon, bg);
            }
            let mut text = Style::default().bg(bg).fg(look.color(if selected {
                c.text_strong
            } else {
                c.text_placeholder
            }));
            if selected {
                text = text.add_modifier(Modifier::BOLD);
            }
            cx = put(buf, area, cx, area.y, label, text);
            let (mark, color) = match (*modified, selected) {
                (true, _) => ("●".to_owned(), c.success),
                (false, true) => (look.glyph(Glyph::Close), c.error),
                (false, false) => (look.glyph(Glyph::Close), c.text_placeholder),
            };
            put(
                buf,
                area,
                cx + 1,
                area.y,
                &mark,
                Style::default().bg(bg).fg(look.color(color)),
            );
            x += width;
        }
    }
}

/// A one-row message: an edge in the severity color, the icon and text on a tint, and the
/// actions as bold links on the right.
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
        let base = Style::default().bg(bg).fg(look.color(c.text_strong));
        fill(buf, area, area.x, area.y, area.width, base);
        let color = look.glyph_color(glyph);
        put(buf, area, area.x, area.y, "▌", base.fg(color));
        let x = put(
            buf,
            area,
            area.x + 2,
            area.y,
            &look.glyph(glyph),
            base.fg(color),
        );
        put(buf, area, x + 1, area.y, self.text, base);
        let mut right = area.right().saturating_sub(1);
        for action in self.actions.iter().rev() {
            right = right.saturating_sub(action.width() as u16);
            let link = base.fg(look.color(c.link)).add_modifier(Modifier::BOLD);
            put(buf, area, right, area.y, action, link);
            right = right.saturating_sub(2);
        }
    }
}

/// Options in a row separated by thin bars; the selected one is bold on a tint.
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
        let mut x = area.x;
        for (index, option) in self.options.iter().enumerate() {
            if index > 0 {
                x = put(buf, area, x, area.y, "│", look.text(c.border));
            }
            let style = if index == self.selected {
                Style::default()
                    .bg(look.color(c.segment_selected_border))
                    .fg(look.color(c.text_strong))
                    .add_modifier(Modifier::BOLD)
            } else {
                look.text(c.text_secondary)
            };
            x = put(buf, area, x, area.y, &format!(" {option} "), style);
        }
    }
}

/// A progress bar: a heavy accent line on a light track, and the percentage.
pub struct ProgressBar<'a> {
    look: &'a Look,
    fraction: f32,
    label: bool,
}

impl<'a> ProgressBar<'a> {
    /// A bar at `fraction` (0..=1).
    pub fn new(look: &'a Look, fraction: f32) -> Self {
        Self {
            look,
            fraction: fraction.clamp(0.0, 1.0),
            label: true,
        }
    }

    /// Whether to show the percentage after the bar (default `true`).
    pub fn label(mut self, label: bool) -> Self {
        self.label = label;
        self
    }
}

impl Widget for ProgressBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let percent = format!(" {:>3.0}%", self.fraction * 100.0);
        let width = if self.label {
            area.width.saturating_sub(percent.width() as u16)
        } else {
            area.width
        };
        let filled = (width as f32 * self.fraction).round() as u16;
        let bar = "━".repeat(filled as usize);
        let x = put(buf, area, area.x, area.y, &bar, look.text(c.accent));
        let rest = "─".repeat((width - filled) as usize);
        let x = put(buf, area, x, area.y, &rest, look.text(c.border));
        if self.label {
            put(buf, area, x, area.y, &percent, look.text(c.text_secondary));
        }
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
        let cap = Style::default()
            .bg(self.look.color(c.border))
            .fg(self.look.color(c.text_strong));
        let mut x = area.x;
        for key in self.keys {
            x = put(buf, area, x, area.y, &format!(" {key} "), cap) + 1;
        }
    }
}

/// The status line, as in NvChad: a mode block in the accent ending in a half circle, key
/// hints on the frame color, and on the right a status block (branch, position) behind a
/// half circle.
pub struct StatusBar<'a> {
    look: &'a Look,
    mode: Option<&'a str>,
    hints: &'a [(&'a str, &'a str)],
    right: &'a str,
}

impl<'a> StatusBar<'a> {
    /// A status line with `hints` as (key, action).
    pub fn new(look: &'a Look, hints: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            look,
            mode: None,
            hints,
            right: "",
        }
    }

    /// The mode block at the left (`NORMAL`, `PROJECT`), in the accent.
    pub fn mode(mut self, mode: &'a str) -> Self {
        self.mode = Some(mode);
        self
    }

    /// The status block at the right edge (branch, position, encoding).
    pub fn right(mut self, text: &'a str) -> Self {
        self.right = text;
        self
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let c = &self.look.theme.colors;
        let look = self.look;
        let frame = look.color_over(c.frame, c.frame);
        let base = Style::default().bg(frame).fg(look.color(c.text_secondary));
        fill(buf, area, area.x, area.y, area.width, base);
        let mut x = area.x;
        if let Some(mode) = self.mode {
            let accent = look.color(c.accent);
            let block = Style::default()
                .bg(accent)
                .fg(look.color(c.text_on_accent))
                .add_modifier(Modifier::BOLD);
            x = put(
                buf,
                area,
                x,
                area.y,
                &format!(" {} {mode} ", look.glyph(Glyph::Terminal)),
                block,
            );
            x = put(
                buf,
                area,
                x,
                area.y,
                &CAP_RIGHT.to_string(),
                Style::default().fg(accent).bg(frame),
            ) + 1;
        }
        for (key, action) in self.hints {
            let key_style = base
                .fg(look.color(c.text_strong))
                .add_modifier(Modifier::BOLD);
            x = put(buf, area, x, area.y, key, key_style) + 1;
            x = put(buf, area, x, area.y, action, base) + 2;
        }
        if !self.right.is_empty() {
            let block_bg = look.color(c.border);
            let width = self.right.width() as u16 + 3;
            let start = area.right().saturating_sub(width);
            if start > x {
                let cap = Style::default().fg(block_bg).bg(frame);
                let cx = put(buf, area, start, area.y, &CAP_LEFT.to_string(), cap);
                let block = Style::default().bg(block_bg).fg(look.color(c.text_strong));
                put(buf, area, cx, area.y, &format!(" {} ", self.right), block);
            }
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
        let cap = Style::default().fg(bg).bg(look.color(c.island));
        let x = put(buf, area, area.x, area.y, &CAP_LEFT.to_string(), cap);
        let x = put(
            buf,
            area,
            x,
            area.y,
            self.text,
            Style::default().bg(bg).fg(fg),
        );
        put(buf, area, x, area.y, &CAP_RIGHT.to_string(), cap);
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
