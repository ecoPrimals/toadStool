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
    /// offset +5. Works when byte+5 has no kernel relocation entry.
    RetAfterFtrace,
    /// Write `0xC3` (ret) directly at offset+0 (the function entry point).
    /// Replaces the first byte of the ftrace call/NOP preamble. Safe from
    /// relocation rejection because byte+0 is the opcode, not the
    /// displacement target (which occupies bytes 1-4).
    /// Use for functions where `RetAfterFtrace` hits a relocation at +5.
    RetAtEntry,
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
    /// Minimum patches that must succeed for the module to be considered
    /// usable. Defaults to 1 — loading an entirely unpatched copy is
    /// almost certainly wrong.
    #[serde(default = "default_min_applied")]
    pub min_applied: usize,
}

fn default_min_applied() -> usize { 1 }

impl PatchSet {
    /// Patch set for Volta (GV100) warm handoff via nouveau.
    ///
    /// NOPs teardown functions that power-gate GPCs and clock-gate engines
    /// on unbind. With these patched, `rmmod nouveau` preserves PMC_ENABLE,
    /// GPC broadcast routing fabric, FECS microcode, and TPC power state.
    ///
    /// Exp 215 identified that the original 5-target set preserved GPC fabric
    /// but TPCs remained power-gated (0xBADF5040 at per-TPC registers).
    /// Added clock gate teardown functions that control BLCG/SLCG/ELPG
    /// power domains within GPCs.
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
                // Exp 215: clock gate teardown — preserve TPC power domains.
                // Uses RetAtEntry because RetAfterFtrace hits kernel
                // relocation checks on these functions (byte+5 has an
                // R_X86_64_PLT32 relocation entry).
                PatchTarget {
                    symbol: "gk104_clkgate_fini".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "nvkm_therm_clkgate_fini".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "g84_therm_fini".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
            ],
            min_applied: 1,
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
            min_applied: 1,
        }
    }

    /// Patch set for Volta (GV100) warm handoff via nvidia open kernel module.
    ///
    /// Targets the nvidia-580-open (or compatible) module's PCI remove path.
    /// `nv_pci_remove` is the per-device teardown entry — NOPing it preserves
    /// the full RM-initialized state (SEC2→ACR→FECS→GR→TPC PRI ring stations).
    ///
    /// Also targets `gpuStateUnload_IMPL` (master engine unload dispatcher)
    /// and `gpuStateDestroy_IMPL` as fallbacks if `nv_pci_remove` cannot be
    /// resolved (symbol visibility varies across driver versions).
    #[must_use]
    pub fn nvidia_warm_handoff() -> Self {
        Self {
            name: "nvidia_warm_handoff".into(),
            module_name: "nvidia".into(),
            targets: vec![
                PatchTarget {
                    symbol: "nv_pci_remove".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "gpuStateUnload_IMPL".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "gpuStateDestroy_IMPL".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "_deviceTeardown".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "clTeardown_IMPL".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "fecsBufferTeardown".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
            ],
            min_applied: 1,
        }
    }

    /// Look up a predefined patch set by name.
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "volta_warm_handoff" => Some(Self::volta_warm_handoff()),
            "kepler_warm_handoff" => Some(Self::kepler_warm_handoff()),
            "nvidia_warm_handoff" => Some(Self::nvidia_warm_handoff()),
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

    #[error("ftrace call site not found at offset {offset:#x} for {symbol} (expected 0xe8/0x90/0x00/0x0f, got {found:#04x})")]
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

    #[error("insufficient patches applied: {applied}/{total} (minimum: {min_required})")]
    InsufficientPatches {
        applied: usize,
        total: usize,
        min_required: usize,
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
        if parts.len() >= 3 && (parts[1] == "T" || parts[1] == "t")
            && let Ok(addr) = u64::from_str_radix(parts[0], 16)
        {
            symbols.insert(parts[2].to_string(), addr);
        }
    }

    Ok(symbols)
}

/// The x86_64 ftrace call prologue: `call __fentry__` = `e8 00 00 00 00`
/// (before relocation fills in the displacement).
///
/// With `CONFIG_DYNAMIC_FTRACE`, the `.ko` on disk may have NOP sleds
/// (`0x90` or `0x0f1f...`) instead — the kernel patches these to `call`
/// at load time. We accept both forms.
const FTRACE_CALL_OPCODE: u8 = 0xe8;
const NOP_OPCODE: u8 = 0x90;
const NULL_BYTE: u8 = 0x00;
/// Lead byte of multi-byte NOP instructions (e.g., `0f 1f 44 00 00`).
/// Kernel 6.17+ `CONFIG_DYNAMIC_FTRACE` can emit these at function entry.
const MULTIBYTE_NOP_LEAD: u8 = 0x0f;

/// x86_64 `ret` instruction.
const RET_OPCODE: u8 = 0xc3;

/// Ftrace call site size in bytes.
const FTRACE_CALL_SIZE: usize = 5;

/// Patch a stock kernel module, optionally rename it, and write the result
/// to a temporary file.
///
/// Reads the source `.ko`, resolves symbol offsets via `nm`, applies the
/// requested patches, and writes the result to `/tmp/toadstool-patched-{name}.ko`.
///
/// If `rename` is `Some((old, new))`, the module identity is rewritten so
/// it can be loaded alongside the original (avoids "module already loaded"
/// rejection).
///
/// Returns the path to the patched module and per-target results.
pub fn patch_module_with_rename(
    source_ko: &Path,
    patch_set: &PatchSet,
    rename: Option<(&str, &str)>,
) -> Result<ModulePatchResult, PatchError> {
    tracing::info!(
        source = %source_ko.display(),
        patch_set = patch_set.name.as_str(),
        targets = patch_set.targets.len(),
        "patching kernel module"
    );

    let mut module_bytes = std::fs::read(source_ko).map_err(PatchError::ReadFailed)?;
    let module_size = module_bytes.len();

    // Normalize relocations for kernel 6.17+ compatibility.
    // Proprietary blobs (nvidia-470) have nonzero values at R_X86_64_64
    // relocation targets that kernel 6.17+ rejects.
    let reloc_normalized = normalize_relocations(&mut module_bytes).unwrap_or_else(|e| {
        tracing::debug!(error = %e, "relocation normalization skipped");
        0
    });
    if reloc_normalized > 0 {
        tracing::info!(
            reloc_normalized,
            "normalized proprietary blob relocations"
        );
    }

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

    if applied_count < patch_set.min_applied {
        tracing::error!(
            applied = applied_count,
            min_required = patch_set.min_applied,
            total = patch_set.targets.len(),
            "insufficient patches applied — refusing to write module"
        );
        return Err(PatchError::InsufficientPatches {
            applied: applied_count,
            total: patch_set.targets.len(),
            min_required: patch_set.min_applied,
        });
    }

    if let Some((old_name, new_name)) = rename {
        let rename_count = rename_module_identity(&mut module_bytes, old_name, new_name)?;
        tracing::info!(
            old_name, new_name, rename_count,
            "module identity renamed for dual-load injection"
        );
        patches.push(PatchResult {
            symbol: format!("__module_rename:{old_name}→{new_name}"),
            applied: rename_count > 0,
            offset: None,
            detail: format!("{rename_count} identity replacements"),
        });
    }

    let output_name = rename
        .map(|(_, new)| new)
        .unwrap_or(&patch_set.module_name);
    let patched_path = format!("/tmp/toadstool-patched-{output_name}.ko");
    std::fs::write(&patched_path, &module_bytes).map_err(|e| PatchError::WriteFailed {
        path: patched_path.clone(),
        source: e,
    })?;

    tracing::info!(
        patched_path = patched_path.as_str(),
        applied = applied_count,
        total = patch_set.targets.len(),
        min_required = patch_set.min_applied,
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

/// Patch a stock kernel module and write the result to a temporary file.
///
/// Convenience wrapper around [`patch_module_with_rename`] without renaming.
pub fn patch_module(source_ko: &Path, patch_set: &PatchSet) -> Result<ModulePatchResult, PatchError> {
    patch_module_with_rename(source_ko, patch_set, None)
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
            let lead_byte = module_bytes[offset];
            let is_ftrace_call = lead_byte == FTRACE_CALL_OPCODE;
            let is_nop_padded = lead_byte == NOP_OPCODE
                || lead_byte == NULL_BYTE
                || lead_byte == MULTIBYTE_NOP_LEAD;

            if !is_ftrace_call && !is_nop_padded {
                return Err(PatchError::NoFtraceCallSite {
                    symbol: target.symbol.clone(),
                    offset,
                    found: lead_byte,
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

            let site_type = if is_ftrace_call { "e8-call" } else { "nop-padded" };
            tracing::debug!(
                symbol = target.symbol.as_str(),
                nm_offset = format_args!("{offset:#x}"),
                patch_offset = format_args!("{patch_offset:#x}"),
                original = format_args!("{original_byte:#04x}"),
                site_type,
                "patched: ret after ftrace"
            );

            Ok(PatchResult {
                symbol: target.symbol.clone(),
                applied: true,
                offset: Some(patch_offset),
                detail: format!(
                    "ret@{patch_offset:#x} (was {original_byte:#04x}, site={site_type})"
                ),
            })
        }
        PatchStrategy::RetAtEntry => {
            let original_byte = module_bytes[offset];
            module_bytes[offset] = RET_OPCODE;

            tracing::debug!(
                symbol = target.symbol.as_str(),
                offset = format_args!("{offset:#x}"),
                original = format_args!("{original_byte:#04x}"),
                "patched: ret at entry"
            );

            Ok(PatchResult {
                symbol: target.symbol.clone(),
                applied: true,
                offset: Some(offset),
                detail: format!(
                    "ret@{offset:#x} (was {original_byte:#04x}, entry-direct)"
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

/// Normalize R_X86_64_64 relocations for kernel 6.17+ compatibility.
///
/// Proprietary kernel modules (e.g., nvidia-470) may have nonzero values at
/// relocation target offsets — the linker pre-baked instruction bytes there.
/// Kernel 6.17+ rejects modules where relocation targets are nonzero
/// ("Invalid relocation target, existing value is nonzero for type 1").
///
/// This pass transfers the existing target value into the relocation's
/// `r_addend` field and zeros the target, preserving semantic equivalence:
/// `final_value = sym_value + r_addend + existing_value` becomes
/// `final_value = sym_value + (r_addend + existing_value) + 0`.
///
/// Returns the number of relocations normalized.
pub fn normalize_relocations(module_bytes: &mut [u8]) -> Result<usize, PatchError> {
    let elf_class = module_bytes.get(4).copied().unwrap_or(0);
    if elf_class != 2 {
        return Err(PatchError::NmFailed {
            path: "(in-memory)".into(),
            detail: "not a 64-bit ELF".into(),
        });
    }

    let e_shoff = u64::from_le_bytes(
        module_bytes[40..48].try_into().unwrap_or([0; 8]),
    ) as usize;
    let e_shentsize = u16::from_le_bytes(
        module_bytes[58..60].try_into().unwrap_or([0; 2]),
    ) as usize;
    let e_shnum = u16::from_le_bytes(
        module_bytes[60..62].try_into().unwrap_or([0; 2]),
    ) as usize;

    if e_shoff == 0 || e_shentsize < 64 || e_shnum == 0 {
        return Err(PatchError::NmFailed {
            path: "(in-memory)".into(),
            detail: "invalid section header table".into(),
        });
    }

    const SHT_RELA: u32 = 4;
    const R_X86_64_64: u32 = 1;

    let mut normalized = 0usize;

    for i in 0..e_shnum {
        let sh_start = e_shoff + i * e_shentsize;
        if sh_start + e_shentsize > module_bytes.len() {
            break;
        }

        let sh_type = u32::from_le_bytes(
            module_bytes[sh_start + 4..sh_start + 8]
                .try_into()
                .unwrap_or([0; 4]),
        );
        if sh_type != SHT_RELA {
            continue;
        }

        let sh_offset = u64::from_le_bytes(
            module_bytes[sh_start + 24..sh_start + 32]
                .try_into()
                .unwrap_or([0; 8]),
        ) as usize;
        let sh_size = u64::from_le_bytes(
            module_bytes[sh_start + 32..sh_start + 40]
                .try_into()
                .unwrap_or([0; 8]),
        ) as usize;
        // sh_info is at offset 44 in the section header (after sh_link at 40)
        let sh_info = u32::from_le_bytes(
            module_bytes[sh_start + 44..sh_start + 48]
                .try_into()
                .unwrap_or([0; 4]),
        ) as usize;

        let target_sh_start = e_shoff + sh_info * e_shentsize;
        let target_sh_offset = if target_sh_start + 32 <= module_bytes.len() {
            u64::from_le_bytes(
                module_bytes[target_sh_start + 24..target_sh_start + 32]
                    .try_into()
                    .unwrap_or([0; 8]),
            ) as usize
        } else {
            continue;
        };

        let entry_size = 24usize; // sizeof(Elf64_Rela)
        let num_entries = sh_size / entry_size;

        for j in 0..num_entries {
            let rela_off = sh_offset + j * entry_size;
            if rela_off + entry_size > module_bytes.len() {
                break;
            }

            let r_offset = u64::from_le_bytes(
                module_bytes[rela_off..rela_off + 8]
                    .try_into()
                    .unwrap_or([0; 8]),
            );
            let r_info = u64::from_le_bytes(
                module_bytes[rela_off + 8..rela_off + 16]
                    .try_into()
                    .unwrap_or([0; 8]),
            );
            let r_type = (r_info & 0xFFFF_FFFF) as u32;

            if r_type != R_X86_64_64 {
                continue;
            }

            let target_file_off = target_sh_offset + r_offset as usize;
            if target_file_off + 8 > module_bytes.len() || target_file_off < 64 {
                continue;
            }

            let existing = i64::from_le_bytes(
                module_bytes[target_file_off..target_file_off + 8]
                    .try_into()
                    .unwrap_or([0; 8]),
            );

            if existing == 0 {
                continue;
            }

            let old_addend = i64::from_le_bytes(
                module_bytes[rela_off + 16..rela_off + 24]
                    .try_into()
                    .unwrap_or([0; 8]),
            );
            let new_addend = old_addend.wrapping_add(existing);

            module_bytes[rela_off + 16..rela_off + 24]
                .copy_from_slice(&new_addend.to_le_bytes());
            module_bytes[target_file_off..target_file_off + 8]
                .copy_from_slice(&0i64.to_le_bytes());

            normalized += 1;
        }
    }

    tracing::info!(normalized, "R_X86_64_64 relocations normalized for kernel 6.17+ compat");
    Ok(normalized)
}

/// Rename the module identity inside a `.ko` binary.
///
/// Replaces occurrences of `old_name` with `new_name` in the module's
/// `.modinfo` and `.gnu.linkonce.this_module` sections. The new name must
/// be <= the old name's length (we pad with NUL bytes). This allows
/// `insmod` to load the module alongside an already-loaded copy with the
/// original name.
///
/// Returns the number of replacements made.
pub fn rename_module_identity(
    module_bytes: &mut [u8],
    old_name: &str,
    new_name: &str,
) -> Result<usize, PatchError> {
    if new_name.len() > old_name.len() {
        return Err(PatchError::NmFailed {
            path: "(in-memory)".into(),
            detail: format!(
                "new module name '{}' ({} bytes) exceeds old name '{}' ({} bytes)",
                new_name, new_name.len(), old_name, old_name.len(),
            ),
        });
    }

    let old_bytes = old_name.as_bytes();
    let mut new_padded = vec![0u8; old_bytes.len()];
    new_padded[..new_name.len()].copy_from_slice(new_name.as_bytes());

    let mut replacements = 0;
    let mut pos = 0;
    while pos + old_bytes.len() <= module_bytes.len() {
        if &module_bytes[pos..pos + old_bytes.len()] == old_bytes {
            let before = if pos > 0 { module_bytes[pos - 1] } else { 0 };
            let after_pos = pos + old_bytes.len();
            let after = if after_pos < module_bytes.len() {
                module_bytes[after_pos]
            } else {
                0
            };
            if before == 0 && (after == 0 || after == b'=' || after == b'\0') {
                module_bytes[pos..pos + old_bytes.len()].copy_from_slice(&new_padded);
                replacements += 1;
                tracing::debug!(
                    offset = format_args!("{pos:#x}"),
                    old = old_name,
                    new = new_name,
                    "renamed module identity"
                );
            }
        }
        pos += 1;
    }

    tracing::info!(
        old = old_name,
        new = new_name,
        replacements,
        "module identity rename complete"
    );

    Ok(replacements)
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
        assert_eq!(ps.targets.len(), 8);

        let names: Vec<&str> = ps.targets.iter().map(|t| t.symbol.as_str()).collect();
        assert!(names.contains(&"gf100_gr_fini"));
        assert!(names.contains(&"nvkm_pmu_fini"));
        assert!(names.contains(&"nvkm_mc_disable"));
        assert!(names.contains(&"nvkm_mc_reset"));
        assert!(names.contains(&"gk104_fifo_fini"));
        assert!(names.contains(&"gk104_clkgate_fini"));
        assert!(names.contains(&"nvkm_therm_clkgate_fini"));
        assert!(names.contains(&"g84_therm_fini"));
    }

    #[test]
    fn kepler_patch_set_targets_correct_functions() {
        let ps = PatchSet::kepler_warm_handoff();
        assert_eq!(ps.module_name, "nouveau");
        assert_eq!(ps.targets.len(), 5);
    }

    #[test]
    fn nvidia_patch_set_targets_correct_functions() {
        let ps = PatchSet::nvidia_warm_handoff();
        assert_eq!(ps.module_name, "nvidia");
        assert_eq!(ps.targets.len(), 6);

        let names: Vec<&str> = ps.targets.iter().map(|t| t.symbol.as_str()).collect();
        assert!(names.contains(&"nv_pci_remove"));
        assert!(names.contains(&"gpuStateUnload_IMPL"));
        assert!(names.contains(&"gpuStateDestroy_IMPL"));
        assert!(names.contains(&"_deviceTeardown"));
        assert!(names.contains(&"clTeardown_IMPL"));
        assert!(names.contains(&"fecsBufferTeardown"));

        assert!(ps.targets.iter().all(|t| t.strategy == PatchStrategy::RetAtEntry));
    }

    #[test]
    fn by_name_resolves_known_sets() {
        assert!(PatchSet::by_name("volta_warm_handoff").is_some());
        assert!(PatchSet::by_name("kepler_warm_handoff").is_some());
        assert!(PatchSet::by_name("nvidia_warm_handoff").is_some());
        assert!(PatchSet::by_name("nonexistent").is_none());
    }

    #[test]
    fn patch_strategy_serde_roundtrip() {
        let ps = PatchSet::volta_warm_handoff();
        let json = serde_json::to_string(&ps).unwrap();
        let back: PatchSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "volta_warm_handoff");
        assert_eq!(back.targets.len(), 8);
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

    #[test]
    fn apply_single_patch_accepts_nop_sled() {
        // 0x90 x5 (single-byte NOP sled) + 0x55 (push rbp)
        let mut bytes = vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x55, 0x48, 0x89];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

        let target = PatchTarget {
            symbol: "test_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"))
            .unwrap();

        assert!(result.applied);
        assert_eq!(result.offset, Some(5));
        assert_eq!(bytes[5], RET_OPCODE);
        assert!(result.detail.contains("nop-padded"));
    }

    #[test]
    fn apply_single_patch_accepts_zero_pad() {
        // 0x00 x5 (null-padded ftrace site) + 0x55
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x55, 0x48, 0x89];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

        let target = PatchTarget {
            symbol: "test_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"))
            .unwrap();

        assert!(result.applied);
        assert_eq!(result.offset, Some(5));
        assert_eq!(bytes[5], RET_OPCODE);
        assert!(result.detail.contains("nop-padded"));
    }

    #[test]
    fn apply_single_patch_accepts_multibyte_nop() {
        // 0x0f 0x1f 0x44 0x00 0x00 (5-byte NOP) + 0x55
        let mut bytes = vec![0x0f, 0x1f, 0x44, 0x00, 0x00, 0x55, 0x48, 0x89];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

        let target = PatchTarget {
            symbol: "test_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"))
            .unwrap();

        assert!(result.applied);
        assert_eq!(result.offset, Some(5));
        assert_eq!(bytes[5], RET_OPCODE);
        assert!(result.detail.contains("nop-padded"));
    }

    #[test]
    fn apply_single_patch_rejects_mid_instruction() {
        // 0xE5 is not a valid ftrace lead byte
        let mut bytes = vec![0xe5, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56];
        let len = bytes.len();
        let symbols: HashMap<String, u64> = [("test_fn".into(), 0u64)].into_iter().collect();

        let target = PatchTarget {
            symbol: "test_fn".into(),
            strategy: PatchStrategy::RetAfterFtrace,
        };

        let result = apply_single_patch(&mut bytes, len, &symbols, &target, Path::new("test.ko"));
        assert!(matches!(result, Err(PatchError::NoFtraceCallSite { found: 0xe5, .. })));
    }

    #[test]
    fn rename_module_identity_replaces_nul_bounded() {
        let mut data = vec![0u8; 32];
        data[0] = 0;
        data[1..7].copy_from_slice(b"nvidia");
        data[7] = 0;
        data[8..14].copy_from_slice(b"nvidia");
        data[14] = b'=';

        let count = rename_module_identity(&mut data, "nvidia", "nvsov").unwrap();
        assert_eq!(count, 2);
        assert_eq!(&data[1..6], b"nvsov");
        assert_eq!(data[6], 0); // NUL-padded since "nvsov" is shorter
        assert_eq!(&data[8..13], b"nvsov");
    }

    #[test]
    fn rename_rejects_longer_new_name() {
        let mut data = vec![0u8; 16];
        let result = rename_module_identity(&mut data, "nv", "nvidia_sovereign_extended");
        assert!(result.is_err());
    }

    #[test]
    fn patch_set_min_applied_default_serde() {
        // When min_applied is absent from JSON, it defaults to 1
        let json = r#"{"name":"test","module_name":"test","targets":[]}"#;
        let ps: PatchSet = serde_json::from_str(json).unwrap();
        assert_eq!(ps.min_applied, 1);
    }

    #[test]
    fn patch_set_min_applied_explicit_serde() {
        let json = r#"{"name":"test","module_name":"test","targets":[],"min_applied":3}"#;
        let ps: PatchSet = serde_json::from_str(json).unwrap();
        assert_eq!(ps.min_applied, 3);
    }
}
