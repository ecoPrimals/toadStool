// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn test_capability_creation() {
    let gpu_cap = Capability::compute_gpu();
    assert_eq!(gpu_cap.id, "compute_gpu");
    assert!(gpu_cap.resource_requirements.gpu_required);
}

#[test]
fn test_registry() {
    let capabilities = vec![Capability::compute_gpu(), Capability::compute_heavy()];

    let registry = CapabilityRegistry::new(capabilities);
    assert_eq!(registry.all_capabilities().len(), 2);
}

#[test]
fn test_capability_update() {
    let capabilities = vec![Capability::compute_gpu()];
    let mut registry = CapabilityRegistry::new(capabilities);

    assert!(!registry.is_available("compute_gpu"));

    let gpu_cap = Capability::compute_gpu();
    registry.update_capability(gpu_cap, true).unwrap();

    assert!(registry.is_available("compute_gpu"));
}

#[test]
fn test_all_capability_constructors() {
    let gpu = Capability::compute_gpu();
    assert_eq!(gpu.id, "compute_gpu");
    assert!(gpu.resource_requirements.gpu_required);

    let heavy = Capability::compute_heavy();
    assert_eq!(heavy.id, "compute_heavy");
    assert!(!heavy.resource_requirements.gpu_required);

    let ml = Capability::compute_ml_training();
    assert_eq!(ml.id, "compute_ml_training");
    assert!(ml.resource_requirements.gpu_required);

    let native = Capability::compute_native();
    assert_eq!(native.id, "compute_native");
    assert!(native.available);

    let container = Capability::compute_container();
    assert_eq!(container.id, "compute_container");

    let wasm = Capability::compute_wasm();
    assert_eq!(wasm.id, "compute_wasm");

    let mainframe = Capability::compute_mainframe();
    assert_eq!(mainframe.id, "compute_mainframe");
    assert!(!mainframe.available);

    let embedded = Capability::compute_embedded();
    assert_eq!(embedded.id, "compute_embedded");
}

#[test]
fn test_registry_add_remove_capability() {
    let mut registry = CapabilityRegistry::new(vec![Capability::compute_native()]);
    assert_eq!(registry.all_capabilities().len(), 1);

    registry.add_capability(Capability::compute_wasm());
    assert_eq!(registry.all_capabilities().len(), 2);

    let removed = registry.remove_capability("compute_native");
    assert!(removed.is_some());
    assert_eq!(registry.all_capabilities().len(), 1);
    assert!(registry.get_capability("compute_native").is_none());
}

#[test]
fn test_registry_available_filter() {
    let caps = vec![
        Capability::compute_gpu(),    // available: false
        Capability::compute_heavy(),  // available: true
        Capability::compute_native(), // available: true
    ];
    let registry = CapabilityRegistry::new(caps);
    let available = registry.available_capabilities();
    assert_eq!(available.len(), 2);
}

#[test]
fn test_capability_resources_serde() {
    let res = CapabilityResources {
        min_cpu_cores: 4,
        min_memory_mb: 8192,
        gpu_required: true,
        gpu_memory_mb: Some(4096),
        special_hardware: vec!["cuda".to_string()],
    };
    let json = serde_json::to_string(&res).unwrap();
    let parsed: CapabilityResources = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.min_cpu_cores, 4);
    assert_eq!(parsed.gpu_memory_mb, Some(4096));
}

// ── ProviderRegistry tests ────────────────────────────────

fn make_registration(cap: &str, name: &str) -> ProviderRegistration {
    ProviderRegistration {
        capability: cap.to_string(),
        socket_path: std::path::PathBuf::from(format!("/tmp/biomeos/{cap}.sock")),
        methods: vec![format!("{cap}.list"), format!("{cap}.query")],
        provider_name: name.to_string(),
        provider_version: "1.0.0".to_string(),
        registered_at: 1_710_000_000,
    }
}

#[test]
fn test_provider_registry_register_and_lookup() {
    let mut reg = ProviderRegistry::new();
    reg.register(make_registration("biology", "wetSpring"));

    assert!(reg.has_provider("biology"));
    assert!(!reg.has_provider("ecology"));

    let provider = reg.get_provider("biology").unwrap();
    assert_eq!(provider.provider_name, "wetSpring");
    assert_eq!(provider.methods.len(), 2);
}

#[test]
fn test_provider_registry_deregister() {
    let mut reg = ProviderRegistry::new();
    reg.register(make_registration("biology", "wetSpring"));
    assert!(reg.has_provider("biology"));

    let removed = reg.deregister("biology");
    assert!(removed.is_some());
    assert!(!reg.has_provider("biology"));
}

#[test]
fn test_provider_registry_replace() {
    let mut reg = ProviderRegistry::new();
    reg.register(make_registration("biology", "wetSpring-v1"));
    reg.register(make_registration("biology", "wetSpring-v2"));

    let provider = reg.get_provider("biology").unwrap();
    assert_eq!(provider.provider_name, "wetSpring-v2");
}

#[test]
fn test_provider_registry_all_providers() {
    let mut reg = ProviderRegistry::new();
    reg.register(make_registration("biology", "wetSpring"));
    reg.register(make_registration("ecology", "airSpring"));

    let all = reg.all_providers();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_provider_registry_resolve_falls_back() {
    let reg = ProviderRegistry::new();
    let path = reg.resolve_socket("biology");
    assert!(path.to_str().unwrap().contains("biology.sock"));
}

#[test]
fn test_provider_registry_resolve_prefers_explicit() {
    let mut reg = ProviderRegistry::new();
    reg.register(make_registration("biology", "wetSpring"));

    let path = reg.resolve_socket("biology");
    assert_eq!(path, std::path::PathBuf::from("/tmp/biomeos/biology.sock"));
}

#[test]
fn test_provider_registry_prune_stale() {
    let mut reg = ProviderRegistry::new();
    reg.register(ProviderRegistration {
        capability: "nonexistent".to_string(),
        socket_path: std::path::PathBuf::from("/tmp/definitely_not_existing_socket.sock"),
        methods: vec![],
        provider_name: "ghost".to_string(),
        provider_version: "0.0.0".to_string(),
        registered_at: 0,
    });
    assert!(reg.has_provider("nonexistent"));

    let stale = reg.prune_stale();
    assert_eq!(stale.len(), 1);
    assert!(!reg.has_provider("nonexistent"));
}

#[test]
fn test_provider_registration_serde() {
    let reg = make_registration("health", "healthSpring");
    let json = serde_json::to_string(&reg).unwrap();
    let parsed: ProviderRegistration = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.capability, "health");
    assert_eq!(parsed.provider_name, "healthSpring");
}
