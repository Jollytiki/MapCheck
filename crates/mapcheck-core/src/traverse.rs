//! Traverse computation: latitudes/departures, closure, area, coordinates.

use serde::{Deserialize, Serialize};

use crate::bearing::{course_azimuth, DEG_TO_RAD};
use crate::types::{Course, Hemi, Quad, Turn};

/// Latitude (ΔN) and departure (ΔE) for one course.
///
/// Port of the `dn#`/`de#` calculation in `MAPCHECK.BAS` (line 203).
pub fn calculate_deltas(quad: Quad, bearing_dec: f64, hemi: Hemi, distance: f64) -> (f64, f64) {
    let rad = bearing_dec * DEG_TO_RAD;
    let dn = distance * rad.cos();
    let de = distance * rad.sin();
    (
        if quad == Quad::N { dn } else { -dn },
        if hemi == Hemi::E { de } else { -de },
    )
}

/// Closure summary for a whole traverse.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Closure {
    /// Sum of course distances (arc length for curves).
    pub total_distance: f64,
    /// Sum of latitudes; zero for a perfectly closed figure.
    pub total_delta_n: f64,
    /// Sum of departures; zero for a perfectly closed figure.
    pub total_delta_e: f64,
    /// Straight-line distance from the last point back to the start.
    pub error: f64,
    /// Denominator of the 1:N precision ratio. `None` when the error is below
    /// the 0.00001 threshold, i.e. precision is effectively infinite.
    pub precision: Option<f64>,
}

impl Closure {
    /// The threshold the app treats as a perfect closure.
    pub fn is_perfect(&self) -> bool {
        self.error < 0.005
    }
}

/// Fill in each course's latitude and departure, and return the closure
/// summary. Invalid courses contribute zero deltas but are left in place.
pub fn recalculate(courses: &mut [Course]) -> Closure {
    let mut total_distance = 0.0;
    let mut total_delta_n = 0.0;
    let mut total_delta_e = 0.0;

    for course in courses.iter_mut() {
        if course.is_invalid() {
            course.set_deltas(0.0, 0.0);
            continue;
        }
        let (quad, bearing, hemi) = match course.bearing_parts() {
            Some(parts) => parts,
            None => continue,
        };
        let (dn, de) = calculate_deltas(quad, bearing, hemi, course.delta_distance());
        total_distance += course.distance();
        total_delta_n += dn;
        total_delta_e += de;
        course.set_deltas(dn, de);
    }

    let error = (total_delta_n * total_delta_n + total_delta_e * total_delta_e).sqrt();
    let precision = if error > 0.00001 {
        Some((total_distance / error).round())
    } else {
        None
    };

    Closure {
        total_distance,
        total_delta_n,
        total_delta_e,
        error,
        precision,
    }
}

/// A plotted point, relative to the point of beginning at (0, 0).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// Easting offset.
    pub x: f64,
    /// Northing offset.
    pub y: f64,
}

/// Walk the traverse from (0, 0), returning `courses.len() + 1` points.
///
/// Requires [`recalculate`] to have run first.
pub fn plat_coordinates(courses: &[Course]) -> Vec<Point> {
    let mut pts = Vec::with_capacity(courses.len() + 1);
    pts.push(Point { x: 0.0, y: 0.0 });
    let (mut x, mut y) = (0.0, 0.0);
    for course in courses {
        let (dn, de) = course.deltas();
        x += de;
        y += dn;
        pts.push(Point { x, y });
    }
    pts
}

/// A traverse point in real northing/easting coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedPoint {
    /// "POB", "Closure", or the course number.
    pub name: String,
    /// Northing; `None` where an invalid course broke the chain.
    pub n: Option<f64>,
    /// Easting; `None` where an invalid course broke the chain.
    pub e: Option<f64>,
}

/// Walk the traverse from a real starting northing/easting.
pub fn absolute_point_coordinates(
    courses: &[Course],
    start_northing: f64,
    start_easting: f64,
) -> Vec<NamedPoint> {
    let mut n = start_northing;
    let mut e = start_easting;
    let mut pts = vec![NamedPoint {
        name: "POB".to_string(),
        n: Some(n),
        e: Some(e),
    }];

    for (index, course) in courses.iter().enumerate() {
        if course.is_invalid() {
            pts.push(NamedPoint {
                name: (index + 1).to_string(),
                n: None,
                e: None,
            });
            continue;
        }
        let (dn, de) = course.deltas();
        n += dn;
        e += de;
        let is_closure = index == courses.len() - 1;
        pts.push(NamedPoint {
            name: if is_closure {
                "Closure".to_string()
            } else {
                (index + 1).to_string()
            },
            n: Some(n),
            e: Some(e),
        });
    }

    pts
}

/// Area enclosed by the traverse, in square units of the input distances.
///
/// Shoelace over the chord polygon, then each curve's circular segment is
/// added (curve turning left, bulging outward) or subtracted (turning right).
/// Returns `None` for fewer than three courses or any invalid course.
pub fn calculate_area(courses: &[Course]) -> Option<f64> {
    if courses.len() < 3 || courses.iter().any(|c| c.is_invalid()) {
        return None;
    }

    let pts = plat_coordinates(courses);
    let mut signed_area = 0.0;
    for w in pts.windows(2) {
        signed_area += w[0].x * w[1].y - w[1].x * w[0].y;
    }
    signed_area /= 2.0;

    for course in courses {
        if let Course::Curve(c) = course {
            if c.radius > 0.0 && c.delta_angle > 0.0 {
                let theta = c.delta_angle.to_radians();
                let segment = 0.5 * c.radius * c.radius * (theta - theta.sin());
                signed_area += if c.turn == Turn::L { segment } else { -segment };
            }
        }
    }

    Some(signed_area.abs())
}

/// Azimuth of the tangent where a course *ends* — the direction the next
/// course leaves in if it is tangent. For a line that is just its azimuth; for
/// a curve the chord azimuth swings by half the central angle.
pub fn course_end_tangent(course: &Course) -> f64 {
    match course {
        Course::Line(c) => course_azimuth(c.quad, c.bearing, c.hemi),
        Course::Curve(c) => {
            let chord_az = course_azimuth(c.quad, c.bearing, c.hemi);
            let half_delta = c.delta_angle / 2.0;
            match c.turn {
                Turn::R => (chord_az + half_delta) % 360.0,
                Turn::L => (chord_az - half_delta + 360.0) % 360.0,
            }
        }
        Course::Invalid(_) => 0.0,
    }
}
