// SPDX-License-Identifier: AGPL-3.0-or-later
//! Semantic Method Name Registry
//!
//! Maps semantic method names to implementation functions following
//! wateringHole/SEMANTIC_METHOD_NAMING_STANDARD.md
//!
//! ## Semantic Namespace Structure
//!
//! Format: `{domain}.{operation}[.{variant}]`
//!
//! - **Domain**: Capability area (compute, resource, storage, network, security)
//! - **Operation**: What the method does (execute, get, store, configure, etc.)
//! - **Variant** (optional): Specific algorithm or mode
//!
//! ## Evolution Strategy
//!
//! **Phase 1** (Current): Backward-compatible aliases
//! - Both old and new names work
//! - New code uses semantic names
//! - Zero breaking changes
//!
//! **Phase 2** (Future): Deprecation warnings
//! - Log warnings for old names
//! - Encourage migration
//!
//! **Phase 3** (Future): Remove old names
//! - Clean semantic-only API
//!
//! ## Example
//!
//! ```rust
//! use toadstool::semantic_methods::SemanticMethodRegistry;
//!
//! let registry = SemanticMethodRegistry::new();
//!
//! // Resolve semantic name to implementation
//! assert_eq!(
//!     registry.resolve("compute.execute"),
//!     Some("execute_workload")
//! );
//!
//! // Check if method is semantic
//! assert!(registry.is_semantic("compute.execute"));
//! assert!(!registry.is_semantic("execute_workload"));
//! ```

use std::collections::HashMap;

/// Semantic method registry
///
/// Maps semantic method names (e.g., `compute.execute`) to implementation
/// method names (e.g., `execute_workload`) for backward compatibility.
#[derive(Debug, Clone)]
pub struct SemanticMethodRegistry {
    /// Method aliases: semantic_name → implementation_name
    aliases: HashMap<String, String>,

    /// Reverse mapping: implementation_name → semantic_name
    reverse: HashMap<String, String>,
}

impl SemanticMethodRegistry {
    /// Create new registry with default mappings
    ///
    /// Initializes all standard ToadStool method mappings following
    /// the wateringHole semantic naming standard.
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        let mut reverse = HashMap::new();

        // Helper to add bidirectional mapping
        let mut add_mapping = |semantic: &str, implementation: &str| {
            aliases.insert(semantic.to_string(), implementation.to_string());
            reverse.insert(implementation.to_string(), semantic.to_string());
        };

        // ═══════════════════════════════════════════════════════════
        // COMPUTE DOMAIN - Workload execution operations
        // ═══════════════════════════════════════════════════════════

        add_mapping("compute.execute", "execute_workload");
        add_mapping("compute.stop", "stop_workload");
        add_mapping("compute.pause", "pause_workload");
        add_mapping("compute.resume", "resume_workload");
        add_mapping("compute.cancel", "cancel_workload");

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

        // ═══════════════════════════════════════════════════════════
        // RUNTIME DOMAIN - Runtime engine management
        // ═══════════════════════════════════════════════════════════

        add_mapping("runtime.engine.list", "list_runtime_engines");
        add_mapping("runtime.engine.get", "get_runtime_engine");
        add_mapping("runtime.engine.capabilities", "get_runtime_capabilities");

        add_mapping("runtime.workload.submit", "submit_workload");
        add_mapping("runtime.workload.status", "get_workload_status");
        add_mapping("runtime.workload.result", "get_workload_result");
        add_mapping("runtime.workload.list", "list_workloads");

        // ═══════════════════════════════════════════════════════════
        // SCIENCE DOMAIN - Scientific compute IPC for springs
        // ═══════════════════════════════════════════════════════════

        add_mapping("science.compute.submit", "science_compute_submit");
        add_mapping("science.compute.status", "science_compute_status");
        add_mapping("science.compute.result", "science_compute_result");
        add_mapping("science.compute.cancel", "science_compute_cancel");

        add_mapping("science.gpu.dispatch", "science_gpu_dispatch");
        add_mapping("science.gpu.capabilities", "science_gpu_capabilities");

        add_mapping("science.npu.dispatch", "science_npu_dispatch");
        add_mapping("science.npu.capabilities", "science_npu_capabilities");

        add_mapping("science.substrate.discover", "science_substrate_discover");
        add_mapping("science.substrate.probe", "science_substrate_probe");

        // ═══════════════════════════════════════════════════════════
        // ECOLOGY DOMAIN - airSpring science offload routing
        //
        // Springs call these through toadStool as compute.offload
        // targets. toadStool routes to the appropriate science
        // primal discovered at runtime via capability sockets.
        // ═══════════════════════════════════════════════════════════

        add_mapping("ecology.et0_fao56", "ecology_et0_fao56");
        add_mapping("ecology.water_balance", "ecology_water_balance");
        add_mapping("ecology.yield_response", "ecology_yield_response");
        add_mapping("ecology.thornthwaite", "ecology_thornthwaite");
        add_mapping("ecology.gdd", "ecology_gdd");
        add_mapping("ecology.pedotransfer", "ecology_pedotransfer");
        add_mapping("ecology.spi_drought_index", "ecology_spi_drought_index");
        add_mapping("ecology.autocorrelation", "ecology_autocorrelation");
        add_mapping("ecology.gamma_cdf", "ecology_gamma_cdf");
        add_mapping("ecology.runoff_scs_cn", "ecology_runoff_scs_cn");
        add_mapping("ecology.van_genuchten_theta", "ecology_van_genuchten_theta");
        add_mapping("ecology.van_genuchten_k", "ecology_van_genuchten_k");
        add_mapping("ecology.bootstrap_ci", "ecology_bootstrap_ci");
        add_mapping("ecology.jackknife_ci", "ecology_jackknife_ci");

        // ═══════════════════════════════════════════════════════════
        // DISCOVERY DOMAIN - NUCLEUS primal discovery (groundSpring V99)
        //
        // Adaptive health checks and direct primal socket discovery
        // absorbed from groundSpring's live NUCLEUS integration.
        // ═══════════════════════════════════════════════════════════

        add_mapping("discovery.primals", "discovery_primals");
        add_mapping("discovery.primal_health", "discovery_primal_health");
        add_mapping("discovery.direct_rpc", "discovery_direct_rpc");
        add_mapping("discovery.topology", "discovery_topology");

        // ═══════════════════════════════════════════════════════════
        // DEPLOY DOMAIN - Science primal deploy graphs (wetSpring V99)
        //
        // Capability routing for science primal orchestration.
        // ═══════════════════════════════════════════════════════════

        add_mapping("deploy.capability_call", "deploy_capability_call");
        add_mapping("deploy.graph_status", "deploy_graph_status");

        // ═══════════════════════════════════════════════════════════
        // SHADER DOMAIN - Shader compilation IPC (coralReef pipeline)
        // ═══════════════════════════════════════════════════════════

        add_mapping("shader.compile.wgsl", "shader_compile_wgsl");
        add_mapping("shader.compile.spirv", "shader_compile_spirv");
        add_mapping("shader.compile.status", "shader_compile_status");
        add_mapping("shader.compile.capabilities", "shader_compile_capabilities");

        // ═══════════════════════════════════════════════════════════
        // SILICON DOMAIN - All-silicon pipeline / performance surface (S159)
        //
        // Springs report measured throughput for (op, unit, precision)
        // triples. toadStool stores and queries for tolerance routing.
        // ═══════════════════════════════════════════════════════════

        add_mapping(
            "compute.performance_surface.report",
            "performance_surface_report",
        );
        add_mapping(
            "compute.performance_surface.query",
            "performance_surface_query",
        );
        add_mapping(
            "compute.performance_surface.list",
            "performance_surface_list",
        );

        // ═══════════════════════════════════════════════════════════
        // PROVENANCE DOMAIN - Cross-spring evolution tracking
        // ═══════════════════════════════════════════════════════════

        add_mapping("toadstool.provenance", "toadstool_provenance"); // deprecated: primal name as domain
        add_mapping("provenance.get", "toadstool_provenance");
        add_mapping("provenance.query", "toadstool_provenance"); // canonical

        // ═══════════════════════════════════════════════════════════
        // INFERENCE DOMAIN - Model inference (capability, not product)
        // ═══════════════════════════════════════════════════════════

        add_mapping("ollama.list_models", "ollama_list_models"); // deprecated: product name as domain
        add_mapping("inference.list_models", "ollama_list_models"); // canonical
        add_mapping("ollama.inference", "ollama_inference"); // deprecated
        add_mapping("inference.execute", "ollama_inference"); // canonical
        add_mapping("ollama.load", "ollama_load"); // deprecated
        add_mapping("inference.load_model", "ollama_load"); // canonical
        add_mapping("ollama.unload", "ollama_unload"); // deprecated
        add_mapping("inference.unload_model", "ollama_unload"); // canonical

        // ═══════════════════════════════════════════════════════════
        // GPU DOMAIN - Query operations (verb form per naming standard)
        // ═══════════════════════════════════════════════════════════

        add_mapping("gpu.info", "gpu_info"); // deprecated: noun → verb
        add_mapping("gpu.query_info", "gpu_info"); // canonical
        add_mapping("gpu.memory", "gpu_memory"); // deprecated: noun → verb
        add_mapping("gpu.query_memory", "gpu_memory"); // canonical
        add_mapping("gpu.telemetry", "gpu_telemetry"); // deprecated: noun → verb
        add_mapping("gpu.query_telemetry", "gpu_telemetry"); // canonical

        Self { aliases, reverse }
    }

    /// Resolve semantic name to implementation name
    ///
    /// Returns the implementation method name if the semantic name is registered,
    /// otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use toadstool::semantic_methods::SemanticMethodRegistry;
    ///
    /// let registry = SemanticMethodRegistry::new();
    /// assert_eq!(registry.resolve("compute.execute"), Some("execute_workload"));
    /// assert_eq!(registry.resolve("unknown.method"), None);
    /// ```
    pub fn resolve(&self, semantic_name: &str) -> Option<&str> {
        self.aliases.get(semantic_name).map(|s| s.as_str())
    }

    /// Get semantic name for implementation method
    ///
    /// Returns the semantic method name if the implementation name is registered,
    /// otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use toadstool::semantic_methods::SemanticMethodRegistry;
    ///
    /// let registry = SemanticMethodRegistry::new();
    /// assert_eq!(registry.get_semantic("execute_workload"), Some("compute.execute"));
    /// ```
    pub fn get_semantic(&self, implementation_name: &str) -> Option<&str> {
        self.reverse.get(implementation_name).map(|s| s.as_str())
    }

    /// Check if method name is semantic (contains '.')
    ///
    /// # Examples
    ///
    /// ```
    /// use toadstool::semantic_methods::SemanticMethodRegistry;
    ///
    /// let registry = SemanticMethodRegistry::new();
    /// assert!(registry.is_semantic("compute.execute"));
    /// assert!(registry.is_semantic("resource.cpu.get_usage"));
    /// assert!(!registry.is_semantic("execute_workload"));
    /// ```
    pub fn is_semantic(&self, method_name: &str) -> bool {
        method_name.contains('.')
    }

    /// Check if semantic name is registered
    pub fn is_registered(&self, semantic_name: &str) -> bool {
        self.aliases.contains_key(semantic_name)
    }

    /// Get all registered semantic names
    pub fn semantic_names(&self) -> Vec<&str> {
        self.aliases.keys().map(|s| s.as_str()).collect()
    }

    /// Get all registered implementation names
    pub fn implementation_names(&self) -> Vec<&str> {
        self.reverse.keys().map(|s| s.as_str()).collect()
    }

    /// Get count of registered mappings
    pub fn count(&self) -> usize {
        self.aliases.len()
    }
}

impl Default for SemanticMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
    fn test_shader_domain_resolution() {
        let registry = SemanticMethodRegistry::new();

        assert_eq!(
            registry.resolve("shader.compile.wgsl"),
            Some("shader_compile_wgsl")
        );
        assert_eq!(
            registry.resolve("shader.compile.spirv"),
            Some("shader_compile_spirv")
        );
        assert_eq!(
            registry.resolve("shader.compile.status"),
            Some("shader_compile_status")
        );
        assert_eq!(
            registry.resolve("shader.compile.capabilities"),
            Some("shader_compile_capabilities")
        );
    }

    #[test]
    fn test_shader_domain_reverse() {
        let registry = SemanticMethodRegistry::new();

        assert_eq!(
            registry.get_semantic("shader_compile_wgsl"),
            Some("shader.compile.wgsl")
        );
        assert_eq!(
            registry.get_semantic("shader_compile_spirv"),
            Some("shader.compile.spirv")
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
}
