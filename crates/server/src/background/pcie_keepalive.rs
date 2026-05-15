// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCIe switch keepalive for PLX PEX 8747 and similar bridge chips.
//!
//! Generates periodic config-space reads on upstream bridges and downstream
//! ports to prevent BIOS/ACPI idle power-gating from transitioning the
//! switch into D3cold. Without this, the PLX can go dark after ~2h of idle
//! (sometimes as short as ~11 min), taking all downstream GPUs offline.
//!
//! ## How it works
//!
//! Every `INTERVAL` seconds, reads PCI config-space offset 0x04 (COMMAND
//! register) on each monitored bridge BDF. This generates a Configuration
//! Read (CfgRd) TLP that traverses the root complex → upstream bridge →
//! PLX fabric, keeping the LTSSM in L0.
//!
//! ## Discovery
//!
//! On startup, scans `/sys/bus/pci/devices` for bridge-class devices
//! (class 0x060400 = PCI-to-PCI bridge) with PLX vendor ID (0x10b5).
//! Falls back to well-known PLX BDF addresses from the Tesla K80 topology.

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use tracing::{debug, info, warn};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(3);

const PLX_VENDOR_ID: u16 = 0x10b5;

fn read_config_u16(bdf: &str, offset: u64) -> Option<u16> {
    let path = format!("/sys/bus/pci/devices/{bdf}/config");
    let mut f = std::fs::File::open(&path).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = [0u8; 2];
    f.read_exact(&mut buf).ok()?;
    Some(u16::from_le_bytes(buf))
}

fn read_config_u32(bdf: &str, offset: u64) -> Option<u32> {
    let path = format!("/sys/bus/pci/devices/{bdf}/config");
    let mut f = std::fs::File::open(&path).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = [0u8; 4];
    f.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

fn discover_plx_bridges() -> Vec<String> {
    let mut bridges = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return bridges;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();
        let Some(vendor) = read_config_u16(&bdf, 0x00) else {
            continue;
        };
        if vendor != PLX_VENDOR_ID {
            continue;
        }
        let Some(class) = read_config_u32(&bdf, 0x08) else {
            continue;
        };
        let class_code = (class >> 8) & 0xFF_FFFF;
        if class_code == 0x0604 {
            bridges.push(bdf);
        }
    }

    bridges.sort();
    bridges
}

fn discover_downstream_gpus(bridges: &[String]) -> Vec<String> {
    let mut gpus = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return gpus;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();
        let Some(class) = read_config_u32(&bdf, 0x08) else {
            continue;
        };
        let class_code = (class >> 8) & 0xFF_FFFF;
        // VGA compatible (0x0300) or 3D controller (0x0302)
        if class_code != 0x0300 && class_code != 0x0302 {
            continue;
        }

        let link = format!("/sys/bus/pci/devices/{bdf}");
        let Ok(canonical) = std::fs::canonicalize(&link) else {
            continue;
        };
        let path_str = canonical.to_string_lossy();
        for bridge_bdf in bridges {
            if path_str.contains(bridge_bdf) {
                gpus.push(bdf.clone());
                break;
            }
        }
    }

    gpus
}

pub(crate) async fn run() {
    let bridges = discover_plx_bridges();
    if bridges.is_empty() {
        info!("No PLX PCIe bridges found — keepalive disabled");
        return;
    }

    let downstream = discover_downstream_gpus(&bridges);

    info!(
        bridge_count = bridges.len(),
        gpu_count = downstream.len(),
        bridges = ?bridges,
        "PLX PCIe keepalive started"
    );

    let all_targets: Vec<String> = bridges
        .iter()
        .chain(downstream.iter())
        .cloned()
        .collect();

    let mut consecutive_failures = 0u32;

    loop {
        let mut alive = 0usize;
        let mut dead = 0usize;

        for bdf in &all_targets {
            match read_config_u16(bdf, 0x04) {
                Some(cmd) if cmd != 0xFFFF => {
                    alive += 1;
                }
                _ => {
                    dead += 1;
                    debug!(bdf, "keepalive: config read failed or returned 0xFFFF");
                }
            }
        }

        if dead > 0 {
            consecutive_failures += 1;
            if consecutive_failures % 20 == 1 {
                warn!(
                    alive,
                    dead,
                    consecutive_failures,
                    "PCIe keepalive: some devices unreachable (D3cold?)"
                );
            }
        } else {
            if consecutive_failures > 0 {
                info!(alive, "PCIe keepalive: all devices recovered");
            }
            consecutive_failures = 0;
        }

        tokio::time::sleep(KEEPALIVE_INTERVAL).await;
    }
}
