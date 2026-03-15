// SPDX-License-Identifier: AGPL-3.0-only
//! Parse strace DRM ioctl output into `TraceEvent`s.
//!
//! Expected strace format (from `strace -e trace=ioctl -y`):
//! ```text
//! ioctl(3</dev/dri/card0>, 0xc0106400, 0x7fff...) = 0
//! ioctl(3</dev/dri/card0>, 0xc0106401, 0x7fff...) = -1 EINVAL
//! ```

use super::{ObserveConfig, ObserveError, ObserveResult, TraceEvent, TraceEventKind};
use std::io::BufRead;

/// Parse strace ioctl output into trace events.
///
/// # Errors
/// Returns `Err` if the trace path is missing, the file cannot be read, or parsing fails.
pub fn parse_ioctl_trace(config: &ObserveConfig) -> Result<ObserveResult, ObserveError> {
    let path = config
        .trace_path
        .as_ref()
        .ok_or_else(|| ObserveError::TraceUnavailable("ioctl trace requires trace_path".into()))?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    let mut seq: u64 = 0;

    for line in reader.lines() {
        let line = line?;
        if let Some(evt) = parse_ioctl_line(&line, &mut seq) {
            events.push(evt);
        }
    }

    Ok(ObserveResult {
        gpu_id: format!("{:?}", config.gpu_selector),
        driver: "strace".to_string(),
        events,
        compute_triggered: config.trigger_compute,
        duration_us: 0,
    })
}

fn parse_ioctl_line(line: &str, seq: &mut u64) -> Option<TraceEvent> {
    if !line.contains("ioctl(") {
        return None;
    }

    let ioctl_nr = extract_hex_after(line, "ioctl(", ",")?;
    let success = line.contains("= 0") && !line.contains("= 0x");

    // Approximate arg size from ioctl number encoding: bits 16..29 = size
    let arg_size = ((ioctl_nr >> 16) & 0x3FFF) as u32;

    *seq += 1;

    Some(TraceEvent {
        timestamp_us: *seq,
        kind: TraceEventKind::IoctlCall {
            ioctl_nr,
            arg_size,
            success,
        },
        context: line.to_string(),
    })
}

fn extract_hex_after(s: &str, prefix: &str, _terminator: &str) -> Option<u64> {
    let start = s.find(prefix)? + prefix.len();
    let rest = &s[start..];
    // Skip fd and path info to find the hex ioctl number
    let hex_start = rest.find("0x")?;
    let rest = &rest[hex_start + 2..];
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(rest.len());
    u64::from_str_radix(&rest[..end], 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_successful_ioctl() {
        let mut seq = 0;
        let evt = parse_ioctl_line(
            "ioctl(3</dev/dri/card0>, 0xc0106400, 0x7fff1234) = 0",
            &mut seq,
        );
        assert!(evt.is_some());
        let evt = evt.unwrap();
        match &evt.kind {
            TraceEventKind::IoctlCall { success, .. } => assert!(success),
            _ => panic!("expected IoctlCall"),
        }
    }

    #[test]
    fn parse_failed_ioctl() {
        let mut seq = 0;
        let evt = parse_ioctl_line(
            "ioctl(3</dev/dri/card0>, 0xc0106400, 0x7fff) = -1 EINVAL",
            &mut seq,
        );
        assert!(evt.is_some());
        match &evt.unwrap().kind {
            TraceEventKind::IoctlCall { success, .. } => assert!(!success),
            _ => panic!("expected IoctlCall"),
        }
    }
}
