//! Keybind options registration

use crate::config::options::{OptionDefBuilder, OptionsRegistry};
use anyhow::Result;

/// App names for keybind organization
pub const APP_GLOBAL: &str = "global";

/// Action names for global keybinds
pub const ACTION_HELP: &str = "help";
pub const ACTION_APP_LAUNCHER: &str = "app_launcher";
pub const ACTION_APP_OVERVIEW: &str = "app_overview";

/// List all apps that have registered keybinds
pub fn list_apps(registry: &OptionsRegistry) -> Vec<String> {
    let keybind_opts = registry.list_namespace("keybind");
    let mut apps = std::collections::HashSet::new();

    for opt_def in keybind_opts {
        let parts: Vec<&str> = opt_def.key.split('.').collect();
        if parts.len() >= 3 {  // Only include keys with 3+ parts (keybind.APP.action)
            apps.insert(parts[1].to_string());
        }
    }

    let mut sorted: Vec<String> = apps.into_iter().collect();
    sorted.sort();
    sorted
}

/// Get all action names for a specific app
pub fn list_actions_for_app(registry: &OptionsRegistry, app: &str) -> Vec<String> {
    let prefix = format!("keybind.{}", app);
    let opts = registry.list_namespace(&prefix);

    opts.into_iter()
        .filter_map(|opt_def| {
            let parts: Vec<&str> = opt_def.key.split('.').collect();
            if parts.len() >= 3 && parts[1] == app {
                Some(parts[2].to_string())
            } else {
                None
            }
        })
        .collect()
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

/// Get all actions across all apps (for UI compatibility)
/// Returns actions in "app.action" format
pub fn list_all_actions(registry: &OptionsRegistry) -> Vec<String> {
    let apps = list_apps(registry);
    let mut all_actions = Vec::new();

    for app in apps {
        let actions = list_actions_for_app(registry, &app);
        for action in actions {
            all_actions.push(format!("{}.{}", app, action));
        }
    }

    all_actions.sort();
    all_actions
}

/// Register all keybind-related options
pub fn register(registry: &OptionsRegistry) -> Result<()> {
    // Global keybinds
    register_app_keybinds(registry, APP_GLOBAL)?;

    log::info!("Registered keybind options for {} apps", list_apps(registry).len());
    Ok(())
}

/// Register keybinds for a specific app
fn register_app_keybinds(registry: &OptionsRegistry, app: &str) -> Result<()> {
    match app {
        APP_GLOBAL => {
            // Help menu keybind
            registry.register(
                OptionDefBuilder::new("keybind", &format!("{}.{}", APP_GLOBAL, ACTION_HELP))
                    .display_name(get_action_display_name(ACTION_HELP))
                    .description(get_action_description(ACTION_HELP))
                    .string_type("F1", Some(32))
                    .build()?
            )?;

            // App launcher keybind
            registry.register(
                OptionDefBuilder::new("keybind", &format!("{}.{}", APP_GLOBAL, ACTION_APP_LAUNCHER))
                    .display_name(get_action_display_name(ACTION_APP_LAUNCHER))
                    .description(get_action_description(ACTION_APP_LAUNCHER))
                    .string_type("Ctrl+A", Some(32))
                    .build()?
            )?;

            // App overview keybind
            registry.register(
                OptionDefBuilder::new("keybind", &format!("{}.{}", APP_GLOBAL, ACTION_APP_OVERVIEW))
                    .display_name(get_action_display_name(ACTION_APP_OVERVIEW))
                    .description(get_action_description(ACTION_APP_OVERVIEW))
                    .string_type("Ctrl+O", Some(32))
                    .build()?
            )?;
        }
        _ => {
            // Future: Add app-specific keybinds here
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_action_count() {
        // Ensure we have descriptions for all global actions
        let actions = [ACTION_HELP, ACTION_APP_LAUNCHER, ACTION_APP_OVERVIEW];
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

    #[test]
    fn test_list_apps() {
        use crate::config::options::OptionsRegistry;

        let registry = OptionsRegistry::new();
        register(&registry).unwrap();

        let apps = list_apps(&registry);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0], APP_GLOBAL);
    }

    #[test]
    fn test_list_actions_for_app() {
        use crate::config::options::OptionsRegistry;

        let registry = OptionsRegistry::new();
        register(&registry).unwrap();

        let actions = list_actions_for_app(&registry, APP_GLOBAL);
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ACTION_HELP.to_string()));
        assert!(actions.contains(&ACTION_APP_LAUNCHER.to_string()));
        assert!(actions.contains(&ACTION_APP_OVERVIEW.to_string()));
    }
}
