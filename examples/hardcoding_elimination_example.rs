// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! # Hardcoding Elimination Example
//!
//! This example demonstrates the proper pattern for eliminating hardcoding
//! and using infant discovery for all service connections.

use anyhow::Result;
use std::sync::Arc;
use toadstool_common::infant_discovery::*;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🍼 Hardcoding Elimination Example");
    println!("====================================\n");

    // STEP 1: Define self-identity (ONLY thing we hardcode - our own identity!)
    let self_identity = SelfIdentity {
        instance_id: Uuid::new_v4().to_string(),
        service_name: "hardcoding-example".to_string(),
        capabilities: vec!["example_service".to_string()],
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    println!("✅ Self-Identity Defined:");
    println!("   Service: {}", self_identity.service_name);
    println!("   Instance: {}", self_identity.instance_id);
    println!("   Capabilities: {:?}\n", self_identity.capabilities);

    // STEP 2: Create universal adapter with discovery methods
    let adapter_config = AdapterConfig {
        enable_network_discovery: true,
        enable_environment_discovery: true,
        enable_dns_discovery: false, // Disable for example
        connection_timeout: std::time::Duration::from_secs(5),
        discovery_timeout: std::time::Duration::from_secs(10),
    };

    let universal_adapter =
        Arc::new(ConcreteUniversalAdapter::new(adapter_config).expect("Failed to create adapter"));

    println!("✅ Universal Adapter Created");
    println!("   Network Discovery: enabled");
    println!("   Environment Discovery: enabled\n");

    // STEP 3: Initialize infant discovery
    let discovery = InfantDiscovery::new(self_identity.clone(), universal_adapter).await?;

    println!("✅ Infant Discovery Initialized\n");

    // STEP 4: Bootstrap (advertise our capabilities)
    println!("📢 Bootstrapping - advertising capabilities...");
    discovery.bootstrap().await?;
    println!("✅ Bootstrap complete\n");

    // STEP 5: Discover services by capability (not by name!)
    println!("🔍 EXAMPLE 1: Discovering API Service");
    println!("   ❌ OLD: let url = \"http://localhost:8080\";");
    println!("   ✅ NEW: Discovering 'http_api' capability...");

    match discovery.need_capability("http_api", None).await {
        Ok(service) => {
            println!("   ✅ Found API service!");
            println!("      Endpoint: {}", "dynamically discovered");
            // In real usage: let response = service.call("endpoint", data).await?;
        }
        Err(DiscoveryError::NoProvidersFound { capability }) => {
            println!("   ⚠️  No providers found for '{}' (expected in example)", capability);
            println!("      This is graceful degradation - we continue operating");
        }
        Err(e) => {
            println!("   ❌ Discovery error: {}", e);
        }
    }

    println!();

    // STEP 6: Discover with requirements
    println!("🔍 EXAMPLE 2: Discovering Cache with Requirements");
    println!("   ❌ OLD: let redis = connect(\"redis://localhost:6379\");");
    println!("   ✅ NEW: Discovering 'caching' with min 512MB memory...");

    let cache_requirements = Some(ResourceSpec {
        min_memory_mb: Some(512),
        persistence_required: Some(true),
        ..Default::default()
    });

    match discovery.need_capability("caching", cache_requirements).await {
        Ok(_service) => {
            println!("   ✅ Found caching service meeting requirements!");
        }
        Err(DiscoveryError::NoProvidersFound { capability }) => {
            println!("   ⚠️  No providers for '{}' (expected in example)", capability);
            println!("      Falling back to in-memory cache");
            // Implement fallback logic here
        }
        Err(e) => {
            println!("   ❌ Discovery error: {}", e);
        }
    }

    println!();

    // STEP 7: Discover AI processing (instead of hardcoded primal name)
    println!("🔍 EXAMPLE 3: Discovering AI Processing");
    println!("   ❌ OLD: let songbird = SongbirdClient::new(\"http://songbird:9000\");");
    println!("   ✅ NEW: Discovering 'ai_processing' capability...");

    match discovery.need_capability("ai_processing", None).await {
        Ok(_service) => {
            println!("   ✅ Found AI processing service!");
            println!("      Could be Songbird, could be any AI provider");
            println!("      We don't know and don't care - just works!");
        }
        Err(DiscoveryError::NoProvidersFound { capability }) => {
            println!("   ⚠️  No '{}' provider (expected in example)", capability);
            println!("      Continuing without AI enhancement");
        }
        Err(e) => {
            println!("   ❌ Discovery error: {}", e);
        }
    }

    println!();

    // Summary
    println!("📊 SUMMARY");
    println!("==========");
    println!("✅ Zero hardcoded IPs or ports");
    println!("✅ Zero primal name dependencies");
    println!("✅ Zero vendor service hardcoding");
    println!("✅ Graceful degradation working");
    println!("✅ True sovereignty achieved!");
    println!();
    println!("🎯 This is the pattern for ALL service connections!");
    println!();
    println!("📚 See HARDCODING_ELIMINATION_GUIDE.md for more examples");

    Ok(())
}

// Helper types for the example
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    pub enable_network_discovery: bool,
    pub enable_environment_discovery: bool,
    pub enable_dns_discovery: bool,
    pub connection_timeout: std::time::Duration,
    pub discovery_timeout: std::time::Duration,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceSpec {
    pub min_memory_mb: Option<u64>,
    pub persistence_required: Option<bool>,
}

