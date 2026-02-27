//! Public CLI Commands for Biome Execution
//!
//! This module contains all user-facing commands:
//! - `new()` - Constructor
//! - `run_biome()` - Start biome in foreground  
//! - `up_biome()` - Start biome in background (detached)
//! - `down_biome()` - Stop running biome
//! - `list_biomes()` - List all biomes
//! - `show_logs()` - View biome/service logs
//!
//! **Deep Debt Principles**:
//! - ✅ Real implementations (no mocks)
//! - ✅ Modern async/await
//! - ✅ Capability-based discovery (no hardcoded registry)

use super::*;

/// Public CLI command implementations
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
    pub async fn run_biome(&self, _ctx: &CliContext, opts: super::RunBiomeOptions) -> Result<()> {
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
                bail!("Biome '{biome_name}' is already running");
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

    /// Execute 'up' command - start biome in background (detached)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Manifest loading or validation fails
    /// - Biome is already running with the same name
    /// - Biome startup fails
    #[must_use = "Result of up_biome should be checked"]
    pub async fn up_biome(&self, _ctx: &CliContext, opts: super::UpBiomeOptions) -> Result<()> {
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
                bail!("Biome '{biome_name}' is already running");
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
        println!("💡 Use 'toadstool logs {}' to view logs", biome_name);
        println!("💡 Use 'toadstool down {}' to stop", biome_name);

        Ok(())
    }

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
        biome_name: String,
        force: bool,
        timeout_secs: u64,
        purge: bool,
    ) -> Result<()> {
        info!("🛑 Stopping biome: {}", biome_name);
        let _timeout_secs = timeout_secs; // For future use

        // Check if biome exists
        {
            let biomes = self.biomes.read().await;
            if !biomes.contains_key(&biome_name) {
                bail!("Biome '{}' is not running", biome_name);
            }
        }

        // Stop biome (with timeout, force)
        self.stop_biome_internal(&biome_name, force, timeout_secs)
            .await?;

        info!("✅ Biome '{}' stopped successfully", biome_name);

        // Purge data if requested
        if purge {
            info!("🗑️  Purging biome data...");
            self.purge_biome_data(&biome_name).await?;
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
        _format: String,
        show_resources: bool,
        _status_filter: Option<String>,
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

        Ok(())
    }

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
        target: String,
        follow: bool,
        lines: usize,
        timestamps: bool,
        level_filter: Option<String>,
        grep_pattern: Option<String>,
    ) -> Result<()> {
        // Parse target (biome or biome.service)
        let (biome_name, service_name) = if let Some((biome, service)) = target.split_once('.') {
            (biome.to_string(), Some(service.to_string()))
        } else {
            (target.clone(), None)
        };
        // Get biome
        let biomes = self.biomes.read().await;
        let biome = biomes
            .get(&biome_name)
            .ok_or_else(|| anyhow::anyhow!("Biome '{}' is not running", biome_name))?;

        // Determine log file (clone to release borrow)
        let log_file = if let Some(service) = &service_name {
            biome
                .log_files
                .get(service)
                .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service))?
                .clone()
        } else {
            // Show all logs (default to first service or biome log)
            biome
                .log_files
                .values()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No log files found for biome"))?
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
