// SPDX-License-Identifier: AGPL-3.0-or-later
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
        // Resolve the log file inside a block so the guard is provably gone
        // before the awaits below. An explicit `drop` is not enough here: the
        // borrow of `biome` keeps the guard in the generator, making this
        // future !Send and unspawnable.
        let log_file = {
            let biomes = self.biomes.read().unwrap_or_else(|e| e.into_inner());
            let biome = biomes.get(&biome_name).ok_or_else(|| {
                crate::CliError::Other(format!("Biome '{biome_name}' is not running"))
            })?;

            if let Some(service) = &service_name {
                biome
                    .log_files
                    .get(service)
                    .ok_or_else(|| {
                        crate::CliError::Other(format!("Service '{service}' not found"))
                    })?
                    .clone()
            } else {
                // Show all logs (default to first service or biome log)
                biome
                    .log_files
                    .values()
                    .next()
                    .ok_or_else(|| {
                        crate::CliError::Other("No log files found for biome".to_string())
                    })?
                    .clone()
            }
        };

        // Apply filters (for future use)
        let _ = timestamps;
        let _ = level_filter;
        let _ = grep_pattern;

        if follow {
            info!("📜 Following logs: {}", log_file.display());
            self.tail_log_file(&log_file, lines).await
        } else {
            info!("📜 Showing logs: {}", log_file.display());
            self.show_log_file(&log_file, Some(lines)).await
        }
    }
}
