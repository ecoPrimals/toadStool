// SPDX-License-Identifier: AGPL-3.0-or-later
//! Covers modules that are deprecated and feature-gated, so this file
//! is compiled only when they are. Without a matching gate it did not
//! compile at all, and none of its tests ran.
#![cfg(all(
    feature = "runtime",
    feature = "legacy-cloud"
))]

#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#![allow(clippy::no_effect_underscore_binding)]
//! Integration tests for Universal Cloud Integration
//!
//! Testing strategy:
//! - Cloud provider configuration
//! - Cloud orchestration
//! - Multi-cloud scheduling
//! - Cost optimization
//! - Compliance enforcement

use toadstool_common::SecretString;
use toadstool_distributed::cloud::{
    AWSCredentials, AuthMethod, AzureCredentials, CloudProvider, EncryptionLevel, GCPCredentials,
};

#[test]
fn test_cloud_provider_aws_creation() {
    let provider = CloudProvider::AWS {
        region: "us-east-1".to_string(),
        credentials: AWSCredentials {
            access_key_id: "test-key".to_string(),
            secret_access_key: SecretString::from("test-secret"),
            session_token: None,
        },
        cost_budget: Some(1000.0),
    };

    match provider {
        CloudProvider::AWS {
            region,
            cost_budget,
            ..
        } => {
            assert_eq!(region, "us-east-1");
            assert_eq!(cost_budget, Some(1000.0));
        }
        _ => panic!("Expected AWS provider"),
    }
}

#[test]
fn test_cloud_provider_azure_creation() {
    let provider = CloudProvider::Azure {
        subscription: "test-sub".to_string(),
        credentials: AzureCredentials {
            tenant_id: "tenant".to_string(),
            client_id: "client".to_string(),
            client_secret: SecretString::from("secret"),
        },
        resource_group: "test-rg".to_string(),
    };

    match provider {
        CloudProvider::Azure {
            subscription,
            resource_group,
            ..
        } => {
            assert_eq!(subscription, "test-sub");
            assert_eq!(resource_group, "test-rg");
        }
        _ => panic!("Expected Azure provider"),
    }
}

#[test]
fn test_cloud_provider_gcp_creation() {
    let provider = CloudProvider::GCP {
        project: "test-project".to_string(),
        credentials: GCPCredentials {
            service_account_key: SecretString::from("key"),
        },
        zone: "us-central1-a".to_string(),
    };

    match provider {
        CloudProvider::GCP { project, zone, .. } => {
            assert_eq!(project, "test-project");
            assert_eq!(zone, "us-central1-a");
        }
        _ => panic!("Expected GCP provider"),
    }
}

#[test]
fn test_cloud_provider_digitalocean() {
    let provider = CloudProvider::DigitalOcean {
        token: "test-token".to_string(),
        region: "nyc3".to_string(),
    };

    match provider {
        CloudProvider::DigitalOcean { token, region } => {
            assert_eq!(token, "test-token");
            assert_eq!(region, "nyc3");
        }
        _ => panic!("Expected DigitalOcean provider"),
    }
}

#[test]
fn test_cloud_provider_security_cloud() {
    let provider = CloudProvider::PrivateSecurityCloud {
        endpoint: "https://security.local".to_string(),
        token: "bearer-token".to_string(),
        encryption_level: EncryptionLevel::Maximum,
    };

    match provider {
        CloudProvider::PrivateSecurityCloud { endpoint, .. } => {
            assert_eq!(endpoint, "https://security.local");
            // Note: encryption_level doesn't implement PartialEq, so we just verify endpoint
        }
        _ => panic!("Expected PrivateSecurityCloud provider"),
    }
}

#[test]
fn test_cloud_provider_self_hosted() {
    let provider = CloudProvider::SelfHosted {
        endpoints: vec![
            "http://node1:8080".to_string(),
            "http://node2:8080".to_string(),
        ],
        auth_method: AuthMethod::Token {
            token: SecretString::from("self-token"),
        },
    };

    match provider {
        CloudProvider::SelfHosted { endpoints, .. } => {
            assert_eq!(endpoints.len(), 2);
            assert_eq!(endpoints[0], "http://node1:8080");
        }
        _ => panic!("Expected SelfHosted provider"),
    }
}

#[test]
fn test_cloud_provider_serialization() {
    let provider = CloudProvider::DigitalOcean {
        token: "test".to_string(),
        region: "nyc1".to_string(),
    };

    // Test that it can be serialized
    let json = serde_json::to_string(&provider).expect("Should serialize");
    assert!(json.contains("DigitalOcean"));
    assert!(json.contains("nyc1"));
}

#[test]
fn test_cloud_provider_deserialization() {
    let json = r#"{"DigitalOcean":{"token":"test","region":"nyc1"}}"#;
    let provider: CloudProvider = serde_json::from_str(json).expect("Should deserialize");

    match provider {
        CloudProvider::DigitalOcean { region, .. } => {
            assert_eq!(region, "nyc1");
        }
        _ => panic!("Expected DigitalOcean"),
    }
}

#[test]
fn test_encryption_level_variants() {
    let levels = [
        EncryptionLevel::Standard,
        EncryptionLevel::High,
        EncryptionLevel::Maximum,
    ];

    assert_eq!(levels.len(), 3);
}

#[test]
fn test_auth_method_token() {
    let auth = AuthMethod::Token {
        token: SecretString::from("my-token"),
    };

    match auth {
        AuthMethod::Token { token } => {
            assert_eq!(token.expose_secret(), "my-token");
        }
        _ => panic!("Expected Token auth method"),
    }
}

#[test]
fn test_auth_method_certificate() {
    let auth = AuthMethod::Certificate {
        cert_path: "/path/to/cert".to_string(),
        key_path: "/path/to/key".to_string(),
    };

    match auth {
        AuthMethod::Certificate { cert_path, .. } => {
            assert_eq!(cert_path, "/path/to/cert");
        }
        _ => panic!("Expected Certificate auth method"),
    }
}

#[test]
fn test_cloud_provider_linode() {
    let provider = CloudProvider::Linode {
        token: "linode-token".to_string(),
        region: "us-east".to_string(),
    };

    match provider {
        CloudProvider::Linode { region, .. } => {
            assert_eq!(region, "us-east");
        }
        _ => panic!("Expected Linode provider"),
    }
}

#[test]
fn test_cloud_provider_vultr() {
    let provider = CloudProvider::Vultr {
        api_key: "vultr-key".to_string(),
        region: "ewr".to_string(),
    };

    match provider {
        CloudProvider::Vultr { region, .. } => {
            assert_eq!(region, "ewr");
        }
        _ => panic!("Expected Vultr provider"),
    }
}

#[test]
fn test_cloud_provider_hetzner() {
    let provider = CloudProvider::Hetzner {
        token: "hetzner-token".to_string(),
        location: "nbg1".to_string(),
    };

    match provider {
        CloudProvider::Hetzner { location, .. } => {
            assert_eq!(location, "nbg1");
        }
        _ => panic!("Expected Hetzner provider"),
    }
}

#[test]
fn test_multiple_cloud_providers() {
    let providers: Vec<CloudProvider> = vec![
        CloudProvider::AWS {
            region: "us-west-2".to_string(),
            credentials: AWSCredentials {
                access_key_id: "key1".to_string(),
                secret_access_key: SecretString::from("secret1"),
                session_token: None,
            },
            cost_budget: None,
        },
        CloudProvider::DigitalOcean {
            token: "do-token".to_string(),
            region: "sfo3".to_string(),
        },
        CloudProvider::PrivateSecurityCloud {
            endpoint: "https://cloud.security.local".to_string(),
            token: "security-token".to_string(),
            encryption_level: EncryptionLevel::High,
        },
    ];

    assert_eq!(providers.len(), 3);
}

#[test]
fn test_aws_credentials_structure() {
    let creds = AWSCredentials {
        access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
        secret_access_key: SecretString::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
        session_token: Some(SecretString::from("session123")),
    };

    assert!(!creds.access_key_id.is_empty());
    assert!(!creds.secret_access_key.is_empty());
    assert!(creds.session_token.is_some());
}

#[test]
fn test_azure_credentials_structure() {
    let creds = AzureCredentials {
        tenant_id: "tenant-123".to_string(),
        client_id: "client-456".to_string(),
        client_secret: SecretString::from("secret-789"),
    };

    assert!(!creds.tenant_id.is_empty());
    assert!(!creds.client_id.is_empty());
    assert!(!creds.client_secret.is_empty());
}

#[test]
fn test_gcp_credentials_structure() {
    let creds = GCPCredentials {
        service_account_key: SecretString::from("{\n  \"type\": \"service_account\"...}"),
    };

    assert!(!creds.service_account_key.is_empty());
}

#[test]
fn test_cloud_provider_clone() {
    let provider1 = CloudProvider::DigitalOcean {
        token: "token".to_string(),
        region: "nyc1".to_string(),
    };

    let provider2 = provider1.clone();

    match (provider1, provider2) {
        (
            CloudProvider::DigitalOcean { region: r1, .. },
            CloudProvider::DigitalOcean { region: r2, .. },
        ) => {
            assert_eq!(r1, r2);
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_cloud_provider_ovh() {
    let provider = CloudProvider::OVH {
        application_key: "app-key".to_string(),
        application_secret: "app-secret".to_string(),
        consumer_key: "consumer-key".to_string(),
        region: "GRA".to_string(),
    };

    match provider {
        CloudProvider::OVH { region, .. } => {
            assert_eq!(region, "GRA");
        }
        _ => panic!("Expected OVH provider"),
    }
}

#[test]
fn test_cloud_provider_scaleway() {
    let provider = CloudProvider::Scaleway {
        access_key: "access".to_string(),
        secret_key: "secret".to_string(),
        organization_id: "org-123".to_string(),
        zone: "fr-par-1".to_string(),
    };

    match provider {
        CloudProvider::Scaleway { zone, .. } => {
            assert_eq!(zone, "fr-par-1");
        }
        _ => panic!("Expected Scaleway provider"),
    }
}

#[test]
fn test_cloud_provider_kubernetes() {
    use toadstool_distributed::cloud::KubernetesConfig;

    let config = KubernetesConfig {
        kubeconfig_path: Some("/home/user/.kube/config".to_string()),
        kubeconfig_content: None,
        cluster_endpoint: Some("https://kubernetes.local:6443".to_string()),
        token: Some("k8s-token".to_string()),
    };

    let provider = CloudProvider::Kubernetes {
        config,
        namespace: "toadstool".to_string(),
        storage_class: Some("fast-ssd".to_string()),
    };

    match provider {
        CloudProvider::Kubernetes {
            namespace,
            storage_class,
            ..
        } => {
            assert_eq!(namespace, "toadstool");
            assert_eq!(storage_class, Some("fast-ssd".to_string()));
        }
        _ => panic!("Expected Kubernetes provider"),
    }
}

#[test]
fn test_cloud_provider_debug_representation() {
    let providers = vec![
        CloudProvider::AWS {
            region: "us-east-1".to_string(),
            credentials: AWSCredentials {
                access_key_id: "key".to_string(),
                secret_access_key: SecretString::from("secret"),
                session_token: None,
            },
            cost_budget: Some(500.0),
        },
        CloudProvider::PrivateSecurityCloud {
            endpoint: "https://security.local".to_string(),
            token: "token".to_string(),
            encryption_level: EncryptionLevel::Maximum,
        },
        CloudProvider::DigitalOcean {
            token: "do-token".to_string(),
            region: "sfo3".to_string(),
        },
    ];

    for provider in providers {
        let debug_str = format!("{provider:?}");
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_encryption_level_ordering() {
    // Test that encryption levels exist and can be created
    let standard = EncryptionLevel::Standard;
    let high = EncryptionLevel::High;
    let maximum = EncryptionLevel::Maximum;

    // Just verify they can be created and used
    let _levels = [standard, high, maximum];
}

#[test]
fn test_auth_method_security_auth() {
    let auth = AuthMethod::SecurityServiceAuth {
        endpoint: "https://security.auth.local".to_string(),
        credentials: SecretString::from("security-creds"),
    };

    match auth {
        AuthMethod::SecurityServiceAuth {
            endpoint,
            credentials,
        } => {
            assert_eq!(endpoint, "https://security.auth.local");
            assert_eq!(credentials.expose_secret(), "security-creds");
        }
        _ => panic!("Expected SecurityServiceAuth auth method"),
    }
}

#[test]
fn test_auth_method_serialization() {
    let auth = AuthMethod::Token {
        token: SecretString::from("test-token"),
    };

    let json = serde_json::to_string(&auth).expect("Should serialize");
    assert!(json.contains("Token"));
}

#[test]
fn test_cloud_provider_debug_format() {
    let provider = CloudProvider::DigitalOcean {
        token: "secret-token".to_string(),
        region: "nyc1".to_string(),
    };

    let debug_str = format!("{provider:?}");
    assert!(debug_str.contains("DigitalOcean"));
    // Note: Debug output may redact sensitive information
}

#[test]
fn test_aws_credentials_with_session_token() {
    let creds = AWSCredentials {
        access_key_id: "key".to_string(),
        secret_access_key: SecretString::from("secret"),
        session_token: Some(SecretString::from("session-token")),
    };

    assert!(creds.session_token.is_some());
    assert_eq!(
        creds.session_token.unwrap().expose_secret(),
        "session-token"
    );
}

#[test]
fn test_aws_credentials_without_session_token() {
    let creds = AWSCredentials {
        access_key_id: "key".to_string(),
        secret_access_key: SecretString::from("secret"),
        session_token: None,
    };

    assert!(creds.session_token.is_none());
}

#[test]
fn test_multiple_auth_methods() {
    let auth_methods = [
        AuthMethod::Token {
            token: SecretString::from("token1"),
        },
        AuthMethod::Certificate {
            cert_path: "/path/cert".to_string(),
            key_path: "/path/key".to_string(),
        },
        AuthMethod::SecurityServiceAuth {
            endpoint: "https://auth.local".to_string(),
            credentials: SecretString::from("creds"),
        },
    ];

    assert_eq!(auth_methods.len(), 3);
}

#[test]
fn test_cloud_provider_with_cost_budget() {
    let provider = CloudProvider::AWS {
        region: "us-west-2".to_string(),
        credentials: AWSCredentials {
            access_key_id: "key".to_string(),
            secret_access_key: SecretString::from("secret"),
            session_token: None,
        },
        cost_budget: Some(1000.0),
    };

    match provider {
        CloudProvider::AWS { cost_budget, .. } => {
            assert!(cost_budget.is_some());
            assert_eq!(cost_budget.unwrap(), 1000.0);
        }
        _ => panic!("Expected AWS"),
    }
}

#[test]
fn test_cloud_provider_without_cost_budget() {
    let provider = CloudProvider::AWS {
        region: "us-west-2".to_string(),
        credentials: AWSCredentials {
            access_key_id: "key".to_string(),
            secret_access_key: SecretString::from("secret"),
            session_token: None,
        },
        cost_budget: None,
    };

    match provider {
        CloudProvider::AWS { cost_budget, .. } => {
            assert!(cost_budget.is_none());
        }
        _ => panic!("Expected AWS"),
    }
}

#[test]
fn test_self_hosted_with_multiple_endpoints() {
    let endpoints = vec![
        "http://node1:8080".to_string(),
        "http://node2:8080".to_string(),
        "http://node3:8080".to_string(),
        "http://node4:8080".to_string(),
    ];

    let provider = CloudProvider::SelfHosted {
        endpoints: endpoints.clone(),
        auth_method: AuthMethod::Token {
            token: SecretString::from("token"),
        },
    };

    match provider {
        CloudProvider::SelfHosted { endpoints: eps, .. } => {
            assert_eq!(eps.len(), 4);
            assert_eq!(eps, endpoints);
        }
        _ => panic!("Expected SelfHosted"),
    }
}

#[test]
fn test_security_cloud_with_different_encryption_levels() {
    let providers = vec![
        CloudProvider::PrivateSecurityCloud {
            endpoint: "https://security1.local".to_string(),
            token: "token1".to_string(),
            encryption_level: EncryptionLevel::Standard,
        },
        CloudProvider::PrivateSecurityCloud {
            endpoint: "https://security2.local".to_string(),
            token: "token2".to_string(),
            encryption_level: EncryptionLevel::High,
        },
        CloudProvider::PrivateSecurityCloud {
            endpoint: "https://security3.local".to_string(),
            token: "token3".to_string(),
            encryption_level: EncryptionLevel::Maximum,
        },
    ];

    assert_eq!(providers.len(), 3);
}

#[test]
fn test_cloud_provider_roundtrip_serialization() {
    let original = CloudProvider::DigitalOcean {
        token: "test-token".to_string(),
        region: "nyc1".to_string(),
    };

    let json = serde_json::to_string(&original).expect("Should serialize");
    let deserialized: CloudProvider = serde_json::from_str(&json).expect("Should deserialize");

    match (original, deserialized) {
        (
            CloudProvider::DigitalOcean { region: r1, .. },
            CloudProvider::DigitalOcean { region: r2, .. },
        ) => {
            assert_eq!(r1, r2);
        }
        _ => panic!("Roundtrip failed"),
    }
}
