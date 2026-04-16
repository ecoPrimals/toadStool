// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core platform semantic mappings: compute, resource, storage, network, security.

/// Register compute, resource, storage, network, and security domain mappings.
///
/// Call order matches the historical `SemanticMethodRegistry::new` layout.
pub(crate) fn register<F>(add_mapping: &mut F)
where
    F: FnMut(&str, &str),
{
    // ═══════════════════════════════════════════════════════════
    // COMPUTE DOMAIN - Workload execution operations
    // ═══════════════════════════════════════════════════════════

    add_mapping("compute.execute", "execute_workload");
    add_mapping("compute.stop", "stop_workload");
    add_mapping("compute.pause", "pause_workload");
    add_mapping("compute.resume", "resume_workload");
    add_mapping("compute.cancel", "cancel_workload");

    // Pipeline dispatch — ordered multi-stage compute (neuralSpring ML inference)
    add_mapping("compute.pipeline.submit", "pipeline_submit");
    add_mapping("compute.pipeline.status", "pipeline_status");

    // Runtime-specific variants
    add_mapping("compute.container.run", "run_container");
    add_mapping("compute.container.stop", "stop_container");
    add_mapping("compute.wasm.execute", "start_wasm_module");
    add_mapping("compute.wasm.stop", "stop_wasm_module");
    add_mapping("compute.python.execute", "run_python_script");
    add_mapping("compute.python.stop", "stop_python_script");
    add_mapping("compute.native.execute", "run_native_binary");
    add_mapping("compute.native.stop", "stop_native_binary");
    add_mapping("compute.gpu.execute", "run_gpu_compute");
    add_mapping("compute.gpu.stop", "stop_gpu_compute");

    // ═══════════════════════════════════════════════════════════
    // RESOURCE DOMAIN - Resource monitoring and management
    // ═══════════════════════════════════════════════════════════

    add_mapping("resource.cpu.get_usage", "get_cpu_usage");
    add_mapping("resource.memory.get_usage", "get_memory_usage");
    add_mapping("resource.disk.get_usage", "get_disk_usage");
    add_mapping("resource.network.get_usage", "get_network_usage");
    add_mapping("resource.gpu.get_usage", "get_gpu_usage");

    add_mapping("resource.health.check", "check_health");
    add_mapping("resource.metrics.get", "get_metrics");
    add_mapping("resource.metrics.list", "list_metrics");
    add_mapping("resource.status.get", "get_status");

    add_mapping("resource.limits.get", "get_resource_limits");
    add_mapping("resource.limits.set", "set_resource_limits");
    add_mapping("resource.limits.update", "update_resource_limits");

    // ═══════════════════════════════════════════════════════════
    // STORAGE DOMAIN - Artifact and data storage
    // ═══════════════════════════════════════════════════════════

    add_mapping("storage.artifact.store", "store_artifact");
    add_mapping("storage.artifact.get", "retrieve_artifact");
    add_mapping("storage.artifact.list", "list_artifacts");
    add_mapping("storage.artifact.delete", "delete_artifact");
    add_mapping("storage.artifact.exists", "artifact_exists");

    add_mapping("storage.cache.get", "get_from_cache");
    add_mapping("storage.cache.set", "set_in_cache");
    add_mapping("storage.cache.clear", "clear_cache");
    add_mapping("storage.cache.stats", "get_cache_stats");

    // ═══════════════════════════════════════════════════════════
    // NETWORK DOMAIN - Network configuration and connectivity
    // ═══════════════════════════════════════════════════════════

    add_mapping("network.configure", "configure_networking");
    add_mapping("network.connectivity.check", "check_connectivity");
    add_mapping("network.status.get", "get_network_status");

    // ═══════════════════════════════════════════════════════════
    // SECURITY DOMAIN - Security policies and permissions
    // ═══════════════════════════════════════════════════════════

    add_mapping("security.policy.apply", "apply_security_policies");
    add_mapping("security.policy.get", "get_security_policy");
    add_mapping("security.policy.list", "list_security_policies");
    add_mapping("security.policy.validate", "validate_security_policy");

    add_mapping("security.permission.check", "check_permissions");
    add_mapping("security.permission.grant", "grant_permission");
    add_mapping("security.permission.revoke", "revoke_permission");

    add_mapping("security.sandbox.create", "create_sandbox");
    add_mapping("security.sandbox.destroy", "destroy_sandbox");
    add_mapping("security.sandbox.status", "get_sandbox_status");
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::register;

    #[test]
    fn register_maps_known_semantic_names_to_implementations() {
        let mut aliases = HashMap::new();
        register(&mut |semantic, implementation| {
            aliases.insert(semantic.to_string(), implementation.to_string());
        });
        assert_eq!(
            aliases.get("compute.execute").map(String::as_str),
            Some("execute_workload")
        );
        assert_eq!(
            aliases.get("storage.artifact.store").map(String::as_str),
            Some("store_artifact")
        );
    }

    #[test]
    fn lookup_returns_none_for_unregistered_semantic_name() {
        let mut aliases = HashMap::new();
        register(&mut |semantic, implementation| {
            aliases.insert(semantic.to_string(), implementation.to_string());
        });
        assert!(aliases.get("not.a.registered.method").is_none());
    }

    #[test]
    fn duplicate_semantic_key_last_registration_wins() {
        let mut aliases = HashMap::new();
        let mut add = |semantic: &str, implementation: &str| {
            aliases.insert(semantic.to_string(), implementation.to_string());
        };
        register(&mut add);
        add("compute.execute", "custom_override");
        assert_eq!(
            aliases.get("compute.execute").map(String::as_str),
            Some("custom_override")
        );
    }
}
