// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn test_registry_creation() {
    let registry = SemanticMethodRegistry::new();
    assert!(registry.count() > 0);
}

#[test]
fn test_compute_domain_resolution() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("compute.execute"),
        Some("execute_workload")
    );
    assert_eq!(registry.resolve("compute.stop"), Some("stop_workload"));
    assert_eq!(registry.resolve("compute.pause"), Some("pause_workload"));
    assert_eq!(registry.resolve("compute.resume"), Some("resume_workload"));
}

#[test]
fn test_resource_domain_resolution() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("resource.cpu.get_usage"),
        Some("get_cpu_usage")
    );
    assert_eq!(
        registry.resolve("resource.memory.get_usage"),
        Some("get_memory_usage")
    );
    assert_eq!(
        registry.resolve("resource.health.check"),
        Some("check_health")
    );
}

#[test]
fn test_storage_domain_resolution() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("storage.artifact.store"),
        Some("store_artifact")
    );
    assert_eq!(
        registry.resolve("storage.artifact.get"),
        Some("retrieve_artifact")
    );
    assert_eq!(
        registry.resolve("storage.artifact.list"),
        Some("list_artifacts")
    );
}

#[test]
fn test_network_domain_resolution() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("network.configure"),
        Some("configure_networking")
    );
    assert_eq!(
        registry.resolve("network.connectivity.check"),
        Some("check_connectivity")
    );
}

#[test]
fn test_security_domain_resolution() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("security.policy.apply"),
        Some("apply_security_policies")
    );
    assert_eq!(
        registry.resolve("security.permission.check"),
        Some("check_permissions")
    );
}

#[test]
fn test_unknown_method() {
    let registry = SemanticMethodRegistry::new();
    assert_eq!(registry.resolve("unknown.method"), None);
}

#[test]
fn test_is_semantic() {
    let registry = SemanticMethodRegistry::new();

    assert!(registry.is_semantic("compute.execute"));
    assert!(registry.is_semantic("resource.cpu.get_usage"));
    assert!(!registry.is_semantic("execute_workload"));
    assert!(!registry.is_semantic("single_word"));
}

#[test]
fn test_reverse_lookup() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.get_semantic("execute_workload"),
        Some("compute.execute")
    );
    assert_eq!(
        registry.get_semantic("get_cpu_usage"),
        Some("resource.cpu.get_usage")
    );
    assert_eq!(registry.get_semantic("unknown_method"), None);
}

#[test]
fn test_is_registered() {
    let registry = SemanticMethodRegistry::new();

    assert!(registry.is_registered("compute.execute"));
    assert!(registry.is_registered("resource.health.check"));
    assert!(!registry.is_registered("unknown.method"));
}

#[test]
fn test_semantic_names_list() {
    let registry = SemanticMethodRegistry::new();
    let names = registry.semantic_names();

    assert!(names.contains(&"compute.execute"));
    assert!(names.contains(&"resource.health.check"));
    assert!(names.len() > 40); // Should have many mappings
}

#[test]
fn test_implementation_names_list() {
    let registry = SemanticMethodRegistry::new();
    let names = registry.implementation_names();

    assert!(names.contains(&"execute_workload"));
    assert!(names.contains(&"check_health"));
}

#[test]
fn test_runtime_variants() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("compute.container.run"),
        Some("run_container")
    );
    assert_eq!(
        registry.resolve("compute.wasm.execute"),
        Some("start_wasm_module")
    );
    assert_eq!(
        registry.resolve("compute.python.execute"),
        Some("run_python_script")
    );
    assert_eq!(
        registry.resolve("compute.native.execute"),
        Some("run_native_binary")
    );
    assert_eq!(
        registry.resolve("compute.gpu.execute"),
        Some("run_gpu_compute")
    );
}

#[test]
fn test_science_domain_resolution() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.resolve("science.compute.submit"),
        Some("science_compute_submit")
    );
    assert_eq!(
        registry.resolve("science.gpu.dispatch"),
        Some("science_gpu_dispatch")
    );
    assert_eq!(
        registry.resolve("science.npu.dispatch"),
        Some("science_npu_dispatch")
    );
    assert_eq!(
        registry.resolve("science.substrate.discover"),
        Some("science_substrate_discover")
    );
}

#[test]
fn test_science_domain_reverse() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(
        registry.get_semantic("science_gpu_capabilities"),
        Some("science.gpu.capabilities")
    );
    assert_eq!(
        registry.get_semantic("science_npu_dispatch"),
        Some("science.npu.dispatch")
    );
}

#[test]
fn test_shader_compile_removed() {
    let registry = SemanticMethodRegistry::new();

    assert!(registry.resolve("shader.compile.wgsl").is_none());
    assert!(registry.resolve("shader.compile.spirv").is_none());
    assert!(registry.resolve("shader.compile.status").is_none());
    assert!(registry.resolve("shader.compile.capabilities").is_none());
}

#[test]
fn test_shader_dispatch_present() {
    let registry = SemanticMethodRegistry::new();

    assert_eq!(registry.resolve("shader.dispatch"), Some("shader_dispatch"));
    assert_eq!(
        registry.get_semantic("shader_dispatch"),
        Some("shader.dispatch")
    );
}

#[test]
fn test_bidirectional_mapping() {
    let registry = SemanticMethodRegistry::new();

    // Forward lookup
    let impl_name = registry.resolve("compute.execute").unwrap();
    assert_eq!(impl_name, "execute_workload");

    // Reverse lookup
    let semantic_name = registry.get_semantic(impl_name).unwrap();
    assert_eq!(semantic_name, "compute.execute");
}
