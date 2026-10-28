use archipel_icons::Glyph;
use serde::{Deserialize, Serialize};

/// Which icon to draw. Never a codepoint and never a texture: each renderer maps the key
/// to its own drawing (the egui kit paints the glyph's vector shapes, the terminal kit
/// prints the Nerd Font character).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IconKey {
    /// A designed glyph of `archipel-icons`, stored by its kebab-case name.
    Glyph(#[serde(with = "glyph_name")] Glyph),
    /// The file icon of a path: a language logo or a symbol, chosen by the renderer from
    /// the file name.
    File(String),
    /// A folder.
    Folder {
        /// Whether it is shown expanded.
        open: bool,
    },
}

impl IconKey {
    /// The glyph, when this key names one.
    pub fn glyph(&self) -> Option<Glyph> {
        match self {
            IconKey::Glyph(glyph) => Some(*glyph),
            _ => None,
        }
    }
}

impl From<Glyph> for IconKey {
    fn from(glyph: Glyph) -> Self {
        IconKey::Glyph(glyph)
    }
}

/// A colored badge with initials: the project avatar of the main toolbar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Avatar {
    /// One or two letters.
    pub initials: String,
    /// The badge color as `0xRRGGBB`: data derived from the project name, not a theme color.
    pub rgb: u32,
}

mod glyph_name {
    use archipel_icons::Glyph;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(glyph: &Glyph, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(glyph.name())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Glyph, D::Error> {
        let name = String::deserialize(deserializer)?;
        Glyph::from_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown glyph `{name}`")))
    }
}
