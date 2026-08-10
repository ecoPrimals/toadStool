// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload execution, sessions, and device selection for runs.

use std::sync::Arc;
use std::time::Instant;

use tracing::warn;
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::traits::ParallelComputeFramework;
use crate::types::{
    ComputeResult, ComputeSession, ComputeWorkload, DeviceId, DeviceRequirements, SessionStatus,
};

use super::UniversalGpuEngine;

impl UniversalGpuEngine {
    /// Execute compute workload
    ///
    /// # Errors
    ///
    /// Returns when device selection, session lifecycle, or workload execution fails.
    pub async fn execute_workload(
        &self,
        workload: ComputeWorkload,
    ) -> ToadStoolResult<ComputeResult> {
        // Select optimal device
        let device_id = self.select_optimal_device(&workload.requirements).await?;

        // Create compute session
        let session_id = self
            .create_compute_session(&device_id, workload.parent_session)
            .await?;

        // Execute workload
        let result = self
            .execute_workload_on_device(session_id, &device_id, workload)
            .await;

        // Cleanup session
        if let Err(e) = self.destroy_compute_session(session_id).await {
            warn!("Failed to cleanup session {}: {}", session_id, e);
        }

        result
    }

    /// Select optimal device for workload
    async fn select_optimal_device(
        &self,
        requirements: &DeviceRequirements,
    ) -> ToadStoolResult<DeviceId> {
        let devices = self.devices.read().await;
        let available_devices: Vec<DeviceId> = devices.keys().cloned().collect();
        drop(devices);

        if available_devices.is_empty() {
            return Err(ToadStoolError::runtime("No devices available"));
        }

        // Use load balancer to select device
        let coordinator = Arc::clone(&self.resource_coordinator);
        coordinator
            .select_device(&available_devices, requirements)
            .await
    }

    /// Create compute session
    async fn create_compute_session(
        &self,
        device_id: &DeviceId,
        parent_session: Option<Uuid>,
    ) -> ToadStoolResult<Uuid> {
        let frameworks = self.frameworks.read().await;
        let device = self
            .devices
            .read()
            .await
            .get(device_id)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime("Device not found"))?;

        let framework = Arc::clone(
            frameworks
                .get(&device.id.framework)
                .ok_or_else(|| ToadStoolError::runtime("Framework not available"))?,
        );
        drop(frameworks);

        let session_id = framework.create_session(device_id).await?;

        // Calculate recursion depth
        let recursion_depth = if let Some(parent_id) = parent_session {
            let sessions = self
                .active_sessions
                .read()
                .unwrap_or_else(|e| e.into_inner());
            sessions
                .get(&parent_id)
                .map_or(0, |s| s.recursion_depth + 1)
        } else {
            0
        };

        // Check recursion limits
        if recursion_depth > self.config.recursion.max_recursion_depth {
            return Err(ToadStoolError::runtime("Maximum recursion depth exceeded"));
        }

        // Allocate resources
        let resource_allocation = self
            .resource_coordinator
            .allocate_resources(device_id, &DeviceRequirements::minimal())
            .await?;

        let session = ComputeSession {
            id: session_id,
            device_id: device_id.clone(),
            parent_session,
            child_sessions: Vec::new(),
            recursion_depth,
            start_time: Instant::now(),
            resource_allocation,
            status: SessionStatus::Initializing,
        };

        // Update parent session if this is recursive
        if let Some(parent_id) = parent_session {
            let mut sessions = self
                .active_sessions
                .write()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(parent_session) = sessions.get_mut(&parent_id) {
                parent_session.child_sessions.push(session_id);
            }
        }

        self.active_sessions
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session_id, session);
        Ok(session_id)
    }

    /// Execute workload on specific device
    async fn execute_workload_on_device(
        &self,
        session_id: Uuid,
        device_id: &DeviceId,
        workload: ComputeWorkload,
    ) -> ToadStoolResult<ComputeResult> {
        let start_time = Instant::now();

        // Update session status
        {
            let mut sessions = self
                .active_sessions
                .write()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = SessionStatus::Running;
            }
        }

        let frameworks = self.frameworks.read().await;
        let device = self
            .devices
            .read()
            .await
            .get(device_id)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime("Device not found"))?;

        let framework = Arc::clone(
            frameworks
                .get(&device.id.framework)
                .ok_or_else(|| ToadStoolError::runtime("Framework not available"))?,
        );
        drop(frameworks);

        // Compile kernel
        let compiled_kernel = framework
            .compile_kernel(session_id, &workload.kernel_source, workload.kernel_format)
            .await?;

        // Execute kernel
        let primary_output = framework
            .execute_kernel(session_id, &compiled_kernel, &workload.inputs)
            .await?;

        // Execute recursive workloads
        let mut recursive_results = Vec::new();
        for recursive_workload in workload.recursive_workloads {
            let recursive_result = Box::pin(self.execute_workload(recursive_workload)).await?;
            recursive_results.push(recursive_result);
        }

        // Update session status
        {
            let mut sessions = self
                .active_sessions
                .write()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = SessionStatus::Completed;
            }
        }

        Ok(ComputeResult {
            session_id,
            device_id: device_id.clone(),
            primary_output,
            recursive_results,
            total_execution_time: start_time.elapsed(),
        })
    }

    /// Destroy compute session
    pub(super) async fn destroy_compute_session(&self, session_id: Uuid) -> ToadStoolResult<()> {
        let session = {
            let mut sessions = self
                .active_sessions
                .write()
                .unwrap_or_else(|e| e.into_inner());
            sessions.remove(&session_id)
        };

        if let Some(session) = session {
            let frameworks = self.frameworks.read().await;
            let device = self
                .devices
                .read()
                .await
                .get(&session.device_id)
                .cloned()
                .ok_or_else(|| ToadStoolError::runtime("Device not found"))?;

            let framework = Arc::clone(
                frameworks
                    .get(&device.id.framework)
                    .ok_or_else(|| ToadStoolError::runtime("Framework not available"))?,
            );
            drop(frameworks);

            // Destroy all child sessions first
            for child_session_id in &session.child_sessions {
                if let Err(e) = Box::pin(self.destroy_compute_session(*child_session_id)).await {
                    warn!(
                        "Failed to destroy child session {}: {}",
                        child_session_id, e
                    );
                }
            }

            // Destroy the session in the framework
            framework.destroy_session(session_id).await?;

            // Release resources
            self.resource_coordinator
                .release_resources(&session.device_id, &session.resource_allocation)
                .await?;
        }

        Ok(())
    }
}
