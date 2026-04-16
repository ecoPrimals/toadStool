// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connect and construct [`super::DisplayClient`] instances.

use std::path::PathBuf;

use super::{AsyncStreamDispatch, DisplayClient, IpcEndpoint};
use crate::DisplayError;
use tokio::net::{TcpStream, UnixStream};

impl DisplayClient {
    /// Discover and connect to display server (ISOMORPHIC!)
    ///
    /// **Zero Configuration**: Automatically discovers Unix socket OR TCP endpoint!
    ///
    /// ## Behavior
    ///
    /// 1. Try Unix socket paths (optimal)
    /// 2. Try TCP discovery file (fallback)
    /// 3. Connect via discovered endpoint
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::ipc::DisplayClient;
    /// # async fn example() -> toadstool_display::Result<()> {
    /// let mut client = DisplayClient::discover().await?;
    /// // Works on Linux (Unix) AND Android (TCP)!
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if no display server endpoint can be discovered or connection fails.
    pub async fn discover() -> crate::Result<Self> {
        tracing::info!("🔍 Discovering display server endpoint (isomorphic mode)...");

        let endpoint = Self::discover_endpoint()?;

        match &endpoint {
            IpcEndpoint::UnixSocket(path) => {
                tracing::info!("   Found Unix socket: {}", path.display());
            }
            IpcEndpoint::TcpLocal(addr) => {
                tracing::info!("   Found TCP endpoint: {}", addr);
            }
        }

        Self::connect_endpoint(endpoint).await
    }

    /// Connect to endpoint (polymorphic!)
    ///
    /// **Works with both Unix and TCP transparently**
    pub(super) async fn connect_endpoint(endpoint: IpcEndpoint) -> crate::Result<Self> {
        match &endpoint {
            IpcEndpoint::UnixSocket(path) => {
                tracing::info!("🔌 Connecting via Unix socket...");

                let stream = UnixStream::connect(path)
                    .await
                    .map_err(|e| DisplayError::IpcError(format!("Unix connection failed: {e}")))?;

                tracing::info!("✅ Connected to display server (Unix socket)");

                Ok(Self {
                    stream: AsyncStreamDispatch::Unix(stream),
                    endpoint,
                })
            }
            IpcEndpoint::TcpLocal(addr) => {
                tracing::info!("🌐 Connecting via TCP fallback...");

                let stream = TcpStream::connect(addr)
                    .await
                    .map_err(|e| DisplayError::IpcError(format!("TCP connection failed: {e}")))?;

                tracing::info!("✅ Connected to display server (TCP fallback)");

                Ok(Self {
                    stream: AsyncStreamDispatch::Tcp(stream),
                    endpoint,
                })
            }
        }
    }

    /// Connect to specific path (backward compatibility)
    ///
    /// **Legacy method**: Use `discover()` for automatic discovery!
    ///
    /// # Errors
    ///
    /// Returns an error if connection to the given path fails.
    pub async fn connect(path: impl Into<PathBuf>) -> crate::Result<Self> {
        let path = path.into();
        tracing::info!("Connecting to display server: {}", path.display());

        let stream = UnixStream::connect(&path)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Connection failed: {e}")))?;

        tracing::info!("✅ Connected to display server");

        Ok(Self {
            stream: AsyncStreamDispatch::Unix(stream),
            endpoint: IpcEndpoint::UnixSocket(path),
        })
    }

    /// Get connected endpoint
    #[must_use]
    pub const fn endpoint(&self) -> &IpcEndpoint {
        &self.endpoint
    }

    /// Create client with mock stream for testing (no real connection)
    #[cfg(test)]
    #[must_use]
    pub fn new_for_test(stream: tokio::io::DuplexStream, endpoint: IpcEndpoint) -> Self {
        Self {
            stream: AsyncStreamDispatch::Duplex(stream),
            endpoint,
        }
    }
}
