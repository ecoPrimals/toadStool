// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;

#[test]
fn test_violation_severity_low() {
    let severity = ViolationSeverity::Low;
    assert!(matches!(severity, ViolationSeverity::Low));
}

#[test]
fn test_violation_severity_medium() {
    let severity = ViolationSeverity::Medium;
    assert!(matches!(severity, ViolationSeverity::Medium));
}

#[test]
fn test_violation_severity_high() {
    let severity = ViolationSeverity::High;
    assert!(matches!(severity, ViolationSeverity::High));
}

#[test]
fn test_violation_severity_critical() {
    let severity = ViolationSeverity::Critical;
    assert!(matches!(severity, ViolationSeverity::Critical));
}
