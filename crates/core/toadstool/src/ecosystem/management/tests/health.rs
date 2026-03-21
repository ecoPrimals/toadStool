// SPDX-License-Identifier: AGPL-3.0-only

use super::{create_test_service, create_test_service_with_id};

#[test]
fn test_healthy_service() {
    let svc = create_test_service("healthy-svc", true);
    assert!(svc.healthy);
    assert!(!svc.name.is_empty());
}

#[test]
fn test_unhealthy_service() {
    let svc = create_test_service("sick-svc", false);
    assert!(!svc.healthy);
}

#[test]
fn test_service_identity_preserved() {
    let svc = create_test_service_with_id("fixed-id-001", "named-svc", true);
    assert_eq!(svc.id, "fixed-id-001");
    assert_eq!(svc.name, "named-svc");
}
