// SPDX-License-Identifier: AGPL-3.0-or-later
use super::super::types::PatchError;

/// Nullify relocation entries that target the given byte ranges.
///
/// When we write NOP bytes at function entries, we clobber relocation
/// targets. Kernel 6.17+ rejects nonzero relocation targets, so we must
/// zero out the corresponding Elf64_Rela entries (set r_info to 0 =
/// R_X86_64_NONE). This effectively tells the kernel to skip them.
pub fn nullify_relocations_at(module_bytes: &mut [u8], patch_ranges: &[(usize, usize)]) -> usize {
    let e_shoff = u64::from_le_bytes(
        module_bytes
            .get(40..48)
            .and_then(|s| s.try_into().ok())
            .unwrap_or([0; 8]),
    ) as usize;
    let e_shentsize = u16::from_le_bytes(
        module_bytes
            .get(58..60)
            .and_then(|s| s.try_into().ok())
            .unwrap_or([0; 2]),
    ) as usize;
    let e_shnum = u16::from_le_bytes(
        module_bytes
            .get(60..62)
            .and_then(|s| s.try_into().ok())
            .unwrap_or([0; 2]),
    ) as usize;

    if e_shentsize == 0 || e_shoff == 0 {
        return 0;
    }

    const SHT_RELA: u32 = 4;
    let mut nullified = 0usize;

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
        let sh_info = u32::from_le_bytes(
            module_bytes[sh_start + 44..sh_start + 48]
                .try_into()
                .unwrap_or([0; 4]),
        ) as usize;

        // Resolve the target section's file offset so we can convert
        // r_offset (section-relative) to file offset for comparison
        // with our patch ranges (which use file offsets from nm).
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

        let entry_size = 24usize;
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
            ) as usize;
            let r_info = u64::from_le_bytes(
                module_bytes[rela_off + 8..rela_off + 16]
                    .try_into()
                    .unwrap_or([0; 8]),
            );
            if r_info == 0 {
                continue;
            } // already nullified

            let target_file_off = target_sh_offset + r_offset;

            let r_type = (r_info & 0xFFFF_FFFF) as u32;
            let reloc_size: usize = match r_type {
                1 => 8,          // R_X86_64_64
                2 | 4 | 11 => 4, // PC32, PLT32, 32S
                _ => continue,
            };

            for &(range_start, range_len) in patch_ranges {
                let range_end = range_start + range_len;
                let reloc_end = target_file_off + reloc_size;
                // Check if the relocation's target bytes overlap our patch
                if target_file_off < range_end && reloc_end > range_start {
                    module_bytes[rela_off + 8..rela_off + 16].copy_from_slice(&0u64.to_le_bytes());
                    nullified += 1;
                    break;
                }
            }
        }
    }

    if nullified > 0 {
        tracing::info!(
            nullified,
            "nullified relocation entries overlapping NOP patches"
        );
    }
    nullified
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

    let e_shoff = u64::from_le_bytes(module_bytes[40..48].try_into().unwrap_or([0; 8])) as usize;
    let e_shentsize =
        u16::from_le_bytes(module_bytes[58..60].try_into().unwrap_or([0; 2])) as usize;
    let e_shnum = u16::from_le_bytes(module_bytes[60..62].try_into().unwrap_or([0; 2])) as usize;

    if e_shoff == 0 || e_shentsize < 64 || e_shnum == 0 {
        return Err(PatchError::NmFailed {
            path: "(in-memory)".into(),
            detail: "invalid section header table".into(),
        });
    }

    const SHT_RELA: u32 = 4;
    const R_X86_64_64: u32 = 1;
    const R_X86_64_PC32: u32 = 2;
    const R_X86_64_PLT32: u32 = 4;
    const R_X86_64_32S: u32 = 11;

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

            let target_file_off = target_sh_offset + r_offset as usize;

            // Handle 32-bit relocations (PC32, PLT32, 32S)
            if r_type == R_X86_64_PC32 || r_type == R_X86_64_PLT32 || r_type == R_X86_64_32S {
                if target_file_off + 4 > module_bytes.len() || target_file_off < 64 {
                    continue;
                }
                let existing = i32::from_le_bytes(
                    module_bytes[target_file_off..target_file_off + 4]
                        .try_into()
                        .unwrap_or([0; 4]),
                );
                if existing == 0 {
                    continue;
                }
                let old_addend = i64::from_le_bytes(
                    module_bytes[rela_off + 16..rela_off + 24]
                        .try_into()
                        .unwrap_or([0; 8]),
                );
                let new_addend = old_addend.wrapping_add(existing as i64);
                module_bytes[rela_off + 16..rela_off + 24]
                    .copy_from_slice(&new_addend.to_le_bytes());
                module_bytes[target_file_off..target_file_off + 4]
                    .copy_from_slice(&0i32.to_le_bytes());
                normalized += 1;
                continue;
            }

            // Handle 64-bit absolute relocations (R_X86_64_64)
            if r_type != R_X86_64_64 {
                continue;
            }

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

            module_bytes[rela_off + 16..rela_off + 24].copy_from_slice(&new_addend.to_le_bytes());
            module_bytes[target_file_off..target_file_off + 8].copy_from_slice(&0i64.to_le_bytes());

            normalized += 1;
        }
    }

    tracing::info!(
        normalized,
        "relocations normalized for kernel 6.17+ compat (types 1,2,4,11)"
    );
    Ok(normalized)
}
