// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[tokio::test]
async fn test_service_discovery_creation() {
    let discovery = ServiceDiscovery::new();
    let default_discovery = ServiceDiscovery::default();
    // Both should work; discover returns None for unknown capability
    let result = discovery.discover_by_capability("test", "unknown").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
    let _ = default_discovery;
}

#[tokio::test]
async fn test_mdns_query_packet_format() {
    let discovery = ServiceDiscovery::new();
    let query = discovery.build_mdns_query("test-service");

    assert!(query.len() >= 12);
    assert_eq!(query[0], 0x00);
    assert_eq!(query[1], 0x00);
    assert_eq!(query[4], 0x00);
    assert_eq!(query[5], 0x01);
}

#[tokio::test]
async fn test_discovery_handles_timeout() {
    let discovery = ServiceDiscovery::new();
    let result = discovery
        .discover_by_capability("nonexistent_z9q8x7w6v5u4", "nonexistent_z9q8x7w6v5u4")
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_parse_mdns_response_too_short() {
    let discovery = ServiceDiscovery::new();
    let result = discovery.parse_mdns_response(&[0u8; 8], "test");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_parse_mdns_response_query_not_response() {
    let discovery = ServiceDiscovery::new();
    // QR bit = 0 (query)
    let mut data = [0u8; 20];
    data[2] = 0x00; // flags: standard query
    data[4] = 0x00;
    data[5] = 0x01; // 1 question
    let result = discovery.parse_mdns_response(&data, "test");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_parse_mdns_response_with_a_record() {
    let discovery = ServiceDiscovery::new();
    // Minimal DNS response: header + question + 1 A record
    let mut data = Vec::new();
    data.extend_from_slice(&[0x00, 0x00]); // id
    data.extend_from_slice(&[0x80, 0x00]); // flags (QR=1)
    data.extend_from_slice(&[0x00, 0x01]); // 1 question
    data.extend_from_slice(&[0x00, 0x01]); // 1 answer
    data.extend_from_slice(&[0x00, 0x00]); // 0 auth
    data.extend_from_slice(&[0x00, 0x00]); // 0 additional
    // Question: _test._tcp.local (simplified)
    data.push(5);
    data.extend_from_slice(b"_test");
    data.push(4);
    data.extend_from_slice(b"_tcp");
    data.push(5);
    data.extend_from_slice(b"local");
    data.push(0);
    data.extend_from_slice(&[0x00, 0x0C]); // PTR
    data.extend_from_slice(&[0x00, 0x01]); // IN
    // Answer: instance._test._tcp.local, A record, 127.0.0.1
    data.push(8);
    data.extend_from_slice(b"instance");
    data.push(5);
    data.extend_from_slice(b"_test");
    data.push(4);
    data.extend_from_slice(b"_tcp");
    data.push(5);
    data.extend_from_slice(b"local");
    data.push(0);
    data.extend_from_slice(&[0x00, 0x01]); // A
    data.extend_from_slice(&[0x00, 0x01]); // IN
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL 60
    data.extend_from_slice(&[0x00, 0x04]); // rdlength 4
    data.extend_from_slice(&[127, 0, 0, 1]); // 127.0.0.1

    let result = discovery.parse_mdns_response(&data, "toadstool");
    assert!(result.is_ok());
    let endpoint = result.unwrap();
    assert!(endpoint.is_some());
    let ep = endpoint.unwrap();
    assert_eq!(ep.name, "toadstool");
    assert!(ep.endpoint.contains("127.0.0.1"));
    let expected_port = toadstool_config::ports::daemon_port().to_string();
    assert!(ep.endpoint.contains(&expected_port));
}

#[tokio::test]
async fn test_parse_mdns_response_no_a_record() {
    let discovery = ServiceDiscovery::new();
    // Response with PTR only (no A record)
    let mut data = Vec::new();
    data.extend_from_slice(&[0x00, 0x00, 0x80, 0x00]);
    data.extend_from_slice(&[0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    data.push(5);
    data.extend_from_slice(b"_test");
    data.push(4);
    data.extend_from_slice(b"_tcp");
    data.push(5);
    data.extend_from_slice(b"local");
    data.push(0);
    data.extend_from_slice(&[0x00, 0x0C, 0x00, 0x01]);
    // Answer: PTR record (type 12), rdata is a name
    data.push(8);
    data.extend_from_slice(b"instance");
    data.push(5);
    data.extend_from_slice(b"_test");
    data.push(4);
    data.extend_from_slice(b"_tcp");
    data.push(5);
    data.extend_from_slice(b"local");
    data.push(0);
    data.extend_from_slice(&[0x00, 0x0C, 0x00, 0x01]);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]);
    data.extend_from_slice(&[0x00, 0x0F]); // rdlength 15
    data.push(8);
    data.extend_from_slice(b"myhost");
    data.push(5);
    data.extend_from_slice(b"local");
    data.push(0);

    let result = discovery.parse_mdns_response(&data, "test");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}
