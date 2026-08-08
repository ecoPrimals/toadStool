// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-Unix/non-runtime stub for the Unix-only `unix_jsonrpc_client` module.
//!
//! Unix domain sockets are unavailable on Windows, WASM, and other non-Unix targets.
//! This module preserves the public API so cross-compiled crates compile; calls
//! fail at runtime with a clear platform error.

use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::{ToadStoolError, ToadStoolResult};

fn platform_unavailable() -> ToadStoolError {
    ToadStoolError::network("Unix domain sockets are not available on this platform")
}

/// Stub JSON-RPC client for non-Unix platforms.
#[derive(Debug, Clone)]
pub struct UnixJsonRpcClient {
    socket_path: PathBuf,
}

impl UnixJsonRpcClient {
    /// Create a stub client (calls will fail at runtime on non-Unix).
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /// Always fails on non-Unix platforms.
    #[allow(clippy::unused_async)]
    pub async fn call(&self, _method: &str, _params: Value) -> ToadStoolResult<Value> {
        let _ = &self.socket_path;
        Err(platform_unavailable())
    }
}

/// Stub persistent-connection client for non-Unix platforms.
#[derive(Debug)]
pub struct ConnectedJsonRpcClient;

impl ConnectedJsonRpcClient {
    /// Always fails on non-Unix platforms.
    #[allow(clippy::unused_async)]
    pub async fn connect(_socket_path: impl AsRef<Path>) -> ToadStoolResult<Self> {
        Err(platform_unavailable())
    }

    /// Always fails on non-Unix platforms.
    #[allow(clippy::unused_async)]
    pub async fn call(&mut self, _method: &str, _params: Value) -> ToadStoolResult<Value> {
        Err(platform_unavailable())
    }
}
