//! The components, as ratatui [`Widget`]s. They keep the GUI kit's names and options and
//! draw them the way NvChad, the Neovim distribution, does: flat full-bleed panes, a thin
//! separator line instead of gaps, a tabufline of buffers, a status line of colored
//! segments, Telescope-style prompts and floats.
//!
//! Icons are Nerd Font characters followed by a space: Nerd Font icons are wider than one
//! cell and draw into it.
//!
//! These widgets only draw. The interactive counterparts, which also handle keys and the
//! mouse, are in [`crate::controls`], [`crate::collections`], [`crate::popup`],
//! [`crate::workbench`] and [`crate::overlay`] and render through the widgets here.

use crate::glyphs::{self, CAP_LEFT, CAP_RIGHT};
use crate::{Glyph, Language, Look, Symbol};
use archipel_kit::ui::{ButtonVariant, Status};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use std::ops::Range;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Writes `text` at (`x`, `y`) clipped to `area`, returning the column after it.
pub(crate) fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, text: &str, style: Style) -> u16 {
    let area = area.intersection(buf.area);
    if y < area.top() || y >= area.bottom() || x >= area.right() || x < area.left() {
        return x;
    }
    let width = (area.right() - x) as usize;
    let (end, _) = buf.set_stringn(x, y, text, width, style);
    end
}

/// Fills cells `x..x + width` of row `y` with blanks in `style`, clipped to `area`.
pub(crate) fn fill(buf: &mut Buffer, area: Rect, x: u16, y: u16, width: u16, style: Style) {
    let area = area.intersection(buf.area);
    if y < area.top() || y >= area.bottom() {
        return;
    }
    for cx in x.max(area.left())..(x.saturating_add(width)).min(area.right()) {
        if let Some(cell) = buf.cell_mut((cx, y)) {
            cell.set_symbol(" ").set_style(style);
        }
    }
}

/// Fills every row of `rect` with blanks in `style`.
pub(crate) fn fill_rect(buf: &mut Buffer, rect: Rect, style: Style) {
    for y in rect.top()..rect.bottom() {
        fill(buf, rect, rect.x, y, rect.width, style);
    }
}

/// A title block as Telescope draws it: ` text ` in bold on `bg`. Returns the column after.
pub(crate) fn title_block(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    fg: Color,
    bg: Color,
) -> u16 {
    let style = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD);
    put(buf, area, x, y, &format!(" {text} "), style)
}

/// The cells `text` takes, with wide characters counted.
pub(crate) fn text_width(text: &str) -> u16 {
    text.width().min(u16::MAX as usize) as u16
}

/// `text` cut to `max` cells, with an ellipsis when it was cut.
pub(crate) fn clip(text: &str, max: u16) -> String {
    if text_width(text) <= max {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut used = 0u16;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0) as u16;
        if used + w + 1 > max {
            break;
        }
        out.push(ch);
        used += w;
    }
    if max > 0 {
        out.push('…');
    }
    out
}

/// `text` wrapped at spaces to lines of at most `width` cells; words longer than a line are
/// cut.
pub(crate) fn wrap_text(text: &str, width: u16) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let mut word = word.to_owned();
            while text_width(&word) > width {
                let mut head = String::new();
                for ch in word.chars() {
                    if text_width(&head) + ch.width().unwrap_or(0) as u16 > width {
                        break;
                    }
                    head.push(ch);
                }
                if head.is_empty() {
                    break;
                }
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                word = word[head.len()..].to_owned();
                lines.push(head);
            }
            if line.is_empty() {
                line = word;
            } else if text_width(&line) + 1 + text_width(&word) <= width {
                line.push(' ');
                line.push_str(&word);
            } else {
                lines.push(std::mem::take(&mut line));
                line = word;
            }
        }
        lines.push(line);
    }
    lines
}

/// What a [`Pane`] is, and so which NvChad surface it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PaneKind {
    /// The editor: `black`.
    #[default]
    Editor,
    /// A side bar tool window, like nvim-tree: `darker_black`.
    Sidebar,
    /// A bottom tool window, like a toggled terminal split: `darker_black`.
    Terminal,
    /// A float, like Telescope results or a hover window: `darker_black`.
    Float,
}

/// A pane: a flat, full-bleed background. Panes touch, with a [`Separator`] line only where
/// two have the same background. A title is a block on the first row, in the accent when
/// the pane has focus (like a Telescope title).
pub struct Pane<'a> {
    look: &'a Look,
    kind: PaneKind,
    title: Option<&'a str>,
    focused: bool,
}

impl<'a> Pane<'a> {
    /// An editor pane.
    pub fn new(look: &'a Look) -> Self {
        Self {
            look,
            kind: PaneKind::Editor,
            title: None,
            focused: false,
        }
    }

    /// The pane kind (its surface).
    pub fn kind(mut self, kind: PaneKind) -> Self {
        self.kind = kind;
        self
    }

    /// A title block on the first row.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Keyboard focus: an accent title block.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The background of a pane of `kind`.
    pub fn background(look: &Look, kind: PaneKind) -> Color {
        match kind {
            PaneKind::Editor => look.palette.black,
            PaneKind::Sidebar | PaneKind::Terminal | PaneKind::Float => look.palette.darker_black,
        }
    }

    /// The content area: below the title row, full width.
    pub fn inner(&self, area: Rect) -> Rect {
        if self.title.is_some() {
            Rect::new(
                area.x,
                area.y.saturating_add(1),
                area.width,
                area.height.saturating_sub(1),
            )
        } else {
            area
        }
    }
}

impl Widget for Pane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let bg = Self::background(self.look, self.kind);
        fill_rect(buf, area, Style::default().bg(bg).fg(p.white));
        if let Some(title) = self.title {
            let (fg, tbg) = if self.focused {
                (p.on(p.nord_blue), p.nord_blue)
            } else {
                (p.light_grey, p.one_bg)
            };
            title_block(buf, area, area.x + 1, area.y, title, fg, tbg);
        }
    }
}

/// The window separator line: `│` between side by side panes, `─` between stacked ones, in
/// the `line` color (NvChad's `WinSeparator`).
pub struct Separator<'a> {
    look: &'a Look,
    horizontal: bool,
    background: Option<Color>,
}

impl<'a> Separator<'a> {
    /// A vertical line, one cell wide.
    pub fn vertical(look: &'a Look) -> Self {
        Self {
            look,
            horizontal: false,
            background: None,
        }
    }

    /// A horizontal line, one cell high.
    pub fn horizontal(look: &'a Look) -> Self {
        Self {
            look,
            horizontal: true,
            background: None,
        }
    }

    /// The background behind the line (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }
}

impl Widget for Separator<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let style = Style::default()
            .fg(p.line)
            .bg(self.background.unwrap_or(p.black));
        let area = area.intersection(buf.area);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(if self.horizontal { "─" } else { "│" })
                        .set_style(style);
                }
            }
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
    pressed: bool,
    on: Option<Color>,
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
            pressed: false,
            on: None,
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

    /// The mouse is down on the button: a darker fill.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// The background the button sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }

    /// Width in cells, focus markers included.
    pub fn width(&self) -> u16 {
        let icon = if self.icon.is_some() { 2 } else { 0 };
        text_width(self.label) + icon + 4
    }
}

fn dim(color: Color, toward: Color, t: f32) -> Color {
    match (color, toward) {
        (Color::Rgb(r, g, b), Color::Rgb(r2, g2, b2)) => {
            let m = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
            Color::Rgb(m(r, r2), m(g, g2), m(b, b2))
        }
        _ => color,
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let under = self.on.unwrap_or(p.black);
        let (mut bg, fg) = match (self.variant, self.enabled) {
            (ButtonVariant::Primary, true) => (p.nord_blue, p.on(p.nord_blue)),
            (ButtonVariant::Danger, true) => (p.red, p.on(p.red)),
            (ButtonVariant::Secondary, true) => (p.one_bg2, p.white),
            (ButtonVariant::Ghost, true) => (under, p.blue),
            (ButtonVariant::Ghost, false) => (under, p.grey_fg),
            (_, false) => (p.black2, p.grey_fg),
        };
        if self.pressed && self.enabled {
            bg = dim(bg, p.black, 0.25);
        }
        let marker = Style::default()
            .fg(p.nord_blue)
            .bg(under)
            .add_modifier(Modifier::BOLD);
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

/// A mark icon and a label, on the cursor line color when `focused`.
#[allow(clippy::too_many_arguments)]
fn mark_row(
    look: &Look,
    buf: &mut Buffer,
    area: Rect,
    under: Color,
    mark: char,
    color: Color,
    label: &str,
    text: Color,
    focused: bool,
) {
    let p = &look.palette;
    let bg = if focused { p.black2 } else { under };
    // The switch icon is wider than the box and ring icons.
    let gap = if mark == glyphs::TOGGLE_ON || mark == glyphs::TOGGLE_OFF {
        3
    } else {
        2
    };
    let width = text_width(label) + gap + 2;
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

/// Width in cells of a mark row with `label`, as drawn by the choice components.
pub fn choice_width(label: &str) -> u16 {
    text_width(label) + 4
}

/// A checkbox: the box icon in the accent when checked, then the label.
pub struct JBCheckBox<'a> {
    look: &'a Look,
    checked: bool,
    mixed: bool,
    label: &'a str,
    focused: bool,
    enabled: bool,
    on: Option<Color>,
}

impl<'a> JBCheckBox<'a> {
    /// A checkbox showing `checked`.
    pub fn new(look: &'a Look, checked: bool, label: &'a str) -> Self {
        Self {
            look,
            checked,
            mixed: false,
            label,
            focused: false,
            enabled: true,
            on: None,
        }
    }

    /// Some, not all, children checked: a dash.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// Keyboard focus: the row on the cursor line color.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Disabled checkboxes are dimmed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The background the row sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }
}

impl Widget for JBCheckBox<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let (mark, color) = match (self.mixed, self.checked) {
            (true, _) => (glyphs::CHECKBOX_MIXED, p.nord_blue),
            (false, true) => (glyphs::CHECKBOX_ON, p.nord_blue),
            (false, false) => (glyphs::CHECKBOX_OFF, p.light_grey),
        };
        let (color, text) = if self.enabled {
            (color, p.white)
        } else {
            (p.grey_fg, p.grey_fg)
        };
        let under = self.on.unwrap_or(p.black);
        mark_row(
            self.look,
            buf,
            area,
            under,
            mark,
            color,
            self.label,
            text,
            self.focused,
        );
    }
}

/// A radio button: a filled ring when selected.
pub struct JBRadioButton<'a> {
    look: &'a Look,
    selected: bool,
    label: &'a str,
    focused: bool,
    enabled: bool,
    on: Option<Color>,
}

impl<'a> JBRadioButton<'a> {
    /// An option showing `selected`.
    pub fn new(look: &'a Look, selected: bool, label: &'a str) -> Self {
        Self {
            look,
            selected,
            label,
            focused: false,
            enabled: true,
            on: None,
        }
    }

    /// Keyboard focus: the row on the cursor line color.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Disabled options are dimmed.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The background the row sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }
}

impl Widget for JBRadioButton<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let (mark, color) = if self.selected {
            (glyphs::RADIO_ON, p.nord_blue)
        } else {
            (glyphs::RADIO_OFF, p.light_grey)
        };
        let (color, text) = if self.enabled {
            (color, p.white)
        } else {
            (p.grey_fg, p.grey_fg)
        };
        let under = self.on.unwrap_or(p.black);
        mark_row(
            self.look,
            buf,
            area,
            under,
            mark,
            color,
            self.label,
            text,
            self.focused,
        );
    }
}

/// An on/off switch icon, green when on, then the label.
pub struct OnOffButton<'a> {
    look: &'a Look,
    on: bool,
    label: &'a str,
    focused: bool,
    under: Option<Color>,
}

impl<'a> OnOffButton<'a> {
    /// A switch showing `on`.
    pub fn new(look: &'a Look, on: bool, label: &'a str) -> Self {
        Self {
            look,
            on,
            label,
            focused: false,
            under: None,
        }
    }

    /// Keyboard focus: the row on the cursor line color.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The background the row sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.under = Some(background);
        self
    }
}

impl Widget for OnOffButton<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let (mark, color) = if self.on {
            (glyphs::TOGGLE_ON, p.green)
        } else {
            (glyphs::TOGGLE_OFF, p.light_grey)
        };
        let under = self.under.unwrap_or(p.black);
        mark_row(
            self.look,
            buf,
            area,
            under,
            mark,
            color,
            self.label,
            p.white,
            self.focused,
        );
    }
}

/// A Telescope-style prompt: one row on the `black2` prompt background with the label as a
/// title block at the left (or a search icon), the value, and a caret while focused. The
/// block is red on error and blue while focused; the error message goes on the row below.
pub struct JBTextField<'a> {
    look: &'a Look,
    value: &'a str,
    placeholder: &'a str,
    label: Option<&'a str>,
    error: Option<&'a str>,
    focused: bool,
    caret: Option<usize>,
    selection: Option<Range<usize>>,
    clear_button: bool,
    on: Option<Color>,
}

impl<'a> JBTextField<'a> {
    /// A field showing `value`.
    pub fn new(look: &'a Look, value: &'a str) -> Self {
        Self {
            look,
            value,
            placeholder: "",
            label: None,
            error: None,
            focused: false,
            caret: None,
            selection: None,
            clear_button: false,
            on: None,
        }
    }

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// The label, in a title block at the left.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// A validation message, on the row below.
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    /// Keyboard focus: a blue block and a caret.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The caret as a character index into the value (default: the end).
    pub fn caret(mut self, caret: usize) -> Self {
        self.caret = Some(caret);
        self
    }

    /// The selected characters, as a character range.
    pub fn selection(mut self, selection: Option<Range<usize>>) -> Self {
        self.selection = selection;
        self
    }

    /// Shows a clear button at the right end while the value is not empty.
    pub fn clear_button(mut self, show: bool) -> Self {
        self.clear_button = show;
        self
    }

    /// The background the field sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }

    /// Rows the field needs: the field and, with an error, the message.
    pub fn height(&self) -> u16 {
        1 + u16::from(self.error.is_some())
    }

    /// The cell where the text starts, for a field at `area` (after the label or icon).
    pub fn text_start(&self, area: Rect) -> u16 {
        match self.label {
            Some(label) => area.x + 1 + text_width(label) + 2 + 1,
            None => area.x + 1 + 2,
        }
    }

    /// The clear button's cell for a field at `area`.
    pub fn clear_cell(area: Rect) -> u16 {
        area.right().saturating_sub(2)
    }
}

impl Widget for JBTextField<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let y = area.y;
        let row = Rect::new(area.x, y, area.width, 1.min(area.height));
        let field = p.black2;
        let mark = if self.error.is_some() {
            p.red
        } else if self.focused {
            p.nord_blue
        } else {
            p.light_grey
        };
        fill(buf, row, area.x, y, area.width, Style::default().bg(field));
        let mut x = area.x + 1;
        match self.label {
            Some(label) => {
                let (fg, bg) = if self.error.is_some() || self.focused {
                    (p.on(mark), mark)
                } else {
                    (p.white, p.one_bg3)
                };
                x = title_block(buf, row, x, y, label, fg, bg) + 1;
            }
            None => {
                let style = Style::default().bg(field).fg(mark);
                x = put(buf, row, x, y, &look.glyph(Glyph::Search), style) + 1;
            }
        }
        let end = if self.clear_button && !self.value.is_empty() {
            Self::clear_cell(area)
        } else {
            area.right().saturating_sub(1)
        };
        let inner = Rect::new(x, y, end.saturating_sub(x), 1.min(area.height));
        let text = Style::default().bg(field);
        if self.value.is_empty() {
            put(buf, inner, x, y, self.placeholder, text.fg(p.grey_fg));
            if self.focused {
                put(
                    buf,
                    inner,
                    x,
                    y,
                    " ",
                    text.add_modifier(Modifier::REVERSED).fg(p.white),
                );
            }
        } else {
            let chars: Vec<char> = self.value.chars().collect();
            let caret = self.caret.unwrap_or(chars.len()).min(chars.len());
            // Scroll so the caret is visible.
            let room = inner.width as usize;
            let start = field_scroll(&chars, caret, room);
            let mut widths: Vec<u16> = chars
                .iter()
                .map(|c| c.width().unwrap_or(0) as u16)
                .collect();
            widths.push(1);
            let mut cx = x;
            for (i, ch) in chars.iter().enumerate().skip(start) {
                let selected = self.selection.as_ref().is_some_and(|r| r.contains(&i));
                let style = if selected {
                    Style::default().bg(p.one_bg3).fg(p.white)
                } else {
                    text.fg(p.white)
                };
                cx = put(buf, inner, cx, y, ch.encode_utf8(&mut [0; 4]), style);
            }
            if self.focused {
                let before: u16 = widths[start..caret].iter().sum();
                let at = x + before;
                if at < inner.right() {
                    let ch = chars.get(caret).copied().unwrap_or(' ');
                    put(
                        buf,
                        inner,
                        at,
                        y,
                        ch.encode_utf8(&mut [0; 4]),
                        Style::default().fg(p.black2).bg(p.white),
                    );
                }
            }
        }
        if self.clear_button && !self.value.is_empty() {
            put(
                buf,
                row,
                Self::clear_cell(area),
                y,
                &look.glyph(Glyph::Close),
                text.fg(p.light_grey),
            );
        }
        if let Some(error) = self.error {
            let under = self.on.unwrap_or(p.black);
            let s = Style::default().fg(p.red).bg(under);
            fill(buf, area, area.x, y + 1, area.width, s);
            let x = put(buf, area, area.x + 1, y + 1, &look.glyph(Glyph::Error), s);
            put(buf, area, x + 1, y + 1, error, s);
        }
    }
}

/// The first visible character of a single-line field with the caret at `caret`, so that the
/// caret fits in `room` cells.
pub(crate) fn field_scroll(chars: &[char], caret: usize, room: usize) -> usize {
    let caret = caret.min(chars.len());
    let width = |i: usize| chars.get(i).map_or(1, |c| c.width().unwrap_or(0));
    let mut start = 0usize;
    while room > 0 && (start..=caret).map(width).sum::<usize>() > room && start < caret {
        start += 1;
    }
    start
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
    /// A folder in the folder color.
    Folder(bool),
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
            RowIcon::Glyph(Glyph::Folder) => {
                (glyphs::nerd_font(Glyph::Folder), look.palette.folder_bg)
            }
            RowIcon::Glyph(Glyph::FolderOpen) => {
                (glyphs::nerd_font(Glyph::FolderOpen), look.palette.folder_bg)
            }
            RowIcon::Glyph(glyph) => (glyphs::nerd_font(glyph), look.glyph_color(glyph)),
            RowIcon::Folder(open) => (
                glyphs::nerd_font(if open {
                    Glyph::FolderOpen
                } else {
                    Glyph::Folder
                }),
                look.palette.folder_bg,
            ),
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
pub(crate) fn draw_icon(
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

/// How a selected row is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RowStyle {
    /// A list or tree: the selected row on `one_bg3`, quieter on `black2` when the list
    /// has no focus (NvChad's cursor line).
    #[default]
    List,
    /// A completion or popup menu: the selected row is filled with the accent (`pmenu_bg`).
    Menu,
}

/// One row of a list or tree. The selected row is a full-width bar on the cursor line
/// color; in an unfocused list it is quieter. Matches of a speed search are bold and
/// underlined in the accent.
pub struct ColoredListCellRenderer<'a> {
    look: &'a Look,
    label: &'a str,
    secondary: Option<&'a str>,
    icon: Option<RowIcon>,
    depth: u16,
    expanded: Option<bool>,
    selected: bool,
    active: bool,
    hovered: bool,
    on: Option<Color>,
    style: RowStyle,
    highlights: &'a [Range<usize>],
    guides: bool,
    muted: bool,
}

impl<'a> ColoredListCellRenderer<'a> {
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
            hovered: false,
            on: None,
            style: RowStyle::List,
            highlights: &[],
            guides: false,
            muted: false,
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

    /// The pointer is over the row.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    /// The background the list sits on (default: the editor background; a side bar uses
    /// `darker_black`).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }

    /// The selected-row style.
    pub fn style(mut self, style: RowStyle) -> Self {
        self.style = style;
        self
    }

    /// Character ranges of the label to highlight (speed search matches).
    pub fn highlights(mut self, highlights: &'a [Range<usize>]) -> Self {
        self.highlights = highlights;
        self
    }

    /// Draws thin indent guides in the `line` color.
    pub fn guides(mut self, guides: bool) -> Self {
        self.guides = guides;
        self
    }

    /// Dims the row (a disabled or ignored item).
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

impl Widget for ColoredListCellRenderer<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let under = self.on.unwrap_or(p.black);
        let (bg, fg, sec) = match (self.style, self.selected, self.active) {
            (RowStyle::Menu, true, _) => (p.pmenu_bg, p.on(p.pmenu_bg), p.on(p.pmenu_bg)),
            (RowStyle::List, true, true) => (p.one_bg3, p.white, p.light_grey),
            (RowStyle::List, true, false) => (p.black2, p.white, p.grey_fg),
            (_, false, _) if self.hovered => (p.black2, p.white, p.grey_fg),
            _ => (
                under,
                if self.muted { p.grey_fg } else { p.white },
                p.grey_fg,
            ),
        };
        let base = Style::default().bg(bg);
        fill(buf, area, area.x, area.y, area.width, base);
        if self.guides {
            for level in 0..self.depth {
                put(
                    buf,
                    area,
                    area.x.saturating_add(1 + level * 2),
                    area.y,
                    "│",
                    base.fg(p.line),
                );
            }
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
                base.fg(p.light_grey),
            );
        }
        x = x.saturating_add(2);
        if let Some(icon) = self.icon {
            x = if self.style == RowStyle::Menu && self.selected {
                let (ch, _) = icon.char_and_color(look);
                let x = put(buf, area, x, area.y, &ch.to_string(), base.fg(fg));
                put(buf, area, x, area.y, " ", base)
            } else {
                draw_icon(look, buf, area, x, area.y, icon, bg)
            };
        }
        let weight = if self.selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        };
        for (i, ch) in self.label.chars().enumerate() {
            let hit = self.highlights.iter().any(|r| r.contains(&i));
            let style = if hit {
                base.fg(if self.style == RowStyle::Menu && self.selected {
                    fg
                } else {
                    p.orange
                })
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                base.fg(fg).add_modifier(weight)
            };
            x = put(buf, area, x, area.y, ch.encode_utf8(&mut [0; 4]), style);
        }
        if let Some(secondary) = self.secondary {
            put(buf, area, x + 1, area.y, secondary, base.fg(sec));
        }
    }
}

/// One tab of the [`JBTabs`] line.
#[derive(Debug, Clone, PartialEq)]
pub struct TabInfo {
    /// The label.
    pub label: String,
    /// An icon before the label.
    pub icon: Option<RowIconKey>,
    /// A modified buffer shows a dot instead of the close icon.
    pub modified: bool,
    /// Whether the close icon is shown.
    pub closable: bool,
    /// A pinned tab has no close icon and cannot be closed.
    pub pinned: bool,
}

/// A [`RowIcon`] that can be compared (for [`TabInfo`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowIconKey {
    /// A glyph.
    Glyph(Glyph),
    /// A language logo.
    Language(Language),
}

impl From<RowIconKey> for RowIcon {
    fn from(key: RowIconKey) -> Self {
        match key {
            RowIconKey::Glyph(g) => RowIcon::Glyph(g),
            RowIconKey::Language(l) => RowIcon::Language(l),
        }
    }
}

impl From<Language> for RowIconKey {
    fn from(l: Language) -> Self {
        RowIconKey::Language(l)
    }
}

impl From<Glyph> for RowIconKey {
    fn from(g: Glyph) -> Self {
        RowIconKey::Glyph(g)
    }
}

impl TabInfo {
    /// A closable tab labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            modified: false,
            closable: true,
            pinned: false,
        }
    }

    /// Sets the icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<RowIconKey>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Marks the buffer as modified.
    #[must_use]
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// Sets whether the tab can be closed.
    #[must_use]
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Pins the tab.
    #[must_use]
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Whether the tab can be closed by the user.
    pub fn can_close(&self) -> bool {
        self.closable && !self.pinned
    }

    fn width(&self) -> u16 {
        let icon = if self.icon.is_some() { 2 } else { 0 };
        2 + icon + text_width(&self.label) + 2 + 2
    }
}

/// Where each tab of a [`JBTabs`] line is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabsLayout {
    /// The cells of each visible tab.
    pub tabs: Vec<(usize, Rect)>,
    /// The cell of each visible tab's close icon, if it has one.
    pub closes: Vec<(usize, Rect)>,
    /// Whether tabs are scrolled out on the left.
    pub clipped_left: bool,
    /// Whether tabs are scrolled out on the right.
    pub clipped_right: bool,
}

/// The tab line of open buffers, as NvChad's tabufline: the selected buffer has the editor
/// background, so it joins the editor below, with bright bold text; the others are on the
/// cursor line color with dim text. Modified buffers show a dot where the close icon is.
pub struct JBTabs<'a> {
    look: &'a Look,
    tabs: &'a [TabInfo],
    selected: usize,
    offset: usize,
    hovered: Option<usize>,
    fill: Option<Color>,
    focused: bool,
}

impl<'a> JBTabs<'a> {
    /// `tabs` with `selected` active.
    pub fn new(look: &'a Look, tabs: &'a [TabInfo], selected: usize) -> Self {
        Self {
            look,
            tabs,
            selected,
            offset: 0,
            hovered: None,
            fill: None,
            focused: true,
        }
    }

    /// The first visible tab (the strip scrolls so the selected tab is visible; see
    /// [`JBTabs::scroll_to_selected`]).
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// The tab the pointer is over.
    pub fn hovered(mut self, hovered: Option<usize>) -> Self {
        self.hovered = hovered;
        self
    }

    /// The color behind the tabs and after the last one (default: `darker_black`).
    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }

    /// Whether the strip has keyboard focus: the selected label is underlined while it has.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The first tab to show so that `selected` is visible in `width` cells, starting from
    /// `offset`.
    pub fn scroll_to_selected(
        tabs: &[TabInfo],
        selected: usize,
        offset: usize,
        width: u16,
    ) -> usize {
        let mut offset = offset.min(tabs.len().saturating_sub(1));
        if selected < offset {
            return selected;
        }
        loop {
            let used: u16 = tabs
                .iter()
                .skip(offset)
                .take(selected.saturating_sub(offset) + 1)
                .map(TabInfo::width)
                .fold(0u16, u16::saturating_add);
            if used <= width || offset >= selected {
                return offset;
            }
            offset += 1;
        }
    }

    /// Where the tabs are in `area`.
    pub fn layout(&self, area: Rect) -> TabsLayout {
        let mut layout = TabsLayout {
            clipped_left: self.offset > 0,
            ..TabsLayout::default()
        };
        let mut x = area.x;
        for (index, tab) in self.tabs.iter().enumerate().skip(self.offset) {
            let width = tab.width();
            if x.saturating_add(width) > area.right() {
                layout.clipped_right = true;
                break;
            }
            let rect = Rect::new(x, area.y, width, 1.min(area.height));
            layout.tabs.push((index, rect));
            if tab.can_close() || tab.modified {
                layout.closes.push((
                    index,
                    Rect::new(x + width - 3, area.y, 2, 1.min(area.height)),
                ));
            }
            x += width;
        }
        layout
    }
}

impl Widget for JBTabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let fill_color = self.fill.unwrap_or(p.darker_black);
        fill(
            buf,
            area,
            area.x,
            area.y,
            area.width,
            Style::default().bg(fill_color),
        );
        let layout = self.layout(area);
        for (index, rect) in &layout.tabs {
            let tab = &self.tabs[*index];
            let selected = *index == self.selected;
            let hovered = self.hovered == Some(*index);
            let bg = if selected {
                p.black
            } else if hovered {
                p.one_bg
            } else {
                p.black2
            };
            fill(
                buf,
                *rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let mut cx = rect.x + 2;
            if let Some(icon) = tab.icon {
                cx = draw_icon(look, buf, *rect, cx, rect.y, icon.into(), bg);
            }
            let mut text =
                Style::default()
                    .bg(bg)
                    .fg(if selected { p.white } else { p.light_grey });
            if selected {
                text = text.add_modifier(Modifier::BOLD);
                if self.focused {
                    text = text.add_modifier(Modifier::UNDERLINED);
                }
            }
            cx = put(buf, *rect, cx, rect.y, &tab.label, text);
            let (mark, color) = if tab.modified {
                ("●".to_owned(), p.green)
            } else if tab.can_close() {
                (
                    look.glyph(Glyph::Close),
                    if selected || hovered {
                        p.red
                    } else {
                        p.grey_fg
                    },
                )
            } else if tab.pinned {
                (look.glyph(Glyph::Pin), p.light_grey)
            } else {
                (" ".to_owned(), p.grey_fg)
            };
            put(
                buf,
                *rect,
                cx + 2,
                rect.y,
                &mark,
                Style::default().bg(bg).fg(color),
            );
        }
        if layout.clipped_left {
            put(
                buf,
                area,
                area.x,
                area.y,
                "‹",
                Style::default().bg(p.one_bg3).fg(p.white),
            );
        }
        if layout.clipped_right && area.width > 0 {
            put(
                buf,
                area,
                area.right() - 1,
                area.y,
                "›",
                Style::default().bg(p.one_bg3).fg(p.white),
            );
        }
    }
}

/// A one-row message: an edge in the severity color, the icon and text on a tint, and the
/// actions as bold links on the right.
pub struct InlineBanner<'a> {
    look: &'a Look,
    severity: Status,
    text: &'a str,
    actions: &'a [&'a str],
    hovered: Option<usize>,
}

impl<'a> InlineBanner<'a> {
    /// A banner of `severity` saying `text`, with `actions`.
    pub fn new(look: &'a Look, severity: Status, text: &'a str, actions: &'a [&'a str]) -> Self {
        Self {
            look,
            severity,
            text,
            actions,
            hovered: None,
        }
    }

    /// The action the pointer is over.
    pub fn hovered(mut self, hovered: Option<usize>) -> Self {
        self.hovered = hovered;
        self
    }

    /// The cells of each action link for a banner at `area`.
    pub fn action_rects(&self, area: Rect) -> Vec<Rect> {
        let mut rects = vec![Rect::default(); self.actions.len()];
        let mut right = area.right().saturating_sub(1);
        for (i, action) in self.actions.iter().enumerate().rev() {
            let w = text_width(action);
            right = right.saturating_sub(w);
            rects[i] = Rect::new(right, area.y, w, 1.min(area.height));
            right = right.saturating_sub(2);
        }
        rects
    }

    fn colors(&self) -> (Color, Color, Glyph) {
        let p = &self.look.palette;
        match self.severity {
            Status::Info => (p.blue, p.blue, Glyph::Info),
            Status::Promo => (p.purple, p.purple, Glyph::Sparkle),
            Status::Success => (p.green, p.green, Glyph::Success),
            Status::Warning => (p.yellow, p.yellow, Glyph::Warning),
            Status::Error => (p.red, p.red, Glyph::Error),
        }
    }
}

impl Widget for InlineBanner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let (edge, icon_color, glyph) = self.colors();
        let base = Style::default().bg(p.black2).fg(p.white);
        fill(buf, area, area.x, area.y, area.width, base);
        put(
            buf,
            area,
            area.x,
            area.y,
            "▌",
            Style::default().fg(edge).bg(p.black2),
        );
        let x = put(
            buf,
            area,
            area.x + 2,
            area.y,
            &look.glyph(glyph),
            base.fg(icon_color),
        );
        put(buf, area, x + 1, area.y, self.text, base);
        for (i, (action, rect)) in self.actions.iter().zip(self.action_rects(area)).enumerate() {
            let mut link = base.fg(p.blue).add_modifier(Modifier::BOLD);
            if self.hovered == Some(i) {
                link = link.add_modifier(Modifier::UNDERLINED);
            }
            put(buf, area, rect.x, rect.y, action, link);
        }
    }
}

/// Options in a row separated by thin bars; the selected one is bold on the accent.
pub struct SegmentedButton<'a> {
    look: &'a Look,
    options: &'a [&'a str],
    selected: usize,
    on: Option<Color>,
    focused: bool,
}

impl<'a> SegmentedButton<'a> {
    /// `options` with `selected` active.
    pub fn new(look: &'a Look, options: &'a [&'a str], selected: usize) -> Self {
        Self {
            look,
            options,
            selected,
            on: None,
            focused: false,
        }
    }

    /// The background the control sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }

    /// Keyboard focus: the selected segment is accented.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The cells of each option for a control at `area`.
    pub fn option_rects(&self, area: Rect) -> Vec<Rect> {
        let mut x = area.x;
        let mut rects = Vec::new();
        for (index, option) in self.options.iter().enumerate() {
            if index > 0 {
                x += 1;
            }
            let w = text_width(option) + 2;
            rects.push(Rect::new(x, area.y, w, 1.min(area.height)));
            x += w;
        }
        rects
    }
}

impl Widget for SegmentedButton<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let under = self.on.unwrap_or(p.black);
        let rects = self.option_rects(area);
        for (index, (option, rect)) in self.options.iter().zip(rects).enumerate() {
            if index > 0 {
                put(
                    buf,
                    area,
                    rect.x.saturating_sub(1),
                    area.y,
                    "│",
                    Style::default().fg(p.line).bg(under),
                );
            }
            let style = if index == self.selected {
                let bg = if self.focused { p.nord_blue } else { p.one_bg3 };
                Style::default()
                    .bg(bg)
                    .fg(if self.focused { p.on(bg) } else { p.white })
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(under).fg(p.light_grey)
            };
            put(buf, area, rect.x, rect.y, &format!(" {option} "), style);
        }
    }
}

/// A progress bar: a heavy accent line on a light track, and the percentage.
pub struct ProgressBar<'a> {
    look: &'a Look,
    fraction: f32,
    label: bool,
    on: Option<Color>,
}

impl<'a> ProgressBar<'a> {
    /// A bar at `fraction` (0..=1).
    pub fn new(look: &'a Look, fraction: f32) -> Self {
        Self {
            look,
            fraction: if fraction.is_nan() {
                0.0
            } else {
                fraction.clamp(0.0, 1.0)
            },
            label: true,
            on: None,
        }
    }

    /// Whether to show the percentage after the bar (default `true`).
    pub fn label(mut self, label: bool) -> Self {
        self.label = label;
        self
    }

    /// The background the bar sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }
}

impl Widget for ProgressBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let under = self.on.unwrap_or(p.black);
        let percent = format!(" {:>3.0}%", self.fraction * 100.0);
        let width = if self.label {
            area.width.saturating_sub(text_width(&percent))
        } else {
            area.width
        };
        let filled = (f32::from(width) * self.fraction).round() as u16;
        let bar = "━".repeat(filled as usize);
        let x = put(
            buf,
            area,
            area.x,
            area.y,
            &bar,
            Style::default().fg(p.nord_blue).bg(under),
        );
        let rest = "─".repeat(width.saturating_sub(filled) as usize);
        let x = put(
            buf,
            area,
            x,
            area.y,
            &rest,
            Style::default().fg(p.grey).bg(under),
        );
        if self.label {
            put(
                buf,
                area,
                x,
                area.y,
                &percent,
                Style::default().fg(p.light_grey).bg(under),
            );
        }
    }
}

/// Keyboard shortcut caps.
pub struct Kbd<'a> {
    look: &'a Look,
    keys: &'a [&'a str],
    on: Option<Color>,
}

impl<'a> Kbd<'a> {
    /// Caps for `keys`.
    pub fn new(look: &'a Look, keys: &'a [&'a str]) -> Self {
        Self {
            look,
            keys,
            on: None,
        }
    }

    /// The background the caps sit on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }

    /// Width in cells.
    pub fn width(&self) -> u16 {
        self.keys
            .iter()
            .map(|key| text_width(key) + 3)
            .sum::<u16>()
            .saturating_sub(1)
    }
}

impl Widget for Kbd<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let cap = Style::default().bg(p.one_bg2).fg(p.white);
        let space = Style::default().bg(self.on.unwrap_or(p.black));
        let mut x = area.x;
        for key in self.keys {
            x = put(buf, area, x, area.y, &format!(" {key} "), cap);
            x = put(buf, area, x, area.y, " ", space);
        }
    }
}

/// A slot of the [`StatusBar`], for hit testing and click handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatusSlot {
    /// The mode block.
    Mode,
    /// The file name block.
    File,
    /// The branch and diff counts.
    Git,
    /// The centered message or progress text.
    Center,
    /// The diagnostics counts.
    Diagnostics,
    /// The language server or inspection profile.
    Lsp,
    /// The active run configuration and whether it runs.
    Run,
    /// The working directory block.
    Cwd,
    /// The cursor position block.
    Cursor,
}

/// Diagnostics counts of the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Diagnostics {
    /// Errors (red).
    pub errors: u32,
    /// Warnings (yellow).
    pub warnings: u32,
    /// Hints and weak warnings (purple).
    pub hints: u32,
    /// Information (green).
    pub info: u32,
}

/// Branch and change counts of the status bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitStatus {
    /// The branch name.
    pub branch: String,
    /// Added lines or files (green).
    pub added: u32,
    /// Changed (yellow).
    pub changed: u32,
    /// Removed (red).
    pub removed: u32,
}

/// The status line, as NvChad's default statusline: a mode block with a rounded end, the
/// file, the git branch and counts; the message centered; and on the right the
/// diagnostics, language server, working directory and cursor position, the last two as an
/// icon segment in a color plus a text segment. The kit's status bar widgets map onto these
/// slots: a `TextPresentation` is a slot's text, an `IconPresentation` a slot's icon.
pub struct StatusBar<'a> {
    look: &'a Look,
    mode: Option<(&'a str, Color)>,
    file: Option<(RowIcon, &'a str)>,
    git: Option<&'a GitStatus>,
    center: &'a str,
    diagnostics: Option<Diagnostics>,
    lsp: Option<&'a str>,
    run: Option<(&'a str, bool)>,
    cwd: Option<&'a str>,
    cursor: Option<&'a str>,
    hovered: Option<StatusSlot>,
}

impl<'a> StatusBar<'a> {
    /// An empty status line.
    pub fn new(look: &'a Look) -> Self {
        Self {
            look,
            mode: None,
            file: None,
            git: None,
            center: "",
            diagnostics: None,
            lsp: None,
            run: None,
            cwd: None,
            cursor: None,
            hovered: None,
        }
    }

    /// The mode block at the left: its label and color (see [`crate::Mode`]).
    pub fn mode(mut self, label: &'a str, color: Color) -> Self {
        self.mode = Some((label, color));
        self
    }

    /// The file block.
    pub fn file(mut self, icon: impl Into<RowIcon>, name: &'a str) -> Self {
        self.file = Some((icon.into(), name));
        self
    }

    /// The branch and counts.
    pub fn git(mut self, git: &'a GitStatus) -> Self {
        self.git = Some(git);
        self
    }

    /// The centered text (a message, a progress).
    pub fn center(mut self, text: &'a str) -> Self {
        self.center = text;
        self
    }

    /// The diagnostics counts.
    pub fn diagnostics(mut self, diagnostics: Diagnostics) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    /// The language server name.
    pub fn lsp(mut self, name: &'a str) -> Self {
        self.lsp = Some(name);
        self
    }

    /// The run configuration segment: the active configuration's name, or `running` while it
    /// runs (`true`). It replaces the IDE's toolbar run widget; clicking it is the caller's
    /// to answer (a picker of the configurations).
    pub fn run(mut self, name: &'a str, running: bool) -> Self {
        self.run = Some((name, running));
        self
    }

    /// The working directory name.
    pub fn cwd(mut self, name: &'a str) -> Self {
        self.cwd = Some(name);
        self
    }

    /// The cursor position text (`Ln 12, Col 8`).
    pub fn cursor(mut self, text: &'a str) -> Self {
        self.cursor = Some(text);
        self
    }

    /// The slot the pointer is over.
    pub fn hovered(mut self, hovered: Option<StatusSlot>) -> Self {
        self.hovered = hovered;
        self
    }

    fn diagnostics_text(d: &Diagnostics) -> String {
        format!(
            " {} {}  {} {}  {} {}  {} {} ",
            glyphs::nerd_font(Glyph::Error),
            d.errors,
            glyphs::nerd_font(Glyph::Warning),
            d.warnings,
            glyphs::nerd_font(Glyph::WeakWarning),
            d.hints,
            glyphs::nerd_font(Glyph::Info),
            d.info
        )
    }

    /// Where the slots are, for hit testing, as (slot, rectangle).
    ///
    /// Slots never overlap. When the bar is too narrow, the least important slots are
    /// dropped first, as NvChad does: the centered message, the language server, the
    /// diagnostics, the branch block, the working directory, the file name, and last the
    /// cursor position and the mode block.
    pub fn slots(&self, area: Rect) -> Vec<(StatusSlot, Rect)> {
        let y = area.y;
        let widths: Vec<(StatusSlot, u16)> = [
            (
                StatusSlot::Mode,
                self.mode.map(|(label, _)| text_width(label) + 5 + 1 + 1),
            ),
            (
                StatusSlot::Cursor,
                self.cursor.map(|t| 3 + text_width(t) + 2 + 1),
            ),
            (StatusSlot::File, self.file.map(|(_, n)| text_width(n) + 4)),
            (StatusSlot::Cwd, self.cwd.map(|t| 3 + text_width(t) + 2 + 1)),
            (
                StatusSlot::Run,
                self.run
                    .map(|(name, running)| text_width(if running { "running" } else { name }) + 5),
            ),
            (
                StatusSlot::Git,
                self.git.map(|g| text_width(&Self::git_text(g)) + 2),
            ),
            (
                StatusSlot::Diagnostics,
                self.diagnostics
                    .map(|d| text_width(&Self::diagnostics_text(&d))),
            ),
            (StatusSlot::Lsp, self.lsp.map(|t| text_width(t) + 10)),
        ]
        .into_iter()
        .filter_map(|(slot, w)| w.map(|w| (slot, w)))
        .collect();
        // Keep slots in priority order while they fit.
        let mut used = 0u16;
        let mut kept = Vec::new();
        for (slot, w) in widths {
            if used.saturating_add(w) <= area.width {
                used += w;
                kept.push((slot, w));
            }
        }
        let width_of = |slot: StatusSlot| kept.iter().find(|(s, _)| *s == slot).map(|(_, w)| *w);
        let mut out = Vec::new();
        let mut left = area.x;
        for slot in [StatusSlot::Mode, StatusSlot::File, StatusSlot::Git] {
            if let Some(w) = width_of(slot) {
                out.push((slot, Rect::new(left, y, w, 1)));
                left += w;
            }
        }
        let mut right = area.right();
        for slot in [
            StatusSlot::Cursor,
            StatusSlot::Cwd,
            StatusSlot::Run,
            StatusSlot::Lsp,
            StatusSlot::Diagnostics,
        ] {
            if let Some(w) = width_of(slot) {
                right -= w;
                out.push((slot, Rect::new(right, y, w, 1)));
            }
        }
        // The message takes what is left between the two sides, if it is readable there.
        let space = right.saturating_sub(left).saturating_sub(2);
        let center_w = text_width(self.center).min(space);
        if center_w > 0 && center_w >= 4.min(text_width(self.center)) {
            let cx = left + 1 + (space - center_w) / 2;
            out.push((StatusSlot::Center, Rect::new(cx, y, center_w, 1)));
        }
        out
    }

    fn git_text(git: &GitStatus) -> String {
        let mut text = format!(" {} {}", glyphs::nerd_font(Glyph::Branch), git.branch);
        for (mark, n) in [("+", git.added), ("~", git.changed), ("-", git.removed)] {
            if n > 0 {
                text.push_str(&format!(" {mark}{n}"));
            }
        }
        text
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }
        let bar = p.statusline_bg;
        let base = Style::default().bg(bar).fg(p.white);
        fill(buf, area, area.x, area.y, area.width, base);
        let slots = self.slots(area);
        let rect_of = |slot: StatusSlot| slots.iter().find(|(s, _)| *s == slot).map(|(_, r)| *r);
        let hover = |slot: StatusSlot| self.hovered == Some(slot);
        if let (Some((label, color)), Some(rect)) = (self.mode, rect_of(StatusSlot::Mode)) {
            let mut bg = color;
            if hover(StatusSlot::Mode) {
                bg = dim(bg, p.white, 0.15);
            }
            let block = Style::default()
                .bg(bg)
                .fg(p.on(color))
                .add_modifier(Modifier::BOLD);
            let x = put(
                buf,
                rect,
                rect.x,
                rect.y,
                &format!(" {} {label} ", look.glyph(Glyph::Terminal)),
                block,
            );
            put(
                buf,
                area,
                x,
                rect.y,
                &CAP_RIGHT.to_string(),
                Style::default().fg(bg).bg(bar),
            );
        }
        if let (Some((icon, name)), Some(rect)) = (self.file, rect_of(StatusSlot::File)) {
            let bg = if hover(StatusSlot::File) {
                p.lightbg
            } else {
                bar
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let x = draw_icon(look, buf, rect, rect.x + 1, rect.y, icon, bg);
            put(
                buf,
                rect,
                x,
                rect.y,
                name,
                Style::default().bg(bg).fg(p.white),
            );
        }
        if let (Some(git), Some(rect)) = (self.git, rect_of(StatusSlot::Git)) {
            let bg = if hover(StatusSlot::Git) {
                p.lightbg
            } else {
                bar
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let mut x = put(
                buf,
                rect,
                rect.x,
                rect.y,
                &format!(" {} {}", glyphs::nerd_font(Glyph::Branch), git.branch),
                Style::default().bg(bg).fg(p.purple),
            );
            for (mark, n, color) in [
                ("+", git.added, p.green),
                ("~", git.changed, p.yellow),
                ("-", git.removed, p.red),
            ] {
                if n > 0 {
                    x = put(
                        buf,
                        rect,
                        x,
                        rect.y,
                        &format!(" {mark}{n}"),
                        Style::default().bg(bg).fg(color),
                    );
                }
            }
        }
        if let Some(rect) = rect_of(StatusSlot::Center) {
            put(
                buf,
                rect,
                rect.x,
                rect.y,
                &clip(self.center, rect.width),
                Style::default().bg(bar).fg(p.light_grey),
            );
        }
        if let (Some(d), Some(rect)) = (self.diagnostics, rect_of(StatusSlot::Diagnostics)) {
            let bg = if hover(StatusSlot::Diagnostics) {
                p.lightbg
            } else {
                bar
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let mut x = rect.x + 1;
            for (icon, n, color) in [
                (glyphs::nerd_font(Glyph::Error), d.errors, p.red),
                (glyphs::nerd_font(Glyph::Warning), d.warnings, p.yellow),
                (glyphs::nerd_font(Glyph::WeakWarning), d.hints, p.purple),
                (glyphs::nerd_font(Glyph::Info), d.info, p.green),
            ] {
                let style = Style::default()
                    .bg(bg)
                    .fg(if n == 0 { p.grey_fg } else { color });
                x = put(buf, rect, x, rect.y, &format!("{icon} {n}"), style) + 2;
            }
        }
        if let (Some(name), Some(rect)) = (self.lsp, rect_of(StatusSlot::Lsp)) {
            let bg = if hover(StatusSlot::Lsp) {
                p.lightbg
            } else {
                bar
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            put(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                &format!("{} LSP ~ {name}", glyphs::nerd_font(Glyph::Settings)),
                Style::default().bg(bg).fg(p.green),
            );
        }
        if let (Some((name, running)), Some(rect)) = (self.run, rect_of(StatusSlot::Run)) {
            let bg = if hover(StatusSlot::Run) {
                p.lightbg
            } else {
                bar
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let (glyph, text, color) = if running {
                (Glyph::Stop, "running", p.yellow)
            } else {
                (Glyph::Run, name, p.green)
            };
            let x = put(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                &format!("{} ", glyphs::nerd_font(glyph)),
                Style::default().bg(bg).fg(color),
            );
            put(
                buf,
                rect,
                x,
                rect.y,
                text,
                Style::default()
                    .bg(bg)
                    .fg(if running { p.yellow } else { p.white }),
            );
        }
        for (slot, text, icon, color) in [
            (
                StatusSlot::Cwd,
                self.cwd,
                glyphs::nerd_font(Glyph::Folder),
                p.orange,
            ),
            (
                StatusSlot::Cursor,
                self.cursor,
                glyphs::nerd_font(Glyph::Code),
                p.nord_blue,
            ),
        ] {
            let (Some(text), Some(rect)) = (text, rect_of(slot)) else {
                continue;
            };
            let bg = if hover(slot) {
                dim(color, p.white, 0.15)
            } else {
                color
            };
            let cap = Style::default().fg(bg).bg(bar);
            let x = put(buf, rect, rect.x, rect.y, &CAP_LEFT.to_string(), cap);
            let x = put(
                buf,
                rect,
                x,
                rect.y,
                &format!("{icon} "),
                Style::default().bg(bg).fg(p.on(color)),
            );
            let x = put(buf, rect, x, rect.y, " ", Style::default().bg(p.lightbg));
            let x = put(
                buf,
                rect,
                x,
                rect.y,
                text,
                Style::default().bg(p.lightbg).fg(p.white),
            );
            put(
                buf,
                rect,
                x,
                rect.y,
                " ",
                Style::default().bg(p.lightbg).fg(p.white),
            );
        }
    }
}

/// A chip of text in a pill: tags and counters.
pub struct Chip<'a> {
    look: &'a Look,
    text: &'a str,
    color: Option<archipel_kit::egui::Color32>,
    accent: bool,
    on: Option<Color>,
}

impl<'a> Chip<'a> {
    /// A neutral tag.
    pub fn tag(look: &'a Look, text: &'a str) -> Self {
        Self {
            look,
            text,
            color: None,
            accent: false,
            on: None,
        }
    }

    /// A tag tinted with `color`.
    pub fn colored(look: &'a Look, text: &'a str, color: archipel_kit::egui::Color32) -> Self {
        Self {
            look,
            text,
            color: Some(color),
            accent: false,
            on: None,
        }
    }

    /// A counter (accent-filled when `accent`).
    pub fn counter(look: &'a Look, text: &'a str, accent: bool) -> Self {
        Self {
            look,
            text,
            color: None,
            accent,
            on: None,
        }
    }

    /// The background the chip sits on (default: the editor background).
    pub fn on(mut self, background: Color) -> Self {
        self.on = Some(background);
        self
    }

    /// Width in cells.
    pub fn width(&self) -> u16 {
        text_width(self.text) + 2
    }
}

impl Widget for Chip<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = &self.look.palette;
        let look = self.look;
        let under = self.on.unwrap_or(p.black);
        let (bg, fg) = match (self.color, self.accent) {
            (Some(color), _) => (
                look.color_over(color.gamma_multiply(0.22), look.theme.editor.background),
                look.readable(
                    u32::from(color.r()) << 16 | u32::from(color.g()) << 8 | u32::from(color.b()),
                ),
            ),
            (None, true) => (p.nord_blue, p.on(p.nord_blue)),
            (None, false) => (p.one_bg2, p.light_grey),
        };
        let cap = Style::default().fg(bg).bg(under);
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
        let bg = self.look.palette.black;
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
        let bg = self.look.palette.black;
        draw_icon(self.look, buf, area, area.x, area.y, self.symbol.into(), bg);
    }
}
