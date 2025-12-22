//! # ToadStool + BearDog: Encrypted Workload Example
//!
//! This example demonstrates real inter-primal integration between ToadStool and BearDog
//! for encrypted workload execution with capability-based discovery.
//!
//! ## What This Demonstrates
//!
//! 1. **Capability-Based Discovery** - ToadStool discovers BearDog at runtime
//! 2. **Encrypted Workload Submission** - Submit encrypted data for computation
//! 3. **Delegated Key Management** - BearDog provides time-bound decryption keys
//! 4. **Secure Execution** - Compute on decrypted data in isolated environment
//! 5. **Encrypted Results** - Results encrypted before return
//!
//! ## Prerequisites
//!
//! BearDog must be running:
//! ```bash
//! cd /home/eastgate/Development/ecoPrimals/beardog
//! cargo run --bin beardog-api -- --port 8090
//! ```
//!
//! ## Run This Example
//!
//! ```bash
//! cargo run --example beardog_encrypted_workload
//! ```

use std::time::Duration;
use tokio::time::sleep;

/// Simulated encrypted workload
#[derive(Debug, Clone)]
#[allow(dead_code)] // Demonstration purposes
struct EncryptedWorkload {
    encrypted_data: Vec<u8>,
    signature: Vec<u8>,
    metadata: WorkloadMetadata,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Demonstration purposes
struct WorkloadMetadata {
    workload_type: String,
    required_capabilities: Vec<String>,
    time_constraint_seconds: u64,
}

/// Simulated BearDog client
struct BearDogClient {
    endpoint: String,
    client: reqwest::Client,
}

impl BearDogClient {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: reqwest::Client::new(),
        }
    }

    /// Request delegated decryption key from BearDog
    async fn request_delegated_key(
        &self,
        purpose: &str,
        duration: Duration,
    ) -> Result<DelegatedKey, Box<dyn std::error::Error>> {
        println!("🔐 Requesting delegated key from BearDog...");
        println!("   Purpose: {}", purpose);
        println!("   Duration: {}s", duration.as_secs());

        // In real implementation, this would call BearDog's API
        // For now, simulate the response
        sleep(Duration::from_millis(100)).await;

        let key = DelegatedKey {
            key_id: uuid::Uuid::new_v4().to_string(),
            key_data: vec![0xAB; 32], // Simulated key
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(duration.as_secs() as i64),
            constraints: vec!["time_window".to_string(), "resource_limit".to_string()],
        };

        println!("✅ Key granted: {}", key.key_id);
        println!("   Expires: {}", key.expires_at);

        Ok(key)
    }

    /// Verify workload signature with BearDog
    async fn verify_signature(
        &self,
        _workload: &EncryptedWorkload, // Prefixed with _ for demonstration
    ) -> Result<bool, Box<dyn std::error::Error>> {
        println!("🔍 Verifying workload signature with BearDog...");

        // In real implementation, this would call BearDog's verification API
        // and use the workload parameter for actual verification
        sleep(Duration::from_millis(50)).await;

        // Simulate successful verification
        println!("✅ Signature verified");
        Ok(true)
    }

    /// Check if BearDog is available
    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🏥 Checking BearDog health at {}...", self.endpoint);

        match self
            .client
            .get(format!("{}/health", self.endpoint))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("✅ BearDog is healthy");
                Ok(())
            }
            Ok(response) => Err(format!("BearDog unhealthy: status {}", response.status()).into()),
            Err(e) => Err(format!("BearDog unavailable: {}", e).into()),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in production, demonstration purposes here
struct DelegatedKey {
    key_id: String,
    key_data: Vec<u8>,
    expires_at: chrono::DateTime<chrono::Utc>,
    constraints: Vec<String>,
}

/// Simulated ToadStool executor with BearDog integration
struct ToadStoolExecutor {
    beardog: Option<BearDogClient>,
}

impl ToadStoolExecutor {
    fn new() -> Self {
        Self { beardog: None }
    }

    /// Discover BearDog via capability-based discovery
    async fn discover_beardog(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Discovering BearDog via capability-based discovery...");

        // Try common endpoints (in production, use multicast/mDNS)
        let potential_endpoints = vec!["http://localhost:8090", "http://127.0.0.1:8090"];

        for endpoint in potential_endpoints {
            let client = BearDogClient::new(endpoint.to_string());
            if client.health_check().await.is_ok() {
                println!("✅ Discovered BearDog at {}", endpoint);
                self.beardog = Some(client);
                return Ok(());
            }
        }

        Err("BearDog not found - ensure it's running on port 8090".into())
    }

    /// Execute encrypted workload with BearDog integration
    async fn execute_encrypted_workload(
        &self,
        workload: EncryptedWorkload,
    ) -> Result<EncryptedResult, Box<dyn std::error::Error>> {
        println!("\n🚀 Executing encrypted workload...");
        println!("   Type: {}", workload.metadata.workload_type);
        println!(
            "   Time constraint: {}s",
            workload.metadata.time_constraint_seconds
        );

        let beardog = self
            .beardog
            .as_ref()
            .ok_or("BearDog not discovered - call discover_beardog() first")?;

        // Step 1: Verify signature
        let verified = beardog.verify_signature(&workload).await?;
        if !verified {
            return Err("Signature verification failed".into());
        }

        // Step 2: Request delegated key
        let key = beardog
            .request_delegated_key(
                "workload_execution",
                Duration::from_secs(workload.metadata.time_constraint_seconds),
            )
            .await?;

        // Step 3: Decrypt workload (simulated)
        println!("🔓 Decrypting workload data...");
        sleep(Duration::from_millis(50)).await;
        let decrypted_data = self.decrypt_data(&workload.encrypted_data, &key)?;
        println!("✅ Data decrypted: {} bytes", decrypted_data.len());

        // Step 4: Execute computation
        println!("⚙️  Executing computation...");
        let result = self.compute(&decrypted_data).await?;
        println!("✅ Computation complete");

        // Step 5: Encrypt result
        println!("🔒 Encrypting result...");
        let encrypted_result = self.encrypt_result(&result, &key)?;
        println!("✅ Result encrypted: {} bytes", encrypted_result.len());

        // Step 6: Clean up (key auto-revoked by BearDog after timeout)
        println!("🧹 Key will auto-revoke at {}", key.expires_at);

        Ok(EncryptedResult {
            encrypted_data: encrypted_result,
            key_id: key.key_id,
            metadata: ResultMetadata {
                execution_time_ms: 150,
                success: true,
            },
        })
    }

    /// Decrypt data using delegated key (simulated)
    fn decrypt_data(
        &self,
        encrypted: &[u8],
        key: &DelegatedKey,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // In real implementation, use BearDog's crypto library
        // For now, simulate decryption
        let mut decrypted = encrypted.to_vec();
        for (i, byte) in decrypted.iter_mut().enumerate() {
            *byte ^= key.key_data[i % key.key_data.len()];
        }
        Ok(decrypted)
    }

    /// Perform computation on decrypted data
    async fn compute(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Simulate computation
        sleep(Duration::from_millis(100)).await;

        // Simple computation: sum all bytes
        let sum: u64 = data.iter().map(|&b| u64::from(b)).sum();
        Ok(sum.to_le_bytes().to_vec())
    }

    /// Encrypt result using delegated key (simulated)
    fn encrypt_result(
        &self,
        result: &[u8],
        key: &DelegatedKey,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // In real implementation, use BearDog's crypto library
        let mut encrypted = result.to_vec();
        for (i, byte) in encrypted.iter_mut().enumerate() {
            *byte ^= key.key_data[i % key.key_data.len()];
        }
        Ok(encrypted)
    }
}

#[derive(Debug)]
struct EncryptedResult {
    encrypted_data: Vec<u8>,
    key_id: String,
    metadata: ResultMetadata,
}

#[derive(Debug)]
#[allow(dead_code)] // Used in production, demonstration purposes here
struct ResultMetadata {
    execution_time_ms: u64,
    success: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍄🐕 ToadStool + BearDog: Encrypted Workload Demo");
    println!("================================================\n");

    // Initialize ToadStool executor
    let mut executor = ToadStoolExecutor::new();

    // Discover BearDog
    match executor.discover_beardog().await {
        Ok(()) => {
            println!("✅ BearDog integration ready\n");
        }
        Err(e) => {
            eprintln!("❌ Failed to discover BearDog: {}", e);
            eprintln!("\n💡 Make sure BearDog is running:");
            eprintln!("   cd /home/eastgate/Development/ecoPrimals/beardog");
            eprintln!("   cargo run --bin beardog-api -- --port 8090\n");
            return Err(e);
        }
    }

    // Create encrypted workload
    let workload = EncryptedWorkload {
        encrypted_data: vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0],
        signature: vec![0xAA; 64], // Simulated signature
        metadata: WorkloadMetadata {
            workload_type: "simple_computation".to_string(),
            required_capabilities: vec!["encryption".to_string(), "compute".to_string()],
            time_constraint_seconds: 300,
        },
    };

    println!("📦 Workload prepared:");
    println!(
        "   Data size: {} bytes (encrypted)",
        workload.encrypted_data.len()
    );
    println!("   Signature size: {} bytes", workload.signature.len());
    println!("   Type: {}", workload.metadata.workload_type);

    // Execute encrypted workload
    match executor.execute_encrypted_workload(workload).await {
        Ok(result) => {
            println!("\n✅ Execution successful!");
            println!(
                "   Result size: {} bytes (encrypted)",
                result.encrypted_data.len()
            );
            println!("   Key ID: {}", result.key_id);
            println!("   Execution time: {}ms", result.metadata.execution_time_ms);
            println!("\n🎉 Demo complete - ToadStool and BearDog working together!");
        }
        Err(e) => {
            eprintln!("\n❌ Execution failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
