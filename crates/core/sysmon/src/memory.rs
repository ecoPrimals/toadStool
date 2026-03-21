// SPDX-License-Identifier: AGPL-3.0-only
//! Memory monitoring via `/proc/meminfo`.

use crate::error::{Result, SysmonError};

/// System memory information (all values in bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryInfo {
    /// Total physical RAM.
    pub total: u64,
    /// Memory available for new allocations (includes reclaimable).
    pub available: u64,
    /// Memory in use (total − available).
    pub used: u64,
    /// Total swap space.
    pub swap_total: u64,
    /// Free swap space.
    pub swap_free: u64,
}

/// Read system memory info from `/proc/meminfo`.
///
/// # Errors
///
/// Returns an error if `/proc/meminfo` cannot be read.
pub fn memory_info() -> Result<MemoryInfo> {
    let content = std::fs::read_to_string("/proc/meminfo")
        .map_err(|e| SysmonError::new("/proc/meminfo", e))?;
    Ok(parse_meminfo(&content))
}

fn parse_meminfo(content: &str) -> MemoryInfo {
    let mut total = 0u64;
    let mut available = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("MemTotal:") {
            total = parse_kb_value(val);
        } else if let Some(val) = line.strip_prefix("MemAvailable:") {
            available = parse_kb_value(val);
        } else if let Some(val) = line.strip_prefix("SwapTotal:") {
            swap_total = parse_kb_value(val);
        } else if let Some(val) = line.strip_prefix("SwapFree:") {
            swap_free = parse_kb_value(val);
        }
    }

    MemoryInfo {
        total,
        available,
        used: total.saturating_sub(available),
        swap_total,
        swap_free,
    }
}

/// Parse a `/proc/meminfo` value line like "  12345 kB" → bytes.
fn parse_kb_value(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
        * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_info_sane() {
        let info = memory_info().unwrap();
        assert!(info.total > 0, "total memory should be > 0");
        assert!(info.available <= info.total, "available <= total");
        assert_eq!(info.used, info.total - info.available);
    }

    #[test]
    fn test_parse_meminfo() {
        let content = "\
MemTotal:       16384000 kB
MemFree:         2048000 kB
MemAvailable:    8192000 kB
SwapTotal:       4096000 kB
SwapFree:        4096000 kB
";
        let info = parse_meminfo(content);
        assert_eq!(info.total, 16_384_000 * 1024);
        assert_eq!(info.available, 8_192_000 * 1024);
        assert_eq!(info.used, (16_384_000 - 8_192_000) * 1024);
        assert_eq!(info.swap_total, 4_096_000 * 1024);
        assert_eq!(info.swap_free, 4_096_000 * 1024);
    }

    #[test]
    fn test_parse_kb_value() {
        assert_eq!(parse_kb_value("  12345 kB"), 12345 * 1024);
        assert_eq!(parse_kb_value("  0 kB"), 0);
        assert_eq!(parse_kb_value(""), 0);
    }

    #[test]
    fn test_parse_kb_value_malformed() {
        assert_eq!(parse_kb_value("abc kB"), 0);
        assert_eq!(parse_kb_value("  xyz  kB"), 0);
        assert_eq!(parse_kb_value("  12345"), 12345 * 1024); // unit optional
    }

    #[test]
    fn test_parse_meminfo_empty() {
        let info = parse_meminfo("");
        assert_eq!(info.total, 0);
        assert_eq!(info.available, 0);
        assert_eq!(info.used, 0);
        assert_eq!(info.swap_total, 0);
        assert_eq!(info.swap_free, 0);
    }

    #[test]
    fn test_parse_meminfo_partial() {
        let content = "MemTotal:       1024 kB\n";
        let info = parse_meminfo(content);
        assert_eq!(info.total, 1024 * 1024);
        assert_eq!(info.available, 0);
        assert_eq!(info.used, 1024 * 1024);
    }

    #[test]
    fn test_parse_meminfo_used_saturating_sub() {
        let content = "\
MemTotal:       1000 kB
MemAvailable:   2000 kB
";
        let info = parse_meminfo(content);
        assert_eq!(info.used, 0); // saturating_sub when available > total
    }
}
