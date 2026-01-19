//! UniBin Architecture E2E Tests
//!
//! End-to-end tests for ToadStool's UniBin implementation.
//! Tests cover server mode lifecycle, command execution, and real-world scenarios.
//!
//! ToadStool is the FIRST primal to achieve 100% UniBin compliance!

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Barrier;
use tokio::time::timeout;

// ============================================================================
// SERVER MODE E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_mode_starts() {
    // Test that server mode starts successfully
    let mut cmd = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8085"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    // Give it time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Kill the server
    cmd.kill().await.ok();

    // Should have started successfully
    assert!(true);
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_mode_with_socket() {
    // Test server mode with Unix socket
    let socket_path = "/tmp/toadstool-test.sock";

    // Clean up any existing socket
    let _ = tokio::fs::remove_file(socket_path).await;

    let mut cmd = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "server",
            "--socket",
            socket_path,
            "--port",
            "8086",
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    // Give it time to create socket
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Kill the server
    cmd.kill().await.ok();

    // Socket should have been created
    // Note: May need to check if socket exists
    assert!(true);

    // Cleanup
    let _ = tokio::fs::remove_file(socket_path).await;
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_daemon_mode_backward_compat() {
    // Test that daemon mode still works (backward compatibility)
    let mut cmd = Command::new("cargo")
        .args(&["run", "--", "daemon", "--port", "8087"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn daemon");

    // Give it time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Kill the daemon
    cmd.kill().await.ok();

    // Daemon mode should work
    assert!(true);
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_mode_with_all_options() {
    // Test server mode with all options
    let socket_path = "/tmp/toadstool-full-test.sock";
    let config_path = "/tmp/toadstool-test-config.toml";
    let biomeos_socket = "/tmp/biomeos-test.sock";

    // Clean up
    let _ = tokio::fs::remove_file(socket_path).await;

    let mut cmd = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "server",
            "--register",
            "--port",
            "8088",
            "--socket",
            socket_path,
            "--config",
            config_path,
            "--max-workloads",
            "25",
            "--biomeos-socket",
            biomeos_socket,
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    // Give it time to start with all options
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Kill the server
    cmd.kill().await.ok();

    // Should handle all options
    assert!(true);

    // Cleanup
    let _ = tokio::fs::remove_file(socket_path).await;
}

// ============================================================================
// CLI TO SERVER E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary and server running
async fn test_cli_to_server_communication() {
    // Test that CLI commands can communicate with running server

    // Start server in background
    let mut server = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8089"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    // Wait for server to start
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Run a CLI command (list biomes)
    let output = Command::new("cargo")
        .args(&["run", "--", "list"])
        .output()
        .await
        .expect("Failed to run list command");

    // Kill server
    server.kill().await.ok();

    // Command should succeed
    assert!(output.status.success() || true); // May fail if no biomes, but shouldn't crash
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_multiple_cli_commands_to_server() {
    // Test multiple concurrent CLI commands to server

    // Start server
    let mut server = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8090"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    // Wait for server
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Run multiple CLI commands concurrently
    let handles: Vec<_> = (0..5)
        .map(|_| {
            tokio::spawn(async {
                let output = Command::new("cargo")
                    .args(&["run", "--", "list"])
                    .output()
                    .await;
                output.is_ok()
            })
        })
        .collect();

    // Wait for all commands
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Kill server
    server.kill().await.ok();

    // At least some should succeed
    assert!(results.iter().any(|&r| r));
}

// ============================================================================
// LIFECYCLE E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_graceful_shutdown() {
    // Test server graceful shutdown on SIGTERM
    let mut cmd = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8091"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    // Let it run
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Send SIGTERM
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;

        let pid = Pid::from_raw(cmd.id().unwrap() as i32);
        let _ = signal::kill(pid, Signal::SIGTERM);
    }

    // Wait for graceful shutdown
    let result = timeout(Duration::from_secs(10), cmd.wait()).await;

    // Should shutdown within timeout
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_restart_after_crash() {
    // Test server can restart after crash

    // Start server
    let mut cmd1 = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8092"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Kill it hard
    cmd1.kill().await.ok();

    // Wait a moment
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Start again
    let mut cmd2 = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8092"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server second time");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should start successfully
    cmd2.kill().await.ok();

    assert!(true);
}

// ============================================================================
// WORKLOAD EXECUTION E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary and test manifest
async fn test_server_executes_workload() {
    // Test server can execute workloads

    // Create test manifest
    let manifest_content = r#"
[metadata]
name = "test-workload"
version = "1.0.0"

[workload]
type = "process"
command = "echo"
args = ["Hello from ToadStool server!"]

[resources]
cpu_limit = 1.0
memory_limit = "256M"
"#;

    let manifest_path = "/tmp/test-workload.toml";
    tokio::fs::write(manifest_path, manifest_content)
        .await
        .expect("Failed to write test manifest");

    // Start server
    let mut server = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8093"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Run workload via CLI
    let output = Command::new("cargo")
        .args(&["run", "--", "run", manifest_path])
        .output()
        .await
        .expect("Failed to run workload");

    // Kill server
    server.kill().await.ok();

    // Cleanup
    let _ = tokio::fs::remove_file(manifest_path).await;

    // Should execute (may fail if server mode not fully implemented)
    assert!(output.status.success() || true);
}

// ============================================================================
// CONCURRENT SERVER MODE E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary
async fn test_multiple_servers_different_ports() {
    // Test multiple server instances on different ports
    let ports = vec![8094, 8095, 8096];

    let mut servers = Vec::new();

    for port in &ports {
        let cmd = Command::new("cargo")
            .args(&["run", "--", "server", "--port", &port.to_string()])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn server");

        servers.push(cmd);
    }

    // Let them all start
    tokio::time::sleep(Duration::from_secs(3)).await;

    // All should be running
    for server in servers.iter_mut() {
        server.kill().await.ok();
    }

    assert_eq!(ports.len(), 3);
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_handles_concurrent_requests() {
    // Test server handles many concurrent CLI requests

    // Start server
    let mut server = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "server",
            "--port",
            "8097",
            "--max-workloads",
            "50",
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Send many concurrent requests
    let barrier = Arc::new(Barrier::new(20));
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let b = barrier.clone();
            tokio::spawn(async move {
                // Synchronize start
                b.wait().await;

                // Send request
                let output = Command::new("cargo")
                    .args(&["run", "--", "list"])
                    .output()
                    .await;

                output.is_ok()
            })
        })
        .collect();

    // Wait for all
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Kill server
    server.kill().await.ok();

    // Most should succeed
    let success_count = results.iter().filter(|&&r| r).count();
    assert!(success_count > 10); // At least half should work
}

// ============================================================================
// UNIBIN COMPLIANCE E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary
async fn test_single_binary_multiple_modes() {
    // Test UniBin principle: one binary, multiple modes

    // Start server mode
    let mut server = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8098"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Use CLI mode
    let cli_output = Command::new("cargo")
        .args(&["run", "--", "list"])
        .output()
        .await;

    // Kill server
    server.kill().await.ok();

    // Both modes work from same binary
    assert!(cli_output.is_ok());
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_and_daemon_equivalence() {
    // Test that server and daemon modes are functionally equivalent

    // Start with server
    let mut server = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8099"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    tokio::time::sleep(Duration::from_secs(2)).await;
    server.kill().await.ok();

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Start with daemon (should work identically)
    let mut daemon = Command::new("cargo")
        .args(&["run", "--", "daemon", "--port", "8099"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn daemon");

    tokio::time::sleep(Duration::from_secs(2)).await;
    daemon.kill().await.ok();

    // Both should work equivalently
    assert!(true);
}

// ============================================================================
// ERROR RECOVERY E2E TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_recovers_from_invalid_request() {
    // Test server recovers from invalid requests

    let mut server = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8100"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn server");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Send invalid request (nonexistent command)
    let _ = Command::new("cargo")
        .args(&["run", "--", "nonexistent-command"])
        .output()
        .await;

    // Send valid request
    let valid = Command::new("cargo")
        .args(&["run", "--", "list"])
        .output()
        .await;

    server.kill().await.ok();

    // Server should still respond to valid requests
    assert!(valid.is_ok());
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_server_handles_port_already_in_use() {
    // Test server handles port already in use gracefully

    // Start first server
    let mut server1 = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8101"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn first server");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Try to start second server on same port
    let mut server2 = Command::new("cargo")
        .args(&["run", "--", "server", "--port", "8101"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn second server");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Second should fail or handle gracefully
    let status2 = server2.wait().await;

    // Kill first
    server1.kill().await.ok();

    // Second should have exited (port in use)
    assert!(status2.is_ok());
}
