use super::{Theme, ThemeVariant, FocusMode};
use crate::config::options::Options;
use crate::tui::color::hex_to_color;
use ratatui::style::Color;

/// Runtime configuration for TUI behavior and appearance
///
/// This struct holds all user preferences that affect how the TUI behaves.
/// Currently set statically via Default, but designed to be loaded from
/// config files, environment variables, or CLI arguments in the future.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Visual theme (colors, styles)
    pub theme: Theme,

    /// How keyboard focus is acquired (click, hover, or hybrid)
    pub focus_mode: FocusMode,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            theme: Theme::new(ThemeVariant::default()),
            focus_mode: FocusMode::default(),
        }
    }
}

impl RuntimeConfig {
    /// Create a new config with explicit settings
    pub fn new(theme: Theme, focus_mode: FocusMode) -> Self {
        Self { theme, focus_mode }
    }

    /// Create config with custom theme variant and default focus mode
    pub fn with_theme(variant: ThemeVariant) -> Self {
        Self {
            theme: Theme::new(variant),
            focus_mode: FocusMode::default(),
        }
    }

    /// Create config with custom focus mode and default theme
    pub fn with_focus_mode(mode: FocusMode) -> Self {
        Self {
            theme: Theme::new(ThemeVariant::default()),
            focus_mode: mode,
        }
    }

    /// Load runtime config from the options system
    pub async fn load_from_options() -> anyhow::Result<Self> {
        let config = crate::global_config();

        // Load focus mode from options (defaults to Hover if not found)
        let focus_mode_str = config.options.get_string("tui.focus_mode").await
            .unwrap_or_else(|_| "hover".to_string());

        let focus_mode = match focus_mode_str.as_str() {
            "click" => FocusMode::Click,
            "hover" => FocusMode::Hover,
            "hover_when_unfocused" => FocusMode::HoverWhenUnfocused,
            _ => FocusMode::default(),
        };

        // Load active theme from options (defaults to mocha if not found)
        let theme_name = config.options.get_string("tui.active_theme").await
            .unwrap_or_else(|_| "mocha".to_string());

        // Load theme colors from options database
        let theme = load_theme_from_options(&config.options, &theme_name).await
            .unwrap_or_else(|e| {
                log::warn!("Failed to load theme '{}': {}. Falling back to mocha.", theme_name, e);
                Theme::mocha()
            });

        Ok(Self {
            theme,
            focus_mode,
        })
    }
}

/// Load a theme by name from the options database
///
/// Returns an error if the theme doesn't exist or if any required color is missing
async fn load_theme_from_options(options: &Options, name: &str) -> anyhow::Result<Theme> {
    let accent_primary = load_color(options, name, "accent_primary").await?;
    let accent_secondary = load_color(options, name, "accent_secondary").await?;
    let accent_tertiary = load_color(options, name, "accent_tertiary").await?;
    let accent_error = load_color(options, name, "accent_error").await?;
    let accent_warning = load_color(options, name, "accent_warning").await?;
    let accent_success = load_color(options, name, "accent_success").await?;
    let accent_info = load_color(options, name, "accent_info").await?;
    let accent_muted = load_color(options, name, "accent_muted").await?;

    let text_primary = load_color(options, name, "text_primary").await?;
    let text_secondary = load_color(options, name, "text_secondary").await?;
    let text_tertiary = load_color(options, name, "text_tertiary").await?;

    let border_primary = load_color(options, name, "border_primary").await?;
    let border_secondary = load_color(options, name, "border_secondary").await?;
    let border_tertiary = load_color(options, name, "border_tertiary").await?;

    let bg_base = load_color(options, name, "bg_base").await?;
    let bg_surface = load_color(options, name, "bg_surface").await?;
    let bg_elevated = load_color(options, name, "bg_elevated").await?;

    let palette_1 = load_color(options, name, "palette_1").await?;
    let palette_2 = load_color(options, name, "palette_2").await?;
    let palette_3 = load_color(options, name, "palette_3").await?;
    let palette_4 = load_color(options, name, "palette_4").await?;

    Ok(Theme {
        accent_primary,
        accent_secondary,
        accent_tertiary,
        accent_error,
        accent_warning,
        accent_success,
        accent_info,
        accent_muted,
        text_primary,
        text_secondary,
        text_tertiary,
        border_primary,
        border_secondary,
        border_tertiary,
        bg_base,
        bg_surface,
        bg_elevated,
        palette_1,
        palette_2,
        palette_3,
        palette_4,
    })
}

/// Load a single color from options by theme name and color name
async fn load_color(options: &Options, theme: &str, color: &str) -> anyhow::Result<Color> {
    let key = format!("theme.{}.{}", theme, color);
    let hex = options.get_string(&key).await?;
    hex_to_color(&hex)
}
