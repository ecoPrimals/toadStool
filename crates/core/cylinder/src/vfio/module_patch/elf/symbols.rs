// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::path::Path;

use super::super::types::PatchError;

/// Abstraction over symbol resolution for `.ko` files.
///
/// Separates the mechanism of finding symbol offsets from the patching logic,
/// enabling alternative implementations (e.g. pure-Rust ELF parsing) without
/// touching the patcher.
pub trait SymbolResolver {
    fn resolve(&self, ko_path: &Path) -> Result<HashMap<String, u64>, PatchError>;
}

/// Symbol resolver backed by the `nm` command-line tool.
///
/// Delegates to [`crate::vfio::kmod::nm_text_symbols`] — the canonical
/// implementation — to avoid duplication.
pub struct NmResolver;

impl SymbolResolver for NmResolver {
    fn resolve(&self, ko_path: &Path) -> Result<HashMap<String, u64>, PatchError> {
        let syms = crate::vfio::kmod::nm_text_symbols(ko_path).map_err(|e| {
            PatchError::NmFailed {
                path: ko_path.display().to_string(),
                detail: format!("{e}"),
            }
        })?;
        Ok(syms.into_iter().collect())
    }
}

/// Resolve symbol FILE offsets by parsing ELF structures directly.
///
/// Unlike `nm` (which returns section-relative virtual addresses), this
/// computes the actual byte offset within the `.ko` file by adding the
/// symbol's st_value to the target section's sh_offset. This correctly
/// handles symbols in .text, .init.text, and any other section.
pub(crate) fn resolve_symbol_file_offsets(elf: &[u8]) -> HashMap<String, u64> {
    let mut result = HashMap::new();
    if elf.len() < 64 { return result; }

    let e_shoff = u64::from_le_bytes(elf[40..48].try_into().unwrap_or([0; 8])) as usize;
    let e_shentsize = u16::from_le_bytes(elf[58..60].try_into().unwrap_or([0; 2])) as usize;
    let e_shnum = u16::from_le_bytes(elf[60..62].try_into().unwrap_or([0; 2])) as usize;

    if e_shentsize == 0 || e_shoff == 0 || e_shnum == 0 { return result; }

    const SHT_SYMTAB: u32 = 2;
    const STT_FUNC: u8 = 2;

    // Build section offset table: section_index -> sh_offset
    let mut section_offsets = Vec::with_capacity(e_shnum);
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        if sh + 32 > elf.len() {
            section_offsets.push(0u64);
            continue;
        }
        let off = u64::from_le_bytes(elf[sh + 24..sh + 32].try_into().unwrap_or([0; 8]));
        section_offsets.push(off);
    }

    // Find symtab section
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        if sh + e_shentsize > elf.len() { break; }

        let sh_type = u32::from_le_bytes(elf[sh + 4..sh + 8].try_into().unwrap_or([0; 4]));
        if sh_type != SHT_SYMTAB { continue; }

        let sym_offset = u64::from_le_bytes(elf[sh + 24..sh + 32].try_into().unwrap_or([0; 8])) as usize;
        let sym_size = u64::from_le_bytes(elf[sh + 32..sh + 40].try_into().unwrap_or([0; 8])) as usize;
        let sym_link = u32::from_le_bytes(elf[sh + 40..sh + 44].try_into().unwrap_or([0; 4])) as usize;

        // sh_link points to the associated strtab section
        if sym_link >= e_shnum { continue; }
        let strtab_sh = e_shoff + sym_link * e_shentsize;
        if strtab_sh + 32 > elf.len() { continue; }
        let strtab_off = u64::from_le_bytes(
            elf[strtab_sh + 24..strtab_sh + 32].try_into().unwrap_or([0; 8]),
        ) as usize;

        let entry_size = 24usize; // sizeof(Elf64_Sym)
        let num_syms = sym_size / entry_size;

        for j in 0..num_syms {
            let sym = sym_offset + j * entry_size;
            if sym + entry_size > elf.len() { break; }

            let st_name = u32::from_le_bytes(elf[sym..sym + 4].try_into().unwrap_or([0; 4])) as usize;
            let st_info = elf[sym + 4];
            let st_shndx = u16::from_le_bytes(elf[sym + 6..sym + 8].try_into().unwrap_or([0; 2])) as usize;
            let st_value = u64::from_le_bytes(elf[sym + 8..sym + 16].try_into().unwrap_or([0; 8]));

            let st_type = st_info & 0xf;
            if st_type != STT_FUNC { continue; }
            if st_shndx == 0 || st_shndx >= section_offsets.len() { continue; }

            let name_off = strtab_off + st_name;
            if name_off >= elf.len() { continue; }
            let name_end = elf[name_off..].iter().position(|&b| b == 0)
                .map(|p| name_off + p)
                .unwrap_or(elf.len());
            if let Ok(name) = std::str::from_utf8(&elf[name_off..name_end]) {
                let file_offset = section_offsets[st_shndx] + st_value;
                result.insert(name.to_string(), file_offset);
            }
        }
    }

    result
}
