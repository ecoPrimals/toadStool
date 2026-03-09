// SPDX-License-Identifier: AGPL-3.0-only
//! OpenCL Backend Implementation
//!
//! Real GPU execution using OpenCL - works on NVIDIA, AMD, Intel
//! No mocks, no hardcoding, capability-based discovery
//!
//! ## Module structure
//! - `backend`: Core OpenClBackend, device discovery, program compilation, kernel execution
//! - `resource`: OpenClComputeResource, UniversalComputeResource implementation
//! - `context`: OpenClComputeContext, workload dispatch
//! - `kernels`: Built-in kernel selection and work size helpers

mod backend;
mod context;
mod kernels;
mod resource;

#[cfg(test)]
mod tests;

// Re-export public API for backward compatibility
pub use backend::{DeviceInfo, OpenClBackend};
pub use resource::OpenClComputeResource;
