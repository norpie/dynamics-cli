//! Registration of all application options

pub mod api;
pub mod tui;

use super::OptionsRegistry;
use anyhow::Result;

/// Register all options from all modules
pub fn register_all(registry: &OptionsRegistry) -> Result<()> {
    api::register(registry)?;
    tui::register(registry)?;
    Ok(())
}
