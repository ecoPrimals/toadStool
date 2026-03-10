// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware transport layer for JSON-RPC handler.

use std::sync::Arc;

use crate::pure_jsonrpc::types::JsonRpcError;

/// Handles hardware transport discovery and routing.
pub(super) struct TransportHandler {
    pub(super) transport_router: Arc<tokio::sync::Mutex<toadstool_core::TransportRouter>>,
}

impl TransportHandler {
    pub(super) fn new() -> Self {
        Self {
            transport_router: Arc::new(tokio::sync::Mutex::new(
                toadstool_core::TransportRouter::new(),
            )),
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
}
