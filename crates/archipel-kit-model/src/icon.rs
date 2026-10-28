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
    /// A code symbol by the lowercase name of its kind (`class`, `method`, `field`, …), drawn
    /// in the symbol's color.
    Symbol(String),
    /// One or two characters drawn as the icon (a plugin tool window's stripe text).
    Text(String),
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

impl Avatar {
    /// The avatar of a project or user `name`: its initials (the first letters of the first
    /// two words, else the first two letters) and a stable color derived from the name.
    pub fn of(name: &str) -> Avatar {
        let words: Vec<&str> = name
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        let initials: String = match words.as_slice() {
            [] => "?".into(),
            [single] => single.chars().take(2).collect(),
            [first, second, ..] => first
                .chars()
                .take(1)
                .chain(second.chars().take(1))
                .collect(),
        };
        Avatar {
            initials: initials.to_uppercase(),
            rgb: avatar_rgb(name),
        }
    }
}

/// A stable color (`0xRRGGBB`) for a name: project and user badges.
pub fn avatar_rgb(name: &str) -> u32 {
    const COLORS: [u32; 9] = [
        0xE08855, 0xB08B14, 0xA1A359, 0x3B92B8, 0x3574F0, 0xC84D8F, 0x955AE0, 0x24A394, 0x5FAD65,
    ];
    let hash = name.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(u32::from(byte))
    });
    COLORS[hash as usize % COLORS.len()]
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
