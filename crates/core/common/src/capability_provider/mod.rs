// SPDX-License-Identifier: AGPL-3.0-only
//! Capability-based service discovery and invocation
//!
//! Deep Debt Solution: Primals discover each other by capability at runtime,
//! not by hardcoded names. This enables true ecosystem agnosticism.
//!
//! Philosophy: "Know thyself, discover others"

mod discovery;
mod error;
mod provider;
mod serialize;

// Re-export public API for backward compatibility
pub use discovery::discover_all;
pub use error::{CapabilityError, Result};
pub use provider::CapabilityProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primal_identity::{Capability, CryptoCapability};
    use std::path::PathBuf;

    fn run_async<F: std::future::Future<Output = O> + Send, O: Send>(
        f: impl FnOnce() -> F + Send,
    ) -> O {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        rt.block_on(f())
    }

    #[tokio::test]
    async fn test_capability_provider_structure() {
        let provider = CapabilityProvider::from_service_info(
            "test-provider".to_string(),
            PathBuf::from("/tmp/test.sock"),
            vec![Capability::Crypto(CryptoCapability::Encryption)],
        );

        assert_eq!(provider.service_name(), "test-provider");
        assert_eq!(provider.capabilities().len(), 1);
    }

    #[test]
    fn test_capability_serialization() {
        let cap = Capability::Crypto(CryptoCapability::Encryption);
        let s = serialize::capability_to_string(&cap);
        assert_eq!(s, "crypto");

        let cap2 = serialize::string_to_capability("crypto");
        assert_eq!(serialize::capability_to_string(&cap2), "crypto");
    }

    #[test]
    fn test_capability_serialization_all_variants() {
        use crate::primal_identity::*;

        let compute = Capability::Compute(ComputeCapability::NativeExecution);
        assert_eq!(serialize::capability_to_string(&compute), "compute");

        let storage = Capability::Storage(StorageCapability::ObjectStorage);
        assert_eq!(serialize::capability_to_string(&storage), "storage");

        let crypto = Capability::Crypto(CryptoCapability::Encryption);
        assert_eq!(serialize::capability_to_string(&crypto), "crypto");

        let auth = Capability::Authentication(AuthCapability::TokenManagement);
        assert_eq!(serialize::capability_to_string(&auth), "authentication");

        let coord = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
        assert_eq!(serialize::capability_to_string(&coord), "coordination");

        let disc = Capability::Discovery(DiscoveryCapability::RegistryDiscovery);
        assert_eq!(serialize::capability_to_string(&disc), "discovery");

        let custom = Capability::Custom {
            name: "custom_cap".to_string(),
            version: "1.0".to_string(),
        };
        assert_eq!(serialize::capability_to_string(&custom), "custom_cap");
    }

    #[test]
    fn test_string_to_capability_all_variants() {
        let compute = serialize::string_to_capability("compute");
        assert_eq!(serialize::capability_to_string(&compute), "compute");

        let storage = serialize::string_to_capability("storage");
        assert_eq!(serialize::capability_to_string(&storage), "storage");

        let crypto = serialize::string_to_capability("crypto");
        assert_eq!(serialize::capability_to_string(&crypto), "crypto");

        let auth1 = serialize::string_to_capability("authentication");
        assert_eq!(serialize::capability_to_string(&auth1), "authentication");

        let auth2 = serialize::string_to_capability("security");
        assert_eq!(serialize::capability_to_string(&auth2), "authentication");

        let coord = serialize::string_to_capability("coordination");
        assert_eq!(serialize::capability_to_string(&coord), "coordination");

        let disc = serialize::string_to_capability("discovery");
        assert_eq!(serialize::capability_to_string(&disc), "discovery");

        let custom = serialize::string_to_capability("unknown_capability");
        assert_eq!(
            serialize::capability_to_string(&custom),
            "unknown_capability"
        );
    }

    #[test]
    fn test_capability_error_variants() {
        use crate::primal_identity::CryptoCapability;

        let err1 =
            CapabilityError::NoProviderFound(Capability::Crypto(CryptoCapability::Encryption));
        assert!(err1.to_string().contains("No provider found"));

        let err2 = CapabilityError::ProviderUnreachable("test-service".to_string());
        assert!(err2.to_string().contains("test-service"));

        let err3 = CapabilityError::RpcFailed("connection timeout".to_string());
        assert!(err3.to_string().contains("connection timeout"));

        let err4 = CapabilityError::DiscoveryUnavailable;
        assert!(err4.to_string().contains("unavailable"));

        let err5 = CapabilityError::InvalidResponse("malformed json".to_string());
        assert!(err5.to_string().contains("malformed json"));
    }

    #[tokio::test]
    async fn test_has_capability() {
        use crate::primal_identity::*;

        let provider = CapabilityProvider::from_service_info(
            "test-provider".to_string(),
            PathBuf::from("/tmp/test.sock"),
            vec![
                Capability::Crypto(CryptoCapability::Encryption),
                Capability::Crypto(CryptoCapability::KeyManagement),
            ],
        );

        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::KeyManagement)));
        assert!(!provider.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
    }

    #[tokio::test]
    async fn test_capabilities_getter() {
        use crate::primal_identity::*;

        let caps = vec![
            Capability::Crypto(CryptoCapability::Encryption),
            Capability::Storage(StorageCapability::ObjectStorage),
        ];

        let provider = CapabilityProvider::from_service_info(
            "multi-provider".to_string(),
            PathBuf::from("/tmp/multi.sock"),
            caps.clone(),
        );

        let retrieved_caps = provider.capabilities();
        assert_eq!(retrieved_caps.len(), 2);
        assert_eq!(retrieved_caps, &caps[..]);
    }

    #[tokio::test]
    async fn test_service_name_getter() {
        let provider = CapabilityProvider::from_service_info(
            "my-service".to_string(),
            PathBuf::from("/tmp/service.sock"),
            vec![],
        );

        assert_eq!(provider.service_name(), "my-service");
    }

    #[test]
    fn test_custom_capability_roundtrip() {
        let custom = Capability::Custom {
            name: "my_custom_cap".to_string(),
            version: "2.0".to_string(),
        };

        let serialized = serialize::capability_to_string(&custom);
        assert_eq!(serialized, "my_custom_cap");

        let deserialized = serialize::string_to_capability(&serialized);
        match deserialized {
            Capability::Custom { name, .. } => assert_eq!(name, "my_custom_cap"),
            _ => panic!("Expected Custom capability"),
        }
    }

    #[test]
    fn test_capability_error_debug() {
        let err = CapabilityError::DiscoveryUnavailable;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("DiscoveryUnavailable"));
    }

    #[tokio::test]
    async fn test_provider_clone() {
        use crate::primal_identity::CryptoCapability;

        let provider1 = CapabilityProvider::from_service_info(
            "original".to_string(),
            PathBuf::from("/tmp/orig.sock"),
            vec![Capability::Crypto(CryptoCapability::Encryption)],
        );

        let provider2 = provider1.clone();
        assert_eq!(provider1.service_name(), provider2.service_name());
        assert_eq!(provider1.capabilities(), provider2.capabilities());
    }

    #[test]
    fn test_discover_fails_when_socket_unavailable() {
        temp_env::with_var(
            "SONGBIRD_SOCKET",
            Some("/tmp/nonexistent_toadstool_test_12345.sock"),
            || {
                let result = run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                });
                assert!(result.is_err());
                assert!(matches!(
                    result.unwrap_err(),
                    CapabilityError::DiscoveryUnavailable
                ));
            },
        );
    }

    #[test]
    fn test_discover_all_fails_when_socket_unavailable() {
        temp_env::with_var(
            "SONGBIRD_SOCKET",
            Some("/tmp/nonexistent_toadstool_test_67890.sock"),
            || {
                let result =
                    run_async(|| discover_all(Capability::Crypto(CryptoCapability::Encryption)));
                assert!(result.is_err());
                assert!(matches!(
                    result.unwrap_err(),
                    CapabilityError::DiscoveryUnavailable
                ));
            },
        );
    }

    #[tokio::test]
    async fn test_call_fails_when_socket_unavailable() {
        let provider = CapabilityProvider::from_service_info(
            "unreachable".to_string(),
            PathBuf::from("/tmp/nonexistent_toadstool_call_test.sock"),
            vec![Capability::Crypto(CryptoCapability::Encryption)],
        );

        let result = provider.call("test.method", serde_json::json!({})).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::RpcFailed(_)));
    }

    #[allow(clippy::unused_async)]
    async fn spawn_mock_discovery_server(
        result: serde_json::Value,
    ) -> (PathBuf, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixListener;

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        let socket_path = std::env::temp_dir().join(format!(
            "toadstool_cap_test_{}_{}.sock",
            std::process::id(),
            nanos
        ));
        let _ = std::fs::remove_file(&socket_path);

        let listener = UnixListener::bind(&socket_path).expect("bind mock socket");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let _ = reader.read_line(&mut line).await;

            let id = serde_json::from_str::<serde_json::Value>(&line)
                .ok()
                .and_then(|r| r.get("id").cloned())
                .unwrap_or_else(|| serde_json::json!(1));

            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            });
            let resp_line = format!("{}\n", serde_json::to_string(&response).unwrap());
            let _ = writer.write_all(resp_line.as_bytes()).await;
            let _ = writer.flush().await;
        });

        (socket_path, handle)
    }

    #[tokio::test]
    async fn test_discover_success() {
        let result = serde_json::json!({
            "services": [{
                "name": "beardog",
                "endpoint": "/tmp/beardog.sock",
                "capabilities": ["crypto", "encryption"]
            }]
        });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let provider = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                })
            })
        })
        .await
        .expect("spawn_blocking")
        .expect("discover should succeed");
        std::fs::remove_file(&socket_path).ok();

        assert_eq!(provider.service_name(), "beardog");
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    }

    #[tokio::test]
    async fn test_discover_no_provider_found() {
        let result = serde_json::json!({ "services": [] });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                })
            })
        })
        .await
        .expect("spawn_blocking")
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::NoProviderFound(_)));
    }

    #[tokio::test]
    async fn test_discover_invalid_response_no_services_array() {
        let result = serde_json::json!({ "not_services": [] });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                })
            })
        })
        .await
        .expect("spawn_blocking")
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::InvalidResponse(_)));
        assert!(err.to_string().contains("No services array"));
    }

    #[tokio::test]
    async fn test_discover_invalid_response_no_name() {
        let result = serde_json::json!({
            "services": [{ "endpoint": "/tmp/x.sock", "capabilities": [] }]
        });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                })
            })
        })
        .await
        .expect("spawn_blocking")
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::InvalidResponse(_)));
        assert!(err.to_string().contains("No name field"));
    }

    #[tokio::test]
    async fn test_discover_invalid_response_no_endpoint() {
        let result = serde_json::json!({
            "services": [{ "name": "beardog", "capabilities": [] }]
        });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                })
            })
        })
        .await
        .expect("spawn_blocking")
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::InvalidResponse(_)));
        assert!(err.to_string().contains("No endpoint field"));
    }

    #[tokio::test]
    async fn test_discover_all_success() {
        let result = serde_json::json!({
            "services": [
                { "name": "beardog1", "endpoint": "/tmp/b1.sock", "capabilities": ["crypto"] },
                { "name": "beardog2", "endpoint": "/tmp/b2.sock", "capabilities": ["crypto"] }
            ]
        });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let providers = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| discover_all(Capability::Crypto(CryptoCapability::Encryption)))
                    .expect("discover_all should succeed")
            })
        })
        .await
        .expect("spawn_blocking");
        std::fs::remove_file(&socket_path).ok();

        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].service_name(), "beardog1");
        assert_eq!(providers[1].service_name(), "beardog2");
    }

    #[tokio::test]
    async fn test_discover_capabilities_from_service() {
        let result = serde_json::json!({
            "services": [{
                "name": "beardog",
                "endpoint": "/tmp/beardog.sock",
                "capabilities": ["crypto", "authentication", "custom_cap"]
            }]
        });
        let (socket_path, _server) = spawn_mock_discovery_server(result).await;
        let path_str = socket_path.to_str().unwrap().to_string();
        let provider = tokio::task::spawn_blocking(move || {
            temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
                run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                })
                .expect("discover should succeed")
            })
        })
        .await
        .expect("spawn_blocking");
        std::fs::remove_file(&socket_path).ok();

        let caps = provider.capabilities();
        assert_eq!(caps.len(), 3);
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    }

    #[test]
    fn test_discover_default_socket_path() {
        temp_env::with_var_unset("SONGBIRD_SOCKET", || {
            let result = run_async(|| {
                CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
            });
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                CapabilityError::DiscoveryUnavailable
            ));
        });
    }
}
