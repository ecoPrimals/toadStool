// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::path::Path;

use super::types::{ModulePatchResult, PatchError, PatchResult, PatchStrategy, PatchTarget};

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
pub(crate) const RET_OPCODE: u8 = 0xc3;

/// Ftrace call site size in bytes.
const FTRACE_CALL_SIZE: usize = 5;

/// Apply a single patch target to the module bytes.
pub(crate) fn apply_single_patch(
    module_bytes: &mut [u8],
    module_size: usize,
    symbols: &HashMap<String, u64>,
    target: &PatchTarget,
    source_path: &Path,
    text_section_offset: usize,
) -> Result<PatchResult, PatchError> {
    let &sym_offset = symbols.get(&target.symbol).ok_or_else(|| {
        PatchError::SymbolNotFound {
            symbol: target.symbol.clone(),
            module: source_path.display().to_string(),
        }
    })?;

    let offset = sym_offset as usize + text_section_offset;

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
            let first_byte = module_bytes[offset];
            let has_ftrace = first_byte == FTRACE_CALL_OPCODE
                || first_byte == MULTIBYTE_NOP_LEAD
                || first_byte == NOP_OPCODE;

            if has_ftrace && offset + FTRACE_CALL_SIZE < module_bytes.len() {
                let patch_off = offset + FTRACE_CALL_SIZE;
                let original_byte = module_bytes[patch_off];
                module_bytes[patch_off] = RET_OPCODE;

                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: true,
                    offset: Some(patch_off),
                    detail: format!(
                        "ret@{patch_off:#x} (was {original_byte:#04x}, entry+5)"
                    ),
                })
            } else if offset < module_bytes.len() {
                let original_byte = module_bytes[offset];
                module_bytes[offset] = RET_OPCODE;

                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: true,
                    offset: Some(offset),
                    detail: format!(
                        "ret@{offset:#x} (was {original_byte:#04x}, entry+0)"
                    ),
                })
            } else {
                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: None,
                    detail: format!("insufficient space for ret at {offset:#x}"),
                })
            }
        }
        PatchStrategy::Ret0AtEntry => {
            let first_byte = module_bytes[offset];
            let has_ftrace = first_byte == FTRACE_CALL_OPCODE
                || first_byte == MULTIBYTE_NOP_LEAD
                || first_byte == NOP_OPCODE;

            if has_ftrace && offset + FTRACE_CALL_SIZE + 3 <= module_bytes.len() {
                let patch_off = offset + FTRACE_CALL_SIZE;
                let original_byte = module_bytes[patch_off];
                module_bytes[patch_off] = 0x31;         // xor
                module_bytes[patch_off + 1] = 0xc0;     // eax, eax
                module_bytes[patch_off + 2] = RET_OPCODE;

                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: true,
                    offset: Some(patch_off),
                    detail: format!(
                        "ret0@{patch_off:#x} (was {original_byte:#04x}, entry+5)"
                    ),
                })
            } else if offset + 3 <= module_bytes.len() {
                let original_byte = module_bytes[offset];
                module_bytes[offset] = 0x31;
                module_bytes[offset + 1] = 0xc0;
                module_bytes[offset + 2] = RET_OPCODE;

                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: true,
                    offset: Some(offset),
                    detail: format!(
                        "ret0@{offset:#x} (was {original_byte:#04x}, entry+0, 3B)"
                    ),
                })
            } else {
                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: None,
                    detail: format!(
                        "insufficient space for ret0 at {offset:#x}"
                    ),
                })
            }
        }
        PatchStrategy::Ret1AtEntry => {
            let first_byte = module_bytes[offset];
            let has_ftrace = first_byte == FTRACE_CALL_OPCODE
                || first_byte == MULTIBYTE_NOP_LEAD
                || first_byte == NOP_OPCODE;

            if has_ftrace && offset + FTRACE_CALL_SIZE + 5 <= module_bytes.len() {
                // Ftrace preamble present: patch at entry+5 with
                // `xor eax,eax; inc eax; ret` (5 bytes).
                let patch_off = offset + FTRACE_CALL_SIZE;
                let original_byte = module_bytes[patch_off];
                module_bytes[patch_off] = 0x31;         // xor
                module_bytes[patch_off + 1] = 0xc0;     // eax, eax
                module_bytes[patch_off + 2] = 0xff;     // inc
                module_bytes[patch_off + 3] = 0xc0;     // eax
                module_bytes[patch_off + 4] = RET_OPCODE;

                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: true,
                    offset: Some(patch_off),
                    detail: format!(
                        "ret1@{patch_off:#x} (was {original_byte:#04x}, entry+5)"
                    ),
                })
            } else if offset + 6 <= module_bytes.len() {
                // No ftrace preamble (proprietary blob function): patch
                // at entry+0 with `mov eax, 1; ret` (6 bytes).
                let original_byte = module_bytes[offset];
                module_bytes[offset] = 0xb8;         // mov eax,
                module_bytes[offset + 1] = 0x01;     // 1
                module_bytes[offset + 2] = 0x00;
                module_bytes[offset + 3] = 0x00;
                module_bytes[offset + 4] = 0x00;
                module_bytes[offset + 5] = RET_OPCODE;

                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: true,
                    offset: Some(offset),
                    detail: format!(
                        "ret1@{offset:#x} (was {original_byte:#04x}, entry+0, 6B)"
                    ),
                })
            } else {
                Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: None,
                    detail: format!(
                        "insufficient space for ret1 at {offset:#x}"
                    ),
                })
            }
        }
        PatchStrategy::NopCallAt(call_offset) => {
            let patch_off = offset + call_offset;
            if patch_off + 5 > module_bytes.len() {
                return Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: None,
                    detail: format!(
                        "NopCallAt offset {call_offset:#x} out of bounds at {offset:#x}"
                    ),
                });
            }

            let orig = module_bytes[patch_off];
            // xor eax,eax (2B) + 3-byte NOP (0f 1f 00)
            module_bytes[patch_off] = 0x31;
            module_bytes[patch_off + 1] = 0xc0;
            module_bytes[patch_off + 2] = 0x0f;
            module_bytes[patch_off + 3] = 0x1f;
            module_bytes[patch_off + 4] = 0x00;

            Ok(PatchResult {
                symbol: target.symbol.clone(),
                applied: true,
                offset: Some(patch_off),
                detail: format!(
                    "nopcall@{patch_off:#x} (was {orig:#04x}, fn+{call_offset:#x}, 5B)"
                ),
            })
        }
        PatchStrategy::PatchByteAt { fn_offset, expected, replacement } => {
            let patch_off = offset + fn_offset;
            if patch_off >= module_bytes.len() {
                return Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: None,
                    detail: format!(
                        "PatchByteAt offset {fn_offset:#x} out of bounds at {offset:#x}"
                    ),
                });
            }
            let actual = module_bytes[patch_off];
            if actual != expected {
                return Ok(PatchResult {
                    symbol: target.symbol.clone(),
                    applied: false,
                    offset: Some(patch_off),
                    detail: format!(
                        "byte@{patch_off:#x} is {actual:#04x}, expected {expected:#04x}"
                    ),
                });
            }
            module_bytes[patch_off] = replacement;
            Ok(PatchResult {
                symbol: target.symbol.clone(),
                applied: true,
                offset: Some(patch_off),
                detail: format!(
                    "byte@{patch_off:#x}: {expected:#04x}\u{2192}{replacement:#04x} (fn+{fn_offset:#x})"
                ),
            })
        }
    }
}

pub fn reapply_nops(module_bytes: &mut [u8], result: &ModulePatchResult) {
    let mut restored = 0usize;
    for patch in &result.patches {
        if !patch.applied {
            continue;
        }
        let Some(off) = patch.offset else { continue };

        if patch.detail.contains("entry+0, 6B") {
            // mov eax,1; ret (6 bytes at entry+0)
            if off + 6 <= module_bytes.len() {
                module_bytes[off] = 0xb8;
                module_bytes[off + 1] = 0x01;
                module_bytes[off + 2] = 0x00;
                module_bytes[off + 3] = 0x00;
                module_bytes[off + 4] = 0x00;
                module_bytes[off + 5] = RET_OPCODE;
                restored += 1;
            }
        } else if patch.detail.starts_with("ret0") {
            // xor eax,eax; ret (3 bytes)
            if off + 3 <= module_bytes.len() {
                module_bytes[off] = 0x31;
                module_bytes[off + 1] = 0xc0;
                module_bytes[off + 2] = RET_OPCODE;
                restored += 1;
            }
        } else if patch.detail.starts_with("ret1") {
            // xor eax,eax; inc eax; ret (5 bytes at entry+5)
            if off + 5 <= module_bytes.len() {
                module_bytes[off] = 0x31;
                module_bytes[off + 1] = 0xc0;
                module_bytes[off + 2] = 0xff;
                module_bytes[off + 3] = 0xc0;
                module_bytes[off + 4] = RET_OPCODE;
                restored += 1;
            }
        } else if patch.detail.starts_with("ret@") {
            // Single ret byte
            if off < module_bytes.len() {
                module_bytes[off] = RET_OPCODE;
                restored += 1;
            }
        } else if patch.detail.starts_with("nopcall@") {
            // xor eax,eax + 3-byte NOP (5 bytes)
            if off + 5 <= module_bytes.len() {
                module_bytes[off] = 0x31;
                module_bytes[off + 1] = 0xc0;
                module_bytes[off + 2] = 0x0f;
                module_bytes[off + 3] = 0x1f;
                module_bytes[off + 4] = 0x00;
                restored += 1;
            }
        }
    }
    if restored > 0 {
        tracing::info!(restored, "re-applied NOP patches after post-objcopy normalization");
    }
}
