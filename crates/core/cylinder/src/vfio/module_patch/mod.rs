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
//! ```ignore
//! use toadstool_cylinder::vfio::module_patch::*;
//!
//! let stock_ko = std::path::Path::new("/lib/modules/6.17.9/nouveau.ko");
//! let patch_set = PatchSet::volta_warm_handoff();
//! let patched_path = patch_module_with_rename(stock_ko, &patch_set, None).unwrap();
//! // patched_path is now ready for insmod
//! ```

mod apply;
mod elf;
mod identity;
mod patch_sets;
mod types;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use apply::reapply_nops;
pub use elf::{
    NmResolver, SymbolResolver, normalize_relocations, nullify_relocations_at, strip_ksymtab,
    strip_ksymtab_sections,
};
pub use identity::rename_module_identity;
pub use types::{ModulePatchResult, PatchError, PatchResult, PatchSet, PatchStrategy, PatchTarget};

use apply::apply_single_patch;
use elf::resolve_symbol_file_offsets;

/// Patch a stock kernel module, optionally rename it, and write the result
/// to a temporary file.
///
/// Reads the source `.ko`, resolves symbol offsets via `nm`, applies the
/// requested patches, and writes the result to `$TMPDIR/toadstool-patched-{name}.ko`.
///
/// If `rename` is `Some((old, new))`, the module identity is rewritten so
/// it can be loaded alongside the original (avoids "module already loaded"
/// rejection).
///
/// Returns the path to the patched module and per-target results.
pub(crate) fn patch_module_with_rename(
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

    let raw_bytes = std::fs::read(source_ko).map_err(PatchError::ReadFailed)?;
    let mut module_bytes = if source_ko.extension().and_then(|e| e.to_str()) == Some("zst") {
        let mut decoder =
            ruzstd::decoding::StreamingDecoder::new(raw_bytes.as_slice()).map_err(|e| {
                PatchError::ReadFailed(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("zstd init for {}: {e}", source_ko.display()),
                ))
            })?;
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).map_err(|e| {
            PatchError::ReadFailed(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("zstd decompress for {}: {e}", source_ko.display()),
            ))
        })?;
        tracing::info!(
            compressed = raw_bytes.len(),
            decompressed = decompressed.len(),
            "decompressed .ko.zst module"
        );
        decompressed
    } else {
        raw_bytes
    };
    let module_size = module_bytes.len();

    // Normalize relocations for kernel 6.17+ compatibility.
    // Proprietary blobs (nvidia-470) have nonzero values at R_X86_64_64
    // relocation targets that kernel 6.17+ rejects.
    let reloc_normalized = normalize_relocations(&mut module_bytes).unwrap_or_else(|e| {
        tracing::debug!(error = %e, "relocation normalization skipped");
        0
    });
    if reloc_normalized > 0 {
        tracing::info!(reloc_normalized, "normalized proprietary blob relocations");
    }

    let symbols = resolve_symbol_file_offsets(&module_bytes);
    tracing::info!(
        symbols_resolved = symbols.len(),
        "resolved symbol file offsets from ELF (section-aware)"
    );

    let mut patches = Vec::new();
    let mut applied_count = 0;

    for target in &patch_set.targets {
        let result = apply_single_patch(
            &mut module_bytes,
            module_size,
            &symbols,
            target,
            source_ko,
            0, // offsets are already file-absolute
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
            old_name,
            new_name,
            rename_count,
            "module identity renamed for dual-load injection"
        );
        patches.push(PatchResult {
            symbol: format!("__module_rename:{old_name}→{new_name}"),
            applied: rename_count > 0,
            offset: None,
            detail: format!("{rename_count} identity replacements"),
        });

        // Strip __ksymtab to prevent "exports duplicate symbol" on co-load
        let ksym_zeroed = strip_ksymtab(&mut module_bytes).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "ksymtab stripping failed (non-fatal)");
            0
        });
        if ksym_zeroed > 0 {
            patches.push(PatchResult {
                symbol: "__ksymtab_strip".into(),
                applied: true,
                offset: None,
                detail: format!("{ksym_zeroed} bytes zeroed in ksymtab/kcrctab/ksymtab_strings"),
            });
        }
    }

    // Nullify relocation entries that target our NOP patches.
    // Kernel 6.17+ rejects nonzero values at relocation targets, and
    // our NOP bytes (0xC3, 0x31, etc.) would be treated as violations.
    let patch_ranges: Vec<(usize, usize)> = patches
        .iter()
        .filter_map(|p| {
            if !p.applied {
                return None;
            }
            let off = p.offset?;
            if !(p.detail.starts_with("ret") || p.detail.starts_with("nopcall")) {
                return None;
            }
            let len = if p.detail.contains("6B") {
                6
            } else if p.detail.starts_with("ret1") || p.detail.starts_with("nopcall") {
                5
            } else if p.detail.starts_with("ret@") {
                1
            } else {
                3
            };
            Some((off, len))
        })
        .collect();
    if !patch_ranges.is_empty() {
        nullify_relocations_at(&mut module_bytes, &patch_ranges);
    }

    // Strip PKCS#7 module signature appended by the kernel build system.
    // Patching invalidates the signature, causing EKEYREJECTED (errno 129)
    // on finit_module even when CONFIG_MODULE_SIG_FORCE is not set.
    // The signature is appended after the ELF with a 28-byte trailer:
    //   [sig_data...] [12-byte info struct] "~Module signature appended~\n"
    const SIG_MAGIC: &[u8] = b"~Module signature appended~\n";
    if module_bytes.len() > SIG_MAGIC.len() + 12 {
        let tail_start = module_bytes.len() - SIG_MAGIC.len();
        if &module_bytes[tail_start..] == SIG_MAGIC {
            let info_start = tail_start - 12;
            let sig_len = u32::from_be_bytes([
                module_bytes[info_start + 8],
                module_bytes[info_start + 9],
                module_bytes[info_start + 10],
                module_bytes[info_start + 11],
            ]) as usize;
            let strip_from = info_start.saturating_sub(sig_len);
            tracing::info!(
                sig_len,
                stripped_bytes = module_bytes.len() - strip_from,
                "stripping PKCS#7 module signature (patching invalidated it)"
            );
            module_bytes.truncate(strip_from);
        }
    }

    let output_name = rename.map(|(_, new)| new).unwrap_or(&patch_set.module_name);
    let patched_path = std::env::temp_dir()
        .join(format!("toadstool-patched-{output_name}.ko"))
        .display()
        .to_string();
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

/// Get the path where a patched module would be written.
#[must_use]
pub(crate) fn patched_module_path(module_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("toadstool-patched-{module_name}.ko"))
}

/// Clean up a previously patched module from the temp directory.
pub(crate) fn cleanup_patched_module(module_name: &str) -> Result<(), std::io::Error> {
    let path = patched_module_path(module_name);
    if path.exists() {
        std::fs::remove_file(&path)?;
        tracing::debug!(path = %path.display(), "cleaned up patched module");
    }
    Ok(())
}
