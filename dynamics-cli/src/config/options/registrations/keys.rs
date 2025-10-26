//! Keyboard-related options registration

use crate::config::options::{OptionDefBuilder, OptionsRegistry};
use anyhow::Result;

/// Register all keyboard-related options
pub fn register(registry: &OptionsRegistry) -> Result<()> {
    // Tab debouncing option
    registry.register(
        OptionDefBuilder::new("keys", "tab.debouncing")
            .display_name("Tab Debouncing")
            .description("Minimum time between Tab key presses in milliseconds (prevents accidental rapid focus changes)")
            .uint_type(150, Some(0), Some(5000))
            .build()?
    )?;

    log::info!("Registered {} keyboard options", 1);
    Ok(())
}
