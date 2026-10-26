//! The bundled IntelliJ themes and editor color schemes: lookups over the generated tables.
//!
//! `generated/` is written by `tools/themes` from the data files of IntelliJ Community (see
//! `NOTICE.md` and `docs/theming.md`). This file holds the hand-written types and lookups the
//! generated code fills in.

use super::{Colors, EditorColors, Theme, generated};
use egui::Color32;

/// A packed `0xRRGGBBAA` color (alpha last, as in IntelliJ's theme files).
pub(super) fn px(rgba: u32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (rgba >> 24) as u8,
        (rgba >> 16) as u8,
        (rgba >> 8) as u8,
        rgba as u8,
    )
}

/// The resolved UI theme data of one IntelliJ UI theme: every UIManager key its theme
/// files (and parent theme files) define, sorted by key.
pub(super) struct UiData {
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,
    /// Id of the editor scheme the theme selects.
    pub scheme: &'static str,
    /// IntelliJ theme ids from the root parent to this theme.
    pub chain: &'static [&'static str],
    pub colors: &'static [(&'static str, u32)],
    pub numbers: &'static [(&'static str, f32)],
    pub flags: &'static [(&'static str, bool)],
    pub strings: &'static [(&'static str, &'static str)],
}

/// One UI theme: its data and the function that fills [`Colors`] from it.
pub(super) struct UiEntry {
    pub data: &'static UiData,
    pub colors: fn(Colors) -> Colors,
}

/// One text attribute as written in a scheme file.
pub(super) struct Attr {
    pub name: &'static str,
    pub fg: Option<u32>,
    pub bg: Option<u32>,
    /// 0 plain, 1 bold, 2 italic, 3 bold italic.
    pub font: u8,
    /// Effect type and color.
    pub effect: Option<(u8, Option<u32>)>,
    pub stripe: Option<u32>,
    /// `baseAttributes`: the attribute this one starts from.
    pub base: Option<&'static str>,
}

/// The data of one editor color scheme: its own colors and attributes (sorted by name);
/// everything else comes from `parent`.
pub(super) struct SchemeData {
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,
    pub parent: Option<&'static SchemeData>,
    pub colors: &'static [(&'static str, Option<u32>)],
    pub attributes: &'static [Attr],
    /// Fills [`EditorColors`] from the scheme.
    pub editor: fn(EditorColors) -> EditorColors,
}

/// A bundled UI theme, for pickers: `Theme::all()`.
#[derive(Debug, Clone, Copy)]
pub struct ThemeInfo {
    /// Stable id, e.g. `"islands_dark"`; also the name of the snapshot in the docs.
    pub id: &'static str,
    /// Display name, e.g. `"Islands Dark"`.
    pub name: &'static str,
    /// Whether the theme is dark.
    pub dark: bool,
    /// Builds the theme.
    pub constructor: fn() -> Theme,
}

/// Where a [`Theme`]'s generated tables come from: ids into the bundled IntelliJ themes
/// and editor schemes. A theme built by hand has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThemeSource {
    /// Id of the UI theme whose UIManager keys [`Theme::ui_color`] reads.
    pub ui: Option<&'static str>,
    /// Id of the editor scheme [`Theme::text_attributes`] reads.
    pub scheme: Option<&'static str>,
}

impl ThemeSource {
    /// The bundled UI theme `ui` with its editor scheme `scheme`.
    pub const fn new(ui: &'static str, scheme: &'static str) -> Self {
        Self {
            ui: Some(ui),
            scheme: Some(scheme),
        }
    }
}

/// How an editor effect is drawn, as IntelliJ names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    /// A box around the text.
    Boxed,
    /// A thin line under the text.
    LineUnderscore,
    /// A wavy line under the text (errors, warnings).
    WaveUnderscore,
    /// A line through the text (deprecated).
    Strikeout,
    /// A thick line under the text.
    BoldLineUnderscore,
    /// A thick dotted line under the text.
    BoldDottedLine,
    /// A search match box.
    SearchMatch,
    /// A box with rounded corners.
    RoundedBox,
}

impl EffectKind {
    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::LineUnderscore,
            2 => Self::WaveUnderscore,
            3 => Self::Strikeout,
            4 => Self::BoldLineUnderscore,
            5 => Self::BoldDottedLine,
            6 => Self::SearchMatch,
            7 => Self::RoundedBox,
            _ => Self::Boxed,
        }
    }
}

/// An effect of a text attribute: a kind and its color (the text color when `None`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    /// How it is drawn.
    pub kind: EffectKind,
    /// Its color.
    pub color: Option<Color32>,
}

/// A resolved text attribute of an editor scheme: what a syntax highlighter paints for one
/// `TextAttributesKey`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextAttributes {
    /// Text color.
    pub foreground: Option<Color32>,
    /// Background fill.
    pub background: Option<Color32>,
    /// Bold font style.
    pub bold: bool,
    /// Italic font style.
    pub italic: bool,
    /// Underline, wave, strikeout or box.
    pub effect: Option<Effect>,
    /// Mark color in the scrollbar's error stripe.
    pub stripe: Option<Color32>,
}

/// A bundled editor color scheme: `EditorScheme::all()` lists them, and
/// [`Theme::with_editor_scheme`] pairs one with any UI theme.
#[derive(Clone, Copy)]
pub struct EditorScheme(pub(super) &'static SchemeData);

impl std::fmt::Debug for EditorScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EditorScheme({})", self.0.id)
    }
}

impl PartialEq for EditorScheme {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for EditorScheme {}

impl EditorScheme {
    /// Every bundled scheme, in picker order.
    pub fn all() -> &'static [EditorScheme] {
        generated::SCHEMES
    }

    /// The scheme with this id (`"islands_dark"`, `"monokai"`, ...).
    pub fn by_id(id: &str) -> Option<EditorScheme> {
        Self::all().iter().copied().find(|scheme| scheme.id() == id)
    }

    /// Stable id.
    pub fn id(&self) -> &'static str {
        self.0.id
    }

    /// Display name.
    pub fn name(&self) -> &'static str {
        self.0.name
    }

    /// Whether the editor background is dark.
    pub fn dark(&self) -> bool {
        self.0.dark
    }

    /// The scheme this one inherits from (`parent_scheme`).
    pub fn parent(&self) -> Option<EditorScheme> {
        self.0.parent.map(EditorScheme)
    }

    /// The code editor colors of the scheme. Fields the scheme does not define keep the
    /// kit's measured values for its brightness (`Theme::dark()` / `Theme::light()`).
    pub fn editor_colors(&self) -> EditorColors {
        let base = if self.0.dark {
            Theme::dark().editor
        } else {
            Theme::light().editor
        };
        (self.0.editor)(base)
    }

    /// A named scheme color (`CARET_ROW_COLOR`, `SELECTION_BACKGROUND`, ...), inherited
    /// through the parent schemes. `None` when undefined or explicitly empty.
    pub fn color(&self, name: &str) -> Option<Color32> {
        let mut scheme = Some(self.0);
        while let Some(current) = scheme {
            if let Some((_, value)) = current.colors.iter().find(|(key, _)| *key == name) {
                return value.map(px);
            }
            scheme = current.parent;
        }
        None
    }

    /// The text attributes of a key (`DEFAULT_KEYWORD`, `ERRORS_ATTRIBUTES`, ...),
    /// resolved through `baseAttributes` and the parent schemes.
    pub fn attributes(&self, name: &str) -> Option<TextAttributes> {
        resolve_attributes(self.0, name, 0).map(|attr| TextAttributes {
            foreground: attr.fg.map(px),
            background: attr.bg.map(px),
            bold: attr.font & 1 != 0,
            italic: attr.font & 2 != 0,
            effect: attr.effect.map(|(kind, color)| Effect {
                kind: EffectKind::from_code(kind),
                color: color.map(px),
            }),
            stripe: attr.stripe.map(px),
        })
    }

    /// Names of all text attribute keys the scheme defines or inherits, sorted.
    pub fn attribute_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        let mut scheme = Some(self.0);
        while let Some(current) = scheme {
            names.extend(current.attributes.iter().map(|attr| attr.name));
            scheme = current.parent;
        }
        names.sort_unstable();
        names.dedup();
        names
    }
}

/// An attribute with `baseAttributes` applied, as plain numbers.
struct Resolved {
    fg: Option<u32>,
    bg: Option<u32>,
    font: u8,
    effect: Option<(u8, Option<u32>)>,
    stripe: Option<u32>,
}

fn resolve_attributes(scheme: &'static SchemeData, name: &str, depth: u8) -> Option<Resolved> {
    let mut current = Some(scheme);
    let attr = loop {
        let candidate = current?;
        if let Ok(index) = candidate
            .attributes
            .binary_search_by_key(&name, |attr| attr.name)
        {
            break &candidate.attributes[index];
        }
        current = candidate.parent;
    };
    let mut out = Resolved {
        fg: attr.fg,
        bg: attr.bg,
        font: attr.font,
        effect: attr.effect,
        stripe: attr.stripe,
    };
    if let (Some(base), true) = (attr.base, depth < 6)
        && let Some(inherited) = resolve_attributes(scheme, base, depth + 1)
    {
        out.fg = out.fg.or(inherited.fg);
        out.bg = out.bg.or(inherited.bg);
        if out.font == 0 {
            out.font = inherited.font;
        }
        out.effect = out.effect.or(inherited.effect);
        out.stripe = out.stripe.or(inherited.stripe);
    }
    Some(out)
}

fn ui_entry(id: &str) -> Option<&'static UiEntry> {
    generated::UI.iter().find(|entry| entry.data.id == id)
}

/// Builds the bundled theme `id`: its UIManager-derived [`Colors`] on the measured base of
/// its brightness, and the editor colors of the scheme it selects.
pub(super) fn build(id: &str) -> Theme {
    let Some(entry) = ui_entry(id) else {
        return Theme::dark();
    };
    let data = entry.data;
    let base = if data.dark {
        Theme::dark()
    } else {
        Theme::light()
    };
    let scheme = EditorScheme::by_id(data.scheme);
    Theme {
        name: data.name.into(),
        dark: data.dark,
        colors: (entry.colors)(base.colors),
        editor: scheme.map_or(base.editor, |scheme| scheme.editor_colors()),
        source: ThemeSource {
            ui: Some(data.id),
            scheme: scheme.map(|scheme| scheme.id()),
        },
        ..base
    }
}

fn sorted_find<'a, T: Copy>(table: &'a [(&'a str, T)], key: &str) -> Option<T> {
    table
        .binary_search_by_key(&key, |(name, _)| *name)
        .ok()
        .map(|index| table[index].1)
}

fn numbers(text: &str) -> Option<Vec<f32>> {
    text.split(',')
        .map(|part| part.trim().parse::<f32>().ok())
        .collect()
}

impl Theme {
    /// Every bundled theme for a picker: the two hand-measured Archipel themes first, then
    /// the IntelliJ Community UI themes.
    pub fn all() -> &'static [ThemeInfo] {
        generated::THEMES
    }

    /// The bundled theme with this id (see [`ThemeInfo::id`]).
    pub fn by_id(id: &str) -> Option<Theme> {
        Self::all()
            .iter()
            .find(|info| info.id == id)
            .map(|info| (info.constructor)())
    }

    /// This theme with the editor colors of `scheme`: any UI theme with any scheme.
    pub fn with_editor_scheme(mut self, scheme: EditorScheme) -> Self {
        let base = if scheme.dark() {
            Theme::dark().editor
        } else {
            Theme::light().editor
        };
        self.editor = (scheme.0.editor)(base);
        self.source.scheme = Some(scheme.id());
        self
    }

    /// The editor scheme the editor colors came from, if the theme was built from one.
    pub fn editor_scheme(&self) -> Option<EditorScheme> {
        self.source.scheme.and_then(EditorScheme::by_id)
    }

    fn ui_data(&self) -> Option<&'static UiData> {
        ui_entry(self.source.ui?).map(|entry| entry.data)
    }

    /// IntelliJ theme ids this theme's UI keys were merged from, root parent first
    /// (`["Darcula", "ExperimentalDark", "Islands Dark"]`); empty for a hand-built theme.
    pub fn ui_theme_chain(&self) -> &'static [&'static str] {
        self.ui_data().map_or(&[], |data| data.chain)
    }

    /// A color of the IntelliJ UI theme this theme was built from, by UIManager key
    /// (`"ToolWindow.Header.background"`), after parent themes, named colors and `*` keys
    /// are resolved. `None` for themes built by hand or keys the theme does not define.
    pub fn ui_color(&self, key: &str) -> Option<Color32> {
        sorted_find(self.ui_data()?.colors, key).map(px)
    }

    /// A numeric UI key such as `"Island.arc"` or `"Tree.rowHeight"` (geometry follows the
    /// Islands look for every theme for now; see `docs/theming.md`).
    pub fn ui_number(&self, key: &str) -> Option<f32> {
        sorted_find(self.ui_data()?.numbers, key)
    }

    /// A boolean UI key.
    pub fn ui_flag(&self, key: &str) -> Option<bool> {
        sorted_find(self.ui_data()?.flags, key)
    }

    /// An insets key (`"4,12,4,12"`) as `[top, left, bottom, right]`.
    pub fn ui_insets(&self, key: &str) -> Option<[f32; 4]> {
        let text = sorted_find(self.ui_data()?.strings, key)?;
        <[f32; 4]>::try_from(numbers(text)?).ok()
    }

    /// A size key (`"72,28"`) as `[width, height]`.
    pub fn ui_size(&self, key: &str) -> Option<[f32; 2]> {
        let text = sorted_find(self.ui_data()?.strings, key)?;
        <[f32; 2]>::try_from(numbers(text)?).ok()
    }

    /// The text attributes of an editor scheme key for syntax highlighting, from the
    /// scheme this theme's editor colors came from.
    pub fn text_attributes(&self, name: &str) -> Option<TextAttributes> {
        self.editor_scheme()?.attributes(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_keeps_alpha_last() {
        assert_eq!(
            px(0x11223344),
            Color32::from_rgba_unmultiplied(0x11, 0x22, 0x33, 0x44)
        );
    }

    #[test]
    fn numbers_parse_insets_and_sizes() {
        assert_eq!(numbers("4,12,4,12"), Some(vec![4.0, 12.0, 4.0, 12.0]));
        assert_eq!(numbers("72, 28"), Some(vec![72.0, 28.0]));
        assert_eq!(numbers("a,b"), None);
    }
}
