// Core EcosystemIntegrator implementation - refactored by protocol
//
// This file contains the main orchestration logic, delegating to
// service-specific modules for protocol details.

use connection::get_local_address;
use discovery::*;

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

impl EcosystemIntegrator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            connections: HashMap::new(),
            credentials: None,
        }
    }

    /// Discover ecosystem services on the network
    pub async fn discover_services(
        &mut self,
        service_types: Vec<String>,
        timeout_secs: u64,
    ) -> Result<DiscoveryResult> {
        info!("🔍 Scanning for ecosystem services");
        let start_time = std::time::Instant::now();

        // Load known service ports
        let service_ports = get_standard_service_ports();

        // Perform network discovery
        let mut discovered_services = Vec::new();

        // If no specific services requested, scan for all
        let scan_services = if service_types.is_empty() {
            vec![
                "songbird".to_string(),
                "beardog".to_string(),
                "nestgate".to_string(),
            ]
        } else {
            service_types
        };

        for service_type in &scan_services {
            info!("🔍 Scanning for {}", service_type);

            let services = timeout(
                Duration::from_secs(timeout_secs),
                scan_for_service(service_type, &service_ports),
            )
            .await
            .with_context(|| format!("Timeout scanning for {service_type}"))?
            .with_context(|| format!("Failed to scan for {service_type}"))?;

            for service in services {
                info!(
                    "✅ Discovered {}: {}:{}",
                    service_type,
                    service.address.ip(),
                    service.address.port()
                );
                discovered_services.push(service);
            }
        }

        // Verify discovered services
        let mut verified_count = 0;
        for service in &mut discovered_services {
            match verify_service(service).await {
                Ok(true) => {
                    service.trust_level = TrustLevel::Verified;
                    verified_count += 1;
                }
                Ok(false) => {
                    warn!("⚠️  Service verification failed: {}", service.address);
                    service.trust_level = TrustLevel::Discovered;
                }
                Err(e) => {
                    warn!("⚠️  Service verification error: {}", e);
                    service.trust_level = TrustLevel::Unknown;
                }
            }
        }

        // Store discovered services
        for service in &discovered_services {
            let key = format!("{}:{}", service.service_type.name(), service.address);
            self.endpoints.insert(key, service.clone());
        }

        let scan_duration = start_time.elapsed();

        info!(
            "🎯 Discovery complete: {} services found, {} verified",
            discovered_services.len(),
            verified_count
        );

        let total_count = discovered_services.len();
        Ok(DiscoveryResult {
            services: discovered_services,
            scan_duration: Duration::from_secs(scan_duration.as_secs()),
            total_discovered: total_count,
            verified_count,
        })
    }

    /// Register with orchestrator via capability discovery
    pub async fn register_with_orchestrator(
        &mut self,
        endpoint: String,
        token: Option<String>,
    ) -> Result<()> {
        info!("🎯 Registering with orchestrator: {}", endpoint);

        // Parse endpoint
        let addr: SocketAddr = endpoint
            .parse()
            .with_context(|| format!("Invalid Songbird endpoint: {endpoint}"))?;

        // Create registration payload
        let registration = SongbirdRegistration {
            service_name: "toadstool".to_string(),
            service_type: "universal-compute".to_string(),
            address: get_local_address()?,
            capabilities: vec![
                "wasm-execution".to_string(),
                "container-runtime".to_string(),
                "universal-substrate".to_string(),
                "sovereign-compute".to_string(),
            ],
            metadata: vec![
                ("version".to_string(), "0.1.0".to_string()),
                ("platform".to_string(), std::env::consts::OS.to_string()),
                ("arch".to_string(), std::env::consts::ARCH.to_string()),
            ]
            .into_iter()
            .collect(),
            auth_token: token.clone(),
        };

        // Attempt registration using Songbird service module
        match services::songbird::send_registration(&addr, &registration).await {
            Ok(response) => {
                info!("✅ Successfully registered with Songbird");
                info!("   Service ID: {}", response.service_id);
                info!("   Registry URL: {}", response.registry_url);

                // Store connection
                let connection = ServiceConnection {
                    endpoint: ServiceEndpoint {
                        service_type: EcosystemService::Songbird,
                        address: addr,
                        version: "unknown".to_string(),
                        capabilities: vec!["discovery".to_string(), "coordination".to_string()],
                        trust_level: TrustLevel::Verified,
                    },
                    status: ConnectionStatus::Connected,
                    last_heartbeat: Utc::now(),
                    _auth_token: token,
                };

                self.connections.insert("songbird".to_string(), connection);
                Ok(())
            }
            Err(e) => {
                warn!("⚠️  Failed to register with Songbird: {}", e);
                Err(e)
            }
        }
    }

    /// Install BearDog cryptographic permissions
    pub async fn install_beardog_permissions(
        &mut self,
        permissions_path: PathBuf,
        validate_only: bool,
    ) -> Result<()> {
        // Delegate to BearDog service module
        services::beardog::install_permissions(&permissions_path, validate_only).await
    }

    /// Connect to NestGate distributed storage
    pub async fn connect_nestgate_storage(
        &mut self,
        endpoint: String,
        mount_point: PathBuf,
        dataset: Option<String>,
    ) -> Result<NestGateMount> {
        info!("🏠 Connecting to NestGate storage: {}", endpoint);

        // Parse endpoint
        let addr: SocketAddr = endpoint
            .parse()
            .with_context(|| format!("Invalid NestGate endpoint: {endpoint}"))?;

        // Delegate to NestGate service module
        let mount_info = services::nestgate::connect_storage(&addr, &mount_point, dataset.as_deref()).await?;

        // Store connection
        let connection = ServiceConnection {
            endpoint: ServiceEndpoint {
                service_type: EcosystemService::NestGate,
                address: addr,
                version: "unknown".to_string(),
                capabilities: vec!["storage".to_string(), "zfs".to_string()],
                trust_level: TrustLevel::Verified,
            },
            status: ConnectionStatus::Connected,
            last_heartbeat: Utc::now(),
            _auth_token: None,
        };

        self.connections.insert("nestgate".to_string(), connection);

        Ok(mount_info)
    }

    /// Show ecosystem connection status
    pub async fn show_ecosystem_status(&self, format: String) -> Result<()> {
        match format.as_str() {
            "json" => {
                let status = EcosystemStatus {
                    endpoints: self.endpoints.clone(),
                    active_connections: self.connections.len(),
                    total_discovered: self.endpoints.len(),
                    credential_status: self.credentials.is_some(),
                };
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            "table" => {
                self.print_ecosystem_table().await?;
            }
            _ => {
                self.print_ecosystem_table().await?;
            }
        }

        Ok(())
    }

    // Internal helper methods

    async fn print_ecosystem_table(&self) -> Result<()> {
        if self.endpoints.is_empty() && self.connections.is_empty() {
            println!("No ecosystem services discovered or connected");
            return Ok(());
        }

        println!("\n🌐 Ecosystem Services Status:");
        println!("{}", "=".repeat(80));

        if !self.endpoints.is_empty() {
            println!("\n📡 Discovered Services:");
            println!(
                "{:<20} {:<20} {:<15} {:<15}",
                "SERVICE", "ADDRESS", "TRUST", "CAPABILITIES"
            );
            println!("{}", "-".repeat(70));

            for endpoint in self.endpoints.values() {
                println!(
                    "{:<20} {:<20} {:<15} {:<15}",
                    endpoint.service_type.name(),
                    endpoint.address.to_string(),
                    format!("{:?}", endpoint.trust_level),
                    endpoint.capabilities.join(",")
                );
            }
        }

        if !self.connections.is_empty() {
            println!("\n🔗 Active Connections:");
            println!(
                "{:<20} {:<20} {:<15} {:<20}",
                "SERVICE", "ADDRESS", "STATUS", "LAST HEARTBEAT"
            );
            println!("{}", "-".repeat(75));

            for connection in self.connections.values() {
                println!(
                    "{:<20} {:<20} {:<15} {:<20}",
                    connection.endpoint.service_type.name(),
                    connection.endpoint.address.to_string(),
                    format!("{:?}", connection.status),
                    connection
                        .last_heartbeat
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                );
            }
        }

        if let Some(creds) = &self.credentials {
            println!("\n🔐 Authentication Status:");
            println!("   Identity: {}", creds.identity);
            println!("   Certificates: {}", creds.certificates.len());
        }

        println!();
        Ok(())
    }


    // Network scanning utilities (kept for tests)

    #[allow(dead_code)]
    async fn scan_local_networks(&self) -> Result<Vec<DiscoveredService>> {
        let mut discovered = Vec::new();

        // Get discovery ranges from configuration or environment
        let discovery_ranges = std::env::var("TOADSTOOL_DISCOVERY_RANGES")
            .map(|ranges| ranges.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| {
                vec![
                    "127.0.0.1/32".to_string(),
                    "192.168.1.0/24".to_string(),
                    "10.0.0.0/24".to_string(),
                    "172.16.0.0/24".to_string(),
                ]
            });

        for range in discovery_ranges {
            info!("Scanning network range: {}", range);

            // Parse CIDR range and scan for services
            match self.scan_cidr_range(&range).await {
                Ok(mut services) => discovered.append(&mut services),
                Err(e) => warn!("Failed to scan range {}: {}", range, e),
            }
        }

        Ok(discovered)
    }

    #[allow(dead_code)]
    async fn scan_cidr_range(&self, cidr: &str) -> Result<Vec<DiscoveredService>> {
        // Parse CIDR notation
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {cidr}");
        }

        let base_ip = parts[0];
        let prefix_len: u32 = parts[1]
            .parse()
            .with_context(|| format!("Invalid prefix length: {}", parts[1]))?;

        // Generate IP range
        let ip_range = self.generate_ip_range(base_ip, prefix_len)?;

        // Scan each IP
        let mut discovered = Vec::new();
        for ip in ip_range {
            match timeout(Duration::from_millis(500), self.scan_ip_for_services(&ip)).await {
                Ok(Ok(mut services)) => discovered.append(&mut services),
                Ok(Err(e)) => warn!("Failed to scan IP {}: {}", ip, e),
                Err(_) => {} // Timeout - skip this IP
            }
        }

        Ok(discovered)
    }

    fn generate_ip_range(&self, base_ip: &str, prefix_len: u32) -> Result<Vec<String>> {
        let mut ips = Vec::new();

        match prefix_len {
            32 => {
                // Single host
                ips.push(base_ip.to_string());
            }
            24 => {
                // Class C network (254 hosts)
                let parts: Vec<&str> = base_ip.split('.').collect();
                if parts.len() != 4 {
                    anyhow::bail!("Invalid IP address: {base_ip}");
                }

                let prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                for i in 1..=254 {
                    ips.push(format!("{prefix}.{i}"));
                }
            }
            _ => {
                // Other prefix lengths not yet supported
                warn!("Unsupported prefix length: {}", prefix_len);
            }
        }

        Ok(ips)
    }

    async fn scan_ip_for_services(&self, ip: &str) -> Result<Vec<DiscoveredService>> {
        let mut discovered = Vec::new();
        let service_ports = get_standard_service_ports();

        for (service_type, port) in &service_ports {
            let addr: SocketAddr = format!("{ip}:{port}")
                .parse()
                .with_context(|| format!("Failed to parse address: {ip}:{port}"))?;

            if self.is_port_open(&addr).await {
                let mut capabilities = HashMap::new();
                capabilities.insert("discovered".to_string(), "true".to_string());

                discovered.push(DiscoveredService {
                    service_type: match service_type.as_str() {
                        "songbird" => ServiceType::Songbird,
                        "beardog" => ServiceType::BearDog,
                        "nestgate" => ServiceType::NestGate,
                        _ => ServiceType::ToadStool,
                    },
                    address: addr,
                    trust_level: TrustLevel::Discovered,
                    capabilities,
                    last_seen: Utc::now(),
                });
            }
        }

        Ok(discovered)
    }

    async fn is_port_open(&self, addr: &SocketAddr) -> bool {
        matches!(
            timeout(
                Duration::from_millis(200),
                tokio::net::TcpStream::connect(addr),
            )
            .await,
            Ok(Ok(_))
        )
    }
}

// Tests are included in the main integrator_impl.rs file

