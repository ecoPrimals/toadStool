// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kernel module build, cache, and guarded load/unload operations.

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::driver_ops::reap_forked_child;
use super::GuardedSysfsError;
use super::{reap_or_orphan, INSMOD_TIMEOUT, RMMOD_TIMEOUT};

/// Run an arbitrary command with timeout (legacy fallback for non-kmod uses).
///
/// Kept for `KmodBuilder` which still needs `make` via `Command::new`.
pub fn kmod_guarded(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, GuardedSysfsError> {
    let args_str = args.join(" ");
    tracing::info!(cmd, args = args_str.as_str(), timeout_ms = timeout.as_millis() as u64,
                   "guarded kmod operation");

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| GuardedSysfsError::KmodFailed {
            cmd: cmd.into(),
            args: args_str.clone(),
            reason: format!("failed to spawn: {e}"),
        })?;

    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().unwrap_or_else(|_| {
                    std::process::Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    }
                });
                if status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    tracing::info!(cmd, args = args_str.as_str(),
                                   elapsed_ms = start.elapsed().as_millis() as u64,
                                   "kmod operation completed");
                    return Ok(stdout);
                }
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: cmd.into(),
                    args: args_str,
                    reason: stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(cmd, args = args_str.as_str(),
                                   timeout_ms = timeout.as_millis() as u64,
                                   "kmod operation timed out — killing child");
                    let _ = child.kill();
                    reap_or_orphan(&mut child, "kmod_guarded");
                    return Err(GuardedSysfsError::KmodTimeout {
                        cmd: cmd.into(),
                        args: args_str,
                        timeout_ms: timeout.as_millis() as u64,
                    });
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: cmd.into(),
                    args: args_str,
                    reason: format!("failed to poll child: {e}"),
                });
            }
        }
    }
}

/// Wait for a forked kmod child with timeout, kill on timeout.
fn wait_for_kmod_child(
    child_pid: rustix::process::Pid,
    label: &str,
    args_str: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    use rustix::process::{Signal, WaitOptions, waitpid};

    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match waitpid(Some(child_pid), WaitOptions::NOHANG) {
            Ok(Some((_pid, status))) => {
                if status.exited() && status.exit_status() == Some(0) {
                    tracing::info!(label, args = args_str,
                                   elapsed_ms = start.elapsed().as_millis() as u64,
                                   "kmod operation completed");
                    return Ok(());
                }
                let code = status.exit_status().unwrap_or(-1);
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: label.into(),
                    args: args_str.into(),
                    reason: format!("child exited with code {code}"),
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    tracing::warn!(label, args = args_str,
                                   timeout_ms = timeout.as_millis() as u64,
                                   "kmod operation timed out — killing child");
                    let _ = rustix::process::kill_process(child_pid, Signal::KILL);
                    reap_forked_child(child_pid);
                    return Err(GuardedSysfsError::KmodTimeout {
                        cmd: label.into(),
                        args: args_str.into(),
                        timeout_ms: timeout.as_millis() as u64,
                    });
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(GuardedSysfsError::KmodFailed {
                    cmd: label.into(),
                    args: args_str.into(),
                    reason: format!("waitpid failed: {e}"),
                });
            }
        }
    }
}

/// Guarded `insmod` — load a kernel module via `finit_module(2)` in a
/// forked child. Pure Rust, no `insmod` binary dependency.
pub fn insmod_guarded(ko_path: &Path, timeout: Duration) -> Result<(), GuardedSysfsError> {
    insmod_guarded_with_params(ko_path, "", timeout)
}

/// Guarded `insmod` with module parameters.
pub fn insmod_guarded_with_params(
    ko_path: &Path,
    params: &str,
    timeout: Duration,
) -> Result<(), GuardedSysfsError> {
    let path_str = ko_path.display().to_string();
    tracing::info!(path = path_str.as_str(), params,
                   timeout_ms = timeout.as_millis() as u64,
                   "guarded insmod (finit_module)");

    let ko_file = std::fs::File::open(ko_path).map_err(|e| GuardedSysfsError::KmodFailed {
        cmd: "finit_module".into(),
        args: path_str.clone(),
        reason: format!("failed to open .ko: {e}"),
    })?;

    let params_c = CString::new(params).map_err(|_| GuardedSysfsError::KmodFailed {
        cmd: "finit_module".into(),
        args: path_str.clone(),
        reason: "params contain NUL byte".into(),
    })?;

    // Pipe for errno propagation: child writes raw errno (4 bytes) on
    // failure, nothing on success. Parent reads after waitpid.
    let (pipe_read, pipe_write) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)
        .map_err(|e| GuardedSysfsError::KmodFailed {
            cmd: "finit_module".into(),
            args: path_str.clone(),
            reason: format!("pipe failed: {e}"),
        })?;

    // SAFETY: fork in multi-threaded context. Child calls only
    // finit_module (syscall) + write (pipe) + exit_group — all async-signal-safe.
    // ko_file fd is inherited by the child (not CLOEXEC).
    let fork_result = unsafe { rustix::runtime::kernel_fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::KmodFailed {
            cmd: "finit_module".into(),
            args: path_str,
            reason: format!("fork failed: {e}"),
        }),
        Ok(rustix::runtime::Fork::Child(_)) => {
            drop(pipe_read);
            match rustix::system::finit_module(&ko_file, &params_c, 0) {
                Ok(()) => rustix::runtime::exit_group(0),
                Err(e) => {
                    let errno = e.raw_os_error();
                    let _ = rustix::io::write(&pipe_write, &errno.to_ne_bytes());
                    rustix::runtime::exit_group(1)
                }
            }
        }
        Ok(rustix::runtime::Fork::ParentOf(child_pid)) => {
            drop(ko_file);
            drop(pipe_write);
            let result = wait_for_kmod_child(child_pid, "finit_module", &path_str, timeout);
            if let Err(GuardedSysfsError::KmodFailed { ref reason, .. }) = result
                && reason.starts_with("child exited with code")
            {
                let mut buf = [0u8; 4];
                if rustix::io::read(&pipe_read, &mut buf) == Ok(4) {
                    let errno = i32::from_ne_bytes(buf);
                    return Err(GuardedSysfsError::KmodFailed {
                        cmd: "finit_module".into(),
                        args: path_str,
                        reason: format!("finit_module errno {errno} ({})",
                            errno_name(errno)),
                    });
                }
            }
            result
        }
    }
}

/// Map common finit_module/delete_module errnos to human-readable names.
fn errno_name(errno: i32) -> &'static str {
    match errno {
        1 => "EPERM",
        2 => "ENOENT",
        12 => "ENOMEM",
        16 => "EBUSY",
        17 => "EEXIST",
        22 => "EINVAL",
        _ => "unknown",
    }
}

/// Guarded `rmmod` — unload a kernel module via `delete_module(2)` in a
/// forked child. Pure Rust, no `rmmod` binary dependency.
///
/// On failure, automatically retries with `O_NONBLOCK | O_TRUNC` (force
/// removal) as a zombie-killer fallback. This handles modules stuck in
/// cleanup due to NOP'd teardown paths.
pub fn rmmod_guarded(name: &str, timeout: Duration) -> Result<(), GuardedSysfsError> {
    match rmmod_with_flags(name, 0, timeout) {
        Ok(()) => Ok(()),
        Err(first_err) => {
            tracing::warn!(module = name, error = %first_err,
                           "normal rmmod failed — trying force rmmod (O_NONBLOCK|O_TRUNC)");
            match rmmod_with_flags(name, O_NONBLOCK | O_TRUNC, timeout) {
                Ok(()) => {
                    tracing::info!(module = name, "force rmmod succeeded (zombie buried)");
                    Ok(())
                }
                Err(force_err) => {
                    tracing::warn!(module = name, error = %force_err,
                                   "force rmmod also failed — module is a permanent zombie");
                    Err(first_err)
                }
            }
        }
    }
}

const O_NONBLOCK: i32 = 0x800;
const O_TRUNC: i32 = 0x200;

/// Inner `delete_module` with configurable flags.
fn rmmod_with_flags(name: &str, flags: i32, timeout: Duration) -> Result<(), GuardedSysfsError> {
    let flag_desc = if flags == 0 { "normal".to_string() }
                    else { format!("flags=0x{flags:x}") };
    tracing::info!(module = name, timeout_ms = timeout.as_millis() as u64,
                   mode = flag_desc.as_str(),
                   "guarded rmmod (delete_module)");

    let name_c = CString::new(name).map_err(|_| GuardedSysfsError::KmodFailed {
        cmd: "delete_module".into(),
        args: name.into(),
        reason: "name contains NUL byte".into(),
    })?;

    let (pipe_read, pipe_write) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)
        .map_err(|e| GuardedSysfsError::KmodFailed {
            cmd: "delete_module".into(),
            args: name.into(),
            reason: format!("pipe failed: {e}"),
        })?;

    // SAFETY: fork + delete_module syscall — async-signal-safe.
    let fork_result = unsafe { rustix::runtime::kernel_fork() };

    match fork_result {
        Err(e) => Err(GuardedSysfsError::KmodFailed {
            cmd: "delete_module".into(),
            args: name.into(),
            reason: format!("fork failed: {e}"),
        }),
        Ok(rustix::runtime::Fork::Child(_)) => {
            drop(pipe_read);
            match rustix::system::delete_module(&name_c, flags) {
                Ok(()) => rustix::runtime::exit_group(0),
                Err(e) => {
                    let errno = e.raw_os_error();
                    let _ = rustix::io::write(&pipe_write, &errno.to_ne_bytes());
                    rustix::runtime::exit_group(1)
                }
            }
        }
        Ok(rustix::runtime::Fork::ParentOf(child_pid)) => {
            drop(pipe_write);
            let result = wait_for_kmod_child(child_pid, "delete_module", name, timeout);
            if let Err(GuardedSysfsError::KmodFailed { ref reason, .. }) = result
                && reason.starts_with("child exited with code")
            {
                let mut buf = [0u8; 4];
                if rustix::io::read(&pipe_read, &mut buf) == Ok(4) {
                    let errno = i32::from_ne_bytes(buf);
                    return Err(GuardedSysfsError::KmodFailed {
                        cmd: "delete_module".into(),
                        args: name.into(),
                        reason: format!("delete_module errno {errno} ({}) [flags=0x{flags:x}]",
                            errno_name(errno)),
                    });
                }
            }
            result
        }
    }
}

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
///     .tmpdir("/tmp/toadstool-no-bus-reset")
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
            tmpdir: format!("/tmp/toadstool-kmod-{name}"),
            params: Vec::new(),
        }
    }

    /// Set the C source code for the module.
    pub fn source(mut self, src: &'static str) -> Self {
        self.source = src;
        self
    }

    /// Override the build directory (default: `/tmp/toadstool-kmod-{name}`).
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
        let sys_path = format!("/sys/module/{}", self.name);
        if !Path::new(&sys_path).exists() {
            return false;
        }
        for (key, value) in &self.params {
            let param_path = format!("{sys_path}/parameters/{key}");
            match std::fs::read_to_string(&param_path) {
                Ok(contents) if contents.trim() == value => {}
                _ => return false,
            }
        }
        true
    }

    /// If the module is already loaded, unload it first (idempotent reload).
    fn ensure_unloaded(&self) -> Result<(), GuardedSysfsError> {
        let sys_path = format!("/sys/module/{}", self.name);
        if Path::new(&sys_path).exists() {
            tracing::info!(module = self.name.as_str(),
                           "kmod already loaded — unloading for reload");
            let _ = rmmod_guarded(&self.name, RMMOD_TIMEOUT);
        }
        Ok(())
    }

    /// Stage source and Makefile, then compile via kbuild.
    ///
    /// Returns the path to the compiled `.ko` file. Does not load the
    /// module — use [`build_and_load`] for the full lifecycle, or call
    /// this when you only need the compiled artifact (e.g. ELF inspection
    /// in kernel health probes).
    pub fn compile_only(&self) -> Result<PathBuf, GuardedSysfsError> {
        let krel = crate::linux_paths::kernel_release().ok_or_else(|| {
            GuardedSysfsError::KmodFailed {
                cmd: "kernel_release".into(),
                args: String::new(),
                reason: "could not read /proc/sys/kernel/osrelease".into(),
            }
        })?;

        // Check persistent cache first — survives reboots, avoids kbuild
        // entirely when the kernel version hasn't changed.
        let cache_dir = PathBuf::from(format!(
            "/var/lib/toadstool/kmod-cache/{krel}"
        ));
        let cached_ko = cache_dir.join(format!("{}.ko", self.name));
        if cached_ko.exists() {
            tracing::info!(module = self.name.as_str(), krel,
                           path = %cached_ko.display(),
                           "kmod builder: using cached .ko");
            return Ok(cached_ko);
        }

        let kbuild = crate::linux_paths::kbuild_dir().ok_or_else(|| {
            GuardedSysfsError::KmodFailed {
                cmd: "kbuild_dir".into(),
                args: String::new(),
                reason: "kernel release unavailable for kbuild path".into(),
            }
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
        tracing::info!(module = self.name.as_str(), krel,
                       "kmod builder: compiling via kbuild");
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
            tracing::info!(module = self.name.as_str(),
                           "kmod already loaded with correct params — skipping rebuild");
            return Ok(());
        }

        self.ensure_unloaded()?;

        let ko_path = self.compile_only()?;

        let params_str: String = self.params.iter()
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
    /// Deletes the entire build directory. Use after [`compile_only`] when
    /// the `.ko` has been consumed and is no longer needed.
    pub fn clean(tmpdir: &str) {
        let path = Path::new(tmpdir);
        if path.exists() {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    /// Unload the module and clean up build artifacts.
    pub fn unload_and_clean(name: &str, tmpdir: &str) -> Result<(), GuardedSysfsError> {
        let sys_path = format!("/sys/module/{name}");
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
const NO_BUS_RESET_TMPDIR: &str = "/tmp/toadstool-no-bus-reset";

const NO_BUS_RESET_SOURCE: &str = r#"
#include <linux/module.h>
#include <linux/pci.h>
#include <linux/string.h>

static char *bdf = "";
module_param(bdf, charp, 0444);
MODULE_PARM_DESC(bdf, "Comma-separated PCI BDFs to suppress bus reset for");

#define MAX_TARGETS 8
static struct pci_dev *targets[MAX_TARGETS];
static int ntargets;

static int __init no_bus_reset_init(void) {
    char buf[256], *p, *tok;
    struct pci_dev *dev;

    strscpy(buf, bdf, sizeof(buf));
    p = buf;
    while ((tok = strsep(&p, ",")) != NULL && ntargets < MAX_TARGETS) {
        while (*tok == ' ') tok++;
        if (*tok == '\0') continue;
        dev = NULL;
        while ((dev = pci_get_device(PCI_ANY_ID, PCI_ANY_ID, dev))) {
            if (strcmp(dev_name(&dev->dev), tok) == 0) {
                dev->dev_flags |= PCI_DEV_FLAGS_NO_BUS_RESET;
                targets[ntargets++] = dev;
                pr_info("no_bus_reset: suppressed on %s\n", tok);
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
            targets[i]->dev_flags &= ~PCI_DEV_FLAGS_NO_BUS_RESET;
            pr_info("no_bus_reset: restored on %s\n", dev_name(&targets[i]->dev));
            pci_dev_put(targets[i]);
        }
    }
}

module_init(no_bus_reset_init);
module_exit(no_bus_reset_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Suppress PCI bus reset for specified devices");
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
    let sys_param = format!("/sys/module/{NO_BUS_RESET_MODULE}/parameters/bdf");
    if Path::new(&sys_param).exists()
        && let Ok(loaded_bdfs) = std::fs::read_to_string(&sys_param)
    {
        let loaded = loaded_bdfs.trim();
        let already_covered = bdf.split(',')
            .all(|b| loaded.split(',').any(|l| l.trim() == b.trim()));
        if already_covered {
            tracing::info!(bdf, loaded, "no_bus_reset already covers this device");
            return Ok(());
        }
        // Need to reload with the union of old + new BDFs
        let mut all_bdfs: Vec<&str> = loaded.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for b in bdf.split(',').map(|s| s.trim()) {
            if !all_bdfs.contains(&b) {
                all_bdfs.push(b);
            }
        }
        let combined = all_bdfs.join(",");
        tracing::info!(bdf, combined, "reloading no_bus_reset with expanded device list");
        return KmodBuilder::new(NO_BUS_RESET_MODULE)
            .source(NO_BUS_RESET_SOURCE)
            .tmpdir(NO_BUS_RESET_TMPDIR)
            .param("bdf", &combined)
            .build_and_load();
    }

    KmodBuilder::new(NO_BUS_RESET_MODULE)
        .source(NO_BUS_RESET_SOURCE)
        .tmpdir(NO_BUS_RESET_TMPDIR)
        .param("bdf", bdf)
        .build_and_load()
}

/// Unload the `no_bus_reset` kernel module and clean up build artifacts.
///
/// Clears `PCI_DEV_FLAGS_NO_BUS_RESET` on the target device (via the
/// module's exit handler) and removes the tmpdir.
pub fn restore_bus_reset() -> Result<(), GuardedSysfsError> {
    KmodBuilder::unload_and_clean(NO_BUS_RESET_MODULE, NO_BUS_RESET_TMPDIR)
}

/// Remove a single BDF from the `no_bus_reset` module's suppression list.
///
/// Reads the currently-loaded BDF parameter, removes the target, then
/// reloads the module with the remaining BDFs. If the target was the only
/// BDF, unloads entirely. This allows SBR for the target while keeping
/// other GPUs protected.
pub fn unsuppress_bus_reset_for(bdf: &str) -> Result<(), GuardedSysfsError> {
    let sys_param = format!("/sys/module/{NO_BUS_RESET_MODULE}/parameters/bdf");
    let param_path = Path::new(&sys_param);
    if !param_path.exists() {
        tracing::info!(bdf, "no_bus_reset module not loaded — SBR already allowed");
        return Ok(());
    }
    let loaded = std::fs::read_to_string(param_path)
        .map_err(|e| GuardedSysfsError::WriteFailed {
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
    KmodBuilder::unload_and_clean(NO_BUS_RESET_MODULE, NO_BUS_RESET_TMPDIR)?;
    if !remaining.is_empty() {
        let combined = remaining.join(",");
        tracing::info!(combined, "reloading no_bus_reset for remaining devices");
        suppress_bus_reset(&combined)?;
    }
    Ok(())
}
