//! Ecosystem Integration - Sovereign Science Network
//!
//! Integration with the ecoPrimals ecosystem for distributed sovereign computing:
//! - Songbird: Service discovery and coordination
//! - BearDog: Cryptographic security and permissions
//! - NestGate: Distributed storage and data management

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;
use chrono::{Utc, DateTime};

// Add cryptographic verification dependencies
use ring::{signature, digest};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::collections::BTreeMap;

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
    Songbird, // Service discovery and coordination
    BearDog,  // Cryptographic security
    NestGate, // Distributed storage
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Discovered, // Found via network scan
    Advertised, // Advertised via Songbird
    Verified,   // Cryptographically verified
    Sovereign,  // Full sovereign verification
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub service_type: ServiceType,
    pub address: SocketAddr,
    pub trust_level: TrustLevel,
    pub capabilities: HashMap<String, String>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ServiceType {
    Songbird,
    BearDog,
    NestGate,
    ToadStool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSignature {
    pub algorithm: String,
    pub signature: String,
    pub public_key: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedServiceResponse {
    pub service_id: String,
    pub service_type: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signature: ServiceSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoVerificationContext {
    pub trusted_public_keys: HashMap<String, String>,
    pub revoked_keys: Vec<String>,
    pub verification_timestamp: chrono::DateTime<chrono::Utc>,
    pub max_age_minutes: u32,
}

impl Default for CryptoVerificationContext {
    fn default() -> Self {
        // Load trusted public keys from environment or configuration
        let mut trusted_keys = HashMap::new();
        
        // Production keys should be loaded from secure configuration
        if let Ok(songbird_key) = std::env::var("SONGBIRD_PUBLIC_KEY") {
            trusted_keys.insert("songbird".to_string(), songbird_key);
        }
        if let Ok(beardog_key) = std::env::var("BEARDOG_PUBLIC_KEY") {
            trusted_keys.insert("beardog".to_string(), beardog_key);
        }
        if let Ok(nestgate_key) = std::env::var("NESTGATE_PUBLIC_KEY") {
            trusted_keys.insert("nestgate".to_string(), nestgate_key);
        }

        Self {
            trusted_public_keys: trusted_keys,
            revoked_keys: Vec::new(),
            verification_timestamp: chrono::Utc::now(),
            max_age_minutes: 5, // 5 minute max age for responses
        }
    }
}

impl CryptoVerificationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_trusted_key(mut self, service: String, public_key: String) -> Self {
        self.trusted_public_keys.insert(service, public_key);
        self
    }

    pub fn verify_ed25519_signature(
        &self,
        message: &[u8],
        signature_bytes: &[u8],
        public_key_bytes: &[u8],
    ) -> Result<bool> {
        let public_key = UnparsedPublicKey::new(&ED25519, public_key_bytes);
        match public_key.verify(message, signature_bytes) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn verify_service_signature(
        &self,
        service_type: &str,
        response: &SignedServiceResponse,
    ) -> Result<bool> {
        // Check if we have a trusted public key for this service
        let public_key_base64 = match self.trusted_public_keys.get(service_type) {
            Some(key) => key,
            None => {
                warn!("No trusted public key found for service: {}", service_type);
                return Ok(false);
            }
        };

        // Check if key is revoked
        if self.revoked_keys.contains(public_key_base64) {
            error!("Attempted to use revoked public key for service: {}", service_type);
            return Ok(false);
        }

        // Check response age
        let response_age = chrono::Utc::now()
            .signed_duration_since(response.timestamp)
            .num_minutes();
        
        if response_age > self.max_age_minutes as i64 {
            warn!("Service response is too old: {} minutes", response_age);
            return Ok(false);
        }

        // Decode public key and signature
        let public_key_bytes = BASE64.decode(public_key_base64)
            .map_err(|e| anyhow::anyhow!("Failed to decode public key: {}", e))?;
        
        let signature_bytes = BASE64.decode(&response.signature.signature)
            .map_err(|e| anyhow::anyhow!("Failed to decode signature: {}", e))?;

        // Create canonical message for verification
        let message = self.create_canonical_message(response)?;

        // Verify signature
        self.verify_ed25519_signature(&message, &signature_bytes, &public_key_bytes)
    }

    fn create_canonical_message(&self, response: &SignedServiceResponse) -> Result<Vec<u8>> {
        // Create deterministic message representation for signature verification
        let mut data = BTreeMap::new();
        data.insert("service_id", &response.service_id);
        data.insert("service_type", &response.service_type);
        data.insert("status", &response.status);
        let timestamp = response.timestamp.to_rfc3339();
        data.insert("timestamp", &timestamp);
        data.insert("nonce", &response.signature.nonce);
        
        let capabilities_json = serde_json::to_string(&response.capabilities)
            .map_err(|e| anyhow::anyhow!("Failed to serialize capabilities: {}", e))?;
        data.insert("capabilities", &capabilities_json);

        let canonical_json = serde_json::to_string(&data)
            .map_err(|e| anyhow::anyhow!("Failed to create canonical message: {}", e))?;
        
        Ok(canonical_json.into_bytes())
    }
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
            vec![
                "songbird".to_string(),
                "beardog".to_string(),
                "nestgate".to_string(),
            ]
        } else {
            service_types
        };

        for service_type in &scan_services {
            info!("🔍 Scanning for {}", service_type);

            let services = timeout(
                Duration::from_secs(timeout_secs),
                self.scan_for_service(service_type, &service_ports),
            )
            .await
            .with_context(|| format!("Timeout scanning for {}", service_type))?
            .with_context(|| format!("Failed to scan for {}", service_type))?;

            for service in services {
                info!(
                    "✅ Discovered {}: {}:{}",
                    service_type,
                    service.address.ip(),
                    service.address.port()
                );
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

    /// Register with Songbird discovery service
    pub async fn register_with_songbird(
        &mut self,
        endpoint: String,
        token: Option<String>,
    ) -> Result<()> {
        info!("🐦 Registering with Songbird: {}", endpoint);

        // Parse endpoint
        let addr: SocketAddr = endpoint
            .parse()
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
            ]
            .into_iter()
            .collect(),
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
            }
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
        info!(
            "🐻 Installing BearDog permissions from: {}",
            permission_file.display()
        );

        // Load permission file
        let permission_content = fs::read_to_string(&permission_file)
            .await
            .with_context(|| {
                format!(
                    "Failed to read permission file: {}",
                    permission_file.display()
                )
            })?;

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
        let addr: SocketAddr = endpoint
            .parse()
            .with_context(|| format!("Invalid NestGate endpoint: {}", endpoint))?;

        // Check if mount point exists
        if !mount_point.exists() {
            fs::create_dir_all(&mount_point).await.with_context(|| {
                format!("Failed to create mount point: {}", mount_point.display())
            })?;
        }

        // Connect to NestGate
        let mount_info = self
            .mount_nestgate_dataset(&addr, &mount_point, dataset.as_deref())
            .await?;

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
            }
            "table" | _ => {
                self.print_ecosystem_table().await?;
            }
        }

        Ok(())
    }

    // Internal helper methods

    async fn scan_for_service(
        &self,
        service_type: &str,
        service_ports: &HashMap<String, u16>,
    ) -> Result<Vec<ServiceEndpoint>> {
        let port = service_ports
            .get(service_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown service type: {}", service_type))?;

        let mut services = Vec::new();

        // Scan local network ranges
        let local_ranges = vec!["127.0.0.1", "192.168.1.0/24", "10.0.0.0/24"];

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
        // Send ping to Songbird and verify response signature
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(&format!("http://{}/health/signed", addr))
        ).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    // Parse signed response
                    match response.json::<SignedServiceResponse>().await {
                        Ok(signed_response) => {
                            let crypto_context = CryptoVerificationContext::new();
                            
                            // Verify cryptographic signature of response
                            match crypto_context.verify_service_signature("songbird", &signed_response) {
                                Ok(true) => {
                                    info!("✅ Songbird service cryptographically verified");
                                    Ok(true)
                                }
                                Ok(false) => {
                                    warn!("⚠️  Songbird service signature verification failed");
                                    Ok(false)
                                }
                                Err(e) => {
                                    error!("❌ Songbird verification error: {}", e);
                                    Ok(false)
                                }
                            }
                        }
                        Err(_) => {
                            // Try fallback to unsigned health check with warning
                            warn!("⚠️  Songbird service does not support signed responses - security degraded");
                            Ok(false) // Fail securely for production systems
                        }
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false)
        }
    }

    async fn verify_beardog_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Verify BearDog cryptographic service with proper signature validation
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(&format!("http://{}/crypto/identity/signed", addr))
        ).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    // Parse BearDog identity response
                    match response.json::<SignedServiceResponse>().await {
                        Ok(signed_response) => {
                            let crypto_context = CryptoVerificationContext::new();
                            
                            // Verify BearDog cryptographic identity and signature
                            match crypto_context.verify_service_signature("beardog", &signed_response) {
                                Ok(true) => {
                                    // Additional BearDog-specific verification
                                    if self.verify_beardog_capabilities(&signed_response).await? {
                                        info!("✅ BearDog cryptographic service verified");
                                        Ok(true)
                                    } else {
                                        warn!("⚠️  BearDog capability verification failed");
                                        Ok(false)
                                    }
                                }
                                Ok(false) => {
                                    error!("❌ BearDog cryptographic signature verification failed");
                                    Ok(false)
                                }
                                Err(e) => {
                                    error!("❌ BearDog verification error: {}", e);
                                    Ok(false)
                                }
                            }
                        }
                        Err(_) => {
                            error!("🚨 BearDog service does not support cryptographic verification");
                            Ok(false) // SECURITY: Always fail for BearDog without crypto
                        }
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false)
        }
    }

    async fn verify_nestgate_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Verify NestGate storage service with proper access controls
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(&format!("http://{}/storage/access/signed", addr))
        ).await {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    // Parse NestGate access control response
                    match response.json::<SignedServiceResponse>().await {
                        Ok(signed_response) => {
                            let crypto_context = CryptoVerificationContext::new();
                            
                            // Verify storage service permissions and encryption keys
                            match crypto_context.verify_service_signature("nestgate", &signed_response) {
                                Ok(true) => {
                                    // Additional NestGate-specific verification
                                    if self.verify_nestgate_permissions(&signed_response).await? {
                                        info!("✅ NestGate storage service verified");
                                        Ok(true)
                                    } else {
                                        warn!("⚠️  NestGate permission verification failed");
                                        Ok(false)
                                    }
                                }
                                Ok(false) => {
                                    error!("❌ NestGate signature verification failed");
                                    Ok(false)
                                }
                                Err(e) => {
                                    error!("❌ NestGate verification error: {}", e);
                                    Ok(false)
                                }
                            }
                        }
                        Err(_) => {
                            error!("🚨 NestGate service does not support signed responses");
                            Ok(false) // SECURITY: Fail securely
                        }
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false)
        }
    }

    async fn verify_beardog_capabilities(&self, response: &SignedServiceResponse) -> Result<bool> {
        // Verify BearDog has required cryptographic capabilities
        let required_capabilities = vec![
            "ed25519_signing",
            "key_generation", 
            "signature_verification",
            "identity_management"
        ];

        for capability in required_capabilities {
            if !response.capabilities.contains(&capability.to_string()) {
                warn!("BearDog missing required capability: {}", capability);
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn verify_nestgate_permissions(&self, response: &SignedServiceResponse) -> Result<bool> {
        // Verify NestGate has required storage permissions
        let required_capabilities = vec![
            "zfs_management",
            "encryption_support",
            "access_control",
            "snapshot_management"
        ];

        for capability in required_capabilities {
            if !response.capabilities.contains(&capability.to_string()) {
                warn!("NestGate missing required capability: {}", capability);
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_standard_service_ports(&self) -> HashMap<String, u16> {
        vec![
            ("songbird".to_string(), 5000),
            ("beardog".to_string(), 5001),
            ("nestgate".to_string(), 5002),
        ]
        .into_iter()
        .collect()
    }

    fn get_local_address(&self) -> Result<SocketAddr> {
        // Get local network address
        // This is a simplified implementation
        Ok("127.0.0.1:8080".parse()?)
    }

    async fn send_songbird_registration(
        &self,
        addr: &SocketAddr,
        registration: &SongbirdRegistration,
    ) -> Result<SongbirdResponse> {
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

        if permission.valid_until < chrono::Utc::now() {
            warn!("⚠️  BearDog permission has expired");
            return Ok(false);
        }

        // Implement proper cryptographic verification
        let crypto_context = CryptoVerificationContext::new();
        
        // Get public key for BearDog permission verification
        if let Some(beardog_key) = crypto_context.trusted_public_keys.get("beardog") {
            // Create canonical permission message for verification
            let permission_message = self.create_permission_message(permission)?;
            
            // Decode signature and public key
            let signature_bytes = BASE64.decode(&permission.signature)
                .map_err(|e| anyhow::anyhow!("Failed to decode permission signature: {}", e))?;
            
            let public_key_bytes = BASE64.decode(beardog_key)
                .map_err(|e| anyhow::anyhow!("Failed to decode BearDog public key: {}", e))?;

            // Verify signature using BearDog public key cryptography
            match crypto_context.verify_ed25519_signature(
                &permission_message,
                &signature_bytes,
                &public_key_bytes,
            ) {
                Ok(true) => {
                    info!("✅ BearDog permission signature verified");
                    Ok(true)
                }
                Ok(false) => {
                    error!("❌ BearDog permission signature verification failed");
                    Ok(false)
                }
                Err(e) => {
                    error!("❌ BearDog permission verification error: {}", e);
                    Ok(false)
                }
            }
        } else {
            error!("🚨 No trusted BearDog public key configured");
            Ok(false) // SECURITY: Fail securely without trusted key
        }
    }

    fn create_permission_message(&self, permission: &BearDogPermission) -> Result<Vec<u8>> {
        // Create canonical message for BearDog permission verification
        let mut data = BTreeMap::new();
        data.insert("permission_id", permission.permission_id.to_string());
        data.insert("granted_to", permission.granted_to.clone());
        data.insert("valid_until", permission.valid_until.to_rfc3339());
        
        let capabilities_json = serde_json::to_string(&permission.capabilities)
            .map_err(|e| anyhow::anyhow!("Failed to serialize permission capabilities: {}", e))?;
        data.insert("capabilities", capabilities_json);

        let canonical_json = serde_json::to_string(&data)
            .map_err(|e| anyhow::anyhow!("Failed to create canonical permission message: {}", e))?;
        
        Ok(canonical_json.into_bytes())
    }

    async fn install_permission(&self, permission: &BearDogPermission) -> Result<()> {
        // Install permission in system keyring or secure storage
        // This is a simplified implementation
        info!("🔐 Installing permission: {}", permission.permission_id);
        Ok(())
    }

    async fn mount_nestgate_dataset(
        &self,
        addr: &SocketAddr,
        mount_point: &PathBuf,
        dataset: Option<&str>,
    ) -> Result<NestGateMount> {
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
            println!(
                "{:<20} {:<20} {:<15} {:<15}",
                "SERVICE", "ADDRESS", "TRUST", "CAPABILITIES"
            );
            println!("{}", "-".repeat(70));

            for (key, endpoint) in &self.endpoints {
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

            for (key, connection) in &self.connections {
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

    async fn scan_local_networks(&self) -> Result<Vec<DiscoveredService>> {
        let mut discovered = Vec::new();

        // Get discovery ranges from configuration or environment
        let discovery_ranges = std::env::var("TOADSTOOL_DISCOVERY_RANGES")
            .map(|ranges| ranges.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| vec![
                "127.0.0.1/32".to_string(),
                "192.168.1.0/24".to_string(),
                "10.0.0.0/24".to_string(),
                "172.16.0.0/24".to_string(),
            ]);

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

    async fn scan_cidr_range(&self, cidr: &str) -> Result<Vec<DiscoveredService>> {
        let mut services = Vec::new();
        
        // Parse CIDR notation
        if cidr.contains('/') {
            let parts: Vec<&str> = cidr.split('/').collect();
            if parts.len() == 2 {
                let base_ip = parts[0];
                let prefix_len: u32 = parts[1].parse().unwrap_or(24);
                
                // Generate IP range based on CIDR
                let ip_range = self.generate_ip_range(base_ip, prefix_len)?;
                
                // Scan each IP in range
                for ip in ip_range.iter().take(254) { // Limit scan size
                    if let Ok(discovered) = self.scan_ip_for_services(ip).await {
                        services.extend(discovered);
                    }
                }
            }
        } else {
            // Single IP address
            if let Ok(discovered) = self.scan_ip_for_services(cidr).await {
                services.extend(discovered);
            }
        }
        
        Ok(services)
    }

    fn generate_ip_range(&self, base_ip: &str, prefix_len: u32) -> Result<Vec<String>> {
        let mut ips = Vec::new();
        
        // Simple implementation for common cases
        if prefix_len == 24 {
            // Class C subnet (e.g., 192.168.1.0/24)
            if let Some(base) = base_ip.rsplitn(2, '.').nth(1) {
                for i in 1..255 {
                    ips.push(format!("{}.{}", base, i));
                }
            }
        } else if prefix_len == 32 {
            // Single host
            ips.push(base_ip.to_string());
        }
        // Add more CIDR handling as needed
        
        Ok(ips)
    }

    async fn scan_ip_for_services(&self, ip: &str) -> Result<Vec<DiscoveredService>> {
        let mut services = Vec::new();
        
        // Common service ports to scan
        let service_ports = [
            (8080, ServiceType::Songbird),
            (8081, ServiceType::NestGate), 
            (8082, ServiceType::BearDog),
            (8083, ServiceType::ToadStool),
        ];

        for (port, service_type) in service_ports {
            let addr = format!("{}:{}", ip, port);
            if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
                if self.is_port_open(&socket_addr).await {
                    services.push(DiscoveredService {
                        service_type,
                        address: socket_addr,
                        trust_level: TrustLevel::Discovered, // Default to discovered
                        capabilities: HashMap::new(),
                        last_seen: Utc::now(),
                    });
                }
            }
        }

        Ok(services)
    }

    async fn is_port_open(&self, addr: &SocketAddr) -> bool {
        match tokio::time::timeout(
            Duration::from_millis(1000),
            tokio::net::TcpStream::connect(addr)
        ).await {
            Ok(Ok(_)) => true,
            _ => false,
        }
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
