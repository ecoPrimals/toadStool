// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
use toadstool_common::platform;

/// Linux system parameters — clock ticks, page size, huge pages.
///
/// Implements [`platform::SystemParameters`] via `rustix::param`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxSystemParameters;

#[cfg(target_os = "linux")]
impl platform::SystemParameters for LinuxSystemParameters {
    fn clock_ticks_per_second(&self) -> u64 {
        rustix::param::clock_ticks_per_second()
    }

    fn page_size(&self) -> usize {
        rustix::param::page_size()
    }

    fn huge_page_size(&self) -> Option<usize> {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|info| {
                info.lines()
                    .find(|l| l.starts_with("Hugepagesize:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|kb| kb.parse::<usize>().ok())
                    .map(|kb| kb * 1024)
            })
    }
}

/// Linux privilege probe — capability-based privilege checking.
///
/// Implements [`platform::PrivilegeProbe`] using `rustix::thread::capabilities`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPrivilegeProbeBackend;

#[cfg(target_os = "linux")]
impl platform::PrivilegeProbe for LinuxPrivilegeProbeBackend {
    fn has_privilege(&self, privilege: &str) -> bool {
        let Ok(caps) = rustix::thread::capabilities(None) else {
            return false;
        };
        match privilege {
            "sys_admin" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::SYS_ADMIN),
            "net_raw" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::NET_RAW),
            "sys_rawio" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::SYS_RAWIO),
            "dac_override" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::DAC_OVERRIDE),
            _ => false,
        }
    }

    fn active_privileges(&self) -> Vec<&'static str> {
        let Ok(caps) = rustix::thread::capabilities(None) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::SYS_ADMIN)
        {
            result.push("sys_admin");
        }
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::NET_RAW)
        {
            result.push("net_raw");
        }
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::SYS_RAWIO)
        {
            result.push("sys_rawio");
        }
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::DAC_OVERRIDE)
        {
            result.push("dac_override");
        }
        result
    }

    fn is_elevated(&self) -> bool {
        self.has_privilege("sys_admin")
    }
}

/// Get monotonic clock time in nanoseconds.
#[cfg(target_os = "linux")]
pub fn clock_monotonic_ns() -> u64 {
    let ts = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// Filesystem statistics (capacity/free space).
#[derive(Debug, Clone)]
pub struct FsStats {
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Available bytes to unprivileged users.
    pub available_bytes: u64,
    /// Total number of inodes.
    pub total_inodes: u64,
    /// Available inodes.
    pub available_inodes: u64,
}

/// Query filesystem statistics for a path (wraps `statvfs`).
#[cfg(target_os = "linux")]
pub fn fs_stats(path: &Path) -> std::io::Result<FsStats> {
    let st = rustix::fs::statvfs(path).map_err(std::io::Error::from)?;
    Ok(FsStats {
        total_bytes: st.f_blocks * st.f_frsize,
        available_bytes: st.f_bavail * st.f_frsize,
        total_inodes: st.f_files,
        available_inodes: st.f_favail,
    })
}
