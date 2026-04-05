// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::ToadStoolResult;
use crate::intelligent::IntelligentAutoConfig;

/// Quick start function for zero-touch configuration
///
/// This is the simplest way to get `ToadStool` configured and running.
/// It performs all auto-configuration steps and returns a ready-to-use configuration.
///
/// # Examples
///
/// ```rust,ignore
/// // Example usage (API may change)
/// use toadstool_auto_config::quick_start;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = quick_start().await?;
///     println!("ToadStool is ready!");
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - Hardware detection fails
/// - System capabilities cannot be determined
/// - Configuration validation fails
/// - File system permissions prevent writing configuration files
pub async fn quick_start() -> ToadStoolResult<toadstool_config::ToadStoolConfig> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok(); // Ignore if already initialized

    tracing::info!("🍄 ToadStool Universal Compute Platform");
    tracing::info!("🎯 Zero-Touch Auto-Configuration Starting...");

    IntelligentAutoConfig::auto_configure().await
}
