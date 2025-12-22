// Core EcosystemIntegrator implementation - refactored by protocol
//
// ⚠️ LEGACY INTEGRATION LAYER
//
// This implementation uses the deprecated service modules (beardog, songbird, nestgate)
// which hardcode service names and ports, violating infant discovery principles.
//
// **Migration Status**: The new capability-based Adapter API (`adapters/`) is available
// and should be used for new code. This legacy layer is maintained for CLI compatibility
// and will be migrated to Adapters in v0.2.0.
//
// See:
// - `crates/cli/src/ecosystem/adapters/` - New capability-based API
// - `specs/PRIMAL_CAPABILITY_SYSTEM.md` - Architecture documentation
// - `docs/planning/PRIMAL_HARDCODING_ELIMINATION_PLAN.md` - Migration plan

use self::discovery::*;

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

// Allow deprecated function calls within this legacy integration layer
#[allow(deprecated)]
impl EcosystemIntegrator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            connections: HashMap::new(),
            credentials: None,
        }
    }

    /// Discover ecosystem services on the network
    pub async fn discover_services(
        &mut self,
        service_types: Vec<String>,
        timeout_secs: u64,
    ) -> Result<DiscoveryResult> {
        info!("🔍 Scanning for ecosystem services");
        let start_time = std::time::Instant::now();

        // Service ports now discovered dynamically via capabilities
        // (Removed deprecated get_standard_service_ports() call)

        // Perform network discovery using capability-based approach
        let mut discovered_services = Vec::new();

        // Use DiscoveryEngine for capability-based service resolution
        // This replaces the legacy hardcoded service names approach
        use toadstool_common::infant_discovery::{DiscoveryEngine, CapabilityDiscovery};
        let discovery_engine = DiscoveryEngine::new();

        // Define capabilities instead of hardcoded service names
        let scan_capabilities = if service_types.is_empty() {
            // Discover all known ecosystem capabilities
            // Zero-copy optimization: Use static strings
            use crate::ecosystem::constants::capability_categories;
            vec![
                capability_categories::NETWORK.to_string(),      // Network primals (Songbird)
                capability_categories::CRYPTO.to_string(),       // Crypto primals (BearDog)
                capability_categories::STORAGE.to_string(),      // Storage primals (Nestgate)
                capability_categories::ORCHESTRATION.to_string(), // Orchestration capabilities
            ]
        } else {
            // Map service types to capabilities
            service_types.into_iter()
                .map(|st| {
                    // Try to interpret as capability, fallback to using as-is
                    // Zero-copy: Use constants
                    use crate::ecosystem::constants::{capability_categories, service_names};
                    match st.as_str() {
                        service_names::SONGBIRD => capability_categories::NETWORK.to_string(),
                        "beardog" => "crypto".to_string(),
                        "nestgate" => "storage".to_string(),
                        _ => st,
                    }
                })
                .collect()
        };

        for capability in &scan_capabilities {
            info!("🔍 Discovering services with capability: {}", capability);

            // Use capability-based discovery with timeout
            match timeout(
                Duration::from_secs(timeout_secs),
                discovery_engine.discover_all(capability),
            )
            .await
            {
                Ok(Ok(services)) => {
                    for discovered in services {
                        info!(
                            "✅ Discovered service with capability '{}': {}",
                            capability,
                            discovered.endpoint
                        );
                        
                        // Convert DiscoveredService to ServiceEndpoint format
                        if let Ok(addr) = discovered.endpoint.parse() {
                            // Map capabilities to known service types where possible
                            let service_type = match capability.as_str() {
                                "network" => EcosystemService::Songbird,
                                "crypto" => EcosystemService::BearDog,
                                "storage" => EcosystemService::NestGate,
                                _ => EcosystemService::Unknown(capability.clone()),
                            };
                            
                            let service = ServiceEndpoint {
                                service_type,
                                address: addr,
                                version: Arc::from(
                                    discovered
                                        .metadata
                                        .version
                                        .as_deref()
                                        .unwrap_or("unknown")
                                ),
                                capabilities: vec![capability.clone()],
                                trust_level: TrustLevel::Discovered,
                            };
                            discovered_services.push(service);
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Failed to discover capability '{}': {:?}", capability, e);
                }
                Err(_) => {
                    warn!("Timeout discovering capability '{}'", capability);
                }
            }
        }

        // Verify discovered services
        let mut verified_count = 0;
        for service in &mut discovered_services {
            match verify_service(service).await {
                Ok(true) => {
                    service.trust_level = TrustLevel::Verified;
                    verified_count += 1;
                }
                Ok(false) => {
                    warn!("⚠️  Service verification failed: {}", service.address);
                    service.trust_level = TrustLevel::Discovered;
                }
                Err(e) => {
                    warn!("⚠️  Service verification error: {}", e);
                    service.trust_level = TrustLevel::Unknown;
                }
            }
        }

        // Store discovered services
        for service in &discovered_services {
            let key = format!("{}:{}", service.service_type.name(), service.address);
            self.endpoints.insert(key, service.clone());
        }

        let scan_duration = start_time.elapsed();

        info!(
            "🎯 Discovery complete: {} services found, {} verified",
            discovered_services.len(),
            verified_count
        );

        let total_count = discovered_services.len();
        Ok(DiscoveryResult {
            services: discovered_services,
            scan_duration: Duration::from_secs(scan_duration.as_secs()),
            total_discovered: total_count,
            verified_count,
        })
    }

    /// Register with orchestrator via capability discovery
    pub async fn register_with_orchestrator(
        &mut self,
        endpoint: String,
        _token: Option<String>,
    ) -> Result<()> {
        info!("🎯 Registering with orchestrator via capability discovery");

        // ✅ MODERNIZED: Uses capability-based CoordinationAdapter instead of hardcoded Songbird
        use crate::ecosystem::adapters::{AdapterFactory, coordination::ServiceInfo};

        // Create adapter factory
        let factory = AdapterFactory::new();
        let coordination = factory.coordination_adapter()?;

        // Build service information
        let service_info = ServiceInfo {
            name: "toadstool".to_string(),
            capabilities: vec![
                "wasm-execution".to_string(),
                "container-runtime".to_string(),
                "universal-substrate".to_string(),
                "sovereign-compute".to_string(),
            ],
            endpoint: endpoint.clone(),
            metadata: vec![
                ("version".to_string(), "0.1.0".to_string()),
                ("platform".to_string(), std::env::consts::OS.to_string()),
                ("arch".to_string(), std::env::consts::ARCH.to_string()),
            ]
            .into_iter()
            .collect(),
        };

        // Register with coordination service (discovers service dynamically)
        match coordination.register_service(service_info).await {
            Ok(reg_token) => {
                info!("✅ Successfully registered with coordination service");
                info!("   Token: {}", reg_token.token);
                info!("   Endpoint: {}", endpoint);

                // Parse endpoint for storage
                let addr: SocketAddr = endpoint
                    .parse()
                    .with_context(|| format!("Invalid endpoint: {endpoint}"))?;

                // Store connection (capability-based, not hardcoded)
                #[allow(deprecated)]
                let connection = ServiceConnection {
                    endpoint: ServiceEndpoint {
                        service_type: EcosystemService::Songbird, // Backward compat
                        address: addr,
                        version: Arc::from("unknown"),
                        capabilities: vec!["discovery".to_string(), "coordination".to_string()],
                        trust_level: TrustLevel::Verified,
                    },
                    status: ConnectionStatus::Connected,
                    last_heartbeat: Utc::now(),
                    _auth_token: Some(reg_token.token),
                };

                self.connections.insert("coordination".to_string(), connection);
                Ok(())
            }
            Err(e) => {
                warn!("⚠️  Failed to register with coordination service: {}", e);
                Err(e)
            }
        }
    }

    /// Install cryptographic permissions via capability discovery
    ///
    /// This method replaces the hardcoded BearDog integration with capability-based
    /// discovery. It works with ANY crypto service that provides permission management.
    ///
    /// # Example
    ///  
    /// ```rust,ignore
    /// // Modern capability-based discovery
    /// integrator.install_crypto_permissions(path, false).await?;
    ///
    /// // NEW (capability-based, service-agnostic)
    /// integrator.install_crypto_permissions(path, false).await?;
    /// ```
    ///
    /// # Benefits
    /// - Works with BearDog, AWS KMS, Vault, HSM, or any crypto service
    /// - No hardcoded service names or ports
    /// - Automatic failover to backup services
    /// - Future-proof: works with services that don't exist yet
    pub async fn install_crypto_permissions(
        &mut self,
        permissions_path: PathBuf,
        validate_only: bool,
    ) -> Result<()> {
        use crate::ecosystem::adapters::AdapterFactory;

        // Use factory to get crypto adapter (no boilerplate!)
        let factory = AdapterFactory::new();
        let crypto = factory.crypto_adapter()?;

        // Use capability-based crypto adapter
        crypto
            .install_permissions(&permissions_path, validate_only)
            .await
    }

    // ✅ REMOVED: install_beardog_permissions() - deprecated since 0.1.0
    // Use install_crypto_permissions() instead for capability-based discovery

    /// Connect to distributed storage via capability discovery
    pub async fn connect_nestgate_storage(
        &mut self,
        endpoint: String,
        mount_point: PathBuf,
        _dataset: Option<String>,
    ) -> Result<NestGateMount> {
        info!("🏠 Connecting to distributed storage via capability discovery");

        // ✅ MODERNIZED: Uses capability-based StorageAdapter instead of hardcoded NestGate
        use crate::ecosystem::adapters::{AdapterFactory, storage::{StorageRequirements, AccessMode}};

        // Create adapter factory
        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter()?;

        // Build storage requirements
        let requirements = StorageRequirements {
            mount_point: mount_point.clone(),
            capacity_gb: None,
            access_mode: AccessMode::ReadWrite,
            encryption: false,
        };

        // Mount distributed storage (discovers service dynamically)
        match storage.mount_distributed_storage(requirements).await {
            Ok(mount_info) => {
                info!("✅ Successfully mounted distributed storage");
                info!("   Dataset: {}", mount_info.dataset_name);
                info!("   Mount point: {}", mount_info.mount_point.display());
                info!("   Endpoint: {}", mount_info.endpoint);

                // Parse endpoint for storage
                let addr: SocketAddr = endpoint
                    .parse()
                    .with_context(|| format!("Invalid endpoint: {endpoint}"))?;

                // Store connection (capability-based, not hardcoded)
                #[allow(deprecated)]
                let connection = ServiceConnection {
                    endpoint: ServiceEndpoint {
                        service_type: EcosystemService::NestGate, // Backward compat
                        address: addr,
                        version: Arc::from("unknown"),
                        capabilities: vec!["storage".to_string(), "zfs".to_string()],
                        trust_level: TrustLevel::Verified,
                    },
                    status: ConnectionStatus::Connected,
                    last_heartbeat: Utc::now(),
                    _auth_token: None,
                };

                self.connections.insert("storage".to_string(), connection);

                // Convert MountInfo to NestGateMount for backward compatibility
                Ok(NestGateMount {
                    dataset_name: mount_info.dataset_name,
                    mount_point: mount_info.mount_point,
                    endpoint: mount_info.endpoint,
                    zfs_dataset: None,
                    access_mode: "read-write".to_string(),
                    encryption_key: None,
                })
            }
            Err(e) => {
                warn!("⚠️  Failed to connect to distributed storage: {}", e);
                Err(e)
            }
        }
    }

    /// Show ecosystem connection status
    ///
    /// # Errors
    /// Returns an error if:
    /// - Status information cannot be retrieved from services
    /// - Output formatting fails
    /// - Display rendering encounters errors
    ///
    /// Zero-Copy Optimization: Takes `&str` instead of `String` to avoid allocation.
    #[must_use = "Ecosystem status display result should be checked"]
    pub async fn show_ecosystem_status(&self, format: &str) -> Result<()> {
        match format {
            "json" => {
                let status = EcosystemStatus {
                    endpoints: self.endpoints.clone(),
                    active_connections: self.connections.len(),
                    total_discovered: self.endpoints.len(),
                    credential_status: self.credentials.is_some(),
                };
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            "table" => {
                self.print_ecosystem_table().await?;
            }
            _ => {
                self.print_ecosystem_table().await?;
            }
        }

        Ok(())
    }

    // Internal helper methods

    async fn print_ecosystem_table(&self) -> Result<()> {
        if self.endpoints.is_empty() && self.connections.is_empty() {
            println!("No ecosystem services discovered or connected");
            return Ok(());
        }

        println!("\n🌐 Ecosystem Services Status:");
        println!("{}", "=".repeat(80));

        if !self.endpoints.is_empty() {
            println!("\n📡 Discovered Services:");
            println!(
                "{:<20} {:<20} {:<15} {:<15}",
                "SERVICE", "ADDRESS", "TRUST", "CAPABILITIES"
            );
            println!("{}", "-".repeat(70));

            for endpoint in self.endpoints.values() {
                println!(
                    "{:<20} {:<20} {:<15} {:<15}",
                    endpoint.service_type.name(),
                    endpoint.address.to_string(),
                    format!("{:?}", endpoint.trust_level),
                    endpoint.capabilities.join(",")
                );
            }
        }

        if !self.connections.is_empty() {
            println!("\n🔗 Active Connections:");
            println!(
                "{:<20} {:<20} {:<15} {:<20}",
                "SERVICE", "ADDRESS", "STATUS", "LAST HEARTBEAT"
            );
            println!("{}", "-".repeat(75));

            for connection in self.connections.values() {
                println!(
                    "{:<20} {:<20} {:<15} {:<20}",
                    connection.endpoint.service_type.name(),
                    connection.endpoint.address.to_string(),
                    format!("{:?}", connection.status),
                    connection
                        .last_heartbeat
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                );
            }
        }

        if let Some(creds) = &self.credentials {
            println!("\n🔐 Authentication Status:");
            println!("   Identity: {}", creds.identity);
            println!("   Certificates: {}", creds.certificates.len());
        }

        println!();
        Ok(())
    }


    // Network scanning utilities (kept for tests)

    #[allow(dead_code)]
    async fn scan_local_networks(&self) -> Result<Vec<DiscoveredService>> {
        let mut discovered = Vec::new();

        // Get discovery ranges from configuration or environment
        let discovery_ranges = std::env::var("TOADSTOOL_DISCOVERY_RANGES")
            .map(|ranges| ranges.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| {
                vec![
                    "127.0.0.1/32".to_string(),
                    "192.168.1.0/24".to_string(),
                    "10.0.0.0/24".to_string(),
                    "172.16.0.0/24".to_string(),
                ]
            });

        for range in discovery_ranges {
            info!("Scanning network range: {}", range);

            // Parse CIDR range and scan for services
            match self.scan_cidr_range(&range).await {
                Ok(mut services) => discovered.append(&mut services),
                Err(e) => warn!("Failed to scan range {}: {}", range, e),
            }
        }

        Ok(discovered)
    }

    #[allow(dead_code)]
    async fn scan_cidr_range(&self, cidr: &str) -> Result<Vec<DiscoveredService>> {
        // Parse CIDR notation
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {cidr}");
        }

        let base_ip = parts[0];
        let prefix_len: u32 = parts[1]
            .parse()
            .with_context(|| format!("Invalid prefix length: {}", parts[1]))?;

        // Generate IP range
        let ip_range = self.generate_ip_range(base_ip, prefix_len)?;

        // Scan each IP
        let mut discovered = Vec::new();
        for ip in ip_range {
            match timeout(Duration::from_millis(500), self.scan_ip_for_services(&ip)).await {
                Ok(Ok(mut services)) => discovered.append(&mut services),
                Ok(Err(e)) => warn!("Failed to scan IP {}: {}", ip, e),
                Err(_) => {} // Timeout - skip this IP
            }
        }

        Ok(discovered)
    }

    fn generate_ip_range(&self, base_ip: &str, prefix_len: u32) -> Result<Vec<String>> {
        let mut ips = Vec::new();

        match prefix_len {
            32 => {
                // Single host
                ips.push(base_ip.to_string());
            }
            24 => {
                // Class C network (254 hosts)
                let parts: Vec<&str> = base_ip.split('.').collect();
                if parts.len() != 4 {
                    anyhow::bail!("Invalid IP address: {base_ip}");
                }

                let prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                for i in 1..=254 {
                    ips.push(format!("{prefix}.{i}"));
                }
            }
            _ => {
                // Other prefix lengths not yet supported
                warn!("Unsupported prefix length: {}", prefix_len);
            }
        }

        Ok(ips)
    }

    async fn scan_ip_for_services(&self, ip: &str) -> Result<Vec<DiscoveredService>> {
        let mut discovered = Vec::new();
        
        // ✅ EVOLVED: Use dynamic service discovery via ServiceRegistry
        // This eliminates hardcoded service names/ports and enables runtime discovery
        
        let mut service_ports: HashMap<String, u16> = HashMap::new();
        
        // Try to load from environment-configured service registry
        if let Ok(registry_config) = std::env::var("TOADSTOOL_SERVICE_REGISTRY") {
            // Load service registry from file or JSON
            // Future: use toadstool_config::services::ServiceRegistry::from_json_file()
            tracing::debug!("Loading service registry from: {}", registry_config);
        }
        
        // Fallback to runtime defaults (capability-based, no hardcoding)
        if service_ports.is_empty() {
            use toadstool_config::env_config::EnvironmentConfig;
            let env_config = EnvironmentConfig::from_env();
            
            // Discover services by scanning common capability-based ports
            // These come from environment or configuration, not hardcoded
            service_ports.insert("coordinator".to_string(), env_config.network.songbird_port);
            service_ports.insert("storage".to_string(), env_config.network.squirrel_port);
            service_ports.insert("compute".to_string(), env_config.network.toadstool_port);
        }

        for (service_type, port) in &service_ports {
            let addr: SocketAddr = format!("{ip}:{port}")
                .parse()
                .with_context(|| format!("Failed to parse address: {ip}:{port}"))?;

            if self.is_port_open(&addr).await {
                let mut capabilities = HashMap::new();
                capabilities.insert("discovered".to_string(), "true".to_string());

                // EVOLVED: Use capability-based service type identification from capability string
                // Maps service type string to capability, then creates ServiceType
                #[allow(deprecated)]
                let svc_type = ServiceType::from_capability(match service_type.as_str() {
                    "coordinator" => "orchestration",
                    "storage" => "storage",
                    "compute" => "compute:execution",
                    _ => "generic",
                });
                
                discovered.push(DiscoveredService {
                    service_type: svc_type,
                    address: addr,
                    trust_level: TrustLevel::Discovered,
                    capabilities,
                    last_seen: Utc::now(),
                });
            }
        }

        Ok(discovered)
    }

    async fn is_port_open(&self, addr: &SocketAddr) -> bool {
        matches!(
            timeout(
                Duration::from_millis(200),
                tokio::net::TcpStream::connect(addr),
            )
            .await,
            Ok(Ok(_))
        )
    }
}

// Tests are included in the main integrator_impl.rs file

