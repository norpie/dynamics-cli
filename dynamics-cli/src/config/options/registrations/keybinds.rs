//! Keybind options registration

use crate::config::options::{OptionDefBuilder, OptionsRegistry};
use anyhow::Result;

/// Action names for keybinds (used as keys in keybind.* namespace)
pub const ACTION_HELP: &str = "help";
pub const ACTION_APP_LAUNCHER: &str = "app_launcher";
pub const ACTION_APP_OVERVIEW: &str = "app_overview";

/// Get all action names as a vector
pub fn list_actions() -> Vec<String> {
    vec![
        ACTION_HELP.to_string(),
        ACTION_APP_LAUNCHER.to_string(),
        ACTION_APP_OVERVIEW.to_string(),
    ]
}

/// Get human-readable description for each action
pub fn get_action_description(action: &str) -> &'static str {
    match action {
        ACTION_HELP => "Toggle help menu showing all keyboard shortcuts",
        ACTION_APP_LAUNCHER => "Open the app launcher to switch between apps",
        ACTION_APP_OVERVIEW => "Show overview of all running apps and their states",
        _ => "Keybind action",
    }
}

/// Get display name for each action
pub fn get_action_display_name(action: &str) -> &'static str {
    match action {
        ACTION_HELP => "Help Menu",
        ACTION_APP_LAUNCHER => "App Launcher",
        ACTION_APP_OVERVIEW => "App Overview",
        _ => "Unknown Action",
    }
}

/// Register all keybind-related options
pub fn register(registry: &OptionsRegistry) -> Result<()> {
    // Help menu keybind
    registry.register(
        OptionDefBuilder::new("keybind", ACTION_HELP)
            .display_name(get_action_display_name(ACTION_HELP))
            .description(get_action_description(ACTION_HELP))
            .string_type("F1", Some(32))
            .build()?
    )?;

    // App launcher keybind
    registry.register(
        OptionDefBuilder::new("keybind", ACTION_APP_LAUNCHER)
            .display_name(get_action_display_name(ACTION_APP_LAUNCHER))
            .description(get_action_description(ACTION_APP_LAUNCHER))
            .string_type("Ctrl+A", Some(32))
            .build()?
    )?;

    // App overview keybind
    registry.register(
        OptionDefBuilder::new("keybind", ACTION_APP_OVERVIEW)
            .display_name(get_action_display_name(ACTION_APP_OVERVIEW))
            .description(get_action_description(ACTION_APP_OVERVIEW))
            .string_type("Ctrl+O", Some(32))
            .build()?
    )?;

    log::info!("Registered {} keybind options", list_actions().len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_count() {
        // Ensure we have descriptions for all actions
        let actions = list_actions();
        assert_eq!(actions.len(), 3);

        for action in &actions {
            let desc = get_action_description(action);
            assert!(!desc.is_empty());
            assert_ne!(desc, "Keybind action"); // All should have specific descriptions

            let display = get_action_display_name(action);
            assert!(!display.is_empty());
            assert_ne!(display, "Unknown Action"); // All should have display names
        }
    }

    #[test]
    fn test_keybind_parsing() {
        use std::str::FromStr;
        use crate::tui::KeyBinding;

        // Test default keybinds can be parsed
        let help = KeyBinding::from_str("F1");
        assert!(help.is_ok());

        let launcher = KeyBinding::from_str("Ctrl+A");
        assert!(launcher.is_ok());

        let overview = KeyBinding::from_str("Ctrl+O");
        assert!(overview.is_ok());
    }
}
