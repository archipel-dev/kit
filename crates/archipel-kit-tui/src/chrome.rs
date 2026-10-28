//! Shared pieces of the chrome components ([`crate::toolbar::ToolbarControl`],
//! [`crate::menubar`], [`crate::stripe`], [`crate::header`], [`crate::tabbar`],
//! [`crate::statusbar::StatusItemsControl`], [`crate::breadcrumbs`]): the colors of the GUI's
//! tokens, icons from [`IconKey`], tooltips from the model, and the record of what a
//! component placed so that tests can read an outline back from the screen.

use crate::components::{put, text_width};
use crate::glyphs;
use crate::hit::Id;
use crate::runtime::Runtime;
use crate::tooltip::Tooltip;
use crate::{Glyph, Language, Look};
use archipel_kit::action_system::Os;
use archipel_kit::egui::Color32;
use archipel_kit_model::{IconKey, Notation, Outline, OutlineEntry, Tone};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

/// The shortcut notation of the operating system of `rt`.
pub fn notation(rt: &Runtime) -> Notation {
    notation_of(rt.caps.os)
}

/// The shortcut notation of `os`.
pub fn notation_of(os: Os) -> Notation {
    if os.is_mac() {
        Notation::Mac
    } else {
        Notation::Text
    }
}

/// The terminal tooltip of a model tooltip: the shortcut formatted for `notation`.
pub fn tooltip(tip: &archipel_kit_model::Tooltip, notation: Notation) -> Tooltip {
    let mut out = Tooltip::new(tip.title.clone());
    if let Some(shortcut) = &tip.shortcut {
        out = out.shortcut(shortcut.label(notation));
    }
    if let Some(description) = &tip.description {
        out = out.description(description.clone());
    }
    out
}

/// A child id for a model id string.
pub fn child(base: Id, key: &str) -> Id {
    Id::new(&format!("{}:{key}", base.0))
}

/// The GUI tokens as terminal colors over one surface.
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    /// The surface the component sits on.
    pub surface: Color32,
    /// Its terminal color.
    pub bg: Color,
}

impl Tokens {
    /// Tokens over `surface`.
    pub fn new(look: &Look, surface: Color32) -> Self {
        Self {
            surface,
            bg: look.over(surface, surface),
        }
    }

    /// `token` over this surface.
    pub fn of(&self, look: &Look, token: Color32) -> Color {
        look.over(token, self.surface)
    }

    /// The text color of `tone` over this surface.
    pub fn text(&self, look: &Look, tone: Tone) -> Color {
        look.tone(tone, self.surface)
    }

    /// The icon color of `tone`: the neutral icon gray for the default tone.
    pub fn icon(&self, look: &Look, tone: Tone) -> Color {
        match tone {
            Tone::Default => self.of(look, look.theme.colors.icon),
            other => self.text(look, other),
        }
    }

    /// A style of `fg` on this surface.
    pub fn style(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.bg)
    }
}

/// The character and color of an icon key over `tokens`' surface.
pub fn icon_cell(look: &Look, tokens: &Tokens, key: &IconKey, tone: Tone) -> (String, Color) {
    match key {
        IconKey::Glyph(glyph) => (look.glyph(*glyph), tokens.icon(look, tone)),
        IconKey::File(name) => {
            let file = name.rsplit('/').next().unwrap_or(name);
            if look.ascii {
                return (".".to_owned(), tokens.icon(look, tone));
            }
            match Language::from_file_name(file) {
                Some(language) => (
                    glyphs::language(language).to_string(),
                    look.readable(language.color()),
                ),
                None => (
                    glyphs::nerd_font(Glyph::File).to_string(),
                    tokens.icon(look, tone),
                ),
            }
        }
        IconKey::Symbol(name) => match symbol_kind(name) {
            Some(kind) if !look.ascii => (
                glyphs::symbol(kind).to_string(),
                look.readable(kind.color()),
            ),
            _ => ("o".to_owned(), tokens.icon(look, tone)),
        },
        IconKey::Text(text) => (text.chars().take(1).collect(), tokens.icon(look, tone)),
        IconKey::Folder { open } => {
            let glyph = if *open {
                Glyph::FolderOpen
            } else {
                Glyph::Folder
            };
            (
                look.glyph(glyph),
                tokens.of(look, look.theme.colors.folder_fill),
            )
        }
        _ => ("?".to_owned(), tokens.icon(look, tone)),
    }
}

/// The symbol kind named `name` (lowercase, as [`IconKey::Symbol`] carries it).
pub fn symbol_kind(name: &str) -> Option<crate::SymbolKind> {
    use crate::SymbolKind as K;
    Some(match name {
        "class" => K::Class,
        "interface" => K::Interface,
        "enum" => K::Enum,
        "record" => K::Record,
        "annotation" => K::Annotation,
        "struct" => K::Struct,
        "trait" => K::Trait,
        "object" => K::Object,
        "method" => K::Method,
        "function" => K::Function,
        "constructor" => K::Constructor,
        "field" => K::Field,
        "property" => K::Property,
        "variable" => K::Variable,
        "constant" => K::Constant,
        "parameter" => K::Parameter,
        "enummember" => K::EnumMember,
        "typeparameter" => K::TypeParameter,
        "lambda" => K::Lambda,
        "package" => K::Package,
        "module" => K::Module,
        _ => return None,
    })
}

/// Draws a badge dot at (`x`, `y`) in `tone`.
#[allow(clippy::too_many_arguments)]
pub fn badge(
    look: &Look,
    tokens: &Tokens,
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    tone: Tone,
    bg: Color,
) {
    let dot = if look.ascii { "*" } else { "●" };
    put(
        buf,
        area,
        x,
        y,
        dot,
        Style::default().fg(tokens.text(look, tone)).bg(bg),
    );
}

/// One thing a component drew, for hit testing and outlines.
#[derive(Debug, Clone)]
pub struct Placed {
    /// The model id.
    pub key: String,
    /// The widget id in the hit map.
    pub id: Id,
    /// Where it is on screen.
    pub rect: Rect,
    /// The outline entry the props describe.
    pub entry: OutlineEntry,
    /// The cells that hold the text when it is only part of `rect` (a dropdown without its
    /// icon and chevron).
    pub label_rect: Option<Rect>,
}

/// The text of `rect` in `buf`, trailing spaces trimmed.
pub fn text_at(buf: &Buffer, rect: Rect) -> String {
    let rect = rect.intersection(buf.area);
    let mut out = String::new();
    for y in rect.top()..rect.bottom() {
        for x in rect.left()..rect.right() {
            out.push_str(buf[(x, y)].symbol());
        }
    }
    out.trim().to_owned()
}

/// An [`Outline`] of what a component placed: entries with a visible text (`read`) take the
/// label from the screen cells, so an element that was not drawn, or drawn with other text,
/// differs from the props' outline.
pub fn outline_of(placed: &[Placed], buf: &Buffer, read: impl Fn(&Placed) -> bool) -> Outline {
    Outline {
        entries: placed
            .iter()
            .map(|p| {
                let mut entry = p.entry.clone();
                if read(p) {
                    entry.label = text_at(buf, p.label_rect.unwrap_or(p.rect));
                }
                entry
            })
            .collect(),
    }
}

/// `text` with the character at `underline` underlined, written at (`x`, `y`).
#[allow(clippy::too_many_arguments)]
pub fn put_mnemonic(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    underline: Option<usize>,
    style: Style,
) -> u16 {
    let mut cx = x;
    for (i, ch) in text.chars().enumerate() {
        let st = if underline == Some(i) {
            style.add_modifier(Modifier::UNDERLINED)
        } else {
            style
        };
        cx = put(buf, area, cx, y, ch.encode_utf8(&mut [0; 4]), st);
    }
    cx
}

/// The cells of `text`.
pub fn width(text: &str) -> u16 {
    text_width(text)
}
