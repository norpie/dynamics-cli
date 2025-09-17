use anyhow::Result;
use dialoguer::{Input, Password, Select};
use crate::auth::credentials::Credentials;

pub fn prompt_environment_name(default_name: Option<String>) -> Result<String> {
    if let Some(name) = default_name {
        Ok(name)
    } else {
        let name = Input::<String>::new()
            .with_prompt("Environment name (e.g., 'production', 'test')")
            .interact()?;
        Ok(name)
    }
}

/// Interactive confirmation prompt using arrow-key navigable selection
///
/// # Arguments
/// * `prompt` - The question to ask the user
/// * `default_yes` - Whether "Yes" should be the default selection (index 0)
///
/// # Returns
/// * `Ok(true)` if user selects "Yes"
/// * `Ok(false)` if user selects "No"
pub fn prompt_confirmation(prompt: &str, default_yes: bool) -> Result<bool> {
    let items = vec!["Yes", "No"];
    let default_index = if default_yes { 0 } else { 1 };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(default_index)
        .interact()?;

    Ok(selection == 0)
}

pub fn prompt_overwrite_confirmation(env_name: &str) -> Result<bool> {
    prompt_confirmation(
        &format!("Environment '{}' already exists. Overwrite?", env_name),
        false // Default to "No" for safety
    )
}

pub fn prompt_save_anyway_confirmation() -> Result<bool> {
    prompt_confirmation(
        "Save configuration anyway?",
        false // Default to "No" for safety
    )
}

pub fn prompt_remove_confirmation(env_name: &str) -> Result<bool> {
    prompt_confirmation(
        &format!("Remove environment '{}'?", env_name),
        false // Default to "No" for safety
    )
}

pub fn prompt_credentials(
    host: Option<String>,
    username: Option<String>,
    password: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Result<Credentials> {
    let host_val = if let Some(h) = host {
        h
    } else {
        Input::<String>::new()
            .with_prompt("Dynamics 365 Host URL (e.g., https://yourorg.crm.dynamics.com)")
            .interact()?
    };

    let username_val = if let Some(u) = username {
        u
    } else {
        Input::<String>::new()
            .with_prompt("Username")
            .interact()?
    };

    let password_val = if let Some(p) = password {
        p
    } else {
        Password::new()
            .with_prompt("Password")
            .interact()?
    };

    let client_id_val = if let Some(c) = client_id {
        c
    } else {
        Input::<String>::new()
            .with_prompt("Azure AD Application Client ID")
            .interact()?
    };

    let client_secret_val = if let Some(s) = client_secret {
        s
    } else {
        Password::new()
            .with_prompt("Azure AD Application Client Secret")
            .interact()?
    };

    Ok(Credentials {
        host: host_val,
        username: username_val,
        password: password_val,
        client_id: client_id_val,
        client_secret: client_secret_val,
    })
}

pub fn prompt_environment_selection(env_names: &[String], current_env: Option<&String>) -> Result<String> {
    let mut items = Vec::new();
    for env in env_names {
        if current_env == Some(env) {
            items.push(format!("{} (current)", env));
        } else {
            items.push(env.clone());
        }
    }

    let selection = Select::new()
        .with_prompt("Select environment")
        .items(&items)
        .interact()?;

    Ok(env_names[selection].clone())
}

/// Simple text input prompt with optional default value
///
/// # Arguments
/// * `prompt` - The prompt message to display
/// * `default` - Optional default value
///
/// # Returns
/// * `Ok(String)` - User input or default value
pub fn text_input(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut input_prompt = Input::<String>::new()
        .with_prompt(prompt);

    if let Some(default_val) = default {
        input_prompt = input_prompt.default(default_val.to_string());
    }

    Ok(input_prompt.interact()?)
}

/// Simple confirmation prompt using the existing prompt_confirmation function
///
/// # Arguments
/// * `message` - The question to ask the user
/// * `default` - Whether "Yes" should be the default selection
///
/// # Returns
/// * `Ok(true)` if user selects "Yes"
/// * `Ok(false)` if user selects "No"
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    prompt_confirmation(message, default)
}