// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use super::KernelHealthError;
use super::elf::{read_cstr, read_u16_le, read_u32_le, read_u64_le, resolve_symtab};
use super::paths::kernel_release;

/// Parse the `.gnu.linkonce.this_module` RELA entries from an existing `.ko`
/// file to determine what `struct module` init/exit offsets that module was
/// built against.
///
/// The init and exit function pointers are stored via relocations into
/// `.gnu.linkonce.this_module`. The relocation `r_offset` values reveal the
/// byte offsets of `init` and `exit` within `struct module`.
pub fn reference_module_offsets(ko_path: &Path) -> Result<(u64, u64), KernelHealthError> {
    let data = std::fs::read(ko_path)?;
    parse_this_module_rela_offsets(&data)
}

pub(crate) fn parse_this_module_rela_offsets(
    elf_data: &[u8],
) -> Result<(u64, u64), KernelHealthError> {
    if elf_data.len() < 64 || &elf_data[0..4] != b"\x7fELF" || elf_data[4] != 2 {
        return Err(KernelHealthError::ElfParse("invalid 64-bit ELF".into()));
    }

    let e_shoff = read_u64_le(elf_data, 40)? as usize;
    let e_shentsize = read_u16_le(elf_data, 58)? as usize;
    let e_shnum = read_u16_le(elf_data, 60)? as usize;
    let e_shstrndx = read_u16_le(elf_data, 62)? as usize;

    let shstrtab_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstrtab_off = read_u64_le(elf_data, shstrtab_hdr + 24)? as usize;

    let mut this_module_idx: Option<usize> = None;
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_name_off = read_u32_le(elf_data, sh)? as usize;
        let name = read_cstr(elf_data, shstrtab_off + sh_name_off);
        if name == ".gnu.linkonce.this_module" {
            this_module_idx = Some(i);
            break;
        }
    }

    let target_idx = this_module_idx.ok_or_else(|| {
        KernelHealthError::ElfParse(".gnu.linkonce.this_module section not found".into())
    })?;

    let mut init_offset: Option<u64> = None;
    let mut exit_offset: Option<u64> = None;

    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_type = read_u32_le(elf_data, sh + 4)?;
        if sh_type != 4 {
            continue;
        }
        let sh_info = read_u32_le(elf_data, sh + 44)? as usize;
        if sh_info != target_idx {
            continue;
        }

        let rela_off = read_u64_le(elf_data, sh + 24)? as usize;
        let rela_size = read_u64_le(elf_data, sh + 32)? as usize;
        let rela_entsize = read_u64_le(elf_data, sh + 56)? as usize;

        if rela_entsize == 0 {
            continue;
        }

        let sh_link = read_u32_le(elf_data, sh + 40)? as usize;
        let (symtab_off, symtab_entsize, strtab_off) =
            resolve_symtab(elf_data, e_shoff, e_shentsize, sh_link)?;

        let num_rela = rela_size / rela_entsize;
        for j in 0..num_rela {
            let entry = rela_off + j * rela_entsize;
            if entry + 24 > elf_data.len() {
                break;
            }
            let r_offset = read_u64_le(elf_data, entry)?;
            let r_info = read_u64_le(elf_data, entry + 8)?;
            let sym_idx = (r_info >> 32) as usize;

            let sym_entry = symtab_off + sym_idx * symtab_entsize;
            if sym_entry + 24 > elf_data.len() {
                continue;
            }
            let st_name = read_u32_le(elf_data, sym_entry)? as usize;
            let sym_name = read_cstr(elf_data, strtab_off + st_name);
            let is_exit_sym = sym_name.contains("cleanup_module") || sym_name.ends_with("_exit");
            let should_capture_exit = exit_offset.is_none() || sym_name.contains("cleanup_module");

            if sym_name.contains("init_module") || sym_name.ends_with("_init") {
                if init_offset.is_none() || sym_name.contains("init_module") {
                    init_offset = Some(r_offset);
                }
            } else if is_exit_sym && should_capture_exit {
                exit_offset = Some(r_offset);
            }
        }
    }

    match (init_offset, exit_offset) {
        (Some(i), Some(e)) => Ok((i, e)),
        _ => Err(KernelHealthError::ElfParse(
            "could not find init/exit relocations in .gnu.linkonce.this_module".into(),
        )),
    }
}

pub(crate) fn find_reference_ko() -> Option<PathBuf> {
    for name in &["nvidia", "nouveau", "snd_hda_intel", "i915"] {
        if let Some(p) = crate::vfio::kmod::modinfo_path(name) {
            return Some(p);
        }
    }

    let krel = kernel_release().ok()?;
    let search_root = format!("/lib/modules/{krel}/kernel");
    let search_path = Path::new(&search_root);
    if !search_path.is_dir() {
        return None;
    }

    if let Some(ko) = walk_first_matching(search_path, "ko") {
        return Some(ko);
    }
    if let Some(zst) = walk_first_matching(search_path, "ko.zst") {
        return decompress_zst_ko(&zst.display().to_string());
    }

    None
}

fn walk_first_matching(root: &Path, ext: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let path = entry.path();
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(&format!(".{ext}")) {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn decompress_zst_ko(zst_path: &str) -> Option<PathBuf> {
    let compressed = std::fs::read(zst_path).ok()?;
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(compressed.as_slice()).ok()?;
    let mut decompressed = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut decompressed).ok()?;
    let dest = std::env::temp_dir().join("toadstool_ref_module.ko");
    std::fs::write(&dest, &decompressed).ok()?;
    Some(dest)
}
