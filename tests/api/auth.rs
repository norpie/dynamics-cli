use anyhow::Result;
use dynamics_cli::api::ClientManager;

#[tokio::test]
#[ignore] // Requires real credentials in .env
async fn test_auth_with_real_credentials() -> Result<()> {
    let mut manager = ClientManager::from_env()?;

    // Verify we can select the environment and credentials (user actions)
    let _env = manager.try_select_env(".env")?;
    let _creds = manager.auth_manager().try_select_credentials(".env")?;

    // Now authenticate with selected environment
    manager.authenticate().await?;
    Ok(())
}