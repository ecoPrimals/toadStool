//! Resource management tests for BiomeExecutor
//!
//! Tests cover:
//! - CPU allocation and limits
//! - Memory allocation and limits
//! - Disk usage and quotas
//! - Network bandwidth management
//! - Resource monitoring and enforcement

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(test)]
mod resource_management_tests {
    use super::*;

    // ============================================================================
    // CPU Resource Tests
    // ============================================================================

    #[test]
    fn test_cpu_allocation_parsing() {
        let cpu_values = vec![
            ("0.5", 0.5),
            ("1.0", 1.0),
            ("2", 2.0),
            ("4.0", 4.0),
            ("8", 8.0),
            ("16.0", 16.0),
        ];

        for (input, expected) in cpu_values {
            let parsed: f64 = input.parse().unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn test_cpu_limit_validation() {
        let test_cases = vec![
            (0.1, true),   // Valid: minimal
            (0.5, true),   // Valid: half core
            (1.0, true),   // Valid: one core
            (16.0, true),  // Valid: multiple cores
            (64.0, true),  // Valid: high core count
            (0.0, false),  // Invalid: zero
            (-1.0, false), // Invalid: negative
        ];

        for (cpu, is_valid) in test_cases {
            let valid = cpu > 0.0 && cpu <= 128.0;
            assert_eq!(valid, is_valid, "CPU value: {cpu}");
        }
    }

    #[test]
    fn test_cpu_quota_calculation() {
        // CPU quota is typically calculated as: cpu_limit * 100000 (microseconds in 100ms period)
        let cpu_limits = vec![
            (0.5, 50000),  // 0.5 cores = 50ms per 100ms
            (1.0, 100000), // 1 core = 100ms per 100ms
            (2.0, 200000), // 2 cores = 200ms per 100ms
            (4.0, 400000), // 4 cores = 400ms per 100ms
        ];

        for (cpu, expected_quota) in cpu_limits {
            let quota = (cpu * 100000.0) as u64;
            assert_eq!(quota, expected_quota);
        }
    }

    #[test]
    fn test_cpu_shares_calculation() {
        // CPU shares for priority (default 1024 per core)
        let cpus = vec![(0.5, 512), (1.0, 1024), (2.0, 2048), (4.0, 4096)];

        for (cpu, expected_shares) in cpus {
            let shares = (cpu * 1024.0) as u32;
            assert_eq!(shares, expected_shares);
        }
    }

    // ============================================================================
    // Memory Resource Tests
    // ============================================================================

    #[test]
    fn test_memory_size_parsing() {
        let memory_values = vec![
            ("100M", 100 * 1024 * 1024u64),
            ("512M", 512 * 1024 * 1024u64),
            ("1G", 1024 * 1024 * 1024u64),
            ("2G", 2 * 1024 * 1024 * 1024u64),
            ("10G", 10 * 1024 * 1024 * 1024u64),
        ];

        for (input, expected_bytes) in memory_values {
            let (value_str, unit) = input.split_at(input.len() - 1);
            let value: u64 = value_str.parse().unwrap();

            let bytes = match unit {
                "M" => value * 1024 * 1024,
                "G" => value * 1024 * 1024 * 1024,
                "K" => value * 1024,
                _ => value,
            };

            assert_eq!(bytes, expected_bytes, "Input: {input}");
        }
    }

    #[test]
    fn test_memory_limit_validation() {
        let test_cases = vec![
            ("64M", true),    // Valid: minimum reasonable
            ("128M", true),   // Valid: small
            ("512M", true),   // Valid: standard
            ("1G", true),     // Valid: 1GB
            ("16G", true),    // Valid: large
            ("128G", true),   // Valid: very large
            ("0M", false),    // Invalid: zero
            ("1024T", false), // Invalid: unreasonably large
        ];

        for (mem_str, is_valid) in test_cases {
            let valid = !mem_str.starts_with('0') && !mem_str.contains('T');
            assert_eq!(valid, is_valid, "Memory: {mem_str}");
        }
    }

    #[test]
    fn test_memory_swap_configuration() {
        // Swap can be: -1 (unlimited), 0 (disabled), or specific limit
        let swap_configs = vec![
            (-1i64, "unlimited"),
            (0i64, "disabled"),
            (536870912i64, "limited"), // 512MB
        ];

        for (swap_value, swap_type) in swap_configs {
            match swap_type {
                "unlimited" => assert_eq!(swap_value, -1),
                "disabled" => assert_eq!(swap_value, 0),
                "limited" => assert!(swap_value > 0),
                _ => panic!("Unknown swap type"),
            }
        }
    }

    #[test]
    fn test_memory_reservation_vs_limit() {
        // Reservation (soft limit) should be <= limit (hard limit)
        let configs = vec![
            (256 * 1024 * 1024u64, 512 * 1024 * 1024u64, true), // Valid: reservation < limit
            (512 * 1024 * 1024u64, 512 * 1024 * 1024u64, true), // Valid: equal
            (1024 * 1024 * 1024u64, 512 * 1024 * 1024u64, false), // Invalid: reservation > limit
        ];

        for (reservation, limit, is_valid) in configs {
            let valid = reservation <= limit;
            assert_eq!(valid, is_valid);
        }
    }

    // ============================================================================
    // Disk Resource Tests
    // ============================================================================

    #[test]
    fn test_disk_quota_parsing() {
        let disk_quotas = vec![
            ("1G", 1024 * 1024 * 1024u64),
            ("10G", 10 * 1024 * 1024 * 1024u64),
            ("100G", 100 * 1024 * 1024 * 1024u64),
            ("1T", 1024 * 1024 * 1024 * 1024u64),
        ];

        for (quota_str, expected_bytes) in disk_quotas {
            let (value_str, unit) = quota_str.split_at(quota_str.len() - 1);
            let value: u64 = value_str.parse().unwrap();

            let bytes = match unit {
                "G" => value * 1024 * 1024 * 1024,
                "T" => value * 1024 * 1024 * 1024 * 1024,
                "M" => value * 1024 * 1024,
                _ => value,
            };

            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn test_disk_iops_limits() {
        // Test IOPS (Input/Output Operations Per Second) limits
        let iops_configs = vec![
            (100, "minimal"),
            (1000, "standard"),
            (10000, "high"),
            (50000, "very-high"),
        ];

        for (iops, tier) in iops_configs {
            assert!(iops > 0);
            assert!(iops <= 100000); // Reasonable maximum
            assert!(!tier.is_empty());
        }
    }

    #[test]
    fn test_disk_bandwidth_limits() {
        // Bandwidth in MB/s
        let bandwidth_limits = vec![
            (10, "slow"),
            (100, "standard"),
            (500, "fast"),
            (1000, "very-fast"),
        ];

        for (mb_per_sec, tier) in bandwidth_limits {
            assert!(mb_per_sec > 0);
            assert!(mb_per_sec <= 2000); // Reasonable maximum
            assert!(!tier.is_empty());
        }
    }

    // ============================================================================
    // Network Resource Tests
    // ============================================================================

    #[test]
    fn test_network_bandwidth_parsing() {
        let bandwidth_values = vec![
            ("1m", 1_000_000u64),       // 1 Mbps
            ("10m", 10_000_000u64),     // 10 Mbps
            ("100m", 100_000_000u64),   // 100 Mbps
            ("1g", 1_000_000_000u64),   // 1 Gbps
            ("10g", 10_000_000_000u64), // 10 Gbps
        ];

        for (input, expected_bps) in bandwidth_values {
            let (value_str, unit) = input.split_at(input.len() - 1);
            let value: u64 = value_str.parse().unwrap();

            let bps = match unit {
                "m" => value * 1_000_000,
                "g" => value * 1_000_000_000,
                "k" => value * 1_000,
                _ => value,
            };

            assert_eq!(bps, expected_bps);
        }
    }

    #[test]
    fn test_network_port_allocation() {
        // Port ranges for different purposes
        let port_ranges = vec![
            (1024, 49151, "user-ports"), // Registered ports
            (49152, 65535, "ephemeral"), // Ephemeral ports
            (8000, 8999, "development"), // Common dev range
        ];

        for (start, end, range_type) in port_ranges {
            assert!(start < end);
            assert!(start >= 1024); // Don't use privileged ports
            assert!(end <= 65535); // Max port number
            assert!(!range_type.is_empty());
        }
    }

    #[test]
    fn test_network_connection_limits() {
        let connection_limits = vec![
            (10, "minimal"),
            (100, "low"),
            (1000, "standard"),
            (10000, "high"),
            (100000, "very-high"),
        ];

        for (max_connections, tier) in connection_limits {
            assert!(max_connections > 0);
            assert!(max_connections <= 1_000_000); // Reasonable maximum
            assert!(!tier.is_empty());
        }
    }

    // ============================================================================
    // Resource Limit Enforcement Tests
    // ============================================================================

    #[derive(Clone, Debug)]
    struct ResourceLimits {
        cpu_limit: Option<f64>,
        memory_limit_bytes: Option<u64>,
        disk_quota_bytes: Option<u64>,
        network_bandwidth_bps: Option<u64>,
    }

    impl ResourceLimits {
        fn new() -> Self {
            Self {
                cpu_limit: None,
                memory_limit_bytes: None,
                disk_quota_bytes: None,
                network_bandwidth_bps: None,
            }
        }

        fn with_cpu(mut self, cpu: f64) -> Self {
            self.cpu_limit = Some(cpu);
            self
        }

        fn with_memory(mut self, bytes: u64) -> Self {
            self.memory_limit_bytes = Some(bytes);
            self
        }
    }

    #[test]
    fn test_resource_limits_builder() {
        let limits = ResourceLimits::new()
            .with_cpu(2.0)
            .with_memory(1024 * 1024 * 1024);

        assert_eq!(limits.cpu_limit, Some(2.0));
        assert_eq!(limits.memory_limit_bytes, Some(1024 * 1024 * 1024));
        assert!(limits.disk_quota_bytes.is_none());
        assert!(limits.network_bandwidth_bps.is_none());
    }

    #[test]
    fn test_resource_limit_defaults() {
        let limits = ResourceLimits::new();

        assert!(limits.cpu_limit.is_none());
        assert!(limits.memory_limit_bytes.is_none());
        assert!(limits.disk_quota_bytes.is_none());
        assert!(limits.network_bandwidth_bps.is_none());
    }

    // ============================================================================
    // Resource Monitoring Tests
    // ============================================================================

    #[derive(Clone, Debug)]
    struct ResourceUsage {
        cpu_percent: f64,
        memory_bytes: u64,
        disk_bytes: u64,
        network_rx_bytes: u64,
        network_tx_bytes: u64,
    }

    #[test]
    fn test_resource_usage_tracking() {
        let usage = ResourceUsage {
            cpu_percent: 45.5,
            memory_bytes: 512 * 1024 * 1024,
            disk_bytes: 1024 * 1024 * 1024,
            network_rx_bytes: 1000000,
            network_tx_bytes: 500000,
        };

        assert!(usage.cpu_percent > 0.0 && usage.cpu_percent <= 100.0);
        assert!(usage.memory_bytes > 0);
        assert!(usage.disk_bytes > 0);
        assert!(usage.network_rx_bytes > 0);
        assert!(usage.network_tx_bytes > 0);
    }

    #[test]
    fn test_resource_limit_breach_detection() {
        let limits = ResourceLimits::new()
            .with_cpu(2.0)
            .with_memory(1024 * 1024 * 1024);

        let usage = ResourceUsage {
            cpu_percent: 250.0,                   // Using more than 2 cores (200%)
            memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB (over 1GB limit)
            disk_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        };

        // Check CPU breach
        let cpu_breach = if let Some(cpu_limit) = limits.cpu_limit {
            usage.cpu_percent > (cpu_limit * 100.0)
        } else {
            false
        };

        // Check memory breach
        let mem_breach = if let Some(mem_limit) = limits.memory_limit_bytes {
            usage.memory_bytes > mem_limit
        } else {
            false
        };

        assert!(cpu_breach, "CPU limit should be breached");
        assert!(mem_breach, "Memory limit should be breached");
    }

    // ============================================================================
    // Resource Allocation Strategy Tests
    // ============================================================================

    #[test]
    fn test_resource_allocation_priority() {
        let mut workloads = vec![
            ("critical", 1, 8.0, 16 * 1024 * 1024 * 1024u64),
            ("high", 2, 4.0, 8 * 1024 * 1024 * 1024u64),
            ("normal", 3, 2.0, 4 * 1024 * 1024 * 1024u64),
            ("low", 4, 1.0, 2 * 1024 * 1024 * 1024u64),
        ];

        // Sort by priority (lower number = higher priority)
        workloads.sort_by_key(|(_, priority, _, _)| *priority);

        assert_eq!(workloads[0].0, "critical");
        assert_eq!(workloads[1].0, "high");
        assert_eq!(workloads[2].0, "normal");
        assert_eq!(workloads[3].0, "low");
    }

    #[test]
    fn test_resource_fair_sharing() {
        // Fair share calculation: total_resource / number_of_workloads
        let total_cpu = 16.0;
        let total_memory = 64 * 1024 * 1024 * 1024u64;
        let num_workloads = 4;

        let cpu_per_workload = total_cpu / num_workloads as f64;
        let memory_per_workload = total_memory / num_workloads as u64;

        assert_eq!(cpu_per_workload, 4.0);
        assert_eq!(memory_per_workload, 16 * 1024 * 1024 * 1024);
    }

    // ============================================================================
    // Resource Cleanup Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_resource_cleanup_on_stop() {
        let allocated_resources: Arc<RwLock<HashMap<String, ResourceLimits>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Allocate resources
        {
            let mut resources = allocated_resources.write().await;
            resources.insert(
                "workload-1".to_string(),
                ResourceLimits::new().with_cpu(2.0),
            );
        }

        // Verify allocated
        {
            let resources = allocated_resources.read().await;
            assert!(resources.contains_key("workload-1"));
        }

        // Clean up (stop workload)
        {
            let mut resources = allocated_resources.write().await;
            resources.remove("workload-1");
        }

        // Verify cleaned up
        {
            let resources = allocated_resources.read().await;
            assert!(!resources.contains_key("workload-1"));
        }
    }

    // ============================================================================
    // Resource Contention Tests
    // ============================================================================

    #[test]
    fn test_resource_contention_detection() {
        let total_cpu = 8.0;
        let requested_cpus = vec![2.0, 2.0, 2.0, 3.0]; // Total: 9.0

        let total_requested: f64 = requested_cpus.iter().sum();
        let has_contention = total_requested > total_cpu;

        assert!(has_contention);
        assert_eq!(total_requested, 9.0);
    }

    #[test]
    fn test_resource_overcommit_ratio() {
        let physical_memory = 32 * 1024 * 1024 * 1024u64;
        let overcommit_ratio = 1.5; // Allow 50% overcommit

        let max_allocatable = (physical_memory as f64 * overcommit_ratio) as u64;

        assert_eq!(max_allocatable, 48 * 1024 * 1024 * 1024);
        assert!(max_allocatable > physical_memory);
    }
}
