// SPDX-License-Identifier: AGPL-3.0-only
//! Network monitoring via `/proc/net/dev`.

use crate::error::{Result, SysmonError};

/// Per-interface network statistics.
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub received: u64,
    pub transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
}

/// Read network interface statistics from `/proc/net/dev`.
///
/// Returns all non-loopback interfaces. The counters are cumulative since boot.
///
/// # Errors
///
/// Returns an error if `/proc/net/dev` cannot be read.
pub fn network_stats() -> Result<Vec<NetworkInterface>> {
    let content = std::fs::read_to_string("/proc/net/dev")
        .map_err(|e| SysmonError::new("/proc/net/dev", e))?;
    Ok(parse_net_dev(&content))
}

fn parse_net_dev(content: &str) -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    // /proc/net/dev format:
    // Inter-|   Receive                                      |  Transmit
    //  face |bytes packets errs drop fifo frame compressed multicast|bytes packets ...
    //   lo: 12345  100  0  0  0  0  0  0  67890  200  0  0  0  0  0  0
    for line in content.lines().skip(2) {
        let Some((name_part, stats_part)) = line.split_once(':') else {
            continue;
        };
        let name = name_part.trim();
        if name == "lo" {
            continue;
        }
        let vals: Vec<u64> = stats_part
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        // Fields: rx_bytes rx_packets rx_errs rx_drop ... tx_bytes tx_packets ...
        if vals.len() >= 10 {
            interfaces.push(NetworkInterface {
                name: name.to_string(),
                received: vals[0],
                packets_received: vals[1],
                transmitted: vals[8],
                packets_transmitted: vals[9],
            });
        }
    }
    interfaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_net_dev() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1234567   10000    0    0    0     0          0         0  1234567   10000    0    0    0     0       0          0
  eth0: 9876543   50000    0    0    0     0          0         0  1111111   20000    0    0    0     0       0          0
wlan0:  5555555   30000    1    0    0     0          0         0   444444   15000    0    0    0     0       0          0
";
        let ifaces = parse_net_dev(content);
        assert_eq!(ifaces.len(), 2, "lo should be excluded");
        assert_eq!(ifaces[0].name, "eth0");
        assert_eq!(ifaces[0].received, 9_876_543);
        assert_eq!(ifaces[0].transmitted, 1_111_111);
        assert_eq!(ifaces[0].packets_received, 50_000);
        assert_eq!(ifaces[0].packets_transmitted, 20_000);
        assert_eq!(ifaces[1].name, "wlan0");
    }

    #[test]
    fn test_network_stats_runs() {
        // Just verify it doesn't error; not all systems have non-lo interfaces
        let _stats = network_stats().unwrap();
    }

    #[test]
    fn test_parse_net_dev_empty() {
        let ifaces = parse_net_dev("");
        assert!(ifaces.is_empty());
    }

    #[test]
    fn test_parse_net_dev_header_only() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
";
        let ifaces = parse_net_dev(content);
        assert!(ifaces.is_empty());
    }

    #[test]
    fn test_parse_net_dev_lo_excluded() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1234567   10000    0    0    0     0          0         0  1234567   10000    0    0    0     0       0          0
";
        let ifaces = parse_net_dev(content);
        assert!(ifaces.is_empty(), "lo should be excluded");
    }

    #[test]
    fn test_parse_net_dev_insufficient_fields() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0: 1 2 3 4 5 6 7 8 9
";
        let ifaces = parse_net_dev(content);
        assert!(ifaces.is_empty(), "need at least 10 numeric fields");
    }

    #[test]
    fn test_parse_net_dev_no_colon_skipped() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  invalid_line_no_colon
";
        let ifaces = parse_net_dev(content);
        assert!(ifaces.is_empty());
    }

    #[test]
    fn test_parse_net_dev_malformed_numbers() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0: abc def ghi jkl mno pqr stu vwx 1000000 2000 0 0 0 0 0 0
";
        let ifaces = parse_net_dev(content);
        // filter_map(|s| s.parse().ok()) skips non-numeric, so we may get fewer than 10 vals
        assert!(ifaces.is_empty() || ifaces[0].received == 0);
    }
}
