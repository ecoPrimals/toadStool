//! Storage and database defaults
//!
//! # Example
//!
//! ```rust
//! use toadstool_config::defaults::storage;
//!
//! // Build storage backend URLs
//! let minio_url = format!("localhost:{}", storage::MINIO_PORT);
//! let redis_url = format!("redis://localhost:{}", storage::REDIS_PORT);
//! let postgres_url = format!("postgres://localhost:{}/toadstool", storage::POSTGRES_PORT);
//!
//! // Validate storage defaults (DISTRIBUTED_URL may be empty - use capability discovery)
//! assert!(storage::MINIO_PORT > 0);
//! ```

/// Default distributed storage URL.
/// Empty = use capability discovery. Override via `DISTRIBUTED_STORAGE_URL` env var.
#[deprecated(
    since = "0.3.0",
    note = "Use capability-based discovery; s3://localhost:9000 was hardcoded. Set DISTRIBUTED_STORAGE_URL or discover at runtime."
)]
pub const DISTRIBUTED_URL: &str = "";

/// Default MinIO/S3 port
pub const MINIO_PORT: u16 = 9000;

/// Default Redis port
pub const REDIS_PORT: u16 = 6379;

/// Default `PostgreSQL` port
pub const POSTGRES_PORT: u16 = 5432;

/// Default AMQP (RabbitMQ) port
pub const AMQP_PORT: u16 = 5672;
