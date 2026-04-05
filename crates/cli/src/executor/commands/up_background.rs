// SPDX-License-Identifier: AGPL-3.0-or-later
//! Background (`up`) command — detached biome startup.

use super::super::*;

impl BiomeExecutor {
    /// Execute 'up' command - start biome in background (detached)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Manifest loading or validation fails
    /// - Biome is already running with the same name
    /// - Biome startup fails
    #[must_use = "Result of up_biome should be checked"]
    pub async fn up_biome(
        &self,
        _ctx: &CliContext,
        opts: super::super::UpBiomeOptions,
    ) -> Result<()> {
        info!("🚀 Starting biome in background mode (detached)");
        let _restart = opts.restart; // For future use
        let _health_interval = opts.health_interval; // For future use

        // Load and validate manifest
        let manifest = load_biome_manifest(&opts.manifest_path).await?;
        let warnings = validate_manifest(&manifest)?;

        for warning in warnings {
            warn!("⚠️  {}", warning);
        }

        // Determine biome name
        let biome_name = opts.name.unwrap_or_else(|| manifest.metadata.name.clone());

        info!("📋 Biome: {} v{}", biome_name, manifest.metadata.version);

        // Check if biome is already running
        {
            let biomes = self.biomes.read().await;
            if biomes.contains_key(&biome_name) {
                return Err(crate::CliError::Other(format!(
                    "Biome '{biome_name}' is already running"
                )));
            }
        }

        // Start biome in detached mode
        let biome_info = self
            .start_biome_internal(
                &biome_name,
                manifest,
                opts.env,
                opts.detach,
                false,      // debug
                "standard", // security level
            )
            .await?;

        info!(
            "✅ Biome '{}' started in background (ID: {})",
            biome_info.name, biome_info.id
        );

        // Print status hint
        println!("💡 Use 'toadstool ps' to view status");
        println!("💡 Use 'toadstool logs {biome_name}' to view logs");
        println!("💡 Use 'toadstool down {biome_name}' to stop");

        Ok(())
    }
}
