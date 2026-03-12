// SPDX-License-Identifier: AGPL-3.0-only
//! Zero-leakage secret value wrapper.
//!
//! [`SecretString`] holds credentials, tokens, and API keys in memory with
//! guarantees against accidental exposure:
//!
//! - **Debug / Display** print `[REDACTED]`, never the raw value.
//! - **Serialize** emits `"[REDACTED]"` — secrets cannot be round-tripped
//!   through JSON config files.
//! - **Drop** zeroizes the backing allocation via [`zeroize::Zeroize`].
//! - **Clone** is intentionally omitted; pass by reference or `Arc<SecretString>`.
//!
//! ## Credential Resolution
//!
//! Use [`resolve_credential`] to obtain secrets at runtime through the
//! standard chain: **environment variable → OS keyring → BearDog delegation**.
//! This eliminates every reason to hardcode a secret in source.
//!
//! ```rust,ignore
//! use toadstool_common::secret_string::resolve_credential;
//!
//! let token = resolve_credential("HUGGINGFACE_TOKEN").await?;
//! // token is SecretString — cannot be logged, serialized, or cloned.
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Opaque wrapper around a secret value.
///
/// The inner `String` is zeroized on drop and never exposed through
/// `Debug`, `Display`, or `Serialize`. `Clone` is available because
/// credential structs are often passed through config layers, but each
/// clone is independently zeroized on drop.
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a raw secret value.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Borrow the secret for use in API calls, headers, etc.
    ///
    /// Callers MUST NOT log, serialize, or persist the returned `&str`.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Returns `true` when the inner value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("[REDACTED]")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s.to_owned())
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecretString {}

// ---------------------------------------------------------------------------
// Credential resolution chain
// ---------------------------------------------------------------------------

/// Errors from credential resolution.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("credential '{name}' not found in environment, keyring, or BearDog")]
    NotFound { name: String },
}

/// Resolve a named credential through the standard chain:
///
/// 1. **Environment variable** (`std::env::var(name)`)
/// 2. **OS keyring** (future: D-Bus Secret Service / macOS Keychain)
/// 3. **BearDog security provider** (future: JSON-RPC delegation)
///
/// Returns [`CredentialError::NotFound`] when all sources are exhausted.
///
/// # Errors
///
/// Returns an error if the credential cannot be found in any source.
pub async fn resolve_credential(name: &str) -> Result<SecretString, CredentialError> {
    // 1. Environment variable — the standard, lowest-friction source.
    if let Ok(val) = std::env::var(name) {
        tracing::debug!(credential = name, source = "env", "credential resolved");
        return Ok(SecretString::new(val));
    }

    // 2. OS keyring (placeholder — wired when LocalKeyringProvider gains
    //    arbitrary-key lookup, tracked as D-KEYRING)
    if let Some(val) = probe_keyring(name) {
        tracing::debug!(credential = name, source = "keyring", "credential resolved");
        return Ok(val);
    }

    // 3. BearDog delegation (placeholder — wired when BearDog exposes a
    //    `secret.resolve` JSON-RPC method, tracked as D-BD-SECRET)
    if let Some(val) = probe_beardog(name).await {
        tracing::debug!(credential = name, source = "beardog", "credential resolved");
        return Ok(val);
    }

    tracing::warn!(
        credential = name,
        "credential not found in env, keyring, or beardog"
    );
    Err(CredentialError::NotFound {
        name: name.to_owned(),
    })
}

/// Keyring probe — returns `None` until `LocalKeyringProvider` supports
/// arbitrary secret lookup.
fn probe_keyring(_name: &str) -> Option<SecretString> {
    None
}

/// BearDog probe — returns `None` until BearDog exposes `secret.resolve`.
#[expect(
    clippy::unused_async,
    reason = "will become async when BearDog RPC is wired (D-BD-SECRET)"
)]
async fn probe_beardog(_name: &str) -> Option<SecretString> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = SecretString::new("super-secret".into());
        assert_eq!(format!("{s:?}"), "[REDACTED]");
    }

    #[test]
    fn display_is_redacted() {
        let s = SecretString::new("super-secret".into());
        assert_eq!(format!("{s}"), "[REDACTED]");
    }

    #[test]
    fn serialize_is_redacted() {
        let s = SecretString::new("super-secret".into());
        let json = serde_json::to_string(&s).expect("serialize");
        assert_eq!(json, "\"[REDACTED]\"");
    }

    #[test]
    fn deserialize_round_trip() {
        let json = "\"my-token\"";
        let s: SecretString = serde_json::from_str(json).expect("deserialize");
        assert_eq!(s.expose_secret(), "my-token");
    }

    #[test]
    fn expose_secret_returns_value() {
        let s = SecretString::new("abc".into());
        assert_eq!(s.expose_secret(), "abc");
    }

    #[test]
    fn is_empty_works() {
        assert!(SecretString::new(String::new()).is_empty());
        assert!(!SecretString::new("x".into()).is_empty());
    }

    #[test]
    fn from_str_works() {
        let s = SecretString::from("token");
        assert_eq!(s.expose_secret(), "token");
    }

    #[test]
    fn from_string_works() {
        let s = SecretString::from("token".to_owned());
        assert_eq!(s.expose_secret(), "token");
    }

    #[test]
    fn equality_works() {
        let a = SecretString::new("x".into());
        let b = SecretString::new("x".into());
        let c = SecretString::new("y".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn resolve_from_env() {
        temp_env::with_var("TEST_CRED_RESOLVE_SECRET_XYZ", Some("val123"), || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("rt");
            rt.block_on(async {
                let s = resolve_credential("TEST_CRED_RESOLVE_SECRET_XYZ")
                    .await
                    .expect("resolve");
                assert_eq!(s.expose_secret(), "val123");
            });
        });
    }

    #[test]
    fn resolve_not_found() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        rt.block_on(async {
            let result = resolve_credential("DEFINITELY_NOT_SET_XYZ_123").await;
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("DEFINITELY_NOT_SET_XYZ_123"));
        });
    }
}
