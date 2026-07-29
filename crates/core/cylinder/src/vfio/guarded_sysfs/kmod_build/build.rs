// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::{GuardedSysfsError, INSMOD_TIMEOUT, RMMOD_TIMEOUT};
use super::load::{insmod_guarded_with_params, rmmod_guarded};

/// Builder for out-of-tree Linux kernel modules via kbuild.
///
/// Encapsulates the full lifecycle: stage source → compile via
/// `make -C /lib/modules/{krel}/build M=$PWD` → `insmod` with
/// parameters → `rmmod` → cleanup. Each step is typed and
/// compiler-verified; the only non-Rust artifacts are the C source
/// literal and the generated Makefile (irreducible kernel ABI boundary).
///
/// ```text
/// KmodBuilder::new("no_bus_reset")
///     .source(C_SOURCE)
///     .tmpdir("/var/tmp/toadstool-no-bus-reset")
///     .param("bdf", "0000:02:00.0")
///     .build_and_load()?;
/// ```
pub struct KmodBuilder {
    name: String,
    source: &'static str,
    tmpdir: String,
    params: Vec<(String, String)>,
}

impl KmodBuilder {
    /// Create a builder for a kernel module with the given name.
    ///
    /// The name determines the `.c` filename, `.ko` output, and
    /// `obj-m` target in the generated Makefile.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            source: "",
            tmpdir: std::env::temp_dir()
                .join(format!("toadstool-kmod-{name}"))
                .display()
                .to_string(),
            params: Vec::new(),
        }
    }

    /// Set the C source code for the module.
    pub fn source(mut self, src: &'static str) -> Self {
        self.source = src;
        self
    }

    /// Override the build directory (default: `$TMPDIR/toadstool-kmod-{name}`).
    pub fn tmpdir(mut self, dir: &str) -> Self {
        self.tmpdir = dir.to_string();
        self
    }

    /// Add a module parameter (passed to `insmod` as `key=value`).
    pub fn param(mut self, key: &str, value: &str) -> Self {
        self.params.push((key.to_string(), value.to_string()));
        self
    }

    /// Check if the module is already loaded with matching parameters.
    ///
    /// Reads `/sys/module/{name}/parameters/{key}` for each configured
    /// parameter and compares against the expected value. Returns true
    /// only if the module is loaded AND all parameters match.
    fn is_loaded_with_matching_params(&self) -> bool {
        let sys_path = crate::linux_paths::sysfs_module_path(&self.name);
        if !Path::new(&sys_path).exists() {
            return false;
        }
        for (key, value) in &self.params {
            let param_path = crate::linux_paths::sysfs_module_parameter(&self.name, key);
            match std::fs::read_to_string(&param_path) {
                Ok(contents) if contents.trim() == value => {}
                _ => return false,
            }
        }
        true
    }

    /// If the module is already loaded, unload it first (idempotent reload).
    fn ensure_unloaded(&self) -> Result<(), GuardedSysfsError> {
        let sys_path = crate::linux_paths::sysfs_module_path(&self.name);
        if Path::new(&sys_path).exists() {
            tracing::info!(
                module = self.name.as_str(),
                "kmod already loaded — unloading for reload"
            );
            let _ = rmmod_guarded(&self.name, RMMOD_TIMEOUT);
        }
        Ok(())
    }

    /// Stage source and Makefile, then compile via kbuild.
    ///
    /// Returns the path to the compiled `.ko` file. Does not load the
    /// module — use `build_and_load` for the full lifecycle, or call
    /// this when you only need the compiled artifact (e.g. ELF inspection
    /// in kernel health probes).
    pub fn compile_only(&self) -> Result<PathBuf, GuardedSysfsError> {
        let krel =
            crate::linux_paths::kernel_release().ok_or_else(|| GuardedSysfsError::KmodFailed {
                cmd: "kernel_release".into(),
                args: String::new(),
                reason: "could not read /proc/sys/kernel/osrelease".into(),
            })?;

        // Check persistent cache first — survives reboots, avoids kbuild
        // entirely when the kernel version hasn't changed.
        let cache_dir = PathBuf::from(format!(
            "{}/kmod-cache/{krel}",
            crate::linux_paths::data_dir()
        ));
        let cached_ko = cache_dir.join(format!("{}.ko", self.name));
        if cached_ko.exists() {
            tracing::info!(module = self.name.as_str(), krel,
                           path = %cached_ko.display(),
                           "kmod builder: using cached .ko");
            return Ok(cached_ko);
        }

        let kbuild =
            crate::linux_paths::kbuild_dir().ok_or_else(|| GuardedSysfsError::KmodFailed {
                cmd: "kbuild_dir".into(),
                args: String::new(),
                reason: "kernel release unavailable for kbuild path".into(),
            })?;

        let tmpdir = Path::new(&self.tmpdir);
        std::fs::create_dir_all(tmpdir)?;

        // Stage source
        let src_path = tmpdir.join(format!("{}.c", self.name));
        std::fs::write(&src_path, self.source)?;

        // Generate Makefile — call kbuild directly, no wrapper
        let makefile_path = tmpdir.join("Makefile");
        std::fs::write(
            &makefile_path,
            format!(
                "obj-m := {name}.o\n\
                 KDIR := {kbuild}\n\
                 all:\n\
                 \t$(MAKE) -C $(KDIR) M=$(PWD) modules\n\
                 clean:\n\
                 \t$(MAKE) -C $(KDIR) M=$(PWD) clean\n",
                name = self.name,
            ),
        )?;

        // Compile
        tracing::info!(
            module = self.name.as_str(),
            krel,
            "kmod builder: compiling via kbuild"
        );
        let compile_out = Command::new("make")
            .arg("-C")
            .arg(tmpdir)
            .output()
            .map_err(|e| GuardedSysfsError::KmodFailed {
                cmd: "make".into(),
                args: format!("-C {}", self.tmpdir),
                reason: format!("failed to spawn: {e}"),
            })?;

        if !compile_out.status.success() {
            let stderr = String::from_utf8_lossy(&compile_out.stderr);
            let snippet: String = stderr.lines().take(20).collect::<Vec<_>>().join("\n");
            return Err(GuardedSysfsError::KmodFailed {
                cmd: format!("kmod make -C {}", self.tmpdir),
                args: String::new(),
                reason: format!("compilation failed:\n{snippet}"),
            });
        }

        let ko_path = tmpdir.join(format!("{}.ko", self.name));
        if !ko_path.exists() {
            return Err(GuardedSysfsError::KmodFailed {
                cmd: "make".into(),
                args: format!("-C {}", self.tmpdir),
                reason: format!("{}.ko not produced", self.name),
            });
        }

        // Cache the compiled .ko for future use
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(error = %e, "failed to create kmod cache dir (non-fatal)");
        } else if let Err(e) = std::fs::copy(&ko_path, &cached_ko) {
            tracing::warn!(error = %e, "failed to cache compiled .ko (non-fatal)");
        } else {
            tracing::info!(module = self.name.as_str(), krel,
                           path = %cached_ko.display(),
                           "kmod builder: cached compiled .ko");
        }

        Ok(ko_path)
    }

    /// Stage source and Makefile, compile via kbuild, and load the module.
    ///
    /// If the module is already loaded with matching parameters, this is a
    /// no-op. This prevents the destructive sequence of rmmod → failed
    /// compile → unprotected device (Exp 226 regression).
    pub fn build_and_load(&self) -> Result<(), GuardedSysfsError> {
        if self.is_loaded_with_matching_params() {
            tracing::info!(
                module = self.name.as_str(),
                "kmod already loaded with correct params — skipping rebuild"
            );
            return Ok(());
        }

        self.ensure_unloaded()?;

        let ko_path = self.compile_only()?;

        let params_str: String = self
            .params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        insmod_guarded_with_params(&ko_path, &params_str, INSMOD_TIMEOUT)?;

        tracing::info!(module = self.name.as_str(),
                       params = ?self.params,
                       "kmod builder: module loaded");
        Ok(())
    }

    /// Remove build artifacts from the tmpdir.
    ///
    /// Deletes the entire build directory. Use after `compile_only` when
    /// the `.ko` has been consumed and is no longer needed.
    pub fn clean(tmpdir: &str) {
        let path = Path::new(tmpdir);
        if path.exists() {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    /// Unload the module and clean up build artifacts.
    pub fn unload_and_clean(name: &str, tmpdir: &str) -> Result<(), GuardedSysfsError> {
        let sys_path = crate::linux_paths::sysfs_module_path(name);
        if !Path::new(&sys_path).exists() {
            return Ok(());
        }
        rmmod_guarded(name, RMMOD_TIMEOUT)?;
        tracing::info!(module = name, "kmod builder: module unloaded");

        KmodBuilder::clean(tmpdir);
        Ok(())
    }
}

const NO_BUS_RESET_MODULE: &str = "no_bus_reset";

fn no_bus_reset_tmpdir() -> String {
    std::env::temp_dir()
        .join("toadstool-no-bus-reset")
        .display()
        .to_string()
}

const NO_BUS_RESET_SOURCE: &str = r#"
#include <linux/module.h>
#include <linux/pci.h>
#include <linux/string.h>

static char *bdf = "";
module_param(bdf, charp, 0444);
MODULE_PARM_DESC(bdf, "Comma-separated PCI BDFs to suppress bus reset for");

static int no_flr = 0;
module_param(no_flr, int, 0444);
MODULE_PARM_DESC(no_flr, "Also suppress FLR and PM reset (preserves warm GPU state)");

#define MAX_TARGETS 8
static struct pci_dev *targets[MAX_TARGETS];
static int ntargets;
static pci_dev_flags_t applied_flags;

static int __init no_bus_reset_init(void) {
    char buf[256], *p, *tok;
    struct pci_dev *dev;

    applied_flags = PCI_DEV_FLAGS_NO_BUS_RESET;
    if (no_flr) {
        applied_flags |= PCI_DEV_FLAGS_NO_FLR_RESET | PCI_DEV_FLAGS_NO_PM_RESET;
        pr_info("no_bus_reset: FLR+PM reset suppression enabled\n");
    }

    strscpy(buf, bdf, sizeof(buf));
    p = buf;
    while ((tok = strsep(&p, ",")) != NULL && ntargets < MAX_TARGETS) {
        while (*tok == ' ') tok++;
        if (*tok == '\0') continue;
        dev = NULL;
        while ((dev = pci_get_device(PCI_ANY_ID, PCI_ANY_ID, dev))) {
            if (strcmp(dev_name(&dev->dev), tok) == 0) {
                dev->dev_flags |= applied_flags;
                targets[ntargets++] = dev;
                pr_info("no_bus_reset: suppressed on %s (flags=0x%x)\n",
                        tok, (unsigned int)applied_flags);
                break;
            }
        }
        if (!dev)
            pr_warn("no_bus_reset: device %s not found\n", tok);
    }
    return ntargets > 0 ? 0 : -ENODEV;
}

static void __exit no_bus_reset_exit(void) {
    int i;
    for (i = 0; i < ntargets; i++) {
        if (targets[i]) {
            targets[i]->dev_flags &= ~applied_flags;
            pr_info("no_bus_reset: restored on %s\n", dev_name(&targets[i]->dev));
            pci_dev_put(targets[i]);
        }
    }
}

module_init(no_bus_reset_init);
module_exit(no_bus_reset_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Suppress PCI bus/FLR/PM reset for specified devices");
"#;

/// Compile and load the `no_bus_reset` kernel module for one or more devices.
///
/// Sets `PCI_DEV_FLAGS_NO_BUS_RESET` on each target device, which makes
/// `pci_bus_resetable()` return false and prevents `pci_reset_bus()` from
/// performing a Secondary Bus Reset (SBR) through the upstream bridge.
///
/// Accepts a single BDF or a comma-separated list. If the module is already
/// loaded with parameters covering the requested BDF(s), this is a no-op.
///
/// Must be called BEFORE dropping VFIO device fds. The module should be
/// unloaded via [`restore_bus_reset`] after the handoff is complete and
/// vfio-pci is re-bound.
pub fn suppress_bus_reset(bdf: &str) -> Result<(), GuardedSysfsError> {
    // If the module is already loaded, check if this BDF is already covered
    let sys_param = crate::linux_paths::sysfs_module_parameter(NO_BUS_RESET_MODULE, "bdf");
    if Path::new(&sys_param).exists()
        && let Ok(loaded_bdfs) = std::fs::read_to_string(&sys_param)
    {
        let loaded = loaded_bdfs.trim();
        let already_covered = bdf
            .split(',')
            .all(|b| loaded.split(',').any(|l| l.trim() == b.trim()));
        if already_covered {
            tracing::info!(bdf, loaded, "no_bus_reset already covers this device");
            return Ok(());
        }
        // Need to reload with the union of old + new BDFs
        let mut all_bdfs: Vec<&str> = loaded
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for b in bdf.split(',').map(|s| s.trim()) {
            if !all_bdfs.contains(&b) {
                all_bdfs.push(b);
            }
        }
        let combined = all_bdfs.join(",");
        tracing::info!(
            bdf,
            combined,
            "reloading no_bus_reset with expanded device list"
        );
        return KmodBuilder::new(NO_BUS_RESET_MODULE)
            .source(NO_BUS_RESET_SOURCE)
            .tmpdir(&no_bus_reset_tmpdir())
            .param("bdf", &combined)
            .build_and_load();
    }

    KmodBuilder::new(NO_BUS_RESET_MODULE)
        .source(NO_BUS_RESET_SOURCE)
        .tmpdir(&no_bus_reset_tmpdir())
        .param("bdf", bdf)
        .build_and_load()
}

/// Like [`suppress_bus_reset`] but also sets `PCI_DEV_FLAGS_NO_FLR_RESET`
/// and `PCI_DEV_FLAGS_NO_PM_RESET`. This prevents VFIO from performing FLR
/// when the device is opened, preserving warm GPU state (GPCs, PRI ring,
/// clock trees) from the seeder driver session.
pub fn suppress_all_resets(bdf: &str) -> Result<(), GuardedSysfsError> {
    let sys_param = crate::linux_paths::sysfs_module_parameter(NO_BUS_RESET_MODULE, "bdf");
    if Path::new(&sys_param).exists()
        && let Ok(loaded_bdfs) = std::fs::read_to_string(&sys_param)
    {
        let loaded = loaded_bdfs.trim();
        let already_covered = bdf
            .split(',')
            .all(|b| loaded.split(',').any(|l| l.trim() == b.trim()));
        let flr_param = crate::linux_paths::sysfs_module_parameter(NO_BUS_RESET_MODULE, "no_flr");
        let flr_active = Path::new(&flr_param).exists()
            && std::fs::read_to_string(&flr_param)
                .map(|v| v.trim() == "1")
                .unwrap_or(false);
        if already_covered && flr_active {
            tracing::info!(
                bdf,
                loaded,
                "no_bus_reset+no_flr already covers this device"
            );
            return Ok(());
        }
        let mut all_bdfs: Vec<&str> = loaded
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for b in bdf.split(',').map(|s| s.trim()) {
            if !all_bdfs.contains(&b) {
                all_bdfs.push(b);
            }
        }
        let combined = all_bdfs.join(",");
        tracing::info!(bdf, combined, "reloading no_bus_reset with no_flr=1");
        return KmodBuilder::new(NO_BUS_RESET_MODULE)
            .source(NO_BUS_RESET_SOURCE)
            .tmpdir(&no_bus_reset_tmpdir())
            .param("bdf", &combined)
            .param("no_flr", "1")
            .build_and_load();
    }

    KmodBuilder::new(NO_BUS_RESET_MODULE)
        .source(NO_BUS_RESET_SOURCE)
        .tmpdir(&no_bus_reset_tmpdir())
        .param("bdf", bdf)
        .param("no_flr", "1")
        .build_and_load()
}

/// Unload the `no_bus_reset` kernel module and clean up build artifacts.
///
/// Clears `PCI_DEV_FLAGS_NO_BUS_RESET` (and `NO_FLR_RESET`/`NO_PM_RESET`
/// if active) on the target device via the module's exit handler.
pub fn restore_bus_reset() -> Result<(), GuardedSysfsError> {
    KmodBuilder::unload_and_clean(NO_BUS_RESET_MODULE, &no_bus_reset_tmpdir())
}

// ── IRQ Clutch ──────────────────────────────────────────────────────────
//
// Kernel module that properly cleans up stale IRQ/MSI state between
// driver unbind and rebind during catalyst handoff.
//
// When nv_close_device is NOP'd, NVIDIA's free_irq + pci_disable_msi
// never run, leaving:
//   1. request_irq() handler still registered on the IRQ descriptor
//   2. MSI domain/vectors still allocated in the kernel's IRQ subsystem
//
// Previous version only called pci_free_irq_vectors() which tears down
// the MSI domain but can't remove the action handler → WARNING:
//   "remove_proc_entry: removing non-empty directory 'irq/176'"
// The leaked handler means rmmod frees the code pages while the IRQ
// descriptor still references the handler → use-after-free on next IRQ.
//
// Fixed version: call free_irq() FIRST to deregister the action handler,
// THEN pci_free_irq_vectors() to tear down the MSI domain cleanly.
// NVIDIA registers its handler with dev_id = pci_get_drvdata(dev).

const IRQ_CLUTCH_MODULE: &str = "irq_clutch";

fn irq_clutch_tmpdir() -> String {
    std::env::temp_dir()
        .join("toadstool-irq-clutch")
        .display()
        .to_string()
}

const IRQ_CLUTCH_SOURCE: &str = r#"
#include <linux/module.h>
#include <linux/pci.h>
#include <linux/interrupt.h>
#include <linux/string.h>

static char *bdf = "";
module_param(bdf, charp, 0444);
MODULE_PARM_DESC(bdf, "PCI BDF of device to clean IRQ vectors for");

static int __init irq_clutch_init(void)
{
    struct pci_dev *dev;
    unsigned int dom, bus, slot, fn;
    void *dev_id;
    int irq;

    if (sscanf(bdf, "%x:%x:%x.%x", &dom, &bus, &slot, &fn) != 4) {
        pr_err("irq_clutch: invalid BDF format: %s\n", bdf);
        return -EINVAL;
    }

    dev = pci_get_domain_bus_and_slot(dom, bus, PCI_DEVFN(slot, fn));
    if (!dev) {
        pr_err("irq_clutch: device %s not found\n", bdf);
        return -ENODEV;
    }

    /*
     * Step 1: Deregister the NVIDIA IRQ handler.
     *
     * nv_close_device was NOP'd so free_irq() never ran. The handler is
     * still registered on dev->irq with dev_id = pci_get_drvdata(dev)
     * (the nv_linux_state_t* that NVIDIA passed to request_irq).
     *
     * We must free_irq BEFORE pci_free_irq_vectors — otherwise the MSI
     * domain teardown hits the registered action and produces:
     *   "remove_proc_entry: removing non-empty directory 'irq/N'"
     */
    irq = dev->irq;
    dev_id = pci_get_drvdata(dev);
    if (dev_id && irq > 0) {
        free_irq(irq, dev_id);
        pr_info("irq_clutch: freed IRQ %d handler (dev_id=%px) for %s\n",
                irq, dev_id, pci_name(dev));
    } else {
        pr_warn("irq_clutch: no handler to free (irq=%d, dev_id=%px) for %s\n",
                irq, dev_id, pci_name(dev));
    }

    /* Step 2: Tear down the MSI domain and free vectors cleanly. */
    pci_free_irq_vectors(dev);
    pci_dev_put(dev);

    pr_info("irq_clutch: cleaned IRQ vectors for %s\n", bdf);
    return 0;
}

static void __exit irq_clutch_exit(void) {}

module_init(irq_clutch_init);
module_exit(irq_clutch_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Clean stale IRQ handlers and MSI state between driver swaps");
"#;

/// Engage the IRQ clutch: load `irq_clutch.ko` which first calls
/// `free_irq()` to deregister the NVIDIA IRQ handler, then calls
/// `pci_free_irq_vectors()` to tear down the MSI domain cleanly.
///
/// Must be called BEFORE unbind (while MSI data structures are valid).
pub fn engage_irq_clutch(bdf: &str) -> Result<(), GuardedSysfsError> {
    tracing::info!(bdf, "engaging IRQ clutch — cleaning stale MSI/IRQ vectors");
    KmodBuilder::new(IRQ_CLUTCH_MODULE)
        .source(IRQ_CLUTCH_SOURCE)
        .tmpdir(&irq_clutch_tmpdir())
        .param("bdf", bdf)
        .build_and_load()
}

/// Disengage the IRQ clutch: unload the module and clean up.
pub fn disengage_irq_clutch() -> Result<(), GuardedSysfsError> {
    tracing::info!("disengaging IRQ clutch");
    KmodBuilder::unload_and_clean(IRQ_CLUTCH_MODULE, &irq_clutch_tmpdir())
}

/// Remove a single BDF from the `no_bus_reset` module's suppression list.
///
/// Reads the currently-loaded BDF parameter, removes the target, then
/// reloads the module with the remaining BDFs. If the target was the only
/// BDF, unloads entirely. This allows SBR for the target while keeping
/// other GPUs protected.
pub fn unsuppress_bus_reset_for(bdf: &str) -> Result<(), GuardedSysfsError> {
    let sys_param = crate::linux_paths::sysfs_module_parameter(NO_BUS_RESET_MODULE, "bdf");
    let param_path = Path::new(&sys_param);
    if !param_path.exists() {
        tracing::info!(bdf, "no_bus_reset module not loaded — SBR already allowed");
        return Ok(());
    }
    let loaded =
        std::fs::read_to_string(param_path).map_err(|e| GuardedSysfsError::WriteFailed {
            path: sys_param.clone(),
            reason: e.to_string(),
        })?;
    let remaining: Vec<&str> = loaded
        .trim()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != bdf)
        .collect();
    tracing::info!(
        bdf,
        loaded = loaded.trim(),
        remaining = remaining.join(",").as_str(),
        "unsuppressing SBR for target BDF"
    );
    KmodBuilder::unload_and_clean(NO_BUS_RESET_MODULE, &no_bus_reset_tmpdir())?;
    if !remaining.is_empty() {
        let combined = remaining.join(",");
        tracing::info!(combined, "reloading no_bus_reset for remaining devices");
        suppress_bus_reset(&combined)?;
    }
    Ok(())
}
