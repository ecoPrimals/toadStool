// SPDX-License-Identifier: AGPL-3.0-or-later
//! MMIO and Falcon JSON-RPC handlers.
//!
//! Provides BAR0 register access and falcon microcontroller status via
//! `mmio.read32`, `mmio.write32`, `mmio.batch`, `mmio.pramin.read32`,
//! `mmio.bar0.probe`, `mmio.falcon.status`, and ember falcon/PRAMIN writers
//! (`ember.falcon.upload_imem`, `ember.falcon.upload_dmem`, `ember.falcon.start_cpu`,
//! `ember.falcon.poll`, `ember.pramin.write`, `ember.pramin.read`), plus ember
//! device probes (`ember.fecs.state`, `ember.device.health`, `ember.device.recover`).
//!
//! All MMIO operations use fork-isolated access via `toadstool-cylinder`'s
//! `SysfsBar0` or `MappedBar` to protect the daemon from hardware hangs.

use crate::pure_jsonrpc::types::JsonRpcError;
use serde_json::Value;
use tracing::debug;

/// Well-known falcon engine base addresses (BAR0-relative).
mod falcon_bases {
    pub const PMU: u32 = 0x0010_A000;
    pub const FECS: u32 = 0x0040_9800;
    pub const GPCCS: u32 = 0x0050_2800;
    pub const SEC2: u32 = 0x0084_0000;
}

/// Falcon register offsets (relative to engine base).
mod falcon_offsets {
    pub const MAILBOX0: u32 = 0x040;
    pub const MAILBOX1: u32 = 0x044;
    pub const OS: u32 = 0x080;
    pub const CPUCTL: u32 = 0x100;
    pub const BOOTVEC: u32 = 0x104;
    pub const HWCFG: u32 = 0x108;
    pub const PC: u32 = 0x030;
    /// CPUCTL bit 1 — release falcon from HRESET (v4+).
    pub const CPUCTL_STARTCPU: u32 = 1 << 1;
    /// CPUCTL bit 5 — falcon halted.
    pub const CPUCTL_HALTED: u32 = 1 << 5;
}

const PMC_BOOT_0: u32 = 0x0000_0000;
const PMC_ENABLE: u32 = 0x0000_0200;
const PMC_MASK_FULL: u32 = 0xFFFF_FFFF;
const PRAMIN_BASE: u32 = 0x0070_0000;
const PBUS_BAR0_WINDOW: u32 = 0x0000_1700;
const PTIMER_TIME_0: u32 = 0x0000_9400;
const FECS_CPUCTL: u32 = 0x0040_9100;
const FECS_MAILBOX0: u32 = 0x0040_9040;
const FECS_MAILBOX1: u32 = 0x0040_9044;
const FECS_OS: u32 = 0x0040_9080;
const FECS_PC: u32 = 0x0040_9030;
const PRAMIN_READ_MAX: u32 = 4096;
const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

fn base64_decode(s: &str) -> Result<Vec<u8>, JsonRpcError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| JsonRpcError::invalid_params(format!("base64 decode: {e}")))
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn boot0_alive(value: u32) -> bool {
    value != 0 && value != 0xFFFF_FFFF
}

fn parse_u32_param(
    params: Option<&Value>,
    key: &str,
    default: Option<u32>,
) -> Result<u32, JsonRpcError> {
    match params.and_then(|p| p.get(key)) {
        Some(v) => v
            .as_u64()
            .or_else(|| {
                v.as_str()
                    .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                    .map(u64::from)
            })
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| JsonRpcError::invalid_params(format!("Invalid '{key}' (u32 or hex string)"))),
        None => default.ok_or_else(|| JsonRpcError::invalid_params(format!("Missing '{key}'"))),
    }
}

fn parse_base(params: Option<&Value>) -> Result<usize, JsonRpcError> {
    Ok(parse_u32_param(params, "base", None)? as usize)
}

/// Open read-write BAR0 as [`MappedBar`] for falcon PIO library calls.
fn open_mapped_bar0_rw(
    bdf: &str,
) -> Result<toadstool_cylinder::vfio::device::MappedBar, JsonRpcError> {
    toadstool_cylinder::vfio::ember_gate::check_channel(bdf)
        .map_err(|e| JsonRpcError::internal_error(format!("ember gate for {bdf}: {e}")))?;
    toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
        .map_err(|e| JsonRpcError::internal_error(format!("BAR0 open (rw) failed for {bdf}: {e}")))
}

fn parse_bdf(params: Option<&Value>) -> Result<String, JsonRpcError> {
    params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))
}

fn parse_offset(params: Option<&Value>) -> Result<u32, JsonRpcError> {
    params
        .and_then(|p| p.get("offset"))
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()).map(u64::from)))
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| JsonRpcError::invalid_params("Missing or invalid 'offset' (u32 or hex string)"))
}

/// Open a read-only sysfs BAR0 mapping for a PCI device.
fn open_sysfs_bar0(bdf: &str) -> Result<toadstool_cylinder::vfio::sysfs_bar0::SysfsBar0, JsonRpcError> {
    toadstool_cylinder::vfio::sysfs_bar0::SysfsBar0::open(bdf, DEFAULT_BAR0_SIZE)
        .map_err(|e| JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}")))
}

/// Open a read-write sysfs BAR0 mapping for a PCI device.
fn open_sysfs_bar0_rw(bdf: &str) -> Result<toadstool_cylinder::vfio::sysfs_bar0::SysfsBar0Rw, JsonRpcError> {
    toadstool_cylinder::vfio::sysfs_bar0::SysfsBar0Rw::open(bdf, DEFAULT_BAR0_SIZE)
        .map_err(|e| JsonRpcError::internal_error(format!("BAR0 open (rw) failed for {bdf}: {e}")))
}

/// `mmio.read32` — read a single 32-bit BAR0 register.
pub fn mmio_read32(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let offset = parse_offset(params)?;
    let bar = open_sysfs_bar0(&bdf)?;
    let value = bar.read_u32(offset as usize);

    debug!(bdf = %bdf, offset = format!("{offset:#010x}"), value = format!("{value:#010x}"), "mmio.read32");
    Ok(serde_json::json!({ "bdf": bdf, "offset": format!("{offset:#010x}"), "value": value }))
}

/// `mmio.write32` — write a single 32-bit BAR0 register.
pub fn mmio_write32(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let offset = parse_offset(params)?;
    let value = params
        .and_then(|p| p.get("value"))
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'value' (u32)"))?;

    let bar = open_sysfs_bar0_rw(&bdf)?;
    bar.write_u32(offset as usize, value)
        .map_err(|e| JsonRpcError::internal_error(format!("MMIO write failed: {e}")))?;

    debug!(bdf = %bdf, offset = format!("{offset:#010x}"), value = format!("{value:#010x}"), "mmio.write32");
    Ok(serde_json::json!({ "bdf": bdf, "offset": format!("{offset:#010x}"), "ok": true }))
}

/// `mmio.batch` — batch read/write operations on BAR0.
///
/// Each op: `{ "offset": u32, "value"?: u32 }`.
/// If `value` is present, it's a write; otherwise a read.
pub fn mmio_batch(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let ops = params
        .and_then(|p| p.get("ops"))
        .and_then(Value::as_array)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'ops' array"))?;

    let has_writes = ops.iter().any(|op| op.get("value").is_some());
    let bar_rw = if has_writes { Some(open_sysfs_bar0_rw(&bdf)?) } else { None };
    let bar_ro = if has_writes { None } else { Some(open_sysfs_bar0(&bdf)?) };
    let mut results = Vec::with_capacity(ops.len());

    for op in ops {
        let offset = op
            .get("offset")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| JsonRpcError::invalid_params("Each op needs 'offset' (u32)"))?;

        if let Some(val) = op.get("value").and_then(Value::as_u64) {
            let val32 = u32::try_from(val)
                .map_err(|_| JsonRpcError::invalid_params("value exceeds u32"))?;
            if let Some(ref rw) = bar_rw {
                match rw.write_u32(offset as usize, val32) {
                    Ok(()) => results.push(serde_json::json!({ "offset": offset, "wrote": val32 })),
                    Err(e) => results.push(serde_json::json!({ "offset": offset, "error": e.to_string() })),
                }
            }
        } else if let Some(ref rw) = bar_rw {
            let v = rw.read_u32(offset as usize);
            results.push(serde_json::json!({ "offset": offset, "value": v }));
        } else if let Some(ref ro) = bar_ro {
            let v = ro.read_u32(offset as usize);
            results.push(serde_json::json!({ "offset": offset, "value": v }));
        }
    }

    debug!(bdf = %bdf, count = results.len(), "mmio.batch");
    Ok(serde_json::json!({ "bdf": bdf, "results": results }))
}

/// `mmio.pramin.read32` — read a 32-bit value from PRAMIN window.
///
/// PRAMIN is the PMC window into VRAM/RAMIN at BAR0+0x700000.
pub fn mmio_pramin_read32(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let offset = parse_offset(params)?;

    let phys_offset = PRAMIN_BASE.checked_add(offset).ok_or_else(|| {
        JsonRpcError::invalid_params("PRAMIN offset overflow")
    })?;

    let bar = open_sysfs_bar0(&bdf)?;
    let value = bar.read_u32(phys_offset as usize);

    debug!(bdf = %bdf, offset = format!("{offset:#010x}"), value = format!("{value:#010x}"), "mmio.pramin.read32");
    Ok(serde_json::json!({ "bdf": bdf, "offset": format!("{offset:#010x}"), "value": value }))
}

/// `mmio.bar0.probe` — read chip identity and PMC state from BAR0.
pub fn mmio_bar0_probe(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let bar = open_sysfs_bar0(&bdf)?;

    let boot0 = bar.read_u32(PMC_BOOT_0 as usize);
    let pmc_enable = bar.read_u32(PMC_ENABLE as usize);
    let chip_id = (boot0 >> 20) & 0x1FF;
    let vendor = boot0 & 0xFFF;

    debug!(bdf = %bdf, boot0 = format!("{boot0:#010x}"), chip_id = format!("{chip_id:#05x}"), "mmio.bar0.probe");
    Ok(serde_json::json!({
        "bdf": bdf,
        "boot0": boot0,
        "chip_id": chip_id,
        "vendor": vendor,
        "pmc_enable": pmc_enable,
        "responsive": boot0 != 0 && boot0 != 0xFFFF_FFFF,
    }))
}

/// `mmio.falcon.status` — read falcon microcontroller registers.
pub fn mmio_falcon_status(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let engine = params
        .and_then(|p| p.get("engine"))
        .and_then(Value::as_str)
        .unwrap_or("pmu");

    let base = match engine.to_ascii_lowercase().as_str() {
        "pmu" => falcon_bases::PMU,
        "fecs" => falcon_bases::FECS,
        "gpccs" => falcon_bases::GPCCS,
        "sec2" => falcon_bases::SEC2,
        other => {
            return Err(JsonRpcError::invalid_params(format!(
                "Unknown falcon engine '{other}'. Valid: pmu, fecs, gpccs, sec2"
            )));
        }
    };

    let bar = open_sysfs_bar0(&bdf)?;
    let read = |off: u32| bar.read_u32((base + off) as usize);

    let cpuctl = read(falcon_offsets::CPUCTL);
    let mailbox0 = read(falcon_offsets::MAILBOX0);
    let mailbox1 = read(falcon_offsets::MAILBOX1);
    let os = read(falcon_offsets::OS);
    let bootvec = read(falcon_offsets::BOOTVEC);
    let hwcfg = read(falcon_offsets::HWCFG);

    let halted = (cpuctl & 0x20) != 0;

    debug!(bdf = %bdf, engine, cpuctl = format!("{cpuctl:#010x}"), halted, "mmio.falcon.status");
    Ok(serde_json::json!({
        "bdf": bdf,
        "engine": engine,
        "cpuctl": cpuctl,
        "mailbox0": mailbox0,
        "mailbox1": mailbox1,
        "os": os,
        "bootvec": bootvec,
        "hwcfg": hwcfg,
        "halted": halted,
    }))
}

/// `ember.falcon.upload_imem` — upload firmware to falcon IMEM via PIO ports.
pub fn falcon_upload_imem(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let base = parse_base(params)?;
    let addr = parse_u32_param(params, "offset", Some(0))?;
    let data_b64 = params
        .and_then(|p| p.get("data_b64"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'data_b64' (base64 firmware)"))?;
    let secure = params
        .and_then(|p| p.get("secure"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let data = base64_decode(data_b64)?;
    let bar = open_mapped_bar0_rw(&bdf)?;

    toadstool_cylinder::nv::falcon_pio::falcon_upload_imem(&bar, base, addr, &data, secure);

    debug!(bdf = %bdf, base, addr, bytes = data.len(), secure, "ember.falcon.upload_imem");
    Ok(serde_json::json!({
        "bdf": bdf,
        "base": base,
        "offset": addr,
        "bytes_written": data.len(),
        "secure": secure,
    }))
}

/// `ember.falcon.upload_dmem` — upload data to falcon DMEM via PIO ports.
pub fn falcon_upload_dmem(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let base = parse_base(params)?;
    let addr = parse_u32_param(params, "offset", Some(0))?;
    let data_b64 = params
        .and_then(|p| p.get("data_b64"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'data_b64' (base64 payload)"))?;

    let data = base64_decode(data_b64)?;
    let bar = open_mapped_bar0_rw(&bdf)?;

    toadstool_cylinder::nv::falcon_pio::falcon_upload_dmem(&bar, base, addr, &data);

    debug!(bdf = %bdf, base, addr, bytes = data.len(), "ember.falcon.upload_dmem");
    Ok(serde_json::json!({
        "bdf": bdf,
        "base": base,
        "offset": addr,
        "bytes_written": data.len(),
    }))
}

/// `ember.falcon.start_cpu` — set boot vector and release falcon from HRESET.
pub fn falcon_start_cpu(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let base = parse_base(params)?;
    let bootvec = parse_u32_param(params, "bootvec", None)?;
    let bar = open_sysfs_bar0_rw(&bdf)?;

    bar.write_u32(base + falcon_offsets::BOOTVEC as usize, bootvec)
        .map_err(|e| JsonRpcError::internal_error(format!("BOOTVEC write failed: {e}")))?;
    bar.write_u32(
        base + falcon_offsets::CPUCTL as usize,
        falcon_offsets::CPUCTL_STARTCPU,
    )
    .map_err(|e| JsonRpcError::internal_error(format!("CPUCTL write failed: {e}")))?;

    debug!(bdf = %bdf, base, bootvec, "ember.falcon.start_cpu");
    Ok(serde_json::json!({
        "bdf": bdf,
        "bootvec": bootvec,
        "started": true,
    }))
}

/// `ember.falcon.poll` — poll falcon CPUCTL until halted or timeout.
pub fn falcon_poll(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let base = parse_base(params)?;
    let timeout_ms = params
        .and_then(|p| p.get("timeout_ms"))
        .and_then(Value::as_u64)
        .unwrap_or(5000);
    let bar = open_sysfs_bar0(&bdf)?;

    let start = std::time::Instant::now();
    let mut cpuctl;
    loop {
        cpuctl = bar.read_u32(base + falcon_offsets::CPUCTL as usize);
        let halted = cpuctl & falcon_offsets::CPUCTL_HALTED != 0;
        if halted || start.elapsed().as_millis() >= timeout_ms as u128 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    let mailbox0 = bar.read_u32(base + falcon_offsets::MAILBOX0 as usize);
    let mailbox1 = bar.read_u32(base + falcon_offsets::MAILBOX1 as usize);
    let halted = cpuctl & falcon_offsets::CPUCTL_HALTED != 0;

    debug!(
        bdf = %bdf,
        base,
        halted,
        cpuctl = format!("{cpuctl:#010x}"),
        elapsed_ms = start.elapsed().as_millis(),
        "ember.falcon.poll"
    );
    Ok(serde_json::json!({
        "bdf": bdf,
        "halted": halted,
        "cpuctl": format!("{cpuctl:#010x}"),
        "mailbox0": mailbox0,
        "mailbox1": mailbox1,
        "elapsed_ms": start.elapsed().as_millis() as u64,
    }))
}

/// `ember.fecs.state` — read FECS falcon registers via BAR0 alias offsets.
pub fn ember_fecs_state(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let bar = open_sysfs_bar0(&bdf)?;

    let cpuctl = bar.read_u32(FECS_CPUCTL as usize);
    let mailbox0 = bar.read_u32(FECS_MAILBOX0 as usize);
    let mailbox1 = bar.read_u32(FECS_MAILBOX1 as usize);
    let os = bar.read_u32(FECS_OS as usize);
    let pc = bar.read_u32(FECS_PC as usize);
    let halted = cpuctl & falcon_offsets::CPUCTL_HALTED != 0;
    let pri_fault = cpuctl & 0xBADF_0000 == 0xBADF_0000;
    let running = boot0_alive(cpuctl) && !halted && !pri_fault;

    debug!(
        bdf = %bdf,
        cpuctl = format!("{cpuctl:#010x}"),
        pc = format!("{pc:#010x}"),
        halted,
        running,
        "ember.fecs.state"
    );
    Ok(serde_json::json!({
        "bdf": bdf,
        "cpuctl": format!("{cpuctl:#010x}"),
        "halted": halted,
        "mailbox0": mailbox0,
        "mailbox1": mailbox1,
        "os": os,
        "running": running,
        "pc": pc,
    }))
}

/// `ember.device.health` — per-device BAR0 health probe.
pub fn ember_device_health(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let bar = open_sysfs_bar0(&bdf)?;

    let boot0 = bar.read_u32(PMC_BOOT_0 as usize);
    let alive = boot0_alive(boot0);

    let ptimer_a = bar.read_u32(PTIMER_TIME_0 as usize);
    std::thread::sleep(std::time::Duration::from_millis(1));
    let ptimer_b = bar.read_u32(PTIMER_TIME_0 as usize);
    let ptimer_ticking = ptimer_a != ptimer_b && ptimer_a != 0xFFFF_FFFF;

    let pmc_enable = bar.read_u32(PMC_ENABLE as usize);
    let engines_enabled = pmc_enable.count_ones();

    debug!(
        bdf = %bdf,
        boot0 = format!("{boot0:#010x}"),
        alive,
        ptimer_ticking,
        engines_enabled,
        "ember.device.health"
    );
    Ok(serde_json::json!({
        "bdf": bdf,
        "alive": alive,
        "ptimer_ticking": ptimer_ticking,
        "engines_enabled": engines_enabled,
        "boot0": boot0,
    }))
}

/// `ember.device.recover` — attempt MMIO recovery via PMC engine re-enable.
pub fn ember_device_recover(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let bar_ro = open_sysfs_bar0(&bdf)?;
    let boot0_before = bar_ro.read_u32(PMC_BOOT_0 as usize);

    if !boot0_alive(boot0_before) {
        debug!(bdf = %bdf, boot0_before = format!("{boot0_before:#010x}"), "ember.device.recover: unrecoverable");
        return Ok(serde_json::json!({
            "bdf": bdf,
            "recovered": false,
            "boot0_before": boot0_before,
            "boot0_after": boot0_before,
        }));
    }

    let bar_rw = open_sysfs_bar0_rw(&bdf)?;
    bar_rw
        .write_u32(PMC_ENABLE as usize, PMC_MASK_FULL)
        .map_err(|e| JsonRpcError::internal_error(format!("PMC_ENABLE write failed: {e}")))?;

    let boot0_after = bar_rw.read_u32(PMC_BOOT_0 as usize);
    let recovered = boot0_alive(boot0_after);

    debug!(
        bdf = %bdf,
        boot0_before = format!("{boot0_before:#010x}"),
        boot0_after = format!("{boot0_after:#010x}"),
        recovered,
        "ember.device.recover"
    );
    Ok(serde_json::json!({
        "bdf": bdf,
        "recovered": recovered,
        "boot0_before": boot0_before,
        "boot0_after": boot0_after,
    }))
}

/// `ember.pramin.read` — bulk read from the PRAMIN VRAM window.
pub fn pramin_read(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let page = parse_u32_param(params, "page", None)?;
    let size = parse_u32_param(params, "size", None)?;
    if size == 0 || size > PRAMIN_READ_MAX {
        return Err(JsonRpcError::invalid_params(format!(
            "Invalid 'size' (must be 1..={PRAMIN_READ_MAX})"
        )));
    }
    if !size.is_multiple_of(4) {
        return Err(JsonRpcError::invalid_params("'size' must be 4-byte aligned"));
    }

    let bar = open_sysfs_bar0_rw(&bdf)?;
    bar.write_u32(PBUS_BAR0_WINDOW as usize, page)
        .map_err(|e| JsonRpcError::internal_error(format!("PRAMIN window select failed: {e}")))?;

    let mut data = Vec::with_capacity(size as usize);
    for i in 0..(size / 4) {
        let offset = PRAMIN_BASE as usize + i as usize * 4;
        let word = bar.read_u32(offset);
        data.extend_from_slice(&word.to_le_bytes());
    }

    debug!(bdf = %bdf, page, bytes = data.len(), "ember.pramin.read");
    Ok(serde_json::json!({
        "bdf": bdf,
        "page": page,
        "data_b64": base64_encode(&data),
        "bytes_read": data.len(),
    }))
}

/// `ember.pramin.write` — stage bytes into VRAM via the PRAMIN window.
pub fn pramin_write(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = parse_bdf(params)?;
    let page = parse_u32_param(params, "page", None)?;
    let data_b64 = params
        .and_then(|p| p.get("data_b64"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'data_b64' (base64 payload)"))?;

    let data = base64_decode(data_b64)?;
    let bar = open_sysfs_bar0_rw(&bdf)?;

    bar.write_u32(PBUS_BAR0_WINDOW as usize, page)
        .map_err(|e| JsonRpcError::internal_error(format!("PRAMIN window select failed: {e}")))?;

    for (i, chunk) in data.chunks(4).enumerate() {
        let word = u32::from_le_bytes([
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
            chunk.get(3).copied().unwrap_or(0),
        ]);
        let offset = PRAMIN_BASE as usize + i * 4;
        bar.write_u32(offset, word)
            .map_err(|e| JsonRpcError::internal_error(format!("PRAMIN write at {offset:#x} failed: {e}")))?;
    }

    debug!(bdf = %bdf, page, bytes = data.len(), "ember.pramin.write");
    Ok(serde_json::json!({
        "bdf": bdf,
        "page": page,
        "bytes_written": data.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bdf_missing() {
        assert!(parse_bdf(None).is_err());
    }

    #[test]
    fn parse_bdf_present() {
        let p = serde_json::json!({"bdf": "0000:01:00.0"});
        assert_eq!(parse_bdf(Some(&p)).unwrap(), "0000:01:00.0");
    }

    #[test]
    fn parse_offset_integer() {
        let p = serde_json::json!({"offset": 512});
        assert_eq!(parse_offset(Some(&p)).unwrap(), 512);
    }

    #[test]
    fn parse_offset_hex_string() {
        let p = serde_json::json!({"offset": "0x200"});
        assert_eq!(parse_offset(Some(&p)).unwrap(), 0x200);
    }

    #[test]
    fn parse_offset_missing() {
        let p = serde_json::json!({});
        assert!(parse_offset(Some(&p)).is_err());
    }

    #[test]
    fn bar0_probe_nonexistent_device() {
        let p = serde_json::json!({"bdf": "ffff:ff:ff.f"});
        let result = mmio_bar0_probe(Some(&p));
        assert!(result.is_err());
    }

    #[test]
    fn falcon_status_unknown_engine() {
        let p = serde_json::json!({"bdf": "0000:01:00.0", "engine": "bogus"});
        let result = mmio_falcon_status(Some(&p));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("bogus"));
    }

    #[test]
    fn read32_nonexistent_device() {
        let p = serde_json::json!({"bdf": "ffff:ff:ff.f", "offset": 0});
        assert!(mmio_read32(Some(&p)).is_err());
    }

    #[test]
    fn write32_nonexistent_device() {
        let p = serde_json::json!({"bdf": "ffff:ff:ff.f", "offset": 0, "value": 42});
        assert!(mmio_write32(Some(&p)).is_err());
    }

    #[test]
    fn batch_nonexistent_device() {
        let p = serde_json::json!({"bdf": "ffff:ff:ff.f", "ops": [{"offset": 0}]});
        assert!(mmio_batch(Some(&p)).is_err());
    }

    #[test]
    fn pramin_nonexistent_device() {
        let p = serde_json::json!({"bdf": "ffff:ff:ff.f", "offset": 0});
        assert!(mmio_pramin_read32(Some(&p)).is_err());
    }

    #[test]
    fn batch_missing_ops() {
        let p = serde_json::json!({"bdf": "0000:01:00.0"});
        let result = mmio_batch(Some(&p));
        assert!(result.is_err());
    }
}
