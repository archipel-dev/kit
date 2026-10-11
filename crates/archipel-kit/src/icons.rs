//! Icons: the designed glyphs of the `archipel-icons` kit, plus file-type icons.
//!
//! [`Glyph`]s are the general icon set and [`Language`]s the language tiles (see
//! `docs/icons/`). [`Icon`] adds what the kit draws from the theme: Java type circles and
//! folders tinted by their role. [`Icon::for_file`] picks an icon for a file name. All
//! drawings are original (no third-party artwork).

use crate::theme::{Colors, Theme};
pub use archipel_icons::{Category, Glyph, Language};
use archipel_icons::{Paint, Part, Shape, Tone};
use egui::{self, Color32, Rect, vec2};

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
const SOURCE_ROOT: Color32 = rgb(0x54, 0x8A, 0xF7);
const TEST_ROOT: Color32 = rgb(0x5F, 0xAD, 0x65);
const RESOURCES_ROOT: Color32 = rgb(0xB1, 0x8A, 0xDB);
const EXCLUDED: Color32 = rgb(0xD0, 0x8A, 0x4B);
const IMAGE: Color32 = rgb(0x5F, 0xAD, 0x65);
const ARCHIVE: Color32 = rgb(0xD9, 0xB0, 0x4C);
const HIDDEN: Color32 = rgb(0x6C, 0x70, 0x7E);

/// What a Java source file declares (the letter in its circle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// `class`: C.
    Class,
    /// `interface`: I.
    Interface,
    /// `enum`: E.
    Enum,
    /// `record`: R.
    Record,
    /// `@interface`: @.
    Annotation,
}

impl TypeKind {
    fn letter(self) -> &'static str {
        match self {
            TypeKind::Class => "C",
            TypeKind::Interface => "I",
            TypeKind::Enum => "E",
            TypeKind::Record => "R",
            TypeKind::Annotation => "@",
        }
    }

    fn color(self) -> Color32 {
        match self {
            TypeKind::Class => rgb(0x4F, 0x9B, 0xE0),
            TypeKind::Interface => rgb(0x5F, 0xAD, 0x65),
            TypeKind::Enum => rgb(0xE0, 0x9A, 0x55),
            TypeKind::Record => rgb(0x4F, 0xB8, 0xC8),
            TypeKind::Annotation => rgb(0xB1, 0x8A, 0xDB),
        }
    }

    /// Cheap peek at the first top-level declaration of a Java source (comments,
    /// strings and nested braces skipped): `class` when nothing is recognized.
    pub fn peek_java(source: &str) -> TypeKind {
        let bytes = source.as_bytes();
        let (mut i, mut depth) = (0, 0usize);
        let mut previous_at = false;
        while i < bytes.len() {
            match bytes[i] {
                b'/' if bytes.get(i + 1) == Some(&b'/') => {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                }
                b'/' if bytes.get(i + 1) == Some(&b'*') => {
                    i += 2;
                    while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    i += 2;
                }
                b'"' | b'\'' => {
                    let quote = bytes[i];
                    i += 1;
                    while i < bytes.len() && bytes[i] != quote && bytes[i] != b'\n' {
                        i += if bytes[i] == b'\\' { 2 } else { 1 };
                    }
                    i += 1;
                }
                b'{' => {
                    depth += 1;
                    i += 1;
                }
                b'}' => {
                    depth = depth.saturating_sub(1);
                    i += 1;
                }
                b'@' => {
                    previous_at = true;
                    i += 1;
                }
                c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' => {
                    let start = i;
                    while i < bytes.len()
                        && (bytes[i].is_ascii_alphanumeric()
                            || bytes[i] == b'_'
                            || bytes[i] == b'$')
                    {
                        i += 1;
                    }
                    let word = &source[start..i];
                    if depth == 0 {
                        let rest = source[i..].trim_start();
                        let named = rest.starts_with(|c: char| c.is_alphabetic() || c == '_');
                        match word {
                            "interface" if previous_at => return TypeKind::Annotation,
                            "interface" => return TypeKind::Interface,
                            "enum" if named => return TypeKind::Enum,
                            "class" if named => return TypeKind::Class,
                            "record" if named => {
                                // `record Name(` or `record Name<T>(` (contextual keyword).
                                let after = rest.trim_start_matches(|c: char| {
                                    c.is_alphanumeric() || c == '_' || c == '$'
                                });
                                if after.starts_with(['(', '<']) {
                                    return TypeKind::Record;
                                }
                            }
                            _ => {}
                        }
                    }
                    previous_at = false;
                }
                c => {
                    if !c.is_ascii_whitespace() {
                        previous_at = false;
                    }
                    i += 1;
                }
            }
        }
        TypeKind::Class
    }
}

/// What a folder is for, shown by its colour like the reference IDE's project view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderKind {
    /// An ordinary folder.
    Plain,
    /// `src/main/java`, `src/main/kotlin`: blue.
    Source,
    /// `src/test/java`, `src/androidTest/kotlin`: green.
    Test,
    /// `src/*/resources`, `src/*/res`.
    Resources,
    /// `build`, `target`, `out`, `node_modules`: orange.
    Excluded,
    /// `.git`, `.idea`, `.gradle`: dimmed.
    Hidden,
    /// A directory inside a source root (a Java package).
    Package,
}

impl FolderKind {
    fn fill(self, colors: &Colors) -> Color32 {
        match self {
            FolderKind::Plain | FolderKind::Package => colors.folder_fill,
            FolderKind::Source => SOURCE_ROOT.gamma_multiply(0.45),
            FolderKind::Test => TEST_ROOT.gamma_multiply(0.45),
            FolderKind::Resources => RESOURCES_ROOT.gamma_multiply(0.45),
            FolderKind::Excluded => EXCLUDED.gamma_multiply(0.45),
            FolderKind::Hidden => Color32::TRANSPARENT,
        }
    }

    fn outline(self) -> Option<Color32> {
        match self {
            FolderKind::Plain | FolderKind::Package => None,
            FolderKind::Source => Some(SOURCE_ROOT),
            FolderKind::Test => Some(TEST_ROOT),
            FolderKind::Resources => Some(RESOURCES_ROOT),
            FolderKind::Excluded => Some(EXCLUDED),
            FolderKind::Hidden => Some(HIDDEN),
        }
    }
}

const IMAGE_EXTENSIONS: [&str; 8] = ["png", "jpg", "jpeg", "svg", "webp", "gif", "ico", "bmp"];
const ARCHIVE_EXTENSIONS: [&str; 7] = ["jar", "zip", "tar", "gz", "tgz", "war", "aar"];

/// Anything the kit can draw as an icon: a [`Glyph`], a [`Language`] tile, or a file-type
/// icon drawn from the theme.
///
/// Every component that takes an icon accepts `impl Into<Icon>`, so pass a glyph or a
/// language directly: `IconButton::new(Glyph::Refresh, "Reload")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// A designed glyph from `archipel-icons`.
    Glyph(Glyph),
    /// A language tile from `archipel-icons`.
    Language(Language),
    /// A Java type: a filled circle with C, I, E, R or @.
    JavaType(TypeKind),
    /// A folder tinted by its role.
    FolderKind(FolderKind),
    /// An image file (green picture).
    FileImage,
    /// An archive file (amber box).
    FileArchive,
}

impl From<Glyph> for Icon {
    fn from(glyph: Glyph) -> Self {
        Icon::Glyph(glyph)
    }
}

impl From<Language> for Icon {
    fn from(language: Language) -> Self {
        Icon::Language(language)
    }
}

impl Icon {
    /// File icon by name: Java sources show their type circle, other languages their tile
    /// (by exact name, then the longest extension), images and archives their glyph, and
    /// everything else a text page.
    pub fn for_file(name: &str) -> Icon {
        let lower = name.to_ascii_lowercase();
        let has_ext = |ext: &str| {
            lower
                .strip_suffix(ext)
                .is_some_and(|rest| rest.ends_with('.') && rest.len() > 1)
        };
        if has_ext("java") {
            return Icon::JavaType(TypeKind::Class);
        }
        if let Some(language) = Language::from_file_name(name) {
            return Icon::Language(language);
        }
        if IMAGE_EXTENSIONS.iter().any(|e| has_ext(e)) {
            return Icon::FileImage;
        }
        if ARCHIVE_EXTENSIONS.iter().any(|e| has_ext(e)) {
            return Icon::FileArchive;
        }
        if has_ext("lock") {
            return Icon::Glyph(Glyph::Lock);
        }
        Icon::Glyph(Glyph::FileText)
    }

    /// Like [`Icon::for_file`], peeking into Java sources for the kind of their type.
    pub fn for_source(name: &str, text: Option<&str>) -> Icon {
        match (Icon::for_file(name), text) {
            (Icon::JavaType(_), Some(text)) => Icon::JavaType(TypeKind::peek_java(text)),
            (icon, _) => icon,
        }
    }
}

/// Semantic tones of the icon kit, from `theme`.
pub fn tones(theme: &Theme) -> archipel_icons::Tones {
    let c = &theme.colors;
    archipel_icons::Tones {
        knockout: hex(c.island),
        error: hex(c.error),
        warning: hex(c.warning),
        muted: hex(theme.editor.weak_warning_effect),
        success: hex(c.success),
        info: hex(c.link),
        run: hex(c.run),
        stop: hex(c.stop),
        ink: hex(c.icon),
    }
}

fn hex(c: Color32) -> u32 {
    u32::from(c.r()) << 16 | u32::from(c.g()) << 8 | u32::from(c.b())
}

/// Paints `icon` into `rect` (scaled from its 16×16 design grid) in `color`. Status
/// glyphs, language tiles and file-type icons keep their own colors.
pub fn paint(
    painter: &egui::Painter,
    rect: Rect,
    icon: impl Into<Icon>,
    color: impl Into<Color32>,
) {
    let color = color.into();
    let theme = Theme::current(painter.ctx());
    let tones = tones(&theme);
    match icon.into() {
        Icon::Glyph(glyph) => archipel_icons::paint(painter, rect, glyph, color, &tones),
        Icon::Language(language) => archipel_icons::paint_language(painter, rect, language),
        Icon::FileImage => archipel_icons::paint(painter, rect, Glyph::Image, IMAGE, &tones),
        Icon::FileArchive => archipel_icons::paint(painter, rect, Glyph::Archive, ARCHIVE, &tones),
        Icon::JavaType(kind) => {
            let mut parts = vec![Part {
                shape: Shape::Circle {
                    center: (8.0, 8.0),
                    radius: 6.5,
                },
                paint: Paint::Fill(Tone::Rgb(hex(kind.color()))),
            }];
            parts.extend(archipel_icons::label(
                kind.letter(),
                (8.0, 8.0),
                Tone::Knockout,
            ));
            archipel_icons::paint_parts(painter, rect, &parts, color, &tones);
        }
        Icon::FolderKind(kind) => paint_folder(painter, rect, kind, color, &theme.colors, &tones),
    }
}

/// A folder filled and outlined by its role, on the [`Glyph::Folder`] outline.
fn paint_folder(
    painter: &egui::Painter,
    rect: Rect,
    kind: FolderKind,
    color: Color32,
    colors: &Colors,
    tones: &archipel_icons::Tones,
) {
    let outline = kind.outline().unwrap_or(color);
    let fill = Tone::Rgb(hex(kind.fill(colors)));
    let mut parts = Vec::new();
    if kind != FolderKind::Hidden {
        // Filled as a tab and a body: egui fills convex polygons only.
        parts.push(Part {
            shape: Shape::Polygon(&[(1.5, 3.0), (6.0, 3.0), (7.5, 5.0), (1.5, 5.0)]),
            paint: Paint::Fill(fill),
        });
        parts.push(Part {
            shape: Shape::Rect {
                min: (1.5, 5.0),
                max: (14.5, 13.0),
                radius: 0.0,
            },
            paint: Paint::Fill(fill),
        });
    }
    parts.extend(Glyph::Folder.parts().iter().map(|part| Part {
        paint: Paint::Stroke {
            width: 1.0,
            tone: Tone::Rgb(hex(outline)),
        },
        ..*part
    }));
    if kind == FolderKind::Package {
        parts.push(Part {
            shape: Shape::Circle {
                center: (8.0, 9.0),
                radius: 1.6,
            },
            paint: Paint::Fill(Tone::Ink),
        });
    }
    archipel_icons::paint_parts(painter, rect, &parts, color, tones);
}

/// Allocates a square icon of `size` points and paints it.
pub fn show(
    ui: &mut egui::Ui,
    icon: impl Into<Icon>,
    size: f32,
    color: impl Into<Color32>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    paint(ui.painter(), rect, icon, color);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_icons_follow_names_then_extensions() {
        for (name, expected) in [
            ("main.rs", Icon::Language(Language::Rust)),
            ("build.gradle.kts", Icon::Language(Language::GradleKotlin)),
            ("App.kt", Icon::Language(Language::Kotlin)),
            ("Dockerfile", Icon::Language(Language::Dockerfile)),
            ("README.md", Icon::Language(Language::Markdown)),
            ("Main.java", Icon::JavaType(TypeKind::Class)),
            ("logo.PNG", Icon::FileImage),
            ("lib.jar", Icon::FileArchive),
            ("Cargo.lock", Icon::Language(Language::Toml)),
            ("deps.lock", Icon::Glyph(Glyph::Lock)),
            ("notes", Icon::Glyph(Glyph::FileText)),
            ("LICENSE", Icon::Glyph(Glyph::FileText)),
        ] {
            assert_eq!(Icon::for_file(name), expected, "{name}");
        }
    }

    #[test]
    fn java_kind_is_peeked_from_the_first_top_level_type() {
        use TypeKind::*;
        for (source, expected) in [
            ("package a;\n\npublic class Foo {\n}\n", Class),
            ("public interface Foo { class Inner {} }", Interface),
            ("public enum Color { RED }", Enum),
            ("public record P(int x) {}", Record),
            ("record Box<T>(T v) {}", Record),
            ("public @interface Marker {}", Annotation),
            ("// class Fake\n/* enum Fake */ final class Real {}", Class),
            ("@Deprecated\npublic interface Old {}", Interface),
            ("class A { enum Inner {} }", Class),
            ("String s = \"interface\";", Class),
            ("int record = 1;", Class),
            ("", Class),
        ] {
            assert_eq!(TypeKind::peek_java(source), expected, "{source}");
        }
        assert_eq!(
            Icon::for_source("Foo.java", Some("enum Foo {}")),
            Icon::JavaType(Enum)
        );
        assert_eq!(Icon::for_source("Foo.java", None), Icon::JavaType(Class));
        assert_eq!(
            Icon::for_source("a.rs", Some("enum Foo {}")),
            Icon::for_file("a.rs")
        );
    }
}
