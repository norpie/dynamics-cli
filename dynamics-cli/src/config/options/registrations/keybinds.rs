//! Keybind options registration

use crate::config::options::{OptionDefBuilder, OptionsRegistry};
use crate::tui::KeyBinding;
use anyhow::Result;
use crossterm::event::KeyCode;

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
    let keybind_opts = registry.list_namespace("keybind");

    keybind_opts.into_iter()
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

/// Get human-readable description for each action by reading from registry
pub fn get_action_description(registry: &OptionsRegistry, app: &str, action: &str) -> String {
    let key = format!("keybind.{}.{}", app, action);
    registry.get(&key)
        .map(|def| def.description.clone())
        .unwrap_or_else(|| "Keybind action".to_string())
}

/// Get display name for each action by reading from registry
pub fn get_action_display_name(registry: &OptionsRegistry, app: &str, action: &str) -> String {
    let key = format!("keybind.{}.{}", app, action);
    registry.get(&key)
        .map(|def| def.display_name.clone())
        .unwrap_or_else(|| "Unknown Action".to_string())
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
    registry.register(
        OptionDefBuilder::new("keybind", "global.help")
            .display_name("Help Menu")
            .description("Toggle help menu showing all keyboard shortcuts")
            .keybind_type(KeyCode::F(1))
            .build()?
    )?;

    registry.register(
        OptionDefBuilder::new("keybind", "global.app_launcher")
            .display_name("App Launcher")
            .description("Open the app launcher to switch between apps")
            .keybind_type(KeyBinding::ctrl(KeyCode::Char('a')))
            .build()?
    )?;

    registry.register(
        OptionDefBuilder::new("keybind", "global.app_overview")
            .display_name("App Overview")
            .description("Show overview of all running apps and their states")
            .keybind_type(KeyBinding::ctrl(KeyCode::Char('o')))
            .build()?
    )?;

    // Migration app keybinds
    registry.register(
        OptionDefBuilder::new("keybind", "migration.create")
            .display_name("Create Migration")
            .description("Create a new migration environment")
            .keybind_type(KeyCode::Char('n'))
            .build()?
    )?;

    registry.register(
        OptionDefBuilder::new("keybind", "migration.delete")
            .display_name("Delete Migration")
            .description("Delete the selected migration environment")
            .keybind_type(KeyCode::Char('d'))
            .build()?
    )?;

    registry.register(
        OptionDefBuilder::new("keybind", "migration.rename")
            .display_name("Rename Migration")
            .description("Rename the selected migration environment")
            .keybind_type(KeyCode::Char('r'))
            .build()?
    )?;

    log::info!("Registered keybind options for {} apps", list_apps(registry).len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_apps() {
        let registry = crate::config::options::OptionsRegistry::new();
        register(&registry).unwrap();

        let apps = list_apps(&registry);
        assert_eq!(apps.len(), 2);
        assert!(apps.contains(&"global".to_string()));
        assert!(apps.contains(&"migration".to_string()));
    }

    #[test]
    fn test_list_actions_for_app() {
        let registry = crate::config::options::OptionsRegistry::new();
        register(&registry).unwrap();

        let actions = list_actions_for_app(&registry, "global");
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&"help".to_string()));
        assert!(actions.contains(&"app_launcher".to_string()));
        assert!(actions.contains(&"app_overview".to_string()));

        let migration_actions = list_actions_for_app(&registry, "migration");
        assert_eq!(migration_actions.len(), 3);
        assert!(migration_actions.contains(&"create".to_string()));
        assert!(migration_actions.contains(&"delete".to_string()));
        assert!(migration_actions.contains(&"rename".to_string()));
    }
}
