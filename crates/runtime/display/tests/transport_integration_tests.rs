// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for hardware transport layer.
//!
//! Tests `TransportRouter`, `TransportFilter`, and framed loopback transport
//! with `encode_frame/decode_frame`.

use toadstool_core::{
    FRAME_HEADER_SIZE, HardwareTransport, TransportDirection, TransportError, TransportFilter,
    TransportInfo, TransportMedium, TransportRouter, decode_frame, encode_frame,
};

/// Loopback transport with proper framing support for testing.
/// Stores raw bytes; caller uses `encode_frame/decode_frame` for protocol.
struct LoopbackTransport {
    info: TransportInfo,
    buf: Vec<u8>,
}

impl LoopbackTransport {
    fn new(id: &str, direction: TransportDirection) -> Self {
        Self {
            info: TransportInfo {
                id: id.to_string(),
                label: id.to_string(),
                medium: TransportMedium::Serial,
                direction,
            },
            buf: Vec::new(),
        }
    }
}

impl HardwareTransport for LoopbackTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }
    fn bandwidth_bps(&self) -> u64 {
        1_000_000
    }
    fn is_available(&self) -> bool {
        true
    }
    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let n = buf.len().min(self.buf.len());
        buf[..n].copy_from_slice(&self.buf[..n]);
        self.buf.drain(..n);
        Ok(n)
    }
}

/// High-bandwidth mock transport for filter tests.
struct HighBandwidthTransport {
    info: TransportInfo,
    bandwidth_bps: u64,
}

impl HighBandwidthTransport {
    fn new(id: &str, direction: TransportDirection, bandwidth_bps: u64) -> Self {
        Self {
            info: TransportInfo {
                id: id.to_string(),
                label: id.to_string(),
                medium: TransportMedium::Display,
                direction,
            },
            bandwidth_bps,
        }
    }
}

impl HardwareTransport for HighBandwidthTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }
    fn bandwidth_bps(&self) -> u64 {
        self.bandwidth_bps
    }
    fn is_available(&self) -> bool {
        true
    }
    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        Ok(data.len())
    }
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        Ok(buf.len().min(0))
    }
}

#[test]
fn framed_loopback_encode_send_recv_decode() {
    let payload = b"Hello, framed transport!";
    let mut frame_buf = vec![0u8; FRAME_HEADER_SIZE + payload.len() + 64];

    let written = encode_frame(0, payload, &mut frame_buf).unwrap();
    assert_eq!(written, FRAME_HEADER_SIZE + payload.len());

    // Tx sends framed data; we use a bidi transport for loopback
    let mut loopback = LoopbackTransport::new("loop", TransportDirection::Bidirectional);
    loopback.send(&frame_buf[..written]).expect("send framed");

    let mut recv_buf = vec![0u8; written];
    let n = loopback.recv(&mut recv_buf).expect("recv framed");
    assert_eq!(n, written);

    let (seq, decoded) = decode_frame(&recv_buf[..n]).unwrap();
    assert_eq!(seq, 0);
    assert_eq!(decoded, payload);
}

#[test]
fn transport_router_register_three_filter_by_direction() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "rx1",
        TransportDirection::Rx,
    )));
    router.register(Box::new(LoopbackTransport::new(
        "tx1",
        TransportDirection::Tx,
    )));
    router.register(Box::new(LoopbackTransport::new(
        "bidi1",
        TransportDirection::Bidirectional,
    )));

    assert_eq!(router.list().len(), 3);

    let tx_only = router.find(&TransportFilter::tx());
    assert!(tx_only.contains(&"tx1".to_string()));
    assert!(tx_only.contains(&"bidi1".to_string()));
    assert!(!tx_only.contains(&"rx1".to_string()));

    let rx_only = router.find(&TransportFilter::rx());
    assert!(rx_only.contains(&"rx1".to_string()));
    assert!(rx_only.contains(&"bidi1".to_string()));
    assert!(!rx_only.contains(&"tx1".to_string()));
}

#[test]
fn transport_router_route_once_rx_to_tx() {
    let mut router = TransportRouter::new();

    let mut rx = LoopbackTransport::new("rx", TransportDirection::Bidirectional);
    rx.send(b"hello transport").unwrap();
    router.register(Box::new(rx));

    router.register(Box::new(LoopbackTransport::new(
        "tx",
        TransportDirection::Bidirectional,
    )));

    let n = router.route_once("rx", "tx", 1024).unwrap();
    assert_eq!(n, 15);
}

#[test]
fn transport_filter_medium_and_bandwidth() {
    let mut router = TransportRouter::new();
    router.register(Box::new(HighBandwidthTransport::new(
        "fast_tx",
        TransportDirection::Tx,
        10_000_000_000,
    )));
    router.register(Box::new(HighBandwidthTransport::new(
        "slow_tx",
        TransportDirection::Tx,
        100_000,
    )));
    router.register(Box::new(LoopbackTransport::new(
        "serial_tx",
        TransportDirection::Tx,
    )));

    let high_bw = TransportFilter::tx()
        .with_medium(TransportMedium::Display)
        .with_min_bandwidth(1_000_000_000);
    let matches = router.find(&high_bw);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], "fast_tx");

    let low_bw = TransportFilter::tx().with_min_bandwidth(50_000);
    assert!(router.find(&low_bw).len() >= 2);
}

#[test]
fn transport_filter_medium_and_bandwidth_fixed() {
    let mut router = TransportRouter::new();
    router.register(Box::new(HighBandwidthTransport::new(
        "fast",
        TransportDirection::Tx,
        10_000_000_000,
    )));
    router.register(Box::new(HighBandwidthTransport::new(
        "slow",
        TransportDirection::Tx,
        100_000,
    )));

    let high_bw = TransportFilter::tx().with_min_bandwidth(10_000_000_000);
    let matches = router.find(&high_bw);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], "fast");

    let low_bw = TransportFilter::tx().with_min_bandwidth(100_000);
    let matches = router.find(&low_bw);
    assert_eq!(matches.len(), 2);
}

#[test]
fn route_to_nonexistent_transport_errors() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "rx",
        TransportDirection::Rx,
    )));

    let err = router.route_once("rx", "nonexistent", 64).unwrap_err();
    assert!(matches!(err, TransportError::Unavailable(_)));
    assert!(err.to_string().contains("tx transport not found"));

    let err = router.route_once("nonexistent", "rx", 64).unwrap_err();
    assert!(matches!(err, TransportError::Unavailable(_)));
    assert!(err.to_string().contains("rx transport not found"));
}

#[test]
fn route_same_id_rejected() {
    let mut router = TransportRouter::new();
    router.register(Box::new(LoopbackTransport::new(
        "self",
        TransportDirection::Bidirectional,
    )));
    let err = router.route_once("self", "self", 64).unwrap_err();
    assert!(matches!(err, TransportError::Unavailable(_)));
    assert!(err.to_string().contains("same transport"));
}
