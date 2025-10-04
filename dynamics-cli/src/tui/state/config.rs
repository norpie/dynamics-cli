use super::{Theme, ThemeVariant, FocusMode};

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

        Ok(Self {
            theme: Theme::new(ThemeVariant::default()), // Theme will be loaded from options later
            focus_mode,
        })
    }
}
