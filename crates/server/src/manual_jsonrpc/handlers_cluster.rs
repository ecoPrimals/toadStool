//! Cross-gate routing handlers (gate.*)

use crate::cross_gate::GateGpuInfo;

use super::{JsonRpcRequest, ManualJsonRpcServer, INVALID_PARAMS};

impl ManualJsonRpcServer {
    pub(crate) async fn handle_gate_update(
        &self,
        mut request: JsonRpcRequest,
    ) -> serde_json::Value {
        let params = match request.params.take() {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let gate_info: GateGpuInfo = match serde_json::from_value(params) {
            Ok(info) => info,
            Err(e) => {
                return self.error_response(
                    INVALID_PARAMS,
                    format!("Invalid gate info: {e}"),
                    &request,
                )
            }
        };

        let gate_id = gate_info.gate_id.clone();
        self.router.write().await.update_gate(gate_info);
        self.success_response(
            serde_json::json!({"updated": true, "gate_id": gate_id}),
            &request,
        )
    }

    pub(crate) async fn handle_gate_remove(&self, request: JsonRpcRequest) -> serde_json::Value {
        let gate_id = match request
            .params
            .as_ref()
            .and_then(|p| p.get("gate_id"))
            .and_then(|v| v.as_str())
        {
            Some(id) => id,
            None => {
                return self.error_response(INVALID_PARAMS, "Missing 'gate_id' param", &request)
            }
        };

        self.router.write().await.remove_gate(gate_id);
        self.success_response(
            serde_json::json!({"removed": true, "gate_id": gate_id}),
            &request,
        )
    }

    pub(crate) async fn handle_gate_list(&self, request: JsonRpcRequest) -> serde_json::Value {
        let router = self.router.read().await;
        let gates: Vec<&GateGpuInfo> = router.gates().values().collect();
        self.success_response(serde_json::json!({"gates": gates}), &request)
    }

    pub(crate) async fn handle_gate_route(&self, request: JsonRpcRequest) -> serde_json::Value {
        let params = match &request.params {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("");
        let vram = params
            .get("vram_required_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096);

        let router = self.router.read().await;
        let decision = router.route(model, vram);
        self.success_response(
            serde_json::json!({
                "gate_id": decision.gate_id,
                "reason": decision.reason,
                "estimated_wait_ms": decision.estimated_wait_ms,
            }),
            &request,
        )
    }
}
