//! Abstract Unix socket implementation for Android/Linux
//!
//! **Deep Debt Principles**:
//! - ✅ Safe Rust (no unsafe, no libc FFI)
//! - ✅ Android-first (SELinux-friendly)
//! - ✅ Modern async (tokio patterns)
//! - ✅ Capability-based (runtime detection)
//!
//! ## Abstract Sockets
//!
//! Linux-specific socket namespace that doesn't create filesystem entries.
//! Name starts with null byte (`\0`), making it invisible to filesystem.
//!
//! **Benefits**:
//! - ✅ No filesystem permissions (SELinux-friendly)
//! - ✅ Automatic cleanup (kernel manages)
//! - ✅ Namespace isolation
//! - ✅ Perfect for Android
//!
//! **biomeOS Standard**: `@biomeos_<primal>` (@ represents null byte)

use crate::{ToadStoolError, ToadStoolResult};
use tokio::net::{UnixListener, UnixStream};

/// Bind abstract Unix socket
///
/// **Deep Debt**: Safe Rust using std::os::unix SocketAddr!
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc::platform::abstract_socket;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let listener = abstract_socket::bind("@biomeos_toadstool").await?;
///     Ok(())
/// }
/// ```
pub async fn bind(name: &str) -> ToadStoolResult<UnixListener> {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::SocketAddr; // Linux-specific extension trait

    // Ensure name starts with @ (convention for null byte)
    let abstract_name = if name.starts_with('@') {
        &name[1..] // Remove @ prefix
    } else {
        name
    };

    // Create abstract socket address using Linux extension
    let socket_addr = SocketAddr::from_abstract_name(abstract_name.as_bytes()).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to create abstract socket address for @{}: {}",
            abstract_name, e
        ))
    })?;

    // Create socket
    let socket = std::os::unix::net::UnixListener::bind_addr(&socket_addr).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to bind abstract socket @{}: {}",
            abstract_name, e
        ))
    })?;

    // Set non-blocking for tokio
    socket.set_nonblocking(true).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to set non-blocking on abstract socket: {}",
            e
        ))
    })?;

    // Convert to tokio::net::UnixListener
    UnixListener::from_std(socket).map_err(|e| {
        ToadStoolError::integration(format!("Failed to convert abstract socket to tokio: {}", e))
    })
}

/// Connect to abstract Unix socket
///
/// **Deep Debt**: Safe async using SocketAddr!
pub async fn connect(name: &str) -> ToadStoolResult<UnixStream> {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::SocketAddr; // Linux-specific extension trait

    // Ensure name starts with @ (convention for null byte)
    let abstract_name = if name.starts_with('@') {
        &name[1..] // Remove @ prefix
    } else {
        name
    };

    // Create abstract socket address using Linux extension
    let socket_addr = SocketAddr::from_abstract_name(abstract_name.as_bytes()).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to create abstract socket address for @{}: {}",
            abstract_name, e
        ))
    })?;

    // Connect using std socket first
    let std_stream = std::os::unix::net::UnixStream::connect_addr(&socket_addr).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to connect to abstract socket @{}: {}",
            abstract_name, e
        ))
    })?;

    // Set non-blocking for tokio
    std_stream.set_nonblocking(true).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to set non-blocking on abstract socket: {}",
            e
        ))
    })?;

    // Convert to tokio::net::UnixStream
    UnixStream::from_std(std_stream).map_err(|e| {
        ToadStoolError::integration(format!("Failed to convert abstract socket to tokio: {}", e))
    })
}

/// Get default ToadStool abstract socket name
///
/// **Deep Debt**: biomeOS standard naming
pub fn default_name() -> String {
    "@biomeos_toadstool".to_string()
}

/// Check if abstract sockets are supported
///
/// **Deep Debt**: Runtime capability detection
pub fn is_supported() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_name() {
        let name = default_name();
        assert_eq!(name, "@biomeos_toadstool");
        assert!(name.starts_with('@'));
    }

    #[test]
    fn test_is_supported() {
        // Abstract sockets are Linux-only
        #[cfg(target_os = "linux")]
        assert!(is_supported());

        #[cfg(not(target_os = "linux"))]
        assert!(!is_supported());
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_bind_and_connect() {
        let test_name = "@toadstool_test_abstract";

        // Bind (no filesystem entry created!)
        let listener = bind(test_name).await.unwrap();

        // Connect
        let stream = connect(test_name).await.unwrap();

        // Cleanup (automatic by kernel, but drop for good practice)
        drop(listener);
        drop(stream);

        // No filesystem cleanup needed - that's the beauty of abstract sockets!
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_name_with_at_symbol() {
        let test_name = "@toadstool_test_at";

        // Should work with @ prefix
        let listener = bind(test_name).await.unwrap();
        let stream = connect(test_name).await.unwrap();

        drop(listener);
        drop(stream);
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_name_without_at_symbol() {
        let test_name = "toadstool_test_no_at";

        // Should add @ prefix automatically
        let listener = bind(test_name).await.unwrap();
        let stream = connect(test_name).await.unwrap();

        drop(listener);
        drop(stream);
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_multiple_binds_same_name() {
        let test_name = "@toadstool_test_rebind";

        // First bind
        let listener1 = bind(test_name).await.unwrap();

        // Second bind should fail (already in use)
        let result = bind(test_name).await;
        assert!(result.is_err());

        // Cleanup
        drop(listener1);

        // Now should succeed
        let listener2 = bind(test_name).await.unwrap();
        drop(listener2);
    }
}
