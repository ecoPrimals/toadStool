// SPDX-License-Identifier: AGPL-3.0-or-later
//! Constructor and foreground (`run`) command.

use super::super::*;

impl BiomeExecutor {
    /// Create new biome executor
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The distributed coordinator fails to initialize
    /// - Configuration loading fails
    #[must_use = "BiomeExecutor creation should be checked"]
    pub async fn new() -> Result<Self> {
        info!("🍄 Initializing Universal Compute Biome Executor");

        // Load configuration
        let config = ToadStoolConfig::default();

        // Initialize distributed coordinator
        let distributed_config = DistributedConfig::default();
        let distributed = Arc::new(
            DistributedCoordinator::new(distributed_config)
                .await
                .context("Failed to initialize distributed coordinator")?,
        );

        // ✅ Discovery via mDNS, environment variables, or configuration
        // ✅ No hardcoded registry client - pure capability-based discovery
        info!("📢 Service discovery via mDNS/environment");

        Ok(Self {
            distributed,
            biomes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            _config: config,
        })
    }

    /// Execute 'run' command - start biome in foreground
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Manifest loading or validation fails
    /// - Biome is already running with the same name
    /// - Biome startup fails
    /// - User interruption handling fails
    #[must_use = "Result of run_biome should be checked"]
    pub async fn run_biome(
        &self,
        _ctx: &CliContext,
        opts: super::super::RunBiomeOptions,
    ) -> Result<()> {
        info!("🚀 Starting biome in foreground mode");

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

        // ✅ OPTIMIZED: Apply resource overrides in-place (avoid clone)
        let mut effective_manifest = manifest;
        if let Some(cpu) = opts.cpu_limit {
            effective_manifest.resources.cpu_limit = Some(cpu);
        }
        if let Some(memory) = opts.memory_limit.clone() {
            effective_manifest.resources.memory_limit = Some(memory);
        }

        // Start biome in foreground mode
        let biome_info = self
            .start_biome_internal(
                &biome_name,
                effective_manifest,
                opts.env,
                false, // not detached (foreground)
                opts.debug,
                &opts.security,
            )
            .await?;

        info!(
            "✅ Biome '{}' started successfully (ID: {})",
            biome_info.name, biome_info.id
        );

        // Wait for interrupt signal (Ctrl+C / SIGTERM)
        info!("📡 Press Ctrl+C to stop...");
        self.wait_for_interruption().await?;

        // Shutdown biome
        info!("🛑 Shutting down biome '{}'...", biome_name);
        self.stop_biome_internal(&biome_name, false, 30).await?;

        info!("✅ Biome stopped successfully");
        Ok(())
    }
}
