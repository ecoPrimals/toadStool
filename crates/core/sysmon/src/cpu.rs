// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU monitoring via `/proc/stat` and `/proc/cpuinfo`.

use crate::error::{Result, SysmonError};
use std::thread;
use std::time::Duration;

/// Number of logical CPU cores.
///
/// Uses `std::thread::available_parallelism` (no FFI).
#[must_use]
pub fn cpu_count() -> usize {
    thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1)
}

/// CPU brand/model string from `/proc/cpuinfo`.
///
/// # Errors
///
/// Returns an error if `/proc/cpuinfo` cannot be read.
pub fn cpu_brand() -> Result<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo")
        .map_err(|e| SysmonError::new("/proc/cpuinfo", e))?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("model name") {
            if let Some(brand) = value.trim_start().strip_prefix(':') {
                return Ok(brand.trim().to_string());
            }
        }
    }
    Ok(String::from("Unknown CPU"))
}

/// Snapshot of `/proc/stat` CPU time counters.
#[derive(Debug, Clone)]
struct CpuTimes {
    /// Per-CPU times: (user+nice+system+irq+softirq+steal, idle+iowait)
    per_cpu: Vec<(u64, u64)>,
}

impl CpuTimes {
    fn read() -> Result<Self> {
        let content =
            std::fs::read_to_string("/proc/stat").map_err(|e| SysmonError::new("/proc/stat", e))?;
        let mut per_cpu = Vec::new();
        for line in content.lines() {
            // Per-cpu lines: "cpu0 user nice system idle iowait irq softirq steal ..."
            if line.starts_with("cpu") && line.as_bytes().get(3).is_some_and(u8::is_ascii_digit) {
                let (busy, idle) = parse_cpu_line(line);
                per_cpu.push((busy, idle));
            }
        }
        Ok(Self { per_cpu })
    }

    fn global_usage_since(&self, prev: &Self) -> f32 {
        let (total_busy, total_idle) =
            self.per_cpu
                .iter()
                .zip(&prev.per_cpu)
                .fold((0u64, 0u64), |(b, i), (cur, prv)| {
                    (
                        b + cur.0.saturating_sub(prv.0),
                        i + cur.1.saturating_sub(prv.1),
                    )
                });
        let total = total_busy + total_idle;
        if total == 0 {
            return 0.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )]
        let pct = (total_busy as f64 / total as f64) * 100.0;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "truncation acceptable for this conversion"
        )]
        let result = pct as f32;
        result
    }

    fn per_cpu_usage_since(&self, prev: &Self) -> Vec<f32> {
        self.per_cpu
            .iter()
            .zip(&prev.per_cpu)
            .map(|(cur, prv)| {
                let busy = cur.0.saturating_sub(prv.0);
                let idle = cur.1.saturating_sub(prv.1);
                let total = busy + idle;
                if total == 0 {
                    0.0
                } else {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "precision loss acceptable for this conversion"
                    )]
                    let pct = (busy as f64 / total as f64) * 100.0;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "truncation acceptable for this conversion"
                    )]
                    let result = pct as f32;
                    result
                }
            })
            .collect()
    }
}

fn parse_cpu_line(line: &str) -> (u64, u64) {
    let mut fields = line.split_whitespace().skip(1); // skip "cpuN"
    let mut next = || {
        fields
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    };
    let user = next();
    let nice = next();
    let system = next();
    let idle = next();
    let iowait = next();
    let irq = next();
    let softirq = next();
    let steal = next();
    let busy = user + nice + system + irq + softirq + steal;
    let idle_total = idle + iowait;
    (busy, idle_total)
}

/// Global CPU usage percentage (0–100) measured over `sample` duration.
///
/// Takes two `/proc/stat` snapshots separated by `sample` and computes the
/// delta. A sample of 200 ms gives a reasonable reading for dashboards.
///
/// # Errors
///
/// Returns an error if `/proc/stat` cannot be read.
pub fn cpu_usage(sample: Duration) -> Result<f32> {
    let t0 = CpuTimes::read()?;
    thread::sleep(sample);
    let t1 = CpuTimes::read()?;
    Ok(t1.global_usage_since(&t0))
}

/// Per-CPU usage percentages measured over `sample` duration.
///
/// # Errors
///
/// Returns an error if `/proc/stat` cannot be read.
pub fn per_cpu_usage(sample: Duration) -> Result<Vec<f32>> {
    let t0 = CpuTimes::read()?;
    thread::sleep(sample);
    let t1 = CpuTimes::read()?;
    Ok(t1.per_cpu_usage_since(&t0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_count_positive() {
        assert!(cpu_count() >= 1);
    }

    #[test]
    fn test_cpu_brand_non_empty() {
        let brand = cpu_brand().unwrap();
        assert!(!brand.is_empty());
    }

    #[test]
    fn test_cpu_usage_in_range() {
        let usage = cpu_usage(Duration::from_millis(50)).unwrap();
        assert!((0.0..=100.0).contains(&usage));
    }

    #[test]
    fn test_per_cpu_count_matches() {
        let usages = per_cpu_usage(Duration::from_millis(50)).unwrap();
        assert_eq!(usages.len(), cpu_count());
    }

    #[test]
    fn test_parse_cpu_line() {
        let (busy, idle) = parse_cpu_line("cpu0 1000 200 300 4000 100 50 25 10 0 0");
        assert_eq!(busy, 1000 + 200 + 300 + 50 + 25 + 10);
        assert_eq!(idle, 4000 + 100);
    }

    #[test]
    fn test_parse_cpu_line_minimal_fields() {
        // Only user, nice, system, idle - rest default to 0
        let (busy, idle) = parse_cpu_line("cpu1 10 0 5 100");
        assert_eq!(busy, 15); // user(10) + nice(0) + system(5), irq/softirq/steal = 0
        assert_eq!(idle, 100); // idle only, iowait = 0
    }

    #[test]
    fn test_parse_cpu_line_empty_after_prefix() {
        let (busy, idle) = parse_cpu_line("cpu2");
        assert_eq!(busy, 0);
        assert_eq!(idle, 0);
    }

    #[test]
    fn test_parse_cpu_line_malformed_numbers() {
        // Non-numeric fields parse as 0 via unwrap_or(0)
        let (busy, idle) = parse_cpu_line("cpu3 abc def xyz 1000 100 50 25 10 0 0");
        // user=0, nice=0, system=0, idle=1000, iowait=100, irq=50, softirq=25, steal=10
        assert_eq!(busy, 50 + 25 + 10); // irq + softirq + steal
        assert_eq!(idle, 1000 + 100);
    }

    #[test]
    fn test_parse_cpu_line_extra_fields_ignored() {
        let (busy, idle) = parse_cpu_line("cpu4 1 2 3 4 5 6 7 8 9 10 11 12");
        assert_eq!(busy, 1 + 2 + 3 + 6 + 7 + 8);
        assert_eq!(idle, 4 + 5);
    }

    #[test]
    fn test_cpu_brand_contains_sane_chars() {
        let brand = cpu_brand().unwrap();
        assert!(!brand.is_empty());
        assert!(brand.chars().all(|c| c.is_ascii() || !c.is_control()));
    }

    #[test]
    fn test_per_cpu_usage_in_range() {
        let usages = per_cpu_usage(Duration::from_millis(50)).unwrap();
        for (i, &u) in usages.iter().enumerate() {
            assert!((0.0..=100.0).contains(&u), "CPU {i} usage {u} out of range",);
        }
    }
}
