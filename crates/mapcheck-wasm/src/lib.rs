//! Browser bindings for `mapcheck-core`.
//!
//! Every function takes and returns the same JSON shapes `app.js` already
//! used for courses, so the UI layer only has to swap which function it calls.
//! Course arrays cross the boundary as plain JS objects via serde.

use mapcheck_core::prelude::*;
// `Closure` also exists in wasm_bindgen's prelude, so the traverse summary is
// imported under a clearer name.
use mapcheck_core::traverse::Closure as ClosureSummary;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn courses_from(value: JsValue) -> Result<Vec<Course>, JsError> {
    from_value(value).map_err(err)
}

/// What [`recalculate`] hands back: the courses with latitudes and departures
/// filled in, plus the closure summary and area.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Recalculated {
    courses: Vec<Course>,
    closure: ClosureSummary,
    /// `null` when the figure cannot enclose an area.
    area: Option<f64>,
}

/// Compute latitudes/departures, closure and area for a set of courses.
#[wasm_bindgen(js_name = recalculate)]
pub fn recalculate_js(courses: JsValue) -> Result<JsValue, JsError> {
    let mut courses = courses_from(courses)?;
    let closure = recalculate(&mut courses);
    let area = calculate_area(&courses);
    to_value(&Recalculated {
        courses,
        closure,
        area,
    })
    .map_err(err)
}

/// Traverse points relative to the point of beginning at (0, 0).
#[wasm_bindgen(js_name = platCoordinates)]
pub fn plat_coordinates_js(courses: JsValue) -> Result<JsValue, JsError> {
    let mut courses = courses_from(courses)?;
    recalculate(&mut courses);
    to_value(&plat_coordinates(&courses)).map_err(err)
}

/// Traverse points in real northing/easting coordinates.
#[wasm_bindgen(js_name = absolutePointCoordinates)]
pub fn absolute_point_coordinates_js(
    courses: JsValue,
    start_northing: f64,
    start_easting: f64,
) -> Result<JsValue, JsError> {
    let mut courses = courses_from(courses)?;
    recalculate(&mut courses);
    to_value(&absolute_point_coordinates(
        &courses,
        start_northing,
        start_easting,
    ))
    .map_err(err)
}

/// Area enclosed by the traverse, or `null` if it cannot be computed.
#[wasm_bindgen(js_name = calculateArea)]
pub fn calculate_area_js(courses: JsValue) -> Result<Option<f64>, JsError> {
    let mut courses = courses_from(courses)?;
    recalculate(&mut courses);
    Ok(calculate_area(&courses))
}

/// Packed DD.MMSS to decimal degrees.
#[wasm_bindgen(js_name = bearDec)]
pub fn bear_dec_js(dms: f64) -> f64 {
    bear_dec(dms)
}

/// Decimal degrees to `DD-MM-SS`.
#[wasm_bindgen(js_name = fmtBear)]
pub fn fmt_bear_js(dec: f64) -> String {
    fmt_bear(dec)
}

/// Decimal degrees to packed `DD.MMSS`.
#[wasm_bindgen(js_name = fmtBearDMS)]
pub fn fmt_bear_dms_js(dec: f64) -> String {
    fmt_bear_dms(dec)
}

/// Parse a typed bearing. Returns `null` if it is unparseable or out of range.
#[wasm_bindgen(js_name = parseBearingInput)]
pub fn parse_bearing_input_js(input: &str) -> Result<JsValue, JsError> {
    match parse_bearing_input(input) {
        Some(parsed) => to_value(&parsed).map_err(err),
        None => Ok(JsValue::NULL),
    }
}

/// Solve a circular curve from any two of radius, arc length, central angle
/// (degrees) and chord. Pass `null`/`undefined` for the unknowns. Returns
/// `null` if the geometry is impossible.
#[wasm_bindgen(js_name = solveCurve)]
pub fn solve_curve_js(
    radius: Option<f64>,
    arc_length: Option<f64>,
    delta_deg: Option<f64>,
    chord: Option<f64>,
) -> Result<JsValue, JsError> {
    match solve_curve(radius, arc_length, delta_deg, chord) {
        Some(solution) => to_value(&solution).map_err(err),
        None => Ok(JsValue::NULL),
    }
}

/// Azimuth of the tangent where a course ends.
#[wasm_bindgen(js_name = courseEndTangent)]
pub fn course_end_tangent_js(course: JsValue) -> Result<f64, JsError> {
    let course: Course = from_value(course).map_err(err)?;
    Ok(course_end_tangent(&course))
}

#[derive(serde::Serialize)]
struct QuadBearingHemi {
    quad: Quad,
    bearing: f64,
    hemi: Hemi,
}

/// A 0..360 azimuth back to a quadrant bearing.
#[wasm_bindgen(js_name = azimuthToQuadBearingHemi)]
pub fn azimuth_to_quad_bearing_hemi_js(az: f64) -> Result<JsValue, JsError> {
    let (quad, bearing, hemi) = azimuth_to_quad_bearing_hemi(az);
    to_value(&QuadBearingHemi {
        quad,
        bearing,
        hemi,
    })
    .map_err(err)
}

/// Read a plat file, auto-detecting legacy `.MAP` versus CSV/text.
#[wasm_bindgen(js_name = detectAndParseFile)]
pub fn detect_and_parse_js(content: &str, file_name: &str) -> Result<JsValue, JsError> {
    let parsed = detect_and_parse(content, file_name).map_err(err)?;
    to_value(&parsed).map_err(err)
}

/// Write a legacy `.MAP` file.
#[wasm_bindgen(js_name = generateMapFile)]
pub fn generate_map_file_js(name: &str, lot: &str, courses: JsValue) -> Result<String, JsError> {
    let courses = courses_from(courses)?;
    generate_map_file(name, lot, &courses).map_err(err)
}

/// Latitude and departure for a single course.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Deltas {
    delta_n: f64,
    delta_e: f64,
}

/// Latitude/departure for one bearing and distance.
#[wasm_bindgen(js_name = calculateDeltas)]
pub fn calculate_deltas_js(
    quad: &str,
    bearing_dec: f64,
    hemi: &str,
    distance: f64,
) -> Result<JsValue, JsError> {
    let quad = Quad::parse(quad).ok_or_else(|| JsError::new("quadrant must be N or S"))?;
    let hemi = Hemi::parse(hemi).ok_or_else(|| JsError::new("hemisphere must be E or W"))?;
    let (delta_n, delta_e) = calculate_deltas(quad, bearing_dec, hemi, distance);
    to_value(&Deltas { delta_n, delta_e }).map_err(err)
}

/// Azimuth of a course, clockwise from north. An invalid course has no
/// bearing and reports 0, matching the JS it replaces.
#[wasm_bindgen(js_name = getCourseAzimuth)]
pub fn course_azimuth_js(course: JsValue) -> Result<f64, JsError> {
    let course: Course = from_value(course).map_err(err)?;
    Ok(match course.bearing_parts() {
        Some((quad, bearing, hemi)) => course_azimuth(quad, bearing, hemi),
        None => 0.0,
    })
}

/// Parse a CSV bearing, explicit (`N 35.0020 E`) or shorthand (`135.0020`).
/// Returns `null` if it is unparseable.
#[wasm_bindgen(js_name = parseCsvBearing)]
pub fn parse_csv_bearing_js(input: &str) -> Result<JsValue, JsError> {
    match parse_csv_bearing(input) {
        Some((quad, bearing, hemi)) => to_value(&QuadBearingHemi {
            quad,
            bearing,
            hemi,
        })
        .map_err(err),
        None => Ok(JsValue::NULL),
    }
}

/// Parse a legacy `.MAP` file.
#[wasm_bindgen(js_name = parseMapFile)]
pub fn parse_map_file_js(content: &str) -> Result<JsValue, JsError> {
    let parsed = parse_map_file(content).map_err(err)?;
    to_value(&parsed).map_err(err)
}

/// Parse the CSV/text format.
#[wasm_bindgen(js_name = parseCsvTextFile)]
pub fn parse_csv_text_js(content: &str, file_name: &str) -> Result<JsValue, JsError> {
    to_value(&parse_csv_text(content, file_name)).map_err(err)
}
