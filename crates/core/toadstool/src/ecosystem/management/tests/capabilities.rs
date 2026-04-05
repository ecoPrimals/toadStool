// SPDX-License-Identifier: AGPL-3.0-or-later

use super::create_test_service_with_capabilities;
use toadstool_common::primal_identity::{Capability, ComputeCapability};

#[test]
fn test_service_with_compute_capability() {
    let svc = create_test_service_with_capabilities(
        "svc-1",
        "compute-node",
        true,
        vec![Capability::Compute(ComputeCapability::NativeExecution)],
    );
    assert!(
        svc.capabilities
            .contains(&Capability::Compute(ComputeCapability::NativeExecution))
    );
    assert!(svc.healthy);
}

#[test]
fn test_service_with_empty_capabilities() {
    let svc = create_test_service_with_capabilities("svc-2", "empty-node", false, vec![]);
    assert!(svc.capabilities.is_empty());
    assert!(!svc.healthy);
}
