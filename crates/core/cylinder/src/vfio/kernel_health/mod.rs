// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kernel build environment health check — preflight gate for module operations.
//!
//! Detects `autoconf.h` corruption and `struct module` layout mismatches
//! that cause misleading `Invalid relocation target` errors at module load
//! time. Discovered via Exp 216: a corrupted `autoconf.h` shifted
//! `struct module` field offsets by 24 bytes, making `INIT_LIST_HEAD`
//! clobber the `exit` relocation target during the kernel's in-memory
//! relocation pass.
//!
//! Three detection layers, from cheapest to most definitive:
//!
//! 1. **Freshness** — `autoconf.h` mtime vs kernel image mtime
//! 2. **Struct probe** — compile a tiny module, read `offsetof(struct module, init/exit)`
//! 3. **Reference cross-check** — parse `.gnu.linkonce.this_module` RELA from a loaded `.ko`

mod autoconf;
mod elf;
mod paths;
mod probe;
mod reference;
mod repair;

use serde::{Deserialize, Serialize};

pub use autoconf::check_autoconf_freshness;
pub use probe::probe_struct_module_layout;
pub use reference::reference_module_offsets;
pub use repair::repair_autoconf;

/// Full health report from all detection layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelHealthReport {
    /// Layer 1: whether autoconf.h is older than or same age as the kernel image.
    pub autoconf_fresh: bool,
    /// Seconds between autoconf.h mtime and kernel image mtime.
    /// Negative means autoconf.h is older (expected/good).
    pub autoconf_age_delta_secs: i64,
    /// Layer 2: `offsetof(struct module, init)` from a freshly compiled probe.
    pub struct_module_init_offset: Option<u64>,
    /// Layer 2: `offsetof(struct module, exit)` from a freshly compiled probe.
    pub struct_module_exit_offset: Option<u64>,
    /// Layer 3: init offset from a reference .ko already known to load.
    pub reference_init_offset: Option<u64>,
    /// Layer 3: exit offset from a reference .ko already known to load.
    pub reference_exit_offset: Option<u64>,
    /// Whether probe offsets match reference offsets (both must be present).
    pub layout_matches: bool,
    /// Human-readable diagnosis.
    pub diagnosis: KernelHealthDiagnosis,
}

/// Diagnosis result from the health check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelHealthDiagnosis {
    /// All layers pass — safe to compile and load modules.
    Healthy,
    /// autoconf.h is newer than the kernel image.
    AutoconfStale { detail: String },
    /// Probe and reference disagree on struct module layout.
    StructLayoutMismatch {
        expected_exit: u64,
        actual_exit: u64,
    },
    /// Could not compile the probe module (missing headers/toolchain).
    ProbeCompileFailed { reason: String },
    /// No reference module found to cross-check against.
    NoReferenceModule,
}

impl std::fmt::Display for KernelHealthDiagnosis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "kernel build environment healthy"),
            Self::AutoconfStale { detail } => write!(f, "autoconf.h stale: {detail}"),
            Self::StructLayoutMismatch {
                expected_exit,
                actual_exit,
            } => {
                write!(
                    f,
                    "struct module layout mismatch: reference exit=0x{expected_exit:x}, \
                     probe exit=0x{actual_exit:x} (delta={} bytes)",
                    (*expected_exit as i64) - (*actual_exit as i64)
                )
            }
            Self::ProbeCompileFailed { reason } => {
                write!(f, "probe module compilation failed: {reason}")
            }
            Self::NoReferenceModule => write!(f, "no reference module available for cross-check"),
        }
    }
}

/// Errors from health check operations.
#[derive(Debug, thiserror::Error)]
pub enum KernelHealthError {
    #[error("cannot determine running kernel release: {0}")]
    KernelRelease(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("probe compilation failed: {0}")]
    ProbeCompile(String),
    #[error("ELF parse error: {0}")]
    ElfParse(String),
}

/// Strategy for repairing a corrupted autoconf.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairStrategy {
    /// Extract from cached .deb in /var/cache/apt/archives/ (fastest, no network).
    PackageRestore,
    /// `apt-get install --reinstall linux-headers-$(uname -r)` (slow, needs network).
    PackageReinstall,
}

/// Run all three detection layers and produce a comprehensive health report.
pub fn full_kernel_health_check() -> Result<KernelHealthReport, KernelHealthError> {
    let (autoconf_fresh, autoconf_age_delta_secs) = match check_autoconf_freshness() {
        Ok((fresh, delta)) => (fresh, delta),
        Err(e) => {
            tracing::warn!(err = %e, "autoconf freshness check failed — assuming stale");
            (false, i64::MAX)
        }
    };

    let (probe_init, probe_exit, probe_err) = match probe_struct_module_layout() {
        Ok((i, e)) => (Some(i), Some(e), None),
        Err(compile_err) => {
            tracing::warn!(err = %compile_err, "struct module probe compilation failed — trying DKMS fallback");
            match probe::probe_from_dkms_module() {
                Some((i, e)) => (Some(i), Some(e), None),
                None => (None, None, Some(compile_err)),
            }
        }
    };

    let (ref_init, ref_exit) = if let Some(ref_ko) = reference::find_reference_ko() {
        match reference_module_offsets(&ref_ko) {
            Ok((i, e)) => (Some(i), Some(e)),
            Err(e) => {
                tracing::warn!(err = %e, ko = %ref_ko.display(), "reference module parse failed");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let layout_matches;
    let diagnosis;

    match (probe_exit, ref_exit) {
        (Some(pe), Some(re)) => {
            layout_matches = pe == re;
            if !layout_matches {
                diagnosis = KernelHealthDiagnosis::StructLayoutMismatch {
                    expected_exit: re,
                    actual_exit: pe,
                };
            } else if !autoconf_fresh {
                diagnosis = KernelHealthDiagnosis::AutoconfStale {
                    detail: format!(
                        "autoconf.h is {autoconf_age_delta_secs}s newer than kernel image, \
                         but struct layout still matches — monitor for drift"
                    ),
                };
            } else {
                diagnosis = KernelHealthDiagnosis::Healthy;
            }
        }
        (None, _) => {
            layout_matches = false;
            if let Some(err) = probe_err {
                diagnosis = KernelHealthDiagnosis::ProbeCompileFailed {
                    reason: err.to_string(),
                };
            } else {
                diagnosis = KernelHealthDiagnosis::ProbeCompileFailed {
                    reason: "unknown probe failure".into(),
                };
            }
        }
        (Some(_), None) => {
            if !autoconf_fresh {
                layout_matches = false;
                diagnosis = KernelHealthDiagnosis::AutoconfStale {
                    detail: format!(
                        "autoconf.h is {autoconf_age_delta_secs}s newer than kernel image \
                         and no reference module for cross-check"
                    ),
                };
            } else {
                layout_matches = true;
                diagnosis = KernelHealthDiagnosis::Healthy;
            }
        }
    }

    Ok(KernelHealthReport {
        autoconf_fresh,
        autoconf_age_delta_secs,
        struct_module_init_offset: probe_init,
        struct_module_exit_offset: probe_exit,
        reference_init_offset: ref_init,
        reference_exit_offset: ref_exit,
        layout_matches,
        diagnosis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtime_delta_same_time() {
        let now = std::time::SystemTime::now();
        assert_eq!(autoconf::mtime_delta_secs(now, now), 0);
    }

    #[test]
    fn mtime_delta_future() {
        let now = std::time::SystemTime::now();
        let future = now + std::time::Duration::from_secs(100);
        assert!(autoconf::mtime_delta_secs(future, now) > 0);
    }

    #[test]
    fn mtime_delta_past() {
        let now = std::time::SystemTime::now();
        let past = now - std::time::Duration::from_secs(100);
        assert!(autoconf::mtime_delta_secs(past, now) < 0);
    }

    #[test]
    fn read_cstr_basic() {
        let data = b"hello\x00world\x00";
        assert_eq!(elf::read_cstr(data, 0), "hello");
        assert_eq!(elf::read_cstr(data, 6), "world");
    }

    #[test]
    fn read_cstr_at_end() {
        let data = b"abc";
        assert_eq!(elf::read_cstr(data, 0), "abc");
    }

    #[test]
    fn diagnosis_display_healthy() {
        let d = KernelHealthDiagnosis::Healthy;
        assert_eq!(d.to_string(), "kernel build environment healthy");
    }

    #[test]
    fn diagnosis_display_mismatch() {
        let d = KernelHealthDiagnosis::StructLayoutMismatch {
            expected_exit: 0x4a8,
            actual_exit: 0x490,
        };
        let s = d.to_string();
        assert!(s.contains("0x4a8"));
        assert!(s.contains("0x490"));
        assert!(s.contains("24 bytes"));
    }

    #[test]
    fn find_note_section_rejects_short_file() {
        let data = vec![0u8; 32];
        assert!(probe::find_note_section_offsets(&data).is_err());
    }

    #[test]
    fn find_note_section_rejects_non_elf() {
        let mut data = vec![0u8; 128];
        data[0..4].copy_from_slice(b"NOTA");
        assert!(probe::find_note_section_offsets(&data).is_err());
    }

    #[test]
    fn parse_this_module_rejects_non_elf() {
        let data = vec![0u8; 128];
        assert!(reference::parse_this_module_rela_offsets(&data).is_err());
    }

    #[test]
    fn repair_strategy_serializes() {
        let json = serde_json::to_string(&RepairStrategy::PackageRestore).unwrap();
        assert!(json.contains("PackageRestore"));
    }

    #[test]
    fn health_report_serializes() {
        let report = KernelHealthReport {
            autoconf_fresh: true,
            autoconf_age_delta_secs: -86400,
            struct_module_init_offset: Some(0x168),
            struct_module_exit_offset: Some(0x4a8),
            reference_init_offset: Some(0x168),
            reference_exit_offset: Some(0x4a8),
            layout_matches: true,
            diagnosis: KernelHealthDiagnosis::Healthy,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Healthy"));
        assert!(json.contains("4a8") || json.contains("1192"));
    }
}
