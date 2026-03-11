// SPDX-License-Identifier: AGPL-3.0-only
//! Parse nouveau GSP RPC debug messages from dmesg.
//!
//! When nouveau loads GSP firmware, it logs RPC exchanges:
//! ```text
//! [  123.456] nouveau 0000:01:00.0: gsp: rpc fn=0x00000001 sz=64
//! [  123.460] nouveau 0000:01:00.0: gsp: rpc reply fn=0x00000001 sz=32
//! ```

use super::{
    ObserveConfig, ObserveError, ObserveResult, RpcDirection, TraceEvent, TraceEventKind,
};
use std::io::BufRead;

pub fn parse_gsp_rpc(config: &ObserveConfig) -> Result<ObserveResult, ObserveError> {
    let path = config.trace_path.as_ref().ok_or_else(|| {
        ObserveError::TraceUnavailable("GSP RPC trace requires trace_path (dmesg log)".into())
    })?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(evt) = parse_gsp_line(&line) {
            events.push(evt);
        }
    }

    Ok(ObserveResult {
        gpu_id: format!("{:?}", config.gpu_selector),
        driver: "nouveau-gsp".to_string(),
        events,
        compute_triggered: config.trigger_compute,
        duration_us: 0,
    })
}

fn parse_gsp_line(line: &str) -> Option<TraceEvent> {
    if !line.contains("gsp:") || !line.contains("rpc") {
        return None;
    }

    let direction = if line.contains("reply") {
        RpcDirection::GspToHost
    } else {
        RpcDirection::HostToGsp
    };

    let func_id = extract_field_u32(line, "fn=")?;
    let payload_size = extract_field_u32(line, "sz=").unwrap_or(0);
    let timestamp_us = extract_dmesg_timestamp_us(line).unwrap_or(0);

    Some(TraceEvent {
        timestamp_us,
        kind: TraceEventKind::GspRpc {
            func_id,
            payload_size,
            direction,
        },
        context: line.to_string(),
    })
}

fn extract_field_u32(line: &str, prefix: &str) -> Option<u32> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find(|c: char| !c.is_ascii_hexdigit() && c != 'x')
        .unwrap_or(rest.len());
    let val_str = &rest[..end];
    if let Some(hex) = val_str.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok()
    } else {
        val_str.parse().ok()
    }
}

fn extract_dmesg_timestamp_us(line: &str) -> Option<u64> {
    let start = line.find('[')? + 1;
    let end = line.find(']')?;
    let ts_str = line[start..end].trim();
    let secs: f64 = ts_str.parse().ok()?;
    Some((secs * 1_000_000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rpc_request() {
        let line = "[  123.456789] nouveau 0000:01:00.0: gsp: rpc fn=0x00000001 sz=64";
        let evt = parse_gsp_line(line);
        assert!(evt.is_some());
        let evt = evt.unwrap();
        assert_eq!(evt.timestamp_us, 123456789);
        match evt.kind {
            TraceEventKind::GspRpc { func_id, payload_size, direction } => {
                assert_eq!(func_id, 1);
                assert_eq!(payload_size, 64);
                assert!(matches!(direction, RpcDirection::HostToGsp));
            }
            _ => panic!("expected GspRpc"),
        }
    }

    #[test]
    fn parse_rpc_reply() {
        let line = "[  123.460000] nouveau 0000:01:00.0: gsp: rpc reply fn=0x00000002 sz=32";
        let evt = parse_gsp_line(line);
        assert!(evt.is_some());
        match evt.unwrap().kind {
            TraceEventKind::GspRpc { func_id, direction, .. } => {
                assert_eq!(func_id, 2);
                assert!(matches!(direction, RpcDirection::GspToHost));
            }
            _ => panic!("expected GspRpc reply"),
        }
    }

    #[test]
    fn skip_non_gsp_lines() {
        assert!(parse_gsp_line("[  1.0] nouveau: loading firmware").is_none());
        assert!(parse_gsp_line("unrelated line").is_none());
    }
}
