// SPDX-License-Identifier: AGPL-3.0-or-later
//! Binary module patcher for sovereign driver rotation.
//!
//! Patches stock kernel modules (`.ko`) at runtime to disable teardown
//! functions during driver unbind. This preserves GPU hardware state
//! (PMC, GPC, CE) across the nouveau→vfio-pci warm handoff.
//!
//! # Technique
//!
//! On x86_64 with `CONFIG_FTRACE=y`, every kernel function begins with a
//! 5-byte `call __fentry__` (opcode `e8 00 00 00 00` before relocation).
//! The actual function prologue starts at offset +5. We write `0xC3`
//! (ret) at offset +5 to make the function return immediately without
//! disturbing the ftrace call site (which has relocation entries that
//! the kernel validates on module load).
//!
//! This technique was proven in Exp 211 (May 2026) on kernel 6.17.9.
//!
//! # Usage
//!
//! ```no_run
//! use toadstool_cylinder::vfio::module_patch::*;
//!
//! let stock_ko = std::path::Path::new("/lib/modules/6.17.9/nouveau.ko");
//! let patch_set = PatchSet::volta_warm_handoff();
//! let patched_path = patch_module(stock_ko, &patch_set).unwrap();
//! // patched_path is now ready for insmod
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// How to patch a function's entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchStrategy {
    /// Skip 5-byte ftrace call at function entry, write `0xC3` (ret) at
    /// offset +5. Proven safe on kernel 6.17+ where relocation checks
    /// reject modifications to ftrace call sites.
    RetAfterFtrace,
}

/// A single function to patch in a kernel module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchTarget {
    /// Function symbol name (e.g., `gf100_gr_fini`).
    pub symbol: String,
    /// Patching strategy.
    pub strategy: PatchStrategy,
}

/// A named collection of patch targets for a specific warm handoff strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSet {
    /// Human-readable name (e.g., "volta_warm_handoff").
    pub name: String,
    /// Module name this patch set applies to (e.g., "nouveau").
    pub module_name: String,
    /// Functions to patch.
    pub targets: Vec<PatchTarget>,
}

impl PatchSet {
    /// Patch set for Volta (GV100) warm handoff via nouveau.
    ///
    /// NOPs the five teardown functions that power-gate GPCs on unbind.
    /// With these patched, `rmmod nouveau` preserves PMC_ENABLE and the
    /// GPC broadcast routing fabric.
    #[must_use]
    pub fn volta_warm_handoff() -> Self {
        Self {
            name: "volta_warm_handoff".into(),
            module_name: "nouveau".into(),
            targets: vec![
                PatchTarget {
                    symbol: "gf100_gr_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_pmu_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_disable".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_reset".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "gk104_fifo_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
            ],
        }
    }

    /// Patch set for Kepler (GK210 / K80) warm handoff via nouveau.
    ///
    /// Kepler has unsigned falcons so nouveau can fully initialize FECS.
    /// These patches preserve the initialized state across unbind.
    #[must_use]
    pub fn kepler_warm_handoff() -> Self {
        Self {
            name: "kepler_warm_handoff".into(),
            module_name: "nouveau".into(),
            targets: vec![
                PatchTarget {
                    symbol: "gf100_gr_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_pmu_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_disable".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_reset".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "gk104_fifo_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
            ],
        }
    }

    /// Look up a predefined patch set by name.
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "volta_warm_handoff" => Some(Self::volta_warm_handoff()),
            "kepler_warm_handoff" => Some(Self::kepler_warm_handoff()),
            _ => None,
        }
    }
}

/// Errors from module patching operations.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    #[error("failed to read module: {0}")]
    ReadFailed(std::io::Error),

    #[error("failed to write patched module to {path}: {source}")]
    WriteFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("symbol '{symbol}' not found in {module}")]
    SymbolNotFound { symbol: String, module: String },

    #[error("nm failed on {path}: {detail}")]
    NmFailed { path: String, detail: String },

    #[error("ftrace call site not found at offset {offset:#x} for {symbol} (expected 0xe8, got {found:#04x})")]
    NoFtraceCallSite {
        symbol: String,
        offset: usize,
        found: u8,
    },

    #[error("symbol offset {offset:#x} for {symbol} exceeds module size {module_size}")]
    OffsetOutOfBounds {
        symbol: String,
        offset: usize,
        module_size: usize,
    },
}

/// Result of patching a single function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    /// Function symbol name.
    pub symbol: String,
    /// Whether the patch was applied.
    pub applied: bool,
    /// File offset where the patch was written.
    pub offset: Option<usize>,
    /// Detail message.
    pub detail: String,
}

/// Result of patching an entire module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulePatchResult {
    /// Path to the patched `.ko` file.
    pub patched_path: String,
    /// Source module path.
    pub source_path: String,
    /// Patch set name.
    pub patch_set: String,
    /// Per-function results.
    pub patches: Vec<PatchResult>,
    /// Number of patches successfully applied.
    pub applied_count: usize,
    /// Total number of targets.
    pub total_count: usize,
}

/// Resolve text symbol offsets in a `.ko` file via `nm`.
///
/// Returns a map of symbol_name → file_offset for all defined text symbols.
fn resolve_symbols(ko_path: &Path) -> Result<HashMap<String, u64>, PatchError> {
    let output = Command::new("nm")
        .args(["--defined-only", "-n"])
        .arg(ko_path)
        .output()
        .map_err(|e| PatchError::NmFailed {
            path: ko_path.display().to_string(),
            detail: format!("failed to execute nm: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PatchError::NmFailed {
            path: ko_path.display().to_string(),
            detail: stderr.trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut symbols = HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && (parts[1] == "T" || parts[1] == "t") {
            if let Ok(addr) = u64::from_str_radix(parts[0], 16) {
                symbols.insert(parts[2].to_string(), addr);
            }
        }
    }

    Ok(symbols)
}

/// The x86_64 ftrace call prologue: `call __fentry__` = `e8 00 00 00 00`
/// (before relocation fills in the displacement).
const FTRACE_CALL_OPCODE: u8 = 0xe8;

/// x86_64 `ret` instruction.
const RET_OPCODE: u8 = 0xc3;

/// Ftrace call site size in bytes.
const FTRACE_CALL_SIZE: usize = 5;

/// Patch a stock kernel module and write the result to a temporary file.
///
/// Reads the source `.ko`, resolves symbol offsets via `nm`, applies the
/// requested patches, and writes the result to `/tmp/toadstool-patched-{name}.ko`.
///
/// Returns the path to the patched module and per-target results.
pub fn patch_module(source_ko: &Path, patch_set: &PatchSet) -> Result<ModulePatchResult, PatchError> {
    tracing::info!(
        source = %source_ko.display(),
        patch_set = patch_set.name.as_str(),
        targets = patch_set.targets.len(),
        "patching kernel module"
    );

    let mut module_bytes = std::fs::read(source_ko).map_err(PatchError::ReadFailed)?;
    let module_size = module_bytes.len();

    let symbols = resolve_symbols(source_ko)?;

    let mut patches = Vec::new();
    let mut applied_count = 0;

    for target in &patch_set.targets {
        let result = apply_single_patch(
            &mut module_bytes,
            module_size,
            &symbols,
            target,
            source_ko,
        );

        match result {
            Ok(pr) => {
                if pr.applied {
                    applied_count += 1;
                }
                patches.push(pr);
            }
            Err(e) => {
                tracing::warn!(
                    symbol = target.symbol.as_str(),
                    error = %e,
                    "patch target failed (non-fatal, continuing)"
                );
                patches.push(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: None,
                    detail: format!("{e}"),
                });
            }
        }
    }

    let patched_path = format!("/tmp/toadstool-patched-{}.ko", patch_set.module_name);
    std::fs::write(&patched_path, &module_bytes).map_err(|e| PatchError::WriteFailed {
        path: patched_path.clone(),
        source: e,
    })?;

    tracing::info!(
        patched_path = patched_path.as_str(),
        applied = applied_count,
        total = patch_set.targets.len(),
        "module patched"
    );

    Ok(ModulePatchResult {
        patched_path: patched_path.clone(),
        source_path: source_ko.display().to_string(),
        patch_set: patch_set.name.clone(),
        patches,
        applied_count,
        total_count: patch_set.targets.len(),
    })
}

/// Apply a single patch target to the module bytes.
fn apply_single_patch(
    module_bytes: &mut [u8],
    module_size: usize,
    symbols: &HashMap<String, u64>,
    target: &PatchTarget,
    source_path: &Path,
) -> Result<PatchResult, PatchError> {
    let &sym_offset = symbols.get(&target.symbol).ok_or_else(|| {
        PatchError::SymbolNotFound {
            symbol: target.symbol.clone(),
            module: source_path.display().to_string(),
        }
    })?;

    let offset = sym_offset as usize;

    if offset + FTRACE_CALL_SIZE >= module_size {
        return Err(PatchError::OffsetOutOfBounds {
            symbol: target.symbol.clone(),
            offset,
            module_size,
        });
    }

    match target.strategy {
        PatchStrategy::RetAfterFtrace => {
            // Verify ftrace call site at function entry
            if module_bytes[offset] != FTRACE_CALL_OPCODE {
                return Err(PatchError::NoFtraceCallSite {
                    symbol: target.symbol.clone(),
                    offset,
                    found: module_bytes[offset],
                });
            }

            let patch_offset = offset + FTRACE_CALL_SIZE;
            if patch_offset >= module_size {
                return Err(PatchError::OffsetOutOfBounds {
                    symbol: target.symbol.clone(),
                    offset: patch_offset,
                    module_size,
                });
            }

            let original_byte = module_bytes[patch_offset];
            module_bytes[patch_offset] = RET_OPCODE;

            tracing::debug!(
                symbol = target.symbol.as_str(),
                nm_offset = format_args!("{offset:#x}"),
                patch_offset = format_args!("{patch_offset:#x}"),
                original = format_args!("{original_byte:#04x}"),
                "patched: ret after ftrace"
            );

            Ok(PatchResult {
                symbol: target.symbol.clone(),
                applied: true,
                offset: Some(patch_offset),
                detail: format!(
                    "ret@{patch_offset:#x} (was {original_byte:#04x})"
                ),
            })
        }
    }
}

/// Get the path where a patched module would be written.
#[must_use]
pub fn patched_module_path(module_name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/toadstool-patched-{module_name}.ko"))
}

/// Clean up a previously patched module from /tmp.
pub fn cleanup_patched_module(module_name: &str) -> Result<(), std::io::Error> {
    let path = patched_module_path(module_name);
    if path.exists() {
        std::fs::remove_file(&path)?;
        tracing::debug!(path = %path.display(), "cleaned up patched module");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volta_patch_set_targets_correct_functions() {
        let ps = PatchSet::volta_warm_handoff();
        assert_eq!(ps.module_name, "nouveau");
        assert_eq!(ps.targets.len(), 5);

        let names: Vec<&str> = ps.targets.iter().map(|t| t.symbol.as_str()).collect();
        assert!(names.contains(&"gf100_gr_fini"));
        assert!(names.contains(&"nvkm_pmu_fini"));
        assert!(names.contains(&"nvkm_mc_disable"));
        assert!(names.contains(&"nvkm_mc_reset"));
        assert!(names.contains(&"gk104_fifo_fini"));
    }

    #[test]
    fn kepler_patch_set_targets_correct_functions() {
        let ps = PatchSet::kepler_warm_handoff();
        assert_eq!(ps.module_name, "nouveau");
        assert_eq!(ps.targets.len(), 5);
    }

    #[test]
    fn by_name_resolves_known_sets() {
        assert!(PatchSet::by_name("volta_warm_handoff").is_some());
        assert!(PatchSet::by_name("kepler_warm_handoff").is_some());
        assert!(PatchSet::by_name("nonexistent").is_none());
    }

    #[test]
    fn patch_strategy_serde_roundtrip() {
        let ps = PatchSet::volta_warm_handoff();
        let json = serde_json::to_string(&ps).unwrap();
        let back: PatchSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "volta_warm_handoff");
        assert_eq!(back.targets.len(), 5);
    }

    #[test]
    fn apply_single_patch_patches_ret_after_ftrace() {
        // Simulate a minimal function: e8 00 00 00 00 55 (call + push rbp)
        let mut bytes = vec![0xe8, 0x00, 0x00, 0x00, 0x00, 0x55, 0x48, 0x89];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

        let target = PatchTarget {
            symbol: "test_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(
            &mut bytes,
            len,
            &symbols,
            &target,
            Path::new("test.ko"),
        )
        .unwrap();

        assert!(result.applied);
        assert_eq!(result.offset, Some(5));
        assert_eq!(bytes[5], RET_OPCODE);
    }

    #[test]
    fn apply_single_patch_rejects_missing_ftrace() {
        let mut bytes = vec![0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

        let target = PatchTarget {
            symbol: "test_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(
            &mut bytes,
            len,
            &symbols,
            &target,
            Path::new("test.ko"),
        );

        assert!(matches!(result, Err(PatchError::NoFtraceCallSite { .. })));
    }

    #[test]
    fn apply_single_patch_rejects_missing_symbol() {
        let mut bytes = vec![0xe8, 0x00, 0x00, 0x00, 0x00, 0x55];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = HashMap::new();

        let target = PatchTarget {
            symbol: "missing_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(
            &mut bytes,
            len,
            &symbols,
            &target,
            Path::new("test.ko"),
        );

        assert!(matches!(result, Err(PatchError::SymbolNotFound { .. })));
    }
}
