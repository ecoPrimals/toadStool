// SPDX-License-Identifier: AGPL-3.0-or-later
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
    let _h = TransportHandler::new();
    let v = TransportHandler::transport_discover(None);
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
