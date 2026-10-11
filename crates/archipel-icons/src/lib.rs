//! # Archipel Icons
//!
//! A designed 16×16 outline icon set for developer tools, stored as data: every [`Glyph`] is
//! a short list of [`Part`]s (lines, polygons, circles, arcs, rounded rects) on a 16-unit
//! grid. The same data renders crisply with egui at any size (feature `egui`) and exports
//! as SVG ([`Glyph::to_svg`]) for websites, design tools and docs.
//!
//! ```
//! use archipel_icons::{Category, Glyph, Tones};
//!
//! assert_eq!(Glyph::Search.name(), "search");
//! assert_eq!(Glyph::Search.category(), Category::Actions);
//! assert_eq!(Glyph::from_name("search"), Some(Glyph::Search));
//! let svg = Glyph::Search.to_svg(16.0, &Tones::dark());
//! assert!(svg.starts_with("<svg"));
//! ```
//!
//! ## Design rules
//!
//! The rules are checked by the crate's tests (`tests/lint.rs`); see `docs/icons/design.md`.
//!
//! - 16×16 grid with a 1-unit margin: everything stays inside 1..15.
//! - Strokes are 1 unit wide with round caps and joins; horizontal and vertical strokes sit
//!   on half units (x.5) so they cover whole pixels at 1×.
//! - Filled shapes have their straight horizontal and vertical edges on whole units, and
//!   filled polygons are convex.
//! - Glyphs are drawn in one color ([`Tone::Ink`]); status glyphs use semantic tones
//!   (error, warning, ...) and [`Tone::Knockout`] for marks cut out of a filled shape.

mod glyphs;
mod languages;
#[cfg(feature = "egui")]
mod paint;
mod svg;
mod symbols;

pub use languages::{Language, LanguageKind};
pub use svg::parts_to_svg;
pub use symbols::{Symbol, SymbolKind, SymbolShape, Visibility};

#[cfg(feature = "egui")]
pub use paint::{paint, paint_language, paint_parts, paint_symbol, show};

/// A color role. Renderers map tones to concrete colors with [`Tones`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    /// The color the caller passes: the glyph's main color.
    Ink,
    /// The background the glyph sits on, for marks cut out of filled shapes.
    Knockout,
    /// Errors (red).
    Error,
    /// Warnings (amber).
    Warning,
    /// Weak warnings and muted status.
    Muted,
    /// Success (green).
    Success,
    /// Information (blue).
    Info,
    /// Run actions (green).
    Run,
    /// Stop actions and breakpoints (red).
    Stop,
    /// A fixed color, `0xRRGGBB` (language bands and their labels).
    Rgb(u32),
}

/// Concrete colors for the semantic tones, as `0xRRGGBB` (ink comes from the caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tones {
    /// Background behind the glyph.
    pub knockout: u32,
    /// Error red.
    pub error: u32,
    /// Warning amber.
    pub warning: u32,
    /// Muted status.
    pub muted: u32,
    /// Success green.
    pub success: u32,
    /// Information blue.
    pub info: u32,
    /// Run green.
    pub run: u32,
    /// Stop red.
    pub stop: u32,
    /// Default ink, used by [`Glyph::to_svg`].
    pub ink: u32,
}

impl Tones {
    /// Tones on a dark background.
    pub const fn dark() -> Self {
        Self {
            knockout: 0x191A1C,
            error: 0xF57E84,
            warning: 0xD59637,
            muted: 0x857042,
            success: 0x6DB083,
            info: 0x71A1FE,
            run: 0x4E9D6C,
            stop: 0xC54E58,
            ink: 0xCED0D6,
        }
    }

    /// Tones on a light background.
    pub const fn light() -> Self {
        Self {
            knockout: 0xFFFFFF,
            error: 0xC54E58,
            warning: 0xA56906,
            muted: 0xA88B4A,
            success: 0x338555,
            info: 0x2F5EB9,
            run: 0x369650,
            stop: 0xE4656E,
            ink: 0x73767C,
        }
    }

    /// The color of `tone`; `Ink` resolves to `ink`.
    pub const fn get(&self, tone: Tone, ink: u32) -> u32 {
        match tone {
            Tone::Ink => ink,
            Tone::Knockout => self.knockout,
            Tone::Error => self.error,
            Tone::Warning => self.warning,
            Tone::Muted => self.muted,
            Tone::Success => self.success,
            Tone::Info => self.info,
            Tone::Run => self.run,
            Tone::Stop => self.stop,
            Tone::Rgb(value) => value,
        }
    }
}

/// A point on the 16×16 grid.
pub type Point = (f32, f32);

/// A primitive on the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// An open polyline.
    Line(&'static [Point]),
    /// A closed polygon (convex if filled).
    Polygon(&'static [Point]),
    /// A circle.
    Circle {
        /// Center.
        center: Point,
        /// Radius.
        radius: f32,
    },
    /// A circular arc from `from` to `to` degrees (0° points right, angles grow clockwise
    /// because y points down; `to < from` sweeps the other way).
    Arc {
        /// Center.
        center: Point,
        /// Radius.
        radius: f32,
        /// Start angle in degrees.
        from: f32,
        /// End angle in degrees.
        to: f32,
    },
    /// A short bold label (language and type badges), centered on `center` by its ink, in
    /// the renderer's bold UI font (egui: the font family named `semibold` when the app
    /// registered one, as Archipel Kit does; SVG: Inter, then the system UI font).
    Text {
        /// The label, one to three characters.
        text: &'static str,
        /// Center of the label's ink.
        center: Point,
        /// Font size in grid units.
        size: f32,
    },
    /// An axis-aligned rectangle with rounded corners.
    Rect {
        /// Top-left corner.
        min: Point,
        /// Bottom-right corner.
        max: Point,
        /// Corner radius.
        radius: f32,
    },
}

/// How a [`Shape`] is painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Paint {
    /// Outline of the given width (in grid units) and tone.
    Stroke {
        /// Width in grid units (1 for regular strokes).
        width: f32,
        /// Color role.
        tone: Tone,
    },
    /// Filled with a tone.
    Fill(Tone),
}

/// One shape of a glyph and how it is painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    /// The geometry.
    pub shape: Shape,
    /// Stroke or fill.
    pub paint: Paint,
}

/// Groups of glyphs, for catalogs and pickers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    /// Arrows, chevrons, menus, window and layout controls.
    Navigation,
    /// Editing, searching, saving and sharing.
    Actions,
    /// Files and folders.
    Files,
    /// Run, debug, build and code.
    Development,
    /// Version control.
    Vcs,
    /// Errors, warnings, notifications.
    Status,
    /// Databases, servers, networks, APIs and security.
    Data,
    /// Everything else.
    General,
}

impl Category {
    /// Every category, in catalog order.
    pub const ALL: [Category; 8] = [
        Category::Navigation,
        Category::Actions,
        Category::Files,
        Category::Development,
        Category::Vcs,
        Category::Status,
        Category::Data,
        Category::General,
    ];

    /// Display name.
    pub const fn label(self) -> &'static str {
        match self {
            Category::Navigation => "Navigation",
            Category::Actions => "Actions",
            Category::Files => "Files",
            Category::Development => "Development",
            Category::Vcs => "Version control",
            Category::Status => "Status",
            Category::Data => "Data & network",
            Category::General => "General",
        }
    }

    /// The glyphs of this category, in catalog order.
    pub fn glyphs(self) -> impl Iterator<Item = Glyph> {
        Glyph::ALL
            .iter()
            .copied()
            .filter(move |glyph| glyph.category() == self)
    }
}

pub use glyphs::Glyph;

impl Glyph {
    /// The glyph named `name` (kebab-case, as [`Glyph::name`] returns it).
    pub fn from_name(name: &str) -> Option<Glyph> {
        Glyph::ALL
            .iter()
            .copied()
            .find(|glyph| glyph.name() == name)
    }

    /// Glyphs whose name or keywords contain `query` (case-insensitive), for pickers.
    pub fn search(query: &str) -> impl Iterator<Item = Glyph> + '_ {
        let query = query.to_lowercase();
        Glyph::ALL.iter().copied().filter(move |glyph| {
            glyph.name().contains(&query)
                || glyph
                    .keywords()
                    .iter()
                    .any(|keyword| keyword.contains(&query))
        })
    }
}

/// A label part: `text` centered on `center`, `size` units tall, in `tone`.
pub const fn label(text: &'static str, center: Point, size: f32, tone: Tone) -> Part {
    Part {
        shape: Shape::Text { text, center, size },
        paint: Paint::Fill(tone),
    }
}

impl Symbol {
    /// The icon as an SVG document of `size` pixels, in `tones`.
    pub fn to_svg(self, size: f32, tones: &Tones) -> String {
        parts_to_svg(&self.parts(), size, tones)
    }

    /// Kebab-case slug for file names, e.g. `"method-private-static"`.
    pub fn slug(self) -> String {
        let mut slug = self.kind.name().to_lowercase().replace(' ', "-");
        if self.is_abstract {
            slug.push_str("-abstract");
        }
        match self.visibility {
            Some(Visibility::Private) => slug.push_str("-private"),
            Some(Visibility::Protected) => slug.push_str("-protected"),
            Some(Visibility::Internal) => slug.push_str("-internal"),
            Some(Visibility::Public) | None => {}
        }
        if self.is_static {
            slug.push_str("-static");
        }
        slug
    }
}

impl Language {
    /// The language of a file, by exact file name (`Dockerfile`, `Makefile`), then by the
    /// longest matching extension (`build.gradle.kts` is Gradle Kotlin DSL, not Kotlin).
    pub fn from_file_name(name: &str) -> Option<Language> {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if let Some(language) = Language::ALL
            .iter()
            .copied()
            .find(|language| language.file_names().contains(&base))
        {
            return Some(language);
        }
        let lower = base.to_ascii_lowercase();
        Language::ALL
            .iter()
            .flat_map(|language| {
                language
                    .extensions()
                    .iter()
                    .map(move |ext| (*language, *ext))
            })
            .filter(|(_, ext)| {
                lower
                    .strip_suffix(ext)
                    .is_some_and(|rest| rest.len() > 1 && rest.ends_with('.'))
            })
            .max_by_key(|(_, ext)| ext.len())
            .map(|(language, _)| language)
    }

    /// The icon: a file outline in the ink color with a band in the language color
    /// carrying its label.
    pub fn parts(self) -> Vec<Part> {
        let ink = |points| Part {
            shape: Shape::Line(points),
            paint: Paint::Stroke {
                width: 1.0,
                tone: Tone::Ink,
            },
        };
        let mut parts = vec![
            // The page stops above the band, with half a unit of air between them.
            ink(&[(3.5, 6.5), (3.5, 1.5), (9.5, 1.5), (12.5, 4.5), (12.5, 6.5)]),
            ink(&[(9.5, 1.5), (9.5, 4.5), (12.5, 4.5)]),
            Part {
                shape: Shape::Rect {
                    min: (1.0, 7.5),
                    max: (15.0, 15.0),
                    radius: 2.0,
                },
                paint: Paint::Fill(Tone::Rgb(self.color())),
            },
        ];
        if let Some(outline) = self.outline() {
            parts.push(Part {
                shape: Shape::Rect {
                    min: (1.5, 8.0),
                    max: (14.5, 14.5),
                    radius: 1.5,
                },
                paint: Paint::Stroke {
                    width: 1.0,
                    tone: Tone::Rgb(outline),
                },
            });
        }
        let label_color = if self.dark_label() {
            0x000000
        } else {
            0xFFFFFF
        };
        let size = if self.label().chars().count() <= 2 {
            7.4
        } else {
            6.2
        };
        parts.push(label(
            self.label(),
            (8.0, 11.25),
            size,
            Tone::Rgb(label_color),
        ));
        parts
    }

    /// An outline for bands that would melt into a dark or a light background: the band
    /// color mixed towards mid-gray, so it reads on both.
    fn outline(self) -> Option<u32> {
        let color = self.color();
        let melts = contrast(color, Tones::dark().knockout) < 1.6
            || contrast(color, Tones::light().knockout) < 1.3;
        melts.then(|| mix(color, 0x8C8F96, 0.5))
    }

    /// The icon as an SVG document of `size` pixels.
    pub fn to_svg(self, size: f32) -> String {
        parts_to_svg(&self.parts(), size, &Tones::dark())
    }

    /// Kebab-case slug for file names, e.g. `"c-sharp"`.
    pub fn slug(self) -> String {
        let mut slug = String::new();
        for c in self.name().chars() {
            match c {
                '+' => slug.push_str("plus"),
                '#' => slug.push_str("-sharp"),
                c if c.is_ascii_alphanumeric() => slug.push(c.to_ascii_lowercase()),
                _ if !slug.ends_with('-') => slug.push('-'),
                _ => {}
            }
        }
        slug.trim_matches('-').replace("--", "-")
    }
}

/// WCAG 2 contrast ratio of two `0xRRGGBB` colors.
pub fn contrast(a: u32, b: u32) -> f32 {
    fn luminance(value: u32) -> f32 {
        let channel = |shift: u32| {
            let c = ((value >> shift) & 0xFF) as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xFF) as f32, ((b >> shift) & 0xFF) as f32);
        ((x + (y - x) * t).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// Points of an arc, `segments` per 90°, for renderers that draw polylines.
pub fn arc_points(center: Point, radius: f32, from: f32, to: f32) -> Vec<Point> {
    let sweep = to - from;
    let steps = ((sweep.abs() / 90.0) * 8.0).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|i| {
            let angle = (from + sweep * i as f32 / steps as f32).to_radians();
            (
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            )
        })
        .collect()
}
