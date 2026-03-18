// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parse Linux kernel mmiotrace logs into `TraceEvent`s.
//!
//! mmiotrace format (from `/sys/kernel/tracing/trace`):
//! ```text
//! W 4 0.123456 1 0xfee00000 0x00000001 0x00000000 0x0
//! R 4 0.123460 1 0xfee00004 0x00000042 0x0
//! ```
//! W/R = write/read, width, timestamp, PID, address, value, PC

use super::{ObserveConfig, ObserveError, ObserveResult, TraceEvent, TraceEventKind};
use std::io::BufRead;

/// Parse an mmiotrace log file into trace events.
///
/// # Errors
/// Returns `Err` if the trace path is missing, the file cannot be read, or parsing fails.
pub fn parse_mmiotrace(config: &ObserveConfig) -> Result<ObserveResult, ObserveError> {
    let path = config
        .trace_path
        .as_ref()
        .ok_or_else(|| ObserveError::TraceUnavailable("mmiotrace requires trace_path".into()))?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    let mut first_ts: Option<f64> = None;
    let mut last_ts: f64 = 0.0;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(evt) = parse_mmio_line(line, &mut first_ts) {
            last_ts = first_ts.unwrap_or(0.0);
            events.push(evt);
        }
    }

    let base_ts = first_ts.unwrap_or(0.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "trace duration in seconds * 1e6 fits in u64"
    )]
    let duration_us = ((last_ts - base_ts) * 1_000_000.0) as u64;

    Ok(ObserveResult {
        gpu_id: format!("{:?}", config.gpu_selector),
        driver: "mmiotrace".to_string(),
        events,
        compute_triggered: config.trigger_compute,
        duration_us,
    })
}

fn parse_mmio_line(line: &str, first_ts: &mut Option<f64>) -> Option<TraceEvent> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    let is_write = match parts[0] {
        "W" => true,
        "R" => false,
        _ => return None,
    };

    let width: u8 = parts[1].parse().ok()?;
    let timestamp: f64 = parts[2].parse().ok()?;
    let offset = u64::from_str_radix(parts[4].trim_start_matches("0x"), 16).ok()?;
    let value = u64::from_str_radix(parts[5].trim_start_matches("0x"), 16).ok()?;

    let base = *first_ts.get_or_insert(timestamp);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "relative timestamp in seconds * 1e6 fits in u64"
    )]
    let timestamp_us = ((timestamp - base) * 1_000_000.0) as u64;

    let kind = if is_write {
        TraceEventKind::RegisterWrite {
            offset,
            value,
            width,
        }
    } else {
        TraceEventKind::RegisterRead {
            offset,
            value,
            width,
        }
    };

    Some(TraceEvent {
        timestamp_us,
        kind,
        context: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_write_line() {
        let mut first_ts = None;
        let evt = parse_mmio_line("W 4 1.000000 1 0xfee00000 0x00000001 0x0", &mut first_ts);
        assert!(evt.is_some());
        let evt = evt.unwrap();
        assert_eq!(evt.timestamp_us, 0);
        match evt.kind {
            TraceEventKind::RegisterWrite {
                offset,
                value,
                width,
            } => {
                assert_eq!(offset, 0xfee0_0000);
                assert_eq!(value, 1);
                assert_eq!(width, 4);
            }
            _ => panic!("expected RegisterWrite"),
        }
    }

    #[test]
    fn parse_read_line() {
        let mut first_ts = None;
        let evt = parse_mmio_line("R 4 2.000000 1 0xfee00004 0x00000042 0x0", &mut first_ts);
        assert!(evt.is_some());
        match evt.unwrap().kind {
            TraceEventKind::RegisterRead { offset, value, .. } => {
                assert_eq!(offset, 0xfee0_0004);
                assert_eq!(value, 0x42);
            }
            _ => panic!("expected RegisterRead"),
        }
    }

    #[test]
    fn skip_comment_and_empty() {
        let mut first_ts = None;
        assert!(parse_mmio_line("# comment", &mut first_ts).is_none());
        assert!(parse_mmio_line("", &mut first_ts).is_none());
    }
}
