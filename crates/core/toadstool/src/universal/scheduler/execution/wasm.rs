// SPDX-License-Identifier: AGPL-3.0-or-later
//! WebAssembly module execution (`execute_wasm`).

use std::collections::HashMap;
use std::time::Duration;

use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::execution::{
    ExecutionInput, ExecutionRequest, ExecutionResponse, RuntimeEngine, RuntimeType,
};
use crate::resources::ResourceRequirements;
use crate::workload::{WasiConfig, WasmModuleSource};
use crate::{SecurityContext, ToadStoolResult, WorkloadSpec};

use super::super::UniversalScheduler;
use crate::universal::traits::UniversalPrimalProvider;

impl<P, E: RuntimeEngine> UniversalScheduler<P, E>
where
    P: UniversalPrimalProvider + Send + Sync + 'static,
{
    /// Execute a WASM job
    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // wasm_engine borrows from engines; must hold lock across execute().await
    pub(in crate::universal::scheduler) async fn execute_wasm(
        &self,
        module: &[u8],
        args: &[String],
        env: &HashMap<String, String>,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing WASM job ({} bytes)", module.len());

        // Check if we have a WASM runtime engine registered
        let engines = self.runtime_engines().read().await;
        if let Some(wasm_engine) = engines.get(&RuntimeType::Wasm) {
            info!("Using registered WASM runtime engine for execution");

            // Build execution request (clone env once, reuse for both fields)
            let env_owned = env.clone();
            let request = ExecutionRequest {
                execution_id: Uuid::new_v4(),
                workload: WorkloadSpec::Wasm {
                    module: WasmModuleSource::Bytes {
                        data: bytes::Bytes::copy_from_slice(module),
                    },
                    args: Some(args.to_vec()),
                    wasi_config: Some(WasiConfig {
                        inherit_env: true,
                        inherit_stdio: true,
                        allowed_dirs: Vec::new(),
                        preopened_dirs: Vec::new(),
                        args: args.to_vec(),
                    }),
                    env_vars: env_owned.clone(),
                },
                runtime_hint: Some(RuntimeType::Wasm),
                resources: ResourceRequirements::default(),
                security_context: SecurityContext::default(),
                timeout: Some(Duration::from_secs(300)),
                environment: env_owned,
                input_data: ExecutionInput::default(),
                callback_config: None,
                encryption_config: None,
            };

            // Execute via the WASM runtime engine
            return wasm_engine.execute(request).await;
        }

        // No WASM engine registered - return proper error
        let error_msg = format!(
            "No WASM execution capability: no runtime engine registered for WASM modules ({} bytes)",
            module.len()
        );
        warn!("{}", error_msg);
        Ok(ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: crate::execution::ExecutionStatus::Failed {
                error: error_msg.clone().into(),
            },
            output: crate::execution::ExecutionOutput {
                data: bytes::Bytes::new(),
                stdout: None,
                stderr: Some(error_msg),
                exit_code: Some(126), // Command not executable
                format: Some("text/plain".to_string()),
                result: HashMap::new(),
                metadata: HashMap::new(),
            },
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(0),
            runtime_used: crate::execution::RuntimeType::Wasm,
            warnings: vec!["Register a WASM runtime engine via register_runtime_engine(RuntimeType::Wasm, engine)".to_string()],
        })
    }
}
