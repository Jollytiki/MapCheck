//! Parity with the original JavaScript implementation.
//!
//! `tests/fixtures/parity.json` is produced by running the real `app.js`
//! functions in Node (`node scripts/gen-parity-fixtures.mjs`). These tests
//! replay the same inputs through the Rust port and require the same answers,
//! so the port is checked against what the app actually does rather than
//! against a reading of the source.

use mapcheck_core::prelude::*;
use serde_json::{json, Value};

fn fixtures() -> Value {
    let raw = include_str!("fixtures/parity.json");
    serde_json::from_str(raw).expect("parity.json is valid JSON")
}

fn cases(section: &str) -> Vec<Value> {
    fixtures()[section]
        .as_array()
        .unwrap_or_else(|| panic!("fixture section `{section}` missing"))
        .clone()
}

/// Floats are compared with a small tolerance: JS and Rust agree to well
/// within this, but the last ulp of `sin`/`asin` is not guaranteed identical
/// across libm implementations.
const TOL: f64 = 1e-9;

fn close(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    let diff = (a - b).abs();
    diff <= TOL || diff <= TOL * a.abs().max(b.abs())
}

/// Recursive comparison of a Rust-produced value against the JS expectation.
///
/// Two accommodations, both for keys JS simply leaves off the object:
/// `JSON.stringify` drops `undefined` properties, and the JS assigns
/// `deltaN`/`deltaE` in a later pass. So a key absent on the expected side is
/// satisfied by `null` or `0` on ours.
fn same(actual: &Value, expected: &Value, path: &str) -> Result<(), String> {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            if close(a, e) {
                Ok(())
            } else {
                Err(format!("{path}: {a} != {e}"))
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            for (key, e_val) in e {
                let a_val = a.get(key).unwrap_or(&Value::Null);
                same(a_val, e_val, &format!("{path}.{key}"))?;
            }
            for (key, a_val) in a {
                if e.contains_key(key) {
                    continue;
                }
                let absent_ok = a_val.is_null() || a_val.as_f64() == Some(0.0);
                if !absent_ok {
                    return Err(format!("{path}.{key}: unexpected {a_val}"));
                }
            }
            Ok(())
        }
        (Value::Array(a), Value::Array(e)) => {
            if a.len() != e.len() {
                return Err(format!("{path}: length {} != {}", a.len(), e.len()));
            }
            for (i, (a_val, e_val)) in a.iter().zip(e).enumerate() {
                same(a_val, e_val, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!("{path}: {actual} != {expected}")),
    }
}

/// The one deliberate divergence from the JS.
///
/// The JS drops the raw text of an unparseable quadrant/hemisphere/turn into
/// `partialData` (`quad: "X"`), because nothing there is typed. Rust's `Quad`,
/// `Hemi` and `Turn` can only hold real variants, so the port stores `null`.
/// This is not a behaviour change: `app.js:978` already discards anything that
/// is not `"N"`/`"S"` when pre-filling the correction form, so the user sees
/// the same default either way — the port just rejects the junk one layer
/// earlier. This normalises those fields so the rest of the object is still
/// compared strictly.
fn normalize_expected(value: &mut Value) {
    const TYPED: [(&str, [&str; 2]); 3] = [
        ("quad", ["N", "S"]),
        ("hemi", ["E", "W"]),
        ("turn", ["L", "R"]),
    ];
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(partial)) = map.get_mut("partialData") {
                for (key, allowed) in TYPED {
                    if let Some(v) = partial.get_mut(key) {
                        let keep = v.as_str().is_some_and(|s| allowed.contains(&s));
                        if !keep && !v.is_null() {
                            *v = Value::Null;
                        }
                    }
                }
            }
            for v in map.values_mut() {
                normalize_expected(v);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalize_expected),
        _ => {}
    }
}

fn assert_same(actual: &Value, expected: &Value, context: &str) {
    let mut expected = expected.clone();
    normalize_expected(&mut expected);
    let expected = &expected;
    if let Err(msg) = same(actual, expected, "") {
        panic!("{context}\n  mismatch at {msg}\n  actual:   {actual}\n  expected: {expected}");
    }
}

fn to_value<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("serializes")
}

#[test]
fn bear_dec_matches_js() {
    for case in cases("bearDec") {
        let dms = case["dms"].as_f64().unwrap();
        let expected = case["expected"].as_f64().unwrap();
        let actual = bear_dec(dms);
        assert!(
            close(actual, expected),
            "bearDec({dms}): {actual} != {expected}"
        );
    }
}

#[test]
fn fmt_bear_matches_js() {
    for case in cases("fmtBear") {
        let dec = case["dec"].as_f64().unwrap();
        assert_eq!(
            fmt_bear(dec),
            case["expected"].as_str().unwrap(),
            "fmtBear({dec})"
        );
    }
    for case in cases("fmtBearDMS") {
        let dec = case["dec"].as_f64().unwrap();
        assert_eq!(
            fmt_bear_dms(dec),
            case["expected"].as_str().unwrap(),
            "fmtBearDMS({dec})"
        );
    }
}

#[test]
fn parse_bearing_input_matches_js() {
    for case in cases("parseBearingInput") {
        let input = case["input"].as_str().unwrap();
        let actual = match parse_bearing_input(input) {
            Some(p) => to_value(&p),
            None => Value::Null,
        };
        assert_same(
            &actual,
            &case["expected"],
            &format!("parseBearingInput({input:?})"),
        );
    }
}

#[test]
fn parse_csv_bearing_matches_js() {
    for case in cases("parseCsvBearing") {
        let input = case["input"].as_str().unwrap();
        let actual = match parse_csv_bearing(input) {
            Some((quad, bearing, hemi)) => json!({
                "quad": quad, "bearing": bearing, "hemi": hemi
            }),
            None => Value::Null,
        };
        assert_same(
            &actual,
            &case["expected"],
            &format!("parseCsvBearing({input:?})"),
        );
    }
}

#[test]
fn calculate_deltas_matches_js() {
    for case in cases("calculateDeltas") {
        let quad = if case["quad"] == json!("N") {
            Quad::N
        } else {
            Quad::S
        };
        let hemi = if case["hemi"] == json!("E") {
            Hemi::E
        } else {
            Hemi::W
        };
        let bearing = case["bearing"].as_f64().unwrap();
        let distance = case["distance"].as_f64().unwrap();
        let (dn, de) = calculate_deltas(quad, bearing, hemi, distance);
        let actual = json!({ "deltaN": dn, "deltaE": de });
        assert_same(&actual, &case["expected"], "calculateDeltas");
    }
}

#[test]
fn solve_curve_matches_js() {
    for case in cases("solveCurve") {
        let arg = |k: &str| case[k].as_f64();
        let actual = match solve_curve(arg("r"), arg("l"), arg("d"), arg("c")) {
            Some(s) => to_value(&s),
            None => Value::Null,
        };
        // The JS returns an object with null members when it cannot solve;
        // the Rust returns None. Both mean "unsolvable" to every caller.
        let expected = &case["expected"];
        if actual.is_null() && expected.is_object() {
            let all_null = expected.as_object().unwrap().values().any(|v| v.is_null());
            assert!(
                all_null,
                "solveCurve({case}): returned None but JS solved it"
            );
            continue;
        }
        assert_same(&actual, expected, &format!("solveCurve({case})"));
    }
}

#[test]
fn course_azimuth_and_tangent_match_js() {
    for case in cases("courseAzimuth") {
        let course: Course = serde_json::from_value(case["course"].clone()).unwrap();
        let (quad, bearing, hemi) = course.bearing_parts().unwrap();
        let actual = course_azimuth(quad, bearing, hemi);
        let expected = case["expected"].as_f64().unwrap();
        assert!(
            close(actual, expected),
            "courseAzimuth: {actual} != {expected}"
        );
    }
    for case in cases("courseEndTangent") {
        let course: Course = serde_json::from_value(case["course"].clone()).unwrap();
        let actual = course_end_tangent(&course);
        let expected = case["expected"].as_f64().unwrap();
        assert!(
            close(actual, expected),
            "courseEndTangent: {actual} != {expected}"
        );
    }
}

#[test]
fn azimuth_to_quad_bearing_hemi_matches_js() {
    for case in cases("azimuthToQuadBearingHemi") {
        let az = case["az"].as_f64().unwrap();
        let (quad, bearing, hemi) = azimuth_to_quad_bearing_hemi(az);
        let actual = json!({ "quad": quad, "bearing": bearing, "hemi": hemi });
        assert_same(
            &actual,
            &case["expected"],
            &format!("azimuthToQuadBearingHemi({az})"),
        );
    }
}

#[test]
fn parse_map_file_matches_js() {
    for case in cases("parseMapFile") {
        let content = case["content"].as_str().unwrap();
        match parse_map_file(content) {
            Ok(parsed) => {
                assert!(
                    case["error"].is_null(),
                    "parseMapFile accepted a file JS rejected: {}",
                    case["error"]
                );
                assert_same(&to_value(&parsed), &case["expected"], "parseMapFile");
            }
            Err(e) => {
                let expected = case["error"]
                    .as_str()
                    .unwrap_or_else(|| panic!("parseMapFile rejected a file JS accepted: {}", e.0));
                assert_eq!(e.0, expected, "parseMapFile error text");
            }
        }
    }
}

#[test]
fn parse_csv_text_matches_js() {
    for case in cases("parseCsvTextFile") {
        let content = case["content"].as_str().unwrap();
        let parsed = parse_csv_text(content, "courses.txt");
        assert_same(
            &to_value(&parsed),
            &case["expected"],
            &format!("parseCsvTextFile({content:?})"),
        );
    }
}

#[test]
fn detect_and_parse_matches_js() {
    for case in cases("detectAndParseFile") {
        let content = case["content"].as_str().unwrap();
        let file_name = case["fileName"].as_str().unwrap();
        match detect_and_parse(content, file_name) {
            Ok(parsed) => assert_same(&to_value(&parsed), &case["expected"], "detectAndParseFile"),
            Err(e) => assert_eq!(
                e.0,
                case["error"].as_str().unwrap_or(""),
                "detectAndParseFile error text"
            ),
        }
    }
}

#[test]
fn generate_map_file_matches_js() {
    for case in cases("generateMapFile") {
        let name = case["name"].as_str().unwrap();
        let lot = case["lot"].as_str().unwrap();
        let courses: Vec<Course> = serde_json::from_value(case["courses"].clone()).unwrap();
        let actual = generate_map_file(name, lot, &courses).expect("valid courses");
        assert_eq!(
            actual,
            case["expected"].as_str().unwrap(),
            "generateMapFile({name})"
        );
    }
}
