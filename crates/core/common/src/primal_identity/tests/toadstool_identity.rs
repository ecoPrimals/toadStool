// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::*;

#[test]
fn test_toadstool_identity() {
    let identity = ToadStoolIdentity::new();

    assert_eq!(identity.primal_name(), "toadstool");
    assert!(!identity.version().is_empty());
    assert!(!identity.capabilities().is_empty());
}

#[test]
fn test_toadstool_identity_default_capabilities() {
    let identity = ToadStoolIdentity::new();
    let caps = identity.capabilities();

    assert!(caps.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::WasmExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::GpuCompute)));
}

#[test]
fn test_toadstool_identity_add_endpoint() {
    let mut identity = ToadStoolIdentity::new();
    identity.add_endpoint(ServiceEndpoint::http("localhost", 8080));

    let endpoints = identity.endpoints();
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].protocol, "http");
}

#[test]
fn test_toadstool_identity_with_endpoints() {
    let endpoints = vec![
        ServiceEndpoint::http("localhost", 8080),
        ServiceEndpoint::grpc("localhost", 9090),
    ];
    let identity = ToadStoolIdentity::new().with_endpoints(endpoints);

    assert_eq!(identity.endpoints().len(), 2);
}

#[test]
fn test_toadstool_identity_add_capability() {
    let mut identity = ToadStoolIdentity::new();
    let initial_count = identity.capabilities().len();

    identity.add_capability(Capability::Storage(StorageCapability::ObjectStorage));
    assert_eq!(identity.capabilities().len(), initial_count + 1);

    identity.add_capability(Capability::Storage(StorageCapability::ObjectStorage));
    assert_eq!(identity.capabilities().len(), initial_count + 1);
}

#[test]
fn test_toadstool_identity_add_metadata() {
    let mut identity = ToadStoolIdentity::new();
    identity.add_metadata("custom_key", "custom_value");

    let metadata = identity.metadata();
    assert_eq!(
        metadata.get("custom_key"),
        Some(&"custom_value".to_string())
    );
}

#[test]
fn test_toadstool_identity_metadata_includes_platform() {
    let identity = ToadStoolIdentity::new();
    let metadata = identity.metadata();

    assert!(metadata.contains_key("platform"));
    assert!(metadata.contains_key("arch"));
    assert!(metadata.contains_key("description"));
}

#[test]
fn test_toadstool_identity_default() {
    let identity = ToadStoolIdentity::default();
    assert_eq!(identity.primal_name(), "toadstool");
    assert!(!identity.version().is_empty());
}

#[test]
fn test_toadstool_identity_debug_formatting() {
    let identity = ToadStoolIdentity::new();
    let debug_str = format!("{identity:?}");
    assert!(!debug_str.is_empty());
}

#[test]
fn test_toadstool_identity_builder_pattern() {
    let identity = ToadStoolIdentity::new().with_endpoints(vec![
        ServiceEndpoint::http("localhost", 8080),
        ServiceEndpoint::grpc("localhost", 9090),
    ]);
    assert_eq!(identity.endpoints().len(), 2);
}

#[test]
fn test_toadstool_identity_add_capability_no_duplicate() {
    let mut identity = ToadStoolIdentity::new();
    let cap = Capability::Storage(StorageCapability::ObjectStorage);
    let count_before = identity.capabilities().len();
    identity.add_capability(cap.clone());
    identity.add_capability(cap);
    assert_eq!(identity.capabilities().len(), count_before + 1);
}

#[test]
fn test_primal_identity_trait_object() {
    let identity = ToadStoolIdentity::new();
    assert_eq!(identity.primal_name(), "toadstool");
    assert!(!identity.version().is_empty());
    assert!(!identity.capabilities().is_empty());
}

#[test]
fn test_toadstool_identity_default_capabilities_contains_all() {
    let identity = ToadStoolIdentity::new();
    let caps = identity.capabilities();

    assert!(caps.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
    assert!(caps.contains(&Capability::Compute(
        ComputeCapability::ContainerOrchestration
    )));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::WasmExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::PythonExecution)));
    assert!(caps.contains(&Capability::Compute(ComputeCapability::GpuCompute)));
    assert_eq!(caps.len(), 5);
}

#[test]
fn test_toadstool_identity_add_capability_dedup() {
    let mut identity = ToadStoolIdentity::new();
    let cap = Capability::Compute(ComputeCapability::EdgeExecution);
    let len_before = identity.capabilities().len();
    identity.add_capability(cap.clone());
    identity.add_capability(cap);
    assert_eq!(identity.capabilities().len(), len_before + 1);
}

#[test]
fn test_primal_identity_metadata_platform_arch() {
    let identity = ToadStoolIdentity::new();
    let meta = identity.metadata();
    assert!(meta.contains_key("platform"));
    assert!(meta.contains_key("arch"));
    assert!(meta.get("platform").is_some_and(|s| !s.is_empty()));
}

#[test]
fn test_toadstool_identity_capabilities_clone() {
    let identity = ToadStoolIdentity::new();
    let caps1 = identity.capabilities();
    let caps2 = identity.capabilities();
    assert_eq!(caps1.len(), caps2.len());
}

#[test]
fn test_toadstool_identity_endpoints_clone() {
    let identity =
        ToadStoolIdentity::new().with_endpoints(vec![ServiceEndpoint::http("localhost", 8080)]);
    let eps1 = identity.endpoints();
    let eps2 = identity.endpoints();
    assert_eq!(eps1.len(), eps2.len());
    assert_eq!(eps1[0].protocol, eps2[0].protocol);
}

#[test]
fn test_toadstool_identity_version_contains_semver() {
    let identity = ToadStoolIdentity::new();
    let version = identity.version();
    assert!(!version.is_empty());
    assert!(version.chars().next().is_some_and(|c| c.is_ascii_digit()) || version.contains('.'));
}
