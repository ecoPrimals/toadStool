// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use super::elf::{read_cstr, read_u32_le, read_u64_le, read_u16_le};
use super::paths::kernel_release;
use super::reference::reference_module_offsets;
use super::KernelHealthError;

/// Minimal C source that stores `offsetof(struct module, init)` and
/// `offsetof(struct module, exit)` in a `.note.module_offsets` ELF section.
const PROBE_SOURCE: &str = r#"
#include <linux/module.h>
#include <linux/init.h>
#include <stddef.h>

struct probe_offsets {
    unsigned long init_off;
    unsigned long exit_off;
} __attribute__((used, section(".note.module_offsets"))) offsets = {
    .init_off = offsetof(struct module, init),
    .exit_off = offsetof(struct module, exit),
};

MODULE_LICENSE("GPL");
static int __init probe_init(void) { return -ENODEV; }
static void __exit probe_exit(void) {}
module_init(probe_init);
module_exit(probe_exit);
"#;

/// Compile a minimal kernel module and read the `init`/`exit` offsets
/// from its `.note.module_offsets` section.
///
/// Returns `(init_offset, exit_offset)`.
pub fn probe_struct_module_layout() -> Result<(u64, u64), KernelHealthError> {
    let krel = kernel_release()?;
    probe_struct_module_layout_for(krel)
}

fn probe_struct_module_layout_for(_krel: &str) -> Result<(u64, u64), KernelHealthError> {
    use crate::vfio::guarded_sysfs::KmodBuilder;

    let tmpdir = std::env::temp_dir().join("toadstool_kernel_probe");
    let tmpdir_str = tmpdir.display().to_string();

    let ko = KmodBuilder::new("probe")
        .source(PROBE_SOURCE)
        .tmpdir(&tmpdir_str)
        .compile_only()
        .map_err(|e| KernelHealthError::ProbeCompile(e.to_string()))?;

    let offsets = read_probe_offsets(&ko)?;

    KmodBuilder::clean(&tmpdir_str);

    Ok(offsets)
}

fn read_probe_offsets(ko_path: &Path) -> Result<(u64, u64), KernelHealthError> {
    let data = std::fs::read(ko_path)?;
    let offsets = find_note_section_offsets(&data)?;
    Ok(offsets)
}

pub(crate) fn find_note_section_offsets(elf_data: &[u8]) -> Result<(u64, u64), KernelHealthError> {
    if elf_data.len() < 64 {
        return Err(KernelHealthError::ElfParse("file too small for ELF".into()));
    }
    if &elf_data[0..4] != b"\x7fELF" {
        return Err(KernelHealthError::ElfParse("not an ELF file".into()));
    }

    let is_64 = elf_data[4] == 2;
    if !is_64 {
        return Err(KernelHealthError::ElfParse("only 64-bit ELF supported".into()));
    }
    let is_le = elf_data[5] == 1;
    if !is_le {
        return Err(KernelHealthError::ElfParse(
            "only little-endian ELF supported".into(),
        ));
    }

    let e_shoff = read_u64_le(elf_data, 40)? as usize;
    let e_shentsize = read_u16_le(elf_data, 58)? as usize;
    let e_shnum = read_u16_le(elf_data, 60)? as usize;
    let e_shstrndx = read_u16_le(elf_data, 62)? as usize;

    if e_shstrndx >= e_shnum {
        return Err(KernelHealthError::ElfParse("invalid shstrndx".into()));
    }

    let shstrtab_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstrtab_off = read_u64_le(elf_data, shstrtab_hdr + 24)? as usize;

    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_name_off = read_u32_le(elf_data, sh)? as usize;
        let name_start = shstrtab_off + sh_name_off;

        let name = read_cstr(elf_data, name_start);
        if name == ".note.module_offsets" {
            let sh_offset = read_u64_le(elf_data, sh + 24)? as usize;
            let sh_size = read_u64_le(elf_data, sh + 32)? as usize;

            if sh_size < 16 {
                return Err(KernelHealthError::ElfParse(
                    ".note.module_offsets too small".into(),
                ));
            }

            let init_off = read_u64_le(elf_data, sh_offset)?;
            let exit_off = read_u64_le(elf_data, sh_offset + 8)?;

            return Ok((init_off, exit_off));
        }
    }

    Err(KernelHealthError::ElfParse(
        ".note.module_offsets section not found in probe.ko".into(),
    ))
}

pub(crate) fn probe_from_dkms_module() -> Option<(u64, u64)> {
    let krel = kernel_release().ok()?;
    let dkms_base = "/var/lib/dkms".to_string();

    let entries = std::fs::read_dir(&dkms_base).ok()?;
    for entry in entries.flatten() {
        let mod_name = entry.file_name().to_string_lossy().to_string();
        let version_dir = entry.path();
        let versions = std::fs::read_dir(&version_dir).ok()?;
        for ver_entry in versions.flatten() {
            let ko_path = ver_entry
                .path()
                .join(&krel)
                .join("x86_64/module")
                .join(format!("{mod_name}.ko"));
            if ko_path.exists()
                && let Ok((i, e)) = reference_module_offsets(&ko_path)
            {
                tracing::info!(
                    ko = %ko_path.display(),
                    init = format_args!("0x{i:x}"),
                    exit = format_args!("0x{e:x}"),
                    "Layer 2 fallback: using DKMS module as probe"
                );
                return Some((i, e));
            }
        }
    }
    None
}
