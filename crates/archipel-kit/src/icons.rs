//! Vector icons drawn on a 16×16 grid in a thin-outline style.
//!
//! These are original drawings made for Archipel (no third-party artwork is
//! copied); they follow the general conventions of contemporary IDE icon sets:
//! 1px outlines, rounded joins, and a single neutral color with semantic accents.

use crate::theme::{Colors, Theme};
use crate::tokens::typography;
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
pub enum Glyph {
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
    pub glyph: Glyph,
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
        glyph: Glyph::Page,
        label,
        color,
    }
}

const fn tile(label: &'static str, color: Color32) -> Badge {
    Badge {
        glyph: Glyph::Tile,
        label,
        color,
    }
}

const fn glyph(glyph: Glyph, color: Color32) -> Badge {
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
        glyph(Glyph::Xml, rgb(0x5F, 0xAD, 0x65)),
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
    ("gradlew", glyph(Glyph::Gradle, rgb(0x4F, 0xA8, 0xA0))),
    ("gradlew.bat", glyph(Glyph::Gradle, rgb(0x4F, 0xA8, 0xA0))),
    ("makefile", badge("Mk", rgb(0x9D, 0xA0, 0xA8))),
];

/// Extensions (lower-case, without the dot) per badge; `gradle.kts` and similar
/// compound extensions are matched by suffix.
const EXTENSION_BADGES: &[(&[&str], Badge)] = &[
    (&["java"], tile("J", rgb(0x4F, 0x9B, 0xE0))),
    (&["kt"], glyph(Glyph::Kotlin, rgb(0xA9, 0x7B, 0xF0))),
    (&["kts"], glyph(Glyph::Kotlin, rgb(0xA9, 0x7B, 0xF0))),
    (
        &["gradle", "gradle.kts"],
        glyph(Glyph::Gradle, rgb(0x4F, 0xA8, 0xA0)),
    ),
    (&["groovy", "gvy"], tile("Gr", rgb(0x4F, 0x9B, 0xE0))),
    (
        &["xml", "xsd", "xsl"],
        glyph(Glyph::Xml, rgb(0xE0, 0x9A, 0x55)),
    ),
    (
        &["json", "jsonc"],
        glyph(Glyph::Json, rgb(0xF2, 0xC5, 0x5C)),
    ),
    (
        &["yaml", "yml"],
        glyph(Glyph::Settings, rgb(0xC0, 0x7B, 0x9B)),
    ),
    (&["toml"], glyph(Glyph::Settings, rgb(0x9D, 0xA0, 0xA8))),
    (
        &["properties"],
        glyph(Glyph::Settings, rgb(0x7F, 0xB0, 0x9A)),
    ),
    (&["lock"], badge("Lk", rgb(0x9D, 0xA0, 0xA8))),
    (
        &["md", "markdown"],
        glyph(Glyph::Markdown, rgb(0x6B, 0x9B, 0xFA)),
    ),
    (&["rs"], glyph(Glyph::Gear, rgb(0xE0, 0x88, 0x55))),
    (&["go"], tile("Go", rgb(0x4F, 0xB8, 0xD0))),
    (&["py"], tile("Py", rgb(0x5C, 0x8F, 0xD6))),
    (&["js", "mjs", "cjs"], tile("JS", rgb(0xF2, 0xC5, 0x5C))),
    (&["ts"], tile("TS", rgb(0x4F, 0x9B, 0xE0))),
    (&["tsx", "jsx"], tile("Tx", rgb(0x4F, 0xB8, 0xD0))),
    (&["html", "htm"], tile("H", rgb(0xE0, 0x70, 0x55))),
    (&["css", "scss"], tile("C", rgb(0x6B, 0x9B, 0xFA))),
    (
        &["sh", "bash", "zsh"],
        glyph(Glyph::Shell, rgb(0x5F, 0xAD, 0x65)),
    ),
    (
        &["bat", "cmd", "ps1"],
        glyph(Glyph::Shell, rgb(0x5F, 0xAD, 0x65)),
    ),
    (&["sql"], tile("Sq", rgb(0xC0, 0x7B, 0x9B))),
    (&["proto"], tile("Pb", rgb(0x4F, 0xA8, 0xA0))),
    (&["csv", "tsv"], tile("Cs", rgb(0x5F, 0xAD, 0x65))),
];

const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "svg", "webp", "gif", "ico"];
const ARCHIVE_EXTENSIONS: [&str; 7] = ["jar", "zip", "tar", "gz", "tgz", "war", "aar"];

/// An icon. Draw it with [`show`] or [`paint`]; [`Icon::for_file`] picks one for a file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Main menu (hamburger).
    MainMenu,
    /// Chevron pointing down (expanded, dropdown).
    ChevronDown,
    /// Chevron pointing up.
    ChevronUp,
    /// Chevron pointing right (collapsed).
    ChevronRight,
    /// Plain folder.
    Folder,
    /// Generic file page.
    File,
    /// A page with a coloured letter badge (language and config files).
    FileBadge(Badge),
    /// A Java type: a filled circle with C, I, E, R or @.
    JavaType(TypeKind),
    /// Image file.
    FileImage,
    /// Archive file (zip, jar).
    FileArchive,
    /// A folder tinted by its role.
    FolderKind(FolderKind),
    /// Project (folder with a dot).
    Project,
    /// VCS commit.
    Commit,
    /// Structure view.
    Structure,
    /// VCS branch.
    Branch,
    /// Terminal prompt.
    Terminal,
    /// Problems list.
    Problems,
    /// Build hammer.
    Build,
    /// Find in files.
    Find,
    /// Run (play).
    Run,
    /// Stop (square).
    Stop,
    /// Magnifier.
    Search,
    /// Close (×).
    Close,
    /// Minimize (hide panel).
    Minimize,
    /// Refresh / reload.
    Refresh,
    /// Curved arrow back (rollback / revert).
    Rollback,
    /// Two-pane compare (show diff).
    Diff,
    /// Filled circle with an exclamation mark.
    Error,
    /// Filled circle with an `i`.
    Info,
    /// Three horizontal dots (overflow menu).
    More,
    /// Warning triangle.
    Warning,
    /// Phone outline (devices, Device Manager).
    Device,
    /// Log output.
    Logcat,
    /// Pause.
    Pause,
    /// Scroll to end.
    ScrollEnd,
    /// Soft wrap.
    SoftWrap,
    /// Weak warning (dimmed triangle).
    WeakWarning,
    /// Pin.
    Pin,
    /// Check mark (success).
    Success,
    /// Rerun.
    Rerun,
    /// Stacked squares (Build Variants).
    Variants,
    /// AI Assistant: a four-point sparkle with a small companion star.
    Ai,
    /// Plus.
    Plus,
    /// Copy.
    Copy,
    /// Send (arrow up).
    Send,
    /// Build-tool model (cube); used for the Gradle tool window and projects.
    Model,
    /// Single task (gear).
    Task,
    /// Expand all.
    ExpandAll,
    /// Collapse all.
    CollapseAll,
    /// Add (plus in a box).
    Add,
    /// Debug (bug).
    Debug,
    /// Debugger step over.
    StepOver,
    /// Debugger step into.
    StepInto,
    /// Debugger step out.
    StepOut,
}

impl Icon {
    /// Every icon without parameters, with its name (for galleries and pickers).
    pub const ALL: &'static [(Icon, &'static str)] = &[
        (Icon::MainMenu, "MainMenu"),
        (Icon::ChevronDown, "ChevronDown"),
        (Icon::ChevronUp, "ChevronUp"),
        (Icon::ChevronRight, "ChevronRight"),
        (Icon::Folder, "Folder"),
        (Icon::File, "File"),
        (Icon::FileImage, "FileImage"),
        (Icon::FileArchive, "FileArchive"),
        (Icon::Project, "Project"),
        (Icon::Commit, "Commit"),
        (Icon::Structure, "Structure"),
        (Icon::Branch, "Branch"),
        (Icon::Terminal, "Terminal"),
        (Icon::Problems, "Problems"),
        (Icon::Build, "Build"),
        (Icon::Find, "Find"),
        (Icon::Run, "Run"),
        (Icon::Stop, "Stop"),
        (Icon::Search, "Search"),
        (Icon::Close, "Close"),
        (Icon::Minimize, "Minimize"),
        (Icon::Refresh, "Refresh"),
        (Icon::Rollback, "Rollback"),
        (Icon::Diff, "Diff"),
        (Icon::Error, "Error"),
        (Icon::Info, "Info"),
        (Icon::More, "More"),
        (Icon::Warning, "Warning"),
        (Icon::Device, "Device"),
        (Icon::Logcat, "Logcat"),
        (Icon::Pause, "Pause"),
        (Icon::ScrollEnd, "ScrollEnd"),
        (Icon::SoftWrap, "SoftWrap"),
        (Icon::WeakWarning, "WeakWarning"),
        (Icon::Pin, "Pin"),
        (Icon::Success, "Success"),
        (Icon::Rerun, "Rerun"),
        (Icon::Variants, "Variants"),
        (Icon::Ai, "Ai"),
        (Icon::Plus, "Plus"),
        (Icon::Copy, "Copy"),
        (Icon::Send, "Send"),
        (Icon::Model, "Model"),
        (Icon::Task, "Task"),
        (Icon::ExpandAll, "ExpandAll"),
        (Icon::CollapseAll, "CollapseAll"),
        (Icon::Add, "Add"),
        (Icon::Debug, "Debug"),
        (Icon::StepOver, "StepOver"),
        (Icon::StepInto, "StepInto"),
        (Icon::StepOut, "StepOut"),
    ];

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
            .unwrap_or(Icon::File)
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

/// Paints `icon` into `rect` (scaled from its 16×16 design grid) in `color`. Status
/// icons (error, warning, info, success, run) keep their semantic colors.
pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: impl Into<Color32>) {
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
        Icon::MainMenu => {
            for y in [4.5, 8.0, 11.5] {
                line(&[(2.5, y), (13.5, y)]);
            }
        }
        Icon::ChevronDown => line(&[(4.5, 6.5), (8.0, 10.0), (11.5, 6.5)]),
        Icon::ChevronUp => line(&[(4.5, 9.5), (8.0, 6.0), (11.5, 9.5)]),
        Icon::ChevronRight => line(&[(6.5, 4.5), (10.0, 8.0), (6.5, 11.5)]),
        Icon::Folder | Icon::Project | Icon::FolderKind(_) => {
            let kind = match icon {
                Icon::FolderKind(kind) => kind,
                _ => FolderKind::Plain,
            };
            let fill = if icon == Icon::Project {
                Color32::TRANSPARENT
            } else {
                kind.fill(&colors)
            };
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
            if icon == Icon::Project {
                line(&[(1.5, 7.0), (14.5, 7.0)]);
            }
            if kind == FolderKind::Package {
                painter.circle_filled(g.p(8.0, 9.5), g.len(1.6), color);
            }
        }
        Icon::File => {
            page(None);
            for y in [7.5, 9.5, 11.5] {
                line(&[(5.5, y), (10.5, y)]);
            }
        }
        Icon::JavaType(kind) => {
            painter.circle_filled(g.p(8.0, 8.0), g.len(6.5), kind.color());
            glyph_text(kind.letter(), g.p(8.0, 8.4), 9.5, colors.island);
        }
        Icon::FileBadge(badge) => match badge.glyph {
            Glyph::Page => {
                page(Some(badge.color));
                badge_text(badge.label);
            }
            Glyph::Tile => {
                painter.rect_filled(g.rect(1.0, 1.0, 15.0, 15.0), g.len(3.0), badge.color);
                glyph_text(badge.label, g.p(8.0, 8.4), 8.0, colors.island);
            }
            Glyph::Kotlin => {
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
            Glyph::Gradle => {
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
            Glyph::Json => glyph_text("{}", g.p(8.0, 8.2), 12.0, badge.color),
            Glyph::Xml => glyph_text("</>", g.p(8.0, 8.2), 9.5, badge.color),
            Glyph::Settings => {
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
            Glyph::Markdown => {
                painter.rect_stroke(
                    g.rect(1.0, 3.0, 15.0, 13.0),
                    g.len(2.0),
                    Stroke::new(stroke.width, badge.color),
                    egui::StrokeKind::Inside,
                );
                glyph_text("M↓", g.p(8.0, 8.2), 8.0, badge.color);
            }
            Glyph::Shell => {
                painter.rect_filled(g.rect(1.0, 2.0, 15.0, 14.0), g.len(2.5), badge.color);
                glyph_text(">_", g.p(8.0, 8.2), 8.5, colors.island);
            }
            Glyph::Gear => {
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
        Icon::Commit => {
            painter.circle_stroke(g.p(8.0, 8.0), g.len(3.0), stroke);
            line(&[(1.0, 8.0), (5.0, 8.0)]);
            line(&[(11.0, 8.0), (15.0, 8.0)]);
        }
        Icon::Structure => {
            // Outline tree: a parent node with two indented children.
            painter.rect_stroke(
                g.rect(1.5, 1.5, 6.5, 5.5),
                g.len(1.0),
                stroke,
                egui::StrokeKind::Middle,
            );
            line(&[(4.0, 5.5), (4.0, 12.5), (8.5, 12.5)]);
            line(&[(4.0, 8.5), (8.5, 8.5)]);
            line(&[(10.0, 8.5), (14.5, 8.5)]);
            line(&[(10.0, 12.5), (14.5, 12.5)]);
        }
        Icon::Branch => {
            painter.circle_stroke(g.p(4.5, 3.5), g.len(1.8), stroke);
            painter.circle_stroke(g.p(4.5, 12.5), g.len(1.8), stroke);
            painter.circle_stroke(g.p(11.5, 4.5), g.len(1.8), stroke);
            line(&[(4.5, 5.3), (4.5, 10.7)]);
            line(&[
                (11.5, 6.3),
                (11.5, 7.5),
                (10.0, 9.0),
                (6.0, 9.5),
                (4.5, 10.7),
            ]);
        }
        Icon::Terminal => {
            painter.rect_stroke(
                g.rect(1.5, 2.5, 14.5, 13.5),
                g.len(2.0),
                stroke,
                egui::StrokeKind::Middle,
            );
            line(&[(4.5, 6.0), (6.5, 8.0), (4.5, 10.0)]);
            line(&[(8.0, 10.5), (11.5, 10.5)]);
        }
        Icon::Problems => {
            painter.circle_stroke(g.p(8.0, 8.0), g.len(6.5), stroke);
            line(&[(8.0, 4.5), (8.0, 9.0)]);
            painter.circle_filled(g.p(8.0, 11.3), g.len(0.9), color);
        }
        Icon::Build => {
            // Hammer: angled head with a claw, and a diagonal handle.
            closed(
                &[
                    (6.0, 2.0),
                    (9.0, 2.0),
                    (14.0, 7.0),
                    (12.5, 8.5),
                    (8.5, 5.5),
                    (5.5, 5.5),
                ],
                Color32::TRANSPARENT,
                stroke,
            );
            closed(
                &[(8.0, 6.5), (9.5, 8.0), (3.5, 14.0), (2.0, 12.5)],
                Color32::TRANSPARENT,
                stroke,
            );
        }
        Icon::Find | Icon::Search => {
            painter.circle_stroke(g.p(7.0, 7.0), g.len(4.5), stroke);
            line(&[(10.3, 10.3), (14.0, 14.0)]);
        }
        Icon::Run => closed(
            &[(4.5, 2.5), (13.0, 8.0), (4.5, 13.5)],
            color,
            Stroke::new(stroke.width, color),
        ),
        Icon::Stop => {
            painter.rect_filled(g.rect(3.5, 3.5, 12.5, 12.5), g.len(1.5), color);
        }
        Icon::Close => {
            line(&[(4.0, 4.0), (12.0, 12.0)]);
            line(&[(12.0, 4.0), (4.0, 12.0)]);
        }
        Icon::Minimize => line(&[(3.5, 8.0), (12.5, 8.0)]),
        Icon::Logcat => {
            // Log lines of varying length with a level marker.
            painter.rect_stroke(
                g.rect(1.5, 2.5, 14.5, 13.5),
                g.len(2.0),
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.circle_filled(g.p(4.5, 6.0), g.len(0.9), color);
            line(&[(6.5, 6.0), (12.0, 6.0)]);
            painter.circle_filled(g.p(4.5, 10.0), g.len(0.9), color);
            line(&[(6.5, 10.0), (10.0, 10.0)]);
        }
        Icon::Variants => {
            painter.rect_stroke(
                g.rect(1.5, 5.5, 10.5, 14.5),
                g.len(1.5),
                stroke,
                egui::StrokeKind::Middle,
            );
            line(&[
                (4.5, 3.5),
                (4.5, 1.5),
                (14.5, 1.5),
                (14.5, 11.5),
                (12.5, 11.5),
            ]);
        }
        Icon::Pause => {
            line(&[(5.5, 3.5), (5.5, 12.5)]);
            line(&[(10.5, 3.5), (10.5, 12.5)]);
        }
        Icon::ScrollEnd => {
            line(&[(8.0, 2.5), (8.0, 10.5)]);
            line(&[(4.5, 7.5), (8.0, 11.0), (11.5, 7.5)]);
            line(&[(3.5, 13.5), (12.5, 13.5)]);
        }
        Icon::SoftWrap => {
            line(&[(2.5, 4.0), (13.5, 4.0)]);
            line(&[(2.5, 8.0), (12.0, 8.0)]);
            line(&[(12.0, 8.0), (13.5, 9.5), (12.0, 11.5), (8.5, 11.5)]);
            line(&[(10.0, 10.0), (8.5, 11.5), (10.0, 13.0)]);
        }
        Icon::Device => {
            painter.rect_stroke(
                g.rect(4.0, 1.5, 12.0, 14.5),
                g.len(1.5),
                stroke,
                egui::StrokeKind::Middle,
            );
            line(&[(7.0, 12.5), (9.0, 12.5)]);
        }
        Icon::Pin => {
            line(&[(5.0, 2.5), (11.0, 2.5)]);
            line(&[
                (6.5, 2.5),
                (6.5, 7.0),
                (4.0, 9.5),
                (12.0, 9.5),
                (9.5, 7.0),
                (9.5, 2.5),
            ]);
            line(&[(8.0, 9.5), (8.0, 14.0)]);
        }
        Icon::Refresh => {
            let center = g.p(8.0, 8.0);
            let points = (0..=20)
                .map(|step| {
                    let angle = -0.6 + step as f32 / 20.0 * 5.0;
                    center + vec2(angle.cos(), angle.sin()) * g.len(5.5)
                })
                .collect::<Vec<_>>();
            let tip = *points.last().unwrap_or(&center);
            painter.add(Shape::Path(PathShape::line(points, stroke)));
            painter.line_segment([tip, tip + vec2(g.len(3.0), g.len(0.3))], stroke);
            painter.line_segment([tip, tip + vec2(g.len(0.3), g.len(3.0))], stroke);
        }
        Icon::Rollback => {
            // Arrow pointing left that curves down to the right.
            line(&[(3.0, 6.0), (10.0, 6.0)]);
            let center = g.p(10.0, 9.5);
            let points = (0..=12)
                .map(|step| {
                    let angle =
                        -std::f32::consts::FRAC_PI_2 + step as f32 / 12.0 * std::f32::consts::PI;
                    center + vec2(angle.cos(), angle.sin()) * g.len(3.5)
                })
                .collect::<Vec<_>>();
            painter.add(Shape::Path(PathShape::line(points, stroke)));
            line(&[(10.0, 13.0), (6.5, 13.0)]);
            line(&[(5.5, 3.5), (3.0, 6.0), (5.5, 8.5)]);
        }
        Icon::Diff => {
            closed(
                &[(2.0, 3.0), (7.0, 3.0), (7.0, 13.0), (2.0, 13.0)],
                Color32::TRANSPARENT,
                stroke,
            );
            closed(
                &[(9.0, 3.0), (14.0, 3.0), (14.0, 13.0), (9.0, 13.0)],
                Color32::TRANSPARENT,
                stroke,
            );
            line(&[(3.5, 6.5), (5.5, 6.5)]);
            line(&[(10.5, 6.5), (12.5, 6.5)]);
            line(&[(10.5, 9.5), (12.5, 9.5)]);
        }
        Icon::Info => {
            painter.circle_filled(g.p(8.0, 8.0), g.len(6.5), colors.link);
            let mark = Stroke::new(g.len(1.5).max(1.0), colors.island);
            painter.line_segment([g.p(8.0, 7.2), g.p(8.0, 11.5)], mark);
            painter.circle_filled(g.p(8.0, 4.8), g.len(1.0), colors.island);
        }
        Icon::More => {
            for x in [3.5, 8.0, 12.5] {
                painter.circle_filled(g.p(x, 8.0), g.len(1.2), color);
            }
        }
        Icon::Error => {
            painter.circle_filled(g.p(8.0, 8.0), g.len(6.5), colors.error);
            let white = Stroke::new(g.len(1.5).max(1.0), colors.island);
            painter.line_segment([g.p(8.0, 4.5), g.p(8.0, 9.0)], white);
            painter.circle_filled(g.p(8.0, 11.3), g.len(1.0), colors.island);
        }
        Icon::Warning | Icon::WeakWarning => {
            let fill = if icon == Icon::Warning {
                colors.warning
            } else {
                Theme::current(painter.ctx()).editor.weak_warning_effect
            };
            closed(
                &[(8.0, 1.5), (15.0, 14.0), (1.0, 14.0)],
                fill,
                Stroke::new(stroke.width, fill),
            );
            let dark = Stroke::new(g.len(1.5).max(1.0), colors.island);
            painter.line_segment([g.p(8.0, 6.0), g.p(8.0, 10.0)], dark);
            painter.circle_filled(g.p(8.0, 12.0), g.len(0.9), colors.island);
        }
        Icon::Success => {
            let check = Stroke::new(g.len(1.6).max(1.0), colors.success);
            painter.add(Shape::Path(PathShape::line(
                vec![g.p(3.0, 8.5), g.p(6.5, 12.0), g.p(13.0, 4.5)],
                check,
            )));
        }
        Icon::Model => {
            // Isometric cube.
            closed(
                &[
                    (8.0, 1.5),
                    (14.0, 4.75),
                    (14.0, 11.25),
                    (8.0, 14.5),
                    (2.0, 11.25),
                    (2.0, 4.75),
                ],
                Color32::TRANSPARENT,
                stroke,
            );
            line(&[(2.0, 4.75), (8.0, 8.0), (14.0, 4.75)]);
            line(&[(8.0, 8.0), (8.0, 14.5)]);
        }
        Icon::Task => {
            let center = g.p(8.0, 8.0);
            for step in 0..8 {
                let angle = step as f32 * std::f32::consts::FRAC_PI_4;
                let dir = vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [center + dir * g.len(4.0), center + dir * g.len(6.5)],
                    Stroke::new(g.len(1.6).max(1.0), color),
                );
            }
            painter.circle_stroke(center, g.len(4.0), stroke);
            painter.circle_stroke(center, g.len(1.6), stroke);
        }
        Icon::Debug => {
            // Bug: body, head and legs (original drawing).
            painter.add(egui::Shape::ellipse_stroke(
                g.p(8.0, 9.5),
                vec2(g.len(3.5), g.len(4.5)),
                stroke,
            ));
            painter.circle_stroke(g.p(8.0, 3.8), g.len(1.8), stroke);
            for y in [7.5, 10.0, 12.5] {
                line(&[(1.5, y), (4.5, y)]);
                line(&[(11.5, y), (14.5, y)]);
            }
        }
        Icon::StepOver => {
            let center = g.p(8.0, 9.0);
            let points = (0..=16)
                .map(|step| {
                    let angle = std::f32::consts::PI + step as f32 / 16.0 * std::f32::consts::PI;
                    center + vec2(angle.cos(), angle.sin()) * g.len(5.0)
                })
                .collect();
            painter.add(Shape::Path(PathShape::line(points, stroke)));
            line(&[(11.0, 7.0), (13.0, 9.0), (15.0, 7.0)]);
            painter.circle_filled(g.p(8.0, 13.5), g.len(1.3), color);
        }
        Icon::StepInto => {
            line(&[(8.0, 1.5), (8.0, 10.0)]);
            line(&[(4.5, 6.5), (8.0, 10.0), (11.5, 6.5)]);
            line(&[(3.0, 14.0), (13.0, 14.0)]);
        }
        Icon::StepOut => {
            line(&[(8.0, 11.0), (8.0, 2.5)]);
            line(&[(4.5, 6.0), (8.0, 2.5), (11.5, 6.0)]);
            line(&[(3.0, 14.0), (13.0, 14.0)]);
        }
        Icon::Add => {
            line(&[(8.0, 3.0), (8.0, 13.0)]);
            line(&[(3.0, 8.0), (13.0, 8.0)]);
        }
        Icon::ExpandAll => {
            line(&[(4.5, 4.0), (8.0, 7.5), (11.5, 4.0)]);
            line(&[(4.5, 8.5), (8.0, 12.0), (11.5, 8.5)]);
        }
        Icon::CollapseAll => {
            line(&[(4.5, 7.5), (8.0, 4.0), (11.5, 7.5)]);
            line(&[(4.5, 12.0), (8.0, 8.5), (11.5, 12.0)]);
        }
        Icon::Rerun => {
            let center = g.p(8.0, 8.0);
            let points = (0..=20)
                .map(|step| {
                    let angle = 0.9 + step as f32 / 20.0 * 4.6;
                    center + vec2(angle.cos(), angle.sin()) * g.len(5.5)
                })
                .collect();
            painter.add(Shape::Path(PathShape::line(points, stroke)));
            closed(
                &[(6.5, 6.0), (11.0, 8.5), (6.5, 11.0)],
                colors.run,
                Stroke::new(stroke.width * 0.5, colors.run),
            );
        }

        Icon::Ai => {
            let star = |cx: f32, cy: f32, r: f32, w: f32| {
                closed(
                    &[
                        (cx, cy - r),
                        (cx + w, cy - w),
                        (cx + r, cy),
                        (cx + w, cy + w),
                        (cx, cy + r),
                        (cx - w, cy + w),
                        (cx - r, cy),
                        (cx - w, cy - w),
                    ],
                    Color32::TRANSPARENT,
                    stroke,
                );
            };
            star(7.0, 9.0, 6.0, 1.5);
            star(12.8, 3.2, 2.2, 0.6);
        }
        Icon::Plus => {
            line(&[(8.0, 3.0), (8.0, 13.0)]);
            line(&[(3.0, 8.0), (13.0, 8.0)]);
        }
        Icon::Copy => {
            closed(
                &[(5.5, 5.5), (13.5, 5.5), (13.5, 14.5), (5.5, 14.5)],
                Color32::TRANSPARENT,
                stroke,
            );
            line(&[
                (3.0, 11.5),
                (2.5, 11.5),
                (2.5, 2.5),
                (10.5, 2.5),
                (10.5, 3.0),
            ]);
        }
        Icon::Send => {
            line(&[(8.0, 13.5), (8.0, 2.5)]);
            line(&[(3.5, 7.0), (8.0, 2.5), (12.5, 7.0)]);
        }
    }
}

/// Allocates a square icon of `size` points and paints it.
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, color: impl Into<Color32>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    paint(ui.painter(), rect, icon, color);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn glyph_of(name: &str) -> Option<(Glyph, &'static str)> {
        match Icon::for_file(name) {
            Icon::FileBadge(badge) => Some((badge.glyph, badge.label)),
            _ => None,
        }
    }

    #[test]
    fn file_icons_follow_names_then_extensions() {
        use Glyph::*;
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
            assert_eq!(Icon::for_file(name), Icon::File, "{name}");
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
