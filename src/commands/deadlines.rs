use anyhow::Result;

mod auth_selector;
mod file_browser;

use auth_selector::run_auth_selector;
use file_browser::run_file_browser;

/// Entry point for deadlines TUI interface
pub async fn start() -> Result<()> {
    // Phase 1: Select authentication environment
    if let Some(selected_env) = run_auth_selector().await? {
        // Phase 2: Select file
        if let Some(result) = run_file_browser(selected_env).await? {
            println!("{}", result);
        }
    }

    Ok(())
}