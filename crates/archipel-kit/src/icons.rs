//! Icons: the designed glyphs of the `archipel-icons` kit, plus file-type icons.
//!
//! [`Glyph`]s are the general icon set (see `docs/icons/`). [`Icon`] adds the file-type
//! icons the kit draws itself because they need the theme and fonts: letter badges, Java
//! types and colored folders. [`Icon::for_file`] picks one for a file name. All drawings
//! are original (no third-party artwork).

use crate::theme::{Colors, Theme};
use crate::tokens::typography;
pub use archipel_icons::{Category, Glyph};
use egui::{self, Color32, Pos2, Rect, Shape, Stroke, epaint::PathShape, vec2};
const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
const SOURCE_ROOT: Color32 = rgb(0x54, 0x8A, 0xF7);
pub(crate) const TEST_ROOT: Color32 = rgb(0x5F, 0xAD, 0x65);
const RESOURCES_ROOT: Color32 = rgb(0xB1, 0x8A, 0xDB);
pub(crate) const EXCLUDED: Color32 = rgb(0xD0, 0x8A, 0x4B);
const IMAGE: Color32 = rgb(0x5F, 0xAD, 0x65);
const ARCHIVE: Color32 = rgb(0xD9, 0xB0, 0x4C);
const HIDDEN: Color32 = rgb(0x6C, 0x70, 0x7E);

/// The shape that carries a file type: the glyph itself says what the file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeShape {
    /// A page with a small coloured corner badge (config and miscellaneous files).
    Page,
    /// A full-size rounded tile with one or two letters.
    Tile,
    /// Kotlin's triangle mark.
    Kotlin,
    /// Stacked bricks: Gradle build scripts.
    Gradle,
    /// `{}`
    Json,
    /// `</>`
    Xml,
    /// A sheet with slider lines: YAML, TOML, properties.
    Settings,
    /// `M↓`
    Markdown,
    /// `>_`
    Shell,
    /// A gear: Rust.
    Gear,
}

/// A file-type glyph, its colour and (for letter glyphs) its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
    /// Shape of the badge.
    pub glyph: BadgeShape,
    /// Letters drawn on letter glyphs (empty for symbol glyphs).
    pub label: &'static str,
    /// Accent color.
    pub color: Color32,
}

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

const fn badge(label: &'static str, color: Color32) -> Badge {
    Badge {
        glyph: BadgeShape::Page,
        label,
        color,
    }
}

const fn tile(label: &'static str, color: Color32) -> Badge {
    Badge {
        glyph: BadgeShape::Tile,
        label,
        color,
    }
}

const fn glyph(glyph: BadgeShape, color: Color32) -> Badge {
    Badge {
        glyph,
        label: "",
        color,
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

/// Exact (lower-case) file names; checked before extensions.
const NAME_BADGES: &[(&str, Badge)] = &[
    (
        "androidmanifest.xml",
        glyph(BadgeShape::Xml, rgb(0x5F, 0xAD, 0x65)),
    ),
    ("dockerfile", tile("D", rgb(0x4F, 0x9B, 0xE0))),
    (".gitignore", badge("Gi", rgb(0xE0, 0x70, 0x55))),
    (".gitattributes", badge("Gi", rgb(0xE0, 0x70, 0x55))),
    (".editorconfig", badge("Ec", rgb(0x9D, 0xA0, 0xA8))),
    ("license", badge("Li", rgb(0xD9, 0xB0, 0x4C))),
    ("licence", badge("Li", rgb(0xD9, 0xB0, 0x4C))),
    ("copying", badge("Li", rgb(0xD9, 0xB0, 0x4C))),
    ("license.md", badge("Li", rgb(0xD9, 0xB0, 0x4C))),
    ("license.txt", badge("Li", rgb(0xD9, 0xB0, 0x4C))),
    ("readme", badge("Rd", rgb(0x6B, 0x9B, 0xFA))),
    ("readme.txt", badge("Rd", rgb(0x6B, 0x9B, 0xFA))),
    ("gradlew", glyph(BadgeShape::Gradle, rgb(0x4F, 0xA8, 0xA0))),
    (
        "gradlew.bat",
        glyph(BadgeShape::Gradle, rgb(0x4F, 0xA8, 0xA0)),
    ),
    ("makefile", badge("Mk", rgb(0x9D, 0xA0, 0xA8))),
];

/// Extensions (lower-case, without the dot) per badge; `gradle.kts` and similar
/// compound extensions are matched by suffix.
const EXTENSION_BADGES: &[(&[&str], Badge)] = &[
    (&["java"], tile("J", rgb(0x4F, 0x9B, 0xE0))),
    (&["kt"], glyph(BadgeShape::Kotlin, rgb(0xA9, 0x7B, 0xF0))),
    (&["kts"], glyph(BadgeShape::Kotlin, rgb(0xA9, 0x7B, 0xF0))),
    (
        &["gradle", "gradle.kts"],
        glyph(BadgeShape::Gradle, rgb(0x4F, 0xA8, 0xA0)),
    ),
    (&["groovy", "gvy"], tile("Gr", rgb(0x4F, 0x9B, 0xE0))),
    (
        &["xml", "xsd", "xsl"],
        glyph(BadgeShape::Xml, rgb(0xE0, 0x9A, 0x55)),
    ),
    (
        &["json", "jsonc"],
        glyph(BadgeShape::Json, rgb(0xF2, 0xC5, 0x5C)),
    ),
    (
        &["yaml", "yml"],
        glyph(BadgeShape::Settings, rgb(0xC0, 0x7B, 0x9B)),
    ),
    (
        &["toml"],
        glyph(BadgeShape::Settings, rgb(0x9D, 0xA0, 0xA8)),
    ),
    (
        &["properties"],
        glyph(BadgeShape::Settings, rgb(0x7F, 0xB0, 0x9A)),
    ),
    (&["lock"], badge("Lk", rgb(0x9D, 0xA0, 0xA8))),
    (
        &["md", "markdown"],
        glyph(BadgeShape::Markdown, rgb(0x6B, 0x9B, 0xFA)),
    ),
    (&["rs"], glyph(BadgeShape::Gear, rgb(0xE0, 0x88, 0x55))),
    (&["go"], tile("Go", rgb(0x4F, 0xB8, 0xD0))),
    (&["py"], tile("Py", rgb(0x5C, 0x8F, 0xD6))),
    (&["js", "mjs", "cjs"], tile("JS", rgb(0xF2, 0xC5, 0x5C))),
    (&["ts"], tile("TS", rgb(0x4F, 0x9B, 0xE0))),
    (&["tsx", "jsx"], tile("Tx", rgb(0x4F, 0xB8, 0xD0))),
    (&["html", "htm"], tile("H", rgb(0xE0, 0x70, 0x55))),
    (&["css", "scss"], tile("C", rgb(0x6B, 0x9B, 0xFA))),
    (
        &["sh", "bash", "zsh"],
        glyph(BadgeShape::Shell, rgb(0x5F, 0xAD, 0x65)),
    ),
    (
        &["bat", "cmd", "ps1"],
        glyph(BadgeShape::Shell, rgb(0x5F, 0xAD, 0x65)),
    ),
    (&["sql"], tile("Sq", rgb(0xC0, 0x7B, 0x9B))),
    (&["proto"], tile("Pb", rgb(0x4F, 0xA8, 0xA0))),
    (&["csv", "tsv"], tile("Cs", rgb(0x5F, 0xAD, 0x65))),
];

const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "svg", "webp", "gif", "ico"];
const ARCHIVE_EXTENSIONS: [&str; 7] = ["jar", "zip", "tar", "gz", "tgz", "war", "aar"];

/// Anything the kit can draw as an icon: a [`Glyph`] from the icon kit, or a file-type icon
/// (letter badges, Java types, colored folders) that needs the theme and fonts.
///
/// Every component that takes an icon accepts `impl Into<Icon>`, so pass a glyph directly:
/// `IconButton::new(Glyph::Refresh, "Reload")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// A designed glyph from `archipel-icons`.
    Glyph(Glyph),
    /// A page with a colored letter badge (language and config files).
    FileBadge(Badge),
    /// A Java type: a filled circle with C, I, E, R or @.
    JavaType(TypeKind),
    /// A folder tinted by its role.
    FolderKind(FolderKind),
    /// An image file (green picture).
    FileImage,
    /// An archive file (page with a zipper).
    FileArchive,
}

impl From<Glyph> for Icon {
    fn from(glyph: Glyph) -> Self {
        Icon::Glyph(glyph)
    }
}

impl Icon {
    /// File icon by well-known file name, then extension; unknown files get the
    /// generic page.
    pub fn for_file(name: &str) -> Icon {
        let lower = name.to_ascii_lowercase();
        if has_java_extension(&lower) {
            return Icon::JavaType(TypeKind::Class);
        }
        if let Some((_, badge)) = NAME_BADGES.iter().find(|(known, _)| *known == lower) {
            return Icon::FileBadge(*badge);
        }
        // Compound extensions first (`build.gradle.kts` is Gradle, not Kotlin script).
        let has_ext = |ext: &str| {
            lower
                .strip_suffix(ext)
                .is_some_and(|rest| rest.ends_with('.') && rest.len() > 1)
        };
        if IMAGE_EXTENSIONS.iter().any(|e| has_ext(e)) {
            return Icon::FileImage;
        }
        if ARCHIVE_EXTENSIONS.iter().any(|e| has_ext(e)) {
            return Icon::FileArchive;
        }
        EXTENSION_BADGES
            .iter()
            .filter_map(|(exts, badge)| {
                exts.iter()
                    .filter(|e| has_ext(e))
                    .map(|e| e.len())
                    .max()
                    .map(|len| (len, badge))
            })
            .max_by_key(|(len, _)| *len)
            .map(|(_, badge)| Icon::FileBadge(*badge))
            .unwrap_or(Icon::Glyph(Glyph::FileText))
    }

    /// Like [`Icon::for_file`], peeking into Java sources for the kind of their type.
    pub fn for_source(name: &str, text: Option<&str>) -> Icon {
        match (Icon::for_file(name), text) {
            (Icon::JavaType(_), Some(text)) => Icon::JavaType(TypeKind::peek_java(text)),
            (icon, _) => icon,
        }
    }
}

fn has_java_extension(lower: &str) -> bool {
    lower
        .strip_suffix("java")
        .is_some_and(|rest| rest.ends_with('.') && rest.len() > 1)
}

/// Maps 16×16 design coordinates into `rect`.
struct Grid {
    origin: Pos2,
    scale: f32,
}

impl Grid {
    fn new(rect: Rect) -> Self {
        let side = rect.width().min(rect.height());
        let scale = side / 16.0;
        let origin = rect.center() - vec2(8.0 * scale, 8.0 * scale);
        Self { origin, scale }
    }
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x * self.scale, y * self.scale)
    }
    fn len(&self, value: f32) -> f32 {
        value * self.scale
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
}

/// Semantic tones of the icon kit, from `theme`.
pub fn tones(theme: &Theme) -> archipel_icons::Tones {
    let hex = |c: Color32| u32::from(c.r()) << 16 | u32::from(c.g()) << 8 | u32::from(c.b());
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

/// Paints `icon` into `rect` (scaled from its 16×16 design grid) in `color`. Status
/// icons (error, warning, info, success, run) keep their semantic colors.
pub fn paint(
    painter: &egui::Painter,
    rect: Rect,
    icon: impl Into<Icon>,
    color: impl Into<Color32>,
) {
    let icon = icon.into();
    let color = color.into();
    let colors = Theme::colors(painter.ctx());
    let g = Grid::new(rect);
    let stroke = Stroke::new(g.len(1.0).max(1.0), color);
    let line = |points: &[(f32, f32)]| {
        painter.add(Shape::Path(PathShape::line(
            points.iter().map(|&(x, y)| g.p(x, y)).collect(),
            stroke,
        )));
    };
    let closed = |points: &[(f32, f32)], fill: Color32, outline: Stroke| {
        painter.add(Shape::Path(PathShape {
            points: points.iter().map(|&(x, y)| g.p(x, y)).collect(),
            closed: true,
            fill,
            stroke: outline.into(),
        }));
    };
    let page = |accent: Option<Color32>| {
        closed(
            &[
                (3.5, 1.5),
                (9.5, 1.5),
                (12.5, 4.5),
                (12.5, 14.5),
                (3.5, 14.5),
            ],
            Color32::TRANSPARENT,
            Stroke::new(stroke.width, colors.icon),
        );
        line(&[(9.5, 1.5), (9.5, 4.5), (12.5, 4.5)]);
        if let Some(accent) = accent {
            painter.rect_filled(g.rect(7.0, 8.0, 15.5, 15.5), g.len(1.5), accent);
        }
    };
    let glyph_text = |text: &str, at: Pos2, size: f32, color: Color32| {
        painter.text(
            at,
            egui::Align2::CENTER_CENTER,
            text,
            typography::semibold(painter.ctx(), g.len(size)),
            color,
        );
    };
    let badge_text = |text: &str| {
        painter.text(
            g.p(11.25, 11.9),
            egui::Align2::CENTER_CENTER,
            text,
            typography::semibold(painter.ctx(), g.len(6.0)),
            colors.island,
        );
    };
    match icon {
        Icon::Glyph(glyph) => {
            let theme = Theme::current(painter.ctx());
            archipel_icons::paint(painter, rect, glyph, color, &tones(&theme));
        }
        Icon::FolderKind(kind) => {
            let fill = kind.fill(&colors);
            let outline = kind
                .outline()
                .map(|color| Stroke::new(stroke.width, color))
                .unwrap_or(stroke);
            closed(
                &[
                    (1.5, 3.5),
                    (6.0, 3.5),
                    (7.5, 5.0),
                    (14.5, 5.0),
                    (14.5, 13.5),
                    (1.5, 13.5),
                ],
                fill,
                outline,
            );
            if kind == FolderKind::Package {
                painter.circle_filled(g.p(8.0, 9.5), g.len(1.6), color);
            }
        }
        Icon::JavaType(kind) => {
            painter.circle_filled(g.p(8.0, 8.0), g.len(6.5), kind.color());
            glyph_text(kind.letter(), g.p(8.0, 8.4), 9.5, colors.island);
        }
        Icon::FileBadge(badge) => match badge.glyph {
            BadgeShape::Page => {
                page(Some(badge.color));
                badge_text(badge.label);
            }
            BadgeShape::Tile => {
                painter.rect_filled(g.rect(1.0, 1.0, 15.0, 15.0), g.len(3.0), badge.color);
                glyph_text(badge.label, g.p(8.0, 8.4), 8.0, colors.island);
            }
            BadgeShape::Kotlin => {
                closed(
                    &[
                        (2.0, 2.0),
                        (14.0, 2.0),
                        (8.0, 8.0),
                        (14.0, 14.0),
                        (2.0, 14.0),
                    ],
                    badge.color,
                    Stroke::NONE,
                );
                closed(
                    &[(8.0, 8.0), (14.0, 14.0), (2.0, 14.0), (2.0, 8.0)],
                    rgb(0xE0, 0x88, 0x55),
                    Stroke::NONE,
                );
            }
            BadgeShape::Gradle => {
                let brick = |x0: f32, y0: f32, x1: f32, y1: f32| {
                    painter.rect_filled(g.rect(x0, y0, x1, y1), g.len(1.0), badge.color);
                };
                brick(1.5, 2.5, 7.5, 6.5);
                brick(8.5, 2.5, 14.5, 6.5);
                brick(1.5, 7.5, 4.5, 11.5);
                brick(5.5, 7.5, 10.5, 11.5);
                brick(11.5, 7.5, 14.5, 11.5);
                brick(1.5, 12.5, 7.5, 15.0);
                brick(8.5, 12.5, 14.5, 15.0);
            }
            BadgeShape::Json => glyph_text("{}", g.p(8.0, 8.2), 12.0, badge.color),
            BadgeShape::Xml => glyph_text("</>", g.p(8.0, 8.2), 9.5, badge.color),
            BadgeShape::Settings => {
                closed(
                    &[(2.5, 2.5), (13.5, 2.5), (13.5, 13.5), (2.5, 13.5)],
                    Color32::TRANSPARENT,
                    Stroke::new(stroke.width, badge.color),
                );
                for (y, knob) in [(5.5, 10.0), (8.0, 6.0), (10.5, 9.0)] {
                    line(&[(4.5, y), (11.5, y)]);
                    painter.circle_filled(g.p(knob, y), g.len(1.3), badge.color);
                }
            }
            BadgeShape::Markdown => {
                painter.rect_stroke(
                    g.rect(1.0, 3.0, 15.0, 13.0),
                    g.len(2.0),
                    Stroke::new(stroke.width, badge.color),
                    egui::StrokeKind::Inside,
                );
                glyph_text("M↓", g.p(8.0, 8.2), 8.0, badge.color);
            }
            BadgeShape::Shell => {
                painter.rect_filled(g.rect(1.0, 2.0, 15.0, 14.0), g.len(2.5), badge.color);
                glyph_text(">_", g.p(8.0, 8.2), 8.5, colors.island);
            }
            BadgeShape::Gear => {
                let center = g.p(8.0, 8.0);
                for step in 0..8 {
                    let angle = step as f32 * std::f32::consts::FRAC_PI_4;
                    let (sin, cos) = angle.sin_cos();
                    painter.line_segment(
                        [
                            center + vec2(cos, sin) * g.len(4.5),
                            center + vec2(cos, sin) * g.len(7.0),
                        ],
                        Stroke::new(g.len(2.4), badge.color),
                    );
                }
                painter.circle_stroke(center, g.len(4.0), Stroke::new(g.len(2.0), badge.color));
            }
        },
        Icon::FileImage => {
            painter.rect_stroke(
                g.rect(1.5, 2.5, 14.5, 13.5),
                g.len(2.0),
                Stroke::new(stroke.width, IMAGE),
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(g.p(11.0, 6.0), g.len(1.4), IMAGE);
            closed(
                &[
                    (3.0, 12.5),
                    (6.5, 7.5),
                    (9.0, 10.5),
                    (10.5, 9.0),
                    (13.0, 12.5),
                ],
                IMAGE,
                Stroke::NONE,
            );
        }
        Icon::FileArchive => {
            page(None);
            // A zipper down the middle with a pull tab.
            for (i, y) in [3.0_f32, 5.0, 7.0, 9.0].into_iter().enumerate() {
                let x = if i % 2 == 0 { 7.0 } else { 9.0 };
                painter.rect_filled(g.rect(x, y, x + 2.0, y + 1.2), 0.0, ARCHIVE);
            }
            painter.rect_stroke(
                g.rect(6.5, 10.5, 9.5, 13.5),
                g.len(0.8),
                Stroke::new(stroke.width, ARCHIVE),
                egui::StrokeKind::Inside,
            );
        }
    }
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
    use egui::pos2;

    fn glyph_of(name: &str) -> Option<(BadgeShape, &'static str)> {
        match Icon::for_file(name) {
            Icon::FileBadge(badge) => Some((badge.glyph, badge.label)),
            _ => None,
        }
    }

    #[test]
    fn file_icons_follow_names_then_extensions() {
        use BadgeShape::*;
        for (name, expected) in [
            ("A.kt", (Kotlin, "")),
            ("x.kts", (Kotlin, "")),
            ("build.gradle.kts", (Gradle, "")),
            ("settings.gradle", (Gradle, "")),
            ("gradlew", (Gradle, "")),
            ("gradlew.bat", (Gradle, "")),
            ("a.groovy", (Tile, "Gr")),
            ("pom.xml", (Xml, "")),
            ("AndroidManifest.xml", (Xml, "")),
            ("a.json", (Json, "")),
            ("a.yml", (Settings, "")),
            ("a.yaml", (Settings, "")),
            ("Cargo.toml", (Settings, "")),
            ("app.properties", (Settings, "")),
            ("a.md", (Markdown, "")),
            ("lib.rs", (Gear, "")),
            ("main.go", (Tile, "Go")),
            ("a.py", (Tile, "Py")),
            ("a.js", (Tile, "JS")),
            ("a.ts", (Tile, "TS")),
            ("a.tsx", (Tile, "Tx")),
            ("index.html", (Tile, "H")),
            ("a.css", (Tile, "C")),
            ("a.sh", (Shell, "")),
            ("a.bat", (Shell, "")),
            ("a.cmd", (Shell, "")),
            ("a.ps1", (Shell, "")),
            ("a.sql", (Tile, "Sq")),
            ("a.proto", (Tile, "Pb")),
            ("a.csv", (Tile, "Cs")),
            ("Dockerfile", (Tile, "D")),
            // Config files keep the page with a corner badge.
            ("Cargo.lock", (Page, "Lk")),
            (".gitignore", (Page, "Gi")),
            (".gitattributes", (Page, "Gi")),
            (".editorconfig", (Page, "Ec")),
            ("LICENSE", (Page, "Li")),
            ("README", (Page, "Rd")),
            ("README.MD", (Markdown, "")),
        ] {
            assert_eq!(glyph_of(name), Some(expected), "{name}");
        }
        assert_eq!(Icon::for_file("Main.java"), Icon::JavaType(TypeKind::Class));
        for name in ["a.png", "a.JPG", "a.svg", "a.webp"] {
            assert_eq!(Icon::for_file(name), Icon::FileImage, "{name}");
        }
        for name in ["a.jar", "a.zip"] {
            assert_eq!(Icon::for_file(name), Icon::FileArchive, "{name}");
        }
        for name in [
            "notes.txt",
            "Unknown.xyz",
            "noextension",
            ".hidden",
            "png",
            "java",
        ] {
            assert_eq!(Icon::for_file(name), Icon::Glyph(Glyph::FileText), "{name}");
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

    #[test]
    fn grid_maps_center_of_any_square() {
        let grid = Grid::new(Rect::from_min_size(pos2(10.0, 10.0), vec2(32.0, 32.0)));
        assert_eq!(grid.p(8.0, 8.0), pos2(26.0, 26.0));
        assert_eq!(grid.len(1.0), 2.0);
    }
}
