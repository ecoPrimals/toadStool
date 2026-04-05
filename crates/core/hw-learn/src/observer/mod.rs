// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU init observation — vendor-neutral trace capture.
//!
//! Parses output from kernel tracing facilities (mmiotrace, strace, dmesg)
//! into a unified `TraceEvent` stream for the distiller.

pub mod amd_pm4;
pub mod gsp_rpc;
pub mod intel_batch;
pub mod ioctl_trace;
pub mod mmio_trace;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which tracing facility to use for observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceMode {
    /// Kernel mmiotrace — captures all MMIO register writes.
    /// Requires: `echo mmiotrace > /sys/kernel/tracing/current_tracer`
    MmioTrace,
    /// strace on the DRM fd — captures all DRM/UVM ioctls.
    IoctlTrace,
    /// nouveau GSP RPC debug log from dmesg.
    GspRpc,
    /// AMD PM4 command stream capture.
    AmdPm4,
    /// Intel batch buffer log.
    IntelBatch,
}

/// Configuration for a trace observation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveConfig {
    /// Which GPU to observe (by card index or PCI slot).
    pub gpu_selector: GpuSelector,
    /// Which trace mode to use.
    pub mode: TraceMode,
    /// Path to raw trace output (mmiotrace log, strace log, etc.).
    pub trace_path: Option<PathBuf>,
    /// Whether to trigger a compute init during observation.
    pub trigger_compute: bool,
}

/// How to select the GPU to observe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuSelector {
    /// By DRM card index (e.g., card0).
    CardIndex(u32),
    /// By PCI slot (e.g., "0000:25:00.0").
    PciSlot(String),
    /// Auto-detect: pick the first working GPU.
    Auto,
}

/// A single trace event from any vendor's tracing facility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Monotonic timestamp in microseconds from trace start.
    pub timestamp_us: u64,
    /// Event kind.
    pub kind: TraceEventKind,
    /// Vendor-specific context (driver name, device path).
    pub context: String,
}

/// Vendor-neutral classification of trace events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceEventKind {
    /// MMIO register write.
    RegisterWrite {
        /// BAR-relative register offset.
        offset: u64,
        /// Value written.
        value: u64,
        /// Access width in bytes (4 or 8).
        width: u8,
    },
    /// MMIO register read.
    RegisterRead {
        /// BAR-relative register offset.
        offset: u64,
        /// Value read back.
        value: u64,
        /// Access width in bytes (4 or 8).
        width: u8,
    },
    /// DRM ioctl call.
    IoctlCall {
        /// DRM ioctl number.
        ioctl_nr: u64,
        /// Argument buffer size.
        arg_size: u32,
        /// Whether the ioctl succeeded.
        success: bool,
    },
    /// Firmware load.
    FirmwareLoad {
        /// Engine name (e.g., MEC, GSP).
        engine: String,
        /// Path to firmware blob.
        path: String,
    },
    /// GSP RPC message (nouveau).
    GspRpc {
        /// RPC function ID.
        func_id: u32,
        /// Payload size in bytes.
        payload_size: u32,
        /// Message direction.
        direction: RpcDirection,
    },
    /// AMD PM4 packet.
    Pm4Packet {
        /// PM4 opcode.
        opcode: u16,
        /// Word count in packet.
        count: u16,
    },
    /// Intel batch buffer command.
    BatchCommand {
        /// Command opcode.
        opcode: u32,
        /// Dword count.
        dwords: u16,
    },
    /// Delay/gap in trace (no activity for this duration).
    Gap {
        /// Gap duration in microseconds.
        duration_us: u64,
    },
}

/// Direction of a GSP RPC message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RpcDirection {
    /// Host (CPU) to GSP (GPU System Processor).
    HostToGsp,
    /// GSP to host.
    GspToHost,
}

/// Result of an observation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveResult {
    /// Observed GPU identifier.
    pub gpu_id: String,
    /// Driver used during observation.
    pub driver: String,
    /// Ordered trace events.
    pub events: Vec<TraceEvent>,
    /// Whether compute was successfully triggered during observation.
    pub compute_triggered: bool,
    /// Total observation duration in microseconds.
    pub duration_us: u64,
}

/// Main observer entry point.
pub struct TraceObserver;

impl TraceObserver {
    /// Run an observation session on the specified GPU.
    ///
    /// Returns the raw trace events for distillation.
    ///
    /// # Errors
    /// Returns `Err` if the trace path is missing, the trace file cannot be read, or parsing fails.
    pub fn observe(config: &ObserveConfig) -> Result<ObserveResult, ObserveError> {
        match &config.mode {
            TraceMode::MmioTrace => mmio_trace::parse_mmiotrace(config),
            TraceMode::IoctlTrace => ioctl_trace::parse_ioctl_trace(config),
            TraceMode::GspRpc => gsp_rpc::parse_gsp_rpc(config),
            TraceMode::AmdPm4 => amd_pm4::parse_pm4_trace(config),
            TraceMode::IntelBatch => intel_batch::parse_batch_trace(config),
        }
    }
}

/// Observation errors.
#[derive(Debug, thiserror::Error)]
pub enum ObserveError {
    /// Trace facility not available (kernel config, permissions).
    #[error("trace unavailable: {0}")]
    TraceUnavailable(String),
    /// Failed to parse trace output.
    #[error("parse error: {0}")]
    ParseError(String),
    /// IO error reading trace file.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// GPU not found.
    #[error("GPU not found: {0}")]
    GpuNotFound(String),
}
