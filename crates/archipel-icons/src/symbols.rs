//! Code symbol icons: classes, interfaces, methods, fields and their modifiers, for
//! structure views, code completion, outlines and search results.
//!
//! The shape says what kind of thing a symbol is, the letter which one, the color its group:
//!
//! - **Types** (class, interface, enum, record, annotation, struct, trait, object): a circle
//!   with a capital letter.
//! - **Members** (method, function, constructor, field, property, variable, constant,
//!   parameter, enum member, type parameter, lambda): a rounded square with a small letter.
//! - **Namespaces**: the package cube for packages, the layers glyph for modules.
//!
//! Modifiers are overlays: `abstract` draws the shape hollow, `static` adds a dot at the top
//! right and a [`Visibility`] adds a mark at the bottom right.

use crate::{Paint, Part, Point, Shape, Tone, label};

/// What a symbol is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    /// A class.
    Class,
    /// An interface.
    Interface,
    /// An enum.
    Enum,
    /// A record (data class).
    Record,
    /// An annotation type.
    Annotation,
    /// A struct.
    Struct,
    /// A trait.
    Trait,
    /// A singleton object (Kotlin `object`, Scala `object`).
    Object,
    /// A method.
    Method,
    /// A free function.
    Function,
    /// A constructor.
    Constructor,
    /// A field.
    Field,
    /// A property (field with accessors).
    Property,
    /// A local variable.
    Variable,
    /// A constant.
    Constant,
    /// A parameter.
    Parameter,
    /// An enum constant.
    EnumMember,
    /// A generic type parameter.
    TypeParameter,
    /// A lambda or closure.
    Lambda,
    /// A package.
    Package,
    /// A module or namespace.
    Module,
}

/// The shape family of a [`SymbolKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolShape {
    /// A circle: types.
    Type,
    /// A rounded square: members.
    Member,
    /// A namespace glyph: the package cube or the module layers.
    Namespace,
}

/// Who can see a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Visible everywhere (no mark).
    Public,
    /// Visible to subclasses: a diamond.
    Protected,
    /// Visible in the package or module (Java package-private, Kotlin `internal`): a ring.
    Internal,
    /// Visible in the declaring type only: a square.
    Private,
}

impl SymbolKind {
    /// Every kind, in catalog order.
    pub const ALL: [SymbolKind; 21] = [
        SymbolKind::Class,
        SymbolKind::Interface,
        SymbolKind::Enum,
        SymbolKind::Record,
        SymbolKind::Annotation,
        SymbolKind::Struct,
        SymbolKind::Trait,
        SymbolKind::Object,
        SymbolKind::Method,
        SymbolKind::Function,
        SymbolKind::Constructor,
        SymbolKind::Field,
        SymbolKind::Property,
        SymbolKind::Variable,
        SymbolKind::Constant,
        SymbolKind::Parameter,
        SymbolKind::EnumMember,
        SymbolKind::TypeParameter,
        SymbolKind::Lambda,
        SymbolKind::Package,
        SymbolKind::Module,
    ];

    /// Display name.
    pub const fn name(self) -> &'static str {
        match self {
            SymbolKind::Class => "Class",
            SymbolKind::Interface => "Interface",
            SymbolKind::Enum => "Enum",
            SymbolKind::Record => "Record",
            SymbolKind::Annotation => "Annotation",
            SymbolKind::Struct => "Struct",
            SymbolKind::Trait => "Trait",
            SymbolKind::Object => "Object",
            SymbolKind::Method => "Method",
            SymbolKind::Function => "Function",
            SymbolKind::Constructor => "Constructor",
            SymbolKind::Field => "Field",
            SymbolKind::Property => "Property",
            SymbolKind::Variable => "Variable",
            SymbolKind::Constant => "Constant",
            SymbolKind::Parameter => "Parameter",
            SymbolKind::EnumMember => "Enum member",
            SymbolKind::TypeParameter => "Type parameter",
            SymbolKind::Lambda => "Lambda",
            SymbolKind::Package => "Package",
            SymbolKind::Module => "Module",
        }
    }

    /// The shape family.
    pub const fn shape(self) -> SymbolShape {
        match self {
            SymbolKind::Class
            | SymbolKind::Interface
            | SymbolKind::Enum
            | SymbolKind::Record
            | SymbolKind::Annotation
            | SymbolKind::Struct
            | SymbolKind::Trait
            | SymbolKind::Object => SymbolShape::Type,
            SymbolKind::Package | SymbolKind::Module => SymbolShape::Namespace,
            _ => SymbolShape::Member,
        }
    }

    /// The letter on the shape (empty for namespaces).
    pub const fn label(self) -> &'static str {
        match self {
            SymbolKind::Class => "C",
            SymbolKind::Interface => "I",
            SymbolKind::Enum => "E",
            SymbolKind::Record => "R",
            SymbolKind::Annotation => "@",
            SymbolKind::Struct => "S",
            SymbolKind::Trait => "T",
            SymbolKind::Object => "O",
            SymbolKind::Method => "m",
            SymbolKind::Function => "ƒ",
            SymbolKind::Constructor => "c",
            SymbolKind::Field => "f",
            SymbolKind::Property => "p",
            SymbolKind::Variable => "v",
            SymbolKind::Constant => "k",
            SymbolKind::Parameter => "a",
            SymbolKind::EnumMember => "e",
            SymbolKind::TypeParameter => "t",
            SymbolKind::Lambda => "λ",
            SymbolKind::Package | SymbolKind::Module => "",
        }
    }

    /// Fill color as `0xRRGGBB`. Every color sits in the luminance band 0.13–0.18: white
    /// labels reach 4.5:1 and the shape reaches 3:1 on dark and light backgrounds.
    pub const fn color(self) -> u32 {
        match self {
            SymbolKind::Class => 0x246FC5,
            SymbolKind::Interface => 0x3A7D45,
            SymbolKind::Enum | SymbolKind::EnumMember => 0x9D6024,
            SymbolKind::Record => 0x247984,
            SymbolKind::Annotation => 0x8853CE,
            SymbolKind::Struct => 0x4B66D2,
            SymbolKind::Trait => 0x327C5C,
            SymbolKind::Object => 0x5165D1,
            SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Lambda => 0xCC344F,
            SymbolKind::Function => 0xB94183,
            SymbolKind::Field => 0x8C6911,
            SymbolKind::Property => 0x926530,
            SymbolKind::Variable | SymbolKind::Parameter => 0x5E7089,
            SymbolKind::Constant => 0x856D18,
            SymbolKind::TypeParameter => 0x6A5FB0,
            SymbolKind::Package | SymbolKind::Module => 0x87694A,
        }
    }

    /// The kind for an LSP `SymbolKind` (1-26, LSP 3.17), or `None` for kinds without a
    /// code symbol icon (file, string, number, boolean, array, key, null).
    pub const fn from_lsp(kind: u32) -> Option<SymbolKind> {
        Some(match kind {
            2 | 3 => SymbolKind::Module,
            4 => SymbolKind::Package,
            5 => SymbolKind::Class,
            6 => SymbolKind::Method,
            7 => SymbolKind::Property,
            8 => SymbolKind::Field,
            9 => SymbolKind::Constructor,
            10 => SymbolKind::Enum,
            11 => SymbolKind::Interface,
            12 => SymbolKind::Function,
            13 => SymbolKind::Variable,
            14 => SymbolKind::Constant,
            19 => SymbolKind::Object,
            22 => SymbolKind::EnumMember,
            23 => SymbolKind::Struct,
            24 => SymbolKind::Function, // Event: treated as a callable.
            25 => SymbolKind::Method,   // Operator: an operator method.
            26 => SymbolKind::TypeParameter,
            _ => return None,
        })
    }
}

/// A symbol icon: a kind plus its modifiers.
///
/// ```
/// use archipel_icons::{Symbol, SymbolKind, Visibility};
///
/// let icon = Symbol::new(SymbolKind::Method).visibility(Visibility::Private).static_(true);
/// assert_eq!(icon.kind, SymbolKind::Method);
/// assert!(!icon.parts().is_empty());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol {
    /// What the symbol is.
    pub kind: SymbolKind,
    /// Who can see it (`None` and `Public` draw no mark).
    pub visibility: Option<Visibility>,
    /// `static`: a dot at the top right.
    pub is_static: bool,
    /// `abstract`: the shape drawn hollow.
    pub is_abstract: bool,
}

impl From<SymbolKind> for Symbol {
    fn from(kind: SymbolKind) -> Self {
        Symbol::new(kind)
    }
}

const fn fill(shape: Shape, tone: Tone) -> Part {
    Part {
        shape,
        paint: Paint::Fill(tone),
    }
}

const fn stroke(shape: Shape, tone: Tone) -> Part {
    Part {
        shape,
        paint: Paint::Stroke { width: 1.0, tone },
    }
}

const fn circle(center: Point, radius: f32) -> Shape {
    Shape::Circle { center, radius }
}

impl Symbol {
    /// A symbol of `kind` without modifiers.
    pub const fn new(kind: SymbolKind) -> Self {
        Self {
            kind,
            visibility: None,
            is_static: false,
            is_abstract: false,
        }
    }

    /// Sets the visibility mark.
    pub const fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = Some(visibility);
        self
    }

    /// Marks it `static`.
    pub const fn static_(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// Marks it `abstract`.
    pub const fn abstract_(mut self, is_abstract: bool) -> Self {
        self.is_abstract = is_abstract;
        self
    }

    /// The icon's parts on the 16×16 grid.
    pub fn parts(self) -> Vec<Part> {
        let color = Tone::Rgb(self.kind.color());
        let mut parts = Vec::new();
        match self.kind.shape() {
            SymbolShape::Type | SymbolShape::Member => {
                let shape = match self.kind.shape() {
                    SymbolShape::Type => circle((8.0, 8.0), 6.5),
                    _ => Shape::Rect {
                        min: (1.5, 1.5),
                        max: (14.5, 14.5),
                        radius: 3.5,
                    },
                };
                let label_tone = if self.is_abstract {
                    // Hollow: a ring in the color, the letter in the color.
                    parts.push(stroke(inset(shape), color));
                    color
                } else {
                    parts.push(fill(shape, color));
                    Tone::Rgb(0xFFFFFF)
                };
                let size = match self.kind.shape() {
                    SymbolShape::Type => 9.0,
                    _ => 9.5,
                };
                parts.push(label(self.kind.label(), (8.0, 8.0), size, label_tone));
            }
            SymbolShape::Namespace => {
                let glyph = match self.kind {
                    SymbolKind::Module => crate::Glyph::Layers,
                    _ => crate::Glyph::Package,
                };
                for part in glyph.parts() {
                    parts.push(Part {
                        paint: Paint::Stroke {
                            width: 1.0,
                            tone: color,
                        },
                        ..*part
                    });
                }
            }
        }
        if self.is_static {
            parts.push(fill(circle((12.0, 4.0), 2.75), Tone::Knockout));
            parts.push(fill(circle((12.0, 4.0), 1.75), Tone::Ink));
        }
        match self.visibility {
            None | Some(Visibility::Public) => {}
            Some(visibility) => {
                parts.push(fill(circle((11.75, 11.75), 3.25), Tone::Knockout));
                parts.push(match visibility {
                    Visibility::Private => fill(
                        Shape::Rect {
                            min: (10.0, 10.0),
                            max: (13.5, 13.5),
                            radius: 0.5,
                        },
                        Tone::Ink,
                    ),
                    Visibility::Protected => fill(
                        Shape::Polygon(&[(11.75, 9.5), (14.0, 11.75), (11.75, 14.0), (9.5, 11.75)]),
                        Tone::Ink,
                    ),
                    _ => stroke(circle((11.75, 11.75), 1.5), Tone::Ink),
                });
            }
        }
        parts
    }
}

/// `shape` pulled in by half a unit, so a 1-unit ring covers the same area as the fill.
fn inset(shape: Shape) -> Shape {
    match shape {
        Shape::Circle { center, radius } => Shape::Circle {
            center,
            radius: radius - 0.5,
        },
        Shape::Rect { min, max, radius } => Shape::Rect {
            min: (min.0 + 0.5, min.1 + 0.5),
            max: (max.0 - 0.5, max.1 - 0.5),
            radius: radius - 0.5,
        },
        other => other,
    }
}
