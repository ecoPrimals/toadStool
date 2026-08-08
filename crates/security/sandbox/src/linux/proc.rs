// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure Rust parsers for `/proc`, cgroup v2, and related pseudo-files.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use toadstool_common::platform::SystemParameters;
use toadstool_hw_safe::LinuxSystemParameters;

use super::constants::CGROUP2_FS_ROOT;
use crate::types::ResourceUsage;

/// Parsed fields from `/proc/[pid]/stat` needed for CPU jiffies.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcStatCpu {
    /// Fields 14+15: utime + stime in clock ticks.
    pub total_jiffies: u64,
}

/// Errors from proc/cgroup parsing (structured; map at call site to [`toadstool::error::ToadStoolError`]).
#[derive(Debug)]
#[expect(
    dead_code,
    reason = "variants constructed for completeness; consumers use ToadStoolError"
)]
pub(crate) enum ProcParseError {
    /// File missing or unreadable.
    Io(std::io::Error),
    /// Unexpected line layout.
    Malformed(&'static str),
}

impl From<std::io::Error> for ProcParseError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Read `/proc/[pid]/stat` and return utime+stime jiffies.
pub(crate) fn read_proc_stat_cpu(pid: u32) -> Result<ProcStatCpu, ProcParseError> {
    let path = format!("/proc/{pid}/stat");
    let data = fs::read_to_string(&path)?;
    parse_proc_stat_cpu_line(&data).ok_or(ProcParseError::Malformed("stat fields"))
}

/// Parse `/proc/[pid]/stat` contents. Comm name may contain spaces; kernel uses `)` after comm.
pub(crate) fn parse_proc_stat_cpu_line(stat: &str) -> Option<ProcStatCpu> {
    let rp = stat.rfind(')')?;
    let after = stat.get(rp + 2..)?; // skip ") "
    let mut it = after.split_whitespace();
    let _state = it.next()?;
    let _ppid = it.next()?;
    let _pgrp = it.next()?;
    let _session = it.next()?;
    let _tty_nr = it.next()?;
    let _tpgid = it.next()?;
    let _flags = it.next()?;
    let _minflt = it.next()?;
    let _cminflt = it.next()?;
    let _majflt = it.next()?;
    let _cmajflt = it.next()?;
    let utime = it.next()?.parse::<u64>().ok()?;
    let stime = it.next()?.parse::<u64>().ok()?;
    Some(ProcStatCpu {
        total_jiffies: utime.saturating_add(stime),
    })
}

/// Read `VmRSS` from `/proc/[pid]/status` (value in kB).
pub(crate) fn read_vm_rss_kb(pid: u32) -> Result<u64, ProcParseError> {
    let path = format!("/proc/{pid}/status");
    let data = fs::read_to_string(&path)?;
    parse_vm_rss_kb(&data).ok_or(ProcParseError::Malformed("VmRSS"))
}

pub(crate) fn parse_vm_rss_kb(status: &str) -> Option<u64> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let mut it = rest.split_whitespace();
            let kb = it.next()?.parse::<u64>().ok()?;
            return Some(kb);
        }
    }
    None
}

/// Read `Threads:` from `/proc/[pid]/status`.
pub(crate) fn read_thread_count(pid: u32) -> Result<u32, ProcParseError> {
    let path = format!("/proc/{pid}/status");
    let data = fs::read_to_string(&path)?;
    parse_thread_count(&data).ok_or(ProcParseError::Malformed("Threads"))
}

pub(crate) fn parse_thread_count(status: &str) -> Option<u32> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Threads:") {
            let n = rest.split_whitespace().next()?.parse::<u32>().ok()?;
            return Some(n);
        }
    }
    None
}

/// Count open file descriptors in `/proc/[pid]/fd`.
pub(crate) fn count_open_fds(pid: u32) -> Result<u32, ProcParseError> {
    let dir = format!("/proc/{pid}/fd");
    let rd = fs::read_dir(&dir)?;
    Ok(rd.filter_map(Result::ok).count() as u32)
}

/// Parse unified cgroup v2 relative path from `/proc/[pid]/cgroup` (line starting with `0::`).
pub(crate) fn parse_cgroup_v2_relative_path(cgroup: &str) -> Option<PathBuf> {
    for line in cgroup.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("0::") {
            let trimmed = rest.trim_start_matches('/');
            if trimmed.is_empty() {
                return Some(PathBuf::new());
            }
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

/// Read cgroup v2 `memory.current` bytes, if present.
pub(crate) fn read_cgroup_memory_current(rel: &Path) -> Option<u64> {
    let p = Path::new(CGROUP2_FS_ROOT).join(rel).join("memory.current");
    let s = fs::read_to_string(&p).ok()?;
    parse_first_u64_line(&s)
}

/// Parse `cpu.stat` for `usage_usec` (microseconds of CPU time).
pub(crate) fn parse_cpu_stat_usage_usec(cpu_stat: &str) -> Option<u64> {
    for line in cpu_stat.lines() {
        let mut it = line.split_whitespace();
        let key = it.next()?;
        if key == "usage_usec" {
            return it.next()?.parse::<u64>().ok();
        }
    }
    None
}

pub(crate) fn read_cgroup_cpu_usage_usec(rel: &Path) -> Option<u64> {
    let p = Path::new(CGROUP2_FS_ROOT).join(rel).join("cpu.stat");
    let s = fs::read_to_string(&p).ok()?;
    parse_cpu_stat_usage_usec(&s)
}

fn parse_first_u64_line(s: &str) -> Option<u64> {
    s.lines().next()?.trim().parse::<u64>().ok()
}

/// Rolling sample for jiffies-based CPU percent.
#[derive(Debug, Clone)]
pub(crate) struct JiffiesCpuSampler {
    last_jiffies: Option<u64>,
    last_instant: Option<Instant>,
}

impl Default for JiffiesCpuSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl JiffiesCpuSampler {
    pub(crate) const fn new() -> Self {
        Self {
            last_jiffies: None,
            last_instant: None,
        }
    }

    /// Updates internal state and returns CPU percent since last call, or `None` on first call.
    pub(crate) fn observe(&mut self, total_jiffies: u64) -> Option<f64> {
        let now = Instant::now();
        let pct = match (self.last_jiffies, self.last_instant) {
            (Some(pj), Some(pi)) => {
                let hz = LinuxSystemParameters.clock_ticks_per_second().max(1) as f64;
                let dt = now.duration_since(pi).as_secs_f64();
                if dt <= f64::EPSILON {
                    return Some(0.0);
                }
                let dj = total_jiffies.saturating_sub(pj) as f64;
                Some(((dj / hz) / dt * 100.0).clamp(0.0, 100.0))
            }
            _ => None,
        };
        self.last_jiffies = Some(total_jiffies);
        self.last_instant = Some(now);
        pct
    }
}

/// Build [`ResourceUsage`] from proc + optional cgroup paths.
pub(crate) fn build_resource_usage(
    pid: u32,
    cgroup_rel: Option<&Path>,
    sampler: &mut JiffiesCpuSampler,
) -> ResourceUsage {
    let mut usage = ResourceUsage::default();

    let stat_cpu = read_proc_stat_cpu(pid);
    if let Ok(sc) = &stat_cpu {
        if let Some(pct) = sampler.observe(sc.total_jiffies) {
            usage.cpu_percent = pct;
        }
    } else {
        tracing::warn!(pid, "could not read /proc/{pid}/stat for CPU jiffies");
    }

    match read_vm_rss_kb(pid) {
        Ok(kb) => {
            usage.memory_bytes = kb.saturating_mul(1024);
        }
        Err(_) => {
            tracing::warn!(pid, "could not read VmRSS from /proc/{pid}/status");
        }
    }

    if let Some(rel) = cgroup_rel {
        if let Some(use_usec) = read_cgroup_cpu_usage_usec(rel) {
            tracing::debug!(pid, cgroup_cpu_usage_usec = use_usec, "cgroup v2 cpu.stat");
        }
        if let Some(mem) = read_cgroup_memory_current(rel) {
            usage.memory_bytes = mem;
        }
    }

    if let Ok(n) = read_thread_count(pid) {
        usage.processes = n;
    } else {
        tracing::warn!(pid, "could not read thread count");
    }

    if let Ok(n) = count_open_fds(pid) {
        usage.file_descriptors = n;
    } else {
        tracing::warn!(pid, "could not enumerate /proc/{pid}/fd");
    }

    usage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proc_stat_cpu_synthetic() {
        // Fields 3–13 after `)` are state, ppid…cmajflt; 14–15 are utime+stime jiffies.
        let line = "12345 (test) R 1 2 3 4 5 6 7 8 9 10 1000 2000";
        let cpu = parse_proc_stat_cpu_line(line).expect("parse");
        assert_eq!(cpu.total_jiffies, 3000);
    }

    #[test]
    fn parse_vm_rss_synthetic() {
        let s = "Name:\tfoo\nVmRSS:\t   128 kB\n";
        assert_eq!(parse_vm_rss_kb(s), Some(128));
    }

    #[test]
    fn parse_threads_synthetic() {
        let s = "Threads:\t5\n";
        assert_eq!(parse_thread_count(s), Some(5));
    }

    #[test]
    fn parse_cgroup_v2_path() {
        let s = "1:name=systemd:/user.slice\n0::/user.slice/user-1000.slice/session-1.scope\n";
        let p = parse_cgroup_v2_relative_path(s).expect("path");
        assert!(p.to_string_lossy().contains("session-1.scope"));
    }

    #[test]
    fn parse_cpu_stat_usage_usec_line() {
        let s = "usage_usec 1234567\nuser_usec 100\nsystem_usec 200\n";
        assert_eq!(parse_cpu_stat_usage_usec(s), Some(1_234_567));
    }
}
