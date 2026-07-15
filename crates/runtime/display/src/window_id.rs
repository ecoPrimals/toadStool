// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-platform window identifier (UUID-based).

use crate::{DisplayError, Result};
use std::fmt;

/// Window identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WindowId(uuid::Uuid);

impl WindowId {
    /// Create a new window ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Parse from string
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid UUID.
    pub fn from_string(s: &str) -> Result<Self> {
        uuid::Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| DisplayError::IpcError(format!("Invalid window ID: {e}")))
    }

    /// Convert to string
    ///
    /// Note: Also available via `Display` trait (`format!("{}", id)`)
    #[must_use]
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
