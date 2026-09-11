//! JavaScript number semantics that the file formats depend on.
//!
//! The `.MAP` and CSV parsers were written against `parseFloat`, which reads a
//! leading numeric prefix and ignores the rest ("123abc" -> 123). Rust's
//! `str::parse` rejects those outright, so a faithful port needs this.

/// `parseFloat(s)` — parses a leading numeric prefix, returning NaN if there
/// isn't one.
pub fn parse_float(s: &str) -> f64 {
    let t = s.trim_start();
    let bytes = t.as_bytes();
    let mut i = 0;

    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    if t[i..].starts_with("Infinity") {
        let v = f64::INFINITY;
        return if t.starts_with('-') { -v } else { v };
    }

    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - int_start;

    let mut frac_digits = 0;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        frac_digits = i - frac_start;
    }

    if int_digits == 0 && frac_digits == 0 {
        return f64::NAN;
    }

    // Exponent, but only if it is well formed — otherwise it isn't part of the
    // number, exactly as parseFloat treats "1e" as 1.
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }

    t[..i].parse::<f64>().unwrap_or(f64::NAN)
}

/// `parseFloat(s) || null` — the truthiness idiom used throughout the JS
/// parsers, which folds both NaN *and* zero to `None`.
pub fn parse_float_truthy(s: &str) -> Option<f64> {
    let v = parse_float(s);
    if v.is_nan() || v == 0.0 {
        None
    } else {
        Some(v)
    }
}

/// `parseFloat(s)` as an `Option`, treating only NaN as absent.
pub fn parse_float_opt(s: &str) -> Option<f64> {
    let v = parse_float(s);
    if v.is_nan() {
        None
    } else {
        Some(v)
    }
}
