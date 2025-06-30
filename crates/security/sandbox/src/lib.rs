//! Cross-Platform Security Sandboxing for ToadStool
//!
//! This crate provides comprehensive security sandboxing capabilities including:
//! - Cross-platform process isolation
//! - Advanced seccomp filtering (Linux)
//! - Capability-based access control
//! - Resource containment and monitoring
//! - Security policy enforcement

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::security::{IsolationLevel, SecurityContext};
use toadstool::workload::WorkloadSpec;
use toadstool_security_policies::{PolicyManager, SecurityPolicy};

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable advanced sandboxing features
    pub advanced_features_enabled: bool,
    /// Default isolation level
    pub default_isolation_level: IsolationLevel,
    /// Enable seccomp filtering (Linux only)
    pub enable_seccomp: bool,
    /// Enable capability dropping
    pub enable_capability_dropping: bool,
    /// Enable namespace isolation
    pub enable_namespace_isolation: bool,
    /// Enable resource limits enforcement
    pub enable_resource_limits: bool,
    /// Sandbox root directory
    pub sandbox_root: PathBuf,
    /// Temporary directory for sandbox operations
    pub temp_dir: PathBuf,
    /// Maximum number of concurrent sandboxes
    pub max_concurrent_sandboxes: u32,
    /// Sandbox cleanup timeout in seconds
    pub cleanup_timeout_secs: u64,
    /// Enable sandbox monitoring
    pub enable_monitoring: bool,
    /// Monitoring interval in milliseconds
    pub monitoring_interval_ms: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            advanced_features_enabled: true,
            default_isolation_level: IsolationLevel::Standard,
            enable_seccomp: cfg!(target_os = "linux"),
            enable_capability_dropping: true,
            enable_namespace_isolation: cfg!(target_os = "linux"),
            enable_resource_limits: true,
            sandbox_root: PathBuf::from("/var/lib/toadstool/sandbox"),
            temp_dir: PathBuf::from("/tmp/toadstool"),
            max_concurrent_sandboxes: 100,
            cleanup_timeout_secs: 30,
            enable_monitoring: true,
            monitoring_interval_ms: 1000,
        }
    }
}

/// Sandbox specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSpec {
    /// Unique sandbox identifier
    pub sandbox_id: String,
    /// Workload to be sandboxed
    pub workload: WorkloadSpec,
    /// Security context
    pub security_context: SecurityContext,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// File system mounts
    pub filesystem_mounts: Vec<FilesystemMount>,
    /// Network configuration
    pub network_config: NetworkConfig,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Working directory inside sandbox
    pub working_directory: Option<PathBuf>,
    /// Sandbox lifetime
    pub lifetime: SandboxLifetime,
}

/// Resource limits for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: Option<u64>,
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: Option<f64>,
    /// Maximum number of file descriptors
    pub max_file_descriptors: Option<u32>,
    /// Maximum number of processes/threads
    pub max_processes: Option<u32>,
    /// Maximum disk usage in bytes
    pub max_disk_bytes: Option<u64>,
    /// Maximum network bandwidth in bytes/second
    pub max_network_bps: Option<u64>,
    /// Maximum execution time
    pub max_execution_time: Option<Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(512 * 1024 * 1024), // 512MB
            max_cpu_percent: Some(80.0),
            max_file_descriptors: Some(1024),
            max_processes: Some(100),
            max_disk_bytes: Some(1024 * 1024 * 1024), // 1GB
            max_network_bps: Some(10 * 1024 * 1024), // 10MB/s
            max_execution_time: Some(Duration::from_secs(300)), // 5 minutes
        }
    }
}

/// Filesystem mount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemMount {
    /// Source path (host)
    pub source: PathBuf,
    /// Target path (sandbox)
    pub target: PathBuf,
    /// Mount type
    pub mount_type: MountType,
    /// Mount options
    pub options: Vec<String>,
}

/// Mount type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MountType {
    /// Read-only bind mount
    ReadOnlyBind,
    /// Read-write bind mount
    ReadWriteBind,
    /// Temporary filesystem
    TmpFs,
    /// Device mount
    Device,
    /// Proc filesystem
    Proc,
    /// Sys filesystem
    Sys,
}

/// Network configuration for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable network access
    pub enabled: bool,
    /// Network isolation mode
    pub isolation_mode: NetworkIsolationMode,
    /// Allowed outbound hosts
    pub allowed_hosts: Vec<String>,
    /// Allowed outbound ports
    pub allowed_ports: Vec<u16>,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Network bandwidth limits
    pub bandwidth_limits: Option<BandwidthLimits>,
}

/// Network isolation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkIsolationMode {
    /// No network isolation
    None,
    /// Basic firewall rules
    Firewall,
    /// Network namespace isolation
    Namespace,
    /// Complete network isolation
    Isolated,
}

/// Network bandwidth limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthLimits {
    /// Upload limit in bytes/second
    pub upload_bps: u64,
    /// Download limit in bytes/second
    pub download_bps: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            isolation_mode: NetworkIsolationMode::Firewall,
            allowed_hosts: Vec::new(),
            allowed_ports: Vec::new(),
            dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
            bandwidth_limits: None,
        }
    }
}

/// Sandbox lifetime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxLifetime {
    /// Ephemeral sandbox (destroyed after execution)
    Ephemeral,
    /// Persistent sandbox with TTL
    Persistent { ttl: Duration },
    /// Manual cleanup required
    Manual,
}

/// Sandbox status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxStatus {
    /// Sandbox is being created
    Creating,
    /// Sandbox is ready for execution
    Ready,
    /// Sandbox is running workload
    Running,
    /// Sandbox execution completed
    Completed,
    /// Sandbox failed
    Failed,
    /// Sandbox is being destroyed
    Destroying,
    /// Sandbox has been destroyed
    Destroyed,
}

/// Sandbox information
#[derive(Debug, Clone)]
pub struct SandboxInfo {
    /// Sandbox identifier
    pub sandbox_id: String,
    /// Current status
    pub status: SandboxStatus,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last updated timestamp
    pub updated_at: SystemTime,
    /// Process ID (if running)
    pub process_id: Option<u32>,
    /// Resource usage statistics
    pub resource_usage: ResourceUsage,
    /// Security violations (if any)
    pub security_violations: Vec<SecurityViolation>,
    /// Sandbox metadata
    pub metadata: HashMap<String, String>,
}

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_bytes: u64,
    /// Current CPU usage percentage
    pub cpu_percent: f64,
    /// Number of open file descriptors
    pub file_descriptors: u32,
    /// Number of running processes
    pub processes: u32,
    /// Disk usage in bytes
    pub disk_bytes: u64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Execution time
    pub execution_time: Duration,
}

/// Security violation information
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// Violation type
    pub violation_type: String,
    /// Violation description
    pub description: String,
    /// Timestamp of violation
    pub timestamp: SystemTime,
    /// Severity level
    pub severity: ViolationSeverity,
    /// Action taken
    pub action_taken: String,
}

/// Violation severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Sandbox manager trait
#[async_trait]
pub trait SandboxManager: Send + Sync {
    /// Create a new sandbox
    async fn create_sandbox(&self, spec: SandboxSpec) -> ToadStoolResult<String>;
    
    /// Start execution in sandbox
    async fn start_execution(&self, sandbox_id: &str) -> ToadStoolResult<()>;
    
    /// Stop execution in sandbox
    async fn stop_execution(&self, sandbox_id: &str) -> ToadStoolResult<()>;
    
    /// Destroy sandbox
    async fn destroy_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<()>;
    
    /// Get sandbox information
    async fn get_sandbox_info(&self, sandbox_id: &str) -> ToadStoolResult<SandboxInfo>;
    
    /// List all sandboxes
    async fn list_sandboxes(&self) -> ToadStoolResult<Vec<String>>;
    
    /// Monitor sandbox resource usage
    async fn monitor_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<ResourceUsage>;
    
    /// Apply security policy to sandbox
    async fn apply_security_policy(
        &self,
        sandbox_id: &str,
        policy: &SecurityPolicy,
    ) -> ToadStoolResult<()>;
    
    /// Get sandbox logs
    async fn get_sandbox_logs(&self, sandbox_id: &str) -> ToadStoolResult<Vec<String>>;
}

/// Cross-platform sandbox manager implementation
pub struct CrossPlatformSandboxManager {
    config: SandboxConfig,
    sandboxes: Arc<RwLock<HashMap<String, SandboxInfo>>>,
    policy_manager: Arc<dyn PolicyManager>,
    
    #[cfg(target_os = "linux")]
    linux_manager: LinuxSandboxManager,
    
    #[cfg(target_os = "macos")]
    macos_manager: MacOSSandboxManager,
    
    #[cfg(windows)]
    windows_manager: WindowsSandboxManager,
}

impl CrossPlatformSandboxManager {
    /// Create new cross-platform sandbox manager
    pub async fn new(
        config: SandboxConfig,
        policy_manager: Arc<dyn PolicyManager>,
    ) -> ToadStoolResult<Self> {
        info!("Creating cross-platform sandbox manager");
        
        // Ensure sandbox directories exist
        tokio::fs::create_dir_all(&config.sandbox_root).await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to create sandbox root directory {}: {}",
                config.sandbox_root.display(),
                e
            )))?;
        
        tokio::fs::create_dir_all(&config.temp_dir).await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to create temp directory {}: {}",
                config.temp_dir.display(),
                e
            )))?;
        
        Ok(Self {
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            
            #[cfg(target_os = "linux")]
            linux_manager: LinuxSandboxManager::new(config.clone()),
            
            #[cfg(target_os = "macos")]
            macos_manager: MacOSSandboxManager::new(config.clone()).await?,
            
            #[cfg(windows)]
            windows_manager: WindowsSandboxManager::new(config.clone()).await?,
            
            config,
            policy_manager,
        })
    }
    
    /// Generate unique sandbox ID
    fn generate_sandbox_id(&self) -> String {
        format!("sandbox_{}", Uuid::new_v4().simple())
    }
    
    /// Validate sandbox specification
    async fn validate_sandbox_spec(&self, spec: &SandboxSpec) -> ToadStoolResult<()> {
        // Validate resource limits
        if let Some(memory) = spec.resource_limits.max_memory_bytes {
            if memory == 0 {
                return Err(ToadStoolError::validation("Memory limit cannot be zero".to_string()));
            }
        }
        
        if let Some(cpu) = spec.resource_limits.max_cpu_percent {
            if cpu <= 0.0 || cpu > 100.0 {
                return Err(ToadStoolError::validation("CPU limit must be between 0 and 100".to_string()));
            }
        }
        
        // Validate filesystem mounts
        for mount in &spec.filesystem_mounts {
            if !mount.source.exists() && !matches!(mount.mount_type, MountType::TmpFs) {
                return Err(ToadStoolError::validation(format!(
                    "Mount source does not exist: {}",
                    mount.source.display()
                )));
            }
        }
        
        // Validate network configuration
        if spec.network_config.enabled {
            for host in &spec.network_config.allowed_hosts {
                if host.is_empty() {
                    return Err(ToadStoolError::validation("Empty host in allowed hosts".to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    /// Create sandbox directory structure
    async fn create_sandbox_directories(&self, sandbox_id: &str) -> ToadStoolResult<PathBuf> {
        let sandbox_dir = self.config.sandbox_root.join(sandbox_id);
        
        tokio::fs::create_dir_all(&sandbox_dir).await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to create sandbox directory {}: {}",
                sandbox_dir.display(),
                e
            )))?;
        
        // Create standard directories
        let dirs = ["bin", "etc", "tmp", "var", "proc", "sys", "dev"];
        for dir in &dirs {
            let dir_path = sandbox_dir.join(dir);
            tokio::fs::create_dir_all(&dir_path).await
                .map_err(|e| ToadStoolError::configuration(format!(
                    "Failed to create sandbox subdirectory {}: {}",
                    dir_path.display(),
                    e
                )))?;
        }
        
        Ok(sandbox_dir)
    }
    
    /// Setup filesystem mounts for sandbox
    async fn setup_filesystem_mounts(
        &self,
        sandbox_dir: &PathBuf,
        mounts: &[FilesystemMount],
    ) -> ToadStoolResult<()> {
        for mount in mounts {
            let target_path = sandbox_dir.join(&mount.target);
            
            // Ensure target directory exists
            if let Some(parent) = target_path.parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| ToadStoolError::configuration(format!(
                        "Failed to create mount target directory {}: {}",
                        parent.display(),
                        e
                    )))?;
            }
            
            // Platform-specific mount implementation
            #[cfg(target_os = "linux")]
            self.linux_manager.setup_mount(mount, &target_path).await?;
            
            #[cfg(target_os = "macos")]
            self.macos_manager.setup_mount(mount, &target_path).await?;
            
            #[cfg(windows)]
            self.windows_manager.setup_mount(mount, &target_path).await?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl SandboxManager for CrossPlatformSandboxManager {
    async fn create_sandbox(&self, mut spec: SandboxSpec) -> ToadStoolResult<String> {
        info!("Creating sandbox for workload: {:?}", spec.workload);
        
        // Generate sandbox ID if not provided
        if spec.sandbox_id.is_empty() {
            spec.sandbox_id = self.generate_sandbox_id();
        }
        
        let sandbox_id = spec.sandbox_id.clone();
        
        // Validate specification
        self.validate_sandbox_spec(&spec).await?;
        
        // Create sandbox directories
        let sandbox_dir = self.create_sandbox_directories(&sandbox_id).await?;
        
        // Setup filesystem mounts
        self.setup_filesystem_mounts(&sandbox_dir, &spec.filesystem_mounts).await?;
        
        // Create sandbox info
        let sandbox_info = SandboxInfo {
            sandbox_id: sandbox_id.clone(),
            status: SandboxStatus::Creating,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            process_id: None,
            resource_usage: ResourceUsage::default(),
            security_violations: Vec::new(),
            metadata: HashMap::new(),
        };
        
        // Store sandbox info
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.insert(sandbox_id.clone(), sandbox_info);
        }
        
        // Platform-specific sandbox creation
        #[cfg(target_os = "linux")]
        self.linux_manager.create_sandbox(&spec, &sandbox_dir).await?;
        
        #[cfg(target_os = "macos")]
        self.macos_manager.create_sandbox(&spec, &sandbox_dir).await?;
        
        #[cfg(windows)]
        self.windows_manager.create_sandbox(&spec, &sandbox_dir).await?;
        
        // Update status to ready
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(&sandbox_id) {
                info.status = SandboxStatus::Ready;
                info.updated_at = SystemTime::now();
            }
        }
        
        info!("Sandbox {} created successfully", sandbox_id);
        Ok(sandbox_id)
    }
    
    async fn start_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Starting execution in sandbox: {}", sandbox_id);
        
        // Update status to running
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                if info.status != SandboxStatus::Ready {
                    return Err(ToadStoolError::runtime(format!(
                        "Sandbox {} is not ready for execution (status: {:?})",
                        sandbox_id,
                        info.status
                    )));
                }
                info.status = SandboxStatus::Running;
                info.updated_at = SystemTime::now();
            } else {
                return Err(ToadStoolError::runtime(format!(
                    "Sandbox {} not found",
                    sandbox_id
                )));
            }
        }
        
        // Platform-specific execution start
        #[cfg(target_os = "linux")]
        self.linux_manager.start_execution(sandbox_id).await?;
        
        #[cfg(target_os = "macos")]
        self.macos_manager.start_execution(sandbox_id).await?;
        
        #[cfg(windows)]
        self.windows_manager.start_execution(sandbox_id).await?;
        
        info!("Execution started in sandbox: {}", sandbox_id);
        Ok(())
    }
    
    async fn stop_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Stopping execution in sandbox: {}", sandbox_id);
        
        // Platform-specific execution stop
        #[cfg(target_os = "linux")]
        self.linux_manager.stop_execution(sandbox_id).await?;
        
        #[cfg(target_os = "macos")]
        self.macos_manager.stop_execution(sandbox_id).await?;
        
        #[cfg(windows)]
        self.windows_manager.stop_execution(sandbox_id).await?;
        
        // Update status
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                info.status = SandboxStatus::Completed;
                info.updated_at = SystemTime::now();
                info.process_id = None;
            }
        }
        
        info!("Execution stopped in sandbox: {}", sandbox_id);
        Ok(())
    }
    
    async fn destroy_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Destroying sandbox: {}", sandbox_id);
        
        // Update status to destroying
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                info.status = SandboxStatus::Destroying;
                info.updated_at = SystemTime::now();
            }
        }
        
        // Stop execution if running
        if let Ok(info) = self.get_sandbox_info(sandbox_id).await {
            if info.status == SandboxStatus::Running {
                let _ = self.stop_execution(sandbox_id).await;
            }
        }
        
        // Platform-specific cleanup
        #[cfg(target_os = "linux")]
        self.linux_manager.destroy_sandbox(sandbox_id).await?;
        
        #[cfg(target_os = "macos")]
        self.macos_manager.destroy_sandbox(sandbox_id).await?;
        
        #[cfg(windows)]
        self.windows_manager.destroy_sandbox(sandbox_id).await?;
        
        // Remove sandbox directory
        let sandbox_dir = self.config.sandbox_root.join(sandbox_id);
        if sandbox_dir.exists() {
            tokio::fs::remove_dir_all(&sandbox_dir).await
                .map_err(|e| ToadStoolError::configuration(format!(
                    "Failed to remove sandbox directory {}: {}",
                    sandbox_dir.display(),
                    e
                )))?;
        }
        
        // Remove from tracking
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.remove(sandbox_id);
        }
        
        info!("Sandbox {} destroyed successfully", sandbox_id);
        Ok(())
    }
    
    async fn get_sandbox_info(&self, sandbox_id: &str) -> ToadStoolResult<SandboxInfo> {
        let sandboxes = self.sandboxes.read().await;
        sandboxes.get(sandbox_id)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime(format!(
                "Sandbox {} not found",
                sandbox_id
            )))
    }
    
    async fn list_sandboxes(&self) -> ToadStoolResult<Vec<String>> {
        let sandboxes = self.sandboxes.read().await;
        Ok(sandboxes.keys().cloned().collect())
    }
    
    async fn monitor_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<ResourceUsage> {
        debug!("Monitoring sandbox: {}", sandbox_id);
        
        // Platform-specific monitoring
        #[cfg(target_os = "linux")]
        let usage = self.linux_manager.monitor_sandbox(sandbox_id).await?;
        
        #[cfg(target_os = "macos")]
        let usage = self.macos_manager.monitor_sandbox(sandbox_id).await?;
        
        #[cfg(windows)]
        let usage = self.windows_manager.monitor_sandbox(sandbox_id).await?;
        
        // Update stored resource usage
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(sandbox_id) {
                info.resource_usage = usage.clone();
                info.updated_at = SystemTime::now();
            }
        }
        
        Ok(usage)
    }
    
    async fn apply_security_policy(
        &self,
        sandbox_id: &str,
        policy: &SecurityPolicy,
    ) -> ToadStoolResult<()> {
        debug!("Applying security policy to sandbox: {}", sandbox_id);
        
        // Platform-specific policy application
        #[cfg(target_os = "linux")]
        self.linux_manager.apply_security_policy(sandbox_id, policy).await?;
        
        #[cfg(target_os = "macos")]
        self.macos_manager.apply_security_policy(sandbox_id, policy).await?;
        
        #[cfg(windows)]
        self.windows_manager.apply_security_policy(sandbox_id, policy).await?;
        
        info!("Security policy applied to sandbox: {}", sandbox_id);
        Ok(())
    }
    
    async fn get_sandbox_logs(&self, sandbox_id: &str) -> ToadStoolResult<Vec<String>> {
        debug!("Getting logs for sandbox: {}", sandbox_id);
        
        // Platform-specific log retrieval
        #[cfg(target_os = "linux")]
        let logs = self.linux_manager.get_sandbox_logs(sandbox_id).await?;
        
        #[cfg(target_os = "macos")]
        let logs = self.macos_manager.get_sandbox_logs(sandbox_id).await?;
        
        #[cfg(windows)]
        let logs = self.windows_manager.get_sandbox_logs(sandbox_id).await?;
        
        Ok(logs)
    }
}

// Platform-specific implementations
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxSandboxManager;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOSSandboxManager;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::WindowsSandboxManager;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use toadstool_security_policies::FilePolicyManager;
    
    fn create_test_config() -> SandboxConfig {
        let temp_dir = TempDir::new().unwrap();
        SandboxConfig {
            sandbox_root: temp_dir.path().join("sandbox"),
            temp_dir: temp_dir.path().join("temp"),
            ..Default::default()
        }
    }
    
    fn create_test_sandbox_spec() -> SandboxSpec {
        SandboxSpec {
            sandbox_id: "test-sandbox".to_string(),
            workload: WorkloadSpec::Native {
                executable: toadstool::workload::ExecutableSource::File {
                    path: PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            },
            security_context: SecurityContext::default(),
            resource_limits: ResourceLimits::default(),
            filesystem_mounts: Vec::new(),
            network_config: NetworkConfig::default(),
            environment: HashMap::new(),
            working_directory: None,
            lifetime: SandboxLifetime::Ephemeral,
        }
    }
    
    #[tokio::test]
    async fn test_sandbox_manager_creation() {
        let config = create_test_config();
        let policy_config = toadstool_security_policies::PolicyManagerConfig {
            policy_dir: config.temp_dir.join("policies"),
            ..Default::default()
        };
        let policy_manager = Arc::new(FilePolicyManager::new(policy_config).unwrap());
        
        let manager = CrossPlatformSandboxManager::new(config, policy_manager).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_sandbox_spec_validation() {
        let config = create_test_config();
        let policy_config = toadstool_security_policies::PolicyManagerConfig {
            policy_dir: config.temp_dir.join("policies"),
            ..Default::default()
        };
        let policy_manager = Arc::new(FilePolicyManager::new(policy_config).unwrap());
        
        let manager = CrossPlatformSandboxManager::new(config, policy_manager).await.unwrap();
        let spec = create_test_sandbox_spec();
        
        let result = manager.validate_sandbox_spec(&spec).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_sandbox_directory_creation() {
        let config = create_test_config();
        let policy_config = toadstool_security_policies::PolicyManagerConfig {
            policy_dir: config.temp_dir.join("policies"),
            ..Default::default()
        };
        let policy_manager = Arc::new(FilePolicyManager::new(policy_config).unwrap());
        
        let manager = CrossPlatformSandboxManager::new(config, policy_manager).await.unwrap();
        let sandbox_id = "test-sandbox";
        
        let result = manager.create_sandbox_directories(sandbox_id).await;
        assert!(result.is_ok());
        
        let sandbox_dir = result.unwrap();
        assert!(sandbox_dir.exists());
        assert!(sandbox_dir.join("bin").exists());
        assert!(sandbox_dir.join("etc").exists());
    }
}
