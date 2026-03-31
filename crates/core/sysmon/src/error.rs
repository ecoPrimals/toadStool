// SPDX-License-Identifier: AGPL-3.0-only
//! Error types for toadstool-sysmon.

/// All sysmon operations are `/proc` I/O; errors are always I/O errors
/// with context about which `/proc` path failed.
#[derive(Debug, thiserror::Error)]
#[error("sysmon: failed to read {path}: {source}")]
pub struct SysmonError {
    /// The `/proc` or sysfs path that failed to read.
    pub path: &'static str,
    /// The underlying I/O error.
    #[source]
    pub source: std::io::Error,
}

impl SysmonError {
    pub(crate) const fn new(path: &'static str, source: std::io::Error) -> Self {
        Self { path, source }
    }
}

/// Result type for sysmon operations; errors carry the path that failed.
pub type Result<T> = std::result::Result<T, SysmonError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_sysmon_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "No such file");
        let err = SysmonError::new("/proc/foo", io_err);
        let display = err.to_string();
        assert!(display.contains("sysmon"));
        assert!(display.contains("/proc/foo"));
        assert!(display.contains("No such file"));
    }

    #[test]
    fn test_sysmon_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = SysmonError::new("/proc/stat", io_err);
        let source = err.source().expect("should have source");
        assert_eq!(source.to_string(), "access denied");
    }

    #[test]
    fn test_sysmon_error_is_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<SysmonError>();
    }
}
