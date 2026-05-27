// SPDX-License-Identifier: AGPL-3.0-or-later
use super::super::types::PatchError;

/// Remove ksymtab export sections from an ELF `.ko` file.
///
/// Strips `__ksymtab`, `__kcrctab`, `__ksymtab_strings`, and
/// `.rela__ksymtab` — the same sections that `objcopy --remove-section`
/// would remove. This prevents duplicate-symbol errors when dual-loading
/// a DKMS module alongside the host driver.
///
/// Pure Rust via the `object` crate — no external `objcopy` process.
pub fn strip_ksymtab_sections(
    input: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), PatchError> {
    const STRIP_SECTIONS: &[&str] = &[
        "__ksymtab",
        "__kcrctab",
        "__ksymtab_strings",
        ".rela__ksymtab",
    ];

    let data = std::fs::read(input).map_err(PatchError::ReadFailed)?;

    let mut patched = data;
    zero_elf_sections_by_name(&mut patched, STRIP_SECTIONS).map_err(|e| PatchError::NmFailed {
        path: input.display().to_string(),
        detail: e,
    })?;

    std::fs::write(output, &patched).map_err(|e| PatchError::WriteFailed {
        path: output.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// Zero out sections by name in raw ELF bytes (in-place).
///
/// Zeroes both the section content and sets `sh_size = 0` in the section
/// header. This effectively removes the section's payload while preserving
/// ELF structure integrity (section count, string table, etc. unchanged).
pub(crate) fn zero_elf_sections_by_name(data: &mut [u8], names: &[&str]) -> Result<(), String> {
    if data.len() < 64 || &data[0..4] != b"\x7fELF" || data[4] != 2 || data[5] != 1 {
        return Err("not a 64-bit little-endian ELF".into());
    }

    let e_shoff = u64::from_le_bytes(
        data[40..48]
            .try_into()
            .map_err(|e| format!("malformed ELF e_shoff: {e}"))?,
    ) as usize;
    let e_shentsize = u16::from_le_bytes(
        data[58..60]
            .try_into()
            .map_err(|e| format!("malformed ELF e_shentsize: {e}"))?,
    ) as usize;
    let e_shnum = u16::from_le_bytes(
        data[60..62]
            .try_into()
            .map_err(|e| format!("malformed ELF e_shnum: {e}"))?,
    ) as usize;
    let e_shstrndx = u16::from_le_bytes(
        data[62..64]
            .try_into()
            .map_err(|e| format!("malformed ELF e_shstrndx: {e}"))?,
    ) as usize;

    if e_shstrndx >= e_shnum {
        return Err("invalid shstrndx".into());
    }

    let shstrtab_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstrtab_off = u64::from_le_bytes(
        data.get(shstrtab_hdr + 24..shstrtab_hdr + 32)
            .ok_or_else(|| "truncated ELF section string table header".to_string())?
            .try_into()
            .map_err(|e| format!("malformed ELF shstrtab offset: {e}"))?,
    ) as usize;

    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        if sh + 40 > data.len() {
            return Err(format!("truncated ELF section header {i}"));
        }
        let sh_name_idx = u32::from_le_bytes(
            data[sh..sh + 4]
                .try_into()
                .map_err(|e| format!("malformed ELF sh_name for section {i}: {e}"))?,
        ) as usize;
        let name_start = shstrtab_off + sh_name_idx;

        let mut end = name_start;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }
        let name = String::from_utf8_lossy(&data[name_start..end]).to_string();

        if !names.iter().any(|&n| n == name) {
            continue;
        }

        let sh_offset = u64::from_le_bytes(
            data[sh + 24..sh + 32]
                .try_into()
                .map_err(|e| format!("malformed ELF sh_offset for section {i}: {e}"))?,
        ) as usize;
        let sh_size = u64::from_le_bytes(
            data[sh + 32..sh + 40]
                .try_into()
                .map_err(|e| format!("malformed ELF sh_size for section {i}: {e}"))?,
        ) as usize;

        if sh_offset + sh_size <= data.len() {
            data[sh_offset..sh_offset + sh_size].fill(0);
        }
        data[sh + 32..sh + 40].copy_from_slice(&0u64.to_le_bytes());

        tracing::debug!(section = name.as_str(), "zeroed ksymtab section ({sh_size} bytes)");
    }

    Ok(())
}

/// Strip kernel symbol export tables from a `.ko` binary.
///
/// Zeros the content of `__ksymtab`, `__kcrctab`, and `__ksymtab_strings`
/// ELF sections. This prevents the module from exporting symbols that would
/// collide with an already-loaded copy (e.g. `nvsov` alongside `nvidia`).
///
/// The kernel loader checks `__ksymtab` entries against all loaded modules;
/// any duplicate causes `ENOEXEC` ("exports duplicate symbol"). Since
/// renamed modules used for warm handoff are leaf modules (nothing depends
/// on them), stripping exports is safe.
///
/// Returns the total bytes zeroed.
pub fn strip_ksymtab(module_bytes: &mut [u8]) -> Result<usize, PatchError> {
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
    let e_shstrndx = u16::from_le_bytes(
        module_bytes[62..64].try_into().unwrap_or([0; 2]),
    ) as usize;

    if e_shoff == 0 || e_shentsize < 64 || e_shnum == 0 || e_shstrndx >= e_shnum {
        return Err(PatchError::NmFailed {
            path: "(in-memory)".into(),
            detail: "invalid ELF section header table".into(),
        });
    }

    let shstr_hdr_off = e_shoff + e_shstrndx * e_shentsize;
    let shstr_offset = u64::from_le_bytes(
        module_bytes[shstr_hdr_off + 24..shstr_hdr_off + 32]
            .try_into().unwrap_or([0; 8]),
    ) as usize;
    let _shstr_size = u64::from_le_bytes(
        module_bytes[shstr_hdr_off + 32..shstr_hdr_off + 40]
            .try_into().unwrap_or([0; 8]),
    ) as usize;

    let mut total_zeroed = 0usize;

    for i in 0..e_shnum {
        let sh_start = e_shoff + i * e_shentsize;
        if sh_start + e_shentsize > module_bytes.len() {
            break;
        }

        let sh_name_idx = u32::from_le_bytes(
            module_bytes[sh_start..sh_start + 4]
                .try_into().unwrap_or([0; 4]),
        ) as usize;

        let name_off = shstr_offset + sh_name_idx;
        if name_off >= module_bytes.len() {
            continue;
        }

        let name_end = module_bytes[name_off..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(0);
        let name_str = std::str::from_utf8(
            &module_bytes[name_off..name_off + name_end]
        ).unwrap_or("");

        let matched = name_str.contains("ksymtab") || name_str.contains("kcrctab");
        if !matched {
            continue;
        }

        let sh_offset = u64::from_le_bytes(
            module_bytes[sh_start + 24..sh_start + 32]
                .try_into().unwrap_or([0; 8]),
        ) as usize;
        let sh_size = u64::from_le_bytes(
            module_bytes[sh_start + 32..sh_start + 40]
                .try_into().unwrap_or([0; 8]),
        ) as usize;

        if sh_offset + sh_size <= module_bytes.len() && sh_size > 0 {
            tracing::info!(
                section = name_str,
                offset = format_args!("{sh_offset:#x}"),
                size = sh_size,
                "zeroing export section to prevent symbol collisions"
            );
            module_bytes[sh_offset..sh_offset + sh_size].fill(0);
            total_zeroed += sh_size;
        }
    }

    if total_zeroed > 0 {
        tracing::info!(total_zeroed, "stripped kernel symbol exports for dual-load isolation");
    }

    Ok(total_zeroed)
}
