//! IPC helpers for primal-to-primal communication
//!
//! ## Deep Debt Principles
//!
//! - **Services, Not Libraries**: Communicate via services, not code embedding
//! - **Runtime Discovery**: Discover primals at runtime via Songbird
//! - **Self-Knowledge**: Only know ourselves, discover others
//! - **Standard Protocol**: JSON-RPC 2.0 over Unix sockets
//!
//! ## Architecture
//!
//! ```text
//! ToadStool ──[discover]──> Songbird ──[resolve]──> Other Primal
//!     │                         │
//!     └─────[register]──────────┘
//! ```

use serde_json::{json, Value};
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};

/// Default Songbird socket path (standard primal namespace)
const SONGBIRD_SOCKET: &str = "/primal/songbird";

/// Request timeout for IPC operations
const IPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Register ToadStool with Songbird discovery service
///
/// ## Deep Debt Principle: Self-Knowledge
///
/// ToadStool announces its own capabilities, not hardcoded info.
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc_helpers;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     ipc_helpers::register_with_songbird().await?;
///     Ok(())
/// }
/// ```
pub async fn register_with_songbird() -> ToadStoolResult<()> {
    // Get Songbird socket path (environment override supported)
    let socket_path =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| SONGBIRD_SOCKET.to_string());

    info!("🌍 Registering with Songbird at {}", socket_path);

    // Connect to Songbird
    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to Songbird"))?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to Songbird at {}: {}. Is Songbird running?",
                socket_path, e
            ))
        })?;

    // Build registration request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.register",
        "params": {
            "primal_name": "toadstool",
            "capabilities": ["compute", "gpu", "wasm", "container"],
            "endpoint": std::env::var("TOADSTOOL_SOCKET")
                .unwrap_or_else(|_| "/primal/toadstool".to_string())
        },
        "id": 1
    });

    // Send registration
    write_json_rpc(&mut stream, &request).await?;

    // Read response
    let response: Value = read_json_rpc(&mut stream).await?;

    // Check for errors
    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Songbird registration failed: {}",
            error
        )));
    }

    info!("✅ Successfully registered with Songbird discovery service");
    debug!("   Registration response: {:?}", response);

    Ok(())
}

/// Resolve a primal's endpoint via Songbird
///
/// ## Deep Debt Principle: Runtime Discovery
///
/// Discover other primals at runtime, not compile time.
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc_helpers;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let endpoint = ipc_helpers::resolve_primal("beardog").await?;
///     println!("BearDog endpoint: {}", endpoint);
///     Ok(())
/// }
/// ```
pub async fn resolve_primal(primal_name: &str) -> ToadStoolResult<String> {
    // Get Songbird socket path
    let socket_path =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| SONGBIRD_SOCKET.to_string());

    debug!("🔍 Resolving {} via Songbird", primal_name);

    // Connect to Songbird
    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to Songbird"))?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to Songbird: {}. Is Songbird running?",
                e
            ))
        })?;

    // Build resolve request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.resolve",
        "params": {
            "primal_name": primal_name
        },
        "id": 1
    });

    // Send request
    write_json_rpc(&mut stream, &request).await?;

    // Read response
    let response: Value = read_json_rpc(&mut stream).await?;

    // Check for errors
    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Failed to resolve {}: {}",
            primal_name, error
        )));
    }

    // Extract endpoint
    let endpoint = response
        .get("result")
        .and_then(|r| r.get("endpoint"))
        .and_then(|e| e.as_str())
        .ok_or_else(|| {
            ToadStoolError::integration(format!(
                "Invalid response from Songbird: missing endpoint for {}",
                primal_name
            ))
        })?
        .to_string();

    debug!("✅ Resolved {} -> {}", primal_name, endpoint);

    Ok(endpoint)
}

/// Connect to another primal
///
/// ## Deep Debt Principle: Service-Based Communication
///
/// Connect via service discovery, not hardcoded endpoints.
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc_helpers;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let stream = ipc_helpers::connect_to_primal("beardog").await?;
///     // Use stream for communication
///     Ok(())
/// }
/// ```
pub async fn connect_to_primal(primal_name: &str) -> ToadStoolResult<UnixStream> {
    // Resolve endpoint
    let endpoint = resolve_primal(primal_name).await?;

    info!("🔗 Connecting to {} at {}", primal_name, endpoint);

    // Connect directly to primal
    let stream = timeout(IPC_TIMEOUT, UnixStream::connect(&endpoint))
        .await
        .map_err(|_| {
            ToadStoolError::integration(format!(
                "Timeout connecting to {} at {}",
                primal_name, endpoint
            ))
        })?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to {} at {}: {}",
                primal_name, endpoint, e
            ))
        })?;

    debug!("✅ Connected to {}", primal_name);

    Ok(stream)
}

/// Find primals by capability
///
/// ## Deep Debt Principle: Capability-Based Discovery
///
/// Discover services by what they can do, not who they are.
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc_helpers;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let crypto_primals = ipc_helpers::find_by_capability("crypto").await?;
///     for primal in crypto_primals {
///         println!("Found crypto service: {}", primal);
///     }
///     Ok(())
/// }
/// ```
pub async fn find_by_capability(capability: &str) -> ToadStoolResult<Vec<String>> {
    // Get Songbird socket path
    let socket_path =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| SONGBIRD_SOCKET.to_string());

    debug!("🔍 Finding primals with capability: {}", capability);

    // Connect to Songbird
    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to Songbird"))?
        .map_err(|e| {
            ToadStoolError::integration(format!("Failed to connect to Songbird: {}", e))
        })?;

    // Build capabilities request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "ipc.capabilities",
        "params": {
            "capability": capability
        },
        "id": 1
    });

    // Send request
    write_json_rpc(&mut stream, &request).await?;

    // Read response
    let response: Value = read_json_rpc(&mut stream).await?;

    // Check for errors
    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Failed to find capability {}: {}",
            capability, error
        )));
    }

    // Extract primal names
    let primals: Vec<String> = response
        .get("result")
        .and_then(|r| r.get("services"))
        .and_then(|s| s.as_array())
        .map(|services| {
            services
                .iter()
                .filter_map(|service| {
                    service
                        .get("primal_name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    debug!(
        "✅ Found {} primals with capability {}",
        primals.len(),
        capability
    );

    Ok(primals)
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Write JSON-RPC message to stream
async fn write_json_rpc(stream: &mut UnixStream, message: &Value) -> ToadStoolResult<()> {
    use tokio::io::AsyncWriteExt;

    let json_str = serde_json::to_string(message)
        .map_err(|e| ToadStoolError::integration(format!("Failed to serialize JSON-RPC: {}", e)))?;

    // Write with newline delimiter
    let data = format!("{}\n", json_str);

    stream
        .write_all(data.as_bytes())
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to write to stream: {}", e)))?;

    stream
        .flush()
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to flush stream: {}", e)))?;

    Ok(())
}

/// Read JSON-RPC message from stream
async fn read_json_rpc(stream: &mut UnixStream) -> ToadStoolResult<Value> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to read from stream: {}", e)))?;

    if line.is_empty() {
        return Err(ToadStoolError::integration("Connection closed by peer"));
    }

    serde_json::from_str(&line)
        .map_err(|e| ToadStoolError::integration(format!("Failed to parse JSON-RPC: {}", e)))
}

// ============================================================================
// Semantic Method Name Resolution
// ============================================================================

use crate::semantic_methods::SemanticMethodRegistry;
use std::sync::OnceLock;

/// Global semantic method registry (initialized once)
static SEMANTIC_REGISTRY: OnceLock<SemanticMethodRegistry> = OnceLock::new();

/// Get the global semantic method registry
fn get_registry() -> &'static SemanticMethodRegistry {
    SEMANTIC_REGISTRY.get_or_init(SemanticMethodRegistry::new)
}

/// Resolve method name from semantic to implementation
///
/// Supports both semantic names (e.g., `compute.execute`) and legacy names
/// (e.g., `execute_workload`) for backward compatibility.
///
/// ## Phase 1: Backward Compatible
///
/// - Semantic names are resolved to implementation names
/// - Implementation names pass through unchanged
/// - Both work seamlessly
///
/// ## Examples
///
/// ```
/// use toadstool::ipc_helpers::resolve_method_name;
///
/// // Semantic name → implementation name
/// assert_eq!(resolve_method_name("compute.execute"), "execute_workload");
///
/// // Implementation name → pass through
/// assert_eq!(resolve_method_name("execute_workload"), "execute_workload");
///
/// // Unknown semantic name → pass through
/// assert_eq!(resolve_method_name("unknown.method"), "unknown.method");
/// ```
pub fn resolve_method_name(method: &str) -> String {
    let registry = get_registry();
    
    // If it's a semantic name, try to resolve it
    if registry.is_semantic(method) {
        if let Some(impl_name) = registry.resolve(method) {
            debug!(
                "Resolved semantic method '{}' → '{}'",
                method, impl_name
            );
            return impl_name.to_string();
        }
        // Unknown semantic name - pass through (might be new/external)
        debug!("Unknown semantic method '{}', passing through", method);
        method.to_string()
    } else {
        // Not semantic (no dot) - pass through as implementation name
        method.to_string()
    }
}

/// Check if a method name is semantic (contains '.')
///
/// ## Examples
///
/// ```
/// use toadstool::ipc_helpers::is_semantic_method;
///
/// assert!(is_semantic_method("compute.execute"));
/// assert!(is_semantic_method("resource.cpu.get_usage"));
/// assert!(!is_semantic_method("execute_workload"));
/// ```
pub fn is_semantic_method(method: &str) -> bool {
    get_registry().is_semantic(method)
}

/// Get semantic name for implementation method (if registered)
///
/// ## Examples
///
/// ```
/// use toadstool::ipc_helpers::get_semantic_name;
///
/// assert_eq!(get_semantic_name("execute_workload"), Some("compute.execute"));
/// assert_eq!(get_semantic_name("unknown_method"), None);
/// ```
pub fn get_semantic_name(implementation: &str) -> Option<String> {
    get_registry()
        .get_semantic(implementation)
        .map(|s| s.to_string())
}

/// Get all registered semantic method names
///
/// Useful for API documentation and capability advertisement.
pub fn list_semantic_methods() -> Vec<String> {
    get_registry()
        .semantic_names()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(SONGBIRD_SOCKET, "/primal/songbird");
        assert_eq!(IPC_TIMEOUT.as_secs(), 5);
    }

    #[tokio::test]
    async fn test_register_with_songbird_graceful_failure() {
        // Should fail gracefully when Songbird not available
        let result = register_with_songbird().await;

        // Expecting error since Songbird not running in test
        assert!(result.is_err());

        // Error should be informative
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("Songbird") || err_msg.contains("connection"));
    }

    #[tokio::test]
    async fn test_resolve_primal_graceful_failure() {
        // Should fail gracefully when Songbird not available
        let result = resolve_primal("beardog").await;

        // Expecting error since Songbird not running in test
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_to_primal_graceful_failure() {
        // Should fail gracefully when Songbird not available
        let result = connect_to_primal("beardog").await;

        // Expecting error since Songbird not running in test
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_capability_graceful_failure() {
        // Should fail gracefully when Songbird not available
        let result = find_by_capability("crypto").await;

        // Expecting error since Songbird not running in test
        assert!(result.is_err());
    }

    #[test]
    fn test_json_rpc_request_format() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "ipc.register",
            "params": {
                "primal_name": "toadstool",
                "capabilities": ["compute"]
            },
            "id": 1
        });

        assert_eq!(request.get("jsonrpc").unwrap(), "2.0");
        assert_eq!(request.get("method").unwrap(), "ipc.register");
        assert!(request.get("params").is_some());
        assert_eq!(request.get("id").unwrap(), 1);
    }
    
    // ========================================================================
    // Semantic Method Resolution Tests
    // ========================================================================
    
    #[test]
    fn test_resolve_semantic_to_implementation() {
        assert_eq!(
            resolve_method_name("compute.execute"),
            "execute_workload"
        );
        assert_eq!(
            resolve_method_name("resource.health.check"),
            "check_health"
        );
        assert_eq!(
            resolve_method_name("storage.artifact.store"),
            "store_artifact"
        );
    }
    
    #[test]
    fn test_resolve_implementation_passthrough() {
        // Implementation names should pass through unchanged
        assert_eq!(
            resolve_method_name("execute_workload"),
            "execute_workload"
        );
        assert_eq!(resolve_method_name("check_health"), "check_health");
    }
    
    #[test]
    fn test_resolve_unknown_semantic() {
        // Unknown semantic names should pass through
        assert_eq!(
            resolve_method_name("unknown.method"),
            "unknown.method"
        );
        assert_eq!(
            resolve_method_name("future.api.call"),
            "future.api.call"
        );
    }
    
    #[test]
    fn test_is_semantic_method() {
        assert!(is_semantic_method("compute.execute"));
        assert!(is_semantic_method("resource.cpu.get_usage"));
        assert!(!is_semantic_method("execute_workload"));
        assert!(!is_semantic_method("single_word"));
    }
    
    #[test]
    fn test_get_semantic_name() {
        assert_eq!(
            get_semantic_name("execute_workload"),
            Some("compute.execute".to_string())
        );
        assert_eq!(
            get_semantic_name("check_health"),
            Some("resource.health.check".to_string())
        );
        assert_eq!(get_semantic_name("unknown_method"), None);
    }
    
    #[test]
    fn test_list_semantic_methods() {
        let methods = list_semantic_methods();
        
        // Should have many methods
        assert!(methods.len() > 40);
        
        // Should include standard methods
        assert!(methods.contains(&"compute.execute".to_string()));
        assert!(methods.contains(&"resource.health.check".to_string()));
        assert!(methods.contains(&"storage.artifact.store".to_string()));
        assert!(methods.contains(&"network.configure".to_string()));
        assert!(methods.contains(&"security.policy.apply".to_string()));
    }
    
    #[test]
    fn test_semantic_resolution_bidirectional() {
        // Forward: semantic → implementation
        let impl_name = resolve_method_name("compute.execute");
        assert_eq!(impl_name, "execute_workload");
        
        // Reverse: implementation → semantic
        let semantic_name = get_semantic_name(&impl_name);
        assert_eq!(semantic_name, Some("compute.execute".to_string()));
    }
    
    #[test]
    fn test_runtime_variant_resolution() {
        assert_eq!(
            resolve_method_name("compute.container.run"),
            "run_container"
        );
        assert_eq!(
            resolve_method_name("compute.wasm.execute"),
            "start_wasm_module"
        );
        assert_eq!(
            resolve_method_name("compute.python.execute"),
            "run_python_script"
        );
        assert_eq!(
            resolve_method_name("compute.native.execute"),
            "run_native_binary"
        );
        assert_eq!(
            resolve_method_name("compute.gpu.execute"),
            "run_gpu_compute"
        );
    }
    
    #[test]
    fn test_all_domains_covered() {
        let methods = list_semantic_methods();
        
        // Verify all domains are represented
        let has_compute = methods.iter().any(|m| m.starts_with("compute."));
        let has_resource = methods.iter().any(|m| m.starts_with("resource."));
        let has_storage = methods.iter().any(|m| m.starts_with("storage."));
        let has_network = methods.iter().any(|m| m.starts_with("network."));
        let has_security = methods.iter().any(|m| m.starts_with("security."));
        let has_runtime = methods.iter().any(|m| m.starts_with("runtime."));
        
        assert!(has_compute, "Missing compute domain");
        assert!(has_resource, "Missing resource domain");
        assert!(has_storage, "Missing storage domain");
        assert!(has_network, "Missing network domain");
        assert!(has_security, "Missing security domain");
        assert!(has_runtime, "Missing runtime domain");
    }
}
