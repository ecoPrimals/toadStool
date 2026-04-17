// SPDX-License-Identifier: AGPL-3.0-or-later
//! IPv4 subnet helpers for BYOB deployment addressing.
//!
//! Gateway and service addresses are derived from [`TeamNetworkConfig::subnet_cidr`] by parsing
//! the CIDR, masking the network address, and allocating the first usable host as the default
//! gateway (typical `.1` on `/24`) and service hosts at `network + INTERNAL_IP_OFFSET + index`.

use std::net::Ipv4Addr;

use toadstool_config::defaults::network::{
    DEFAULT_NETWORK_SUBNET, GATEWAY_FALLBACK_IP, INTERNAL_IP_OFFSET,
};

#[inline]
fn ipv4_to_u32(ip: Ipv4Addr) -> u32 {
    u32::from_be_bytes(ip.octets())
}

#[inline]
fn u32_to_ipv4(n: u32) -> Ipv4Addr {
    Ipv4Addr::from(n)
}

fn parse_ipv4_prefix(cidr: &str) -> Option<(u32, u8)> {
    let (addr_s, plen_s) = cidr.trim().split_once('/')?;
    let addr: Ipv4Addr = addr_s.parse().ok()?;
    let plen: u8 = plen_s.parse().ok()?;
    if plen > 32 {
        return None;
    }
    Some((ipv4_to_u32(addr), plen))
}

fn network_address(ip: u32, prefix_len: u8) -> u32 {
    if prefix_len >= 32 {
        return ip;
    }
    let mask = !0u32 << (32 - prefix_len);
    ip & mask
}

fn broadcast_address(network: u32, prefix_len: u8) -> u32 {
    if prefix_len >= 32 {
        return network;
    }
    network | ((1u32 << (32 - prefix_len)) - 1)
}

fn last_usable_host(network: u32, prefix_len: u8) -> u32 {
    broadcast_address(network, prefix_len).saturating_sub(1)
}

/// First usable host address in the subnet (typically the default gateway).
fn first_usable_host(network: u32, prefix_len: u8) -> Option<u32> {
    let b = broadcast_address(network, prefix_len);
    let candidate = network.checked_add(1)?;
    if candidate < b { Some(candidate) } else { None }
}

/// Compute default gateway IP and internal service IPs from an IPv4 CIDR.
///
/// When `subnet_cidr` is empty or invalid, [`DEFAULT_NETWORK_SUBNET`] is used (see
/// `toadstool_config::defaults::network::DEFAULT_NETWORK_SUBNET`). If computation still fails,
/// returns [`GATEWAY_FALLBACK_IP`] and empty service list (should not occur for defaults).
pub(crate) fn gateway_and_service_ips(
    subnet_cidr: &str,
    service_count: usize,
) -> (String, Vec<String>) {
    let trimmed = subnet_cidr.trim();
    let primary = if trimmed.is_empty() {
        DEFAULT_NETWORK_SUBNET
    } else {
        trimmed
    };

    if let Some((gw, ips)) = compute_layout(primary, service_count) {
        return (gw, ips);
    }

    if primary != DEFAULT_NETWORK_SUBNET {
        if let Some((gw, ips)) = compute_layout(DEFAULT_NETWORK_SUBNET, service_count) {
            return (gw, ips);
        }
    }

    (GATEWAY_FALLBACK_IP.to_string(), Vec::new())
}

fn compute_layout(cidr: &str, service_count: usize) -> Option<(String, Vec<String>)> {
    let (ip, prefix_len) = parse_ipv4_prefix(cidr)?;
    let network = network_address(ip, prefix_len);
    let last = last_usable_host(network, prefix_len);
    let gateway_host = first_usable_host(network, prefix_len)?;

    let mut ips = Vec::with_capacity(service_count);
    for i in 0..service_count {
        let offset = (INTERNAL_IP_OFFSET + i) as u32;
        let preferred = network.checked_add(offset)?;
        let host_ip = if preferred <= last && preferred > network {
            preferred
        } else {
            // Subnet too small for `INTERNAL_IP_OFFSET` layout: pack after the first usable host.
            let packed = gateway_host.checked_add(1 + i as u32)?;
            if packed <= last {
                packed
            } else {
                return None;
            }
        };
        ips.push(u32_to_ipv4(host_ip).to_string());
    }

    Some((u32_to_ipv4(gateway_host).to_string(), ips))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byob_layout_192_168_slash_24() {
        let (gw, ips) = gateway_and_service_ips("192.168.1.0/24", 1);
        assert_eq!(gw, "192.168.1.1");
        assert_eq!(ips, vec!["192.168.1.10"]);
    }

    #[test]
    fn byob_layout_10_slash_24_two_services() {
        let (gw, ips) = gateway_and_service_ips("10.0.0.0/24", 2);
        assert_eq!(gw, "10.0.0.1");
        assert_eq!(ips, vec!["10.0.0.10", "10.0.0.11"]);
    }
}
