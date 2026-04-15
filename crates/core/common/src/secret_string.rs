// SPDX-License-Identifier: AGPL-3.0-or-later
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
//! standard chain: **environment variable → OS keyring → security provider**.
//! This eliminates every reason to hardcode a secret in source.
//!
//! ```rust,ignore
//! use toadstool_common::secret_string::resolve_credential;
//!
//! let token = resolve_credential("HUGGINGFACE_TOKEN").await?;
//! // token is SecretString — cannot be logged, serialized, or cloned.
//! ```

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::constants::PRIMAL_NAME;

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
    pub const fn new(value: String) -> Self {
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
    /// Credential not found in any source (env, keyring, security provider)
    #[error("credential '{name}' not found in environment, keyring, or security provider")]
    NotFound {
        /// Credential name that was requested
        name: String,
    },
}

/// Resolve a named credential through the standard chain:
///
/// 1. **Environment variable** (`std::env::var(name)`)
/// 2. **Credentials file** (`$XDG_CONFIG_HOME/toadstool/credentials`, 0600)
///    2.5. **OS keyring** (D-Bus `SecretService` on Linux, Keychain on macOS)
/// 3. **Security provider** (capability `crypto` → `secret.resolve` JSON-RPC)
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

    // 2. File-based credentials (`$XDG_CONFIG_HOME/toadstool/credentials`).
    //    Requires 0600 permissions. Format: KEY=VALUE per line.
    if let Some(val) = probe_keyring(name) {
        tracing::debug!(
            credential = name,
            source = "credentials_file",
            "credential resolved"
        );
        return Ok(val);
    }

    // 2.5. OS keyring (D-Bus SecretService on Linux, Keychain on macOS)
    if let Some(val) = crate::os_keyring::query_os_keyring(name) {
        tracing::debug!(
            credential = name,
            source = "os_keyring",
            "credential resolved"
        );
        return Ok(val);
    }

    // 3. Security provider delegation — discovers the `crypto` capability
    //    socket and calls `secret.resolve` over JSON-RPC.
    if let Some(val) = probe_security_provider(name).await {
        tracing::debug!(
            credential = name,
            source = "security_provider",
            "credential resolved"
        );
        return Ok(val);
    }

    tracing::warn!(
        credential = name,
        "credential not found in env, keyring, or security provider"
    );
    Err(CredentialError::NotFound {
        name: name.to_owned(),
    })
}

/// Resolve the credentials file path:
/// `$TOADSTOOL_CREDENTIALS` > `$XDG_CONFIG_HOME/toadstool/credentials`
/// > `$HOME/.config/toadstool/credentials`.
fn credentials_file_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TOADSTOOL_CREDENTIALS") {
        return Some(PathBuf::from(p));
    }
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok()?;
    Some(config_dir.join(PRIMAL_NAME).join("credentials"))
}

/// File-based credential lookup from the toadStool credentials file.
///
/// Format: one `KEY=VALUE` per line (shell-style, no quoting).
/// The file MUST have `0600` permissions or it is rejected.
fn probe_keyring(name: &str) -> Option<SecretString> {
    let path = credentials_file_path()?;
    if !path.is_file() {
        return None;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&path).ok()?;
        let mode = meta.permissions().mode() & 0o777;
        if mode != 0o600 {
            tracing::warn!(
                path = %path.display(),
                mode = format!("{mode:o}"),
                "credentials file has unsafe permissions (need 0600), skipping"
            );
            return None;
        }
    }

    let contents = std::fs::read_to_string(&path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == name {
                return Some(SecretString::new(value.trim().to_owned()));
            }
        }
    }
    None
}

/// Discover the security provider via capability and request credential
/// resolution over JSON-RPC (`secret.resolve`).
///
/// Returns `None` when the provider socket is absent or the call fails.
async fn probe_security_provider(name: &str) -> Option<SecretString> {
    let socket = crate::primal_sockets::get_socket_path_for_capability("crypto");
    if !socket.exists() {
        tracing::trace!(
            capability = "crypto",
            "security provider socket not present, skipping"
        );
        return None;
    }

    let client = crate::unix_jsonrpc_client::UnixJsonRpcClient::new(&socket);
    let params = serde_json::json!({ "name": name });

    match client.call("secret.resolve", params).await {
        Ok(val) => {
            let secret = val.as_str()?;
            Some(SecretString::new(secret.to_owned()))
        }
        Err(e) => {
            tracing::debug!(
                credential = name,
                error = %e,
                "security provider secret.resolve call failed"
            );
            None
        }
    }
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

    #[tokio::test]
    async fn resolve_from_env() {
        temp_env::async_with_vars([("TEST_CRED_RESOLVE_SECRET_XYZ", Some("val123"))], async {
            let s = resolve_credential("TEST_CRED_RESOLVE_SECRET_XYZ")
                .await
                .expect("resolve");
            assert_eq!(s.expose_secret(), "val123");
        })
        .await;
    }

    #[test]
    fn probe_keyring_reads_credentials_file() {
        let dir = std::env::temp_dir().join("toadstool_test_keyring");
        let _ = std::fs::create_dir_all(&dir);
        let cred_path = dir.join("credentials");
        std::fs::write(
            &cred_path,
            "# comment\nMY_TEST_KEY=secret_val_42\nOTHER=x\n",
        )
        .expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cred_path, std::fs::Permissions::from_mode(0o600))
                .expect("perms");
        }
        temp_env::with_var(
            "TOADSTOOL_CREDENTIALS",
            Some(cred_path.to_str().unwrap()),
            || {
                let val = probe_keyring("MY_TEST_KEY");
                assert!(val.is_some());
                assert_eq!(val.unwrap().expose_secret(), "secret_val_42");

                let missing = probe_keyring("NOPE");
                assert!(missing.is_none());
            },
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn probe_keyring_rejects_unsafe_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join("toadstool_test_keyring_perms");
        let _ = std::fs::create_dir_all(&dir);
        let cred_path = dir.join("credentials");
        std::fs::write(&cred_path, "KEY=val\n").expect("write");
        std::fs::set_permissions(&cred_path, std::fs::Permissions::from_mode(0o644))
            .expect("perms");
        temp_env::with_var(
            "TOADSTOOL_CREDENTIALS",
            Some(cred_path.to_str().unwrap()),
            || {
                assert!(probe_keyring("KEY").is_none());
            },
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn resolve_not_found() {
        let result = resolve_credential("DEFINITELY_NOT_SET_XYZ_123").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("DEFINITELY_NOT_SET_XYZ_123"));
    }
}
