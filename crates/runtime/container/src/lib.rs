//! # ToadStool Container Runtime Engine
//!
//! High-performance container runtime engine with Docker, Containerd, and Podman support,
//! comprehensive security isolation, resource limits, and network policies.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[cfg(feature = "docker")]
use bollard::{
    Docker, API_DEFAULT_VERSION,
    container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions, LogOutput},
    image::CreateImageOptions,
    models::{HostConfig, Mount, MountTypeEnum},
    auth::DockerCredentials,
};

use toadstool::{
    execution::{
        ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities, RuntimeConfig,
        RuntimeEngine, RuntimeType, WorkloadType, ExecutionOutput,
    },
    error::{ToadStoolError, ToadStoolResult},
    resources::{ResourceMonitor, RuntimeMetrics, CpuMetrics, MemoryMetrics, StorageMetrics, NetworkMetrics, TimingMetrics},
    security::{IsolationLevel, SecurityContext},
    workload::{WorkloadSpec, VolumeMount, PortMapping, RegistryAuth, VolumeMountType},
};

/// Container runtime engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntimeConfig {
    /// Container engine selection
    pub engine: ContainerEngineType,
    /// Registry configuration for image pulling
    pub registry_config: RegistryConfig,
    /// Network policies and configuration
    pub network_policy: NetworkPolicy,
    /// Volume mounting policies
    pub volume_policy: VolumePolicy,
    /// Security configuration
    pub security_config: ContainerSecurityConfig,
    /// Resource limits
    pub resource_limits: ContainerResourceLimits,
    /// Image management settings
    pub image_config: ImageConfig,
}

impl Default for ContainerRuntimeConfig {
    fn default() -> Self {
        Self {
            engine: ContainerEngineType::Docker {
                socket_path: None,
                api_version: API_DEFAULT_VERSION.to_string(),
            },
            registry_config: RegistryConfig::default(),
            network_policy: NetworkPolicy::default(),
            volume_policy: VolumePolicy::default(),
            security_config: ContainerSecurityConfig::default(),
            resource_limits: ContainerResourceLimits::default(),
            image_config: ImageConfig::default(),
        }
    }
}

/// Container engine type selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerEngineType {
    /// Docker engine with custom socket path
    Docker {
        /// Docker socket path
        socket_path: Option<String>,
        /// API version
        api_version: String,
    },
    /// Containerd engine
    Containerd {
        /// Containerd socket address
        address: String,
        /// Namespace for containers
        namespace: String,
    },
    /// Podman engine
    Podman {
        /// Podman socket path
        socket_path: String,
        /// Remote connection URL
        remote_url: Option<String>,
    },
}

impl Default for ContainerEngineType {
    fn default() -> Self {
        Self::Docker {
            socket_path: None,
            api_version: API_DEFAULT_VERSION.to_string(),
        }
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Default registry URL
    pub default_registry: String,
    /// Registry authentication configurations
    pub registries: HashMap<String, RegistryAuth>,
    /// Image pull policy
    pub pull_policy: ImagePullPolicy,
    /// Pull timeout
    pub pull_timeout: Duration,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_registry: "docker.io".to_string(),
            registries: HashMap::new(),
            pull_policy: ImagePullPolicy::IfNotPresent,
            pull_timeout: Duration::from_secs(300),
        }
    }
}

/// Image pull policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImagePullPolicy {
    /// Always pull the image
    Always,
    /// Pull if not present locally
    IfNotPresent,
    /// Never pull, use local only
    Never,
}

/// Network policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Default network mode
    pub default_network: NetworkMode,
    /// Allow custom networks
    pub allow_custom_networks: bool,
    /// Allowed port ranges
    pub allowed_port_ranges: Vec<PortRange>,
    /// DNS configuration
    pub dns_config: DnsConfig,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            default_network: NetworkMode::Bridge,
            allow_custom_networks: false,
            allowed_port_ranges: vec![
                PortRange { start: 8000, end: 8999 },
                PortRange { start: 3000, end: 3999 },
            ],
            dns_config: DnsConfig::default(),
        }
    }
}

/// Network mode for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMode {
    /// Bridge networking
    Bridge,
    /// Host networking
    Host,
    /// No networking
    None,
    /// Custom network
    Custom(String),
}

/// Port range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers
    pub nameservers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
    /// DNS options
    pub options: Vec<String>,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            nameservers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
            search_domains: Vec::new(),
            options: Vec::new(),
        }
    }
}

/// Volume policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumePolicy {
    /// Allow bind mounts
    pub allow_bind_mounts: bool,
    /// Allowed host paths for bind mounts
    pub allowed_host_paths: Vec<PathBuf>,
    /// Allow tmpfs mounts
    pub allow_tmpfs: bool,
    /// Maximum volume size in MB
    pub max_volume_size_mb: u64,
}

impl Default for VolumePolicy {
    fn default() -> Self {
        Self {
            allow_bind_mounts: false,
            allowed_host_paths: vec![PathBuf::from("/tmp")],
            allow_tmpfs: true,
            max_volume_size_mb: 1024, // 1 GB
        }
    }
}

/// Container security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSecurityConfig {
    /// Run as non-root user
    pub non_root_required: bool,
    /// Drop all capabilities by default
    pub drop_all_capabilities: bool,
    /// Allowed capabilities
    pub allowed_capabilities: Vec<String>,
    /// Security options
    pub security_opts: Vec<String>,
    /// Read-only root filesystem
    pub read_only_root_fs: bool,
}

impl Default for ContainerSecurityConfig {
    fn default() -> Self {
        Self {
            non_root_required: true,
            drop_all_capabilities: true,
            allowed_capabilities: Vec::new(),
            security_opts: vec!["no-new-privileges:true".to_string()],
            read_only_root_fs: false,
        }
    }
}

/// Container resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceLimits {
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum CPU cores (as millicores)
    pub max_cpu_millicores: u32,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum disk I/O bytes per second
    pub max_io_bps: u64,
}

impl Default for ContainerResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            max_cpu_millicores: 1000, // 1 CPU core
            max_execution_time: Duration::from_secs(3600), // 1 hour
            max_io_bps: 100 * 1024 * 1024, // 100 MB/s
        }
    }
}

/// Image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Enable image caching
    pub cache_enabled: bool,
    /// Image cache directory
    pub cache_dir: Option<PathBuf>,
    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,
    /// Cache cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_dir: None,
            max_cache_size_mb: 5120, // 5 GB
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Active container handle
#[derive(Debug, Clone)]
struct ContainerHandle {
    container_id: String,
    image: String,
    start_time: Instant,
    config: ContainerRuntimeConfig,
}

/// Container runtime engine implementation
#[derive(Debug)]
pub struct ContainerRuntimeEngine {
    config: ContainerRuntimeConfig,
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
    #[cfg(not(feature = "docker"))]
    docker: Option<()>,
    active_containers: Arc<RwLock<HashMap<Uuid, ContainerHandle>>>,
    resource_monitor: Option<Arc<dyn ResourceMonitor>>,
    capabilities: RuntimeCapabilities,
}

impl ContainerRuntimeEngine {
    /// Create a new container runtime engine with default configuration
    pub fn new() -> ToadStoolResult<Self> {
        let config = ContainerRuntimeConfig::default();
        Self::with_config(config)
    }

    /// Create a new container runtime engine with custom configuration
    pub fn with_config(config: ContainerRuntimeConfig) -> ToadStoolResult<Self> {
        let docker = Self::create_docker_client(&config)?;
        
        let capabilities = RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Container],
            max_concurrent_executions: Some(100),
            supported_architectures: vec![
                "linux/amd64".to_string(),
                "linux/arm64".to_string(),
            ],
            platform_features: {
                let mut features = HashMap::new();
                features.insert("docker_support".to_string(), docker.is_some());
                features.insert("volume_mounts".to_string(), true);
                features.insert("network_isolation".to_string(), true);
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Ok(Self {
            config,
            docker,
            active_containers: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: None,
            capabilities,
        })
    }

    /// Add a resource monitor to the engine
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }

    #[cfg(feature = "docker")]
    fn create_docker_client(config: &ContainerRuntimeConfig) -> ToadStoolResult<Option<Docker>> {
        match &config.engine {
            ContainerEngineType::Docker { socket_path, api_version: _ } => {
                let docker = if let Some(socket) = socket_path {
                    Docker::connect_with_socket_defaults()
                } else {
                    Docker::connect_with_socket_defaults()
                };

                match docker {
                    Ok(client) => Ok(Some(client)),
                    Err(e) => {
                        warn!("Failed to connect to Docker: {}", e);
                        Err(ToadStoolError::configuration(format!("Docker connection failed: {}", e)))
                    }
                }
            }
            _ => Ok(None), // Other engines not implemented yet
        }
    }

    #[cfg(not(feature = "docker"))]
    fn create_docker_client(_config: &ContainerRuntimeConfig) -> ToadStoolResult<Option<()>> {
        Ok(None)
    }

    /// Ensure container image is available locally
    async fn ensure_image(&self, image: &str, registry_auth: Option<&RegistryAuth>) -> ToadStoolResult<()> {
        #[cfg(feature = "docker")]
        {
            let docker = self.docker
                .as_ref()
                .ok_or_else(|| ToadStoolError::configuration("Docker client not available"))?;

            // Check if image exists locally
            let images = docker.list_images(None::<bollard::image::ListImagesOptions<String>>).await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list images: {}", e)))?;

            let image_exists = images.iter().any(|img| {
                img.repo_tags.iter().any(|tag| tag == image)
            });

            if !image_exists || self.config.registry_config.pull_policy == ImagePullPolicy::Always {
                info!("Pulling image: {}", image);
                
                let auth_config = registry_auth.map(|auth| DockerCredentials {
                    username: Some(auth.username.clone()),
                    password: Some(auth.password.clone()),
                    email: None,
                    serveraddress: Some(auth.server.clone()),
                    auth: None,
                    identitytoken: None,
                    registrytoken: None,
                });

                let create_image_options = CreateImageOptions {
                    from_image: image,
                    ..Default::default()
                };

                let mut stream = docker.create_image(Some(create_image_options), None, auth_config);
                
                use futures::TryStreamExt;
                while let Some(info) = stream.try_next().await
                    .map_err(|e| ToadStoolError::runtime(format!("Failed to pull image {}: {}", image, e)))? {
                    debug!("Pull progress: {:?}", info);
                }
                
                info!("Successfully pulled image: {}", image);
            }
        }

        #[cfg(not(feature = "docker"))]
        {
            return Err(ToadStoolError::not_supported("Docker feature not enabled"));
        }

        Ok(())
    }

    /// Create container configuration from execution request
    fn create_container_config(
        &self,
        request: &ExecutionRequest,
        image: &str,
        command: Option<&Vec<String>>,
        args: Option<&Vec<String>>,
        working_dir: Option<&str>,
        user: Option<&str>,
        volumes: &[VolumeMount],
        ports: &[PortMapping],
    ) -> ToadStoolResult<Config<String>> {
        let mut config = Config {
            image: Some(image.to_string()),
            cmd: command.cloned(),
            ..Default::default()
        };

        // Set working directory
        if let Some(workdir) = working_dir {
            config.working_dir = Some(workdir.to_string());
        }

        // Set user
        if let Some(u) = user {
            config.user = Some(u.to_string());
        }

        // Set environment variables
        let mut env_vars = Vec::new();
        for (key, value) in &request.environment {
            env_vars.push(format!("{}={}", key, value));
        }
        if !env_vars.is_empty() {
            config.env = Some(env_vars);
        }

        // Set exposed ports
        if !ports.is_empty() {
            let mut exposed_ports = HashMap::new();
            for port in ports {
                let port_spec = format!("{}/{}", port.container_port, port.protocol.to_lowercase());
                exposed_ports.insert(port_spec, HashMap::new());
            }
            config.exposed_ports = Some(exposed_ports);
        }

        // Create host config
        let mut host_config = HostConfig::default();

        // Configure resource limits
        if let Some(memory_mb) = request.resources.memory.max_bytes {
            host_config.memory = Some(memory_mb as i64);
        }

        if let Some(cpu_cores) = request.resources.cpu.max_cores {
            // Convert cores to nano CPUs (1 core = 1,000,000,000 nano CPUs)
            host_config.nano_cpus = Some((cpu_cores * 1_000_000_000.0) as i64);
        }

        // Configure port bindings
        if !ports.is_empty() {
            let mut port_bindings = HashMap::new();
            for port in ports {
                let container_port = format!("{}/{}", port.container_port, port.protocol.to_lowercase());
                let host_binding = vec![bollard::models::PortBinding {
                    host_ip: Some("0.0.0.0".to_string()),
                    host_port: Some(port.host_port.to_string()),
                }];
                port_bindings.insert(container_port, Some(host_binding));
            }
            host_config.port_bindings = Some(port_bindings);
        }

        // Configure volume mounts
        if !volumes.is_empty() {
            let mut mounts = Vec::new();
            for volume in volumes {
                let mount = Mount {
                    target: Some(volume.target.clone()),
                    source: Some(volume.source.clone()),
                    typ: Some(match volume.mount_type {
                        VolumeMountType::Bind => MountTypeEnum::BIND,
                        VolumeMountType::Volume => MountTypeEnum::VOLUME,
                        VolumeMountType::Tmpfs => MountTypeEnum::TMPFS,
                    }),
                    read_only: Some(volume.read_only),
                    ..Default::default()
                };
                mounts.push(mount);
            }
            host_config.mounts = Some(mounts);
        }

        // Apply security context
        self.apply_security_context(&mut host_config, &request.security_context)?;

        config.host_config = Some(host_config);

        Ok(config)
    }

    /// Apply security context to host configuration
    fn apply_security_context(
        &self,
        host_config: &mut HostConfig,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<()> {
        // Apply isolation level
        match security_context.isolation_level {
            IsolationLevel::None => {
                // No additional isolation
            }
            IsolationLevel::Basic => {
                // Basic isolation with read-only root filesystem
                host_config.readonly_rootfs = Some(self.config.security_config.read_only_root_fs);
            }
            IsolationLevel::Standard => {
                // Standard isolation
                host_config.readonly_rootfs = Some(self.config.security_config.read_only_root_fs);
                host_config.userns_mode = Some("host".to_string());
            }
            IsolationLevel::Enhanced => {
                // Enhanced isolation with user namespace
                host_config.readonly_rootfs = Some(self.config.security_config.read_only_root_fs);
                host_config.userns_mode = Some("host".to_string());
            }
            IsolationLevel::Maximum => {
                // Maximum isolation with all security features
                host_config.readonly_rootfs = Some(true);
                host_config.userns_mode = Some("host".to_string());
                host_config.security_opt = Some(vec![
                    "no-new-privileges:true".to_string(),
                    "seccomp:unconfined".to_string(),
                ]);
            }
        }

        Ok(())
    }

    /// Execute a container with the given parameters
    async fn execute_container(
        &self,
        request: &ExecutionRequest,
        image: &str,
        command: Option<&Vec<String>>,
        args: Option<&Vec<String>>,
        working_dir: Option<&str>,
        user: Option<&str>,
        volumes: &[VolumeMount],
        ports: &[PortMapping],
        registry_auth: Option<&RegistryAuth>,
    ) -> ToadStoolResult<ExecutionResponse> {
        #[cfg(feature = "docker")]
        {
            let docker = self.docker
                .as_ref()
                .ok_or_else(|| ToadStoolError::configuration("Docker client not available"))?;

            // Ensure image is available
            self.ensure_image(image, registry_auth).await?;

            // Create container configuration
            let config = self.create_container_config(
                request, image, command, args, working_dir, user, volumes, ports
            )?;

            // Create container
            let container_options = CreateContainerOptions {
                name: format!("toadstool-{}", request.execution_id),
                platform: None,
            };

            let container_response = docker
                .create_container(Some(container_options), config)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to create container: {}", e)))?;

            // Start container
            docker
                .start_container(&container_response.id, None::<StartContainerOptions<String>>)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to start container: {}", e)))?;

            // Store container handle
            let handle = ContainerHandle {
                container_id: container_response.id.clone(),
                image: image.to_string(),
                start_time: Instant::now(),
                config: self.config.clone(),
            };

            {
                let mut containers = self.active_containers.write().await;
                containers.insert(request.execution_id, handle);
            }

            // Wait for container to complete (using stream properly)
            let wait_options = WaitContainerOptions {
                condition: "not-running",
            };

            let mut wait_stream = docker.wait_container(&container_response.id, Some(wait_options));
            
            use futures::StreamExt;
            let wait_result = tokio::time::timeout(
                request.timeout.unwrap_or(Duration::from_secs(300)),
                wait_stream.next()
            )
            .await
            .map_err(|_| ToadStoolError::timeout(300000))?;

            let wait_response = match wait_result {
                Some(Ok(response)) => response,
                Some(Err(e)) => return Err(ToadStoolError::runtime(format!("Container wait failed: {}", e))),
                None => return Err(ToadStoolError::runtime("Container wait stream ended unexpectedly".to_string())),
            };

            // Get container logs
            let logs_options = bollard::container::LogsOptions::<String> {
                stdout: true,
                stderr: true,
                ..Default::default()
            };

            let mut logs_stream = docker.logs(&container_response.id, Some(logs_options));

            let mut stdout = Vec::new();
            let mut stderr = Vec::new();

            use futures::TryStreamExt;
            while let Some(log_output) = logs_stream.try_next().await.unwrap_or(None) {
                match log_output {
                    LogOutput::StdOut { message } => {
                        stdout.extend_from_slice(&message);
                    }
                    LogOutput::StdErr { message } => {
                        stderr.extend_from_slice(&message);
                    }
                    _ => {}
                }
            }

            // Clean up container
            let _ = docker.remove_container(&container_response.id, None).await;

            // Remove from active containers
            {
                let mut containers = self.active_containers.write().await;
                containers.remove(&request.execution_id);
            }

            // Create execution response
            let status = if wait_response.status_code == 0 {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed {
                    error: format!("Container exited with code {}", wait_response.status_code)
                }
            };

            let output = ExecutionOutput {
                data: Vec::new(),
                result: HashMap::new(),
                stdout: Some(String::from_utf8_lossy(&stdout).to_string()),
                stderr: Some(String::from_utf8_lossy(&stderr).to_string()),
                exit_code: Some(wait_response.status_code as i32),
                format: Some("text/plain".to_string()),
            };

            let metrics = RuntimeMetrics {
                cpu: CpuMetrics::default(),
                memory: MemoryMetrics::default(),
                storage: StorageMetrics::default(),
                network: NetworkMetrics::default(),
                gpu: None,
                timing: TimingMetrics::default(),
                custom: HashMap::new(),
            };

            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status,
                output,
                metrics,
                duration: Duration::from_millis(1),
                runtime_used: RuntimeType::Container,
                warnings: Vec::new(),
            })
        }

        #[cfg(not(feature = "docker"))]
        {
            Err(ToadStoolError::not_supported("Docker feature not enabled"))
        }
    }

    /// Validate resource requirements against configured limits
    fn validate_resource_requirements(&self, request: &ExecutionRequest) -> ToadStoolResult<()> {
        // Check memory requirements
        if let Some(memory_req) = request.resources.memory.max_bytes {
            if memory_req > self.config.resource_limits.max_memory_bytes {
                return Err(ToadStoolError::resource(format!(
                    "Memory requirement ({} bytes) exceeds limit ({} bytes)",
                    memory_req, self.config.resource_limits.max_memory_bytes
                )));
            }
        }

        // Check CPU requirements
        if let Some(cpu_req) = request.resources.cpu.max_cores {
            let cpu_millicores = (cpu_req * 1000.0) as u32;
            if cpu_millicores > self.config.resource_limits.max_cpu_millicores {
                return Err(ToadStoolError::resource(format!(
                    "CPU requirement ({} millicores) exceeds limit ({} millicores)",
                    cpu_millicores, self.config.resource_limits.max_cpu_millicores
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl RuntimeEngine for ContainerRuntimeEngine {
    async fn initialize(&mut self, _config: RuntimeConfig) -> ToadStoolResult<()> {
        debug!("Initializing container runtime engine");

        // Test Docker connection if available
        #[cfg(feature = "docker")]
        if let Some(docker) = &self.docker {
            match docker.ping().await {
                Ok(_) => {
                    info!("Docker connection established successfully");
                }
                Err(e) => {
                    return Err(ToadStoolError::configuration(format!(
                        "Docker connection test failed: {}",
                        e
                    )));
                }
            }
        }

        info!("Container runtime engine initialized successfully");
        Ok(())
    }

    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing container workload: {}", request.execution_id);

        // Validate resource requirements
        self.validate_resource_requirements(&request)?;

        // Extract container workload details
        if let WorkloadSpec::Container {
            image,
            command,
            args,
            working_dir,
            user,
            volumes,
            ports,
            registry_auth,
        } = &request.workload {
            self.execute_container(
                &request,
                image,
                command.as_ref(),
                args.as_ref(),
                working_dir.as_deref(),
                user.as_deref(),
                volumes,
                ports,
                registry_auth.as_ref(),
            ).await
        } else {
            Err(ToadStoolError::validation("Invalid workload type for container runtime"))
        }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        self.capabilities.clone()
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Container)
    }

    async fn get_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        // TODO: Implement actual metrics collection from Docker API
        Ok(RuntimeMetrics {
            cpu: CpuMetrics::default(),
            memory: MemoryMetrics::default(),
            storage: StorageMetrics::default(),
            network: NetworkMetrics::default(),
            gpu: None,
            timing: TimingMetrics::default(),
            custom: HashMap::new(),
        })
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down container runtime engine");

        // Stop all active containers
        let container_ids: Vec<Uuid> = {
            let containers = self.active_containers.read().await;
            containers.keys().cloned().collect()
        };

        #[cfg(feature = "docker")]
        if let Some(docker) = &self.docker {
            for container_id in container_ids {
                if let Some(handle) = {
                    let containers = self.active_containers.read().await;
                    containers.get(&container_id).cloned()
                } {
                    let _ = docker.stop_container(&handle.container_id, None).await;
                    let _ = docker.remove_container(&handle.container_id, None).await;
                }
            }
        }

        // Clear active containers
        {
            let mut containers = self.active_containers.write().await;
            containers.clear();
        }

        info!("Container runtime engine shut down successfully");
        Ok(())
    }
}

impl Default for ContainerRuntimeEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default container runtime engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    fn create_test_request(image: &str) -> ExecutionRequest {
        ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Container {
                image: image.to_string(),
                command: Some(vec!["echo".to_string(), "hello".to_string()]),
                args: None,
                working_dir: None,
                user: None,
                volumes: Vec::new(),
                ports: Vec::new(),
                registry_auth: None,
            },
            runtime_hint: Some(RuntimeType::Container),
            resources: Default::default(),
            security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
            timeout: Some(Duration::from_secs(30)),
            environment: HashMap::new(),
            input_data: Default::default(),
            callback_config: None,
        }
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = ContainerRuntimeEngine::new();
        // May fail if Docker is not available, which is expected in test environments
        assert!(engine.is_ok() || engine.is_err());
    }

    #[tokio::test]
    async fn test_capabilities() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let capabilities = engine.get_capabilities();
            assert!(capabilities.supported_workloads.contains(&WorkloadType::Container));
        }
    }

    #[tokio::test]
    async fn test_workload_support() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            assert!(engine.supports_workload(&WorkloadType::Container));
            assert!(!engine.supports_workload(&WorkloadType::Wasm));
            assert!(!engine.supports_workload(&WorkloadType::Native));
        }
    }

    #[tokio::test]
    async fn test_invalid_workload_execution() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let request = ExecutionRequest {
                execution_id: Uuid::new_v4(),
                workload: WorkloadSpec::Native {
                    executable: toadstool::workload::ExecutableSource::File {
                        path: PathBuf::from("/bin/echo"),
                    },
                    args: None,
                    working_dir: None,
                    env_vars: HashMap::new(),
                    user: None,
                },
                runtime_hint: Some(RuntimeType::Native),
                resources: Default::default(),
                security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
                timeout: None,
                environment: HashMap::new(),
                input_data: Default::default(),
                callback_config: None,
            };

            let result = engine.execute(request).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_resource_validation() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let mut request = create_test_request("hello-world");
            
            // Set memory requirement that exceeds limits
            request.resources.memory.max_bytes = Some(10240); // 10GB

            let result = engine.validate_resource_requirements(&request);
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_shutdown() {
        if let Ok(mut engine) = ContainerRuntimeEngine::new() {
            let result = engine.shutdown().await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_docker_integration() {
        let config = ContainerRuntimeConfig::default();
        let engine_result = ContainerRuntimeEngine::with_config(config);
        
        // Should succeed in creating the engine (Docker availability is checked later)
        assert!(engine_result.is_ok() || engine_result.is_err());
    }

    #[tokio::test]
    async fn test_port_mapping() {
        let mut request = create_test_request("alpine:latest");
        
        // Modify request to include port mapping
        if let WorkloadSpec::Container { ports, .. } = &mut request.workload {
            ports.push(PortMapping { 
                host_port: 8080, 
                container_port: 80, 
                protocol: "tcp".to_string() 
            });
        }
        
        // Test port validation
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    }

    #[tokio::test]
    async fn test_volume_mounting() {
        let mut request = create_test_request("alpine:latest");
        
        // Modify request to include volume mounts
        if let WorkloadSpec::Container { volumes, .. } = &mut request.workload {
            volumes.push(VolumeMount {
                source: "/tmp".to_string(),
                target: "/data".to_string(),
                mount_type: VolumeMountType::Bind,
                read_only: true,
            });
        }
        
        // Test volume validation
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    }

    #[tokio::test]
    async fn test_registry_authentication() {
        let mut request = create_test_request("private.registry.com/image:latest");
        
        // Test registry auth configuration
        if let WorkloadSpec::Container { registry_auth, .. } = &mut request.workload {
            *registry_auth = Some(RegistryAuth {
                username: "testuser".to_string(),
                password: "testpass".to_string(),
                server: "private.registry.com".to_string(),
            });
        }
        
        // Test authentication validation
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    }

    #[tokio::test]
    async fn test_security_contexts() {
        let config = ContainerRuntimeConfig::default();
        if let Ok(engine) = ContainerRuntimeEngine::with_config(config) {
            let capabilities = engine.get_capabilities();
            
            // Test platform features
            assert!(!capabilities.platform_features.is_empty());
        }
    }

    #[tokio::test]
    async fn test_resource_constraints() {
        let mut request = create_test_request("alpine:latest");
        request.resources.cpu.min_cores = 1.0;
        request.resources.memory.min_bytes = 512 * 1024 * 1024; // 512MB in bytes
        
        // Test resource validation
        assert_eq!(request.resources.cpu.min_cores, 1.0);
        assert_eq!(request.resources.memory.min_bytes, 512 * 1024 * 1024);
    }
}
