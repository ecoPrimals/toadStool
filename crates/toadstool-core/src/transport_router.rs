// SPDX-License-Identifier: AGPL-3.0-only
//! Transport Router — any hardware input to any hardware output.
//!
//! Discovers all available [`HardwareTransport`] endpoints and routes data
//! between any Rx transport and any Tx transport. Supports capability-based
//! selection (e.g. "give me a 10+ Gbps unidirectional Tx").

use std::collections::HashMap;

use crate::hardware_transport::{
    HardwareTransport, TransportDirection, TransportError, TransportInfo, TransportMedium,
};

/// Criteria for selecting a transport endpoint.
#[derive(Debug, Clone, Default)]
pub struct TransportFilter {
    /// Required direction (None = any).
    pub direction: Option<TransportDirection>,
    /// Required medium (None = any).
    pub medium: Option<TransportMedium>,
    /// Minimum bandwidth in bits per second (0 = no minimum).
    pub min_bandwidth_bps: u64,
}

impl TransportFilter {
    /// Match only Tx transports.
    #[must_use]
    pub fn tx() -> Self {
        Self {
            direction: Some(TransportDirection::Tx),
            ..Default::default()
        }
    }

    /// Match only Rx transports.
    #[must_use]
    pub fn rx() -> Self {
        Self {
            direction: Some(TransportDirection::Rx),
            ..Default::default()
        }
    }

    /// Require a specific medium.
    #[must_use]
    pub fn with_medium(mut self, medium: TransportMedium) -> Self {
        self.medium = Some(medium);
        self
    }

    /// Require at least this bandwidth.
    #[must_use]
    pub fn with_min_bandwidth(mut self, bps: u64) -> Self {
        self.min_bandwidth_bps = bps;
        self
    }

    fn matches(&self, transport: &dyn HardwareTransport) -> bool {
        let info = transport.info();
        if let Some(dir) = self.direction {
            if info.direction != dir && info.direction != TransportDirection::Bidirectional {
                return false;
            }
        }
        if let Some(med) = self.medium {
            if info.medium != med {
                return false;
            }
        }
        if self.min_bandwidth_bps > 0 && transport.bandwidth_bps() < self.min_bandwidth_bps {
            return false;
        }
        true
    }
}

/// A registered transport with its runtime handle.
struct RegisteredTransport {
    transport: Box<dyn HardwareTransport>,
}

/// Routes data between any registered hardware transports.
///
/// ```text
///   [Capture Card Rx] ──┐
///                        ├──▶ TransportRouter ──▶ [Serial Tx]
///   [PCIe Rx]       ────┘                    ──▶ [HDMI Tx]
/// ```
pub struct TransportRouter {
    transports: HashMap<String, RegisteredTransport>,
}

impl TransportRouter {
    /// Create an empty router.
    #[must_use]
    pub fn new() -> Self {
        Self {
            transports: HashMap::new(),
        }
    }

    /// Register a transport. Uses `transport.info().id` as the key.
    pub fn register(&mut self, transport: Box<dyn HardwareTransport>) {
        let id = transport.info().id.clone();
        self.transports
            .insert(id, RegisteredTransport { transport });
    }

    /// Remove a transport by ID.
    pub fn unregister(&mut self, id: &str) -> Option<Box<dyn HardwareTransport>> {
        self.transports.remove(id).map(|r| r.transport)
    }

    /// List metadata for all registered transports.
    #[must_use]
    pub fn list(&self) -> Vec<&TransportInfo> {
        self.transports
            .values()
            .map(|r| r.transport.info())
            .collect()
    }

    /// Find transports matching a filter. Returns their IDs.
    #[must_use]
    pub fn find(&self, filter: &TransportFilter) -> Vec<String> {
        self.transports
            .iter()
            .filter(|(_, r)| filter.matches(r.transport.as_ref()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get a reference to a transport by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&dyn HardwareTransport> {
        self.transports
            .get(id)
            .map(|r| &*r.transport as &dyn HardwareTransport)
    }

    /// Get a mutable reference to a transport by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut (dyn HardwareTransport + 'static)> {
        self.transports
            .get_mut(id)
            .map(|r| &mut *r.transport as &mut (dyn HardwareTransport + 'static))
    }

    /// Route a chunk of data from one transport (Rx) to another (Tx).
    ///
    /// Reads up to `buf_size` bytes from `rx_id` and writes them to `tx_id`.
    /// Returns the number of bytes transferred.
    ///
    /// Temporarily removes the Rx transport from the map to satisfy the borrow
    /// checker, then re-inserts it. This avoids unsafe code.
    ///
    /// # Errors
    /// Returns [`TransportError::Unavailable`] if rx/tx are the same, or either
    /// transport is not found. Propagates I/O errors from send/recv.
    pub fn route_once(
        &mut self,
        rx_id: &str,
        tx_id: &str,
        buf_size: usize,
    ) -> Result<usize, TransportError> {
        if rx_id == tx_id {
            return Err(TransportError::Unavailable(
                "rx and tx are the same transport".into(),
            ));
        }

        // Temporarily take the Rx transport out so we can mutably borrow both.
        let mut rx_reg = self.transports.remove(rx_id).ok_or_else(|| {
            TransportError::Unavailable(format!("rx transport not found: {rx_id}"))
        })?;

        let result = (|| {
            let tx_reg = self.transports.get_mut(tx_id).ok_or_else(|| {
                TransportError::Unavailable(format!("tx transport not found: {tx_id}"))
            })?;

            let mut buf = vec![0u8; buf_size];
            let n = rx_reg.transport.recv(&mut buf)?;
            if n == 0 {
                return Ok(0);
            }
            tx_reg.transport.send(&buf[..n])
        })();

        // Always re-insert the Rx transport.
        self.transports.insert(rx_id.to_string(), rx_reg);
        result
    }

    /// Continuously route data from `rx_id` to `tx_id` until an error occurs
    /// or the callback returns `false`.
    ///
    /// `on_chunk` receives the number of bytes transferred per iteration.
    ///
    /// # Errors
    /// Propagates errors from [`route_once`](Self::route_once).
    pub fn route_loop<F>(
        &mut self,
        rx_id: &str,
        tx_id: &str,
        buf_size: usize,
        mut on_chunk: F,
    ) -> Result<u64, TransportError>
    where
        F: FnMut(usize) -> bool,
    {
        let mut total = 0u64;
        loop {
            let n = self.route_once(rx_id, tx_id, buf_size)?;
            total += n as u64;
            if !on_chunk(n) {
                break;
            }
        }
        Ok(total)
    }
}

impl Default for TransportRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_transport::{TransportDirection, TransportInfo, TransportMedium};

    /// Minimal loopback transport for testing.
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

    #[test]
    fn register_and_list() {
        let mut router = TransportRouter::new();
        router.register(Box::new(LoopbackTransport::new(
            "a",
            TransportDirection::Tx,
        )));
        router.register(Box::new(LoopbackTransport::new(
            "b",
            TransportDirection::Rx,
        )));
        assert_eq!(router.list().len(), 2);
    }

    #[test]
    fn filter_by_direction() {
        let mut router = TransportRouter::new();
        router.register(Box::new(LoopbackTransport::new(
            "tx1",
            TransportDirection::Tx,
        )));
        router.register(Box::new(LoopbackTransport::new(
            "rx1",
            TransportDirection::Rx,
        )));
        router.register(Box::new(LoopbackTransport::new(
            "bidi1",
            TransportDirection::Bidirectional,
        )));

        let tx_only = router.find(&TransportFilter::tx());
        assert!(tx_only.contains(&"tx1".to_string()));
        assert!(tx_only.contains(&"bidi1".to_string()));
        assert!(!tx_only.contains(&"rx1".to_string()));
    }

    #[test]
    fn filter_by_bandwidth() {
        let mut router = TransportRouter::new();
        router.register(Box::new(LoopbackTransport::new(
            "slow",
            TransportDirection::Tx,
        )));

        let high_bw = TransportFilter::tx().with_min_bandwidth(10_000_000_000);
        assert!(router.find(&high_bw).is_empty());

        let low_bw = TransportFilter::tx().with_min_bandwidth(100_000);
        assert_eq!(router.find(&low_bw).len(), 1);
    }

    #[test]
    fn route_once_transfers_data() {
        let mut router = TransportRouter::new();

        let mut rx = LoopbackTransport::new("rx", TransportDirection::Bidirectional);
        rx.buf = b"hello transport".to_vec();
        router.register(Box::new(rx));
        router.register(Box::new(LoopbackTransport::new(
            "tx",
            TransportDirection::Bidirectional,
        )));

        let n = router.route_once("rx", "tx", 1024).unwrap();
        assert_eq!(n, 15);
    }

    #[test]
    fn route_same_id_rejected() {
        let mut router = TransportRouter::new();
        router.register(Box::new(LoopbackTransport::new(
            "self",
            TransportDirection::Bidirectional,
        )));
        assert!(router.route_once("self", "self", 64).is_err());
    }

    #[test]
    fn unregister() {
        let mut router = TransportRouter::new();
        router.register(Box::new(LoopbackTransport::new(
            "a",
            TransportDirection::Tx,
        )));
        assert!(router.unregister("a").is_some());
        assert!(router.unregister("a").is_none());
        assert!(router.list().is_empty());
    }
}
