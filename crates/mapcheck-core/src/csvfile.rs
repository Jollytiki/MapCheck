//! The comma-separated text format.
//!
//! One course per line. Lines are `num, bearing, distance` for a straight
//! course, or `num, [bearing,] C, <params>` for a curve, where the params are
//! prefixed values: `R` radius, `LA`/`RA` arc length (and turn direction),
//! `CL`/`CD`/`CH` chord length, `CB` chord bearing, `D` central angle. Blank
//! lines and `#` / `//` comments are ignored.

use crate::bearing::{bear_dec, fmt_bear_dms, parse_csv_bearing};
use crate::curve::solve_curve;
use crate::jsnum::parse_float;
use crate::mapfile::{significant_lines, ParsedFile};
use crate::traverse::course_end_tangent;
use crate::types::{
    Course, CurveCourse, InvalidCourse, LineCourse, PartialData, PartialType, Turn,
};

/// Parse the CSV/text format. Individual bad lines become
/// [`Course::Invalid`] entries rather than failing the whole file.
pub fn parse_csv_text(content: &str, file_name: &str) -> ParsedFile {
    let lines = significant_lines(content);
    let mut courses: Vec<Course> = Vec::new();
    let mut error_count = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let line_no = i + 1;
        let parts: Vec<String> = line.split(',').map(|p| p.trim().to_string()).collect();

        if parts.len() < 3 {
            courses.push(Course::Invalid(InvalidCourse::new(
                format!(
                    "Line {}: Invalid format. Expected at least 3 values.",
                    line_no
                ),
                PartialType::Line,
                PartialData::default(),
            )));
            error_count += 1;
            continue;
        }

        let course_num = parts[0].clone();

        // A `C` in field 2 means the bearing was omitted (tangent curve); in
        // field 3 it means field 2 held the chord bearing.
        let (is_curve, param_start, initial_bearing) = if parts[1].eq_ignore_ascii_case("C") {
            (true, 2usize, None)
        } else if parts[2].eq_ignore_ascii_case("C") {
            (true, 3usize, Some(parts[1].clone()))
        } else {
            (false, 0usize, None)
        };

        let result = if is_curve {
            parse_curve_line(
                &course_num,
                line_no,
                &parts,
                param_start,
                initial_bearing.clone(),
                courses.last(),
            )
        } else {
            parse_line_course(&course_num, line_no, &parts)
        };

        match result {
            Ok(course) => courses.push(course),
            Err(msg) => {
                courses.push(Course::Invalid(InvalidCourse::new(
                    msg,
                    if is_curve {
                        PartialType::Curve
                    } else {
                        PartialType::Line
                    },
                    salvage(&parts, is_curve, param_start, initial_bearing),
                )));
                error_count += 1;
            }
        }
    }

    let name = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| file_name.to_string());

    ParsedFile {
        name,
        lot: String::new(),
        courses,
        error_count,
    }
}

fn parse_line_course(course_num: &str, line_no: usize, parts: &[String]) -> Result<Course, String> {
    let bearing_str = &parts[1];
    let distance = parse_float(&parts[2]);

    let (quad, bearing, hemi) = parse_csv_bearing(bearing_str).ok_or_else(|| {
        format!(
            "Course #{} (on line {}): Invalid bearing value \"{}\". Formats: QDD.MMSS (e.g. 135.0020) or explicit (e.g. N 35.0020 E).",
            course_num, line_no, bearing_str
        )
    })?;

    if distance.is_nan() || distance <= 0.0 {
        return Err(format!(
            "Course #{} (on line {}): Invalid distance value \"{}\". Must be greater than zero.",
            course_num, line_no, parts[2]
        ));
    }

    Ok(Course::Line(LineCourse {
        quad,
        bearing,
        hemi,
        distance,
        raw_bearing: bearing_str.clone(),
        delta_n: 0.0,
        delta_e: 0.0,
    }))
}

/// Whether an explicit chord *length* prefix is present. Without one, a `CH`
/// param is read as the chord bearing; with one, `CH` means the bearing and
/// `CL`/`CD` carry the length.
fn has_chord_length_param(parts: &[String], param_start: usize) -> bool {
    parts[param_start.min(parts.len())..].iter().any(|p| {
        let u = p.trim().to_ascii_uppercase();
        u.starts_with("CL") || u.starts_with("CD")
    })
}

struct CurveParams {
    radius: Option<f64>,
    arc_length: Option<f64>,
    chord_length: Option<f64>,
    delta_angle: Option<f64>,
    turn: Option<Turn>,
    bearing_str: Option<String>,
}

/// Scan the prefixed curve parameters. Order matters: `LA`/`RA` are checked
/// before the bare `R` radius prefix.
fn scan_curve_params(
    parts: &[String],
    param_start: usize,
    initial_bearing: Option<String>,
) -> CurveParams {
    let has_cl = has_chord_length_param(parts, param_start);
    let mut p = CurveParams {
        radius: None,
        arc_length: None,
        chord_length: None,
        delta_angle: None,
        turn: None,
        bearing_str: initial_bearing,
    };

    for raw in parts.iter().skip(param_start) {
        let raw = raw.trim();
        let param = raw.to_ascii_uppercase();
        if param.is_empty() {
            continue;
        }

        // All the prefixes are ASCII, so splitting at a byte offset is safe.
        let tag = param.get(..2).unwrap_or("");
        let value = param.get(2..).unwrap_or("");
        // Bearings keep their original case for the error message.
        let raw_value = raw.get(2..).unwrap_or("");

        match tag {
            "CB" => p.bearing_str = Some(raw_value.to_string()),
            "CL" | "CD" => p.chord_length = Some(parse_float(value)),
            // Ambiguous: chord bearing when a length is given elsewhere on the
            // line, chord length otherwise.
            "CH" if has_cl => p.bearing_str = Some(raw_value.to_string()),
            "CH" => p.chord_length = Some(parse_float(value)),
            "LA" | "RA" => {
                if !value.is_empty() {
                    p.arc_length = Some(parse_float(value));
                }
                p.turn = Some(if tag == "LA" { Turn::L } else { Turn::R });
            }
            _ => {
                if let Some(rest) = param.strip_prefix('R') {
                    p.radius = Some(parse_float(rest));
                } else if let Some(rest) = param.strip_prefix('D') {
                    p.delta_angle = Some(bear_dec(parse_float(rest)));
                } else if param == "L" {
                    p.turn = Some(Turn::L);
                }
            }
        }
    }

    p
}

fn parse_curve_line(
    course_num: &str,
    line_no: usize,
    parts: &[String],
    param_start: usize,
    initial_bearing: Option<String>,
    prev: Option<&Course>,
) -> Result<Course, String> {
    let p = scan_curve_params(parts, param_start, initial_bearing);

    let turn = p.turn.ok_or_else(|| format!(
        "Course #{} (on line {}) (Curve): Turn direction is missing. Please add 'LA' (Left Arc) or 'RA' (Right Arc) for arc length, or specify turn direction as 'L' or 'R'.",
        course_num, line_no
    ))?;

    let supplied = |v: Option<f64>| v.is_some_and(|x| !x.is_nan());
    let mut provided: Vec<&str> = Vec::new();
    if supplied(p.radius) {
        provided.push("Radius (R)");
    }
    if supplied(p.arc_length) {
        provided.push("Arc Length (LA/RA)");
    }
    if supplied(p.chord_length) {
        provided.push("Chord Distance (CH/CD/CL)");
    }
    if supplied(p.delta_angle) {
        provided.push("Delta Angle (D)");
    }

    if provided.len() < 2 {
        let mut missing: Vec<&str> = Vec::new();
        if p.radius.is_none() {
            missing.push("Radius (R)");
        }
        if p.arc_length.is_none() {
            missing.push("Arc Length (LA/RA)");
        }
        if p.chord_length.is_none() {
            missing.push("Chord Distance (CH/CD/CL)");
        }
        if p.delta_angle.is_none() {
            missing.push("Delta Angle (D)");
        }
        let provided_str = if provided.is_empty() {
            "None".to_string()
        } else {
            provided.join(", ")
        };
        return Err(format!(
            "Course #{} (on line {}) (Curve): Not enough parameters to compute curve. Provided: {}. Please provide at least one of the following to resolve the curve geometry: {}.",
            course_num, line_no, provided_str, missing.join(", ")
        ));
    }

    let solved = solve_curve(p.radius, p.arc_length, p.delta_angle, p.chord_length)
        .filter(|s| s.is_valid())
        .ok_or_else(|| {
            format!(
                "Course #{} (on line {}) (Curve): Invalid curve geometry combination.",
                course_num, line_no
            )
        })?;

    let (quad, bearing, hemi) = match &p.bearing_str {
        Some(bs) if !bs.is_empty() => parse_csv_bearing(bs).ok_or_else(|| format!(
            "Course #{} (on line {}) (Curve): Invalid bearing value \"{}\". Formats: QDD.MMSS (e.g. 135.0020) or explicit (e.g. N 35.0020 E).",
            course_num, line_no, bs
        ))?,
        // No chord bearing given: assume the curve is tangent to the previous
        // course and swing half the central angle off that tangent.
        _ => {
            let prev = prev.ok_or_else(|| format!(
                "Course #{} (on line {}) (Curve): No chord bearing specified, and there is no previous course to calculate a tangent curve. Please specify a bearing (e.g. \"1, 135.0020, C, ...\").",
                course_num, line_no
            ))?;
            if prev.is_invalid() {
                return Err(format!(
                    "Course #{} (on line {}) (Curve): Cannot compute tangent chord bearing because the previous course is invalid.",
                    course_num, line_no
                ));
            }
            let start_tangent = course_end_tangent(prev);
            let half_delta = solved.delta / 2.0;
            let chord_az = match turn {
                Turn::R => (start_tangent + half_delta) % 360.0,
                Turn::L => (start_tangent - half_delta + 360.0) % 360.0,
            };
            crate::bearing::azimuth_to_quad_bearing_hemi(chord_az)
        }
    };

    Ok(Course::Curve(CurveCourse {
        quad,
        bearing,
        hemi,
        distance: solved.l,
        chord_length: solved.c,
        radius: solved.r,
        delta_angle: solved.delta,
        turn,
        raw_bearing: p
            .bearing_str
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fmt_bear_dms(bearing)),
        delta_n: 0.0,
        delta_e: 0.0,
    }))
}

/// Recover whatever the failed line did specify, for the correction form.
fn salvage(
    parts: &[String],
    is_curve: bool,
    param_start: usize,
    initial_bearing: Option<String>,
) -> PartialData {
    let mut data = PartialData::default();

    let bearing_str = if is_curve {
        let p = scan_curve_params(parts, param_start, initial_bearing);
        data.radius = p.radius.filter(|v| !v.is_nan());
        data.chord_length = p.chord_length.filter(|v| !v.is_nan());
        data.arc_length = p.arc_length.filter(|v| !v.is_nan());
        data.delta_angle = p.delta_angle.filter(|v| !v.is_nan());
        data.turn = p.turn;
        p.bearing_str
    } else {
        data.distance = {
            let d = parse_float(&parts[2]);
            if d.is_nan() {
                None
            } else {
                Some(d)
            }
        };
        Some(parts[1].clone())
    };

    if let Some(bs) = bearing_str.filter(|s| !s.is_empty()) {
        if let Some((quad, _, hemi)) = parse_csv_bearing(&bs) {
            data.quad = Some(quad);
            data.hemi = Some(hemi);
        }
        data.raw_bearing = Some(bs);
    }

    data
}
