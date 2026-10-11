//! # Archipel Kit TUI
//!
//! Archipel Kit for the terminal: the same [`Theme`] colors, the same icons and components
//! with the same names, drawn into terminal cells with [ratatui]. An app built on the kit
//! can offer a GUI and a TUI that look and work alike.
//!
//! ```
//! use archipel_kit_tui::{Look, components::{Button, Island}};
//! use archipel_kit_tui::ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
//!
//! let look = Look::dark();
//! let area = Rect::new(0, 0, 30, 4);
//! let mut buf = Buffer::empty(area);
//! let island = Island::new(&look).title("Project");
//! let inner = island.inner(area);
//! island.render(area, &mut buf);
//! Button::primary(&look, "Run").render(inner, &mut buf);
//! ```
//!
//! The look is the GUI's (theme colors, icons, Islands layout) in the flat style of modern
//! Neovim setups such as NvChad:
//!
//! - **Islands** are flat blocks of the island color one cell apart on the frame color, with
//!   a title pill, in the accent when the panel has focus. No borders.
//! - **Colors** come from the kit [`Theme`] as 24-bit color; translucent colors (hover,
//!   selection) are blended over the island.
//! - **Icons** are Nerd Font characters: kit [`Glyph`]s map to codicons, languages to their
//!   logos in the language color, symbols to codicon symbols in the symbol color. Use a Nerd
//!   Font in the terminal; JetBrainsMono Nerd Font matches the GUI's code font.
//!
//! The guide with images is `docs/tui/README.md`.

pub mod components;
pub mod glyphs;
pub mod testing;

pub use archipel_icons::{Glyph, Language, Symbol, SymbolKind};

/// The code blocks of `docs/tui/README.md`, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../../../docs/tui/README.md")]
struct TuiGuide;
pub use archipel_kit::Theme;
pub use ratatui;

use ratatui::style::{Color, Modifier, Style};

/// A kit [`Theme`] prepared for the terminal.
#[derive(Debug, Clone)]
pub struct Look {
    /// The theme the colors come from.
    pub theme: Theme,
}

impl Look {
    /// The dark theme.
    pub fn dark() -> Self {
        Self::new(Theme::dark())
    }

    /// The light theme.
    pub fn light() -> Self {
        Self::new(Theme::light())
    }

    /// `theme` in the terminal.
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// `color` as a terminal color, blended over the island when it is translucent.
    pub fn color(&self, color: archipel_kit::egui::Color32) -> Color {
        self.color_over(color, self.theme.colors.island)
    }

    /// `color` blended over `background`.
    pub fn color_over(
        &self,
        color: archipel_kit::egui::Color32,
        background: archipel_kit::egui::Color32,
    ) -> Color {
        let [r, g, b, a] = color.to_srgba_unmultiplied();
        if a == 255 {
            return Color::Rgb(r, g, b);
        }
        let t = a as f32 / 255.0;
        let mix = |c: u8, under: u8| (under as f32 + (c as f32 - under as f32) * t).round() as u8;
        Color::Rgb(
            mix(r, background.r()),
            mix(g, background.g()),
            mix(b, background.b()),
        )
    }

    /// A `0xRRGGBB` value as a terminal color.
    pub fn rgb(value: u32) -> Color {
        Color::Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }

    /// Text in `color` on the island.
    pub fn text(&self, color: archipel_kit::egui::Color32) -> Style {
        Style::default()
            .fg(self.color(color))
            .bg(self.color(self.theme.colors.island))
    }

    /// Default text on the island.
    pub fn body(&self) -> Style {
        self.text(self.theme.colors.text)
    }

    /// Bold strong text on the island.
    pub fn strong(&self) -> Style {
        self.text(self.theme.colors.text_strong)
            .add_modifier(Modifier::BOLD)
    }

    /// The Nerd Font character of `glyph`.
    pub fn glyph(&self, glyph: Glyph) -> String {
        glyphs::nerd_font(glyph).to_string()
    }

    /// The color `glyph` is drawn in: status glyphs in their status color, the rest in the
    /// icon color, as in the GUI.
    pub fn glyph_color(&self, glyph: Glyph) -> Color {
        let c = &self.theme.colors;
        self.color(match glyph {
            Glyph::Run | Glyph::Debug | Glyph::Success => c.run,
            Glyph::Stop | Glyph::Error => c.error,
            Glyph::Warning | Glyph::WeakWarning => c.warning,
            Glyph::Info => c.link,
            _ => c.icon,
        })
    }

    /// `0xRRGGBB` as a foreground on the island, mixed toward the text color until it has
    /// 3:1 contrast (dark language colors on a dark island stay visible).
    pub fn readable(&self, value: u32) -> Color {
        let island = self.theme.colors.island;
        let text = self.theme.colors.text_strong;
        let channel = |v: u32, shift: u32| ((v >> shift) & 0xFF) as f32;
        let (mut r, mut g, mut b) = (channel(value, 16), channel(value, 8), channel(value, 0));
        let luminance = |r: f32, g: f32, b: f32| {
            let lin = |c: f32| {
                let c = c / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
        };
        let back = luminance(island.r() as f32, island.g() as f32, island.b() as f32);
        for _ in 0..10 {
            let front = luminance(r, g, b);
            let (hi, lo) = if front > back {
                (front, back)
            } else {
                (back, front)
            };
            if (hi + 0.05) / (lo + 0.05) >= 3.0 {
                break;
            }
            r += (text.r() as f32 - r) * 0.2;
            g += (text.g() as f32 - g) * 0.2;
            b += (text.b() as f32 - b) * 0.2;
        }
        Color::Rgb(r.round() as u8, g.round() as u8, b.round() as u8)
    }
}
