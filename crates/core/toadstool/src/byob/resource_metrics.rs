// SPDX-License-Identifier: AGPL-3.0-or-later
//! cgroup v2 and `/proc` resource sampling for BYOB deployments (pure Rust parsing).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tokio::fs;
use tracing::{debug, warn};

use super::byob_types::ServiceSpec;

/// Rolling state for delta-based CPU and I/O metrics between polls.
#[derive(Debug, Clone)]
#[allow(
    clippy::struct_field_names,
    reason = "fields share `last_` prefix for delta-tracking clarity"
)]
pub(crate) struct ResourcePollState {
    pub last_instant: Instant,
    pub last_cpu_usage_usec: Option<u64>,
    pub last_proc_ticks: Option<u64>,
    pub last_net_rx: Option<u64>,
    pub last_net_tx: Option<u64>,
}

/// Reads host metrics: cgroup v2 first, then `/proc`, then caller-supplied simulation.
pub(crate) struct ResourceMetricsReader {
    cgroup_root: PathBuf,
}

impl Default for ResourceMetricsReader {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceMetricsReader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
        }
    }

    /// Sample metrics for `pid`, using `prev` for CPU and I/O deltas.
    pub async fn sample(
        &self,
        pid: u32,
        prev: Option<&ResourcePollState>,
    ) -> (DeploymentResourceSample, ResourcePollState) {
        let now = Instant::now();
        let cgroup_dir = self.discover_cgroup_v2_dir(pid).await;

        let mut cgroup_mem: Option<u64> = None;
        let mut cgroup_cpu_usec: Option<u64> = None;
        let mut cgroup_io_r: u64 = 0;
        let mut cgroup_io_w: u64 = 0;

        if let Some(ref dir) = cgroup_dir {
            if let Some(s) = read_file_join(dir, "memory.current").await {
                cgroup_mem = parse_single_u64_line(&s);
            }
            let cgroup_mem_max = read_file_join(dir, "memory.max")
                .await
                .as_deref()
                .and_then(parse_cgroup_memory_max);
            debug!(
                memory_current = cgroup_mem,
                memory_max = cgroup_mem_max,
                "cgroup v2 memory"
            );
            if let Some(s) = read_file_join(dir, "cpu.stat").await {
                cgroup_cpu_usec = parse_cpu_stat_usage_usec(&s);
            }
            if let Some(s) = read_file_join(dir, "io.stat").await {
                let (r, w) = parse_io_stat_rbytes_wbytes(&s);
                cgroup_io_r = r;
                cgroup_io_w = w;
            }
        }

        let status = read_proc_file(pid, "status").await;
        let stat = read_proc_file(pid, "stat").await;
        let net_dev = read_proc_file(pid, "net/dev").await;

        let proc_vmrss = status.as_deref().and_then(parse_status_vmrss_kb);
        let proc_ticks = stat.as_deref().and_then(parse_proc_stat_utime_stime_ticks);

        let memory_bytes = cgroup_mem.unwrap_or_else(|| proc_vmrss.map(|k| k * 1024).unwrap_or(0));

        let wall_us = prev
            .map(|p| now.duration_since(p.last_instant).as_micros() as u64)
            .unwrap_or(1)
            .max(1);

        let mut cpu_cores = 0.0_f64;
        if let Some(p) = prev {
            if let (Some(c), Some(pc)) = (cgroup_cpu_usec, p.last_cpu_usage_usec) {
                let du = c.saturating_sub(pc);
                cpu_cores = (du as f64) / (wall_us as f64);
            } else if let (Some(c), Some(pt)) = (proc_ticks, p.last_proc_ticks) {
                let dt = c.saturating_sub(pt);
                let hz = f64::from(clock_ticks_per_sec());
                cpu_cores = ((dt as f64) / hz) / (wall_us as f64 / 1_000_000.0);
            }
        }

        let storage_io_bytes = cgroup_io_r.saturating_add(cgroup_io_w);

        let (net_rx, net_tx) = net_dev
            .as_deref()
            .and_then(parse_proc_net_dev_totals)
            .unwrap_or((0, 0));

        let (network_sent, network_recv) = if let Some(p) = prev {
            if let (Some(lrx), Some(ltx)) = (p.last_net_rx, p.last_net_tx) {
                (net_tx.saturating_sub(ltx), net_rx.saturating_sub(lrx))
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        let proc_readable = status.is_some() && stat.is_some();
        let cgroup_readable = cgroup_dir.is_some();
        let simulated = !cgroup_readable && !proc_readable;

        let sample = DeploymentResourceSample {
            cpu_cores,
            memory_bytes,
            storage_io_bytes,
            network_sent,
            network_recv,
            simulated,
        };

        let new_state = ResourcePollState {
            last_instant: now,
            last_cpu_usage_usec: cgroup_cpu_usec,
            last_proc_ticks: proc_ticks,
            last_net_rx: Some(net_rx),
            last_net_tx: Some(net_tx),
        };

        (sample, new_state)
    }
}

/// One deployment-level resource snapshot (may combine cgroup + `/proc`).
#[derive(Debug, Clone)]
pub(crate) struct DeploymentResourceSample {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_io_bytes: u64,
    pub network_sent: u64,
    pub network_recv: u64,
    pub simulated: bool,
}

impl ResourceMetricsReader {
    async fn discover_cgroup_v2_dir(&self, pid: u32) -> Option<PathBuf> {
        let content = read_proc_file(pid, "cgroup").await?;
        cgroup_v2_relative_path(&content).map(|rel| {
            let rel = rel.as_path().strip_prefix("/").unwrap_or(rel.as_path());
            self.cgroup_root.join(rel)
        })
    }
}

async fn read_file_join(dir: &Path, name: &str) -> Option<String> {
    let path = dir.join(name);
    fs::read_to_string(&path).await.ok()
}

async fn read_proc_file(pid: u32, rest: &str) -> Option<String> {
    let path = PathBuf::from(format!("/proc/{pid}/{rest}"));
    fs::read_to_string(&path).await.ok()
}

/// Linux `USER_HZ` is almost always 100; used for `/proc/[pid]/stat` times.
#[must_use]
pub(crate) fn clock_ticks_per_sec() -> u32 {
    100
}

/// Parse cgroup v2 unified hierarchy line `0::/foo/bar` → `foo/bar`.
pub(crate) fn cgroup_v2_relative_path(cgroup_content: &str) -> Option<PathBuf> {
    for line in cgroup_content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("0::") {
            let p = rest.trim();
            if p.is_empty() {
                return Some(PathBuf::new());
            }
            return Some(PathBuf::from(p.trim_start_matches('/')));
        }
    }
    None
}

/// Parse `memory.current` body (single u64).
pub(crate) fn parse_single_u64_line(s: &str) -> Option<u64> {
    s.trim().parse().ok()
}

/// Parse `memory.max` — `max` means no fixed limit.
fn parse_cgroup_memory_max(s: &str) -> Option<u64> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("max") {
        return None;
    }
    t.parse().ok()
}

/// Sum `usage_usec` from `cpu.stat` (may contain multiple lines / keys).
pub(crate) fn parse_cpu_stat_usage_usec(cpu_stat: &str) -> Option<u64> {
    for line in cpu_stat.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("usage_usec") {
            let num = rest
                .trim()
                .trim_start_matches(|c: char| c == ':' || c.is_whitespace());
            if let Ok(v) = num.parse::<u64>() {
                return Some(v);
            }
        }
    }
    None
}

/// Sum `rbytes=` and `wbytes=` across devices in cgroup `io.stat`.
pub(crate) fn parse_io_stat_rbytes_wbytes(io_stat: &str) -> (u64, u64) {
    let mut r = 0u64;
    let mut w = 0u64;
    for line in io_stat.lines() {
        for part in line.split_whitespace() {
            if let Some(v) = part.strip_prefix("rbytes=") {
                if let Ok(n) = v.parse::<u64>() {
                    r = r.saturating_add(n);
                }
            } else if let Some(v) = part.strip_prefix("wbytes=") {
                if let Ok(n) = v.parse::<u64>() {
                    w = w.saturating_add(n);
                }
            }
        }
    }
    (r, w)
}

/// Parse `VmRSS:` from `/proc/[pid]/status` (value in kB).
pub(crate) fn parse_status_vmrss_kb(status: &str) -> Option<u64> {
    for line in status.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let mut parts = rest.split_whitespace();
            let kb = parts.next()?.parse::<u64>().ok()?;
            return Some(kb);
        }
    }
    None
}

/// Parse utime+stime fields from `/proc/[pid]/stat` (combined scheduler ticks).
pub(crate) fn parse_proc_stat_utime_stime_ticks(stat_line: &str) -> Option<u64> {
    let idx = stat_line.rfind(')')?;
    let after = stat_line.get(idx + 1..)?.trim_start();
    let fields: Vec<&str> = after.split_whitespace().collect();
    // After comm: state ppid ... utime(14) stime(15) — 0-based index 11 and 12 in `fields`.
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

/// Aggregate non-loopback RX/TX byte counters from `/proc/[pid]/net/dev`.
pub(crate) fn parse_proc_net_dev_totals(dev: &str) -> Option<(u64, u64)> {
    let mut rx = 0u64;
    let mut tx = 0u64;
    for line in dev.lines().skip(2) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let colon = line.find(':')?;
        let iface = line.get(..colon)?.trim();
        if iface == "lo" {
            continue;
        }
        let rest = line.get(colon + 1..)?.trim();
        let nums: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if nums.len() > 8 {
            rx = rx.saturating_add(nums[0]);
            tx = tx.saturating_add(nums[8]);
        }
    }
    Some((rx, tx))
}

/// Spec-based estimates (last resort). Mirrors previous BYOB heuristic.
pub(crate) fn simulated_resource_usage(
    services: &HashMap<String, ServiceSpec>,
) -> super::byob_types::ResourceUsage {
    use super::byob_types::{NetworkUsage, ResourceUsage};

    let mut cpu_total = 0.0;
    let mut total_memory = 0u64;
    let mut total_storage = 0u64;
    let mut gpu_total = 0u32;
    let mut total_network_sent = 0u64;
    let mut total_network_received = 0u64;

    for service_spec in services.values() {
        if let Some(cpu_cores) = service_spec.resources.cpu_cores {
            cpu_total += cpu_cores * 0.65;
        }
        if let Some(memory_bytes) = service_spec.resources.memory_bytes {
            total_memory += (memory_bytes * 3) / 4;
        }
        if let Some(storage_bytes) = service_spec.resources.storage_bytes {
            total_storage += (storage_bytes * 2) / 5;
        }
        if let Some(gpu_count) = service_spec.resources.gpu_count {
            gpu_total = gpu_total.saturating_add(gpu_count);
        }
        let base_network_usage = match service_spec.image.as_deref() {
            Some(image) if image.contains("web") || image.contains("api") => 1024 * 1024,
            Some(image) if image.contains("database") => 512 * 1024,
            _ => 256 * 1024,
        };
        total_network_sent += base_network_usage;
        total_network_received += base_network_usage / 2;
    }

    ResourceUsage {
        cpu_usage: cpu_total,
        memory_usage: total_memory,
        storage_usage: total_storage,
        gpu_usage: gpu_total,
        network_usage: NetworkUsage {
            bytes_sent: total_network_sent,
            bytes_received: total_network_received,
            packets_sent: total_network_sent / 1024,
            packets_received: total_network_received / 1024,
        },
    }
}

/// Merge a real sample with GPU totals from specs; optionally replace with full simulation.
pub(crate) fn merge_sample_with_gpu(
    sample: DeploymentResourceSample,
    services: &HashMap<String, ServiceSpec>,
) -> super::byob_types::ResourceUsage {
    use super::byob_types::{NetworkUsage, ResourceUsage};

    if sample.simulated {
        warn!(
            "BYOB resource metrics: using spec-based simulation (cgroup/proc unavailable or empty)"
        );
        return simulated_resource_usage(services);
    }

    let mut gpu_total = 0u32;
    for service_spec in services.values() {
        if let Some(gpu_count) = service_spec.resources.gpu_count {
            gpu_total = gpu_total.saturating_add(gpu_count);
        }
    }

    ResourceUsage {
        cpu_usage: sample.cpu_cores,
        memory_usage: sample.memory_bytes,
        storage_usage: sample.storage_io_bytes,
        gpu_usage: gpu_total,
        network_usage: NetworkUsage {
            bytes_sent: sample.network_sent,
            bytes_received: sample.network_recv,
            packets_sent: sample.network_sent.max(1) / 1024,
            packets_received: sample.network_recv.max(1) / 1024,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgroup_v2_path_from_proc_cgroup() {
        let s = "0::/user.slice/user-1000.slice/session-1.scope\n";
        assert_eq!(
            cgroup_v2_relative_path(s),
            Some(PathBuf::from("user.slice/user-1000.slice/session-1.scope"))
        );
    }

    #[test]
    fn parse_cpu_stat_usage_usec_synthetic() {
        let s = "usage_usec 123456789\nnr_periods 1\n";
        assert_eq!(parse_cpu_stat_usage_usec(s), Some(123_456_789));
    }

    #[test]
    fn parse_io_stat_sums() {
        let s = r"259:0 rbytes=1000 wbytes=2000 rios=1 wios=2
259:1 rbytes=3 wbytes=4
";
        assert_eq!(parse_io_stat_rbytes_wbytes(s), (1003, 2004));
    }

    #[test]
    fn parse_status_vmrss() {
        let s = "Name: bash\nVmRSS:\t  12345 kB\n";
        assert_eq!(parse_status_vmrss_kb(s), Some(12345));
    }

    #[test]
    fn parse_proc_stat_times_synthetic() {
        // pid (comm) rest — utime and stime at indices 11,12 after closing paren
        let line = "1234 (bash) S 1 2 3 4 5 6 7 8 9 10 1000 2000";
        assert_eq!(parse_proc_stat_utime_stime_ticks(line), Some(3000));
    }

    #[test]
    fn parse_net_dev_synthetic() {
        let s = r"Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    eth0: 100 2 0 0 0 0 0 0 50 1 0 0 0 0 0 0
";
        assert_eq!(parse_proc_net_dev_totals(s), Some((100, 50)));
    }

    #[test]
    fn parse_memory_max_synthetic() {
        assert_eq!(super::parse_cgroup_memory_max("max"), None);
        assert_eq!(
            super::parse_cgroup_memory_max("  1048576\n"),
            Some(1_048_576)
        );
    }
}
