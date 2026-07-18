// SPDX-License-Identifier: AGPL-3.0-or-later
//! Signature and permission JSON-RPC operations for [`CryptoServiceClient`](super::CryptoServiceClient).

#[cfg(unix)]
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};
#[cfg(not(unix))]
use toadstool_common::{ToadStoolError, ToadStoolResult};

use crate::crypto_integration::types::{
    PermissionResponse, RevocationRequest, SignatureRequest, SignatureResponse, ValidationResponse,
    VerificationRequest, VerificationResponse,
};

use super::CryptoServiceClient;

impl CryptoServiceClient {
    /// Sign data via unix socket
    pub async fn sign(&self, data: &[u8]) -> ToadStoolResult<SignatureResponse> {
        #[cfg(not(unix))]
        {
            let _ = data;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let request = SignatureRequest {
                request_id: uuid::Uuid::new_v4(),
                data: data.to_vec(),
                key_id: None,
                algorithm: None,
            };

            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("crypto.sign", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto sign timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto sign failed: {e}"),
                })
            })
        }
    }

    /// Verify signature via unix socket
    pub async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<bool> {
        #[cfg(not(unix))]
        {
            let _ = (data, signature, public_key_id);
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let request = VerificationRequest {
                request_id: uuid::Uuid::new_v4(),
                data: data.to_vec(),
                signature: signature.to_vec(),
                public_key_id: public_key_id.to_string(),
            };

            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            let result: VerificationResponse = tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("crypto.verify", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto verify timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto verify failed: {e}"),
                })
            })?;

            Ok(result.valid)
        }
    }

    /// Create permission via unix socket
    pub async fn create_permission(
        &self,
        request: &crate::security_provider::PermissionRequest,
    ) -> ToadStoolResult<PermissionResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("crypto.create_permission", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission creation timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission creation failed: {e}"),
                })
            })
        }
    }

    /// Validate permission via unix socket
    pub async fn validate_permission(
        &self,
        permission: &crate::security_provider::SecurityPermission,
    ) -> ToadStoolResult<bool> {
        #[cfg(not(unix))]
        {
            let _ = permission;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(permission).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            let result: ValidationResponse = tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("crypto.validate_permission", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission validation timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission validation failed: {e}"),
                })
            })?;

            Ok(result.valid)
        }
    }

    /// Revoke permission via unix socket
    pub async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        #[cfg(not(unix))]
        {
            let _ = (permission_id, reason);
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let request = RevocationRequest {
                reason: reason.to_string(),
            };

            let mut params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            if let Some(obj) = params.as_object_mut() {
                obj.insert(
                    "permission_id".to_string(),
                    serde_json::json!(permission_id.to_string()),
                );
            }

            let _: serde_json::Value = tokio::time::timeout(
                self.timeout,
                self.rpc_client.call("crypto.revoke_permission", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission revocation timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission revocation failed: {e}"),
                })
            })?;

            Ok(())
        }
    }
}
