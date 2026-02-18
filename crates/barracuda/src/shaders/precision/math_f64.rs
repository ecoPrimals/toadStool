//! math_f64 WGSL injection: dependency graph, extraction, and injection logic

/// Function dependency map for math_f64.wgsl
pub const F64_FUNCTION_DEPS: &[(&str, &[&str])] = &[
    ("abs_f64", &[]),
    ("sign_f64", &[]),
    ("floor_f64", &[]),
    ("ceil_f64", &[]),
    ("round_f64", &["floor_f64"]),
    ("fract_f64", &["floor_f64"]),
    ("min_f64", &[]),
    ("max_f64", &[]),
    ("clamp_f64", &["min_f64", "max_f64"]),
    ("sqrt_f64", &[]),
    ("cbrt_f64", &["abs_f64"]),
    ("ipow_f64", &[]),
    ("pow_one_third", &["cbrt_f64"]),
    ("pow_one_half", &["sqrt_f64"]),
    ("pow_two_thirds", &["cbrt_f64"]),
    ("exp_f64", &["abs_f64", "round_f64"]),
    ("log_f64", &[]),
    (
        "pow_f64",
        &[
            "round_f64",
            "abs_f64",
            "sqrt_f64",
            "cbrt_f64",
            "pow_two_thirds",
            "exp_f64",
            "log_f64",
            "ipow_f64",
        ],
    ),
    ("sin_f64", &[]),
    ("cos_f64", &["sin_f64"]),
    ("tan_f64", &["sin_f64", "cos_f64"]),
    ("sinh_f64", &["exp_f64"]),
    ("cosh_f64", &["exp_f64"]),
    ("tanh_f64", &["exp_f64"]),
    ("gamma_f64", &["sin_f64", "abs_f64", "pow_f64", "exp_f64"]),
    ("erf_f64", &["sign_f64", "abs_f64", "exp_f64"]),
    (
        "bessel_j0_f64",
        &["abs_f64", "sqrt_f64", "cos_f64", "sin_f64"],
    ),
];

/// Ordered list of functions for correct emission order
pub const F64_FUNCTION_ORDER: &[&str] = &[
    "abs_f64",
    "sign_f64",
    "floor_f64",
    "ceil_f64",
    "min_f64",
    "max_f64",
    "round_f64",
    "fract_f64",
    "clamp_f64",
    "sqrt_f64",
    "ipow_f64",
    "cbrt_f64",
    "pow_one_third",
    "pow_one_half",
    "pow_two_thirds",
    "exp_f64",
    "log_f64",
    "pow_f64",
    "sin_f64",
    "cos_f64",
    "tan_f64",
    "sinh_f64",
    "cosh_f64",
    "tanh_f64",
    "gamma_f64",
    "erf_f64",
    "bessel_j0_f64",
];

/// Extract a WGSL function from source by name
pub fn extract_wgsl_function(source: &str, name: &str) -> Option<String> {
    let fn_pattern = format!("fn {name}(");
    let fn_pattern_space = format!("fn {name} (");

    let start_idx = source
        .find(&fn_pattern)
        .or_else(|| source.find(&fn_pattern_space))?;

    let brace_idx = source[start_idx..].find('{')?;
    let fn_start = start_idx;

    let mut brace_count = 0;
    let mut fn_end = fn_start + brace_idx;

    for (i, c) in source[fn_start + brace_idx..].char_indices() {
        match c {
            '{' => brace_count += 1,
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    fn_end = fn_start + brace_idx + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    let mut doc_start = fn_start;
    let before = &source[..fn_start];
    if let Some(last_newline) = before.rfind('\n') {
        let prev_lines: Vec<&str> = before[..last_newline + 1].lines().rev().take(5).collect();
        for (i, line) in prev_lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("///") || trimmed.starts_with("//") || trimmed.is_empty() {
                if i == 0 {
                    if let Some(pos) = before.rfind(line) {
                        doc_start = pos;
                    }
                }
            } else {
                break;
            }
        }
    }

    Some(source[doc_start..fn_end].trim().to_string())
}
