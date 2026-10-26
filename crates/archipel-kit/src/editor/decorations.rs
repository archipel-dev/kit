//! Data the caller hands to an [`EditorComponent`](crate::editor::EditorComponent) and what it
//! reports back: gutter icons and change markers, highlighted ranges for the error stripe,
//! clicks and navigation requests. All of it is plain data: the component keeps none of it
//! between frames and decides no policy (what a breakpoint is, where Go to Declaration goes).

use crate::{Glyph, Tone};
use egui::{Modifiers, PointerButton, Pos2};
use std::ops::Range;

/// How serious a [`RangeHighlight`] is (IntelliJ `HighlightSeverity`); it decides the squiggle
/// in the text and the color of the mark on the error stripe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HighlightSeverity {
    /// An error: red squiggle and stripe mark.
    Error,
    /// A warning: amber squiggle and stripe mark.
    Warning,
    /// A weak warning: a faint squiggle, no stripe mark color of its own.
    WeakWarning,
    /// Information: only a stripe mark.
    Info,
    /// A text search result: only a stripe mark (the find bar's matches are marked by the
    /// component itself).
    TextSearch,
}

/// A range of the text the caller wants marked (the output of an inspection or a search): the
/// component paints the squiggle, a mark on the error stripe, and reports the pointer resting
/// on it. IntelliJ `RangeHighlighter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeHighlight {
    /// Byte range in the document.
    pub range: Range<usize>,
    /// The severity.
    pub severity: HighlightSeverity,
    /// What the tooltip says; the component reports the hover, the caller shows it.
    pub message: Option<String>,
}

impl RangeHighlight {
    /// A highlight of `range`.
    pub fn new(range: Range<usize>, severity: HighlightSeverity) -> Self {
        Self {
            range,
            severity,
            message: None,
        }
    }

    /// The tooltip message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// An icon in one of the gutter's lanes (a breakpoint, a run arrow, a bookmark). IntelliJ
/// `GutterIconRenderer`. The component draws it and reports clicks as
/// [`GutterClick`]s; what it means is the caller's.
#[derive(Debug, Clone, PartialEq)]
pub struct GutterIcon {
    /// The logical line (0-based) it sits on.
    pub line: usize,
    /// The lane, 0 being the one next to the line numbers. Icons of other lines may share a
    /// lane.
    pub lane: usize,
    /// The glyph.
    pub glyph: Glyph,
    /// Its color.
    pub tone: Tone,
    /// The accessible name ("Breakpoint on line 12").
    pub name: String,
}

impl GutterIcon {
    /// An icon in lane 0.
    pub fn new(line: usize, glyph: Glyph, name: impl Into<String>) -> Self {
        Self {
            line,
            lane: 0,
            glyph,
            tone: Tone::Icon,
            name: name.into(),
        }
    }

    /// Puts the icon in `lane`.
    pub fn lane(mut self, lane: usize) -> Self {
        self.lane = lane;
        self
    }

    /// Colors the icon.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// What a version control change marker says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// New lines.
    Added,
    /// Changed lines.
    Modified,
    /// Lines were removed after `lines.start`.
    Deleted,
}

/// A change marker in the strip left of the line numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeMarker {
    /// The logical lines (0-based) it covers; a deletion marks the line it happened before.
    pub lines: Range<usize>,
    /// What changed.
    pub kind: ChangeKind,
}

impl ChangeMarker {
    /// A marker over `lines`.
    pub fn new(lines: Range<usize>, kind: ChangeKind) -> Self {
        Self { lines, kind }
    }
}

/// Where in the gutter a click landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GutterArea {
    /// On the line number.
    LineNumber,
    /// On the icon in this lane.
    Icon(usize),
    /// On the change marker strip.
    ChangeMarker,
    /// Anywhere else in the gutter, the fold column included.
    Other,
}

/// A click in the gutter that the component did not use itself (a click on a fold chevron
/// folds, and a drag over the numbers selects lines).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GutterClick {
    /// The logical line (0-based).
    pub line: usize,
    /// Where.
    pub area: GutterArea,
    /// Which button.
    pub button: PointerButton,
    /// The modifiers held.
    pub modifiers: Modifiers,
}

/// What a Ctrl (Cmd on macOS) click asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKind {
    /// Ctrl/Cmd+click: Go to Declaration.
    Declaration,
    /// Ctrl/Cmd+Alt+click: Go to Implementation.
    Implementation,
}

/// A Ctrl (Cmd) click on the text: the editor only reports it; resolving a symbol and going
/// somewhere is the application's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationRequest {
    /// The byte offset clicked.
    pub offset: usize,
    /// What was asked.
    pub kind: NavigationKind,
}

/// The pointer resting on the text for [`EDITOR_TOOLTIP_DELAY`](crate::tokens::motion::EDITOR_TOOLTIP_DELAY):
/// show a tooltip for what is there.
#[derive(Debug, Clone, PartialEq)]
pub struct HoverInfo {
    /// The byte offset under the pointer.
    pub offset: usize,
    /// The pointer, in screen coordinates.
    pub pos: Pos2,
    /// The index (in the highlights passed to the component) of the first highlight containing
    /// `offset`.
    pub highlight: Option<usize>,
}

impl HoverInfo {
    /// What the pointer rests on, without its screen position: the renderer-independent part
    /// the terminal editor reports too.
    pub fn target(&self) -> super::logic::hover::HoverTarget {
        super::logic::hover::HoverTarget {
            offset: self.offset,
            highlight: self.highlight,
        }
    }
}
