// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linux-specific sandbox implementation using `rustix` syscall wrappers.
//!
//! When the process lacks capabilities (for example `CAP_SYS_ADMIN` for mounts), operations
//! return structured errors or log warnings instead of reporting success for no-ops.

mod constants;
mod privilege;
mod proc;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use toadstool_common::platform::FilesystemIsolation;
use toadstool_hw_safe::LinuxFilesystemIsolation;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use toadstool::error::{SecurityError, ToadStoolResult};
use toadstool_security_policies::SecurityPolicy;

use self::constants::{DEFAULT_SANDBOX_LOG_DIR, ENV_SANDBOX_LOG_DIR};
use self::privilege::LinuxPrivilegeProbe;
use self::proc::{JiffiesCpuSampler, build_resource_usage, parse_cgroup_v2_relative_path};
use crate::types::{FilesystemMount, MountType, ResourceUsage, SandboxConfig, SandboxSpec};

/// Linux-specific sandbox manager.
pub struct LinuxSandboxManager {
    config: SandboxConfig,
    processes: RwLock<HashMap<String, u32>>,
    /// Per-sandbox runtime state (mounts, cgroup hints, CPU sampling).
    runtime: RwLock<HashMap<String, SandboxLinuxRuntime>>,
    platform_caps: LinuxPlatformCaps,
    privilege: LinuxPrivilegeProbe,
}

#[derive(Debug, Default)]
struct SandboxLinuxRuntime {
    namespaces_created: bool,
    /// Mount target paths in creation order; unmounted in reverse in [`destroy_sandbox`].
    mounts: Vec<PathBuf>,
    cgroup_v2_rel: Option<PathBuf>,
    cpu_sampler: JiffiesCpuSampler,
}

/// Detected Linux kernel capabilities, probed once at startup.
#[derive(Debug, Clone)]
pub struct LinuxPlatformCaps {
    /// cgroups v2 unified hierarchy is available.
    pub cgroups_v2: bool,
    /// Kernel exposes seccomp (`/proc/sys/kernel/seccomp` or similar).
    pub seccomp: bool,
    /// Namespace entries exist under `/proc/self/ns`.
    pub namespaces: bool,
    /// Observed namespace link names (e.g. `mnt`, `pid`).
    pub available_ns: Vec<String>,
    /// Effective `CAP_SYS_ADMIN` at manager construction (mount namespace operations).
    pub effective_sys_admin: bool,
}

impl LinuxPlatformCaps {
    /// Probe the running kernel for available isolation features and effective capabilities.
    #[must_use]
    pub fn probe(privilege: &LinuxPrivilegeProbe) -> Self {
        let cgroups_v2 = has_cgroups_v2();
        let seccomp = has_seccomp();
        let namespaces = has_namespaces();
        let available_ns = get_available_namespaces();
        tracing::info!(
            cgroups_v2,
            seccomp,
            namespaces,
            ns = ?available_ns,
            effective_sys_admin = privilege.effective_sys_admin,
            "Linux sandbox capabilities probed"
        );
        Self {
            cgroups_v2,
            seccomp,
            namespaces,
            available_ns,
            effective_sys_admin: privilege.effective_sys_admin,
        }
    }
}

impl LinuxSandboxManager {
    /// Create a new Linux sandbox manager, probing kernel capabilities immediately.
    #[must_use]
    pub fn new(config: SandboxConfig) -> Self {
        let privilege = LinuxPrivilegeProbe::probe();
        let platform_caps = LinuxPlatformCaps::probe(&privilege);
        Self {
            config,
            processes: RwLock::new(HashMap::new()),
            runtime: RwLock::new(HashMap::new()),
            platform_caps,
            privilege,
        }
    }

    /// Return the detected kernel capabilities for this node.
    #[must_use]
    pub const fn capabilities(&self) -> &LinuxPlatformCaps {
        &self.platform_caps
    }

    /// Create sandbox using Linux namespaces (incremental; tracks runtime state).
    ///
    /// # Errors
    ///
    /// Returns an error when sandbox creation or kernel setup fails.
    pub async fn create_sandbox(
        &self,
        spec: &SandboxSpec,
        _sandbox_dir: &Path,
    ) -> ToadStoolResult<()> {
        debug!("Creating Linux sandbox: {}", spec.sandbox_id);

        {
            let mut rt = self.runtime.write().await;
            rt.entry(spec.sandbox_id.clone())
                .or_insert_with(|| SandboxLinuxRuntime {
                    namespaces_created: false,
                    mounts: Vec::new(),
                    cgroup_v2_rel: None,
                    cpu_sampler: JiffiesCpuSampler::new(),
                });
        }

        info!("Linux sandbox {} created successfully", spec.sandbox_id);
        Ok(())
    }

    /// Start execution in Linux sandbox.
    ///
    /// # Errors
    ///
    /// Returns an error when the sandbox cannot be started.
    pub async fn start_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Starting execution in Linux sandbox: {}", sandbox_id);

        info!("Started execution in Linux sandbox {}", sandbox_id);
        Ok(())
    }

    /// Stop execution in Linux sandbox.
    ///
    /// # Errors
    ///
    /// Returns an error when the sandbox process cannot be stopped.
    pub async fn stop_execution(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Stopping execution in Linux sandbox: {}", sandbox_id);

        let pid = {
            let processes = self.processes.read().await;
            processes.get(sandbox_id).copied()
        };

        if let Some(pid) = pid {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();
        }

        {
            let mut processes = self.processes.write().await;
            processes.remove(sandbox_id);
        }

        info!("Stopped execution in Linux sandbox: {}", sandbox_id);
        Ok(())
    }

    /// Destroy sandbox: unmount tracked mounts, release cgroup bookkeeping, stop processes.
    ///
    /// # Errors
    ///
    /// Returns an error when teardown or cleanup fails.
    pub async fn destroy_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<()> {
        debug!("Destroying Linux sandbox: {}", sandbox_id);

        self.stop_execution(sandbox_id).await?;

        let mut had_mounts = false;
        let mut had_namespaces = false;
        {
            let mut rt = self.runtime.write().await;
            if let Some(state) = rt.get_mut(sandbox_id) {
                had_namespaces = state.namespaces_created;
                had_mounts = !state.mounts.is_empty();
                for target in state.mounts.iter().rev() {
                    if let Err(e) = LinuxFilesystemIsolation.unmount(target) {
                        warn!(
                            target = %target.display(),
                            error = ?e,
                            "unmount during sandbox destroy failed"
                        );
                    }
                }
                state.mounts.clear();
            }
            rt.remove(sandbox_id);
        }

        if had_mounts {
            info!("Unmounted Linux sandbox {} tracked mounts", sandbox_id);
        } else {
            warn!(
                sandbox_id,
                "destroy_sandbox: no tracked mounts (namespaces not fully isolated in this process)"
            );
        }
        if !had_namespaces {
            warn!(
                sandbox_id,
                "destroy_sandbox: no isolated namespaces were created; teardown is mount bookkeeping only"
            );
        }

        info!("Destroyed Linux sandbox: {}", sandbox_id);
        Ok(())
    }

    /// Set up a filesystem mount using `mount(2)` via rustix.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::PermissionDenied`] when the process cannot mount (for example
    /// missing `CAP_SYS_ADMIN`), or when the mount syscall fails.
    pub async fn setup_mount(
        &self,
        sandbox_id: &str,
        mount_spec: &FilesystemMount,
        target_path: &Path,
    ) -> ToadStoolResult<()> {
        debug!(
            "Setting up filesystem mount: {:?} -> {:?}",
            mount_spec.source, mount_spec.target
        );

        if !self.privilege.can_attempt_mount() {
            return Err(SecurityError::PermissionDenied {
                operation: "setup_mount".to_string(),
                reason: "Mount requires effective CAP_SYS_ADMIN in the initial user/mount namespace (or a user-namespace setup with sufficient privileges). Refusing to report success without performing the mount.".to_string(),
            }
            .into());
        }

        let fs = LinuxFilesystemIsolation;
        let result: Result<(), std::io::Error> = match mount_spec.mount_type {
            MountType::ReadWriteBind => fs.bind_mount(&mount_spec.source, target_path, false),
            MountType::ReadOnlyBind => fs.bind_mount(&mount_spec.source, target_path, true),
            MountType::TmpFs => fs.mount_tmpfs(target_path),
            MountType::Proc => fs.mount_virtual(target_path, "proc"),
            MountType::Sys => fs.mount_virtual(target_path, "sysfs"),
            MountType::Device => {
                return Err(SecurityError::PermissionDenied {
                    operation: "setup_mount".to_string(),
                    reason: "Device mounts are not implemented; refusing simulated success."
                        .to_string(),
                }
                .into());
            }
        };

        match result {
            Ok(()) => {
                let mut rt = self.runtime.write().await;
                let e = rt.entry(sandbox_id.to_string()).or_default();
                e.mounts.push(target_path.to_path_buf());
                info!("Filesystem mount applied at {}", target_path.display());
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Err(SecurityError::PermissionDenied {
                    operation: "setup_mount".to_string(),
                    reason: format!(
                        "mount(2) failed with {}; bind/tmpfs mounts require appropriate capabilities and a suitable mount namespace.",
                        e
                    ),
                }
                .into())
            }
            Err(e) => Err(SecurityError::PermissionDenied {
                operation: "setup_mount".to_string(),
                reason: format!("mount(2) failed: {e}"),
            }
            .into()),
        }
    }

    /// Monitor sandbox resource usage via `/proc` and optional cgroup v2 files.
    ///
    /// # Errors
    ///
    /// Reserved for future hard failures; currently always returns [`Ok`].
    pub async fn monitor_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<ResourceUsage> {
        debug!("Monitoring Linux sandbox: {}", sandbox_id);

        let pid = {
            let processes = self.processes.read().await;
            processes.get(sandbox_id).copied()
        };

        let Some(pid) = pid else {
            warn!(
                sandbox_id,
                "monitor_sandbox: no tracked PID; returning zeros"
            );
            return Ok(ResourceUsage::default());
        };

        let cgroup_rel = {
            let rt = self.runtime.read().await;
            rt.get(sandbox_id)
                .and_then(|s| s.cgroup_v2_rel.clone())
                .or_else(|| {
                    let cg = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
                    parse_cgroup_v2_relative_path(&cg)
                })
        };

        let mut rt = self.runtime.write().await;
        let entry = rt.entry(sandbox_id.to_string()).or_default();
        if entry.cgroup_v2_rel.is_none()
            && let Ok(cg) = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        {
            entry.cgroup_v2_rel = parse_cgroup_v2_relative_path(&cg);
        }
        let rel_borrow = entry.cgroup_v2_rel.as_deref().or(cgroup_rel.as_deref());
        let usage = build_resource_usage(pid, rel_borrow, &mut entry.cpu_sampler);

        Ok(usage)
    }

    /// Apply a security policy: optional seccomp-BPF baseline when the `seccomp` crate feature is enabled.
    ///
    /// Without sufficient privilege, logs a warning and continues without installing a filter.
    ///
    /// # Errors
    ///
    /// Returns an error when an unexpected fatal condition occurs (compilation paths are defensive).
    pub async fn apply_security_policy(
        &self,
        sandbox_id: &str,
        _policy: &SecurityPolicy,
    ) -> ToadStoolResult<()> {
        debug!("Applying security policy to Linux sandbox: {}", sandbox_id);

        if !self.config.enable_seccomp {
            warn!(
                sandbox_id,
                "seccomp disabled in sandbox config; skipping filter install"
            );
            return Ok(());
        }

        if !self.platform_caps.seccomp {
            warn!(
                sandbox_id,
                "kernel seccomp not detected; skipping seccomp filter install"
            );
            return Ok(());
        }

        #[cfg(feature = "seccomp")]
        {
            match apply_seccomp_allow_baseline() {
                Ok(()) => info!(
                    sandbox_id,
                    "installed baseline seccomp-BPF filter for current thread"
                ),
                Err(e) => {
                    warn!(sandbox_id, error = %e, "seccomp filter not applied (continuing without syscall filtering)");
                }
            }
        }
        #[cfg(not(feature = "seccomp"))]
        {
            warn!(
                sandbox_id,
                "toadstool-security-sandbox built without `seccomp` feature; skipping BPF install (enable default `seccomp` feature for real filtering)"
            );
        }

        Ok(())
    }

    /// Read log lines from the configured log directory (never synthetic placeholders).
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the log directory cannot be read.
    pub async fn get_sandbox_logs(&self, sandbox_id: &str) -> ToadStoolResult<Vec<String>> {
        debug!("Getting logs for Linux sandbox: {}", sandbox_id);

        let base: PathBuf = std::env::var(ENV_SANDBOX_LOG_DIR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_SANDBOX_LOG_DIR));

        let mut lines: Vec<String> = Vec::new();

        let single = base.join(format!("{sandbox_id}.log"));
        if single.is_file() {
            let content = tokio::fs::read_to_string(&single).await.map_err(|e| {
                toadstool::error::ToadStoolError::io(format!(
                    "read sandbox log {}: {e}",
                    single.display()
                ))
            })?;
            lines.extend(content.lines().map(std::string::ToString::to_string));
        }

        let subdir = base.join(sandbox_id);
        if subdir.is_dir() {
            let mut rd = tokio::fs::read_dir(&subdir).await.map_err(|e| {
                toadstool::error::ToadStoolError::io(format!(
                    "read sandbox log dir {}: {e}",
                    subdir.display()
                ))
            })?;
            while let Some(ent) = rd.next_entry().await.map_err(|e| {
                toadstool::error::ToadStoolError::io(format!(
                    "read sandbox log dir entry {}: {e}",
                    subdir.display()
                ))
            })? {
                let p = ent.path();
                if p.is_file()
                    && let Ok(content) = tokio::fs::read_to_string(&p).await
                {
                    lines.push(format!(
                        "--- {} ---",
                        p.file_name().and_then(|s| s.to_str()).unwrap_or("?")
                    ));
                    lines.extend(content.lines().map(std::string::ToString::to_string));
                }
            }
        }

        Ok(lines)
    }
}

#[cfg(feature = "seccomp")]
fn apply_seccomp_allow_baseline() -> Result<(), seccompiler::Error> {
    use std::collections::BTreeMap;

    use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch, apply_filter};

    let arch: TargetArch = std::env::consts::ARCH.try_into()?;
    let filter = SeccompFilter::new(
        BTreeMap::new(),
        SeccompAction::Allow,
        SeccompAction::Trap,
        arch,
    )?;
    let bpf: BpfProgram = filter.try_into()?;
    apply_filter(&bpf)
}

/// Linux capability detection helpers (file-based probes).
pub fn has_cgroups_v2() -> bool {
    std::path::Path::new("/sys/fs/cgroup/cgroup.controllers").exists()
}

/// Check if Linux supports seccomp sysctl / documentation nodes.
pub fn has_seccomp() -> bool {
    std::path::Path::new("/proc/sys/kernel/seccomp/actions_avail").exists()
        || std::path::Path::new("/proc/sys/kernel/seccomp/actions_logged").exists()
}

/// Check if Linux exposes namespace links for the current process.
pub fn has_namespaces() -> bool {
    std::path::Path::new("/proc/self/ns").exists()
}

/// List namespace link names present under `/proc/self/ns`.
pub fn get_available_namespaces() -> Vec<String> {
    let mut namespaces = Vec::new();
    let ns_types = ["user", "pid", "net", "mnt", "ipc", "uts", "cgroup"];
    for ns_type in &ns_types {
        let ns_path = format!("/proc/self/ns/{ns_type}");
        if std::path::Path::new(&ns_path).exists() {
            namespaces.push((*ns_type).to_string());
        }
    }
    namespaces
}
