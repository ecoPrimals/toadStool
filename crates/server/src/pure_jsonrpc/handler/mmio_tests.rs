// SPDX-License-Identifier: AGPL-3.0-or-later
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
