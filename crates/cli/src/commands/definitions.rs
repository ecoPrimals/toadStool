// SPDX-License-Identifier: AGPL-3.0-only
//! CLI command definitions
//!
//! Subcommand enums and option structs for the ToadStool CLI.

use clap::Subcommand;
use std::path::PathBuf;

use toadstool_config::network::DEFAULT_CONNECTION_TIMEOUT_SECS;

/// Top-level CLI subcommands (run, up, down, ps, logs, validate, init, etc.)
#[derive(Subcommand)]
pub enum Commands {
    /// Start and run a biome in the foreground
    Run {
        /// Path to biome.yaml manifest file
        manifest: PathBuf,

        /// Override biome name
        #[arg(short, long)]
        name: Option<String>,

        /// Environment variables to set
        #[arg(short, long)]
        env: Vec<String>,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,

        /// CPU limit override (cores)
        #[arg(long)]
        cpu_limit: Option<f64>,
        /// Memory limit override (e.g. 512Mi)
        #[arg(long)]
        memory_limit: Option<String>,

        /// Security level (low, medium, high, maximum)
        #[arg(long, default_value = "high")]
        security: String,
    },

    /// Start a biome in the background (detached mode)
    Up {
        /// Path to biome.yaml manifest file
        manifest: PathBuf,

        /// Run in detached mode (background)
        #[arg(short, long)]
        detach: bool,

        /// Override biome name
        #[arg(short, long)]
        name: Option<String>,

        /// Environment variables to set
        #[arg(short, long)]
        env: Vec<String>,

        /// Auto-restart on failure
        #[arg(long)]
        restart: bool,

        /// Health check interval in seconds
        #[arg(long, default_value = "30")]
        health_interval: u64,
    },

    /// Stop a running biome
    Down {
        /// Biome name or ID to stop
        biome: String,

        /// Force stop (SIGKILL)
        #[arg(short, long)]
        force: bool,

        /// Timeout for graceful shutdown
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Remove all associated data
        #[arg(long)]
        purge: bool,
    },

    /// List all running biomes on the host
    Ps {
        /// Show all biomes (including stopped)
        #[arg(short, long)]
        all: bool,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show resource usage
        #[arg(short, long)]
        resources: bool,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },

    /// View logs for a specific biome or service
    Logs {
        /// Biome name or service name (biome.service)
        target: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        lines: usize,

        /// Show timestamps
        #[arg(short, long)]
        timestamps: bool,

        /// Filter by log level
        #[arg(long)]
        level: Option<String>,

        /// Search pattern
        #[arg(long)]
        grep: Option<String>,
    },

    /// Validate a biome.yaml manifest
    Validate {
        /// Path to biome.yaml manifest file
        manifest: PathBuf,

        /// Check resource availability
        #[arg(long)]
        check_resources: bool,

        /// Validate security policies
        #[arg(long)]
        check_security: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Initialize a new biome.yaml template
    Init {
        /// Directory to create biome.yaml in
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Biome template type
        #[arg(short, long, default_value = "basic")]
        template: String,

        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },

    /// Show system capabilities and detected platforms
    Capabilities {
        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show detailed platform information
        #[arg(short, long)]
        detailed: bool,

        /// Test specific platform
        #[arg(long)]
        test_platform: Option<String>,
    },

    /// Ecosystem integration commands
    Ecosystem {
        /// Subcommand (discover, register, auth, storage)
        #[command(subcommand)]
        action: EcosystemCommands,
    },

    /// Advanced universal compute operations
    Universal {
        /// Subcommand (detect, benchmark, migrate, federate)
        #[command(subcommand)]
        operation: UniversalCommands,
    },

    /// Hardware transport operations (HDMI, serial, capture)
    Transport {
        /// Subcommand (discover, list, status)
        #[command(subcommand)]
        action: TransportCommands,
    },

    /// Start ToadStool in server mode (long-running service)
    ///
    /// **UniBin Standard Compliant**: Ecosystem-standard subcommand for running ToadStool
    /// as a long-running service. Server mode processes workloads via JSON-RPC over Unix sockets.
    ///
    /// Like the fungus: CLI = fruiting body (specialized), Server = mycelium (network-wide)
    ///
    /// ## What Server Mode Does
    ///
    /// 1. **JSON-RPC API** (Unix Socket): Primal-to-primal workload submission
    /// 2. **Ecosystem Integration**: Registers with biomeOS capability registry
    /// 3. **Workload Orchestration**: Multi-runtime support (Native, Python, WASM, GPU)
    Server {
        /// Register with biomeOS capability registry
        #[arg(long)]
        register: bool,

        /// HTTP API port
        #[arg(long, default_value_t = toadstool_config::defaults::network::API_PORT)]
        port: u16,

        /// Unix socket path for IPC
        #[arg(long)]
        socket: Option<PathBuf>,

        /// Configuration file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Maximum concurrent workloads
        #[arg(long, default_value = "10")]
        max_workloads: usize,

        /// biomeOS registry socket path
        #[arg(long)]
        biomeos_socket: Option<PathBuf>,

        /// Family ID for multi-family socket support (creates toadstool-{family_id}.sock)
        #[arg(long)]
        family_id: Option<String>,
    },

    /// Start ToadStool as a daemon service (workload execution service)
    ///
    /// Like the fungus: CLI = fruiting body (specialized), Daemon = mycelium (network-wide)
    /// The daemon mode transforms ToadStool from a CLI tool into an ecosystem compute service:
    /// - JSON-RPC API for workload submission (Unix Socket)
    /// - Auto-registration with biomeOS capability registry
    /// - Resource monitoring and reporting
    /// - Multi-tower coordination
    /// - Persistent service management
    Daemon {
        /// Register with biomeOS capability registry
        #[arg(long)]
        register: bool,

        /// HTTP API port
        #[arg(long, default_value_t = toadstool_config::defaults::network::API_PORT)]
        port: u16,

        /// Unix socket path for IPC
        #[arg(long)]
        socket: Option<PathBuf>,

        /// Configuration file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Maximum concurrent workloads
        #[arg(long, default_value = "10")]
        max_workloads: usize,

        /// biomeOS registry socket path
        #[arg(long)]
        biomeos_socket: Option<PathBuf>,

        /// Family ID for multi-family socket support (creates toadstool-{family_id}.sock)
        #[arg(long)]
        family_id: Option<String>,
    },

    /// System health check and diagnostics
    ///
    /// **UniBin Standard Compliant**: Diagnose ToadStool installation, runtime,
    /// and ecosystem connectivity. Checks hardware detection, primal discovery,
    /// socket availability, and configuration validity.
    Doctor {
        /// Run all diagnostic checks
        #[arg(long)]
        all: bool,

        /// Check hardware detection (GPU, NPU, CPU capabilities)
        #[arg(long)]
        hardware: bool,

        /// Check ecosystem connectivity (Songbird, BearDog, NestGate)
        #[arg(long)]
        ecosystem: bool,

        /// Check configuration validity
        #[arg(long)]
        config: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Attempt to fix detected issues
        #[arg(long)]
        fix: bool,
    },

    /// BYOB server - HTTP API for team biome deployments from Songbird
    ///
    /// **UniBin Standard Compliant**: Compute execution service for BYOB (Bring Your Own Biome)
    /// deployments. Provides HTTP endpoints for deploy, list, stop, and resource usage.
    ByobServer {
        /// Server bind address (default from TOADSTOOL_BIND_ADDRESS or BIND_ADDRESS env)
        #[arg(short, long)]
        bind: Option<String>,

        /// Server port (default from TOADSTOOL_DAEMON_API_PORT or config)
        #[arg(short, long)]
        port: Option<u16>,

        /// Configuration file path (TOML)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// GPU mode switching for single-GPU systems.
    ///
    /// Switches between gaming mode (nvidia/nouveau for display) and science
    /// mode (vfio-pci for sovereign compute dispatch).
    Mode {
        /// Subcommand (science, gaming, status)
        #[command(subcommand)]
        action: ModeCommand,
    },

    /// Execute a workload directly (no biome.yaml required)
    Execute {
        /// Workload specification file (TOML or JSON)
        workload: PathBuf,

        /// Runtime hint (native, wasm, container, python, gpu)
        #[arg(short, long)]
        runtime: Option<String>,

        /// Override environment variables
        #[arg(short, long)]
        env: Vec<String>,

        /// Execution timeout in seconds
        #[arg(short, long, default_value = "300")]
        timeout: u64,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

/// Ecosystem integration subcommands (discover, register, auth, storage)
#[derive(Subcommand)]
pub enum EcosystemCommands {
    /// Discover and connect to ecosystem services
    Discover {
        /// Service types to discover
        #[arg(short, long)]
        services: Vec<String>,

        /// Network scan timeout
        #[arg(long, default_value_t = DEFAULT_CONNECTION_TIMEOUT_SECS)]
        timeout: u64,
    },

    /// Register with Songbird discovery service
    Register {
        /// Songbird endpoint
        endpoint: String,

        /// Authentication token
        #[arg(short, long)]
        token: Option<String>,
    },

    /// Install `BearDog` crypto permissions
    Auth {
        /// Permission file path
        permission_file: PathBuf,

        /// Validate only (don't install)
        #[arg(long)]
        validate_only: bool,
    },

    /// Connect to `NestGate` storage
    Storage {
        /// `NestGate` endpoint
        endpoint: String,

        /// Mount point
        #[arg(short, long, default_value = "/data")]
        mount: PathBuf,

        /// ZFS dataset name
        #[arg(long)]
        dataset: Option<String>,
    },
}

/// Hardware transport subcommands (discover, list, status)
#[derive(Subcommand)]
pub enum TransportCommands {
    /// Discover available hardware transports
    Discover {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// List registered transports (via daemon)
    List {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show transport layer status
    Status,
}

/// Universal compute subcommands (detect, benchmark, migrate, federate)
#[derive(Subcommand)]
pub enum UniversalCommands {
    /// Detect all available compute substrates
    Detect {
        /// Platform categories to detect
        #[arg(short, long)]
        categories: Vec<String>,

        /// Run detection tests
        #[arg(long)]
        test: bool,

        /// Save results to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Benchmark compute capabilities
    Benchmark {
        /// Benchmark suite to run
        #[arg(short, long, default_value = "standard")]
        suite: String,

        /// Target platforms
        #[arg(short, long)]
        platforms: Vec<String>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Migrate workloads between substrates
    Migrate {
        /// Source biome
        source: String,

        /// Target platform
        target: String,

        /// Pause source during migration
        #[arg(long)]
        pause: bool,

        /// Verify after migration
        #[arg(long)]
        verify: bool,
    },

    /// Federate with other `ToadStool` instances
    Federate {
        /// Remote `ToadStool` endpoint
        endpoint: String,

        /// Federation mode (peer, leader, follower)
        #[arg(short, long, default_value = "peer")]
        mode: String,

        /// Shared resources
        #[arg(short, long)]
        resources: Vec<String>,
    },
}

/// GPU mode switching subcommands.
#[derive(Clone, Subcommand)]
pub enum ModeCommand {
    /// Switch to science mode — bind GPU to vfio-pci for sovereign compute.
    Science {
        /// PCI BDF address (e.g. "0000:01:00.0"). Auto-detects if omitted.
        #[arg(long)]
        bdf: Option<String>,
    },
    /// Switch to gaming mode — unbind GPU from vfio-pci back to display driver.
    Gaming {
        /// PCI BDF address. Auto-detects if omitted.
        #[arg(long)]
        bdf: Option<String>,
    },
    /// Show current GPU mode.
    Status {
        /// PCI BDF address. Auto-detects if omitted.
        #[arg(long)]
        bdf: Option<String>,
    },
}
