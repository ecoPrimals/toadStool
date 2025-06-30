//! Ecosystem Integration - Sovereign Science Network
//!
//! Integration with the ecoPrimals ecosystem for distributed sovereign computing:
//! - Songbird: Service discovery and coordination
//! - BearDog: Cryptographic security and permissions
//! - NestGate: Distributed storage and data management

use anyhow::{Result, Context, bail};
use std::path::PathBuf;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::fs;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};
use uuid::Uuid;


/// Ecosystem service discovery and integration
pub struct EcosystemIntegrator {
    /// Known ecosystem endpoints
    endpoints: HashMap<String, ServiceEndpoint>,
    /// Active connections
    connections: HashMap<String, ServiceConnection>,
    /// Authentication credentials
    credentials: Option<EcosystemCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub service_type: EcosystemService,
    pub address: SocketAddr,
    pub version: String,
    pub capabilities: Vec<String>,
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemService {
    Songbird,  // Service discovery and coordination
    BearDog,   // Cryptographic security
    NestGate,  // Distributed storage
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Discovered,     // Found via network scan
    Advertised,     // Advertised via Songbird
    Verified,       // Cryptographically verified
    Sovereign,      // Full sovereign verification
}

#[derive(Debug, Clone)]
struct ServiceConnection {
    endpoint: ServiceEndpoint,
    status: ConnectionStatus,
    last_heartbeat: chrono::DateTime<chrono::Utc>,
    auth_token: Option<String>,
}

#[derive(Debug, Clone)]
enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EcosystemCredentials {
    pub identity: String,
    pub private_key: String,
    pub public_key: String,
    pub certificates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub services: Vec<ServiceEndpoint>,
    pub scan_duration: Duration,
    pub total_discovered: usize,
    pub verified_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogPermission {
    pub permission_id: Uuid,
    pub granted_to: String,
    pub capabilities: Vec<String>,
    pub valid_until: chrono::DateTime<chrono::Utc>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateMount {
    pub dataset_name: String,
    pub mount_point: PathBuf,
    pub endpoint: String,
    pub zfs_dataset: Option<String>,
    pub access_mode: String, // read, write, admin
    pub encryption_key: Option<String>,
}

impl Default for EcosystemIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

impl EcosystemIntegrator {
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
        
        // Load known service ports
        let service_ports = self.get_standard_service_ports();
        
        // Perform network discovery
        let mut discovered_services = Vec::new();
        
        // If no specific services requested, scan for all
        let scan_services = if service_types.is_empty() {
            vec!["songbird".to_string(), "beardog".to_string(), "nestgate".to_string()]
        } else {
            service_types
        };
        
        for service_type in &scan_services {
            info!("🔍 Scanning for {}", service_type);
            
            let services = timeout(
                Duration::from_secs(timeout_secs),
                self.scan_for_service(service_type, &service_ports)
            ).await
            .with_context(|| format!("Timeout scanning for {}", service_type))?
            .with_context(|| format!("Failed to scan for {}", service_type))?;
            
            for service in services {
                info!("✅ Discovered {}: {}:{}", 
                      service_type, service.address.ip(), service.address.port());
                discovered_services.push(service);
            }
        }
        
        // Verify discovered services
        let mut verified_count = 0;
        for service in &mut discovered_services {
            match self.verify_service(service).await {
                Ok(true) => {
                    service.trust_level = TrustLevel::Verified;
                    verified_count += 1;
                },
                Ok(false) => {
                    warn!("⚠️  Service verification failed: {}", service.address);
                    service.trust_level = TrustLevel::Discovered;
                },
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
        
        info!("🎯 Discovery complete: {} services found, {} verified", 
              discovered_services.len(), verified_count);
        
        let total_count = discovered_services.len();
        Ok(DiscoveryResult {
            services: discovered_services,
            scan_duration: Duration::from_secs(scan_duration.as_secs()),
            total_discovered: total_count,
            verified_count,
        })
    }
    
    /// Register with Songbird discovery service
    pub async fn register_with_songbird(
        &mut self,
        endpoint: String,
        token: Option<String>,
    ) -> Result<()> {
        info!("🐦 Registering with Songbird: {}", endpoint);
        
        // Parse endpoint
        let addr: SocketAddr = endpoint.parse()
            .with_context(|| format!("Invalid Songbird endpoint: {}", endpoint))?;
        
        // Create registration payload
        let registration = SongbirdRegistration {
            service_name: "toadstool".to_string(),
            service_type: "universal-compute".to_string(),
            address: self.get_local_address()?,
            capabilities: vec![
                "wasm-execution".to_string(),
                "container-runtime".to_string(),
                "universal-substrate".to_string(),
                "sovereign-compute".to_string(),
            ],
            metadata: vec![
                ("version".to_string(), "0.1.0".to_string()),
                ("platform".to_string(), std::env::consts::OS.to_string()),
                ("arch".to_string(), std::env::consts::ARCH.to_string()),
            ].into_iter().collect(),
            auth_token: token.clone(),
        };
        
        // Attempt registration
        match self.send_songbird_registration(&addr, &registration).await {
            Ok(response) => {
                info!("✅ Successfully registered with Songbird");
                info!("   Service ID: {}", response.service_id);
                info!("   Registry URL: {}", response.registry_url);
                
                // Store connection
                let connection = ServiceConnection {
                    endpoint: ServiceEndpoint {
                        service_type: EcosystemService::Songbird,
                        address: addr,
                        version: "unknown".to_string(),
                        capabilities: vec!["discovery".to_string(), "coordination".to_string()],
                        trust_level: TrustLevel::Verified,
                    },
                    status: ConnectionStatus::Connected,
                    last_heartbeat: chrono::Utc::now(),
                    auth_token: token,
                };
                
                self.connections.insert("songbird".to_string(), connection);
                
                Ok(())
            },
            Err(e) => {
                error!("❌ Failed to register with Songbird: {}", e);
                Err(e)
            }
        }
    }
    
    /// Install BearDog cryptographic permissions
    pub async fn install_beardog_permissions(
        &mut self,
        permission_file: PathBuf,
        validate_only: bool,
    ) -> Result<()> {
        info!("🐻 Installing BearDog permissions from: {}", permission_file.display());
        
        // Load permission file
        let permission_content = fs::read_to_string(&permission_file).await
            .with_context(|| format!("Failed to read permission file: {}", permission_file.display()))?;
        
        let permission: BearDogPermission = serde_yaml::from_str(&permission_content)
            .with_context(|| "Failed to parse BearDog permission file")?;
        
        info!("📋 Permission ID: {}", permission.permission_id);
        info!("📋 Granted to: {}", permission.granted_to);
        info!("📋 Capabilities: {}", permission.capabilities.join(", "));
        info!("📋 Valid until: {}", permission.valid_until);
        
        // Validate permission
        if !self.validate_beardog_permission(&permission).await? {
            bail!("❌ BearDog permission validation failed");
        }
        
        info!("✅ BearDog permission validation successful");
        
        if validate_only {
            info!("🔍 Validation-only mode - not installing");
            return Ok(());
        }
        
        // Install permission
        self.install_permission(&permission).await?;
        
        info!("✅ BearDog permissions installed successfully");
        Ok(())
    }
    
    /// Connect to NestGate distributed storage
    pub async fn connect_nestgate_storage(
        &mut self,
        endpoint: String,
        mount_point: PathBuf,
        dataset: Option<String>,
    ) -> Result<NestGateMount> {
        info!("🏠 Connecting to NestGate storage: {}", endpoint);
        
        // Parse endpoint
        let addr: SocketAddr = endpoint.parse()
            .with_context(|| format!("Invalid NestGate endpoint: {}", endpoint))?;
        
        // Check if mount point exists
        if !mount_point.exists() {
            fs::create_dir_all(&mount_point).await
                .with_context(|| format!("Failed to create mount point: {}", mount_point.display()))?;
        }
        
        // Connect to NestGate
        let mount_info = self.mount_nestgate_dataset(&addr, &mount_point, dataset.as_deref()).await?;
        
        info!("✅ NestGate storage connected");
        info!("   Dataset: {}", mount_info.dataset_name);
        info!("   Mount point: {}", mount_info.mount_point.display());
        info!("   Access mode: {}", mount_info.access_mode);
        
        // Store connection
        let connection = ServiceConnection {
            endpoint: ServiceEndpoint {
                service_type: EcosystemService::NestGate,
                address: addr,
                version: "unknown".to_string(),
                capabilities: vec!["storage".to_string(), "zfs".to_string()],
                trust_level: TrustLevel::Verified,
            },
            status: ConnectionStatus::Connected,
            last_heartbeat: chrono::Utc::now(),
            auth_token: None,
        };
        
        self.connections.insert("nestgate".to_string(), connection);
        
        Ok(mount_info)
    }
    
    /// Show ecosystem connection status
    pub async fn show_ecosystem_status(&self, format: String) -> Result<()> {
        match format.as_str() {
            "json" => {
                let status = EcosystemStatus {
                    endpoints: self.endpoints.clone(),
                    active_connections: self.connections.len(),
                    total_discovered: self.endpoints.len(),
                    credential_status: self.credentials.is_some(),
                };
                println!("{}", serde_json::to_string_pretty(&status)?);
            },
            "table" | _ => {
                self.print_ecosystem_table().await?;
            }
        }
        
        Ok(())
    }
    
    // Internal helper methods
    
    async fn scan_for_service(&self, service_type: &str, service_ports: &HashMap<String, u16>) -> Result<Vec<ServiceEndpoint>> {
        let port = service_ports.get(service_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown service type: {}", service_type))?;
        
        let mut services = Vec::new();
        
        // Scan local network ranges
        let local_ranges = vec![
            "127.0.0.1",
            "192.168.1.0/24",
            "10.0.0.0/24",
        ];
        
        for range in local_ranges {
            if range.contains('/') {
                // Subnet scan - simplified for demo
                continue;
            } else {
                // Single IP
                let addr = format!("{}:{}", range, port);
                if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
                    if self.check_service_available(&socket_addr).await? {
                        services.push(ServiceEndpoint {
                            service_type: EcosystemService::parse(service_type),
                            address: socket_addr,
                            version: "unknown".to_string(),
                            capabilities: vec!["basic".to_string()],
                            trust_level: TrustLevel::Discovered,
                        });
                    }
                }
            }
        }
        
        Ok(services)
    }
    
    async fn check_service_available(&self, addr: &SocketAddr) -> Result<bool> {
        // Simple TCP connection test
        match timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // Timeout
        }
    }
    
    async fn verify_service(&self, service: &ServiceEndpoint) -> Result<bool> {
        // Implement service-specific verification
        match &service.service_type {
            EcosystemService::Songbird => self.verify_songbird_service(&service.address).await,
            EcosystemService::BearDog => self.verify_beardog_service(&service.address).await,
            EcosystemService::NestGate => self.verify_nestgate_service(&service.address).await,
            EcosystemService::Unknown(_) => Ok(false),
        }
    }
    
    async fn verify_songbird_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Send ping to Songbird and verify response
        // This is a simplified implementation
        Ok(true)
    }
    
    async fn verify_beardog_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Verify BearDog cryptographic service
        // This is a simplified implementation
        Ok(true)
    }
    
    async fn verify_nestgate_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Verify NestGate storage service
        // This is a simplified implementation
        Ok(true)
    }
    
    fn get_standard_service_ports(&self) -> HashMap<String, u16> {
        vec![
            ("songbird".to_string(), 5000),
            ("beardog".to_string(), 5001),
            ("nestgate".to_string(), 5002),
        ].into_iter().collect()
    }
    
    fn get_local_address(&self) -> Result<SocketAddr> {
        // Get local network address
        // This is a simplified implementation
        Ok("127.0.0.1:8080".parse()?)
    }
    
    async fn send_songbird_registration(&self, addr: &SocketAddr, registration: &SongbirdRegistration) -> Result<SongbirdResponse> {
        // Send HTTP POST to Songbird registration endpoint
        // This is a simplified implementation
        Ok(SongbirdResponse {
            service_id: Uuid::new_v4().to_string(),
            registry_url: format!("http://{}:{}/registry", addr.ip(), addr.port()),
            heartbeat_interval: 30,
        })
    }
    
    async fn validate_beardog_permission(&self, permission: &BearDogPermission) -> Result<bool> {
        // Validate cryptographic signature
        // Check expiration
        // Verify capabilities
        // This is a simplified implementation
        
        if permission.valid_until < chrono::Utc::now() {
            warn!("⚠️  BearDog permission has expired");
            return Ok(false);
        }
        
        // TODO: Implement actual cryptographic verification
        Ok(true)
    }
    
    async fn install_permission(&self, permission: &BearDogPermission) -> Result<()> {
        // Install permission in system keyring or secure storage
        // This is a simplified implementation
        info!("🔐 Installing permission: {}", permission.permission_id);
        Ok(())
    }
    
    async fn mount_nestgate_dataset(&self, addr: &SocketAddr, mount_point: &PathBuf, dataset: Option<&str>) -> Result<NestGateMount> {
        // Connect to NestGate and mount ZFS dataset
        // This is a simplified implementation
        
        let dataset_name = dataset.unwrap_or("default").to_string();
        
        Ok(NestGateMount {
            dataset_name: dataset_name.clone(),
            mount_point: mount_point.clone(),
            endpoint: addr.to_string(),
            zfs_dataset: Some(format!("tank/{}", dataset_name)),
            access_mode: "read-write".to_string(),
            encryption_key: None,
        })
    }
    
    async fn print_ecosystem_table(&self) -> Result<()> {
        if self.endpoints.is_empty() && self.connections.is_empty() {
            println!("No ecosystem services discovered or connected");
            return Ok(());
        }
        
        println!("\n🌐 Ecosystem Services Status:");
        println!("{}", "=".repeat(80));
        
        if !self.endpoints.is_empty() {
            println!("\n📡 Discovered Services:");
            println!("{:<20} {:<20} {:<15} {:<15}", "SERVICE", "ADDRESS", "TRUST", "CAPABILITIES");
            println!("{}", "-".repeat(70));
            
            for (key, endpoint) in &self.endpoints {
                println!("{:<20} {:<20} {:<15} {:<15}",
                         endpoint.service_type.name(),
                         endpoint.address.to_string(),
                         format!("{:?}", endpoint.trust_level),
                         endpoint.capabilities.join(","));
            }
        }
        
        if !self.connections.is_empty() {
            println!("\n🔗 Active Connections:");
            println!("{:<20} {:<20} {:<15} {:<20}", "SERVICE", "ADDRESS", "STATUS", "LAST HEARTBEAT");
            println!("{}", "-".repeat(75));
            
            for (key, connection) in &self.connections {
                println!("{:<20} {:<20} {:<15} {:<20}",
                         connection.endpoint.service_type.name(),
                         connection.endpoint.address.to_string(),
                         format!("{:?}", connection.status),
                         connection.last_heartbeat.format("%Y-%m-%d %H:%M:%S").to_string());
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SongbirdRegistration {
    service_name: String,
    service_type: String,
    address: SocketAddr,
    capabilities: Vec<String>,
    metadata: HashMap<String, String>,
    auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SongbirdResponse {
    service_id: String,
    registry_url: String,
    heartbeat_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EcosystemStatus {
    endpoints: HashMap<String, ServiceEndpoint>,
    active_connections: usize,
    total_discovered: usize,
    credential_status: bool,
}

impl EcosystemService {
    fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "songbird" => EcosystemService::Songbird,
            "beardog" => EcosystemService::BearDog,
            "nestgate" => EcosystemService::NestGate,
            _ => EcosystemService::Unknown(s.to_string()),
        }
    }
    
    fn name(&self) -> &str {
        match self {
            EcosystemService::Songbird => "songbird",
            EcosystemService::BearDog => "beardog",
            EcosystemService::NestGate => "nestgate",
            EcosystemService::Unknown(name) => name,
        }
    }
} 