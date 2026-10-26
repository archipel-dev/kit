//! The NvChad color roles, computed from a kit [`Theme`].
//!
//! NvChad themes name their colors by role (`black` is the editor background,
//! `darker_black` the side bars, `black2` the cursor line and prompt, `one_bg` to
//! `one_bg3` raised surfaces and so on). The roles are re-derived here from the theme's
//! editor background, editor text and semantic colors, so every bundled IntelliJ theme and
//! editor scheme gets a consistent NvChad-style palette. Nothing is copied from a base46
//! theme: the shades are mixes between the editor background and the editor text.

use archipel_kit::Theme;
use archipel_kit::egui::Color32;
use ratatui::style::Color;

/// An opaque color as three channels.
type Rgb = (f32, f32, f32);

fn channels(color: Color32, under: Rgb) -> Rgb {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    let t = f32::from(a) / 255.0;
    (
        under.0 + (f32::from(r) - under.0) * t,
        under.1 + (f32::from(g) - under.1) * t,
        under.2 + (f32::from(b) - under.2) * t,
    )
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

fn luminance(c: Rgb) -> f32 {
    let lin = |v: f32| {
        let v = v / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2)
}

fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// `color` moved toward `toward` until it has `ratio` contrast on `bg`.
fn lift(color: Rgb, bg: Rgb, toward: Rgb, ratio: f32) -> Rgb {
    let mut c = color;
    for _ in 0..12 {
        if contrast(c, bg) >= ratio {
            break;
        }
        c = mix(c, toward, 0.18);
    }
    c
}

fn color(c: Rgb) -> Color {
    Color::Rgb(
        c.0.round().clamp(0.0, 255.0) as u8,
        c.1.round().clamp(0.0, 255.0) as u8,
        c.2.round().clamp(0.0, 255.0) as u8,
    )
}

fn rgb_of(color: Color) -> Rgb {
    match color {
        Color::Rgb(r, g, b) => (f32::from(r), f32::from(g), f32::from(b)),
        _ => (0.0, 0.0, 0.0),
    }
}

/// The NvChad (base46) color roles of one theme. All colors are 24-bit.
///
/// The surfaces follow NvChad's use: [`black`](Self::black) is the editor, the darker
/// [`darker_black`](Self::darker_black) the side bar and Telescope results,
/// [`black2`](Self::black2) the cursor line, inactive tabs and prompt, and
/// [`one_bg`](Self::one_bg) to [`one_bg3`](Self::one_bg3) raised surfaces such as the
/// completion menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvChadPalette {
    /// Whether the theme is dark.
    pub dark: bool,
    /// The default text color.
    pub white: Color,
    /// The editor background.
    pub black: Color,
    /// Side bars, terminal, Telescope results.
    pub darker_black: Color,
    /// Cursor line, inactive tabs, prompt background.
    pub black2: Color,
    /// A raised surface (completion menu).
    pub one_bg: Color,
    /// A more raised surface (hover, inactive selection).
    pub one_bg2: Color,
    /// The most raised surface (selection).
    pub one_bg3: Color,
    /// Dim lines and disabled marks.
    pub grey: Color,
    /// Line numbers.
    pub grey_fg: Color,
    /// Comments and inactive text.
    pub light_grey: Color,
    /// Window separators.
    pub line: Color,
    /// The status line background.
    pub statusline_bg: Color,
    /// Light background for blocks on the status line.
    pub lightbg: Color,
    /// The selected completion item (the accent).
    pub pmenu_bg: Color,
    /// The folder icon color.
    pub folder_bg: Color,
    /// Red (errors, close).
    pub red: Color,
    /// Light pink.
    pub baby_pink: Color,
    /// Green (success, added).
    pub green: Color,
    /// Blue (links).
    pub blue: Color,
    /// Bright blue (the accent, normal mode).
    pub nord_blue: Color,
    /// Yellow (warnings, modified).
    pub yellow: Color,
    /// Purple (keywords, hints).
    pub purple: Color,
    /// Cyan (properties, info).
    pub cyan: Color,
    /// Teal (types).
    pub teal: Color,
    /// Orange (constants).
    pub orange: Color,
    on_dark: Color,
    on_light: Color,
}

impl NvChadPalette {
    /// The roles of `theme`.
    pub fn new(theme: &Theme) -> Self {
        let c = &theme.colors;
        let e = &theme.editor;
        let black = channels(e.background, (0.0, 0.0, 0.0));
        let text = channels(e.text, black);
        let dark = theme.dark;
        let ink = (0.0, 0.0, 0.0);
        let darker = mix(black, ink, if dark { 0.22 } else { 0.045 });
        let shade = |t: f32| mix(black, text, t);
        let accent = |value: Color32, fallback: Rgb| {
            let rgb = if value.a() == 0 {
                fallback
            } else {
                channels(value, black)
            };
            lift(rgb, darker, text, 3.2)
        };
        let blue = accent(c.link, (97.0, 175.0, 239.0));
        let nord_blue = accent(c.accent, blue);
        let red = accent(c.error, (224.0, 108.0, 117.0));
        let green = accent(c.success, (152.0, 195.0, 121.0));
        let yellow = accent(c.warning, (229.0, 192.0, 123.0));
        let purple = accent(e.keyword, (198.0, 120.0, 221.0));
        let cyan = accent(e.property, (86.0, 182.0, 194.0));
        let teal = accent(e.type_name, (86.0, 182.0, 194.0));
        let orange = accent(e.constant, (209.0, 154.0, 102.0));
        let folder = accent(c.folder_fill, blue);
        Self {
            dark,
            white: color(text),
            black: color(black),
            darker_black: color(darker),
            black2: color(shade(0.055)),
            one_bg: color(shade(0.085)),
            one_bg2: color(shade(0.15)),
            one_bg3: color(shade(0.21)),
            grey: color(shade(0.30)),
            grey_fg: color(lift(shade(0.42), black, text, 2.2)),
            light_grey: color(lift(shade(0.55), black, text, 3.0)),
            line: color(shade(0.11)),
            statusline_bg: color(shade(0.07)),
            lightbg: color(shade(0.13)),
            pmenu_bg: color(nord_blue),
            folder_bg: color(folder),
            red: color(red),
            baby_pink: color(mix(red, (255.0, 255.0, 255.0), 0.45)),
            green: color(green),
            blue: color(blue),
            nord_blue: color(nord_blue),
            yellow: color(yellow),
            purple: color(purple),
            cyan: color(cyan),
            teal: color(teal),
            orange: color(orange),
            on_dark: color((22.0, 24.0, 28.0)),
            on_light: color((250.0, 250.0, 252.0)),
        }
    }

    /// The foreground to put on a block of `background`: the editor background as NvChad
    /// does when it reads (3:1), else whichever dark or light ink has more contrast.
    pub fn on(&self, background: Color) -> Color {
        let bg = rgb_of(background);
        if contrast(rgb_of(self.black), bg) >= 3.0 {
            return self.black;
        }
        if contrast(rgb_of(self.on_dark), bg) >= contrast(rgb_of(self.on_light), bg) {
            self.on_dark
        } else {
            self.on_light
        }
    }
}

/// A status line mode and its block color, as NvChad's mode indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mode {
    /// Navigating: the project tree, lists, the editor in read-only mode.
    Normal,
    /// Typing into the editor or a field.
    Insert,
    /// A selection is active.
    Visual,
    /// A command line or the search popup is open.
    Command,
    /// A terminal or run window has focus.
    Terminal,
}

impl Mode {
    /// The block label.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual => "VISUAL",
            Mode::Command => "COMMAND",
            Mode::Terminal => "TERMINAL",
        }
    }

    /// The block color in `palette`.
    pub fn color(self, palette: &NvChadPalette) -> Color {
        match self {
            Mode::Normal => palette.nord_blue,
            Mode::Insert => palette.green,
            Mode::Visual => palette.purple,
            Mode::Command => palette.orange,
            Mode::Terminal => palette.teal,
        }
    }
}
