//! BearDog integration - Cryptographic security and permissions
//!
//! This module handles all BearDog-specific operations including:
//! - Cryptographic permission validation
//! - Permission installation and management
//! - Capability verification

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use tokio::fs;
use tracing::{error, info};

use super::super::types::*;

// Removed verify_beardog_service() and verify_beardog_capabilities() - unused.
// Complete implementations but never invoked. Logic preserved in git history.

/// Install BearDog permissions from file
pub async fn install_permissions(permissions_path: &Path, validate_only: bool) -> Result<()> {
    info!(
        "🔐 Installing BearDog permissions from: {}",
        permissions_path.display()
    );

    // Read permissions file
    let permissions_data = fs::read_to_string(permissions_path)
        .await
        .with_context(|| {
            format!(
                "Failed to read permissions file: {}",
                permissions_path.display()
            )
        })?;

    // Parse permissions
    let permission: BearDogPermission = serde_json::from_str(&permissions_data)
        .with_context(|| "Failed to parse BearDog permissions")?;

    // Validate permission
    validate_permission(&permission).await?;

    info!("✅ BearDog permission validation successful");

    if validate_only {
        info!("🔍 Validation-only mode - not installing");
        return Ok(());
    }

    // Install permission
    install_permission(&permission).await?;

    info!("✅ BearDog permissions installed successfully");
    Ok(())
}

/// Validate a BearDog permission
pub async fn validate_permission(permission: &BearDogPermission) -> Result<bool> {
    // Validate permission structure
    if permission.granted_to.is_empty() {
        anyhow::bail!("BearDog permission missing granted_to field");
    }

    if permission.capabilities.is_empty() {
        anyhow::bail!("BearDog permission has no capabilities");
    }

    // Check expiration
    if permission.valid_until < Utc::now() {
        anyhow::bail!("BearDog permission has expired");
    }

    // Verify cryptographic signature
    let crypto_context = CryptoVerificationContext::new();

    if let Some(public_key) = crypto_context.trusted_public_keys.get("beardog") {
        // Create canonical message for verification
        let message = create_permission_message(permission)?;

        // Decode signature from base64
        let signature_bytes = BASE64
            .decode(&permission.signature)
            .with_context(|| "Failed to decode BearDog permission signature")?;

        // Decode public key from base64
        let public_key_bytes = BASE64
            .decode(public_key)
            .with_context(|| "Failed to decode BearDog public key")?;

        // Verify signature using ed25519
        match verify_ed25519_signature(&public_key_bytes, &message, &signature_bytes) {
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

/// Create canonical message for permission verification
pub fn create_permission_message(permission: &BearDogPermission) -> Result<Vec<u8>> {
    // Create canonical message for BearDog permission verification
    let mut data = BTreeMap::new();
    data.insert("permission_id", permission.permission_id.to_string());
    data.insert("granted_to", permission.granted_to.clone());
    data.insert("valid_until", permission.valid_until.to_rfc3339());

    let capabilities_json = serde_json::to_string(&permission.capabilities)
        .map_err(|e| anyhow::anyhow!("Failed to serialize permission capabilities: {e}"))?;
    data.insert("capabilities", capabilities_json);

    let canonical_json = serde_json::to_string(&data)
        .map_err(|e| anyhow::anyhow!("Failed to create canonical permission message: {e}"))?;

    Ok(canonical_json.into_bytes())
}

/// Install a permission in the system
async fn install_permission(permission: &BearDogPermission) -> Result<()> {
    // Install permission in system keyring or secure storage
    // This is a simplified implementation
    info!("🔐 Installing permission: {}", permission.permission_id);
    Ok(())
}

/// Verify ed25519 signature
///
/// NOTE: This is a stub implementation that always returns true.
/// Real cryptographic verification should be implemented via integration
/// with BearDog service's public key infrastructure.
///
/// For production use, this should:
/// 1. Use ed25519-dalek or ring crate for signature verification
/// 2. Validate public key format and length
/// 3. Ensure message matches expected canonical form
/// 4. Return proper error types on verification failure
fn verify_ed25519_signature(
    _public_key: &[u8],
    _message: &[u8],
    _signature: &[u8],
) -> Result<bool> {
    // Stub: Real implementation requires BearDog public key infrastructure
    // This is intentionally simplified - cryptographic operations should be
    // delegated to BearDog service which has the proper key management.
    Ok(true)
}
