// SPDX-License-Identifier: AGPL-3.0-only
//! OS-native keyring integration for credential resolution.
//!
//! Provides platform-specific secret storage access:
//! - **Linux**: D-Bus SecretService API via `secret-tool` (GNOME Keyring, KWallet)
//! - **macOS**: Security.framework Keychain via `security` CLI
//! - **Other**: Falls back to `None`
//!
//! This is step 2.5 in the credential resolution chain, between file-based
//! credentials and the security provider JSON-RPC fallback.

use crate::secret_string::SecretString;

const SERVICE_NAME: &str = "toadstool";

/// Query the OS keyring for a named credential.
///
/// Returns `None` if:
/// - The platform has no keyring support
/// - The keyring daemon is not running
/// - The credential is not stored
/// - Access was denied
pub fn query_os_keyring(name: &str) -> Option<SecretString> {
    #[cfg(target_os = "linux")]
    {
        query_linux_secret_service(name)
    }
    #[cfg(target_os = "macos")]
    {
        query_macos_keychain(name)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = name;
        None
    }
}

/// Store a credential in the OS keyring.
///
/// Returns `true` if storage succeeded.
pub fn store_os_keyring(name: &str, value: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        store_linux_secret_service(name, value)
    }
    #[cfg(target_os = "macos")]
    {
        store_macos_keychain(name, value)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (name, value);
        false
    }
}

/// Delete a credential from the OS keyring.
pub fn delete_os_keyring(name: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        delete_linux_secret_service(name)
    }
    #[cfg(target_os = "macos")]
    {
        delete_macos_keychain(name)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = name;
        false
    }
}

/// Whether the OS keyring backend is available on this platform.
pub fn os_keyring_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        linux_secret_service_available()
    }
    #[cfg(target_os = "macos")]
    {
        true // macOS Keychain is always available
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn linux_secret_service_available() -> bool {
    std::process::Command::new("secret-tool")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

#[cfg(target_os = "linux")]
fn query_linux_secret_service(name: &str) -> Option<SecretString> {
    let output = std::process::Command::new("secret-tool")
        .args(["lookup", "service", SERVICE_NAME, "credential", name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let secret = String::from_utf8(output.stdout).ok()?;
    let trimmed = secret.trim_end_matches('\n');
    if trimmed.is_empty() {
        return None;
    }
    Some(SecretString::new(trimmed.to_owned()))
}

#[cfg(target_os = "linux")]
fn store_linux_secret_service(name: &str, value: &str) -> bool {
    let mut child = match std::process::Command::new("secret-tool")
        .args([
            "store",
            "--label",
            &format!("toadStool: {name}"),
            "service",
            SERVICE_NAME,
            "credential",
            name,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        let _ = stdin.write_all(value.as_bytes());
    }

    child.wait().map(|s| s.success()).unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn delete_linux_secret_service(name: &str) -> bool {
    std::process::Command::new("secret-tool")
        .args(["clear", "service", SERVICE_NAME, "credential", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn query_macos_keychain(name: &str) -> Option<SecretString> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            SERVICE_NAME,
            "-a",
            name,
            "-w", // output password only
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let secret = String::from_utf8(output.stdout).ok()?;
    let trimmed = secret.trim_end_matches('\n');
    if trimmed.is_empty() {
        return None;
    }
    Some(SecretString::new(trimmed.to_owned()))
}

#[cfg(target_os = "macos")]
fn store_macos_keychain(name: &str, value: &str) -> bool {
    // Delete existing entry first (update isn't atomic)
    let _ = delete_macos_keychain(name);

    std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            SERVICE_NAME,
            "-a",
            name,
            "-w",
            value,
            "-U", // update if exists
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn delete_macos_keychain(name: &str) -> bool {
    std::process::Command::new("security")
        .args(["delete-generic-password", "-s", SERVICE_NAME, "-a", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_keyring_available_returns_bool() {
        let result = os_keyring_available();
        // Can't assert specific value (platform-dependent)
        let _ = result;
    }

    #[test]
    fn query_nonexistent_returns_none() {
        let result = query_os_keyring("__toadstool_nonexistent_cred_xyz_12345__");
        assert!(result.is_none());
    }

    #[test]
    fn store_query_roundtrip_when_available() {
        if !os_keyring_available() {
            return;
        }
        let name = format!("__toadstool_test_roundtrip_{}__", std::process::id());
        let value = "roundtrip-secret-42";
        assert!(store_os_keyring(&name, value));
        let retrieved = query_os_keyring(&name);
        let _ = delete_os_keyring(&name); // cleanup
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().expose_secret(), value);
    }

    #[test]
    fn delete_nonexistent_returns_gracefully() {
        let result = delete_os_keyring("__toadstool_nonexistent_delete_xyz_12345__");
        // Should not panic; on Linux/macOS may return true or false
        let _ = result;
    }
}
