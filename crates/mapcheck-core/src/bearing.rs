//! Bearing conversion, formatting, and parsing.
//!
//! The DD.MMSS packed-decimal convention comes straight from the 1992
//! `MAPCHECK.BAS`: 35.0020 means 35 degrees 00 minutes 20 seconds.

use crate::jsnum::parse_float;
use crate::types::{Hemi, Quad};

/// Degrees to radians, using the same constant the BASIC original did
/// (`.01745329252#`) rounded to f64 — kept explicit so results match.
pub const DEG_TO_RAD: f64 = 0.017_453_292_519_943_295;

/// Convert packed DD.MMSS to decimal degrees.
///
/// Port of `BearDec#` in `MAPCHECK.BAS` (line 128), including the `+ .0001`
/// nudge that keeps 35.0060-style floating point noise from losing a minute.
pub fn bear_dec(dms: f64) -> f64 {
    let (sign, dms) = if dms < 0.0 {
        (-1.0, dms.abs())
    } else {
        (1.0, dms)
    };
    let deg = dms.floor();
    let frac = (dms - deg) * 100.0;
    let mm = (frac + 0.0001).floor();
    let ss = (frac - mm) * 100.0;
    sign * (deg + mm / 60.0 + ss / 3600.0)
}

/// Split decimal degrees into whole degrees/minutes/seconds, carrying any
/// rounding spill upward.
fn dms_parts(dec: f64) -> (i64, i64, i64) {
    let d = dec.floor();
    let m = ((dec - d) * 60.0).floor();
    let s = (((dec - d) * 60.0 - m) * 60.0).round();

    let mut d = d as i64;
    let mut m = m as i64;
    let mut s = s as i64;

    if s >= 60 {
        s -= 60;
        m += 1;
    }
    if m >= 60 {
        m -= 60;
        d += 1;
    }
    (d, m, s)
}

/// Format decimal degrees as `DD-MM-SS`.
pub fn fmt_bear(dec: f64) -> String {
    if dec == 0.0 {
        return "00-00-00".to_string();
    }
    let (d, m, s) = dms_parts(dec);
    format!("{}-{:02}-{:02}", d, m, s)
}

/// Format decimal degrees as packed `DD.MMSS`.
pub fn fmt_bear_dms(dec: f64) -> String {
    if dec == 0.0 {
        return "0.0000".to_string();
    }
    let (d, m, s) = dms_parts(dec);
    format!("{}.{:02}{:02}", d, m, s)
}

/// A bearing parsed from user input.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedBearing {
    /// Angle in decimal degrees, 0..=90.
    pub bearing: f64,
    pub quad: Option<Quad>,
    pub hemi: Option<Hemi>,
    /// True when the input carried a `1`-`4` quadrant prefix.
    pub has_prefix: bool,
}

/// Quadrant shorthand: 1=NE, 2=SE, 3=SW, 4=NW.
fn quadrant_code(code: u32) -> Option<(Quad, Hemi)> {
    match code {
        1 => Some((Quad::N, Hemi::E)),
        2 => Some((Quad::S, Hemi::E)),
        3 => Some((Quad::S, Hemi::W)),
        4 => Some((Quad::N, Hemi::W)),
        _ => None,
    }
}

/// Match `^([1-4])(\d{2})(.*)$` — a quadrant digit followed by two degree
/// digits. Returns the code and the remainder with the prefix stripped.
fn split_quadrant_prefix(s: &str) -> Option<(u32, String)> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 3 {
        return None;
    }
    let code = chars[0].to_digit(10)?;
    if !(1..=4).contains(&code) {
        return None;
    }
    if !chars[1].is_ascii_digit() || !chars[2].is_ascii_digit() {
        return None;
    }
    let rest: String = chars[1..].iter().collect();
    Some((code, rest))
}

const SEPARATORS: [char; 6] = ['-', ' ', ':', '°', '\'', '"'];

/// Parse a bearing angle in any of the accepted forms: packed `DD.MMSS`,
/// separated `DD-MM-SS` / `DD MM SS` / `DD°MM'SS"`, each optionally carrying a
/// `1`-`4` quadrant prefix (e.g. `135.0020` = N 35°00'20" E).
///
/// Returns `None` if the value is out of range or unparseable.
pub fn parse_bearing_input(input: &str) -> Option<ParsedBearing> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let (q_code, working) = match split_quadrant_prefix(input) {
        Some((code, rest)) => (Some(code), rest),
        None => (None, input.to_string()),
    };

    let has_separator = working.chars().any(|c| SEPARATORS.contains(&c));

    let bearing = if has_separator {
        let parts: Vec<&str> = working
            .split(|c| SEPARATORS.contains(&c))
            .filter(|p| !p.is_empty())
            .collect();

        let field = |i: usize| -> f64 {
            let v = parts.get(i).map(|p| parse_float(p)).unwrap_or(f64::NAN);
            if v.is_nan() {
                0.0
            } else {
                v
            }
        };
        let deg = field(0);
        let min = field(1);
        let sec = field(2);

        if !(0.0..=90.0).contains(&deg)
            || !(0.0..60.0).contains(&min)
            || !(0.0..60.0).contains(&sec)
        {
            return None;
        }
        deg + min / 60.0 + sec / 3600.0
    } else {
        let val = parse_float(&working);
        if val.is_nan() || !(0.0..=90.0).contains(&val) {
            return None;
        }
        bear_dec(val)
    };

    match q_code.and_then(quadrant_code) {
        Some((quad, hemi)) => Some(ParsedBearing {
            bearing,
            quad: Some(quad),
            hemi: Some(hemi),
            has_prefix: true,
        }),
        None => Some(ParsedBearing {
            bearing,
            quad: None,
            hemi: None,
            has_prefix: false,
        }),
    }
}

/// Parse the CSV bearing forms: explicit (`N 35.0020 E`) or shorthand
/// (`135.0020`). Both must resolve to a full quadrant bearing.
pub fn parse_csv_bearing(bearing_str: &str) -> Option<(Quad, f64, Hemi)> {
    let s = bearing_str.trim().to_ascii_uppercase();

    // Explicit form: leading N/S, trailing E/W, angle in between.
    if s.len() >= 2 {
        let first = s.chars().next().unwrap();
        let last = s.chars().last().unwrap();
        if matches!(first, 'N' | 'S') && matches!(last, 'E' | 'W') {
            let inner = s[first.len_utf8()..s.len() - last.len_utf8()].trim();
            if let Some(parsed) = parse_bearing_input(inner) {
                let quad = if first == 'N' { Quad::N } else { Quad::S };
                let hemi = if last == 'E' { Hemi::E } else { Hemi::W };
                return Some((quad, parsed.bearing, hemi));
            }
        }
    }

    let parsed = parse_bearing_input(&s)?;
    if parsed.has_prefix {
        return Some((parsed.quad?, parsed.bearing, parsed.hemi?));
    }
    None
}

/// Quadrant bearing to a 0..360 azimuth clockwise from north.
pub fn course_azimuth(quad: Quad, bearing: f64, hemi: Hemi) -> f64 {
    match (quad, hemi) {
        (Quad::N, Hemi::E) => bearing,
        (Quad::S, Hemi::E) => 180.0 - bearing,
        (Quad::S, Hemi::W) => 180.0 + bearing,
        (Quad::N, Hemi::W) => 360.0 - bearing,
    }
}

/// A 0..360 azimuth back to a quadrant bearing.
pub fn azimuth_to_quad_bearing_hemi(az: f64) -> (Quad, f64, Hemi) {
    let az = ((az % 360.0) + 360.0) % 360.0;
    if az < 90.0 {
        (Quad::N, az, Hemi::E)
    } else if az < 180.0 {
        (Quad::S, 180.0 - az, Hemi::E)
    } else if az < 270.0 {
        (Quad::S, az - 180.0, Hemi::W)
    } else {
        (Quad::N, 360.0 - az, Hemi::W)
    }
}
