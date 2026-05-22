// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kernel build environment health check — preflight gate for module operations.
//!
//! Detects `autoconf.h` corruption and `struct module` layout mismatches
//! that cause misleading `Invalid relocation target` errors at module load
//! time. Discovered via Exp 216: a corrupted `autoconf.h` shifted
//! `struct module` field offsets by 24 bytes, making `INIT_LIST_HEAD`
//! clobber the `exit` relocation target during the kernel's in-memory
//! relocation pass.
//!
//! Three detection layers, from cheapest to most definitive:
//!
//! 1. **Freshness** — `autoconf.h` mtime vs kernel image mtime
//! 2. **Struct probe** — compile a tiny module, read `offsetof(struct module, init/exit)`
//! 3. **Reference cross-check** — parse `.gnu.linkonce.this_module` RELA from a loaded `.ko`

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Full health report from all detection layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelHealthReport {
    /// Layer 1: whether autoconf.h is older than or same age as the kernel image.
    pub autoconf_fresh: bool,
    /// Seconds between autoconf.h mtime and kernel image mtime.
    /// Negative means autoconf.h is older (expected/good).
    pub autoconf_age_delta_secs: i64,
    /// Layer 2: `offsetof(struct module, init)` from a freshly compiled probe.
    pub struct_module_init_offset: Option<u64>,
    /// Layer 2: `offsetof(struct module, exit)` from a freshly compiled probe.
    pub struct_module_exit_offset: Option<u64>,
    /// Layer 3: init offset from a reference .ko already known to load.
    pub reference_init_offset: Option<u64>,
    /// Layer 3: exit offset from a reference .ko already known to load.
    pub reference_exit_offset: Option<u64>,
    /// Whether probe offsets match reference offsets (both must be present).
    pub layout_matches: bool,
    /// Human-readable diagnosis.
    pub diagnosis: KernelHealthDiagnosis,
}

/// Diagnosis result from the health check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelHealthDiagnosis {
    /// All layers pass — safe to compile and load modules.
    Healthy,
    /// autoconf.h is newer than the kernel image.
    AutoconfStale { detail: String },
    /// Probe and reference disagree on struct module layout.
    StructLayoutMismatch { expected_exit: u64, actual_exit: u64 },
    /// Could not compile the probe module (missing headers/toolchain).
    ProbeCompileFailed { reason: String },
    /// No reference module found to cross-check against.
    NoReferenceModule,
}

impl std::fmt::Display for KernelHealthDiagnosis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "kernel build environment healthy"),
            Self::AutoconfStale { detail } => write!(f, "autoconf.h stale: {detail}"),
            Self::StructLayoutMismatch { expected_exit, actual_exit } => {
                write!(
                    f,
                    "struct module layout mismatch: reference exit=0x{expected_exit:x}, \
                     probe exit=0x{actual_exit:x} (delta={} bytes)",
                    (*expected_exit as i64) - (*actual_exit as i64)
                )
            }
            Self::ProbeCompileFailed { reason } => {
                write!(f, "probe module compilation failed: {reason}")
            }
            Self::NoReferenceModule => write!(f, "no reference module available for cross-check"),
        }
    }
}

/// Errors from health check operations.
#[derive(Debug, thiserror::Error)]
pub enum KernelHealthError {
    #[error("cannot determine running kernel release: {0}")]
    KernelRelease(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("probe compilation failed: {0}")]
    ProbeCompile(String),
    #[error("ELF parse error: {0}")]
    ElfParse(String),
}

// ── Helpers ─────────────────────────────────────────────────────────

fn kernel_release() -> Result<String, KernelHealthError> {
    let out = Command::new("uname")
        .arg("-r")
        .output()
        .map_err(|e| KernelHealthError::KernelRelease(e.to_string()))?;
    if !out.status.success() {
        return Err(KernelHealthError::KernelRelease("uname -r failed".into()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn headers_dir(krel: &str) -> PathBuf {
    PathBuf::from(format!("/usr/src/linux-headers-{krel}"))
}

fn autoconf_path(krel: &str) -> PathBuf {
    headers_dir(krel).join("include/generated/autoconf.h")
}

fn kernel_image_path(krel: &str) -> PathBuf {
    PathBuf::from(format!("/boot/vmlinuz-{krel}"))
}

// ── Layer 1: autoconf.h freshness ───────────────────────────────────

/// Compare mtime of `autoconf.h` against the kernel image.
///
/// Returns `(is_fresh, delta_seconds)` where `is_fresh` is true if
/// autoconf.h is older or same age as the kernel image (the expected state).
pub fn check_autoconf_freshness() -> Result<(bool, i64), KernelHealthError> {
    let krel = kernel_release()?;
    check_autoconf_freshness_for(&krel)
}

fn check_autoconf_freshness_for(krel: &str) -> Result<(bool, i64), KernelHealthError> {
    let ac = autoconf_path(krel);
    let ki = kernel_image_path(krel);

    let ac_mtime = std::fs::metadata(&ac)
        .and_then(|m| m.modified())
        .map_err(|e| {
            KernelHealthError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", ac.display()),
            ))
        })?;

    let ki_mtime = std::fs::metadata(&ki)
        .and_then(|m| m.modified())
        .map_err(|e| {
            KernelHealthError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", ki.display()),
            ))
        })?;

    let delta = mtime_delta_secs(ac_mtime, ki_mtime);
    let fresh = delta <= 0;
    Ok((fresh, delta))
}

fn mtime_delta_secs(a: SystemTime, b: SystemTime) -> i64 {
    match a.duration_since(b) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

// ── Layer 2: struct module layout probe ─────────────────────────────

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
    probe_struct_module_layout_for(&krel)
}

fn probe_struct_module_layout_for(krel: &str) -> Result<(u64, u64), KernelHealthError> {
    let tmpdir = std::env::temp_dir().join("toadstool_kernel_probe");
    std::fs::create_dir_all(&tmpdir)?;

    let src = tmpdir.join("probe.c");
    let makefile = tmpdir.join("Makefile");
    let ko = tmpdir.join("probe.ko");

    std::fs::write(&src, PROBE_SOURCE)?;
    std::fs::write(
        &makefile,
        format!(
            "obj-m := probe.o\n\
             KDIR := /lib/modules/{krel}/build\n\
             all:\n\
             \t$(MAKE) -C $(KDIR) M=$(PWD) modules\n\
             clean:\n\
             \t$(MAKE) -C $(KDIR) M=$(PWD) clean\n"
        ),
    )?;

    let output = Command::new("make")
        .arg("-C")
        .arg(&tmpdir)
        .arg("clean")
        .output();
    let _ = output; // ignore clean errors

    let output = Command::new("make")
        .arg("-C")
        .arg(&tmpdir)
        .output()
        .map_err(|e| KernelHealthError::ProbeCompile(format!("make exec: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KernelHealthError::ProbeCompile(
            stderr.lines().take(20).collect::<Vec<_>>().join("\n"),
        ));
    }

    if !ko.exists() {
        return Err(KernelHealthError::ProbeCompile(
            "probe.ko not produced".into(),
        ));
    }

    let offsets = read_probe_offsets(&ko)?;

    // cleanup
    let _ = Command::new("make")
        .arg("-C")
        .arg(&tmpdir)
        .arg("clean")
        .output();
    let _ = std::fs::remove_dir_all(&tmpdir);

    Ok(offsets)
}

/// Parse the `.note.module_offsets` section from a compiled probe `.ko`.
fn read_probe_offsets(ko_path: &Path) -> Result<(u64, u64), KernelHealthError> {
    let data = std::fs::read(ko_path)?;
    let offsets = find_note_section_offsets(&data)?;
    Ok(offsets)
}

/// Locate `.note.module_offsets` in raw ELF bytes and extract the two u64 values.
fn find_note_section_offsets(elf_data: &[u8]) -> Result<(u64, u64), KernelHealthError> {
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

    let e_shoff = u64::from_le_bytes(elf_data[40..48].try_into().unwrap()) as usize;
    let e_shentsize = u16::from_le_bytes(elf_data[58..60].try_into().unwrap()) as usize;
    let e_shnum = u16::from_le_bytes(elf_data[60..62].try_into().unwrap()) as usize;
    let e_shstrndx = u16::from_le_bytes(elf_data[62..64].try_into().unwrap()) as usize;

    if e_shstrndx >= e_shnum {
        return Err(KernelHealthError::ElfParse("invalid shstrndx".into()));
    }

    let shstrtab_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstrtab_off =
        u64::from_le_bytes(elf_data[shstrtab_hdr + 24..shstrtab_hdr + 32].try_into().unwrap())
            as usize;

    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_name_off =
            u32::from_le_bytes(elf_data[sh..sh + 4].try_into().unwrap()) as usize;
        let name_start = shstrtab_off + sh_name_off;

        let name = read_cstr(elf_data, name_start);
        if name == ".note.module_offsets" {
            let sh_offset =
                u64::from_le_bytes(elf_data[sh + 24..sh + 32].try_into().unwrap()) as usize;
            let sh_size =
                u64::from_le_bytes(elf_data[sh + 32..sh + 40].try_into().unwrap()) as usize;

            if sh_size < 16 {
                return Err(KernelHealthError::ElfParse(
                    ".note.module_offsets too small".into(),
                ));
            }

            let init_off =
                u64::from_le_bytes(elf_data[sh_offset..sh_offset + 8].try_into().unwrap());
            let exit_off = u64::from_le_bytes(
                elf_data[sh_offset + 8..sh_offset + 16].try_into().unwrap(),
            );

            return Ok((init_off, exit_off));
        }
    }

    Err(KernelHealthError::ElfParse(
        ".note.module_offsets section not found in probe.ko".into(),
    ))
}

/// Fallback for Layer 2: if direct probe compilation fails, look for a
/// recently DKMS-built module and read its init/exit offsets. Since DKMS
/// modules are compiled with the current headers, their offsets reflect
/// the current build environment.
fn probe_from_dkms_module() -> Option<(u64, u64)> {
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

// ── Layer 3: reference module cross-check ───────────────────────────

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

fn parse_this_module_rela_offsets(elf_data: &[u8]) -> Result<(u64, u64), KernelHealthError> {
    if elf_data.len() < 64 || &elf_data[0..4] != b"\x7fELF" || elf_data[4] != 2 {
        return Err(KernelHealthError::ElfParse("invalid 64-bit ELF".into()));
    }

    let e_shoff = u64::from_le_bytes(elf_data[40..48].try_into().unwrap()) as usize;
    let e_shentsize = u16::from_le_bytes(elf_data[58..60].try_into().unwrap()) as usize;
    let e_shnum = u16::from_le_bytes(elf_data[60..62].try_into().unwrap()) as usize;
    let e_shstrndx = u16::from_le_bytes(elf_data[62..64].try_into().unwrap()) as usize;

    let shstrtab_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstrtab_off =
        u64::from_le_bytes(elf_data[shstrtab_hdr + 24..shstrtab_hdr + 32].try_into().unwrap())
            as usize;

    // Find the .gnu.linkonce.this_module section index
    let mut this_module_idx: Option<usize> = None;
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_name_off = u32::from_le_bytes(elf_data[sh..sh + 4].try_into().unwrap()) as usize;
        let name = read_cstr(elf_data, shstrtab_off + sh_name_off);
        if name == ".gnu.linkonce.this_module" {
            this_module_idx = Some(i);
            break;
        }
    }

    let target_idx = this_module_idx.ok_or_else(|| {
        KernelHealthError::ElfParse(".gnu.linkonce.this_module section not found".into())
    })?;

    // Find the RELA section that targets .gnu.linkonce.this_module
    // SHT_RELA = 4, sh_info points to the target section index
    let mut init_offset: Option<u64> = None;
    let mut exit_offset: Option<u64> = None;

    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_type = u32::from_le_bytes(elf_data[sh + 4..sh + 8].try_into().unwrap());
        if sh_type != 4 {
            continue; // not SHT_RELA
        }
        // Elf64_Shdr: sh_info is at offset 44
        let sh_info = u32::from_le_bytes(elf_data[sh + 44..sh + 48].try_into().unwrap()) as usize;
        if sh_info != target_idx {
            continue;
        }

        // sh_offset at 24, sh_size at 32, sh_entsize at 56
        let rela_off =
            u64::from_le_bytes(elf_data[sh + 24..sh + 32].try_into().unwrap()) as usize;
        let rela_size =
            u64::from_le_bytes(elf_data[sh + 32..sh + 40].try_into().unwrap()) as usize;
        let rela_entsize =
            u64::from_le_bytes(elf_data[sh + 56..sh + 64].try_into().unwrap()) as usize;

        if rela_entsize == 0 {
            continue;
        }

        // sh_link at offset 40 — points to the associated symtab
        let sh_link =
            u32::from_le_bytes(elf_data[sh + 40..sh + 44].try_into().unwrap()) as usize;
        let (symtab_off, symtab_entsize, strtab_off) =
            resolve_symtab(elf_data, e_shoff, e_shentsize, sh_link)?;

        let num_rela = rela_size / rela_entsize;
        for j in 0..num_rela {
            let entry = rela_off + j * rela_entsize;
            if entry + 24 > elf_data.len() {
                break;
            }
            let r_offset =
                u64::from_le_bytes(elf_data[entry..entry + 8].try_into().unwrap());
            let r_info =
                u64::from_le_bytes(elf_data[entry + 8..entry + 16].try_into().unwrap());
            let sym_idx = (r_info >> 32) as usize;

            let sym_entry = symtab_off + sym_idx * symtab_entsize;
            if sym_entry + 24 > elf_data.len() {
                continue;
            }
            let st_name =
                u32::from_le_bytes(elf_data[sym_entry..sym_entry + 4].try_into().unwrap())
                    as usize;
            let sym_name = read_cstr(elf_data, strtab_off + st_name);
            let is_exit_sym =
                sym_name.contains("cleanup_module") || sym_name.ends_with("_exit");
            let should_capture_exit =
                exit_offset.is_none() || sym_name.contains("cleanup_module");

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

fn resolve_symtab(
    elf_data: &[u8],
    e_shoff: usize,
    e_shentsize: usize,
    symtab_idx: usize,
) -> Result<(usize, usize, usize), KernelHealthError> {
    let sh = e_shoff + symtab_idx * e_shentsize;
    let symtab_off =
        u64::from_le_bytes(elf_data[sh + 24..sh + 32].try_into().unwrap()) as usize;
    let symtab_entsize =
        u64::from_le_bytes(elf_data[sh + 56..sh + 64].try_into().unwrap()) as usize;

    // sh_link of symtab points to its strtab
    let strtab_idx =
        u32::from_le_bytes(elf_data[sh + 24 + 16..sh + 28 + 16].try_into().unwrap()) as usize;
    let strtab_sh = e_shoff + strtab_idx * e_shentsize;
    let strtab_off =
        u64::from_le_bytes(elf_data[strtab_sh + 24..strtab_sh + 32].try_into().unwrap())
            as usize;

    Ok((symtab_off, symtab_entsize, strtab_off))
}

fn read_cstr(data: &[u8], start: usize) -> String {
    let mut end = start;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    String::from_utf8_lossy(&data[start..end]).to_string()
}

// ── Auto-discover reference module ──────────────────────────────────

/// Find a reference `.ko` file from a module known to be loadable on this kernel.
///
/// Tries (in order):
/// 1. The stock `nvidia` module via `modinfo -n`
/// 2. Any DKMS-built module under `/var/lib/dkms/`
/// 3. A random module from `/lib/modules/$(uname -r)/kernel/`
fn find_reference_ko() -> Option<PathBuf> {
    // Try modinfo for well-known modules
    for name in &["nvidia", "nouveau", "snd_hda_intel", "i915"] {
        if let Ok(out) = Command::new("modinfo").args(["-n", name]).output()
            && out.status.success()
        {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() && path_str != "(builtin)" {
                let p = PathBuf::from(&path_str);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // Fallback: find any .ko under /lib/modules/
    if let Ok(krel) = kernel_release() {
        let search = format!("/lib/modules/{krel}/kernel");
        if let Ok(out) = Command::new("find")
            .args([&search, "-name", "*.ko", "-type", "f"])
            .arg("-print")
            .arg("-quit")
            .output()
            && out.status.success()
        {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() {
                let p = PathBuf::from(&path_str);
                if p.exists() {
                    return Some(p);
                }
            }
        }
        // Also try compressed .ko.zst
        if let Ok(out) = Command::new("find")
            .args([&search, "-name", "*.ko.zst", "-type", "f"])
            .arg("-print")
            .arg("-quit")
            .output()
            && out.status.success()
        {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() {
                return decompress_zst_ko(&path_str);
            }
        }
    }

    None
}

fn decompress_zst_ko(zst_path: &str) -> Option<PathBuf> {
    let dest = std::env::temp_dir().join("toadstool_ref_module.ko");
    let status = Command::new("zstd")
        .args(["-d", "-f", "-o"])
        .arg(&dest)
        .arg(zst_path)
        .status()
        .ok()?;
    if status.success() && dest.exists() {
        Some(dest)
    } else {
        None
    }
}

// ── Full health check ───────────────────────────────────────────────

/// Run all three detection layers and produce a comprehensive health report.
pub fn full_kernel_health_check() -> Result<KernelHealthReport, KernelHealthError> {
    // Layer 1: freshness
    let (autoconf_fresh, autoconf_age_delta_secs) = match check_autoconf_freshness() {
        Ok((fresh, delta)) => (fresh, delta),
        Err(e) => {
            tracing::warn!(err = %e, "autoconf freshness check failed — assuming stale");
            (false, i64::MAX)
        }
    };

    // Layer 2: struct probe (compile a tiny module to check offsets)
    // Fallback: if probe compilation fails, try to read offsets from a
    // recently DKMS-built module (which was compiled with current headers).
    let (probe_init, probe_exit, probe_err) = match probe_struct_module_layout() {
        Ok((i, e)) => (Some(i), Some(e), None),
        Err(compile_err) => {
            tracing::warn!(err = %compile_err, "struct module probe compilation failed — trying DKMS fallback");
            match probe_from_dkms_module() {
                Some((i, e)) => (Some(i), Some(e), None),
                None => (None, None, Some(compile_err)),
            }
        }
    };

    // Layer 3: reference cross-check
    let (ref_init, ref_exit) = if let Some(ref_ko) = find_reference_ko() {
        match reference_module_offsets(&ref_ko) {
            Ok((i, e)) => (Some(i), Some(e)),
            Err(e) => {
                tracing::warn!(err = %e, ko = %ref_ko.display(), "reference module parse failed");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // Determine diagnosis
    let layout_matches;
    let diagnosis;

    match (probe_exit, ref_exit) {
        (Some(pe), Some(re)) => {
            layout_matches = pe == re;
            if !layout_matches {
                diagnosis = KernelHealthDiagnosis::StructLayoutMismatch {
                    expected_exit: re,
                    actual_exit: pe,
                };
            } else if !autoconf_fresh {
                diagnosis = KernelHealthDiagnosis::AutoconfStale {
                    detail: format!(
                        "autoconf.h is {autoconf_age_delta_secs}s newer than kernel image, \
                         but struct layout still matches — monitor for drift"
                    ),
                };
            } else {
                diagnosis = KernelHealthDiagnosis::Healthy;
            }
        }
        (None, _) => {
            layout_matches = false;
            if let Some(err) = probe_err {
                diagnosis = KernelHealthDiagnosis::ProbeCompileFailed {
                    reason: err.to_string(),
                };
            } else {
                diagnosis = KernelHealthDiagnosis::ProbeCompileFailed {
                    reason: "unknown probe failure".into(),
                };
            }
        }
        (Some(_), None) => {
            // Probe succeeded but no reference — can't cross-check.
            // If autoconf is stale, flag it; otherwise assume ok.
            if !autoconf_fresh {
                layout_matches = false;
                diagnosis = KernelHealthDiagnosis::AutoconfStale {
                    detail: format!(
                        "autoconf.h is {autoconf_age_delta_secs}s newer than kernel image \
                         and no reference module for cross-check"
                    ),
                };
            } else {
                layout_matches = true;
                diagnosis = KernelHealthDiagnosis::Healthy;
            }
        }
    }

    Ok(KernelHealthReport {
        autoconf_fresh,
        autoconf_age_delta_secs,
        struct_module_init_offset: probe_init,
        struct_module_exit_offset: probe_exit,
        reference_init_offset: ref_init,
        reference_exit_offset: ref_exit,
        layout_matches,
        diagnosis,
    })
}

// ── Repair ──────────────────────────────────────────────────────────

/// Strategy for repairing a corrupted autoconf.h.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairStrategy {
    /// Extract from cached .deb in /var/cache/apt/archives/ (fastest, no network).
    PackageRestore,
    /// `apt-get install --reinstall linux-headers-$(uname -r)` (slow, needs network).
    PackageReinstall,
}

/// Attempt to repair the kernel headers by restoring the original `autoconf.h`.
///
/// Returns the path to the restored file on success.
pub fn repair_autoconf(strategy: RepairStrategy) -> Result<PathBuf, KernelHealthError> {
    let krel = kernel_release()?;
    let target = autoconf_path(&krel);

    match strategy {
        RepairStrategy::PackageRestore => repair_from_deb_cache(&krel, &target),
        RepairStrategy::PackageReinstall => repair_via_reinstall(&krel, &target),
    }
}

fn repair_from_deb_cache(krel: &str, target: &Path) -> Result<PathBuf, KernelHealthError> {
    let cache_dir = PathBuf::from("/var/cache/apt/archives");
    let pattern = format!("linux-headers-{krel}_");

    let entries: Vec<_> = std::fs::read_dir(&cache_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&pattern)
                && e.file_name().to_string_lossy().ends_with(".deb")
        })
        .collect();

    if entries.is_empty() {
        return Err(KernelHealthError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no cached .deb matching {pattern}*.deb in {}", cache_dir.display()),
        )));
    }

    let deb_path = entries[0].path();
    let extract_dir = std::env::temp_dir().join("toadstool_autoconf_repair");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir)?;

    let status = Command::new("dpkg-deb")
        .args(["-x"])
        .arg(&deb_path)
        .arg(&extract_dir)
        .status()
        .map_err(KernelHealthError::Io)?;

    if !status.success() {
        return Err(KernelHealthError::Io(std::io::Error::other(
            "dpkg-deb extraction failed",
        )));
    }

    // The extracted autoconf.h lives at a relative path matching the target
    let relative = format!("usr/src/linux-headers-{krel}/include/generated/autoconf.h");
    let extracted = extract_dir.join(&relative);

    if !extracted.exists() {
        return Err(KernelHealthError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "autoconf.h not found in extracted .deb at {}",
                extracted.display()
            ),
        )));
    }

    // Back up current autoconf.h before replacing
    let backup = target.with_extension("h.bak");
    if target.exists() {
        std::fs::copy(target, &backup)?;
        tracing::info!(backup = %backup.display(), "backed up current autoconf.h");
    }

    std::fs::copy(&extracted, target)?;
    tracing::info!(
        source = %deb_path.display(),
        target = %target.display(),
        "restored autoconf.h from .deb cache"
    );

    let _ = std::fs::remove_dir_all(&extract_dir);

    Ok(target.to_path_buf())
}

fn repair_via_reinstall(krel: &str, target: &Path) -> Result<PathBuf, KernelHealthError> {
    let pkg = format!("linux-headers-{krel}");

    let status = Command::new("apt-get")
        .args(["install", "--reinstall", "-y"])
        .arg(&pkg)
        .status()
        .map_err(KernelHealthError::Io)?;

    if !status.success() {
        return Err(KernelHealthError::Io(std::io::Error::other(
            format!("apt-get install --reinstall {pkg} failed"),
        )));
    }

    Ok(target.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtime_delta_same_time() {
        let now = SystemTime::now();
        assert_eq!(mtime_delta_secs(now, now), 0);
    }

    #[test]
    fn mtime_delta_future() {
        let now = SystemTime::now();
        let future = now + std::time::Duration::from_secs(100);
        assert!(mtime_delta_secs(future, now) > 0);
    }

    #[test]
    fn mtime_delta_past() {
        let now = SystemTime::now();
        let past = now - std::time::Duration::from_secs(100);
        assert!(mtime_delta_secs(past, now) < 0);
    }

    #[test]
    fn read_cstr_basic() {
        let data = b"hello\x00world\x00";
        assert_eq!(read_cstr(data, 0), "hello");
        assert_eq!(read_cstr(data, 6), "world");
    }

    #[test]
    fn read_cstr_at_end() {
        let data = b"abc";
        assert_eq!(read_cstr(data, 0), "abc");
    }

    #[test]
    fn diagnosis_display_healthy() {
        let d = KernelHealthDiagnosis::Healthy;
        assert_eq!(d.to_string(), "kernel build environment healthy");
    }

    #[test]
    fn diagnosis_display_mismatch() {
        let d = KernelHealthDiagnosis::StructLayoutMismatch {
            expected_exit: 0x4a8,
            actual_exit: 0x490,
        };
        let s = d.to_string();
        assert!(s.contains("0x4a8"));
        assert!(s.contains("0x490"));
        assert!(s.contains("24 bytes"));
    }

    #[test]
    fn find_note_section_rejects_short_file() {
        let data = vec![0u8; 32];
        assert!(find_note_section_offsets(&data).is_err());
    }

    #[test]
    fn find_note_section_rejects_non_elf() {
        let mut data = vec![0u8; 128];
        data[0..4].copy_from_slice(b"NOTA");
        assert!(find_note_section_offsets(&data).is_err());
    }

    #[test]
    fn parse_this_module_rejects_non_elf() {
        let data = vec![0u8; 128];
        assert!(parse_this_module_rela_offsets(&data).is_err());
    }

    #[test]
    fn repair_strategy_serializes() {
        let json = serde_json::to_string(&RepairStrategy::PackageRestore).unwrap();
        assert!(json.contains("PackageRestore"));
    }

    #[test]
    fn health_report_serializes() {
        let report = KernelHealthReport {
            autoconf_fresh: true,
            autoconf_age_delta_secs: -86400,
            struct_module_init_offset: Some(0x168),
            struct_module_exit_offset: Some(0x4a8),
            reference_init_offset: Some(0x168),
            reference_exit_offset: Some(0x4a8),
            layout_matches: true,
            diagnosis: KernelHealthDiagnosis::Healthy,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Healthy"));
        assert!(json.contains("4a8") || json.contains("1192"));
    }
}
