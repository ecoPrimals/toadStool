// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from unix.rs (S334).

use super::ribocipher;
use super::unix::*;

#[test]
fn is_plaintext_protocol_byte_accepts_json_and_http() {
    assert!(is_plaintext_protocol_byte(b'{'));
    assert!(is_plaintext_protocol_byte(b'P'));
    assert!(is_plaintext_protocol_byte(b'G'));
    assert!(is_plaintext_protocol_byte(b'H'));
}

#[test]
fn is_plaintext_protocol_byte_rejects_btsp_binary_prefix() {
    assert!(!is_plaintext_protocol_byte(0x00));
    assert!(!is_plaintext_protocol_byte(0x01));
    assert!(!is_plaintext_protocol_byte(0x08));
}

#[test]
fn is_plaintext_protocol_byte_boundary_at_tab() {
    assert!(is_plaintext_protocol_byte(0x09));
    assert!(!is_plaintext_protocol_byte(0x08));
}

#[test]
fn is_ribocipher_signal_byte_detects_all_prefixes() {
    assert!(is_ribocipher_signal_byte(ribocipher::CLEAR));
    assert!(is_ribocipher_signal_byte(ribocipher::MITO));
    assert!(is_ribocipher_signal_byte(ribocipher::NUCLEAR));
    assert!(!is_ribocipher_signal_byte(b'{'));
    assert!(!is_ribocipher_signal_byte(0x00));
}

#[test]
fn ribocipher_signal_detection_distinguishes_prefixes_from_json() {
    assert!(is_ribocipher_signal_byte(ribocipher::CLEAR));
    assert!(is_ribocipher_signal_byte(ribocipher::MITO));
    assert!(!is_ribocipher_signal_byte(b'{'));
    // riboCipher prefixes are high bytes; plaintext check alone cannot distinguish them.
    assert!(is_plaintext_protocol_byte(ribocipher::CLEAR));
}

#[test]
fn ndjson_line_prefix_clears_buffer_for_clear_and_mito_signals() {
    assert_eq!(ndjson_line_prefix_after_first_byte(ribocipher::CLEAR), "");
    assert_eq!(ndjson_line_prefix_after_first_byte(ribocipher::MITO), "");
}

#[test]
fn ndjson_line_prefix_preserves_non_signalled_first_byte() {
    assert_eq!(ndjson_line_prefix_after_first_byte(b'{'), "{");
    assert_eq!(ndjson_line_prefix_after_first_byte(b'P'), "P");
}

#[test]
fn format_http_response_header_keep_alive_and_close() {
    let body = br#"{"jsonrpc":"2.0","result":{},"id":1}"#;
    let keep_alive = format_http_response_header(body.len(), false);
    assert!(keep_alive.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(keep_alive.contains("Content-Type: application/json\r\n"));
    assert!(keep_alive.contains(&format!("Content-Length: {}\r\n", body.len())));
    assert!(keep_alive.contains("Connection: keep-alive\r\n"));
    assert!(keep_alive.ends_with("\r\n\r\n"));

    let close = format_http_response_header(body.len(), true);
    assert!(close.contains("Connection: close\r\n"));
    assert!(!close.contains("Connection: keep-alive"));
}

#[test]
fn parse_http_header_field_normalizes_case_and_whitespace() {
    assert_eq!(
        parse_http_header_field("Content-Length: 42\r\n"),
        Some(("content-length".into(), "42".into()))
    );
    assert_eq!(
        parse_http_header_field("Connection:  close"),
        Some(("connection".into(), "close".into()))
    );
    assert_eq!(parse_http_header_field("not-a-header"), None);
}

#[test]
fn early_health_response_maps_known_methods() {
    let liveness = early_health_response(Some("health.liveness"), serde_json::json!(7));
    assert_eq!(liveness["result"]["status"], "alive");
    assert_eq!(liveness["id"], 7);

    let readiness = early_health_response(Some("health.readiness"), serde_json::Value::Null);
    assert_eq!(readiness["result"]["status"], "starting");

    let check = early_health_response(Some("health.check"), serde_json::json!("req-1"));
    assert_eq!(check["result"]["status"], "starting");
    assert_eq!(check["result"]["uptime_secs"], 0);
    assert_eq!(check["id"], "req-1");
}

#[test]
fn early_health_response_unknown_method_returns_initializing_error() {
    let response = early_health_response(Some("compute.submit"), serde_json::json!(99));
    assert_eq!(response["error"]["code"], -32002);
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("initializing")
    );
    assert_eq!(response["id"], 99);
}

#[test]
fn unsignalled_connection_reject_json_has_ribocipher_guidance() {
    let response = unsignalled_connection_reject_json();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("riboCipher")
    );
    assert!(response["id"].is_null());
}
