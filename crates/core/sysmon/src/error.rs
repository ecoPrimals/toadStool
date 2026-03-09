// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for toadstool-sysmon.

use std::fmt;

/// All sysmon operations are `/proc` I/O; errors are always I/O errors
/// with context about which `/proc` path failed.
#[derive(Debug)]
pub struct SysmonError {
    pub path: &'static str,
    pub source: std::io::Error,
}

impl fmt::Display for SysmonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sysmon: failed to read {}: {}", self.path, self.source)
    }
}

impl std::error::Error for SysmonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl SysmonError {
    pub(crate) fn new(path: &'static str, source: std::io::Error) -> Self {
        Self { path, source }
    }
}

pub type Result<T> = std::result::Result<T, SysmonError>;
