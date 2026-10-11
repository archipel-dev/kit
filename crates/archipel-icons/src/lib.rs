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
#[cfg(feature = "egui")]
mod paint;
mod svg;

#[cfg(feature = "egui")]
pub use paint::{paint, show};

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
