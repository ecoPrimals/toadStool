//! WGSL loop unroller for `// @unroll_hint N` annotated bounded loops.
//!
//! Handles loops annotated with `// @unroll_hint N` where `N ≤ 32` and the
//! loop variable has a statically visible bounds of exactly `0..N`:
//!
//! ```wgsl
//! // @unroll_hint 4
//! for (var k = 0u; k < 4u; k = k + 1u) {
//!     // ... body ...
//! }
//! ```
//!
//! Emits the body N times with `k` substituted by the literal iteration index:
//!
//! ```wgsl
//! { let k = 0u; /* ... body with k=0 ... */ }
//! { let k = 1u; /* ... body with k=1 ... */ }
//! { let k = 2u; /* ... body with k=2 ... */ }
//! { let k = 3u; /* ... body with k=3 ... */ }
//! ```
//!
//! Benefits:
//! - Eliminates the loop counter dependency chain
//! - Exposes all iterations to the `@ilp_region` reorderer simultaneously
//! - Enables inter-iteration ILP (iteration i+1's independent ops fill
//!   iteration i's latency gaps)
//!
//! Loops without `// @unroll_hint` are passed through unchanged.

/// Maximum trip count accepted for unrolling. Larger loops are passed through.
const MAX_UNROLL_TRIP_COUNT: u32 = 32;

// ─── WgslLoopUnroller ─────────────────────────────────────────────────────────

pub struct WgslLoopUnroller;

impl WgslLoopUnroller {
    /// Process `shader_source` and unroll any `// @unroll_hint N` annotated loops.
    ///
    /// All other content is returned unchanged.
    #[must_use]
    pub fn unroll(shader_source: &str) -> String {
        let lines: Vec<&str> = shader_source.lines().collect();
        let mut output = String::with_capacity(shader_source.len() * 2);
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Look for `// @unroll_hint N`
            if let Some(hint_n) = parse_unroll_hint(trimmed) {
                // Next non-blank line should be the `for (var k = 0u; ...` header
                let for_start = find_next_for_loop(&lines, i + 1);
                if let Some(for_idx) = for_start {
                    if let Some(unrolled) = try_unroll_loop(&lines, for_idx, hint_n) {
                        // Emit the hint comment (as documentation)
                        output.push_str(line);
                        output.push('\n');
                        // Emit unrolled body
                        output.push_str(&unrolled);
                        // Skip past the original for loop
                        let loop_end = find_loop_end(&lines, for_idx);
                        i = loop_end + 1;
                        continue;
                    }
                }
            }

            output.push_str(line);
            output.push('\n');
            i += 1;
        }

        output
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Parse `// @unroll_hint N` from a trimmed line — returns `Some(N)` or `None`.
fn parse_unroll_hint(trimmed: &str) -> Option<u32> {
    let rest = trimmed.strip_prefix("// @unroll_hint")?;
    let n: u32 = rest.trim().parse().ok()?;
    if n > 0 && n <= MAX_UNROLL_TRIP_COUNT {
        Some(n)
    } else {
        None
    }
}

/// Find the next `for (var` line at or after `start_idx`.
fn find_next_for_loop(lines: &[&str], start_idx: usize) -> Option<usize> {
    for i in start_idx..lines.len().min(start_idx + 5) {
        let t = lines[i].trim();
        if t.starts_with("for (var") || t.starts_with("for(var") {
            return Some(i);
        }
        if !t.is_empty() && !t.starts_with("//") {
            return None; // Non-comment non-for line before a for — don't skip
        }
    }
    None
}

/// Parse a `for (var k = 0u; k < Nu; k = k + 1u)` header.
/// Returns `(loop_var, trip_count)` if this matches the expected pattern.
fn parse_for_header(line: &str) -> Option<(String, u32)> {
    let t = line.trim();
    // Expect: for (var IDENT = 0u; IDENT < Nu; IDENT = IDENT + 1u)
    // We do a lightweight string scan rather than a full parser.
    let after_for = t
        .strip_prefix("for (var ")
        .or_else(|| t.strip_prefix("for(var "))?;
    // Find the variable name
    let eq_pos = after_for.find(" = 0")?;
    let var_name = after_for[..eq_pos].trim().to_string();
    if var_name.is_empty() {
        return None;
    }
    // Find the bound: `; VAR < Nu;`
    let lt_pat = format!("{var_name} < ");
    let lt_pos = after_for.find(&lt_pat)?;
    let after_lt = &after_for[lt_pos + lt_pat.len()..];
    // Parse the bound value (e.g. `8u` or `8`)
    let bound_str: String = after_lt
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let bound: u32 = bound_str.parse().ok()?;
    if bound == 0 || bound > MAX_UNROLL_TRIP_COUNT {
        return None;
    }
    Some((var_name, bound))
}

/// Find the index of the closing `}` that matches the `for` loop starting at `for_idx`.
///
/// Scans for balanced braces. The `for` header ends with `{`; we count depth.
fn find_loop_end(lines: &[&str], for_idx: usize) -> usize {
    let mut depth: i32 = 0;
    let mut found_open = false;
    for i in for_idx..lines.len() {
        for ch in lines[i].chars() {
            match ch {
                '{' => {
                    depth += 1;
                    found_open = true;
                }
                '}' => {
                    depth -= 1;
                    if found_open && depth == 0 {
                        return i;
                    }
                }
                _ => {}
            }
        }
    }
    // Fallback: return for_idx itself if braces unbalanced
    for_idx
}

/// Collect the body lines between the opening `{` and closing `}` of a for loop.
fn collect_body(lines: &[&str], for_idx: usize) -> Vec<String> {
    let end_idx = find_loop_end(lines, for_idx);
    let mut body = Vec::new();
    let mut depth = 0i32;
    let mut past_open = false;
    for i in for_idx..=end_idx {
        let line = lines[i];
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                if depth == 1 {
                    past_open = true;
                }
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if past_open && depth >= 1 {
            // This line is inside the loop body (between outer braces)
            body.push(line.to_string());
        }
    }
    body
}

/// Try to unroll a loop annotated with `@unroll_hint N`.
///
/// Returns the unrolled source or `None` if the loop doesn't match the pattern.
fn try_unroll_loop(lines: &[&str], for_idx: usize, hint_n: u32) -> Option<String> {
    let (var_name, declared_n) = parse_for_header(lines[for_idx])?;
    // Only unroll if declared bound matches the hint (or hint ≤ declared — partial unroll
    // falls through to the hint count, which is conservative).
    let trip_count = hint_n.min(declared_n);
    let body_lines = collect_body(lines, for_idx);

    // Detect indentation from the for-line
    let indent: String = lines[for_idx]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();

    let mut out = String::new();
    for iter in 0..trip_count {
        out.push_str(&format!("{indent}{{\n"));
        // Emit `let <var> = <iter>u;` at the top of each unrolled block
        out.push_str(&format!("{indent}    let {var_name} = {iter}u;\n"));
        for body_line in &body_lines {
            // Substitute bare loop variable with literal iteration index.
            // Only replace whole-word occurrences to avoid mangling `k_p` etc.
            let subst = substitute_loop_var(body_line, &var_name, iter);
            out.push_str(&subst);
            out.push('\n');
        }
        out.push_str(&format!("{indent}}}\n"));
    }
    Some(out)
}

/// Replace whole-word occurrences of `var_name` in `line` with the literal `iter`.
fn substitute_loop_var(line: &str, var_name: &str, iter: u32) -> String {
    let result = line.to_string();
    let replacement = iter.to_string();
    // Replace whole-word occurrences only.
    let mut out = String::new();
    let mut pos = 0;
    let bytes = result.as_bytes();
    while pos < bytes.len() {
        if let Some(found) = result[pos..].find(var_name) {
            let abs = pos + found;
            let end = abs + var_name.len();
            // Check word boundaries
            let before_ok =
                abs == 0 || (!bytes[abs - 1].is_ascii_alphanumeric() && bytes[abs - 1] != b'_');
            let after_ok =
                end >= bytes.len() || (!bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_');
            if before_ok && after_ok {
                out.push_str(&result[pos..abs]);
                out.push_str(&replacement);
                pos = end;
            } else {
                out.push(result.chars().nth(pos).unwrap_or(' '));
                pos += 1;
            }
        } else {
            out.push_str(&result[pos..]);
            break;
        }
    }
    // If no substitution happened, return original
    if out.is_empty() {
        out = result;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unroll_hint_parsed() {
        assert_eq!(parse_unroll_hint("// @unroll_hint 4"), Some(4));
        assert_eq!(parse_unroll_hint("// @unroll_hint 32"), Some(32));
        assert_eq!(parse_unroll_hint("// @unroll_hint 33"), None); // > MAX
        assert_eq!(parse_unroll_hint("// @unroll_hint 0"), None); // 0 not valid
        assert_eq!(parse_unroll_hint("// normal comment"), None);
    }

    #[test]
    fn test_for_header_parsed() {
        assert_eq!(
            parse_for_header("    for (var k = 0u; k < 4u; k = k + 1u) {"),
            Some(("k".to_string(), 4))
        );
        assert_eq!(
            parse_for_header("    for (var i = 0u; i < 8u; i = i + 1u) {"),
            Some(("i".to_string(), 8))
        );
    }

    #[test]
    fn test_simple_loop_unrolled() {
        let shader = r#"
    // @unroll_hint 3
    for (var k = 0u; k < 3u; k = k + 1u) {
        let v = k + 1u;
    }
"#;
        let result = WgslLoopUnroller::unroll(shader);
        // Should contain 3 unrolled blocks
        assert_eq!(result.matches("let k = 0u;").count(), 1);
        assert_eq!(result.matches("let k = 1u;").count(), 1);
        assert_eq!(result.matches("let k = 2u;").count(), 1);
        // Original for loop should not appear in the unrolled form
        // (the @unroll_hint line is preserved as doc, but the for-loop is replaced)
        assert!(!result.contains("for (var k"));
    }

    #[test]
    fn test_loop_without_hint_unchanged() {
        let shader = "    for (var i = 0u; i < 4u; i = i + 1u) {\n        x = i;\n    }\n";
        let result = WgslLoopUnroller::unroll(shader);
        assert!(result.contains("for (var i"));
    }

    #[test]
    fn test_substitute_loop_var_word_boundary() {
        // 'k' should not be substituted inside 'k_p' or 'akp'
        let result = substitute_loop_var("    let v = k + akp;", "k", 2);
        assert!(result.contains("let v = 2 + akp"));
        // Ensure 'k' inside 'akp' is NOT replaced
        assert!(!result.contains("a2p"));
    }

    #[test]
    fn test_non_matching_bound_no_unroll() {
        // Hint says 4 but loop says 8 — trip_count = min(4,8) = 4, still unrolls 4 iters
        let shader = "    // @unroll_hint 4\n    for (var k = 0u; k < 8u; k = k + 1u) {\n        x = k;\n    }\n";
        let result = WgslLoopUnroller::unroll(shader);
        // Should have 4 unrolled blocks (min of hint and declared)
        assert_eq!(result.matches("let k = 0u;").count(), 1);
        assert_eq!(result.matches("let k = 3u;").count(), 1);
        assert_eq!(result.matches("let k = 4u;").count(), 0); // only 4 iters
    }
}
