// SPDX-License-Identifier: AGPL-3.0-only
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

use super::resources::ResourceManager;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CliContext;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_valid_manifest_file(name: &str) -> (PathBuf, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir");
        let manifest_path = temp_dir.path().join("biome.toml");

        let now = std::time::SystemTime::now();
        let created_secs = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        let content = format!(
            r#"
[metadata]
name = "{}"
version = "1.0.0"
created = {}
updated = {}
tags = []

[primals.test-primal]
version = "latest"
enabled = true
config = {{}}
dependencies = []

[primals.test-primal.source]
type = "Container"
registry = "registry.example.com"
image = "test-image"
tag = "latest"

[services]

[resources]
cpu_limit = 1.0

[security]
isolation_level = "standard"
trust_level = "medium"
beardog_required = false
crypto_policies = []
allowed_networks = []
forbidden_syscalls = []

[networking]
mode = "bridge"
dns_servers = []
port_mappings = []
network_policies = []

[storage]
datasets = []
volumes = []
"#,
            name, created_secs, created_secs
        );

        fs::write(&manifest_path, content)
            .await
            .expect("write manifest");
        (manifest_path, temp_dir)
    }

    #[tokio::test]
    async fn test_up_biome_success() {
        let (manifest_path, _temp) = create_valid_manifest_file("up-test-biome").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path: manifest_path.clone(),
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        let result = executor.up_biome(&ctx, opts).await;
        assert!(result.is_ok(), "up_biome should succeed: {:?}", result);

        let _ = executor.down_biome("up-test-biome", true, 5, false).await;
        let _ = executor.purge_biome_data("up-test-biome").await;
    }

    #[tokio::test]
    async fn test_up_biome_with_name_override() {
        let (manifest_path, _temp) = create_valid_manifest_file("manifest-name").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path,
            detach: true,
            name: Some("custom-name-override".to_string()),
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        let result = executor.up_biome(&ctx, opts).await;
        assert!(result.is_ok());
        let info = executor.list_biomes(false, "table", false, None).await;
        assert!(info.is_ok());

        let _ = executor
            .down_biome("custom-name-override", true, 5, false)
            .await;
        let _ = executor.purge_biome_data("custom-name-override").await;
    }

    #[tokio::test]
    async fn test_up_biome_already_running_returns_err() {
        let (manifest_path, _temp) = create_valid_manifest_file("already-running").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path: manifest_path.clone(),
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        executor.up_biome(&ctx, opts.clone()).await.unwrap();

        let result = executor.up_biome(&ctx, opts).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already running"));

        let _ = executor.down_biome("already-running", true, 5, false).await;
        let _ = executor.purge_biome_data("already-running").await;
    }

    #[tokio::test]
    async fn test_down_biome_with_purge() {
        let (manifest_path, _temp) = create_valid_manifest_file("purge-test").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path,
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        executor.up_biome(&ctx, opts).await.unwrap();

        let result = executor.down_biome("purge-test", false, 10, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_biomes_with_running_biome() {
        let (manifest_path, _temp) = create_valid_manifest_file("list-test").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path,
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        executor.up_biome(&ctx, opts).await.unwrap();

        let result = executor.list_biomes(false, "table", true, None).await;
        assert!(result.is_ok());

        let _ = executor.down_biome("list-test", true, 5, false).await;
        let _ = executor.purge_biome_data("list-test").await;
    }

    #[tokio::test]
    async fn test_show_logs_service_not_found() {
        let (manifest_path, _temp) = create_valid_manifest_file("logs-service-test").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path,
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        executor.up_biome(&ctx, opts).await.unwrap();

        let result = executor
            .show_logs(
                "logs-service-test.nonexistent-service",
                false,
                10,
                false,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        let _ = executor
            .down_biome("logs-service-test", true, 5, false)
            .await;
        let _ = executor.purge_biome_data("logs-service-test").await;
    }

    #[tokio::test]
    async fn test_show_logs_biome_service() {
        let (manifest_path, _temp) = create_valid_manifest_file("logs-biome-svc").await;
        let executor = BiomeExecutor::new().await.expect("executor");
        let ctx = CliContext {
            config_path: None,
            working_dir: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let opts = UpBiomeOptions {
            manifest_path,
            detach: true,
            name: None,
            env: vec![],
            restart: false,
            health_interval: 30,
        };

        executor.up_biome(&ctx, opts).await.unwrap();

        // Create log file (lifecycle stores path but doesn't create file)
        let env = toadstool_common::platform_paths::PathEnv::from_env();
        let paths = toadstool_common::platform_paths::PlatformPaths::new(&env);
        let log_path = paths
            .toadstool_log_dir()
            .join("logs-biome-svc")
            .join("test-primal.log");
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }
        let _ = fs::write(&log_path, "test log line\n").await;

        let result = executor
            .show_logs("logs-biome-svc.test-primal", false, 10, false, None, None)
            .await;
        assert!(
            result.is_ok(),
            "show_logs for primal should succeed: {:?}",
            result
        );

        let _ = executor.down_biome("logs-biome-svc", true, 5, false).await;
        let _ = executor.purge_biome_data("logs-biome-svc").await;
    }
}
