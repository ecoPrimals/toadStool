// SPDX-License-Identifier: AGPL-3.0-or-later

use super::KernelHealthError;

pub(crate) fn elf_parse_err(detail: impl Into<String>) -> KernelHealthError {
    KernelHealthError::ElfParse(detail.into())
}

pub(crate) fn read_u16_le(data: &[u8], off: usize) -> Result<u16, KernelHealthError> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| elf_parse_err(format!("invalid u16 at offset 0x{off:x}")))?;
    Ok(u16::from_le_bytes(bytes))
}

pub(crate) fn read_u32_le(data: &[u8], off: usize) -> Result<u32, KernelHealthError> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| elf_parse_err(format!("invalid u32 at offset 0x{off:x}")))?;
    Ok(u32::from_le_bytes(bytes))
}

pub(crate) fn read_u64_le(data: &[u8], off: usize) -> Result<u64, KernelHealthError> {
    let bytes: [u8; 8] = data
        .get(off..off + 8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| elf_parse_err(format!("invalid u64 at offset 0x{off:x}")))?;
    Ok(u64::from_le_bytes(bytes))
}

pub(crate) fn read_cstr(data: &[u8], start: usize) -> String {
    let mut end = start;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    String::from_utf8_lossy(&data[start..end]).to_string()
}

pub(crate) fn resolve_symtab(
    elf_data: &[u8],
    e_shoff: usize,
    e_shentsize: usize,
    symtab_idx: usize,
) -> Result<(usize, usize, usize), KernelHealthError> {
    let sh = e_shoff + symtab_idx * e_shentsize;
    let symtab_off = read_u64_le(elf_data, sh + 24)? as usize;
    let symtab_entsize = read_u64_le(elf_data, sh + 56)? as usize;

    let strtab_idx = read_u32_le(elf_data, sh + 40)? as usize;
    let strtab_sh = e_shoff + strtab_idx * e_shentsize;
    let strtab_off = read_u64_le(elf_data, strtab_sh + 24)? as usize;

    Ok((symtab_off, symtab_entsize, strtab_off))
}
