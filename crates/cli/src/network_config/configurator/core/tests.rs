// SPDX-License-Identifier: AGPL-3.0-only

use super::defaults::{songbird_default_network_config, system_dns_resolvers};
use crate::network_config::SongbirdNetworkConfigurator;

#[test]
fn default_config_enables_service_mesh_by_default() {
    let cfg = songbird_default_network_config();
    assert!(cfg.service_mesh.enabled);
    assert_eq!(cfg.service_mesh.mesh_type, "native");
}

#[test]
fn default_config_dns_discovery_lists_search_domains() {
    let cfg = songbird_default_network_config();
    assert!(cfg.dns_discovery.enabled);
    assert!(
        cfg.dns_discovery
            .search_domains
            .iter()
            .any(|d| d.contains("toadstool.local"))
    );
}

#[test]
fn default_config_health_endpoints_cover_core_capabilities() {
    let cfg = songbird_default_network_config();
    let names: Vec<&str> = cfg
        .health_monitoring
        .endpoints
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert!(names.contains(&"orchestration"));
    assert!(names.contains(&"pki"));
    assert!(names.contains(&"storage"));
    assert!(names.contains(&"ai"));
    assert_eq!(cfg.health_monitoring.endpoints.len(), 4);
}

#[test]
fn system_dns_resolvers_honors_env_override() {
    temp_env::with_var("TOADSTOOL_DNS_RESOLVERS", Some("8.8.8.8, 1.1.1.1"), || {
        let r = system_dns_resolvers();
        assert_eq!(r, vec!["8.8.8.8", "1.1.1.1"]);
    });
}

#[test]
fn system_dns_resolvers_empty_env_falls_through() {
    temp_env::with_var("TOADSTOOL_DNS_RESOLVERS", Some(""), || {
        let r = system_dns_resolvers();
        assert!(r.is_empty() || !r.iter().all(|s| s.is_empty()));
    });
}

#[test]
fn new_configurator_uses_default_config_shape() {
    let c = SongbirdNetworkConfigurator::new();
    assert!(c.config.service_mesh.enabled);
    assert!(c.config.dns_discovery.enabled);
    assert_eq!(
        c.config.health_monitoring.endpoints.len(),
        songbird_default_network_config()
            .health_monitoring
            .endpoints
            .len()
    );
}

#[test]
fn configuration_summary_reflects_mesh_toggle() {
    let mut on = SongbirdNetworkConfigurator::new();
    on.config.service_mesh.enabled = true;
    assert!(on.generate_configuration_summary().contains("enabled"));

    let mut off = SongbirdNetworkConfigurator::new();
    off.config.service_mesh.enabled = false;
    let s = off.generate_configuration_summary();
    assert!(s.contains("disabled"));
    assert!(s.contains("Songbird Network Configuration Summary"));
}

#[test]
fn default_network_policies_have_expected_ingress_name() {
    let cfg = songbird_default_network_config();
    assert!(cfg.network_policies.enabled);
    assert!(
        cfg.network_policies
            .ingress_rules
            .iter()
            .any(|r| r.name == "allow-intra-mesh")
    );
}

#[test]
fn default_traffic_management_canary_percentage_is_nonzero() {
    let cfg = songbird_default_network_config();
    assert!(cfg.traffic_management.enabled);
    assert!(cfg.traffic_management.canary.percentage > 0);
}
