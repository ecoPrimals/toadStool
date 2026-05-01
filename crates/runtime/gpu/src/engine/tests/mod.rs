// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit and integration tests for [`super::UniversalGpuEngine`](crate::engine::UniversalGpuEngine).

#![allow(clippy::wildcard_imports)]

pub(crate) use std::sync::Arc;

pub(crate) use toadstool::execution::{RuntimeConfig, RuntimeEngine};
pub(crate) use toadstool::{ExecutionRequest, WorkloadSpec, WorkloadType};
pub(crate) use uuid::Uuid;

pub(crate) use super::UniversalGpuEngine;
pub(crate) use crate::{
    ComputeWorkload, DeviceId, DeviceRequirements, GpuFramework, KernelFormat, UniversalGpuConfig,
};

mod conversion_tests;
mod engine_tests;
