//! biomeOS Registry Client
//!
//! **Purpose**: Connect ToadStool to biomeOS capability registry via Unix socket IPC.
//!
//! **Design Philosophy**:
//! - Self-knowledge only: ToadStool knows what it provides
//! - Runtime discovery: Discover other primals by capability, not by name
//! - No hardcoding: Never hardcode "BearDog", "Songbird", or "NestGate"
//! - Graceful degradation: Work without biomeOS (standalone mode)
//!
//! **Architecture**:
//! ```text
//! ToadStool (Workload Orchestrator)
//!     ↓ (Unix socket IPC)
//! biomeOS Registry (/tmp/biomeos-registry-{family}.sock)
//!     ↓ (Capability queries)
//! BearDog (provides: Security, Encryption)
//! Songbird (provides: Discovery, ConnectionManagement)
//! NestGate (provides: Storage, DataPersistence)
//! ```
//!
//! **Usage**:
//! ```rust,no_run
//! use toadstool::biomeos_integration::BiomeOSClient;
//! use toadstool_common::Capability;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to biomeOS registry
//! let client = BiomeOSClient::connect().await?;
//!
//! // Register ToadStool capabilities
//! client.register_self().await?;
//!
//! // Discover security provider (BearDog) by capability, not by name
//! let security = client.get_provider(Capability::Security).await?;
//! println!("Security provider: {} at {}", security.name, security.endpoint);
//!
//! // Discover discovery provider (Songbird) by capability
//! let discovery = client.get_provider(Capability::Discovery).await?;
//! println!("Discovery provider: {} at {}", discovery.name, discovery.endpoint);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use toadstool_common::primal_identity::Capability;
use toadstool_common::{ToadStoolError, ToadStoolResult};

/// Information about a discovered primal
#[derive(Debug, Clone)]
pub struct PrimalInfo {
    /// Primal name (e.g., "beardog", "songbird")
    pub name: String,
    
    /// Service endpoint (e.g., "unix:///tmp/beardog-{family}.sock", "http://localhost:8081")
    pub endpoint: String,
    
    /// Capabilities provided
    pub capabilities: Vec<Capability>,
    
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// biomeOS Registry Client
///
/// **Thread-safe**: Uses Arc<RwLock<_>> internally
#[derive(Clone)]
pub struct BiomeOSClient {
    /// Path to biomeOS registry Unix socket
    socket_path: PathBuf,
    
    /// Cached connection (lazy-initialized)
    connection: Arc<RwLock<Option<UnixStream>>>,
    
    /// Cached primal info (to avoid repeated queries)
    cache: Arc<RwLock<std::collections::HashMap<String, PrimalInfo>>>,
}

impl BiomeOSClient {
    /// Connect to biomeOS registry
    ///
    /// **Design**: Auto-detects socket path from environment or uses default.
    ///
    /// **Environment Variables**:
    /// - `BIOMEOS_REGISTRY_SOCKET`: Custom socket path
    /// - `BIOMEOS_FAMILY`: Family ID (default: "default")
    ///
    /// **Default Path**: `/tmp/biomeos-registry-{family}.sock`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - biomeOS registry socket doesn't exist
    /// - Connection to socket fails
    /// - Socket permissions are incorrect
    pub async fn connect() -> ToadStoolResult<Self> {
        let socket_path = Self::detect_socket_path();
        
        debug!("🔍 Connecting to biomeOS registry: {}", socket_path.display());
        
        // Verify socket exists (but don't connect yet - lazy)
        if !socket_path.exists() {
            warn!(
                "⚠️  biomeOS registry socket not found: {}",
                socket_path.display()
            );
            warn!("   Running in standalone mode (no biomeOS orchestration)");
            
            // Return "disconnected" client for graceful degradation
            return Ok(Self {
                socket_path,
                connection: Arc::new(RwLock::new(None)),
                cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            });
        }
        
        info!("✅ biomeOS registry socket found: {}", socket_path.display());
        
        Ok(Self {
            socket_path,
            connection: Arc::new(RwLock::new(None)),
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }
    
    /// Connect with custom socket path
    ///
    /// **Use case**: Testing, custom deployments
    pub async fn connect_with_path(socket_path: impl Into<PathBuf>) -> ToadStoolResult<Self> {
        let socket_path = socket_path.into();
        
        Ok(Self {
            socket_path,
            connection: Arc::new(RwLock::new(None)),
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }
    
    /// Register ToadStool capabilities with biomeOS
    ///
    /// **Design**: Self-knowledge only - ToadStool advertises what it provides.
    ///
    /// **Registered Capabilities**:
    /// - `Compute`: Universal compute orchestration
    /// - `Storage`: Workload data storage
    /// - `Orchestration`: Multi-service deployment
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection to biomeOS registry fails
    /// - Registration message serialization fails
    /// - biomeOS rejects registration
    pub async fn register_self(&self) -> ToadStoolResult<()> {
        use toadstool_common::primal_identity::{
            ComputeCapability, CoordinationCapability, StorageCapability,
        };
        
        info!("📝 Registering ToadStool with biomeOS registry");
        
        // ToadStool self-knowledge: what we provide
        let capabilities = vec![
            Capability::Compute(ComputeCapability::ContainerOrchestration),
            Capability::Compute(ComputeCapability::WasmExecution),
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Storage(StorageCapability::ObjectStorage),
            Capability::Coordination(CoordinationCapability::WorkflowOrchestration),
        ];
        
        let metadata: HashMap<String, String> = [
            ("version", env!("CARGO_PKG_VERSION")),
            ("platform", std::env::consts::OS),
            ("arch", std::env::consts::ARCH),
            ("runtime", "toadstool"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        
        // Build registration message (JSON protocol)
        let registration = serde_json::json!({
            "action": "register",
            "primal": "toadstool",
            "capabilities": capabilities.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>(),
            "endpoint": Self::detect_endpoint(),
            "metadata": metadata,
        });
        
        // Send registration to biomeOS
        self.send_message(&registration).await?;
        
        info!("✅ ToadStool registered with biomeOS");
        
        Ok(())
    }
    
    /// Get primal provider by capability
    ///
    /// **Design**: Capability-based discovery - no hardcoded primal names!
    ///
    /// **Examples**:
    /// ```rust,no_run
    /// # use toadstool::biomeos_integration::BiomeOSClient;
    /// # use toadstool_common::Capability;
    /// # async fn example(client: &BiomeOSClient) -> Result<(), Box<dyn std::error::Error>> {
    /// // Discover security provider (BearDog) - NO HARDCODING!
    /// let security = client.get_provider(Capability::Security).await?;
    ///
    /// // Discover discovery provider (Songbird) - NO HARDCODING!
    /// let discovery = client.get_provider(Capability::Discovery).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No provider found for capability
    /// - Connection to biomeOS registry fails
    /// - Response parsing fails
    pub async fn get_provider(&self, capability: Capability) -> ToadStoolResult<PrimalInfo> {
        let cap_key = format!("{capability:?}");
        
        // Check cache first (fast path)
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(&cap_key) {
                debug!("📦 Cache hit for capability: {cap_key}");
                return Ok(info.clone());
            }
        }
        
        debug!("🔍 Discovering provider for capability: {cap_key}");
        
        // Query biomeOS registry
        let query = serde_json::json!({
            "action": "query",
            "capability": cap_key,
        });
        
        let response = self.send_message(&query).await?;
        
        // Parse response
        let primal_info = Self::parse_provider_response(&response)?;
        
        // Cache result (fast future lookups)
        {
            let mut cache = self.cache.write().await;
            cache.insert(cap_key.clone(), primal_info.clone());
        }
        
        info!(
            "✅ Discovered {} for capability: {}",
            primal_info.name, cap_key
        );
        
        Ok(primal_info)
    }
    
    /// Get security provider (BearDog) by capability
    ///
    /// **Convenience wrapper** for `get_provider(Capability::Authentication(AuthCapability::CryptoOperations))`
    pub async fn get_security_provider(&self) -> ToadStoolResult<PrimalInfo> {
        use toadstool_common::primal_identity::AuthCapability;
        self.get_provider(Capability::Authentication(AuthCapability::CryptoOperations)).await
    }
    
    /// Get discovery provider (Songbird) by capability
    ///
    /// **Convenience wrapper** for `get_provider(Capability::Coordination(CoordinationCapability::ServiceDiscovery))`
    pub async fn get_discovery_provider(&self) -> ToadStoolResult<PrimalInfo> {
        use toadstool_common::primal_identity::CoordinationCapability;
        self.get_provider(Capability::Coordination(CoordinationCapability::ServiceDiscovery)).await
    }
    
    /// Get storage provider (NestGate) by capability
    ///
    /// **Convenience wrapper** for `get_provider(Capability::Storage(StorageCapability::ObjectStorage))`
    pub async fn get_storage_provider(&self) -> ToadStoolResult<PrimalInfo> {
        use toadstool_common::primal_identity::StorageCapability;
        self.get_provider(Capability::Storage(StorageCapability::ObjectStorage)).await
    }
    
    /// Check if connected to biomeOS
    ///
    /// **Use case**: Determine if running in orchestrated or standalone mode
    pub async fn is_connected(&self) -> bool {
        self.socket_path.exists() && self.connection.read().await.is_some()
    }
    
    // -------------------------------------------------------------------------
    // Internal implementation
    // -------------------------------------------------------------------------
    
    /// Detect biomeOS registry socket path
    fn detect_socket_path() -> PathBuf {
        // Environment variable override
        if let Ok(path) = std::env::var("BIOMEOS_REGISTRY_SOCKET") {
            return PathBuf::from(path);
        }
        
        // Family-based socket path (default)
        let family = std::env::var("BIOMEOS_FAMILY").unwrap_or_else(|_| "default".to_string());
        PathBuf::from(format!("/tmp/biomeos-registry-{family}.sock"))
    }
    
    /// Detect ToadStool endpoint
    fn detect_endpoint() -> String {
        // Environment variable override
        if let Ok(endpoint) = std::env::var("TOADSTOOL_ENDPOINT") {
            return endpoint;
        }
        
        // Default: Unix socket (preferred for local primals)
        let family = std::env::var("BIOMEOS_FAMILY").unwrap_or_else(|_| "default".to_string());
        format!("unix:///tmp/toadstool-{family}.sock")
    }
    
    /// Send message to biomeOS registry and receive response
    async fn send_message(
        &self,
        message: &serde_json::Value,
    ) -> ToadStoolResult<serde_json::Value> {
        // Lazy connection (establish if needed)
        {
            let mut conn = self.connection.write().await;
            if conn.is_none() {
                if !self.socket_path.exists() {
                    return Err(ToadStoolError::network(format!(
                        "biomeOS registry socket not found: {}",
                        self.socket_path.display()
                    )));
                }
                
                let stream = UnixStream::connect(&self.socket_path)
                    .await
                    .map_err(|e| {
                        ToadStoolError::network(format!(
                            "Failed to connect to biomeOS registry: {e}"
                        ))
                    })?;
                
                *conn = Some(stream);
                debug!("✅ Connected to biomeOS registry");
            }
        }
        
        // Send message (JSON + newline protocol)
        let message_str = serde_json::to_string(message)
            .map_err(|e| ToadStoolError::runtime(format!("Message serialization: {e}")))?;
        
        let mut conn = self.connection.write().await;
        let stream = conn
            .as_mut()
            .ok_or_else(|| ToadStoolError::network("Connection lost"))?;
        
        stream
            .write_all(message_str.as_bytes())
            .await
            .map_err(|e| ToadStoolError::network(format!("Send failed: {e}")))?;
        
        stream
            .write_all(b"\n")
            .await
            .map_err(|e| ToadStoolError::network(format!("Send newline: {e}")))?;
        
        // Read response (JSON + newline protocol)
        let mut reader = BufReader::new(stream);
        let mut response_str = String::new();
        reader
            .read_line(&mut response_str)
            .await
            .map_err(|e| ToadStoolError::network(format!("Receive failed: {e}")))?;
        
        // Parse JSON response
        serde_json::from_str(&response_str).map_err(|e| {
            ToadStoolError::runtime(format!("Response parsing: {e}"))
        })
    }
    
    /// Parse provider info from biomeOS response
    fn parse_provider_response(
        response: &serde_json::Value,
    ) -> ToadStoolResult<PrimalInfo> {
        let obj = response
            .as_object()
            .ok_or_else(|| ToadStoolError::runtime("Response not an object"))?;
        
        // Check for error
        if let Some(error) = obj.get("error") {
            return Err(ToadStoolError::network(format!(
                "biomeOS error: {}",
                error.as_str().unwrap_or("unknown")
            )));
        }
        
        // Extract primal info
        let name = obj
            .get("primal")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToadStoolError::runtime("Missing 'primal' field"))?
            .to_string();
        
        let endpoint = obj
            .get("endpoint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToadStoolError::runtime("Missing 'endpoint' field"))?
            .to_string();
        
        let capabilities = obj
            .get("capabilities")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToadStoolError::runtime("Missing 'capabilities' field"))?
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| {
                // Parse capability string (e.g., "Authentication(CryptoOperations)" -> Capability::Authentication(AuthCapability::CryptoOperations))
                use toadstool_common::primal_identity::{
                    AuthCapability, ComputeCapability, CoordinationCapability,
                    DiscoveryCapability, StorageCapability,
                };
                
                // Simple string-based parsing (can be improved with serde)
                if s.contains("Compute") {
                    Capability::Compute(ComputeCapability::NativeExecution)
                } else if s.contains("Storage") {
                    Capability::Storage(StorageCapability::ObjectStorage)
                } else if s.contains("Authentication") || s.contains("Security") || s.contains("Crypto") {
                    Capability::Authentication(AuthCapability::CryptoOperations)
                } else if s.contains("Discovery") {
                    Capability::Discovery(DiscoveryCapability::CapabilityDiscovery)
                } else if s.contains("Coordination") || s.contains("Orchestration") {
                    Capability::Coordination(CoordinationCapability::ServiceDiscovery)
                } else {
                    Capability::Custom {
                        name: s.to_string(),
                        version: "1.0".to_string(),
                    }
                }
            })
            .collect();
        
        let metadata = obj
            .get("metadata")
            .and_then(|v| v.as_object())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        v.as_str().map(|s| (k.clone(), s.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(PrimalInfo {
            name,
            endpoint,
            capabilities,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_socket_path() {
        // Default path
        let path = BiomeOSClient::detect_socket_path();
        assert!(path.to_string_lossy().contains("/tmp/biomeos-registry-"));
        assert!(path.to_string_lossy().contains(".sock"));
    }
    
    #[test]
    fn test_detect_socket_path_custom_family() {
        std::env::set_var("BIOMEOS_FAMILY", "test-family");
        let path = BiomeOSClient::detect_socket_path();
        assert_eq!(
            path,
            PathBuf::from("/tmp/biomeos-registry-test-family.sock")
        );
        std::env::remove_var("BIOMEOS_FAMILY");
    }
    
    #[test]
    fn test_detect_endpoint() {
        let endpoint = BiomeOSClient::detect_endpoint();
        assert!(endpoint.starts_with("unix:///tmp/toadstool-"));
        assert!(endpoint.ends_with(".sock"));
    }
    
    #[test]
    fn test_parse_provider_response_success() {
        let response = serde_json::json!({
            "primal": "beardog",
            "endpoint": "unix:///tmp/beardog-default.sock",
            "capabilities": ["Security", "Encryption"],
            "metadata": {
                "version": "0.1.0"
            }
        });
        
        let info = BiomeOSClient::parse_provider_response(&response).unwrap();
        assert_eq!(info.name, "beardog");
        assert_eq!(info.endpoint, "unix:///tmp/beardog-default.sock");
        assert_eq!(info.capabilities.len(), 2);
        assert_eq!(info.metadata.get("version").unwrap(), "0.1.0");
    }
    
    #[test]
    fn test_parse_provider_response_error() {
        let response = serde_json::json!({
            "error": "No provider found"
        });
        
        let result = BiomeOSClient::parse_provider_response(&response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No provider found"));
    }
}

