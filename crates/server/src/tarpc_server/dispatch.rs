// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`WorkloadExecutorDispatch`] — enum dispatch over concrete [`WorkloadExecutor`] implementations.

use std::future::Future;

use crate::coordinator_executor::CoordinatorExecutor;
use crate::rpc_types::{ComputeCapabilities, WorkloadResult, WorkloadSubmission};

#[cfg(any(test, feature = "test-mocks"))]
use super::executor::TestWorkloadDouble;
use super::executor::{StandaloneExecutor, WorkloadExecutor};

/// Dispatch wrapper for concrete executors (standalone vs distributed coordinator).
///
/// Production servers use [`WorkloadExecutorDispatch::Standalone`] or
/// [`WorkloadExecutorDispatch::Coordinator`]. When compiled for tests or with the
/// `test-mocks` feature, [`WorkloadExecutorDispatch::TestDouble`] exists for injected behavior.
pub enum WorkloadExecutorDispatch {
    /// Single-node / dev executor (queries local hardware).
    Standalone(StandaloneExecutor),
    /// Distributed coordinator-backed executor.
    Coordinator(CoordinatorExecutor),
    /// Injected test behavior (not used in production startup).
    #[cfg(any(test, feature = "test-mocks"))]
    #[doc(hidden)]
    TestDouble(TestWorkloadDouble),
}

impl WorkloadExecutor for WorkloadExecutorDispatch {
    fn execute(
        &self,
        submission: WorkloadSubmission,
    ) -> impl Future<Output = Result<WorkloadResult, String>> + Send + '_ {
        async move {
            match self {
                Self::Standalone(e) => e.execute(submission).await,
                Self::Coordinator(c) => c.execute(submission).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::TestDouble(t) => t.execute(submission).await,
            }
        }
    }

    fn query_capabilities(
        &self,
    ) -> impl Future<Output = Result<ComputeCapabilities, String>> + Send + '_ {
        async move {
            match self {
                Self::Standalone(e) => e.query_capabilities().await,
                Self::Coordinator(c) => c.query_capabilities().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::TestDouble(t) => t.query_capabilities().await,
            }
        }
    }

    fn cancel<'a>(
        &'a self,
        workload_id: &'a str,
    ) -> impl Future<Output = Result<(), String>> + Send + 'a {
        async move {
            match self {
                Self::Standalone(e) => e.cancel(workload_id).await,
                Self::Coordinator(c) => c.cancel(workload_id).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::TestDouble(t) => t.cancel(workload_id).await,
            }
        }
    }
}
