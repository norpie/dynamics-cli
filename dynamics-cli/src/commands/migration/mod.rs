pub mod services;
/// Modern migration module using clean architecture
pub mod ui;
pub mod export;

use anyhow::Result;

/// Entry point for interactive migration interface
pub async fn start() -> Result<()> {
    // For now, use the new UI system
    ui::start_new_ui().await
}
