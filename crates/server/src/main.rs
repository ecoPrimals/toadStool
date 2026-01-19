//! # ToadStool Server Daemon
//!
//! Universal compute server for biomeOS ecosystem integration.
//!
//! ## Deep Debt Principles
//!
//! - **Capability-Based Discovery**: Registers with Songbird at runtime
//! - **Self-Knowledge Only**: No hardcoded knowledge of other primals
//! - **Unix Socket PRIMARY**: No TCP hardcoding, multi-instance support
//! - **Unique Family IDs**: Each instance has unique identity
//! - **Graceful Degradation**: Works standalone if Songbird unavailable
//! - **Modern Idiomatic Rust**: No unwrap(), proper error handling
//!
//! ## UniBin Architecture
//!
//! This standalone binary shares logic with the `toadstool server` UniBin command.
//! Both call the same `run_server_main()` function from the toadstool-server library.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with env filter support
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Call shared UniBin server implementation
    toadstool_server::run_server_main().await
}
