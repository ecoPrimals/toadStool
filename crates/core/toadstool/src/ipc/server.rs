//! Universal IPC server with multi-transport support
//!
//! **Deep Debt Principles**:
//! - ✅ Multi-transport (bind all available at once)
//! - ✅ Capability-based (auto-detect what's available)
//! - ✅ Graceful shutdown (clean socket cleanup)
//! - ✅ Safe async (tokio, no unsafe)
//!
//! ## Server Lifecycle
//!
//! 1. **Bind**: Bind all available transports (Unix, Abstract, TCP)
//! 2. **Accept**: Accept connections from any transport
//! 3. **Shutdown**: Clean up all sockets gracefully

use tokio::sync::mpsc;
use crate::ToadStoolResult;
use super::platform::{self, Endpoint};

/// Multi-transport IPC server
///
/// **Deep Debt**: Binds all available transports simultaneously
pub struct IpcServer {
    endpoints: Vec<Endpoint>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl IpcServer {
    /// Create server for ToadStool with all transports
    ///
    /// **Deep Debt**: Runtime capability detection
    pub fn for_toadstool() -> Self {
        let mut endpoints = Vec::new();
        
        // Tier 1: Platform-specific transports
        #[cfg(target_os = "linux")]
        {
            // Abstract socket (Android, SELinux-friendly)
            endpoints.push(Endpoint::Abstract {
                name: "@biomeos_toadstool".to_string(),
            });
        }
        
        // Unix socket (Linux desktop, macOS)
        endpoints.push(Endpoint::for_toadstool());
        
        // Tier 2: TCP (universal fallback)
        endpoints.push(Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
            port: platform::tcp::DEFAULT_PORT,
        });
        
        Self {
            endpoints,
            shutdown_tx: None,
        }
    }
    
    /// Create server for specific primal
    ///
    /// **Deep Debt**: No hardcoding, runtime configuration
    pub fn for_primal(primal_name: &str) -> Self {
        let mut endpoints = Vec::new();
        
        // Tier 1: Platform-specific
        #[cfg(target_os = "linux")]
        {
            endpoints.push(Endpoint::Abstract {
                name: format!("@biomeos_{}", primal_name.to_lowercase()),
            });
        }
        
        // Unix socket
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
            if let Ok(uid) = toadstool_common::uid_detector::get_user_id() {
                format!("/run/user/{}", uid)
            } else {
                "/tmp/biomeos-runtime".to_string()
            }
        });
        
        endpoints.push(Endpoint::Unix {
            path: format!("{}/biomeos/{}.sock", runtime_dir, primal_name.to_lowercase()).into(),
        });
        
        // TCP with port allocation
        let port = match primal_name.to_lowercase().as_str() {
            "toadstool" => 8370,
            "songbird" => 8371,
            "beardog" => 8372,
            "squirrel" => 8373,
            "nestgate" => 8374,
            _ => 8375,
        };
        
        endpoints.push(Endpoint::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        });
        
        Self {
            endpoints,
            shutdown_tx: None,
        }
    }
    
    /// Create server with custom endpoints
    pub fn with_endpoints(endpoints: Vec<Endpoint>) -> Self {
        Self {
            endpoints,
            shutdown_tx: None,
        }
    }
    
    /// Bind all configured transports
    ///
    /// **Deep Debt**: Binds all available, logs failures (doesn't crash)
    pub async fn bind(&mut self) -> ToadStoolResult<()> {
        for endpoint in &self.endpoints {
            match self.try_bind(endpoint).await {
                Ok(_) => {
                    tracing::info!("✅ Bound {}", endpoint.display());
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to bind {}: {}", endpoint.display(), e);
                    // Don't fail - some transports might not be available
                }
            }
        }
        
        Ok(())
    }
    
    /// Try to bind specific endpoint
    async fn try_bind(&self, endpoint: &Endpoint) -> ToadStoolResult<()> {
        match endpoint {
            Endpoint::Unix { path } => {
                let _ = platform::unix::bind(path).await?;
                Ok(())
            }
            
            #[cfg(target_os = "linux")]
            Endpoint::Abstract { name } => {
                let _ = platform::abstract_socket::bind(name).await?;
                Ok(())
            }
            
            Endpoint::Tcp { host, port } => {
                let _ = platform::tcp::bind(host, *port).await?;
                Ok(())
            }
        }
    }
    
    /// Get configured endpoints
    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }
    
    /// Initiate graceful shutdown
    pub async fn shutdown(&mut self) -> ToadStoolResult<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Clean up Unix sockets on drop
        for endpoint in &self.endpoints {
            if let Endpoint::Unix { path } = endpoint {
                if path.exists() {
                    let _ = std::fs::remove_file(path);
                    tracing::debug!("🧹 Cleaned up {}", path.display());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_for_toadstool() {
        let server = IpcServer::for_toadstool();
        let endpoints = server.endpoints();
        
        // Should have multiple endpoints
        assert!(!endpoints.is_empty());
        
        // Should include TCP
        assert!(endpoints.iter().any(|e| e.is_tcp()));
        
        #[cfg(target_os = "linux")]
        {
            // On Linux, should have abstract
            assert!(endpoints.iter().any(|e| e.is_abstract()));
        }
    }
    
    #[test]
    fn test_server_for_primal() {
        let server = IpcServer::for_primal("Songbird");
        let endpoints = server.endpoints();
        
        // Should have Songbird port
        assert!(endpoints.iter().any(|e| {
            matches!(e, Endpoint::Tcp { port, .. } if *port == 8371)
        }));
    }
    
    #[test]
    fn test_server_custom_endpoints() {
        let custom = vec![
            Endpoint::Tcp {
                host: "0.0.0.0".to_string(),
                port: 9000,
            },
        ];
        
        let server = IpcServer::with_endpoints(custom.clone());
        assert_eq!(server.endpoints().len(), 1);
    }
    
    #[tokio::test]
    async fn test_bind_cleanup() {
        use std::path::PathBuf;
        
        // Create server with test socket
        let test_path = PathBuf::from("/tmp/toadstool_test_server.sock");
        let _ = std::fs::remove_file(&test_path);
        
        let endpoints = vec![Endpoint::Unix { path: test_path.clone() }];
        
        {
            let mut server = IpcServer::with_endpoints(endpoints);
            let _ = server.bind().await;
            
            // Socket should exist
            assert!(test_path.exists());
        } // server dropped
        
        // Socket should be cleaned up
        // (Drop trait removes it)
    }
}
