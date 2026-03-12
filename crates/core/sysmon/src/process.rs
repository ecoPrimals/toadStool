// SPDX-License-Identifier: AGPL-3.0-only
//! Process monitoring via `/proc/[pid]/stat` and `/proc/[pid]/status`.

use crate::error::{Result, SysmonError};

const CLK_TCK: u64 = 100; // Standard on Linux for all mainstream architectures

/// Information about a running process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub start_time: u64,
}

/// Look up a single process by PID.
///
/// # Errors
///
/// Returns an error if `/proc` cannot be read (other than the target PID
/// not existing or being inaccessible).
pub fn process_info(pid: u32) -> Result<Option<ProcessInfo>> {
    let stat_path = format!("/proc/{pid}/stat");
    let stat_content = match std::fs::read_to_string(&stat_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(None),
        Err(e) => return Err(SysmonError::new("/proc/[pid]/stat", e)),
    };

    let status_path = format!("/proc/{pid}/status");
    let rss_bytes = std::fs::read_to_string(&status_path).map_or(0, |c| parse_vm_rss(&c));

    let boot_time = boot_time_secs()?;
    Ok(parse_proc_stat(&stat_content, rss_bytes, boot_time))
}

/// Count of running processes (fast: just counts `/proc/[0-9]+` dirs).
///
/// # Errors
///
/// Returns an error if `/proc` cannot be read.
pub fn process_count() -> Result<usize> {
    let entries = std::fs::read_dir("/proc").map_err(|e| SysmonError::new("/proc", e))?;
    let count = entries
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|name| name.bytes().all(|b| b.is_ascii_digit()))
        })
        .count();
    Ok(count)
}

/// Enumerate all processes with basic info.
///
/// Skips processes that vanish between directory listing and stat reading.
///
/// # Errors
///
/// Returns an error if `/proc` cannot be read.
pub fn all_processes() -> Result<Vec<ProcessInfo>> {
    let entries = std::fs::read_dir("/proc").map_err(|e| SysmonError::new("/proc", e))?;
    let boot_time = boot_time_secs()?;
    let mut procs = Vec::new();

    for entry in entries.filter_map(std::result::Result::ok) {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !name_str.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let stat_path = format!("/proc/{pid}/stat");
        let Ok(stat_content) = std::fs::read_to_string(&stat_path) else {
            continue;
        };

        let status_path = format!("/proc/{pid}/status");
        let rss_bytes = std::fs::read_to_string(&status_path)
            .map(|c| parse_vm_rss(&c))
            .unwrap_or(0);

        if let Some(info) = parse_proc_stat(&stat_content, rss_bytes, boot_time) {
            procs.push(info);
        }
    }

    Ok(procs)
}

/// Parse `/proc/[pid]/stat`. The comm field (field 2) is in parens and may
/// contain spaces or parens, so we find the *last* `)` to delimit it.
fn parse_proc_stat(content: &str, rss_bytes: u64, boot_time: u64) -> Option<ProcessInfo> {
    let open = content.find('(')?;
    let close = content.rfind(')')?;
    let pid_str = content[..open].trim();
    let pid: u32 = pid_str.parse().ok()?;
    let name = content[open + 1..close].to_string();

    // Fields after the closing paren (field 3 onwards, 1-indexed from field 1=pid)
    let rest = &content[close + 2..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // field 14 = utime (index 11 in rest), field 15 = stime (index 12)
    // field 22 = starttime (index 19 in rest)
    if fields.len() < 20 {
        return None;
    }

    let utime: u64 = fields[11].parse().unwrap_or(0);
    let stime: u64 = fields[12].parse().unwrap_or(0);
    let starttime: u64 = fields[19].parse().unwrap_or(0);

    let total_cpu_ticks = utime + stime;
    #[allow(
        clippy::cast_precision_loss,
        reason = "tick counts fit f64 without meaningful precision loss"
    )]
    let cpu_seconds = total_cpu_ticks as f64 / CLK_TCK as f64;

    let start_epoch = boot_time + starttime / CLK_TCK;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let alive_secs = now.saturating_sub(start_epoch);

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "CPU percentage 0-100 fits f32; seconds fit f64"
    )]
    let cpu_usage = if alive_secs > 0 {
        (cpu_seconds / alive_secs as f64 * 100.0) as f32
    } else {
        0.0
    };

    Some(ProcessInfo {
        pid,
        name,
        cpu_usage,
        memory: rss_bytes,
        start_time: start_epoch,
    })
}

/// Extract `VmRSS` from `/proc/[pid]/status` (in bytes).
fn parse_vm_rss(content: &str) -> u64 {
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("VmRSS:") {
            return val
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024; // kB → bytes
        }
    }
    0
}

/// Boot time in seconds since epoch, from `/proc/stat` `btime` line.
fn boot_time_secs() -> Result<u64> {
    let content =
        std::fs::read_to_string("/proc/stat").map_err(|e| SysmonError::new("/proc/stat", e))?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("btime ") {
            return Ok(val.trim().parse().unwrap_or(0));
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_count_positive() {
        let count = process_count().unwrap();
        assert!(count > 0, "should have at least 1 process");
    }

    #[test]
    fn test_self_process() {
        let pid = std::process::id();
        let info = process_info(pid).unwrap();
        assert!(info.is_some(), "should find our own process");
        let info = info.unwrap();
        assert_eq!(info.pid, pid);
        assert!(!info.name.is_empty());
        assert!(info.memory > 0);
        assert!(info.start_time > 0);
    }

    #[test]
    fn test_nonexistent_process() {
        let info = process_info(u32::MAX).unwrap();
        assert!(info.is_none());
    }

    #[test]
    fn test_parse_vm_rss() {
        let content = "\
Name:   test
VmPeak: 12345 kB
VmSize: 10000 kB
VmRSS:  5000 kB
Threads: 1
";
        assert_eq!(parse_vm_rss(content), 5_000 * 1024);
    }

    #[test]
    fn test_all_processes_includes_self() {
        let procs = all_processes().unwrap();
        let self_pid = std::process::id();
        assert!(
            procs.iter().any(|p| p.pid == self_pid),
            "should include our own process"
        );
    }

    #[test]
    fn test_parse_proc_stat_basic() {
        // Format: pid (comm) state ppid ... utime stime ... starttime
        // Fields 1-13 before utime, so utime=field 14 (index 11), stime=15 (12), starttime=22 (19)
        let content = "12345 (test_process) R 1 12345 12345 0 -1 4194560 100 0 0 0 500 200 0 0 20 0 1 0 100000 0 0 0 0 0 0 0 0 0 0 0 0 0 0";
        let info = parse_proc_stat(content, 4096 * 1024, 1_700_000_000); // rss 4MB, boot ~2023
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.pid, 12345);
        assert_eq!(info.name, "test_process");
        assert_eq!(info.memory, 4096 * 1024);
    }

    #[test]
    fn test_parse_proc_stat_comm_with_spaces() {
        let content = "999 (process with spaces) R 1 999 999 0 -1 0 0 0 0 0 10 5 0 0 20 0 1 0 50000 0 0 0 0 0 0 0 0 0 0 0 0 0 0";
        let info = parse_proc_stat(content, 0, 1_700_000_000);
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "process with spaces");
    }

    #[test]
    fn test_parse_proc_stat_too_few_fields() {
        // Need at least 20 fields after ); we provide 19
        let content = "1 (init) S 0 1 1 0 -1 0 0 0 0 0 1 2 3 4 5 6 7 8";
        let info = parse_proc_stat(content, 0, 1_700_000_000);
        assert!(info.is_none());
    }

    #[test]
    fn test_parse_proc_stat_no_parens() {
        let content = "12345 invalid R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20";
        let info = parse_proc_stat(content, 0, 1_700_000_000);
        assert!(info.is_none());
    }

    #[test]
    fn test_parse_vm_rss_empty() {
        assert_eq!(parse_vm_rss(""), 0);
    }

    #[test]
    fn test_parse_vm_rss_no_vmrss() {
        let content = "Name: test\nVmSize: 1000 kB\n";
        assert_eq!(parse_vm_rss(content), 0);
    }

    #[test]
    fn test_parse_vm_rss_malformed() {
        let content = "VmRSS: abc kB\n";
        assert_eq!(parse_vm_rss(content), 0);
    }

    #[test]
    fn test_parse_vm_rss_zero() {
        let content = "VmRSS:\t0 kB\n";
        assert_eq!(parse_vm_rss(content), 0);
    }
}
