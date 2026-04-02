// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware transport layer for JSON-RPC handler.

use std::collections::HashMap;
use std::sync::Arc;

use toadstool_core::HardwareTransport;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::pure_jsonrpc::types::JsonRpcError;

/// Active stream metadata tracked by the handler.
struct ActiveStream {
    rx_id: String,
    tx_id: String,
    cancel: CancellationToken,
    bytes_transferred: Arc<std::sync::atomic::AtomicU64>,
    started_at: std::time::Instant,
}

/// Handles hardware transport discovery and routing.
pub(super) struct TransportHandler {
    pub(super) transport_router: Arc<Mutex<toadstool_core::TransportRouter>>,
    active_streams: Mutex<HashMap<String, ActiveStream>>,
    next_stream_id: Mutex<u64>,
}

impl TransportHandler {
    pub(super) fn new() -> Self {
        Self {
            transport_router: Arc::new(Mutex::new(toadstool_core::TransportRouter::new())),
            active_streams: Mutex::new(HashMap::new()),
            next_stream_id: Mutex::new(1),
        }
    }

    pub(super) async fn transport_discover(
        &self,
        _params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let display = toadstool_display::discover_display_transports();
        let capture = toadstool_display::discover_capture_transports();
        let serial = toadstool_display::serial_transport::discover_serial_transports();
        let pcie = toadstool_display::discover_pcie_transports();

        let all: Vec<_> = display
            .iter()
            .chain(capture.iter())
            .chain(serial.iter())
            .chain(pcie.iter())
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "label": t.label,
                    "medium": format!("{}", t.medium),
                    "direction": format!("{}", t.direction),
                })
            })
            .collect();

        Ok(serde_json::json!({"transports": all, "count": all.len()}))
    }

    pub(super) async fn transport_list(&self) -> Result<serde_json::Value, JsonRpcError> {
        let router = self.transport_router.lock().await;
        let list: Vec<_> = router
            .list()
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "label": t.label,
                    "medium": format!("{}", t.medium),
                    "direction": format!("{}", t.direction),
                })
            })
            .collect();
        Ok(serde_json::json!({"transports": list, "count": list.len()}))
    }

    pub(super) async fn transport_route(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let rx_id = params
            .get("rx_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'rx_id'"))?;
        let tx_id = params
            .get("tx_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'tx_id'"))?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "buf_size is capped at practical values"
        )]
        let buf_size = params
            .get("buf_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(65536) as usize;

        let mut router = self.transport_router.lock().await;
        let bytes = router
            .route_once(rx_id, tx_id, buf_size)
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;
        Ok(serde_json::json!({
            "bytes_transferred": bytes,
            "rx_id": rx_id,
            "tx_id": tx_id
        }))
    }

    /// Register a discovered `PCIe` transport into the router by its link endpoints.
    ///
    /// Params: `{ "source_slot": "0000:25:00.0", "target_slot": "0000:41:00.0" }`
    pub(super) async fn transport_open(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let source_slot = params
            .get("source_slot")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'source_slot'"))?;
        let target_slot = params
            .get("target_slot")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'target_slot'"))?;

        let links = toadstool_display::pcie_transport::discover_pcie_links();
        let link = links
            .iter()
            .find(|l| l.source.pci_slot == source_slot && l.target.pci_slot == target_slot)
            .ok_or_else(|| {
                JsonRpcError::invalid_params(format!(
                    "No PCIe link found between {source_slot} and {target_slot}"
                ))
            })?;

        let transport =
            toadstool_display::PcieTransport::open(link.source.clone(), link.target.clone())
                .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        let id = transport.info().id.clone();
        let bandwidth = transport.bandwidth_bps();

        let mut router = self.transport_router.lock().await;
        router.register(Box::new(transport));

        Ok(serde_json::json!({
            "id": id,
            "bandwidth_bps": bandwidth,
            "status": "registered"
        }))
    }

    /// Start continuous streaming between two registered transports.
    ///
    /// Params: `{ "rx_id": "...", "tx_id": "...", "buf_size": 65536 }`
    /// Returns: `{ "stream_id": "...", "status": "streaming" }`
    pub(super) async fn transport_stream(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let rx_id = params
            .get("rx_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'rx_id'"))?
            .to_string();
        let tx_id = params
            .get("tx_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'tx_id'"))?
            .to_string();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "buf_size is capped at practical values"
        )]
        let buf_size = params
            .get("buf_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(65536) as usize;

        {
            let router = self.transport_router.lock().await;
            if router.get(&rx_id).is_none() {
                return Err(JsonRpcError::invalid_params(format!(
                    "rx transport not registered: {rx_id}"
                )));
            }
            if router.get(&tx_id).is_none() {
                return Err(JsonRpcError::invalid_params(format!(
                    "tx transport not registered: {tx_id}"
                )));
            }
        }

        let mut id_counter = self.next_stream_id.lock().await;
        let stream_id = format!("stream-{}", *id_counter);
        *id_counter += 1;

        let cancel = CancellationToken::new();
        let bytes_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let stream = ActiveStream {
            rx_id: rx_id.clone(),
            tx_id: tx_id.clone(),
            cancel: cancel.clone(),
            bytes_transferred: Arc::clone(&bytes_counter),
            started_at: std::time::Instant::now(),
        };

        self.active_streams
            .lock()
            .await
            .insert(stream_id.clone(), stream);

        let router = Arc::clone(&self.transport_router);
        tokio::spawn(async move {
            loop {
                if cancel.is_cancelled() {
                    break;
                }

                let result = {
                    let mut r = router.lock().await;
                    r.route_once(&rx_id, &tx_id, buf_size)
                };

                match result {
                    Ok(n) => {
                        bytes_counter.fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                }

                tokio::task::yield_now().await;
            }
        });

        Ok(serde_json::json!({
            "stream_id": stream_id,
            "status": "streaming"
        }))
    }

    /// Query status of active streams.
    ///
    /// Params: `{ "stream_id": "stream-1" }` (optional — all streams if omitted)
    pub(super) async fn transport_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let streams = self.active_streams.lock().await;

        if let Some(params) = params
            && let Some(stream_id) = params.get("stream_id").and_then(|v| v.as_str())
        {
            let stream = streams.get(stream_id).ok_or_else(|| {
                JsonRpcError::invalid_params(format!("Unknown stream: {stream_id}"))
            })?;

            let bytes = stream
                .bytes_transferred
                .load(std::sync::atomic::Ordering::Relaxed);
            let elapsed = stream.started_at.elapsed();

            return Ok(serde_json::json!({
                "stream_id": stream_id,
                "rx_id": stream.rx_id,
                "tx_id": stream.tx_id,
                "bytes_transferred": bytes,
                "elapsed_seconds": elapsed.as_secs_f64(),
                "active": !stream.cancel.is_cancelled(),
            }));
        }

        let all: Vec<_> = streams
            .iter()
            .map(|(id, s)| {
                let bytes = s
                    .bytes_transferred
                    .load(std::sync::atomic::Ordering::Relaxed);
                let elapsed = s.started_at.elapsed();
                serde_json::json!({
                    "stream_id": id,
                    "rx_id": s.rx_id,
                    "tx_id": s.tx_id,
                    "bytes_transferred": bytes,
                    "elapsed_seconds": elapsed.as_secs_f64(),
                    "active": !s.cancel.is_cancelled(),
                })
            })
            .collect();

        Ok(serde_json::json!({ "streams": all, "count": all.len() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure_jsonrpc::types::JsonRpcError;
    use serde_json::json;
    use toadstool_core::{
        HardwareTransport, TransportDirection, TransportError, TransportInfo, TransportMedium,
    };

    /// Minimal transport for exercising the router in unit tests (mirrors `toadstool_core` tests).
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

    async fn register_rx_tx_pair(handler: &TransportHandler) {
        let mut router = handler.transport_router.lock().await;
        let mut rx = LoopbackTransport::new("rx", TransportDirection::Rx);
        rx.buf = b"chunk".to_vec();
        router.register(Box::new(rx));
        router.register(Box::new(LoopbackTransport::new(
            "tx",
            TransportDirection::Tx,
        )));
    }

    #[tokio::test]
    async fn new_creates_handler_with_empty_router() {
        let h = TransportHandler::new();
        let router = h.transport_router.lock().await;
        assert!(router.list().is_empty());
    }

    #[tokio::test]
    async fn transport_discover_returns_transports_and_count() {
        let h = TransportHandler::new();
        let v = h.transport_discover(None).await.unwrap();
        assert!(v.get("transports").is_some());
        assert!(v.get("count").is_some());
        assert_eq!(
            v["count"].as_u64().unwrap() as usize,
            v["transports"].as_array().unwrap().len()
        );
    }

    #[tokio::test]
    async fn transport_list_empty_router() {
        let h = TransportHandler::new();
        let v = h.transport_list().await.unwrap();
        assert_eq!(v["count"], 0);
        assert_eq!(v["transports"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn transport_route_missing_params() {
        let h = TransportHandler::new();
        let e = h.transport_route(None).await.unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("Missing params"));
    }

    #[tokio::test]
    async fn transport_route_missing_rx_id_or_tx_id() {
        let h = TransportHandler::new();
        let e = h
            .transport_route(Some(&json!({ "tx_id": "t" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("rx_id"));

        let e = h
            .transport_route(Some(&json!({ "rx_id": "r" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("tx_id"));
    }

    #[tokio::test]
    async fn transport_stream_missing_params() {
        let h = TransportHandler::new();
        let e = h.transport_stream(None).await.unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("Missing params"));
    }

    #[tokio::test]
    async fn transport_stream_missing_rx_id() {
        let h = TransportHandler::new();
        let e = h
            .transport_stream(Some(&json!({ "tx_id": "tx" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("rx_id"));
    }

    #[tokio::test]
    async fn transport_stream_missing_tx_id() {
        let h = TransportHandler::new();
        let e = h
            .transport_stream(Some(&json!({ "rx_id": "rx" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("tx_id"));
    }

    #[tokio::test]
    async fn transport_stream_unregistered_rx_id() {
        let h = TransportHandler::new();
        let e = h
            .transport_stream(Some(&json!({ "rx_id": "ghost-rx", "tx_id": "ghost-tx" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("rx transport not registered"));
    }

    #[tokio::test]
    async fn transport_stream_unregistered_tx_id() {
        let h = TransportHandler::new();
        {
            let mut router = h.transport_router.lock().await;
            router.register(Box::new(LoopbackTransport::new(
                "rx-only",
                TransportDirection::Rx,
            )));
        }
        let e = h
            .transport_stream(Some(&json!({ "rx_id": "rx-only", "tx_id": "missing-tx" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("tx transport not registered"));
    }

    #[tokio::test]
    async fn transport_status_no_params_lists_all_empty() {
        let h = TransportHandler::new();
        let v = h.transport_status(None).await.unwrap();
        assert_eq!(v["count"], 0);
        assert_eq!(v["streams"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn transport_status_unknown_stream_id() {
        let h = TransportHandler::new();
        let e = h
            .transport_status(Some(&json!({ "stream_id": "stream-999" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("Unknown stream"));
    }

    #[tokio::test]
    async fn transport_route_transfers_bytes_when_transports_registered() {
        let h = TransportHandler::new();
        register_rx_tx_pair(&h).await;
        let v = h
            .transport_route(Some(&json!({
                "rx_id": "rx",
                "tx_id": "tx",
                "buf_size": 1024
            })))
            .await
            .unwrap();
        assert_eq!(v["bytes_transferred"], 5);
        assert_eq!(v["rx_id"], "rx");
        assert_eq!(v["tx_id"], "tx");
    }

    #[tokio::test]
    async fn transport_stream_and_status_happy_path() {
        let h = TransportHandler::new();
        register_rx_tx_pair(&h).await;
        let started = h
            .transport_stream(Some(&json!({
                "rx_id": "rx",
                "tx_id": "tx",
                "buf_size": 256
            })))
            .await
            .unwrap();
        let stream_id = started["stream_id"].as_str().unwrap();
        assert_eq!(started["status"], "streaming");

        let one = h
            .transport_status(Some(&json!({ "stream_id": stream_id })))
            .await
            .unwrap();
        assert_eq!(one["stream_id"], stream_id);
        assert_eq!(one["rx_id"], "rx");
        assert_eq!(one["tx_id"], "tx");
        assert_eq!(one["active"], true);

        let all = h.transport_status(None).await.unwrap();
        assert_eq!(all["count"], 1);
        assert_eq!(all["streams"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn transport_open_missing_params() {
        let h = TransportHandler::new();
        let e = h.transport_open(None).await.unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("Missing params"));
    }

    #[tokio::test]
    async fn transport_open_missing_source_slot() {
        let h = TransportHandler::new();
        let e = h
            .transport_open(Some(&json!({ "target_slot": "0000:00:00.0" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("source_slot"));
    }

    #[tokio::test]
    async fn transport_open_missing_target_slot() {
        let h = TransportHandler::new();
        let e = h
            .transport_open(Some(&json!({ "source_slot": "0000:00:00.0" })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("target_slot"));
    }

    #[tokio::test]
    async fn transport_open_no_pcie_link_for_slots() {
        let h = TransportHandler::new();
        let e = h
            .transport_open(Some(&json!({
                "source_slot": "ffff:ff:00.0",
                "target_slot": "ffff:ff:00.1"
            })))
            .await
            .unwrap_err();
        assert_eq!(e.code, JsonRpcError::INVALID_PARAMS);
        assert!(e.message.contains("No PCIe link"));
    }
}
