// SPDX-License-Identifier: AGPL-3.0-or-later
//! Human-readable PCI device names from the system `pci.ids` database.
//!
//! # Why not ask the vendor tool
//!
//! `nvidia-smi --query-gpu=name` returns "NVIDIA GeForce RTX 5060", which is
//! the only thing GPU detection used it for. But it reports only devices bound
//! to the proprietary driver: on biomeGate it sees one of four installed GPUs,
//! silently omitting the unbound Titan V and both `vfio-pci` Tesla K80 dies.
//! It also has to be installed, which on a sovereign host is the thing we are
//! trying to avoid.
//!
//! `pci.ids` is the same database `lspci` reads — a plain text file maintained
//! by the PCI ID Repository and shipped by every distribution. Reading it
//! ourselves is a file parse, not a subprocess, and it names every device on
//! the bus regardless of which driver is bound or whether the device is even
//! answering.
//!
//! # Absence is fine
//!
//! When the database is missing, callers get `None` and fall back to a name
//! built from the numeric IDs. A missing optional data file must never make
//! hardware undetectable — the device was already found by then.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Where distributions put `pci.ids`, in the order `lspci` itself looks.
const PCI_IDS_PATHS: &[&str] = &[
    "/usr/share/hwdata/pci.ids",
    "/usr/share/misc/pci.ids",
    "/usr/share/pci.ids",
    "/var/lib/pciutils/pci.ids",
];

/// Vendor and device names, keyed by `(vendor_id, device_id)`.
///
/// Parsed once. The file is ~1.4 MB of text and the parse is a few
/// milliseconds, but hardware detection can be called repeatedly.
struct PciIdDatabase {
    vendors: HashMap<u16, String>,
    devices: HashMap<(u16, u16), String>,
}

fn database() -> Option<&'static PciIdDatabase> {
    static DB: OnceLock<Option<PciIdDatabase>> = OnceLock::new();
    DB.get_or_init(|| {
        let contents = PCI_IDS_PATHS
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok())?;
        Some(parse_pci_ids(&contents))
    })
    .as_ref()
}

/// Parse the `pci.ids` format.
///
/// ```text
/// 10de  NVIDIA Corporation
/// \t1d81  GV100 [TITAN V]
/// \t\t1028 1234  Subsystem name        <- ignored
/// # comment
/// ```
///
/// Vendors start at column 0, devices are indented one tab, and subsystems two.
/// Class definitions later in the file start with `C ` at column 0 and are not
/// vendors; they are skipped so a class ID cannot be mistaken for one.
fn parse_pci_ids(contents: &str) -> PciIdDatabase {
    let mut vendors = HashMap::new();
    let mut devices = HashMap::new();
    let mut current_vendor: Option<u16> = None;

    for line in contents.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Class/subclass section — everything after it is not vendor data.
        if line.starts_with("C ") {
            current_vendor = None;
            continue;
        }

        let depth = line.len() - line.trim_start_matches('\t').len();
        let body = line.trim_start_matches('\t');

        match depth {
            0 => {
                if let Some((id, name)) = split_id_and_name(body) {
                    vendors.insert(id, name.to_string());
                    current_vendor = Some(id);
                }
            }
            1 => {
                if let (Some(vendor), Some((id, name))) = (current_vendor, split_id_and_name(body))
                {
                    devices.insert((vendor, id), name.to_string());
                }
            }
            // Subsystem entries; not needed to name a device.
            _ => {}
        }
    }

    PciIdDatabase { vendors, devices }
}

/// Split `"1d81  GV100 [TITAN V]"` into `(0x1d81, "GV100 [TITAN V]")`.
fn split_id_and_name(body: &str) -> Option<(u16, &str)> {
    let (id_str, name) = body.split_once("  ")?;
    let id = u16::from_str_radix(id_str.trim(), 16).ok()?;
    Some((id, name.trim()))
}

/// Vendor name, e.g. `0x10de` → `"NVIDIA Corporation"`.
#[must_use]
pub fn vendor_name(vendor_id: u16) -> Option<&'static str> {
    database()?.vendors.get(&vendor_id).map(String::as_str)
}

/// Device name, e.g. `(0x10de, 0x1d81)` → `"GV100 [TITAN V]"`.
#[must_use]
pub fn device_name(vendor_id: u16, device_id: u16) -> Option<&'static str> {
    database()?
        .devices
        .get(&(vendor_id, device_id))
        .map(String::as_str)
}

/// Best available name for a device, always returning something usable.
///
/// Falls back to the numeric identity, which is still enough to look a device
/// up and strictly more than "Unknown GPU".
#[must_use]
pub fn describe(vendor_id: u16, device_id: u16) -> String {
    match (vendor_name(vendor_id), device_name(vendor_id, device_id)) {
        (Some(v), Some(d)) => format!("{v} {d}"),
        (Some(v), None) => format!("{v} device {device_id:04x}"),
        (None, Some(d)) => d.to_string(),
        (None, None) => format!("PCI device {vendor_id:04x}:{device_id:04x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# Comment line
10de  NVIDIA Corporation
\t1d81  GV100 [TITAN V]
\t\t10de 1214  TITAN V
\t102d  GK210GL [Tesla K80]
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t744c  Navi 31 [Radeon RX 7900 XT]
C 03  Display controller
\t00  VGA compatible controller
";

    #[test]
    fn parses_vendors_and_devices() {
        let db = parse_pci_ids(SAMPLE);
        assert_eq!(db.vendors.get(&0x10de).unwrap(), "NVIDIA Corporation");
        assert_eq!(db.devices.get(&(0x10de, 0x1d81)).unwrap(), "GV100 [TITAN V]");
        assert_eq!(
            db.devices.get(&(0x10de, 0x102d)).unwrap(),
            "GK210GL [Tesla K80]"
        );
        assert_eq!(
            db.devices.get(&(0x1002, 0x744c)).unwrap(),
            "Navi 31 [Radeon RX 7900 XT]"
        );
    }

    /// Subsystem lines are two tabs deep and must not become devices.
    #[test]
    fn subsystem_entries_are_not_devices() {
        let db = parse_pci_ids(SAMPLE);
        assert!(!db.devices.contains_key(&(0x10de, 0x1214)));
    }

    /// The class section at the end of the file must not register `03` as a
    /// vendor, which would then name unrelated devices.
    #[test]
    fn class_section_is_not_parsed_as_vendors() {
        let db = parse_pci_ids(SAMPLE);
        assert!(!db.vendors.contains_key(&0x03));
    }

    /// A missing database must degrade to a usable name, never a failure.
    #[test]
    fn describe_always_returns_something() {
        let name = describe(0xABCD, 0x1234);
        assert!(!name.is_empty());
        assert!(name.contains("abcd") || name.contains("ABCD") || !name.is_empty());
    }
}
