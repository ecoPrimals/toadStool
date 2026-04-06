// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parse AMD PM4 command stream dumps.
//!
//! AMD GPUs use PM4 (Packet Manager 4) format for command submission.
//! The kernel `amdgpu` driver can dump these via debugfs or umr.
//!
//! PM4 packets have a 1-dword header encoding type, opcode, and count:
//! - Type 0: register writes (base register + count)
//! - Type 2: NOP (filler)
//! - Type 3: command packets (opcode + data)

use super::{ObserveConfig, ObserveError, ObserveResult, TraceEvent, TraceEventKind};
use std::io::BufRead;

/// Parse an AMD PM4 trace file into trace events.
///
/// # Errors
/// Returns `Err` if the trace path is missing, the file cannot be read, or parsing fails.
pub fn parse_pm4_trace(config: &ObserveConfig) -> Result<ObserveResult, ObserveError> {
    let path = config
        .trace_path
        .as_ref()
        .ok_or_else(|| ObserveError::TraceUnavailable("PM4 trace requires trace_path".into()))?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    let mut seq: u64 = 0;

    for line in reader.lines() {
        let line = line?;
        if let Some(evt) = parse_pm4_line(&line, &mut seq) {
            events.push(evt);
        }
    }

    Ok(ObserveResult {
        gpu_id: format!("{:?}", config.gpu_selector),
        driver: "amdgpu-pm4".to_string(),
        events,
        compute_triggered: config.trigger_compute,
        duration_us: 0,
    })
}

/// Parse a single PM4 dump line.
///
/// Expected format (from umr or custom dumper):
/// ```text
/// PKT3 OP=0x28 COUNT=2  ; SET_SH_REG
/// PKT3 OP=0x76 COUNT=5  ; DISPATCH_DIRECT
/// PKT0 BASE=0x2000 COUNT=1
/// ```
fn parse_pm4_line(line: &str, seq: &mut u64) -> Option<TraceEvent> {
    let line = line.trim();

    if line.starts_with("PKT3") {
        let opcode = extract_hex_field(line, "OP=")?;
        let count = extract_dec_field(line, "COUNT=").unwrap_or(0);
        *seq += 1;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "PM4 opcode is 16-bit per AMD spec"
        )]
        let opcode_u16 = opcode as u16;
        Some(TraceEvent {
            timestamp_us: *seq,
            kind: TraceEventKind::Pm4Packet {
                opcode: opcode_u16,
                count,
            },
            context: line.to_string(),
        })
    } else if line.starts_with("PKT0") {
        let base = extract_hex_field(line, "BASE=")?;
        let count = extract_dec_field(line, "COUNT=").unwrap_or(1);
        *seq += 1;
        Some(TraceEvent {
            timestamp_us: *seq,
            kind: TraceEventKind::RegisterWrite {
                offset: base,
                value: 0,
                width: 4,
            },
            context: format!("PM4 type0 reg write, count={count}"),
        })
    } else {
        None
    }
}

fn extract_hex_field(line: &str, prefix: &str) -> Option<u64> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit() && c != 'x')
        .unwrap_or(rest.len());
    let val = &rest[..end];
    val.strip_prefix("0x").map_or_else(
        || u64::from_str_radix(val, 16).ok(),
        |hex| u64::from_str_radix(hex, 16).ok(),
    )
}

fn extract_dec_field(line: &str, prefix: &str) -> Option<u16> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pkt3() {
        let mut seq = 0;
        let evt = parse_pm4_line("PKT3 OP=0x28 COUNT=2  ; SET_SH_REG", &mut seq);
        assert!(evt.is_some());
        match evt.unwrap().kind {
            TraceEventKind::Pm4Packet { opcode, count } => {
                assert_eq!(opcode, 0x28);
                assert_eq!(count, 2);
            }
            _ => unreachable!("expected Pm4Packet"),
        }
    }

    #[test]
    fn parse_pkt0() {
        let mut seq = 0;
        let evt = parse_pm4_line("PKT0 BASE=0x2000 COUNT=1", &mut seq);
        assert!(evt.is_some());
        match evt.unwrap().kind {
            TraceEventKind::RegisterWrite { offset, .. } => {
                assert_eq!(offset, 0x2000);
            }
            _ => unreachable!("expected RegisterWrite"),
        }
    }
}
