//! # CLI Command Handler
//!
//! Implementation of all CLI commands for toadStool.

use crate::manifest::{BiomeManifest, BiomeRuntime, BiomeStatus, ManifestLoader, ManifestError};
use crate::scheduler::{WorkloadScheduler, SchedulerError};
use crate::federation::{FederationManager, FederationError};
use crate::resources::{ResourceManager, ResourceError};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// CLI-specific errors
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),
    
    #[error("Scheduler error: {0}")]
    Scheduler(#[from] SchedulerError),
    
    #[error("Federation error: {0}")]
    Federation(#[from] FederationError),
    
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),
    
    #[error("Biome not found: {name}")]
    BiomeNotFound { name: String },
    
    #[error("Biome already exists: {name}")]
    BiomeAlreadyExists { name: String },
    
    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("System error: {message}")]
    System { message: String },
}

/// Main CLI handler
pub struct CliHandler {
    config_path: Option<PathBuf>,
    biomes: Arc<RwLock<HashMap<String, BiomeRuntime>>>,
    scheduler: Arc<WorkloadScheduler>,
    federation: Arc<FederationManager>,
    resource_manager: Arc<ResourceManager>,
    manifest_loader: ManifestLoader,
}

impl CliHandler {
    pub async fn new(config_path: Option<PathBuf>) -> Result<Self, CliError> {
        let scheduler = Arc::new(WorkloadScheduler::new().await?);
        let federation = Arc::new(FederationManager::new().await?);
        let resource_manager = Arc::new(ResourceManager::new().await?);
        
        Ok(Self {
            config_path,
            biomes: Arc::new(RwLock::new(HashMap::new())),
            scheduler,
            federation,
            resource_manager,
            manifest_loader: ManifestLoader::new(false),
        })
    }

    /// Run a biome from manifest (foreground)
    pub async fn run_biome(
        &mut self,
        manifest_path: PathBuf,
        foreground: bool,
        mut shutdown_signal: broadcast::Receiver<()>,
    ) -> Result<(), CliError> {
        info!("Running biome from manifest: {}", manifest_path.display());
        
        // Load and validate manifest
        let manifest = self.manifest_loader.load_from_file(&manifest_path).await?;
        let biome_name = manifest.metadata.name.clone();
        
        // Check if biome already exists
        {
            let biomes = self.biomes.read().await;
            if let Some(existing) = biomes.get(&biome_name) {
                if existing.is_running() {
                    return Err(CliError::BiomeAlreadyExists { name: biome_name });
                }
            }
        }
        
        // Create biome runtime
        let mut biome_runtime = BiomeRuntime::new(manifest);
        biome_runtime.status = BiomeStatus::Starting;
        biome_runtime.started_at = Some(chrono::Utc::now());
        
        info!("Starting biome: {}", biome_name);
        
        // Schedule the biome
        let biome_id = biome_runtime.id;
        self.scheduler.schedule_biome(&biome_runtime).await?;
        
        // Update biome status
        biome_runtime.status = BiomeStatus::Running;
        
        // Store biome runtime
        {
            let mut biomes = self.biomes.write().await;
            biomes.insert(biome_name.clone(), biome_runtime);
        }
        
        info!("Biome {} is now running", biome_name);
        
        if foreground {
            // Wait for shutdown signal or biome completion
            tokio::select! {
                _ = shutdown_signal.recv() => {
                    info!("Received shutdown signal, stopping biome...");
                    self.stop_biome_internal(&biome_name, false, 30).await?;
                }
                result = self.scheduler.wait_for_biome(biome_id) => {
                    match result {
                        Ok(exit_code) => {
                            info!("Biome {} completed with exit code: {}", biome_name, exit_code);
                            self.update_biome_status(&biome_name, BiomeStatus::Stopped, Some(exit_code)).await?;
                        }
                        Err(e) => {
                            error!("Biome {} failed: {}", biome_name, e);
                            self.update_biome_status(&biome_name, BiomeStatus::Failed, None).await?;
                            return Err(e.into());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Start biome in background
    pub async fn start_biome(
        &mut self,
        manifest_path: PathBuf,
        detached: bool,
        name_override: Option<String>,
    ) -> Result<(), CliError> {
        info!("Starting biome from manifest: {}", manifest_path.display());
        
        // Load and validate manifest
        let mut manifest = self.manifest_loader.load_from_file(&manifest_path).await?;
        
        // Override name if provided
        if let Some(name) = name_override {
            manifest.metadata.name = name;
        }
        
        let biome_name = manifest.metadata.name.clone();
        
        // Check if biome already exists
        {
            let biomes = self.biomes.read().await;
            if let Some(existing) = biomes.get(&biome_name) {
                if existing.is_running() {
                    return Err(CliError::BiomeAlreadyExists { name: biome_name });
                }
            }
        }
        
        // Create biome runtime
        let mut biome_runtime = BiomeRuntime::new(manifest);
        biome_runtime.status = BiomeStatus::Starting;
        biome_runtime.started_at = Some(chrono::Utc::now());
        
        // Schedule the biome
        let biome_id = biome_runtime.id;
        self.scheduler.schedule_biome(&biome_runtime).await?;
        
        // Update biome status
        biome_runtime.status = BiomeStatus::Running;
        
        // Store biome runtime
        {
            let mut biomes = self.biomes.write().await;
            biomes.insert(biome_name.clone(), biome_runtime);
        }
        
        info!("Biome {} started successfully", biome_name);
        
        if !detached {
            println!("Biome '{}' started with ID: {}", biome_name, biome_id);
        }
        
        Ok(())
    }

    /// List running biomes
    pub async fn list_biomes(
        &self,
        format: String,
        show_all: bool,
        filter: Option<String>,
    ) -> Result<(), CliError> {
        let biomes = self.biomes.read().await;
        
        let mut filtered_biomes: Vec<&BiomeRuntime> = biomes
            .values()
            .filter(|biome| {
                if !show_all && biome.is_stopped() {
                    return false;
                }
                
                if let Some(filter_pattern) = &filter {
                    return biome.name.contains(filter_pattern);
                }
                
                true
            })
            .collect();
        
        // Sort by name
        filtered_biomes.sort_by(|a, b| a.name.cmp(&b.name));
        
        match format.as_str() {
            "json" => {
                let json_output = serde_json::to_string_pretty(&filtered_biomes)?;
                println!("{}", json_output);
            }
            "yaml" => {
                let yaml_output = serde_yaml::to_string(&filtered_biomes)
                    .map_err(|e| CliError::System { message: e.to_string() })?;
                println!("{}", yaml_output);
            }
            "table" | _ => {
                self.print_biomes_table(&filtered_biomes);
            }
        }
        
        Ok(())
    }

    /// Show biome logs
    pub async fn show_logs(
        &self,
        biome_name: String,
        follow: bool,
        tail: u32,
        timestamps: bool,
        mut shutdown_signal: broadcast::Receiver<()>,
    ) -> Result<(), CliError> {
        let biome = {
            let biomes = self.biomes.read().await;
            biomes.get(&biome_name)
                .ok_or_else(|| CliError::BiomeNotFound { name: biome_name.clone() })?
                .clone()
        };
        
        info!("Showing logs for biome: {}", biome_name);
        
        if follow {
            // Follow logs until shutdown signal
            let mut log_stream = self.scheduler.get_log_stream(biome.id).await?;
            
            loop {
                tokio::select! {
                    log_line = log_stream.recv() => {
                        match log_line {
                            Ok(line) => {
                                if timestamps {
                                    println!("[{}] {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), line);
                                } else {
                                    println!("{}", line);
                                }
                            }
                            Err(e) => {
                                error!("Error reading logs: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_signal.recv() => {
                        info!("Received shutdown signal, stopping log follow");
                        break;
                    }
                }
            }
        } else {
            // Get last N lines
            let logs = self.scheduler.get_logs(biome.id, tail).await?;
            
            for log_line in logs {
                if timestamps {
                    println!("[{}] {}", log_line.timestamp.format("%Y-%m-%d %H:%M:%S"), log_line.message);
                } else {
                    println!("{}", log_line.message);
                }
            }
        }
        
        Ok(())
    }

    /// Stop a running biome
    pub async fn stop_biome(
        &mut self,
        biome_name: String,
        force: bool,
        timeout: u64,
    ) -> Result<(), CliError> {
        self.stop_biome_internal(&biome_name, force, timeout).await
    }

    async fn stop_biome_internal(
        &mut self,
        biome_name: &str,
        force: bool,
        timeout: u64,
    ) -> Result<(), CliError> {
        let biome = {
            let biomes = self.biomes.read().await;
            biomes.get(biome_name)
                .ok_or_else(|| CliError::BiomeNotFound { name: biome_name.to_string() })?
                .clone()
        };
        
        if !biome.is_running() {
            warn!("Biome {} is not running", biome_name);
            return Ok(());
        }
        
        info!("Stopping biome: {}", biome_name);
        
        // Update status to stopping
        self.update_biome_status(biome_name, BiomeStatus::Stopping, None).await?;
        
        // Stop the biome
        let exit_code = if force {
            self.scheduler.force_stop_biome(biome.id).await?
        } else {
            self.scheduler.stop_biome(biome.id, timeout).await?
        };
        
        // Update final status
        self.update_biome_status(biome_name, BiomeStatus::Stopped, Some(exit_code)).await?;
        
        info!("Biome {} stopped successfully", biome_name);
        
        Ok(())
    }

    /// Show federation status
    pub async fn federation_status(&self, format: String) -> Result<(), CliError> {
        let status = self.federation.get_status().await?;
        
        match format.as_str() {
            "json" => {
                let json_output = serde_json::to_string_pretty(&status)?;
                println!("{}", json_output);
            }
            "yaml" => {
                let yaml_output = serde_yaml::to_string(&status)
                    .map_err(|e| CliError::System { message: e.to_string() })?;
                println!("{}", yaml_output);
            }
            "table" | _ => {
                self.print_federation_status(&status);
            }
        }
        
        Ok(())
    }

    /// List federation peers
    pub async fn federation_peers(&self, show_all: bool) -> Result<(), CliError> {
        let peers = self.federation.get_peers(show_all).await?;
        
        self.print_federation_peers(&peers);
        
        Ok(())
    }

    /// Join a federation
    pub async fn federation_join(&mut self, peer: String, trust_policy: String) -> Result<(), CliError> {
        info!("Joining federation peer: {} with trust policy: {}", peer, trust_policy);
        
        self.federation.join_peer(peer, trust_policy).await?;
        
        println!("Successfully joined federation");
        
        Ok(())
    }

    /// Leave federation
    pub async fn federation_leave(&mut self, force: bool) -> Result<(), CliError> {
        info!("Leaving federation (force: {})", force);
        
        self.federation.leave(force).await?;
        
        println!("Successfully left federation");
        
        Ok(())
    }

    /// Show system information
    pub async fn system_info(&self, detailed: bool, format: String) -> Result<(), CliError> {
        let system_info = self.resource_manager.get_system_info(detailed).await?;
        
        match format.as_str() {
            "json" => {
                let json_output = serde_json::to_string_pretty(&system_info)?;
                println!("{}", json_output);
            }
            "yaml" => {
                let yaml_output = serde_yaml::to_string(&system_info)
                    .map_err(|e| CliError::System { message: e.to_string() })?;
                println!("{}", yaml_output);
            }
            "table" | _ => {
                self.print_system_info(&system_info, detailed);
            }
        }
        
        Ok(())
    }

    /// Validate a biome manifest
    pub async fn validate_manifest(&self, manifest_path: PathBuf, strict: bool) -> Result<(), CliError> {
        info!("Validating manifest: {}", manifest_path.display());
        
        let loader = ManifestLoader::new(strict);
        
        match loader.load_from_file(&manifest_path).await {
            Ok(manifest) => {
                println!("✓ Manifest is valid");
                println!("  Biome name: {}", manifest.metadata.name);
                println!("  API version: {}", manifest.api_version);
                println!("  Services: {}", manifest.services.len());
                println!("  Primals: {}", manifest.primals.len());
                
                if let Some(federation) = &manifest.federation {
                    println!("  Federation: {}", if federation.enabled { "enabled" } else { "disabled" });
                }
                
                Ok(())
            }
            Err(e) => {
                error!("✗ Manifest validation failed: {}", e);
                Err(e.into())
            }
        }
    }

    // Helper methods for printing output

    fn print_biomes_table(&self, biomes: &[&BiomeRuntime]) {
        if biomes.is_empty() {
            println!("No biomes found");
            return;
        }
        
        println!("{:<20} {:<12} {:<15} {:<20} {:<8}", "NAME", "STATUS", "CREATED", "STARTED", "SERVICES");
        println!("{:-<20} {:-<12} {:-<15} {:-<20} {:-<8}", "", "", "", "", "");
        
        for biome in biomes {
            let status = format!("{:?}", biome.status);
            let created = biome.created_at.format("%Y-%m-%d %H:%M").to_string();
            let started = biome.started_at
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let services = biome.manifest.services.len();
            
            println!("{:<20} {:<12} {:<15} {:<20} {:<8}", 
                biome.name, status, created, started, services);
        }
    }

    fn print_federation_status(&self, status: &crate::federation::FederationStatus) {
        println!("Federation Status:");
        println!("  Enabled: {}", status.enabled);
        println!("  Node ID: {}", status.node_id);
        println!("  Peers: {}", status.peer_count);
        println!("  Trust Policy: {}", status.trust_policy);
        
        if let Some(network) = &status.network_info {
            println!("  Network:");
            println!("    Listen Address: {}", network.listen_address);
            println!("    Public Address: {}", network.public_address);
        }
    }

    fn print_federation_peers(&self, peers: &[crate::federation::PeerInfo]) {
        if peers.is_empty() {
            println!("No federation peers found");
            return;
        }
        
        println!("{:<20} {:<15} {:<12} {:<20}", "PEER ID", "ADDRESS", "STATUS", "LAST SEEN");
        println!("{:-<20} {:-<15} {:-<12} {:-<20}", "", "", "", "");
        
        for peer in peers {
            let last_seen = peer.last_seen
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            
            println!("{:<20} {:<15} {:<12} {:<20}", 
                peer.id, peer.address, format!("{:?}", peer.status), last_seen);
        }
    }

    fn print_system_info(&self, info: &crate::resources::SystemInfo, detailed: bool) {
        println!("System Information:");
        println!("  OS: {}", info.os_info);
        println!("  Kernel: {}", info.kernel_version);
        println!("  Architecture: {}", info.architecture);
        println!("  Uptime: {}", info.uptime);
        
        println!("\nResources:");
        println!("  CPU Cores: {}", info.cpu_cores);
        println!("  CPU Usage: {:.1}%", info.cpu_usage);
        println!("  Memory: {:.1}% ({} / {})", 
            info.memory_usage_percent, 
            info.memory_used, 
            info.memory_total);
        println!("  Disk: {:.1}% ({} / {})", 
            info.disk_usage_percent, 
            info.disk_used, 
            info.disk_total);
        
        if detailed {
            println!("\nRuntime Information:");
            println!("  WASM Runtime: {}", info.wasm_runtime);
            println!("  Container Runtime: {}", info.container_runtime);
            println!("  Python Runtime: {}", info.python_runtime);
            
            if let Some(federation) = &info.federation_info {
                println!("\nFederation:");
                println!("  Status: {}", federation.status);
                println!("  Peers: {}", federation.peer_count);
            }
        }
    }

    async fn update_biome_status(
        &self,
        biome_name: &str,
        status: BiomeStatus,
        exit_code: Option<i32>,
    ) -> Result<(), CliError> {
        let mut biomes = self.biomes.write().await;
        
        if let Some(biome) = biomes.get_mut(biome_name) {
            biome.status = status;
            biome.exit_code = exit_code;
            
            if matches!(status, BiomeStatus::Stopped | BiomeStatus::Failed) {
                biome.stopped_at = Some(chrono::Utc::now());
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_cli_handler_creation() {
        let handler = CliHandler::new(None).await;
        assert!(handler.is_ok());
    }

    #[tokio::test]
    async fn test_validate_manifest() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
apiVersion: biomeOS/v1
kind: Biome
metadata:
  name: test-biome
services:
  - name: test-service
    runtime: wasm
"#).unwrap();
        
        let handler = CliHandler::new(None).await.unwrap();
        let result = handler.validate_manifest(temp_file.path().to_path_buf(), false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_empty_biomes() {
        let handler = CliHandler::new(None).await.unwrap();
        let result = handler.list_biomes("table".to_string(), false, None).await;
        assert!(result.is_ok());
    }
} 