//! Survey correctness, checked against geometry that can be worked out by
//! hand rather than against the previous implementation.
//!
//! `tests/parity.rs` proves the port matches the JS app. These prove the
//! answers are right in the first place.

use mapcheck_core::prelude::*;

fn line(quad: Quad, bearing: f64, hemi: Hemi, distance: f64) -> Course {
    Course::Line(LineCourse {
        quad,
        bearing,
        hemi,
        distance,
        raw_bearing: fmt_bear_dms(bearing),
        delta_n: 0.0,
        delta_e: 0.0,
    })
}

fn curve(
    quad: Quad,
    bearing: f64,
    hemi: Hemi,
    radius: f64,
    delta_angle: f64,
    turn: Turn,
) -> Course {
    let solved = solve_curve(Some(radius), None, Some(delta_angle), None).expect("solvable");
    Course::Curve(CurveCourse {
        quad,
        bearing,
        hemi,
        distance: solved.l,
        chord_length: solved.c,
        radius,
        delta_angle,
        turn,
        raw_bearing: fmt_bear_dms(bearing),
        delta_n: 0.0,
        delta_e: 0.0,
    })
}

/// A 100' square, walked clockwise from the south-west corner.
fn square() -> Vec<Course> {
    vec![
        line(Quad::N, 0.0, Hemi::E, 100.0),
        line(Quad::N, 90.0, Hemi::E, 100.0),
        line(Quad::S, 0.0, Hemi::E, 100.0),
        line(Quad::S, 90.0, Hemi::W, 100.0),
    ]
}

#[test]
fn packed_dms_converts_the_way_the_1992_program_did() {
    // 35.0020 is 35 degrees 00 minutes 20 seconds, not 35.002 degrees.
    assert!((bear_dec(35.0020) - (35.0 + 20.0 / 3600.0)).abs() < 1e-12);
    assert!((bear_dec(45.3000) - 45.5).abs() < 1e-12);
    assert!((bear_dec(89.5959) - (89.0 + 59.0 / 60.0 + 59.0 / 3600.0)).abs() < 1e-12);
    assert_eq!(bear_dec(0.0), 0.0);
    // Negative angles keep their sign.
    assert!((bear_dec(-45.3000) + 45.5).abs() < 1e-12);
}

#[test]
fn dms_formatting_round_trips() {
    for packed in [35.0020, 45.3000, 89.5959, 12.3456, 1.0101] {
        let decimal = bear_dec(packed);
        assert_eq!(
            fmt_bear_dms(decimal),
            format!("{:.4}", packed),
            "round trip of {packed}"
        );
    }
}

#[test]
fn seconds_rounding_carries_into_minutes_and_degrees() {
    // 44 degrees 59 minutes 59.6 seconds must not format as 44-59-60.
    let dec = 44.0 + 59.0 / 60.0 + 59.6 / 3600.0;
    assert_eq!(fmt_bear(dec), "45-00-00");
}

#[test]
fn a_closed_square_has_no_error() {
    let mut courses = square();
    let closure = recalculate(&mut courses);

    assert!(closure.total_delta_n.abs() < 1e-9, "latitudes must cancel");
    assert!(closure.total_delta_e.abs() < 1e-9, "departures must cancel");
    assert!(closure.error < 1e-9);
    assert!(closure.is_perfect());
    assert_eq!(closure.precision, None, "perfect closure has no ratio");
    assert!((closure.total_distance - 400.0).abs() < 1e-9);
}

#[test]
fn latitudes_and_departures_use_the_right_signs() {
    let mut courses = vec![
        line(Quad::N, 45.0, Hemi::E, 100.0),
        line(Quad::S, 45.0, Hemi::E, 100.0),
        line(Quad::S, 45.0, Hemi::W, 100.0),
        line(Quad::N, 45.0, Hemi::W, 100.0),
    ];
    recalculate(&mut courses);
    let leg = 100.0 * (45.0f64).to_radians().cos();

    let expected = [(leg, leg), (-leg, leg), (-leg, -leg), (leg, -leg)];
    for (course, (dn, de)) in courses.iter().zip(expected) {
        let (actual_n, actual_e) = course.deltas();
        assert!((actual_n - dn).abs() < 1e-9, "latitude sign");
        assert!((actual_e - de).abs() < 1e-9, "departure sign");
    }
}

#[test]
fn an_open_traverse_reports_the_gap_and_precision() {
    // Same square, but the last leg is a foot short: the figure fails to close
    // by exactly one foot, over 399 feet walked.
    let mut courses = square();
    if let Course::Line(last) = courses.last_mut().unwrap() {
        last.distance = 99.0;
    }
    let closure = recalculate(&mut courses);

    assert!((closure.error - 1.0).abs() < 1e-9, "one foot of misclosure");
    assert!(!closure.is_perfect());
    assert_eq!(
        closure.precision,
        Some(399.0),
        "399 feet walked per foot of error"
    );
}

#[test]
fn area_of_a_square_is_exact() {
    let mut courses = square();
    recalculate(&mut courses);
    let area = calculate_area(&courses).expect("closed figure");
    assert!((area - 10_000.0).abs() < 1e-6, "got {area}");
}

#[test]
fn area_ignores_the_direction_of_travel() {
    let mut clockwise = square();
    recalculate(&mut clockwise);

    // The same square walked the other way round.
    let mut counter = vec![
        line(Quad::N, 90.0, Hemi::E, 100.0),
        line(Quad::N, 0.0, Hemi::E, 100.0),
        line(Quad::S, 90.0, Hemi::W, 100.0),
        line(Quad::S, 0.0, Hemi::E, 100.0),
    ];
    recalculate(&mut counter);

    let a = calculate_area(&clockwise).unwrap();
    let b = calculate_area(&counter).unwrap();
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn four_quarter_curves_enclose_a_circle() {
    // Four 90-degree curves of radius 100 close on themselves and must
    // enclose pi * r^2. This exercises the circular-segment correction: the
    // chord polygon alone would only give 20,000.
    let r = 100.0;
    let mut courses = vec![
        curve(Quad::N, 45.0, Hemi::E, r, 90.0, Turn::R),
        curve(Quad::S, 45.0, Hemi::E, r, 90.0, Turn::R),
        curve(Quad::S, 45.0, Hemi::W, r, 90.0, Turn::R),
        curve(Quad::N, 45.0, Hemi::W, r, 90.0, Turn::R),
    ];
    let closure = recalculate(&mut courses);
    assert!(closure.error < 1e-9, "circle must close");

    let area = calculate_area(&courses).expect("closed figure");
    let expected = std::f64::consts::PI * r * r;
    assert!(
        (area - expected).abs() < 1e-6,
        "got {area}, want {expected}"
    );

    // Arc length round the whole circle is the circumference.
    let expected_circumference = 2.0 * std::f64::consts::PI * r;
    assert!((closure.total_distance - expected_circumference).abs() < 1e-9);
}

#[test]
fn curves_bulging_inward_and_outward_differ_by_two_segments() {
    let r = 100.0;
    let delta: f64 = 60.0;
    let segment = {
        let theta: f64 = delta.to_radians();
        0.5 * r * r * (theta - theta.sin())
    };

    let mut left = square();
    left.push(curve(Quad::N, 45.0, Hemi::E, r, delta, Turn::L));
    let mut right = square();
    right.push(curve(Quad::N, 45.0, Hemi::E, r, delta, Turn::R));

    recalculate(&mut left);
    recalculate(&mut right);

    // Both figures have the same chord polygon; only the segment sign differs.
    let difference = calculate_area(&left).unwrap() - calculate_area(&right).unwrap();
    assert!(
        (difference.abs() - 2.0 * segment).abs() < 1e-6,
        "expected two segments of difference, got {difference}"
    );
}

#[test]
fn area_needs_a_closed_valid_figure() {
    let mut two_courses = vec![
        line(Quad::N, 0.0, Hemi::E, 100.0),
        line(Quad::N, 90.0, Hemi::E, 100.0),
    ];
    recalculate(&mut two_courses);
    assert_eq!(
        calculate_area(&two_courses),
        None,
        "two courses cannot enclose area"
    );

    let mut with_invalid = square();
    with_invalid.push(Course::Invalid(InvalidCourse::new(
        "bad".into(),
        PartialType::Line,
        PartialData::default(),
    )));
    recalculate(&mut with_invalid);
    assert_eq!(
        calculate_area(&with_invalid),
        None,
        "a broken course voids the area"
    );
}

#[test]
fn coordinates_start_at_the_point_of_beginning() {
    let mut courses = square();
    recalculate(&mut courses);

    let pts = absolute_point_coordinates(&courses, 5000.0, 10_000.0);
    assert_eq!(pts.len(), courses.len() + 1);
    assert_eq!(pts[0].name, "POB");
    assert_eq!(pts[0].n, Some(5000.0));
    assert_eq!(pts[0].e, Some(10_000.0));

    // First leg runs due north 100 feet.
    assert!((pts[1].n.unwrap() - 5100.0).abs() < 1e-9);
    assert!((pts[1].e.unwrap() - 10_000.0).abs() < 1e-9);

    // A closed figure returns to where it started.
    let last = pts.last().unwrap();
    assert_eq!(last.name, "Closure");
    assert!((last.n.unwrap() - 5000.0).abs() < 1e-9);
    assert!((last.e.unwrap() - 10_000.0).abs() < 1e-9);
}

#[test]
fn an_invalid_course_breaks_the_coordinate_chain() {
    let mut courses = vec![
        line(Quad::N, 0.0, Hemi::E, 100.0),
        Course::Invalid(InvalidCourse::new(
            "bad".into(),
            PartialType::Line,
            PartialData::default(),
        )),
        line(Quad::S, 0.0, Hemi::E, 100.0),
    ];
    recalculate(&mut courses);
    let pts = absolute_point_coordinates(&courses, 0.0, 0.0);

    assert_eq!(pts[2].n, None, "the broken course has no coordinate");
    // The chain resumes from the last good point rather than giving up.
    assert!((pts[3].n.unwrap() - 0.0).abs() < 1e-9);
}

#[test]
fn a_curve_solves_the_same_from_any_pair_of_knowns() {
    let r = 250.0;
    let delta = 37.5;
    let reference = solve_curve(Some(r), None, Some(delta), None).expect("R + delta");

    let pairs = [
        solve_curve(Some(reference.r), Some(reference.l), None, None),
        solve_curve(Some(reference.r), None, None, Some(reference.c)),
        solve_curve(None, Some(reference.l), Some(reference.delta), None),
        solve_curve(None, None, Some(reference.delta), Some(reference.c)),
        solve_curve(None, Some(reference.l), None, Some(reference.c)),
    ];

    for (i, solved) in pairs.iter().enumerate() {
        let s = solved.unwrap_or_else(|| panic!("pair {i} failed to solve"));
        assert!((s.r - reference.r).abs() < 1e-5, "pair {i}: radius {}", s.r);
        assert!((s.l - reference.l).abs() < 1e-5, "pair {i}: arc {}", s.l);
        assert!(
            (s.delta - reference.delta).abs() < 1e-5,
            "pair {i}: delta {}",
            s.delta
        );
        assert!((s.c - reference.c).abs() < 1e-5, "pair {i}: chord {}", s.c);
    }
}

#[test]
fn a_semicircle_has_the_expected_arc_and_chord() {
    let solved = solve_curve(Some(100.0), None, Some(180.0), None).expect("solvable");
    assert!(
        (solved.l - std::f64::consts::PI * 100.0).abs() < 1e-9,
        "half circumference"
    );
    assert!((solved.c - 200.0).abs() < 1e-9, "chord is the diameter");
}

#[test]
fn impossible_curve_geometry_is_rejected() {
    // A chord cannot be longer than the diameter.
    assert_eq!(solve_curve(Some(10.0), None, None, Some(25.0)), None);
    // A chord cannot be longer than its own arc.
    assert_eq!(solve_curve(None, Some(50.0), None, Some(60.0)), None);
    // Nor equal to it, which would mean a curve of infinite radius.
    assert_eq!(solve_curve(None, Some(50.0), None, Some(50.0)), None);
    // A zero central angle is not a curve.
    assert_eq!(solve_curve(None, None, Some(0.0), Some(50.0)), None);
    // One known is never enough.
    assert_eq!(solve_curve(Some(100.0), None, None, None), None);
    assert_eq!(solve_curve(None, None, None, None), None);
}

#[test]
fn a_tangent_curve_leaves_along_the_previous_bearing() {
    // Due north, then a curve turning right through 90 degrees: the chord
    // must bear N45E and the curve must end heading due east.
    let straight = line(Quad::N, 0.0, Hemi::E, 100.0);
    assert!((course_end_tangent(&straight) - 0.0).abs() < 1e-9);

    let bend = curve(Quad::N, 45.0, Hemi::E, 100.0, 90.0, Turn::R);
    assert!(
        (course_end_tangent(&bend) - 90.0).abs() < 1e-9,
        "ends due east"
    );

    let bend_left = curve(Quad::N, 45.0, Hemi::W, 100.0, 90.0, Turn::L);
    assert!(
        (course_end_tangent(&bend_left) - 270.0).abs() < 1e-9,
        "ends due west"
    );
}

#[test]
fn quadrant_bearings_and_azimuths_round_trip() {
    for az in [0.0, 15.0, 89.9, 90.0, 135.0, 180.0, 225.0, 270.0, 359.9] {
        let (quad, bearing, hemi) = azimuth_to_quad_bearing_hemi(az);
        assert!(
            (0.0..=90.0).contains(&bearing),
            "bearing {bearing} out of range"
        );
        let back = course_azimuth(quad, bearing, hemi);
        assert!(
            (back - az).abs() < 1e-9,
            "{az} -> {quad}{bearing}{hemi} -> {back}"
        );
    }
}

#[test]
fn a_plat_survives_a_round_trip_through_the_map_format() {
    let mut original = square();
    original.push(curve(Quad::N, 45.0, Hemi::E, 200.0, 22.5, Turn::R));
    recalculate(&mut original);

    let text = generate_map_file("Round Trip", "Lot 7", &original).expect("valid");
    let parsed = parse_map_file(&text).expect("re-reads");

    assert_eq!(parsed.name, "Round Trip");
    assert_eq!(parsed.lot, "Lot 7");
    assert_eq!(parsed.error_count, 0);
    assert_eq!(parsed.courses.len(), original.len());

    for (before, after) in original.iter().zip(&parsed.courses) {
        let (q1, b1, h1) = before.bearing_parts().unwrap();
        let (q2, b2, h2) = after.bearing_parts().unwrap();
        assert_eq!(q1, q2);
        assert_eq!(h1, h2);
        assert!((b1 - b2).abs() < 1e-9, "bearing drifted");
        assert!(
            (before.distance() - after.distance()).abs() < 1e-3,
            "distance drifted"
        );
    }
}

#[test]
fn a_map_file_with_no_lot_omits_the_separator() {
    let courses = square();
    let text = generate_map_file("No Lot", "", &courses).expect("valid");
    assert!(text.starts_with("No Lot\r\n4\r\n"), "got {:?}", &text[..20]);
    assert_eq!(parse_map_file(&text).unwrap().lot, "");
}

#[test]
fn an_invalid_course_cannot_be_exported() {
    let mut courses = square();
    courses.push(Course::Invalid(InvalidCourse::new(
        "bad".into(),
        PartialType::Line,
        PartialData::default(),
    )));
    let err = generate_map_file("Broken", "", &courses).unwrap_err();
    assert!(err.0.contains("course #5"), "got {}", err.0);
}

// Regression: chord bearing with the documented `CB` prefix, and a bare `R`
// turn direction, both used to be rejected on import.
#[test]
fn csv_curve_accepts_cb_prefix_and_bare_r_turn() {
    use mapcheck_core::csvfile::parse_csv_text;
    use mapcheck_core::types::{Course, Turn};
    let txt = "1, N 00-00-00 E, 100\n\
               2, CBN 45-00-00 E, C, R500, LA157.08\n\
               3, cb S 10-00-00 E, C, L, R150, D38.5633\n\
               4, C, R, R150, CH100\n\
               5, C, R150, CH100, R\n";
    let f = parse_csv_text(txt, "t.txt");
    assert_eq!(f.error_count, 0, "{:?}", f.courses);
    let turns: Vec<Turn> = f.courses.iter().filter_map(|c| match c {
        Course::Curve(cc) => Some(cc.turn),
        _ => None,
    }).collect();
    assert_eq!(turns, vec![Turn::L, Turn::L, Turn::R, Turn::R]);
    if let Course::Curve(cc) = &f.courses[4] {
        assert!((cc.radius - 150.0).abs() < 1e-9, "trailing R clobbered radius");
    }
}
