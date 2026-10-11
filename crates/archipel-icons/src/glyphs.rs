//! The glyph designs. Coordinates are on the 16×16 grid; see the design rules in `lib.rs`.
//!
//! Conventions used below:
//! - Glyphs with a vertical or horizontal center stroke are centered on 7.5 (a 1-unit
//!   stroke cannot be centered on 8 and stay crisp).
//! - Containers (rects) span 1.5..14.5 or 2.5..13.5 with radius 1.5.
//! - Arrowheads are 3.5 units long at 45°.

use crate::{Category, Paint, Part, Point, Shape, Tone};

const fn ink(shape: Shape) -> Part {
    Part {
        shape,
        paint: Paint::Stroke {
            width: 1.0,
            tone: Tone::Ink,
        },
    }
}

const fn stroke(shape: Shape, width: f32, tone: Tone) -> Part {
    Part {
        shape,
        paint: Paint::Stroke { width, tone },
    }
}

const fn fill(shape: Shape, tone: Tone) -> Part {
    Part {
        shape,
        paint: Paint::Fill(tone),
    }
}

const fn line(points: &'static [Point]) -> Shape {
    Shape::Line(points)
}

const fn poly(points: &'static [Point]) -> Shape {
    Shape::Polygon(points)
}

const fn circle(x: f32, y: f32, radius: f32) -> Shape {
    Shape::Circle {
        center: (x, y),
        radius,
    }
}

const fn arc(x: f32, y: f32, radius: f32, from: f32, to: f32) -> Shape {
    Shape::Arc {
        center: (x, y),
        radius,
        from,
        to,
    }
}

const fn rect(x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) -> Shape {
    Shape::Rect {
        min: (x0, y0),
        max: (x1, y1),
        radius,
    }
}

const fn dot(x: f32, y: f32, radius: f32) -> Part {
    fill(circle(x, y, radius), Tone::Ink)
}

macro_rules! glyphs {
    ($(
        $category:ident {
            $( $(#[doc = $doc:literal])+ $name:ident $slug:literal [$($keyword:literal),*] { $($part:expr),+ $(,)? } )+
        }
    )+) => {
        /// A designed icon. See [`Glyph::ALL`] for the catalog and `docs/icons/` for images.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Glyph {
            $($( $(#[doc = $doc])+ $name, )+)+
        }

        impl Glyph {
            /// Every glyph, grouped by category, in catalog order.
            pub const ALL: &'static [Glyph] = &[$($(Glyph::$name,)+)+];

            /// Kebab-case name, e.g. `"chevron-down"` (stable: use it in configs and SVG
            /// file names).
            pub const fn name(self) -> &'static str {
                match self { $($(Glyph::$name => $slug,)+)+ }
            }

            /// The catalog group.
            pub const fn category(self) -> Category {
                match self { $($(Glyph::$name => Category::$category,)+)+ }
            }

            /// Search keywords besides the name.
            pub const fn keywords(self) -> &'static [&'static str] {
                match self { $($(Glyph::$name => &[$($keyword),*],)+)+ }
            }

            /// The shapes, painted in order.
            pub const fn parts(self) -> &'static [Part] {
                match self {
                    $($(Glyph::$name => {
                        const PARTS: &[Part] = &[$($part),+];
                        PARTS
                    })+)+
                }
            }
        }
    };
}

glyphs! {
    Navigation {
        /// Chevron pointing down: expanded node, dropdown.
        ChevronDown "chevron-down" ["expanded", "dropdown", "open"] {
            ink(line(&[(4.5, 6.0), (8.0, 9.5), (11.5, 6.0)])),
        }
        /// Chevron pointing up.
        ChevronUp "chevron-up" ["collapse", "close"] {
            ink(line(&[(4.5, 10.0), (8.0, 6.5), (11.5, 10.0)])),
        }
        /// Chevron pointing left: back, previous.
        ChevronLeft "chevron-left" ["back", "previous"] {
            ink(line(&[(10.0, 4.5), (6.5, 8.0), (10.0, 11.5)])),
        }
        /// Chevron pointing right: collapsed node, next.
        ChevronRight "chevron-right" ["collapsed", "next", "forward"] {
            ink(line(&[(6.0, 4.5), (9.5, 8.0), (6.0, 11.5)])),
        }
        /// Arrow pointing left.
        ArrowLeft "arrow-left" ["back"] {
            ink(line(&[(13.5, 8.0), (2.5, 8.0)])),
            ink(line(&[(6.0, 4.5), (2.5, 8.0), (6.0, 11.5)])),
        }
        /// Arrow pointing right.
        ArrowRight "arrow-right" ["forward", "next"] {
            ink(line(&[(2.5, 8.0), (13.5, 8.0)])),
            ink(line(&[(10.0, 4.5), (13.5, 8.0), (10.0, 11.5)])),
        }
        /// Arrow pointing up.
        ArrowUp "arrow-up" ["up"] {
            ink(line(&[(8.0, 13.5), (8.0, 2.5)])),
            ink(line(&[(4.5, 6.0), (8.0, 2.5), (11.5, 6.0)])),
        }
        /// Arrow pointing down.
        ArrowDown "arrow-down" ["down"] {
            ink(line(&[(8.0, 2.5), (8.0, 13.5)])),
            ink(line(&[(4.5, 10.0), (8.0, 13.5), (11.5, 10.0)])),
        }
        /// Box with an arrow leaving it: open in a browser or another window.
        ExternalLink "external-link" ["open", "browser", "new window"] {
            ink(line(&[(7.5, 2.5), (2.5, 2.5), (2.5, 13.5), (13.5, 13.5), (13.5, 8.5)])),
            ink(line(&[(7.5, 8.5), (13.5, 2.5)])),
            ink(line(&[(9.5, 2.5), (13.5, 2.5), (13.5, 6.5)])),
        }
        /// Three lines: main menu.
        Menu "menu" ["hamburger", "main menu"] {
            ink(line(&[(2.5, 4.0), (13.5, 4.0)])),
            ink(line(&[(2.5, 8.0), (13.5, 8.0)])),
            ink(line(&[(2.5, 12.0), (13.5, 12.0)])),
        }
        /// Three dots: overflow menu.
        More "more" ["overflow", "ellipsis", "options"] {
            dot(3.0, 8.0, 1.25),
            dot(8.0, 8.0, 1.25),
            dot(13.0, 8.0, 1.25),
        }
        /// Three vertical dots: overflow menu in a narrow column.
        MoreVertical "more-vertical" ["overflow", "kebab", "options"] {
            dot(8.0, 3.0, 1.25),
            dot(8.0, 8.0, 1.25),
            dot(8.0, 13.0, 1.25),
        }
        /// House: home, welcome screen.
        Home "home" ["welcome", "start"] {
            ink(line(&[(2.0, 8.0), (8.0, 2.0), (14.0, 8.0)])),
            ink(line(&[(4.0, 6.0), (4.0, 14.0), (12.0, 14.0), (12.0, 6.0)])),
            ink(line(&[(7.0, 14.0), (7.0, 10.0), (9.0, 10.0), (9.0, 14.0)])),
        }
        /// Two chevrons down: expand all.
        ExpandAll "expand-all" ["unfold", "tree"] {
            ink(line(&[(4.5, 3.0), (8.0, 6.5), (11.5, 3.0)])),
            ink(line(&[(4.5, 9.0), (8.0, 12.5), (11.5, 9.0)])),
        }
        /// Two chevrons up: collapse all.
        CollapseAll "collapse-all" ["fold", "tree"] {
            ink(line(&[(4.5, 7.0), (8.0, 3.5), (11.5, 7.0)])),
            ink(line(&[(4.5, 13.0), (8.0, 9.5), (11.5, 13.0)])),
        }
        /// Four corners: full screen.
        Fullscreen "fullscreen" ["maximize", "expand", "zen"] {
            ink(line(&[(2.5, 6.0), (2.5, 2.5), (6.0, 2.5)])),
            ink(line(&[(10.0, 2.5), (13.5, 2.5), (13.5, 6.0)])),
            ink(line(&[(13.5, 10.0), (13.5, 13.5), (10.0, 13.5)])),
            ink(line(&[(6.0, 13.5), (2.5, 13.5), (2.5, 10.0)])),
        }
        /// A horizontal bar: minimize, hide a panel.
        Minimize "minimize" ["hide", "collapse panel"] {
            ink(line(&[(3.5, 8.0), (12.5, 8.0)])),
        }
        /// An ×: close.
        Close "close" ["dismiss", "remove", "x"] {
            ink(line(&[(4.0, 4.0), (12.0, 12.0)])),
            ink(line(&[(12.0, 4.0), (4.0, 12.0)])),
        }
        /// Window with a left sidebar.
        SidebarLeft "sidebar-left" ["panel", "layout", "tool window"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(5.5, 2.5), (5.5, 13.5)])),
        }
        /// Window with a right sidebar.
        SidebarRight "sidebar-right" ["panel", "layout", "inspector"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(10.5, 2.5), (10.5, 13.5)])),
        }
        /// Window with a bottom panel.
        PanelBottom "panel-bottom" ["layout", "terminal panel", "console"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(1.5, 9.5), (14.5, 9.5)])),
        }
        /// Window split into two columns.
        SplitColumns "split-columns" ["split right", "side by side", "layout"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(7.5, 2.5), (7.5, 13.5)])),
        }
        /// Window split into two rows.
        SplitRows "split-rows" ["split down", "layout"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(1.5, 7.5), (14.5, 7.5)])),
        }
    }

    Actions {
        /// Plus: add, new.
        Plus "plus" ["add", "new", "create"] {
            ink(line(&[(8.0, 3.0), (8.0, 13.0)])),
            ink(line(&[(3.0, 8.0), (13.0, 8.0)])),
        }
        /// Minus: remove.
        Minus "minus" ["remove", "subtract"] {
            ink(line(&[(3.0, 8.0), (13.0, 8.0)])),
        }
        /// Plus in a box: add an item to a list.
        AddBox "add-box" ["add", "new", "plus"] {
            ink(rect(2.0, 2.0, 14.0, 14.0, 1.5)),
            ink(line(&[(8.0, 5.0), (8.0, 11.0)])),
            ink(line(&[(5.0, 8.0), (11.0, 8.0)])),
        }
        /// Pencil: edit, rename.
        Edit "edit" ["pencil", "rename", "write"] {
            ink(poly(&[(10.5, 2.5), (13.5, 5.5), (5.5, 13.5), (2.5, 13.5), (2.5, 10.5)])),
            ink(line(&[(8.5, 4.5), (11.5, 7.5)])),
        }
        /// Trash can: delete.
        Delete "delete" ["trash", "remove", "bin"] {
            ink(line(&[(2.5, 4.5), (13.5, 4.5)])),
            ink(line(&[(6.5, 4.5), (6.5, 2.5), (9.5, 2.5), (9.5, 4.5)])),
            ink(line(&[(3.5, 4.5), (4.5, 13.5), (11.5, 13.5), (12.5, 4.5)])),
            ink(line(&[(6.5, 7.0), (6.5, 11.0)])),
            ink(line(&[(9.5, 7.0), (9.5, 11.0)])),
        }
        /// Two sheets: copy.
        Copy "copy" ["duplicate", "clipboard"] {
            ink(line(&[(2.5, 10.5), (2.5, 2.5), (10.5, 2.5)])),
            ink(rect(5.5, 5.5, 13.5, 13.5, 1.5)),
        }
        /// Clipboard: paste.
        Paste "paste" ["clipboard"] {
            ink(line(&[(5.5, 3.5), (3.5, 3.5), (3.5, 14.5), (12.5, 14.5), (12.5, 3.5), (10.5, 3.5)])),
            ink(rect(5.5, 1.5, 10.5, 4.5, 1.0)),
        }
        /// Disk: save.
        Save "save" ["disk", "store"] {
            ink(rect(2.5, 2.5, 13.5, 13.5, 1.5)),
            ink(line(&[(5.5, 2.5), (5.5, 5.5), (10.5, 5.5), (10.5, 2.5)])),
            ink(line(&[(4.5, 13.5), (4.5, 9.5), (11.5, 9.5), (11.5, 13.5)])),
        }
        /// Curved arrow back: undo, revert, rollback.
        Undo "undo" ["revert", "rollback", "back"] {
            ink(line(&[(2.5, 5.5), (9.5, 5.5)])),
            ink(arc(9.5, 9.5, 4.0, -90.0, 90.0)),
            ink(line(&[(9.5, 13.5), (5.5, 13.5)])),
            ink(line(&[(5.5, 2.5), (2.5, 5.5), (5.5, 8.5)])),
        }
        /// Curved arrow forward: redo.
        Redo "redo" ["repeat", "forward"] {
            ink(line(&[(13.5, 5.5), (6.5, 5.5)])),
            ink(arc(6.5, 9.5, 4.0, 270.0, 90.0)),
            ink(line(&[(6.5, 13.5), (10.5, 13.5)])),
            ink(line(&[(10.5, 2.5), (13.5, 5.5), (10.5, 8.5)])),
        }
        /// Circular arrow: refresh, reload, sync.
        Refresh "refresh" ["reload", "sync", "update"] {
            ink(arc(8.0, 8.5, 5.5, 20.0, 320.0)),
            ink(line(&[(12.5, 2.0), (12.5, 5.0), (9.5, 5.0)])),
        }
        /// Magnifier: search.
        Search "search" ["find", "magnifier", "lookup"] {
            ink(circle(7.0, 7.0, 4.5)),
            ink(line(&[(10.5, 10.5), (13.5, 13.5)])),
        }
        /// Magnifier over lines: find in files.
        FindInFiles "find-in-files" ["search everywhere", "grep"] {
            ink(line(&[(2.5, 2.5), (12.5, 2.5)])),
            ink(line(&[(2.5, 5.5), (6.5, 5.5)])),
            ink(line(&[(2.5, 8.5), (5.5, 8.5)])),
            ink(circle(10.0, 9.0, 2.75)),
            ink(line(&[(12.0, 11.0), (14.0, 13.0)])),
        }
        /// Funnel: filter.
        Filter "filter" ["funnel", "narrow"] {
            ink(poly(&[(2.5, 3.0), (13.5, 3.0), (9.5, 8.0), (9.5, 13.0), (6.5, 11.0), (6.5, 8.0)])),
        }
        /// Bars of decreasing length: sort.
        Sort "sort" ["order", "arrange"] {
            ink(line(&[(2.5, 4.0), (13.5, 4.0)])),
            ink(line(&[(2.5, 8.0), (10.5, 8.0)])),
            ink(line(&[(2.5, 12.0), (7.5, 12.0)])),
        }
        /// Gear: settings, preferences, task.
        Settings "settings" ["gear", "preferences", "options", "configure"] {
            ink(poly(&[(6.65, 3.45), (6.92, 1.5), (9.08, 1.5), (9.35, 3.45), (11.27, 4.55), (13.01, 3.86), (14.09, 5.73), (12.5, 6.89), (12.5, 9.11), (14.09, 10.27), (13.01, 12.14), (11.27, 11.45), (9.35, 12.55), (9.08, 14.5), (6.92, 14.5), (6.65, 12.55), (4.73, 11.45), (2.99, 12.14), (1.91, 10.27), (3.5, 9.11), (3.5, 6.89), (1.91, 5.73), (2.99, 3.86), (4.73, 4.55)])),
            ink(circle(8.0, 8.0, 2.25)),
        }
        /// Thumbtack: pin.
        Pin "pin" ["thumbtack", "keep", "stick"] {
            ink(poly(&[(5.5, 2.5), (10.5, 2.5), (9.5, 6.5), (11.5, 9.5), (4.5, 9.5), (6.5, 6.5)])),
            ink(line(&[(8.0, 9.5), (8.0, 14.0)])),
        }
        /// Star: favorite, bookmark.
        Star "star" ["favorite", "bookmark", "rate"] {
            ink(poly(&[
                (8.0, 2.1), (9.59, 6.42), (14.18, 6.59), (10.57, 9.43), (11.82, 13.86),
                (8.0, 11.3), (4.18, 13.86), (5.43, 9.43), (1.82, 6.59), (6.41, 6.42),
            ])),
        }
        /// Closed padlock: locked, read-only.
        Lock "lock" ["locked", "read-only", "secure"] {
            ink(rect(3.5, 7.5, 12.5, 13.5, 1.5)),
            ink(line(&[(5.5, 7.5), (5.5, 5.0)])),
            ink(arc(8.0, 5.0, 2.5, 180.0, 360.0)),
            ink(line(&[(10.5, 5.0), (10.5, 7.5)])),
            dot(8.0, 10.5, 1.0),
        }
        /// Open padlock: unlocked, writable.
        Unlock "unlock" ["unlocked", "writable"] {
            ink(rect(3.5, 7.5, 12.5, 13.5, 1.5)),
            ink(line(&[(5.5, 7.5), (5.5, 5.0)])),
            ink(arc(8.0, 5.0, 2.5, 180.0, 330.0)),
            dot(8.0, 10.5, 1.0),
        }
        /// Eye: show, preview, visible.
        Eye "eye" ["show", "preview", "visible", "watch"] {
            ink(arc(8.0, 10.44, 6.94, 200.6, 339.4)),
            ink(arc(8.0, 5.56, 6.94, 20.6, 159.4)),
            ink(circle(8.0, 8.0, 2.0)),
        }
        /// Crossed-out eye: hide, hidden.
        EyeOff "eye-off" ["hide", "hidden", "invisible"] {
            ink(arc(8.0, 10.44, 6.94, 200.6, 339.4)),
            ink(arc(8.0, 5.56, 6.94, 20.6, 159.4)),
            ink(circle(8.0, 8.0, 2.0)),
            ink(line(&[(2.5, 2.5), (13.5, 13.5)])),
        }
        /// Arrow into a tray: download, import.
        Download "download" ["import", "save to disk"] {
            ink(line(&[(8.0, 2.5), (8.0, 10.0)])),
            ink(line(&[(4.5, 6.5), (8.0, 10.0), (11.5, 6.5)])),
            ink(line(&[(3.0, 10.5), (3.0, 13.5), (13.0, 13.5), (13.0, 10.5)])),
        }
        /// Arrow out of a tray: upload, export, push.
        Upload "upload" ["export", "push", "publish"] {
            ink(line(&[(8.0, 10.0), (8.0, 2.5)])),
            ink(line(&[(4.5, 6.0), (8.0, 2.5), (11.5, 6.0)])),
            ink(line(&[(3.0, 10.5), (3.0, 13.5), (13.0, 13.5), (13.0, 10.5)])),
        }
        /// Two chain links: link, URL.
        Link "link" ["url", "chain", "hyperlink"] {
            ink(rect(1.5, 5.5, 9.5, 10.5, 2.5)),
            ink(rect(6.5, 5.5, 14.5, 10.5, 2.5)),
        }
        /// Paper plane: send.
        Send "send" ["submit", "paper plane", "execute request"] {
            ink(poly(&[(1.5, 7.0), (14.0, 2.0), (10.0, 14.0), (7.5, 9.0)])),
            ink(line(&[(7.5, 9.0), (14.0, 2.0)])),
        }
    }

    Files {
        /// Blank page.
        File "file" ["document", "page"] {
            ink(poly(&[(3.5, 1.5), (9.5, 1.5), (12.5, 4.5), (12.5, 14.5), (3.5, 14.5)])),
            ink(line(&[(9.5, 1.5), (9.5, 4.5), (12.5, 4.5)])),
        }
        /// Page with lines: text, document.
        FileText "file-text" ["document", "text", "log file"] {
            ink(poly(&[(3.5, 1.5), (9.5, 1.5), (12.5, 4.5), (12.5, 14.5), (3.5, 14.5)])),
            ink(line(&[(9.5, 1.5), (9.5, 4.5), (12.5, 4.5)])),
            ink(line(&[(5.5, 7.5), (10.5, 7.5)])),
            ink(line(&[(5.5, 9.5), (10.5, 9.5)])),
            ink(line(&[(5.5, 11.5), (8.5, 11.5)])),
        }
        /// Page with angle brackets: source file.
        FileCode "file-code" ["source", "code file"] {
            ink(poly(&[(3.5, 1.5), (9.5, 1.5), (12.5, 4.5), (12.5, 14.5), (3.5, 14.5)])),
            ink(line(&[(9.5, 1.5), (9.5, 4.5), (12.5, 4.5)])),
            ink(line(&[(6.5, 7.0), (4.75, 9.5), (6.5, 12.0)])),
            ink(line(&[(9.5, 7.0), (11.25, 9.5), (9.5, 12.0)])),
        }
        /// Page with a plus: new file.
        NewFile "new-file" ["create file", "add file"] {
            ink(line(&[(11.0, 6.5), (11.0, 4.5), (8.0, 1.5), (2.0, 1.5), (2.0, 13.5), (6.0, 13.5)])),
            ink(line(&[(8.0, 1.5), (8.0, 4.5), (11.0, 4.5)])),
            ink(line(&[(11.0, 8.5), (11.0, 14.5)])),
            ink(line(&[(8.0, 11.5), (14.0, 11.5)])),
        }
        /// Closed folder.
        Folder "folder" ["directory"] {
            ink(poly(&[(1.5, 3.0), (6.0, 3.0), (7.5, 5.0), (14.5, 5.0), (14.5, 13.0), (1.5, 13.0)])),
        }
        /// Open folder.
        FolderOpen "folder-open" ["directory", "open project"] {
            ink(line(&[(1.5, 13.0), (1.5, 3.0), (6.0, 3.0), (7.5, 5.0), (12.5, 5.0), (12.5, 7.0)])),
            ink(poly(&[(1.5, 13.0), (3.5, 7.0), (14.5, 7.0), (12.5, 13.0)])),
        }
        /// Folder with a plus: new folder.
        NewFolder "new-folder" ["create directory"] {
            ink(line(&[(8.5, 12.5), (1.5, 12.5), (1.5, 2.5), (6.0, 2.5), (7.5, 4.5), (14.5, 4.5), (14.5, 7.5)])),
            ink(line(&[(11.5, 8.0), (11.5, 13.0)])),
            ink(line(&[(9.0, 10.5), (14.0, 10.5)])),
        }
        /// Folder with a bar: project root.
        Project "project" ["workspace", "root"] {
            ink(poly(&[(1.5, 3.0), (6.0, 3.0), (7.5, 5.0), (14.5, 5.0), (14.5, 13.0), (1.5, 13.0)])),
            ink(line(&[(1.5, 8.0), (14.5, 8.0)])),
        }
        /// Picture: image file.
        Image "image" ["picture", "photo", "png"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(1.5, 11.5), (5.5, 7.5), (9.0, 11.0), (11.0, 9.0), (14.5, 12.5)])),
            ink(circle(10.5, 5.5, 1.25)),
        }
        /// Box with a lid: archive, zip, jar.
        Archive "archive" ["zip", "jar", "compressed", "box"] {
            ink(rect(1.5, 2.5, 14.5, 5.5, 1.0)),
            ink(line(&[(2.5, 5.5), (2.5, 13.5), (13.5, 13.5), (13.5, 5.5)])),
            ink(line(&[(6.5, 8.5), (9.5, 8.5)])),
        }
    }

    Development {
        /// Play triangle: run.
        Run "run" ["play", "start", "execute"] {
            fill(poly(&[(3.5, 2.5), (12.5, 8.0), (3.5, 13.5)]), Tone::Run),
        }
        /// Bug: debug.
        Debug "debug" ["bug", "debugger"] {
            ink(arc(8.0, 4.5, 2.0, 180.0, 360.0)),
            ink(rect(5.0, 4.5, 11.0, 12.5, 3.0)),
            ink(line(&[(8.0, 7.5), (8.0, 12.5)])),
            ink(line(&[(5.0, 6.5), (2.5, 5.0)])),
            ink(line(&[(5.0, 9.5), (2.5, 9.5)])),
            ink(line(&[(5.0, 11.5), (2.5, 13.0)])),
            ink(line(&[(11.0, 6.5), (13.5, 5.0)])),
            ink(line(&[(11.0, 9.5), (13.5, 9.5)])),
            ink(line(&[(11.0, 11.5), (13.5, 13.0)])),
        }
        /// Square: stop.
        Stop "stop" ["terminate", "halt"] {
            fill(rect(3.0, 3.0, 13.0, 13.0, 1.5), Tone::Stop),
        }
        /// Two bars: pause.
        Pause "pause" ["suspend", "hold"] {
            fill(rect(4.0, 3.0, 7.0, 13.0, 1.0), Tone::Ink),
            fill(rect(9.0, 3.0, 12.0, 13.0, 1.0), Tone::Ink),
        }
        /// Circular arrow around a play triangle: rerun.
        Rerun "rerun" ["restart", "run again"] {
            ink(arc(8.0, 8.5, 5.5, 20.0, 320.0)),
            ink(line(&[(12.5, 2.0), (12.5, 5.0), (9.5, 5.0)])),
            fill(poly(&[(6.0, 6.0), (10.5, 8.5), (6.0, 11.0)]), Tone::Run),
        }
        /// Arc over a dot: step over.
        StepOver "step-over" ["debugger", "next line"] {
            ink(arc(6.5, 7.5, 4.5, 180.0, 360.0)),
            ink(line(&[(8.5, 5.5), (11.0, 8.0), (13.5, 5.5)])),
            dot(6.5, 12.0, 1.5),
        }
        /// Arrow down to a dot: step into.
        StepInto "step-into" ["debugger", "enter"] {
            ink(line(&[(8.0, 2.0), (8.0, 9.0)])),
            ink(line(&[(5.0, 6.0), (8.0, 9.0), (11.0, 6.0)])),
            dot(8.0, 13.0, 1.5),
        }
        /// Arrow up from a dot: step out.
        StepOut "step-out" ["debugger", "return"] {
            ink(line(&[(8.0, 9.5), (8.0, 2.0)])),
            ink(line(&[(5.0, 5.0), (8.0, 2.0), (11.0, 5.0)])),
            dot(8.0, 13.0, 1.5),
        }
        /// Red dot: breakpoint.
        Breakpoint "breakpoint" ["debugger", "stop point"] {
            fill(circle(8.0, 8.0, 5.0), Tone::Stop),
        }
        /// Hammer: build, compile.
        Build "build" ["hammer", "compile", "make"] {
            ink(poly(&[(6.0, 5.0), (9.0, 2.0), (14.0, 7.0), (11.0, 10.0)])),
            ink(line(&[(8.0, 8.0), (2.0, 14.0)])),
        }
        /// Prompt in a window: terminal.
        Terminal "terminal" ["console", "shell", "command line"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(4.5, 6.0), (6.5, 8.0), (4.5, 10.0)])),
            ink(line(&[(8.5, 10.5), (11.5, 10.5)])),
        }
        /// Angle brackets with a slash: code.
        Code "code" ["source", "brackets", "html"] {
            ink(line(&[(5.0, 4.5), (1.5, 8.0), (5.0, 11.5)])),
            ink(line(&[(11.0, 4.5), (14.5, 8.0), (11.0, 11.5)])),
            ink(line(&[(9.5, 3.5), (6.5, 12.5)])),
        }
        /// Flask: tests.
        Test "test" ["flask", "experiment", "unit test"] {
            ink(line(&[(5.5, 2.0), (10.5, 2.0)])),
            ink(line(&[(6.5, 2.0), (6.5, 6.5), (2.5, 14.0), (13.5, 14.0), (9.5, 6.5), (9.5, 2.0)])),
            ink(line(&[(4.1, 11.0), (11.9, 11.0)])),
        }
        /// Cube: package, module, dependency.
        Package "package" ["module", "dependency", "cube", "crate", "model"] {
            ink(poly(&[(8.0, 1.5), (14.0, 4.5), (14.0, 11.5), (8.0, 14.5), (2.0, 11.5), (2.0, 4.5)])),
            ink(line(&[(2.0, 4.5), (8.0, 7.5), (14.0, 4.5)])),
            ink(line(&[(8.0, 7.5), (8.0, 14.5)])),
        }
        /// Nested list: structure, outline.
        Structure "structure" ["outline", "hierarchy", "tree"] {
            ink(line(&[(2.5, 3.5), (13.5, 3.5)])),
            ink(line(&[(4.5, 3.5), (4.5, 12.5)])),
            ink(line(&[(4.5, 8.5), (13.5, 8.5)])),
            ink(line(&[(4.5, 12.5), (13.5, 12.5)])),
        }
        /// Circle with an exclamation mark: problems.
        Problems "problems" ["issues", "diagnostics", "errors"] {
            ink(circle(8.0, 8.0, 6.0)),
            ink(line(&[(8.0, 5.0), (8.0, 9.0)])),
            dot(8.0, 11.25, 0.9),
        }
        /// Stacked layers: variants, environments.
        Layers "layers" ["variants", "environments", "stack"] {
            ink(poly(&[(8.0, 2.0), (14.0, 5.0), (8.0, 8.0), (2.0, 5.0)])),
            ink(line(&[(2.0, 8.0), (8.0, 11.0), (14.0, 8.0)])),
            ink(line(&[(2.0, 11.0), (8.0, 14.0), (14.0, 11.0)])),
        }
        /// Lines of output: log.
        Log "log" ["output", "logcat", "console output"] {
            ink(line(&[(2.5, 3.5), (13.5, 3.5)])),
            ink(line(&[(2.5, 6.5), (11.5, 6.5)])),
            ink(line(&[(2.5, 9.5), (13.5, 9.5)])),
            ink(line(&[(2.5, 12.5), (9.5, 12.5)])),
        }
        /// Arrow down to a bar: scroll to end.
        ScrollEnd "scroll-end" ["bottom", "follow output"] {
            ink(line(&[(8.0, 2.5), (8.0, 10.5)])),
            ink(line(&[(4.5, 7.0), (8.0, 10.5), (11.5, 7.0)])),
            ink(line(&[(3.0, 13.5), (13.0, 13.5)])),
        }
        /// Wrapping line: soft wrap.
        SoftWrap "soft-wrap" ["word wrap", "wrap lines"] {
            ink(line(&[(2.5, 2.5), (13.5, 2.5)])),
            ink(line(&[(2.5, 6.5), (11.0, 6.5)])),
            ink(arc(11.0, 9.0, 2.5, 270.0, 450.0)),
            ink(line(&[(11.0, 11.5), (8.5, 11.5)])),
            ink(line(&[(10.0, 10.0), (8.5, 11.5), (10.0, 13.0)])),
            ink(line(&[(2.5, 11.5), (5.5, 11.5)])),
        }
        /// Phone outline: device, emulator.
        Device "device" ["phone", "mobile", "emulator"] {
            ink(rect(4.5, 1.5, 11.5, 14.5, 1.5)),
            ink(line(&[(6.5, 12.5), (9.5, 12.5)])),
        }
        /// Four-point sparkle: AI assistant.
        Sparkle "sparkle" ["ai", "assistant", "magic", "generate"] {
            ink(poly(&[(7.0, 3.5), (8.3, 7.7), (12.5, 9.0), (8.3, 10.3), (7.0, 14.5), (5.7, 10.3), (1.5, 9.0), (5.7, 7.7)])),
            ink(line(&[(12.5, 1.5), (12.5, 5.5)])),
            ink(line(&[(10.5, 3.5), (14.5, 3.5)])),
        }
        /// Puzzle piece: plugin, extension.
        Plugin "plugin" ["extension", "puzzle", "add-on"] {
            ink(line(&[(5.25, 4.5), (2.5, 4.5), (2.5, 13.5), (11.5, 13.5), (11.5, 10.75)])),
            ink(arc(7.0, 4.5, 1.75, 180.0, 360.0)),
            ink(line(&[(8.75, 4.5), (11.5, 4.5), (11.5, 7.25)])),
            ink(arc(11.5, 9.0, 1.75, 270.0, 450.0)),
        }
    }

    Vcs {
        /// Two lines diverging: branch.
        Branch "branch" ["git", "fork"] {
            ink(circle(4.5, 3.5, 1.75)),
            ink(circle(4.5, 12.5, 1.75)),
            ink(circle(11.5, 4.5, 1.75)),
            ink(line(&[(4.5, 5.25), (4.5, 10.75)])),
            ink(line(&[(11.5, 6.25), (11.5, 7.5), (6.0, 10.8)])),
        }
        /// Dot on a line: commit.
        Commit "commit" ["git", "revision", "checkin"] {
            ink(circle(8.0, 8.0, 2.5)),
            ink(line(&[(2.0, 8.0), (5.5, 8.0)])),
            ink(line(&[(10.5, 8.0), (14.0, 8.0)])),
        }
        /// Two lines joining: merge.
        Merge "merge" ["git", "join"] {
            ink(circle(4.5, 3.5, 1.75)),
            ink(circle(4.5, 12.5, 1.75)),
            ink(circle(11.5, 12.5, 1.75)),
            ink(line(&[(4.5, 5.25), (4.5, 10.75)])),
            ink(line(&[(5.6, 4.9), (11.5, 10.75)])),
        }
        /// Branch with an arrow back: pull request, merge request.
        PullRequest "pull-request" ["git", "merge request", "review"] {
            ink(circle(4.5, 3.5, 1.75)),
            ink(circle(4.5, 12.5, 1.75)),
            ink(circle(11.5, 12.5, 1.75)),
            ink(line(&[(4.5, 5.25), (4.5, 10.75)])),
            ink(line(&[(11.5, 10.75), (11.5, 5.5), (7.5, 5.5)])),
            ink(line(&[(9.0, 4.0), (7.5, 5.5), (9.0, 7.0)])),
        }
        /// Two panes with minus and plus: diff, compare.
        Diff "diff" ["compare", "changes"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(7.5, 2.5), (7.5, 13.5)])),
            ink(line(&[(3.0, 7.5), (6.0, 7.5)])),
            ink(line(&[(9.0, 7.5), (12.0, 7.5)])),
            ink(line(&[(10.5, 6.0), (10.5, 9.0)])),
        }
        /// Clock with a back arrow: history, recent.
        History "history" ["recent", "log", "time"] {
            ink(arc(8.5, 8.0, 5.5, 180.0, 450.0)),
            ink(line(&[(2.5, 5.75), (3.0, 8.25), (5.25, 7.0)])),
            ink(line(&[(8.5, 5.0), (8.5, 8.5), (11.0, 10.0)])),
        }
        /// Tag: release, version.
        Tag "tag" ["release", "version", "label"] {
            ink(poly(&[(1.5, 1.5), (8.0, 1.5), (14.5, 8.0), (8.0, 14.5), (1.5, 8.0)])),
            ink(circle(5.0, 5.0, 1.25)),
        }
    }

    Status {
        /// Red circle with an exclamation mark: error.
        Error "error" ["failure", "problem", "danger"] {
            fill(circle(8.0, 8.0, 6.5), Tone::Error),
            stroke(line(&[(8.0, 4.5), (8.0, 8.75)]), 1.5, Tone::Knockout),
            fill(circle(8.0, 11.25, 1.0), Tone::Knockout),
        }
        /// Amber triangle with an exclamation mark: warning.
        Warning "warning" ["caution", "alert"] {
            fill(poly(&[(8.0, 1.5), (15.0, 14.0), (1.0, 14.0)]), Tone::Warning),
            stroke(line(&[(8.0, 5.75), (8.0, 9.5)]), 1.5, Tone::Knockout),
            fill(circle(8.0, 11.75, 0.9), Tone::Knockout),
        }
        /// Muted triangle: weak warning.
        WeakWarning "weak-warning" ["hint", "suggestion"] {
            fill(poly(&[(8.0, 1.5), (15.0, 14.0), (1.0, 14.0)]), Tone::Muted),
            stroke(line(&[(8.0, 5.75), (8.0, 9.5)]), 1.5, Tone::Knockout),
            fill(circle(8.0, 11.75, 0.9), Tone::Knockout),
        }
        /// Blue circle with an i: information.
        Info "info" ["information", "about", "details"] {
            fill(circle(8.0, 8.0, 6.5), Tone::Info),
            stroke(line(&[(8.0, 7.25), (8.0, 11.5)]), 1.5, Tone::Knockout),
            fill(circle(8.0, 4.75, 1.0), Tone::Knockout),
        }
        /// Green check mark: success, done.
        Success "success" ["ok", "done", "passed"] {
            stroke(line(&[(2.5, 8.5), (6.0, 12.0), (13.5, 4.5)]), 1.5, Tone::Success),
        }
        /// Check mark in the ink color: checked, selected.
        Check "check" ["checked", "selected", "tick"] {
            ink(line(&[(2.5, 8.5), (6.0, 12.0), (13.5, 4.5)])),
        }
        /// Circle with a question mark: help.
        Help "help" ["question", "docs", "support"] {
            ink(circle(8.0, 8.0, 6.5)),
            ink(arc(8.0, 6.25, 2.25, 180.0, 405.0)),
            ink(line(&[(9.6, 7.84), (8.0, 9.0), (8.0, 9.75)])),
            dot(8.0, 11.75, 0.9),
        }
        /// Bell: notifications.
        Bell "bell" ["notifications", "alerts", "event log"] {
            ink(arc(8.0, 6.0, 3.5, 180.0, 360.0)),
            ink(line(&[(4.5, 6.0), (4.5, 11.0)])),
            ink(line(&[(11.5, 6.0), (11.5, 11.0)])),
            ink(line(&[(2.5, 11.0), (13.5, 11.0)])),
            ink(arc(8.0, 12.0, 1.5, 0.0, 180.0)),
        }
        /// Clock: time, duration, pending.
        Clock "clock" ["time", "duration", "pending", "timer"] {
            ink(circle(8.0, 8.0, 6.0)),
            ink(line(&[(8.0, 4.5), (8.0, 8.5), (11.0, 10.5)])),
        }
    }

    Data {
        /// Cylinder: database.
        Database "database" ["db", "storage", "sql", "mongodb"] {
            ink(poly(&[
                (13.5, 4.0), (13.31, 4.52), (12.76, 5.0), (11.89, 5.41), (10.75, 5.73), (9.42, 5.93),
                (8.0, 6.0), (6.58, 5.93), (5.25, 5.73), (4.11, 5.41), (3.24, 5.0), (2.69, 4.52),
                (2.5, 4.0), (2.69, 3.48), (3.24, 3.0), (4.11, 2.59), (5.25, 2.27), (6.58, 2.07),
                (8.0, 2.0), (9.42, 2.07), (10.75, 2.27), (11.89, 2.59), (12.76, 3.0), (13.31, 3.48),
            ])),
            ink(line(&[(2.5, 4.0), (2.5, 12.0)])),
            ink(line(&[(13.5, 4.0), (13.5, 12.0)])),
            ink(line(&[
                (13.5, 8.0), (13.31, 8.52), (12.76, 9.0), (11.89, 9.41), (10.75, 9.73), (9.42, 9.93),
                (8.0, 10.0), (6.58, 9.93), (5.25, 9.73), (4.11, 9.41), (3.24, 9.0), (2.69, 8.52), (2.5, 8.0),
            ])),
            ink(line(&[
                (13.5, 12.0), (13.31, 12.52), (12.76, 13.0), (11.89, 13.41), (10.75, 13.73), (9.42, 13.93),
                (8.0, 14.0), (6.58, 13.93), (5.25, 13.73), (4.11, 13.41), (3.24, 13.0), (2.69, 12.52), (2.5, 12.0),
            ])),
        }
        /// Grid: table, collection.
        Table "table" ["grid", "collection", "spreadsheet", "rows"] {
            ink(rect(1.5, 2.5, 14.5, 13.5, 1.5)),
            ink(line(&[(1.5, 5.5), (14.5, 5.5)])),
            ink(line(&[(1.5, 9.5), (14.5, 9.5)])),
            ink(line(&[(5.5, 5.5), (5.5, 13.5)])),
        }
        /// Curly braces: JSON, object.
        Json "json" ["braces", "object", "document"] {
            ink(line(&[(6.0, 2.5), (5.0, 2.5), (4.5, 3.0), (4.5, 7.0), (3.0, 8.0), (4.5, 9.0), (4.5, 13.0), (5.0, 13.5), (6.0, 13.5)])),
            ink(line(&[(10.0, 2.5), (11.0, 2.5), (11.5, 3.0), (11.5, 7.0), (13.0, 8.0), (11.5, 9.0), (11.5, 13.0), (11.0, 13.5), (10.0, 13.5)])),
        }
        /// Key: credentials, primary key.
        Key "key" ["credentials", "secret", "primary key", "api key"] {
            ink(circle(5.5, 10.5, 3.25)),
            ink(line(&[(7.8, 8.2), (14.0, 2.0)])),
            ink(line(&[(12.0, 4.0), (13.5, 5.5)])),
            ink(line(&[(10.0, 6.0), (11.25, 7.25)])),
        }
        /// Two stacked units: server, host.
        Server "server" ["host", "machine", "backend"] {
            ink(rect(2.5, 2.5, 13.5, 7.5, 1.5)),
            ink(rect(2.5, 8.5, 13.5, 13.5, 1.5)),
            dot(5.0, 5.0, 0.9),
            dot(5.0, 11.0, 0.9),
        }
        /// Cloud: remote, hosted.
        Cloud "cloud" ["remote", "hosted", "online"] {
            ink(arc(5.0, 10.0, 3.0, 90.0, 270.0)),
            ink(arc(8.5, 7.5, 4.25, 188.0, 369.0)),
            ink(arc(11.5, 10.5, 2.5, 270.0, 450.0)),
            ink(line(&[(5.0, 13.0), (11.5, 13.0)])),
        }
        /// Globe: HTTP, web, network.
        Globe "globe" ["http", "web", "internet", "network", "url"] {
            ink(circle(8.0, 8.0, 6.0)),
            ink(poly(&[
                (10.75, 8.0), (10.66, 9.55), (10.38, 11.0), (9.94, 12.24), (9.38, 13.2), (8.71, 13.8),
                (8.0, 14.0), (7.29, 13.8), (6.63, 13.2), (6.06, 12.24), (5.62, 11.0), (5.34, 9.55),
                (5.25, 8.0), (5.34, 6.45), (5.62, 5.0), (6.06, 3.76), (6.62, 2.8), (7.29, 2.2),
                (8.0, 2.0), (8.71, 2.2), (9.38, 2.8), (9.94, 3.76), (10.38, 5.0), (10.66, 6.45),
            ])),
            ink(line(&[(2.0, 8.0), (14.0, 8.0)])),
        }
        /// Plug: connection, data source.
        Plug "plug" ["connect", "connection", "data source"] {
            ink(line(&[(6.0, 1.5), (6.0, 4.5)])),
            ink(line(&[(10.0, 1.5), (10.0, 4.5)])),
            ink(poly(&[(4.0, 4.5), (12.0, 4.5), (12.0, 7.0), (10.0, 10.5), (6.0, 10.5), (4.0, 7.0)])),
            ink(line(&[(8.0, 10.5), (8.0, 14.5)])),
        }
        /// Shield: security, authentication.
        Shield "shield" ["security", "auth", "authentication", "protected"] {
            ink(poly(&[(8.0, 1.5), (13.5, 3.5), (13.5, 8.0), (12.0, 11.0), (8.0, 14.5), (4.0, 11.0), (2.5, 8.0), (2.5, 3.5)])),
        }
        /// Person: user, account.
        User "user" ["account", "person", "profile"] {
            ink(circle(8.0, 4.5, 2.75)),
            ink(arc(8.0, 14.0, 5.5, 180.0, 360.0)),
        }
        /// Lightning bolt: event, trigger, index.
        Lightning "lightning" ["event", "trigger", "fast", "index"] {
            ink(poly(&[(9.5, 1.5), (3.5, 9.5), (7.5, 9.5), (6.5, 14.5), (12.5, 6.5), (8.5, 6.5)])),
        }
    }

    General {
        /// Ribbon: bookmark.
        Bookmark "bookmark" ["save for later", "mark"] {
            ink(poly(&[(4.5, 1.5), (11.5, 1.5), (11.5, 14.5), (8.0, 11.0), (4.5, 14.5)])),
        }
        /// Speech bubble: comment.
        Comment "comment" ["message", "chat", "review comment"] {
            ink(line(&[(4.5, 11.0), (3.0, 11.0)])),
            ink(arc(3.0, 9.5, 1.5, 90.0, 180.0)),
            ink(line(&[(1.5, 9.5), (1.5, 3.5)])),
            ink(arc(3.0, 3.5, 1.5, 180.0, 270.0)),
            ink(line(&[(3.0, 2.0), (13.0, 2.0)])),
            ink(arc(13.0, 3.5, 1.5, 270.0, 360.0)),
            ink(line(&[(14.5, 3.5), (14.5, 9.5)])),
            ink(arc(13.0, 9.5, 1.5, 0.0, 90.0)),
            ink(line(&[(13.0, 11.0), (7.5, 11.0), (4.5, 14.0), (4.5, 11.0)])),
        }
        /// Sun: light theme.
        Sun "sun" ["light theme", "day", "brightness"] {
            ink(circle(8.0, 8.0, 2.75)),
            ink(line(&[(12.6, 8.0), (14.0, 8.0)])),
            ink(line(&[(2.0, 8.0), (3.4, 8.0)])),
            ink(line(&[(8.0, 12.6), (8.0, 14.0)])),
            ink(line(&[(8.0, 2.0), (8.0, 3.4)])),
            ink(line(&[(11.25, 11.25), (12.6, 12.6)])),
            ink(line(&[(4.75, 4.75), (3.4, 3.4)])),
            ink(line(&[(4.75, 11.25), (3.4, 12.6)])),
            ink(line(&[(11.25, 4.75), (12.6, 3.4)])),
        }
        /// Crescent: dark theme.
        Moon "moon" ["dark theme", "night"] {
            ink(arc(8.0, 8.0, 6.0, 10.71, 259.29)),
            ink(arc(11.5, 4.5, 5.2, 62.57, 207.43)),
        }
    }
}
