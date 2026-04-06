// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parse Intel GPU batch buffer command dumps.
//!
//! Intel GPUs (i915, xe) submit work via batch buffers containing
//! MI_* and `COMPUTE_WALKER` commands. The `intel_gpu_tools` suite
//! can dump these, as can i915 debugfs.
//!
//! Expected format:
//! ```text
//! CMD 0x10000005 DWORDS=6  ; MI_BATCH_BUFFER_START
//! CMD 0x7a000004 DWORDS=5  ; COMPUTE_WALKER
//! ```

use super::{ObserveConfig, ObserveError, ObserveResult, TraceEvent, TraceEventKind};
use std::io::BufRead;

/// Parse Intel batch buffer trace into trace events.
///
/// # Errors
/// Returns `Err` if the trace path is missing, the file cannot be read, or parsing fails.
pub fn parse_batch_trace(config: &ObserveConfig) -> Result<ObserveResult, ObserveError> {
    let path = config.trace_path.as_ref().ok_or_else(|| {
        ObserveError::TraceUnavailable("Intel batch trace requires trace_path".into())
    })?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    let mut seq: u64 = 0;

    for line in reader.lines() {
        let line = line?;
        if let Some(evt) = parse_batch_line(&line, &mut seq) {
            events.push(evt);
        }
    }

    Ok(ObserveResult {
        gpu_id: format!("{:?}", config.gpu_selector),
        driver: "i915-batch".to_string(),
        events,
        compute_triggered: config.trigger_compute,
        duration_us: 0,
    })
}

fn parse_batch_line(line: &str, seq: &mut u64) -> Option<TraceEvent> {
    let line = line.trim();
    if !line.starts_with("CMD") {
        return None;
    }

    let opcode = extract_hex(line, "CMD ")?;
    let dwords = extract_dec(line, "DWORDS=").unwrap_or(1);
    *seq += 1;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Intel batch opcode is 32-bit per spec"
    )]
    let opcode_u32 = opcode as u32;
    Some(TraceEvent {
        timestamp_us: *seq,
        kind: TraceEventKind::BatchCommand {
            opcode: opcode_u32,
            dwords,
        },
        context: line.to_string(),
    })
}

fn extract_hex(line: &str, prefix: &str) -> Option<u64> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let hex_start = rest.find("0x")? + 2;
    let rest = &rest[hex_start..];
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(rest.len());
    u64::from_str_radix(&rest[..end], 16).ok()
}

fn extract_dec(line: &str, prefix: &str) -> Option<u16> {
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
    fn parse_batch_command() {
        let mut seq = 0;
        let evt = parse_batch_line("CMD 0x10000005 DWORDS=6  ; MI_BATCH_BUFFER_START", &mut seq);
        assert!(evt.is_some());
        match evt.unwrap().kind {
            TraceEventKind::BatchCommand { opcode, dwords } => {
                assert_eq!(opcode, 0x1000_0005);
                assert_eq!(dwords, 6);
            }
            _ => unreachable!("expected BatchCommand"),
        }
    }
}
