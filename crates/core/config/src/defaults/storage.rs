// SPDX-License-Identifier: AGPL-3.0-or-later
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
//! assert!(storage::MINIO_PORT > 0);
//! ```

/// Default MinIO/S3 port
pub const MINIO_PORT: u16 = 9000;

/// Default Redis port
pub const REDIS_PORT: u16 = 6379;

/// Default `PostgreSQL` port
pub const POSTGRES_PORT: u16 = 5432;

/// Default AMQP (`RabbitMQ`) port
pub const AMQP_PORT: u16 = 5672;
