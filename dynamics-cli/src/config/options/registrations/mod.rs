//! Registration of all application options

pub mod api;
pub mod tui;
pub mod themes;
pub mod update;

use super::OptionsRegistry;
use anyhow::Result;

/// Register all options from all modules
pub fn register_all(registry: &OptionsRegistry) -> Result<()> {
    api::register(registry)?;
    tui::register(registry)?;
    themes::register(registry)?;
    update::register(registry)?;
    Ok(())
}
