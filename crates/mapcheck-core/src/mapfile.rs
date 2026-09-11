//! The legacy `.MAP` file format written by the 1992 BASIC program.
//!
//! Layout is one value per line: plat name, course count, then four lines per
//! course (quadrant, bearing as DD.MMSS, hemisphere, distance). Curves are
//! smuggled into the quadrant line as pipe-delimited fields, and the distance
//! line carries the chord so the original program still plots them sensibly.

use crate::bearing::fmt_bear_dms;
use crate::jsnum::{parse_float, parse_float_truthy};
use crate::types::{
    Course, CurveCourse, Hemi, InvalidCourse, LineCourse, PartialData, PartialType, Quad, Turn,
};

/// A parse failure severe enough to reject the whole file. Per-course problems
/// become [`Course::Invalid`] entries instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// The result of reading a plat file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedFile {
    pub name: String,
    pub lot: String,
    pub courses: Vec<Course>,
    /// How many courses failed and were kept as [`Course::Invalid`].
    pub error_count: usize,
}

const CURVE_MARKER: &str = " | CURVE | ";
const FIELD_SEP: &str = " | ";

/// Trim every line and drop the blank ones, as all three JS parsers do.
pub(crate) fn significant_lines(content: &str) -> Vec<&str> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Auto-detect the format and parse. A second line of nothing but digits means
/// a legacy `.MAP` course count; anything else is treated as CSV/text.
pub fn detect_and_parse(content: &str, file_name: &str) -> Result<ParsedFile, ParseError> {
    let lines = significant_lines(content);
    if lines.is_empty() {
        return Err(ParseError("Empty file".to_string()));
    }

    let looks_like_retro =
        lines.len() >= 2 && !lines[1].is_empty() && lines[1].chars().all(|c| c.is_ascii_digit());

    if looks_like_retro {
        parse_map_file(content)
    } else {
        Ok(crate::csvfile::parse_csv_text(content, file_name))
    }
}

/// `parseInt(s, 10)` — leading integer prefix, NaN if there isn't one.
fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let mut end = 0;
    let bytes = t.as_bytes();
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == digits_start {
        return None;
    }
    t[..end].parse::<i64>().ok()
}

/// Parse a legacy `.MAP` file.
pub fn parse_map_file(content: &str) -> Result<ParsedFile, ParseError> {
    let lines = significant_lines(content);
    if lines.len() < 2 {
        return Err(ParseError("Invalid file format: too few lines".to_string()));
    }

    let (name, lot) = match lines[0].split_once(FIELD_SEP) {
        Some((n, l)) => (n.to_string(), l.to_string()),
        None => (lines[0].to_string(), String::new()),
    };

    let num_courses = parse_int(lines[1]).ok_or_else(|| {
        ParseError("Invalid file format: line 2 should contain course count".to_string())
    })?;

    let mut courses = Vec::new();
    let mut error_count = 0;
    let mut idx = 2usize;

    for i in 0..num_courses.max(0) {
        if idx + 3 >= lines.len() {
            return Err(ParseError(format!(
                "Invalid file format: missing details for course #{}",
                i + 1
            )));
        }

        let raw_quad = lines[idx];
        let bearing_dec = parse_float(lines[idx + 1]);
        let hemi_str = lines[idx + 2].to_ascii_uppercase();
        let distance = parse_float(lines[idx + 3]);

        match parse_map_course(i + 1, raw_quad, bearing_dec, &hemi_str, distance) {
            Ok(course) => courses.push(course),
            Err(msg) => {
                courses.push(Course::Invalid(InvalidCourse::new(
                    msg,
                    if raw_quad.contains(CURVE_MARKER) {
                        PartialType::Curve
                    } else {
                        PartialType::Line
                    },
                    salvage_map_course(raw_quad, bearing_dec, &hemi_str, distance),
                )));
                error_count += 1;
            }
        }

        idx += 4;
    }

    Ok(ParsedFile {
        name,
        lot,
        courses,
        error_count,
    })
}

fn parse_map_course(
    number: i64,
    raw_quad: &str,
    bearing_dec: f64,
    hemi_str: &str,
    distance: f64,
) -> Result<Course, String> {
    let hemi = Hemi::parse(hemi_str);
    if bearing_dec.is_nan() || hemi.is_none() || distance.is_nan() {
        return Err(format!("Invalid data in course #{} fields", number));
    }
    let hemi = hemi.unwrap();
    let raw_bearing = fmt_bear_dms(bearing_dec);

    if raw_quad.contains(CURVE_MARKER) {
        let parts: Vec<&str> = raw_quad.split(FIELD_SEP).collect();
        if parts.len() < 7 {
            return Err(format!("Invalid curve format in course #{}", number));
        }
        let quad = Quad::parse(parts[0]).ok_or_else(|| {
            format!(
                "Invalid curve quadrant value in course #{}: {}",
                number,
                parts[0].to_ascii_uppercase()
            )
        })?;
        let turn = Turn::parse(parts[2]).ok_or_else(|| {
            format!(
                "Invalid curve turn direction in course #{}: {}",
                number,
                parts[2].to_ascii_uppercase()
            )
        })?;

        let radius = parse_float(parts[3]);
        let arc_length = parse_float(parts[4]);
        let chord_length = parse_float(parts[5]);
        let delta_angle = parse_float(parts[6]);
        if radius.is_nan() || arc_length.is_nan() || chord_length.is_nan() || delta_angle.is_nan() {
            return Err(format!("Invalid curve measurements in course #{}", number));
        }

        return Ok(Course::Curve(CurveCourse {
            quad,
            bearing: bearing_dec,
            hemi,
            distance: arc_length,
            chord_length,
            radius,
            delta_angle,
            turn,
            raw_bearing,
            delta_n: 0.0,
            delta_e: 0.0,
        }));
    }

    let quad = Quad::parse(raw_quad).ok_or_else(|| {
        format!(
            "Invalid quadrant value in course #{}: {}",
            number,
            raw_quad.to_ascii_uppercase()
        )
    })?;

    Ok(Course::Line(LineCourse {
        quad,
        bearing: bearing_dec,
        hemi,
        distance,
        raw_bearing,
        delta_n: 0.0,
        delta_e: 0.0,
    }))
}

/// Pull whatever is still usable out of a course that failed, so the UI can
/// pre-fill the correction form.
fn salvage_map_course(
    raw_quad: &str,
    bearing_dec: f64,
    hemi_str: &str,
    distance: f64,
) -> PartialData {
    let is_curve = raw_quad.contains(CURVE_MARKER);
    let parts: Vec<&str> = raw_quad.split(FIELD_SEP).collect();
    let part = |i: usize| parts.get(i).copied().unwrap_or("");

    PartialData {
        quad: Quad::parse(if is_curve { part(0) } else { raw_quad }),
        hemi: Hemi::parse(hemi_str),
        raw_bearing: if bearing_dec.is_nan() {
            None
        } else {
            Some(fmt_bear_dms(bearing_dec))
        },
        radius: if is_curve {
            parse_float_truthy(part(3))
        } else {
            None
        },
        arc_length: if is_curve {
            parse_float_truthy(part(4))
        } else {
            None
        },
        chord_length: if is_curve {
            parse_float_truthy(part(5))
        } else {
            None
        },
        delta_angle: if is_curve {
            parse_float_truthy(part(6))
        } else {
            None
        },
        turn: if is_curve { Turn::parse(part(2)) } else { None },
        distance: if distance.is_nan() {
            None
        } else {
            Some(distance)
        },
    }
}

/// Write a legacy `.MAP` file. Line endings are CRLF, as the DOS original
/// expects.
///
/// Fails if any course is invalid: the header declares a course count, so a
/// file that silently omitted the broken ones would not round-trip.
pub fn generate_map_file(name: &str, lot: &str, courses: &[Course]) -> Result<String, ParseError> {
    if let Some(pos) = courses.iter().position(|c| c.is_invalid()) {
        return Err(ParseError(format!(
            "Cannot export: course #{} is invalid. Fix it first.",
            pos + 1
        )));
    }

    let first_line = if lot.is_empty() {
        name.to_string()
    } else {
        format!("{}{}{}", name, FIELD_SEP, lot)
    };

    let mut out = format!("{}\r\n{}\r\n", first_line, courses.len());

    for course in courses {
        match course {
            Course::Curve(c) => {
                // Curve details ride inside the legacy quadrant field.
                out.push_str(&format!(
                    "{} | CURVE | {} | {:.3} | {:.3} | {:.3} | {:.5}\r\n",
                    c.quad, c.turn, c.radius, c.distance, c.chord_length, c.delta_angle
                ));
                out.push_str(&format!("{:.10}\r\n", c.bearing));
                out.push_str(&format!("{}\r\n", c.hemi));
                // The 1992 program reads this as a straight-line distance, so
                // give it the chord rather than the arc.
                out.push_str(&format!("{:.3}\r\n", c.chord_length));
            }
            Course::Line(c) => {
                out.push_str(&format!("{}\r\n", c.quad));
                out.push_str(&format!("{:.10}\r\n", c.bearing));
                out.push_str(&format!("{}\r\n", c.hemi));
                out.push_str(&format!("{:.3}\r\n", c.distance));
            }
            Course::Invalid(_) => unreachable!("checked above"),
        }
    }

    Ok(out)
}
