//! TUI-related options registration

use crate::config::options::{OptionDefBuilder, OptionsRegistry};
use anyhow::Result;

/// Register all TUI-related options
pub fn register(registry: &OptionsRegistry) -> Result<()> {
    // Focus mode option
    registry.register(
        OptionDefBuilder::new("tui", "focus_mode")
            .display_name("Focus Mode")
            .description("How interactive elements gain keyboard focus")
            .enum_type(
                vec!["click", "hover", "hover_when_unfocused"],
                "hover"
            )
            .build()?
    )?;

    // Active theme option
    registry.register(
        OptionDefBuilder::new("tui", "active_theme")
            .display_name("Active Theme")
            .description("The currently active color theme")
            .string_type("mocha", Some(32))
            .build()?
    )?;

    // Color picker mode option
    registry.register(
        OptionDefBuilder::new("tui", "colorpicker_mode")
            .display_name("Color Picker Mode")
            .description("Default color adjustment mode (HSL or RGB)")
            .enum_type(
                vec!["hsl", "rgb"],
                "hsl"
            )
            .build()?
    )?;

    log::info!("Registered {} TUI options", 3);
    Ok(())
}
