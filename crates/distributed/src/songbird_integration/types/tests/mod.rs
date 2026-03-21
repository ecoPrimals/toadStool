// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for Songbird integration types

mod tests_core;
mod tests_extra;

use super::*;
use crate::{DistributedRetryConfig, ExecutionTarget, ResourceRequirements, UniversalJobType};
use std::time::SystemTime;
use uuid::Uuid;

pub(super) fn make_test_job(resource_requirements: ResourceRequirements) -> crate::UniversalJob {
    crate::UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::Local),
        execution_request: toadstool::ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: toadstool::JobPriority::Normal,
        dependencies: vec![],
        resource_requirements,
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}
