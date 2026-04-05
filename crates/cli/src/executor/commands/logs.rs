// SPDX-License-Identifier: AGPL-3.0-only
//! Log viewing (`logs`) command.

use super::super::*;

impl BiomeExecutor {
    /// Execute 'logs' command - view biome/service logs
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Biome is not running
    /// - Log file doesn't exist
    /// - Reading/tailing the log file fails
    #[must_use = "Result of show_logs should be checked"]
    pub async fn show_logs(
        &self,
        target: impl AsRef<str>,
        follow: bool,
        lines: usize,
        timestamps: bool,
        level_filter: Option<&str>,
        grep_pattern: Option<&str>,
    ) -> Result<()> {
        // Parse target (biome or biome.service)
        let target = target.as_ref();
        let (biome_name, service_name) = if let Some((biome, service)) = target.split_once('.') {
            (biome.to_owned(), Some(service.to_owned()))
        } else {
            (target.to_owned(), None)
        };
        // Get biome
        let biomes = self.biomes.read().await;
        let biome = biomes.get(&biome_name).ok_or_else(|| {
            crate::CliError::Other(format!("Biome '{biome_name}' is not running"))
        })?;

        // Determine log file (clone to release borrow)
        let log_file = if let Some(service) = &service_name {
            biome
                .log_files
                .get(service)
                .ok_or_else(|| crate::CliError::Other(format!("Service '{service}' not found")))?
                .clone()
        } else {
            // Show all logs (default to first service or biome log)
            biome
                .log_files
                .values()
                .next()
                .ok_or_else(|| crate::CliError::Other("No log files found for biome".to_string()))?
                .clone()
        };

        // Drop the lock before async operations
        drop(biomes);

        // Apply filters (for future use)
        let _timestamps = timestamps;
        let _level_filter = level_filter;
        let _grep_pattern = grep_pattern;

        if follow {
            info!("📜 Following logs: {}", log_file.display());
            self.tail_log_file(&log_file, lines).await
        } else {
            info!("📜 Showing logs: {}", log_file.display());
            self.show_log_file(&log_file, Some(lines)).await
        }
    }
}
