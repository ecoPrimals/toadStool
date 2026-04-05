// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stop (`down`) and list (`ps`) commands.

use super::super::resources::ResourceManager;
use super::super::*;

impl BiomeExecutor {
    /// Execute 'down' command - stop running biome
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Biome is not running
    /// - Stopping the biome fails
    /// - Data purging fails (if requested)
    #[must_use = "Result of down_biome should be checked"]
    pub async fn down_biome(
        &self,
        biome_name: impl AsRef<str>,
        force: bool,
        timeout_secs: u64,
        purge: bool,
    ) -> Result<()> {
        let biome_name = biome_name.as_ref();
        info!("🛑 Stopping biome: {}", biome_name);
        let _timeout_secs = timeout_secs; // For future use

        // Check if biome exists
        if !ResourceManager::new(self).biome_exists(biome_name).await {
            return Err(crate::CliError::Other(format!(
                "Biome '{biome_name}' is not running"
            )));
        }

        // Stop biome (with timeout, force)
        self.stop_biome_internal(biome_name, force, timeout_secs)
            .await?;

        info!("✅ Biome '{}' stopped successfully", biome_name);

        // Purge data if requested
        if purge {
            info!("🗑️  Purging biome data...");
            self.purge_biome_data(biome_name).await?;
            info!("✅ Data purged successfully");
        }

        Ok(())
    }

    /// Execute 'ps' command - list all biomes
    ///
    /// # Errors
    ///
    /// Returns an error if printing the biomes table fails
    #[must_use = "Result of list_biomes should be checked"]
    pub async fn list_biomes(
        &self,
        _all: bool,
        _format: &str,
        show_resources: bool,
        _status_filter: Option<&str>,
    ) -> Result<()> {
        let biomes = self.biomes.read().await;
        let biome_refs: Vec<&RunningBiome> = biomes.values().collect();
        self.print_biomes_table(&biome_refs, show_resources).await?;

        // Print summary
        if !biome_refs.is_empty() {
            println!();
            println!("💡 Use 'toadstool logs <name>' to view logs");
            println!("💡 Use 'toadstool down <name>' to stop a biome");
        }
        drop(biomes);

        Ok(())
    }
}
