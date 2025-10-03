/*
Catppuccin Color Palette

Latte
Rosewater,dc8a78
Flamingo,dd7878
Pink,ea76cb
Mauve,8839ef
Red,d20f39
Maroon,e64553
Peach,fe640b
Yellow,df8e1d
Green,40a02b
Teal,179299
Sky,04a5e5
Sapphire,209fb5
Blue,1e66f5
Lavender,7287fd
Text,4c4f69
Subtext 1,5c5f77
Subtext 0,6c6f85
Overlay 2,7c7f93
Overlay 1,8c8fa1
Overlay 0,9ca0b0
Surface 2,acb0be
Surface 1,bcc0cc
Surface 0,ccd0da
Base,eff1f5
Mantle,e6e9ef
Crust,dce0e8

Mocha
Rosewater,f5e0dc
Flamingo,f2cdcd
Pink,f5c2e7
Mauve,cba6f7
Red,f38ba8
Maroon,eba0ac
Peach,fab387
Yellow,f9e2af
Green,a6e3a1
Teal,94e2d5
Sky,89dceb
Sapphire,74c7ec
Blue,89b4fa
Lavender,b4befe
Text,cdd6f4
Subtext 1,bac2de
Subtext 0,a6adc8
Overlay 2,9399b2
Overlay 1,7f849c
Overlay 0,6c7086
Surface 2,585b70
Surface 1,45475a
Surface 0,313244
Base,1e1e2e
Mantle,181825
Crust,11111b
*/

use ratatui::style::{Color, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariant {
    Mocha,  // Dark theme (default)
    Latte,  // Light theme
}

impl Default for ThemeVariant {
    fn default() -> Self {
        Self::Mocha
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    // Catppuccin color palette
    pub rosewater: Color,
    pub flamingo: Color,
    pub pink: Color,
    pub mauve: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub blue: Color,
    pub lavender: Color,
    pub text: Color,
    pub subtext1: Color,
    pub subtext0: Color,
    pub overlay2: Color,
    pub overlay1: Color,
    pub overlay0: Color,
    pub surface2: Color,
    pub surface1: Color,
    pub surface0: Color,
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
}

impl Theme {
    pub fn new(variant: ThemeVariant) -> Self {
        match variant {
            ThemeVariant::Mocha => Self::mocha(),
            ThemeVariant::Latte => Self::latte(),
        }
    }

    fn mocha() -> Self {
        Self {
            // Catppuccin Mocha colors
            rosewater: Color::Rgb(0xf5, 0xe0, 0xdc),
            flamingo: Color::Rgb(0xf2, 0xcd, 0xcd),
            pink: Color::Rgb(0xf5, 0xc2, 0xe7),
            mauve: Color::Rgb(0xcb, 0xa6, 0xf7),
            red: Color::Rgb(0xf3, 0x8b, 0xa8),
            maroon: Color::Rgb(0xeb, 0xa0, 0xac),
            peach: Color::Rgb(0xfa, 0xb3, 0x87),
            yellow: Color::Rgb(0xf9, 0xe2, 0xaf),
            green: Color::Rgb(0xa6, 0xe3, 0xa1),
            teal: Color::Rgb(0x94, 0xe2, 0xd5),
            sky: Color::Rgb(0x89, 0xdc, 0xeb),
            sapphire: Color::Rgb(0x74, 0xc7, 0xec),
            blue: Color::Rgb(0x89, 0xb4, 0xfa),
            lavender: Color::Rgb(0xb4, 0xbe, 0xfe),
            text: Color::Rgb(0xcd, 0xd6, 0xf4),
            subtext1: Color::Rgb(0xba, 0xc2, 0xde),
            subtext0: Color::Rgb(0xa6, 0xad, 0xc8),
            overlay2: Color::Rgb(0x93, 0x99, 0xb2),
            overlay1: Color::Rgb(0x7f, 0x84, 0x9c),
            overlay0: Color::Rgb(0x6c, 0x70, 0x86),
            surface2: Color::Rgb(0x58, 0x5b, 0x70),
            surface1: Color::Rgb(0x45, 0x47, 0x5a),
            surface0: Color::Rgb(0x31, 0x32, 0x44),
            base: Color::Rgb(0x1e, 0x1e, 0x2e),
            mantle: Color::Rgb(0x18, 0x18, 0x25),
            crust: Color::Rgb(0x11, 0x11, 0x1b),
        }
    }

    fn latte() -> Self {
        Self {
            // Catppuccin Latte colors
            rosewater: Color::Rgb(0xdc, 0x8a, 0x78),
            flamingo: Color::Rgb(0xdd, 0x78, 0x78),
            pink: Color::Rgb(0xea, 0x76, 0xcb),
            mauve: Color::Rgb(0x88, 0x39, 0xef),
            red: Color::Rgb(0xd2, 0x0f, 0x39),
            maroon: Color::Rgb(0xe6, 0x45, 0x53),
            peach: Color::Rgb(0xfe, 0x64, 0x0b),
            yellow: Color::Rgb(0xdf, 0x8e, 0x1d),
            green: Color::Rgb(0x40, 0xa0, 0x2b),
            teal: Color::Rgb(0x17, 0x92, 0x99),
            sky: Color::Rgb(0x04, 0xa5, 0xe5),
            sapphire: Color::Rgb(0x20, 0x9f, 0xb5),
            blue: Color::Rgb(0x1e, 0x66, 0xf5),
            lavender: Color::Rgb(0x72, 0x87, 0xfd),
            text: Color::Rgb(0x4c, 0x4f, 0x69),
            subtext1: Color::Rgb(0x5c, 0x5f, 0x77),
            subtext0: Color::Rgb(0x6c, 0x6f, 0x85),
            overlay2: Color::Rgb(0x7c, 0x7f, 0x93),
            overlay1: Color::Rgb(0x8c, 0x8f, 0xa1),
            overlay0: Color::Rgb(0x9c, 0xa0, 0xb0),
            surface2: Color::Rgb(0xac, 0xb0, 0xbe),
            surface1: Color::Rgb(0xbc, 0xc0, 0xcc),
            surface0: Color::Rgb(0xcc, 0xd0, 0xda),
            base: Color::Rgb(0xef, 0xf1, 0xf5),
            mantle: Color::Rgb(0xe6, 0xe9, 0xef),
            crust: Color::Rgb(0xdc, 0xe0, 0xe8),
        }
    }

    // Helper methods following Catppuccin Terminal style guide
    // Use these sparingly - prefer direct color access for clarity

    // Status indicators (errors, warnings, success)
    pub fn error_style(&self) -> Style {
        Style::default().fg(self.red)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.yellow)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.green)
    }

    pub fn info_style(&self) -> Style {
        Style::default().fg(self.teal)
    }

    // Links (URLs, clickable elements)
    pub fn link_style(&self) -> Style {
        Style::default().fg(self.blue)
    }

    // Terminal cursor
    pub fn cursor_style(&self) -> Style {
        Style::default().bg(self.rosewater).fg(self.base)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeVariant::default())
    }
}