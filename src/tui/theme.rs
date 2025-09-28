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
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
    pub active: Color,
    pub inactive: Color,
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub surface: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
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
            danger: Color::Rgb(243, 139, 168),      // Red: #f38ba8
            warning: Color::Rgb(249, 226, 175),     // Yellow: #f9e2af
            success: Color::Rgb(166, 227, 161),     // Green: #a6e3a1
            active: Color::Rgb(137, 180, 250),      // Blue: #89b4fa
            inactive: Color::Rgb(69, 71, 90),       // Surface1: #45475a
            primary: Color::Rgb(203, 166, 247),     // Mauve: #cba6f7
            secondary: Color::Rgb(180, 190, 254),   // Lavender: #b4befe
            background: Color::Rgb(30, 30, 46),     // Base: #1e1e2e
            surface: Color::Rgb(49, 50, 68),        // Surface0: #313244
            text_primary: Color::Rgb(205, 214, 244), // Text: #cdd6f4
            text_secondary: Color::Rgb(186, 194, 222), // Subtext1: #bac2de
        }
    }

    fn latte() -> Self {
        Self {
            danger: Color::Rgb(210, 15, 57),        // Red: #d20f39
            warning: Color::Rgb(223, 142, 29),      // Yellow: #df8e1d
            success: Color::Rgb(64, 160, 43),       // Green: #40a02b
            active: Color::Rgb(30, 102, 245),       // Blue: #1e66f5
            inactive: Color::Rgb(188, 192, 204),    // Surface1: #bcc0cc
            primary: Color::Rgb(136, 57, 239),      // Mauve: #8839ef
            secondary: Color::Rgb(114, 135, 253),   // Lavender: #7287fd
            background: Color::Rgb(239, 241, 245),  // Base: #eff1f5
            surface: Color::Rgb(204, 208, 218),     // Surface0: #ccd0da
            text_primary: Color::Rgb(76, 79, 105),  // Text: #4c4f69
            text_secondary: Color::Rgb(92, 95, 119), // Subtext1: #5c5f77
        }
    }

    // Style helper methods for common patterns
    pub fn danger_style(&self) -> Style {
        Style::default().fg(self.danger)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn active_style(&self) -> Style {
        Style::default().bg(self.active).fg(self.background)
    }

    pub fn primary_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    pub fn header_style(&self) -> Style {
        Style::default().bg(self.surface).fg(self.text_primary)
    }

    pub fn footer_style(&self) -> Style {
        Style::default().bg(self.surface).fg(self.text_secondary)
    }

    pub fn surface_style(&self) -> Style {
        Style::default().bg(self.surface).fg(self.text_primary)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeVariant::default())
    }
}