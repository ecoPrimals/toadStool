// SPDX-License-Identifier: AGPL-3.0-or-later
//! MMIO and Falcon JSON-RPC handlers.
//!
//! Provides BAR0 register access and falcon microcontroller status via
//! `mmio.read32`, `mmio.write32`, `mmio.batch`, `mmio.pramin.read32`,
//! `mmio.bar0.probe`, and `mmio.falcon.status`.
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
}

const PMC_BOOT_0: u32 = 0x0000_0000;
const PMC_ENABLE: u32 = 0x0000_0200;
const PRAMIN_BASE: u32 = 0x0070_0000;
const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

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
