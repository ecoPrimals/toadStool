// SPDX-License-Identifier: AGPL-3.0-or-later
//! Encrypt/decrypt JSON-RPC operations for [`CryptoServiceClient`](super::CryptoServiceClient).

#[cfg(unix)]
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};
#[cfg(not(unix))]
use toadstool_common::{ToadStoolError, ToadStoolResult};

use crate::crypto_integration::types::{CryptoRequest, CryptoResponse};

use super::CryptoServiceClient;

impl CryptoServiceClient {
    /// Encrypt data via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn encrypt(&self, request: CryptoRequest) -> ToadStoolResult<CryptoResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("crypto.encrypt", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto encrypt timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto encrypt failed: {e}"),
                })
            })
        }
    }

    /// Decrypt data via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn decrypt(&self, request: CryptoRequest) -> ToadStoolResult<CryptoResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("crypto.decrypt", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto decrypt timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto decrypt failed: {e}"),
                })
            })
        }
    }
}
