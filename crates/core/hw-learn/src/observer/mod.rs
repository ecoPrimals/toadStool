// SPDX-License-Identifier: AGPL-3.0-only
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
    /// MMIO register write: offset, value, width in bytes.
    RegisterWrite { offset: u64, value: u64, width: u8 },
    /// MMIO register read: offset, returned value.
    RegisterRead { offset: u64, value: u64, width: u8 },
    /// DRM ioctl call: ioctl number, argument size, success/failure.
    IoctlCall { ioctl_nr: u64, arg_size: u32, success: bool },
    /// Firmware load: engine name, firmware path.
    FirmwareLoad { engine: String, path: String },
    /// GSP RPC message: function ID, payload size.
    GspRpc { func_id: u32, payload_size: u32, direction: RpcDirection },
    /// AMD PM4 packet: opcode, word count.
    Pm4Packet { opcode: u16, count: u16 },
    /// Intel batch buffer command: opcode, dword count.
    BatchCommand { opcode: u32, dwords: u16 },
    /// Delay/gap in trace (no activity for this duration).
    Gap { duration_us: u64 },
}

/// Direction of a GSP RPC message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RpcDirection {
    HostToGsp,
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
#[derive(Debug)]
pub enum ObserveError {
    /// Trace facility not available (kernel config, permissions).
    TraceUnavailable(String),
    /// Failed to parse trace output.
    ParseError(String),
    /// IO error reading trace file.
    Io(std::io::Error),
    /// GPU not found.
    GpuNotFound(String),
}

impl std::fmt::Display for ObserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TraceUnavailable(msg) => write!(f, "trace unavailable: {msg}"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::GpuNotFound(sel) => write!(f, "GPU not found: {sel}"),
        }
    }
}

impl std::error::Error for ObserveError {}

impl From<std::io::Error> for ObserveError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
