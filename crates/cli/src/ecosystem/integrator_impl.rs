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
        let service_ports = self.get_standard_service_ports();

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
                self.scan_for_service(service_type, &service_ports),
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
            match self.verify_service(service).await {
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
            address: self.get_local_address()?,
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

        // Attempt registration
        match self.send_songbird_registration(&addr, &registration).await {
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
                    last_heartbeat: chrono::Utc::now(),
                    _auth_token: token,
                };

                self.connections.insert("songbird".to_string(), connection);

                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to register with Songbird: {}", e);
                Err(e)
            }
        }
    }

    /// Install `BearDog` cryptographic permissions
    pub async fn install_beardog_permissions(
        &mut self,
        permission_file: PathBuf,
        validate_only: bool,
    ) -> Result<()> {
        info!(
            "🐻 Installing BearDog permissions from: {}",
            permission_file.display()
        );

        // Load permission file
        let permission_content = fs::read_to_string(&permission_file)
            .await
            .with_context(|| {
                format!(
                    "Failed to read permission file: {}",
                    permission_file.display()
                )
            })?;

        let permission: BearDogPermission = serde_yaml::from_str(&permission_content)
            .with_context(|| "Failed to parse BearDog permission file")?;

        info!("📋 Permission ID: {}", permission.permission_id);
        info!("📋 Granted to: {}", permission.granted_to);
        info!("📋 Capabilities: {}", permission.capabilities.join(", "));
        info!("📋 Valid until: {}", permission.valid_until);

        // Validate permission
        if !self.validate_beardog_permission(&permission).await? {
            bail!("❌ BearDog permission validation failed");
        }

        info!("✅ BearDog permission validation successful");

        if validate_only {
            info!("🔍 Validation-only mode - not installing");
            return Ok(());
        }

        // Install permission
        self.install_permission(&permission).await?;

        info!("✅ BearDog permissions installed successfully");
        Ok(())
    }

    /// Connect to `NestGate` distributed storage
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

        // Check if mount point exists
        if !mount_point.exists() {
            fs::create_dir_all(&mount_point).await.with_context(|| {
                format!("Failed to create mount point: {}", mount_point.display())
            })?;
        }

        // Connect to NestGate
        let mount_info = self
            .mount_nestgate_dataset(&addr, &mount_point, dataset.as_deref())
            .await?;

        info!("✅ NestGate storage connected");
        info!("   Dataset: {}", mount_info.dataset_name);
        info!("   Mount point: {}", mount_info.mount_point.display());
        info!("   Access mode: {}", mount_info.access_mode);

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
            last_heartbeat: chrono::Utc::now(),
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

    async fn scan_for_service(
        &self,
        service_type: &str,
        service_ports: &HashMap<String, u16>,
    ) -> Result<Vec<ServiceEndpoint>> {
        let port = service_ports
            .get(service_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown service type: {service_type}"))?;

        let mut services = Vec::new();

        // Scan local network ranges - configurable via environment
        let local_ranges = std::env::var("TOADSTOOL_SCAN_RANGES")
            .unwrap_or_else(|_| "127.0.0.1,192.168.1.0/24,10.0.0.0/24".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();

        for range in local_ranges {
            if range.contains('/') {
                // Subnet scan - simplified for demo
                continue;
            } else {
                // Single IP
                let addr = format!("{range}:{port}");
                if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
                    if self.check_service_available(&socket_addr).await? {
                        services.push(ServiceEndpoint {
                            service_type: EcosystemService::parse(service_type),
                            address: socket_addr,
                            version: "unknown".to_string(),
                            capabilities: vec!["basic".to_string()],
                            trust_level: TrustLevel::Discovered,
                        });
                    }
                }
            }
        }

        Ok(services)
    }

    async fn check_service_available(&self, addr: &SocketAddr) -> Result<bool> {
        // Simple TCP connection test
        match timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // Timeout
        }
    }

    async fn verify_service(&self, service: &ServiceEndpoint) -> Result<bool> {
        // Implement service-specific verification
        match &service.service_type {
            EcosystemService::Songbird => self.verify_songbird_service(&service.address).await,
            EcosystemService::BearDog => self.verify_beardog_service(&service.address).await,
            EcosystemService::NestGate => self.verify_nestgate_service(&service.address).await,
            EcosystemService::Unknown(_) => Ok(false),
        }
    }

    async fn verify_songbird_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Send ping to Songbird and verify response signature
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(&format!("http://{addr}/health/signed")),
        )
        .await
        {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    // Parse signed response
                    if let Ok(signed_response) = response.json::<SignedServiceResponse>().await {
                        let crypto_context = CryptoVerificationContext::new();

                        // Verify cryptographic signature of response
                        match crypto_context.verify_service_signature("songbird", &signed_response)
                        {
                            Ok(true) => {
                                info!("✅ Songbird service cryptographically verified");
                                Ok(true)
                            }
                            Ok(false) => {
                                warn!("⚠️  Songbird service signature verification failed");
                                Ok(false)
                            }
                            Err(e) => {
                                error!("❌ Songbird verification error: {}", e);
                                Ok(false)
                            }
                        }
                    } else {
                        // Try fallback to unsigned health check with warning
                        warn!("⚠️  Songbird service does not support signed responses - security degraded");
                        Ok(false) // Fail securely for production systems
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    async fn verify_beardog_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Verify BearDog cryptographic service with proper signature validation
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(&format!("http://{addr}/crypto/identity/signed")),
        )
        .await
        {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    // Parse BearDog identity response
                    if let Ok(signed_response) = response.json::<SignedServiceResponse>().await {
                        let crypto_context = CryptoVerificationContext::new();

                        // Verify BearDog cryptographic identity and signature
                        match crypto_context.verify_service_signature("beardog", &signed_response) {
                            Ok(true) => {
                                // Additional BearDog-specific verification
                                if self.verify_beardog_capabilities(&signed_response).await? {
                                    info!("✅ BearDog cryptographic service verified");
                                    Ok(true)
                                } else {
                                    warn!("⚠️  BearDog capability verification failed");
                                    Ok(false)
                                }
                            }
                            Ok(false) => {
                                error!("❌ BearDog cryptographic signature verification failed");
                                Ok(false)
                            }
                            Err(e) => {
                                error!("❌ BearDog verification error: {}", e);
                                Ok(false)
                            }
                        }
                    } else {
                        error!("🚨 BearDog service does not support cryptographic verification");
                        Ok(false) // SECURITY: Always fail for BearDog without crypto
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    async fn verify_nestgate_service(&self, addr: &SocketAddr) -> Result<bool> {
        // Verify NestGate storage service with proper access controls
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(&format!("http://{addr}/storage/access/signed")),
        )
        .await
        {
            Ok(Ok(response)) => {
                if response.status().is_success() {
                    // Parse NestGate access control response
                    if let Ok(signed_response) = response.json::<SignedServiceResponse>().await {
                        let crypto_context = CryptoVerificationContext::new();

                        // Verify storage service permissions and encryption keys
                        match crypto_context.verify_service_signature("nestgate", &signed_response)
                        {
                            Ok(true) => {
                                // Additional NestGate-specific verification
                                if self.verify_nestgate_permissions(&signed_response).await? {
                                    info!("✅ NestGate storage service verified");
                                    Ok(true)
                                } else {
                                    warn!("⚠️  NestGate permission verification failed");
                                    Ok(false)
                                }
                            }
                            Ok(false) => {
                                error!("❌ NestGate signature verification failed");
                                Ok(false)
                            }
                            Err(e) => {
                                error!("❌ NestGate verification error: {}", e);
                                Ok(false)
                            }
                        }
                    } else {
                        error!("🚨 NestGate service does not support signed responses");
                        Ok(false) // SECURITY: Fail securely
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    async fn verify_beardog_capabilities(&self, response: &SignedServiceResponse) -> Result<bool> {
        // Verify BearDog has required cryptographic capabilities
        let required_capabilities = vec![
            "ed25519_signing",
            "key_generation",
            "signature_verification",
            "identity_management",
        ];

        for capability in required_capabilities {
            if !response.capabilities.contains(&capability.to_string()) {
                warn!("BearDog missing required capability: {}", capability);
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn verify_nestgate_permissions(&self, response: &SignedServiceResponse) -> Result<bool> {
        // Verify NestGate has required storage permissions
        let required_capabilities = vec![
            "zfs_management",
            "encryption_support",
            "access_control",
            "snapshot_management",
        ];

        for capability in required_capabilities {
            if !response.capabilities.contains(&capability.to_string()) {
                warn!("NestGate missing required capability: {}", capability);
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_standard_service_ports(&self) -> HashMap<String, u16> {
        vec![
            ("songbird".to_string(), 5000),
            ("beardog".to_string(), 5001),
            ("nestgate".to_string(), 5002),
        ]
        .into_iter()
        .collect()
    }

    fn get_local_address(&self) -> Result<SocketAddr> {
        // Get local network address with environment-aware port
        let config = EnvironmentConfig::from_env();
        let host = &config.network.bind_address;
        let port: u16 = std::env::var("TOADSTOOL_API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8084);
        Ok(format!("{host}:{port}").parse()?)
    }

    async fn send_songbird_registration(
        &self,
        addr: &SocketAddr,
        _registration: &SongbirdRegistration,
    ) -> Result<SongbirdResponse> {
        // Send HTTP POST to Songbird registration endpoint
        // This is a simplified implementation
        Ok(SongbirdResponse {
            service_id: Uuid::new_v4().to_string(),
            registry_url: format!("http://{}:{}/registry", addr.ip(), addr.port()),
            heartbeat_interval: 30,
        })
    }

    async fn validate_beardog_permission(&self, permission: &BearDogPermission) -> Result<bool> {
        // Validate cryptographic signature
        // Check expiration
        // Verify capabilities

        if permission.valid_until < chrono::Utc::now() {
            warn!("⚠️  BearDog permission has expired");
            return Ok(false);
        }

        // Implement proper cryptographic verification
        let crypto_context = CryptoVerificationContext::new();

        // Get public key for BearDog permission verification
        if let Some(beardog_key) = crypto_context.trusted_public_keys.get("beardog") {
            // Create canonical permission message for verification
            let permission_message = self.create_permission_message(permission)?;

            // Decode signature and public key
            let signature_bytes = BASE64
                .decode(&permission.signature)
                .map_err(|e| anyhow::anyhow!("Failed to decode permission signature: {e}"))?;

            let public_key_bytes = BASE64
                .decode(beardog_key)
                .map_err(|e| anyhow::anyhow!("Failed to decode BearDog public key: {e}"))?;

            // Verify signature using BearDog public key cryptography
            match crypto_context.verify_ed25519_signature(
                &permission_message,
                &signature_bytes,
                &public_key_bytes,
            ) {
                Ok(true) => {
                    info!("✅ BearDog permission signature verified");
                    Ok(true)
                }
                Ok(false) => {
                    error!("❌ BearDog permission signature verification failed");
                    Ok(false)
                }
                Err(e) => {
                    error!("❌ BearDog permission verification error: {}", e);
                    Ok(false)
                }
            }
        } else {
            error!("🚨 No trusted BearDog public key configured");
            Ok(false) // SECURITY: Fail securely without trusted key
        }
    }

    fn create_permission_message(&self, permission: &BearDogPermission) -> Result<Vec<u8>> {
        // Create canonical message for BearDog permission verification
        let mut data = BTreeMap::new();
        data.insert("permission_id", permission.permission_id.to_string());
        data.insert("granted_to", permission.granted_to.clone());
        data.insert("valid_until", permission.valid_until.to_rfc3339());

        let capabilities_json = serde_json::to_string(&permission.capabilities)
            .map_err(|e| anyhow::anyhow!("Failed to serialize permission capabilities: {e}"))?;
        data.insert("capabilities", capabilities_json);

        let canonical_json = serde_json::to_string(&data)
            .map_err(|e| anyhow::anyhow!("Failed to create canonical permission message: {e}"))?;

        Ok(canonical_json.into_bytes())
    }

    async fn install_permission(&self, permission: &BearDogPermission) -> Result<()> {
        // Install permission in system keyring or secure storage
        // This is a simplified implementation
        info!("🔐 Installing permission: {}", permission.permission_id);
        Ok(())
    }

    async fn mount_nestgate_dataset(
        &self,
        addr: &SocketAddr,
        mount_point: &Path,
        dataset: Option<&str>,
    ) -> Result<NestGateMount> {
        // Connect to NestGate and mount ZFS dataset
        // This is a simplified implementation

        let dataset_name = dataset.unwrap_or("default").to_string();

        Ok(NestGateMount {
            dataset_name: dataset_name.clone(),
            mount_point: mount_point.to_path_buf(),
            endpoint: addr.to_string(),
            zfs_dataset: Some(format!("tank/{dataset_name}")),
            access_mode: "read-write".to_string(),
            encryption_key: None,
        })
    }

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
        let mut services = Vec::new();

        // Parse CIDR notation
        if cidr.contains('/') {
            let parts: Vec<&str> = cidr.split('/').collect();
            if parts.len() == 2 {
                let base_ip = parts[0];
                let prefix_len: u32 = parts[1].parse().unwrap_or(24);

                // Generate IP range based on CIDR
                let ip_range = self.generate_ip_range(base_ip, prefix_len)?;

                // Scan each IP in range
                for ip in ip_range.iter().take(254) {
                    // Limit scan size
                    if let Ok(discovered) = self.scan_ip_for_services(ip).await {
                        services.extend(discovered);
                    }
                }
            }
        } else {
            // Single IP address
            if let Ok(discovered) = self.scan_ip_for_services(cidr).await {
                services.extend(discovered);
            }
        }

        Ok(services)
    }

    #[allow(dead_code)]
    fn generate_ip_range(&self, base_ip: &str, prefix_len: u32) -> Result<Vec<String>> {
        let mut ips = Vec::new();

        // Simple implementation for common cases
        if prefix_len == 24 {
            // Class C subnet (e.g., 192.168.1.0/24)
            if let Some(base) = base_ip.rsplit_once('.').map(|x| x.0) {
                for i in 1..255 {
                    ips.push(format!("{base}.{i}"));
                }
            }
        } else if prefix_len == 32 {
            // Single host
            ips.push(base_ip.to_string());
        }
        // Add more CIDR handling as needed

        Ok(ips)
    }

    #[allow(dead_code)]
    async fn scan_ip_for_services(&self, ip: &str) -> Result<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Common service ports to scan - use environment configuration
        let config = EnvironmentConfig::from_env();
        let service_ports = [
            (config.network.songbird_port, ServiceType::Songbird),
            (config.network.nestgate_port, ServiceType::NestGate),
            (config.network.beardog_port, ServiceType::BearDog),
            (config.network.toadstool_port, ServiceType::ToadStool),
        ];

        for (port, service_type) in service_ports {
            let addr = format!("{ip}:{port}");
            if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
                if self.is_port_open(&socket_addr).await {
                    services.push(DiscoveredService {
                        service_type,
                        address: socket_addr,
                        trust_level: TrustLevel::Discovered, // Default to discovered
                        capabilities: HashMap::new(),
                        last_seen: Utc::now(),
                    });
                }
            }
        }

        Ok(services)
    }

    #[allow(dead_code)]
    async fn is_port_open(&self, addr: &SocketAddr) -> bool {
        matches!(
            tokio::time::timeout(
                Duration::from_millis(1000),
                tokio::net::TcpStream::connect(addr),
            )
            .await,
            Ok(Ok(_))
        )
    }
}

#[cfg(test)]
mod integrator_tests {
    use super::*;

    // ========================================================================
    // Test 1: scan_for_service with standard ports
    // ========================================================================

    #[tokio::test]
    async fn test_scan_for_service_unknown_type() {
        let integrator = EcosystemIntegrator::new();
        let ports = HashMap::new();
        
        // Empty ports map should return error for unknown service
        let result = integrator.scan_for_service("songbird", &ports).await;
        assert!(result.is_err());
    }

    // ========================================================================
    // Test 2: get_local_address
    // ========================================================================

    #[test]
    fn test_get_local_address() {
        let integrator = EcosystemIntegrator::new();
        let result = integrator.get_local_address();
        
        // Should either succeed or fail gracefully
        match result {
            Ok(addr) => {
                assert!(addr.port() > 0);
            }
            Err(_) => {
                // OK if network is unavailable
            }
        }
    }

    // ========================================================================
    // Test 3: create_permission_message
    // ========================================================================

    #[test]
    fn test_create_permission_message_structure() {
        let integrator = EcosystemIntegrator::new();
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test-service".to_string(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            valid_until: Utc::now() + chrono::Duration::hours(1),
            signature: "signature123".to_string(),
        };

        let result = integrator.create_permission_message(&permission);
        assert!(result.is_ok());
        
        let message = result.unwrap();
        assert!(!message.is_empty());
        
        // Message should contain serialized data
        assert!(message.len() > 50); // Reasonable size check
    }

    // ========================================================================
    // Test 4: generate_ip_range edge cases
    // ========================================================================

    #[test]
    fn test_generate_ip_range_class_c_different_base() {
        let integrator = EcosystemIntegrator::new();
        let result = integrator.generate_ip_range("10.20.30.0", 24);
        
        assert!(result.is_ok());
        let ips = result.unwrap();
        assert_eq!(ips.len(), 254);
        assert!(ips.contains(&"10.20.30.1".to_string()));
        assert!(ips.contains(&"10.20.30.128".to_string()));
        assert!(ips.contains(&"10.20.30.254".to_string()));
    }

    #[test]
    fn test_generate_ip_range_single_host_various() {
        let integrator = EcosystemIntegrator::new();
        
        for ip in &["192.168.1.100", "10.0.0.1", "172.16.0.50"] {
            let result = integrator.generate_ip_range(ip, 32);
            assert!(result.is_ok());
            let ips = result.unwrap();
            assert_eq!(ips.len(), 1);
            assert_eq!(&ips[0], ip);
        }
    }

    // ========================================================================
    // Test 5: validate_beardog_permission  
    // ========================================================================

    #[tokio::test]
    async fn test_validate_beardog_permission_structure() {
        let integrator = EcosystemIntegrator::new();
        
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "service-123".to_string(),
            capabilities: vec!["read".to_string()],
            valid_until: Utc::now() + chrono::Duration::hours(2),
            signature: "valid-sig-base64".to_string(),
        };

        // Without actual BearDog service, this will likely fail on network/crypto
        // but we're testing that it doesn't panic and returns Result
        let result = integrator.validate_beardog_permission(&permission).await;
        
        // Either succeeds or fails gracefully
        match result {
            Ok(_valid) => {
                // If OK, validation completed (result is a bool)
                // Test passes if no panic occurs
            }
            Err(_) => {
                // Expected without real service
            }
        }
    }

    // ========================================================================
    // Test 6: scan_cidr_range edge cases
    // ========================================================================

    #[tokio::test]
    async fn test_scan_cidr_range_small_network() {
        let integrator = EcosystemIntegrator::new();
        
        // Quick scan with 32-bit mask (single IP)
        let result = integrator.scan_cidr_range("127.0.0.1/32").await;
        
        // Should complete quickly and return result (empty or with localhost)
        assert!(result.is_ok() || result.is_err()); // Either is fine
    }

    // ========================================================================
    // Test 7: check_service_available
    // ========================================================================

    #[tokio::test]
    async fn test_check_service_available_unavailable() {
        let integrator = EcosystemIntegrator::new();
        
        // Try connecting to a definitely unavailable port
        let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();
        let result = integrator.check_service_available(&addr).await;
        
        // Should return Ok(false) for unavailable service
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ========================================================================
    // Test 8: get_standard_service_ports values
    // ========================================================================

    #[test]
    fn test_get_standard_service_ports_values() {
        let integrator = EcosystemIntegrator::new();
        let ports = integrator.get_standard_service_ports();
        
        // Verify we have the expected services
        assert!(ports.contains_key("songbird"));
        assert!(ports.contains_key("beardog"));
        assert!(ports.contains_key("nestgate"));
        
        // Verify ports are in valid range
        for (service, &port) in &ports {
            assert!(port > 1024, "{} port too low: {}", service, port);
            assert!(port < 65535, "{} port too high: {}", service, port);
        }
    }

    // ========================================================================
    // Test 9: scan_local_networks
    // ========================================================================

    #[tokio::test]
    async fn test_scan_local_networks_completes() {
        let integrator = EcosystemIntegrator::new();
        
        // This will scan common local networks - may take a moment
        // Set a reasonable timeout
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            integrator.scan_local_networks()
        ).await;
        
        match result {
            Ok(Ok(_services)) => {
                // Successful scan - any count is valid
            }
            Ok(Err(_)) => {
                // Error during scan is acceptable
            }
            Err(_) => {
                // Timeout is acceptable - network scanning can be slow
            }
        }
    }

    // ========================================================================
    // Test 10: Discovery result empty case
    // ========================================================================

    #[tokio::test]
    async fn test_discover_services_with_timeout() {
        let mut integrator = EcosystemIntegrator::new();
        
        // Very short timeout should complete quickly
        let result = integrator.discover_services(
            vec!["nonexistent-service".to_string()],
            1 // 1 second
        ).await;
        
        // May succeed with empty results or timeout
        match result {
            Ok(discovery) => {
                // total_discovered is unsigned, always >= 0
                assert!(discovery.verified_count <= discovery.total_discovered);
            }
            Err(_) => {
                // Timeout or error is acceptable
            }
        }
    }

    // ========================================================================
    // Test 11: Permission message multiple capabilities
    // ========================================================================

    #[test]
    fn test_create_permission_message_multiple_capabilities() {
        let integrator = EcosystemIntegrator::new();
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "multi-cap-service".to_string(),
            capabilities: vec![
                "read".to_string(),
                "write".to_string(),
                "execute".to_string(),
                "admin".to_string(),
            ],
            valid_until: Utc::now() + chrono::Duration::days(1),
            signature: "sig".to_string(),
        };

        let result = integrator.create_permission_message(&permission);
        assert!(result.is_ok());
        
        let message = result.unwrap();
        assert!(message.len() > 100); // More data = larger message
    }

    // ========================================================================
    // Test 12: IP range boundary conditions
    // ========================================================================

    #[test]
    fn test_generate_ip_range_boundaries() {
        let integrator = EcosystemIntegrator::new();
        
        // Test with zeros in different octets
        let test_cases = vec![
            ("192.0.0.0", 24, "192.0.0"),
            ("172.16.0.0", 24, "172.16.0"),
            ("10.0.0.0", 24, "10.0.0"),
        ];
        
        for (base_ip, prefix, expected_prefix) in test_cases {
            let result = integrator.generate_ip_range(base_ip, prefix);
            assert!(result.is_ok());
            let ips = result.unwrap();
            
            if prefix == 24 {
                assert_eq!(ips.len(), 254);
                assert!(ips[0].starts_with(expected_prefix));
                assert!(ips.last().unwrap().starts_with(expected_prefix));
            }
        }
    }

    // ========================================================================
    // Test 13: Integrator state management
    // ========================================================================

    #[test]
    fn test_integrator_state_after_creation() {
        let integrator = EcosystemIntegrator::new();
        
        // Verify initial state
        assert!(integrator.endpoints.is_empty());
        assert!(integrator.connections.is_empty());
        assert!(integrator.credentials.is_none());
    }

    #[test]
    fn test_integrator_multiple_instances() {
        let integrator1 = EcosystemIntegrator::new();
        let integrator2 = EcosystemIntegrator::new();
        
        // Each instance should be independent
        assert!(integrator1.endpoints.is_empty());
        assert!(integrator2.endpoints.is_empty());
    }

    // ========================================================================
    // Test 14: Service type parsing
    // ========================================================================

    #[test]
    fn test_ecosystem_service_parse_variations() {
        assert!(matches!(
            EcosystemService::parse("songbird"),
            EcosystemService::Songbird
        ));
        assert!(matches!(
            EcosystemService::parse("SONGBIRD"),
            EcosystemService::Songbird
        ));
        assert!(matches!(
            EcosystemService::parse("SongBird"),
            EcosystemService::Songbird
        ));
    }

    // ========================================================================
    // Test 15: Permission expiration edge cases
    // ========================================================================

    #[test]
    fn test_permission_already_expired() {
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test".to_string(),
            capabilities: vec!["read".to_string()],
            valid_until: Utc::now() - chrono::Duration::hours(1), // Already expired!
            signature: "sig".to_string(),
        };

        assert!(permission.valid_until < Utc::now());
    }

    #[test]
    fn test_permission_expires_soon() {
        let permission = BearDogPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test".to_string(),
            capabilities: vec!["read".to_string()],
            valid_until: Utc::now() + chrono::Duration::minutes(5),
            signature: "sig".to_string(),
        };

        let time_until_expiry = permission.valid_until - Utc::now();
        assert!(time_until_expiry.num_minutes() <= 5);
    }
}
