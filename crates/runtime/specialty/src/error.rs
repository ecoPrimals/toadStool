// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool::{SystemError, ToadStoolError};

/// Errors that can occur during specialty runtime operations.
#[derive(Debug, thiserror::Error)]
pub enum SpecialtyRuntimeError {
    /// The requested legacy system type is not supported.
    #[error("System not supported: {0}")]
    SystemNotSupported(String),

    /// The requested architecture is not supported for cross-compilation.
    #[error("Architecture not supported: {0}")]
    ArchitectureNotSupported(String),

    /// Cross-compilation or build failed.
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    /// Communication with a legacy system or emulator failed.
    #[error("Communication error: {0}")]
    CommunicationError(String),

    /// Emulation of a legacy architecture failed.
    #[error("Emulation error: {0}")]
    EmulationError(String),

    /// Invalid or inconsistent configuration.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Operation exceeded the allowed time limit.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Underlying I/O operation failed.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON or other serialization failed.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Catch-all for other runtime errors.
    #[error("Other error: {0}")]
    Other(String),

    /// Not available on this platform / build: [`crate::embedded::types::ProgrammerInterface`] until hardware backends exist.
    ///
    /// See DEBT.md `D-EMBEDDED-PROGRAMMER` for evolution tracking.
    #[error("{operation} is not yet implemented for platform `{platform}`: {detail}")]
    EmbeddedProgrammerPlaceholder {
        /// Stable platform id (e.g. `generic_isp`, `parallel_eprom`).
        platform: &'static str,
        /// Operation name (e.g. `"Memory read"`).
        operation: &'static str,
        /// What a full implementation would require (transport, protocol, device support).
        detail: &'static str,
    },

    /// Not available on this platform / build: [`crate::embedded::types::EmbeddedEmulator`] until CPU cores exist.
    ///
    /// See DEBT.md `D-EMBEDDED-EMULATOR` for evolution tracking.
    #[error(
        "`{operation}` is not yet implemented for platform `{platform}` (feature {feature_id})"
    )]
    EmbeddedEmulatorPlaceholder {
        /// Architecture / platform id (e.g. `mos6502`, `z80`).
        platform: &'static str,
        /// Stable feature id for `SystemError::NotSupported` mapping (e.g. `embedded_emulator_mos6502`).
        feature_id: &'static str,
        /// Operation name (e.g. `"step"`).
        operation: &'static str,
    },
}

impl From<SpecialtyRuntimeError> for ToadStoolError {
    fn from(err: SpecialtyRuntimeError) -> Self {
        match err {
            SpecialtyRuntimeError::EmbeddedProgrammerPlaceholder {
                platform,
                operation,
                detail,
            } => ToadStoolError::not_supported(format!(
                "{operation} is not yet implemented for platform `{platform}`: {detail}"
            )),
            SpecialtyRuntimeError::EmbeddedEmulatorPlaceholder {
                platform,
                feature_id,
                operation,
            } => SystemError::NotSupported {
                feature: feature_id.to_string(),
                reason: format!(
                    "{operation} is not yet implemented for platform `{platform}`: embedded CPU emulator core not available in this build"
                ),
            }
            .into(),
            e => ToadStoolError::runtime(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpecialtyRuntimeError;
    use std::error::Error as StdError;
    use std::io;
    use toadstool::ToadStoolError;

    fn assert_nonempty_debug_display(err: &SpecialtyRuntimeError) {
        let d = format!("{err:?}");
        let s = err.to_string();
        assert!(!d.is_empty());
        assert!(!s.is_empty());
    }

    #[test]
    fn system_not_supported_debug_display() {
        let err = SpecialtyRuntimeError::SystemNotSupported("mips".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("mips"));
    }

    #[test]
    fn architecture_not_supported_debug_display() {
        let err = SpecialtyRuntimeError::ArchitectureNotSupported("h8300".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("h8300"));
    }

    #[test]
    fn compilation_failed_debug_display() {
        let err = SpecialtyRuntimeError::CompilationFailed("linker died".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("linker"));
    }

    #[test]
    fn communication_error_debug_display() {
        let err = SpecialtyRuntimeError::CommunicationError("serial timeout".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("serial"));
    }

    #[test]
    fn emulation_error_debug_display() {
        let err = SpecialtyRuntimeError::EmulationError("bad opcode".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("opcode"));
    }

    #[test]
    fn configuration_error_debug_display() {
        let err = SpecialtyRuntimeError::ConfigurationError("missing baud".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("baud"));
    }

    #[test]
    fn timeout_debug_display() {
        let err = SpecialtyRuntimeError::Timeout("30s".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("30s"));
    }

    #[test]
    fn other_debug_display() {
        let err = SpecialtyRuntimeError::Other("unknown".into());
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn io_error_from_std_io_preserves_source() {
        let inner = io::Error::new(io::ErrorKind::PermissionDenied, "no access");
        let err: SpecialtyRuntimeError = inner.into();
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("I/O"));
        assert!(err.to_string().contains("no access"));
        assert!(err.source().is_some());
    }

    #[test]
    fn serialization_error_from_serde_json_preserves_source() {
        let json_err = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let err: SpecialtyRuntimeError = json_err.into();
        assert_nonempty_debug_display(&err);
        assert!(err.to_string().contains("Serialization"));
        assert!(err.source().is_some());
    }

    #[test]
    fn serde_json_round_trip_and_invalid_input_maps_to_specialty_error() {
        let n: u64 = serde_json::from_str("7").unwrap();
        assert_eq!(n, 7);
        let back = serde_json::to_string(&n).unwrap();
        assert_eq!(back, "7");

        let json_err = serde_json::from_str::<u64>("not a number").unwrap_err();
        let mapped: SpecialtyRuntimeError = json_err.into();
        assert!(!mapped.to_string().is_empty());
    }

    #[test]
    fn into_toadstool_error_carries_message() {
        let cases = [
            SpecialtyRuntimeError::SystemNotSupported("a".into()),
            SpecialtyRuntimeError::ArchitectureNotSupported("b".into()),
            SpecialtyRuntimeError::CompilationFailed("c".into()),
            SpecialtyRuntimeError::CommunicationError("d".into()),
            SpecialtyRuntimeError::EmulationError("e".into()),
            SpecialtyRuntimeError::ConfigurationError("f".into()),
            SpecialtyRuntimeError::Timeout("g".into()),
            SpecialtyRuntimeError::IoError(io::Error::other("h")),
            SpecialtyRuntimeError::SerializationError(
                serde_json::from_str::<()>("oops").unwrap_err(),
            ),
            SpecialtyRuntimeError::Other("i".into()),
        ];
        for spec in cases {
            let msg = spec.to_string();
            let top: ToadStoolError = spec.into();
            let out = top.to_string();
            assert!(!out.is_empty());
            assert!(out.contains(&msg), "expected {out:?} to contain {msg:?}");
        }
    }

    #[test]
    fn embedded_programmer_placeholder_maps_to_not_supported() {
        let spec = SpecialtyRuntimeError::EmbeddedProgrammerPlaceholder {
            platform: "generic_isp",
            operation: "Memory read",
            detail: "test detail",
        };
        let top: ToadStoolError = spec.into();
        let out = top.to_string();
        assert!(
            out.to_lowercase().contains("not supported"),
            "expected not-supported semantics: {out}"
        );
        assert!(out.contains("Memory read"), "{out}");
    }

    #[test]
    fn embedded_emulator_placeholder_maps_to_system_not_supported() {
        let spec = SpecialtyRuntimeError::EmbeddedEmulatorPlaceholder {
            platform: "mos6502",
            feature_id: "embedded_emulator_mos6502",
            operation: "step",
        };
        let top: ToadStoolError = spec.into();
        let out = top.to_string();
        assert!(
            out.to_lowercase().contains("not supported"),
            "expected not-supported semantics: {out}"
        );
        assert!(
            out.contains("emulator") || out.contains("embedded_emulator"),
            "{out}"
        );
    }
}
