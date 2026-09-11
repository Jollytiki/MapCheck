//! Circular curve solver.
//!
//! A circular curve is fixed by any two of radius, arc length, central angle
//! and chord length; this solves for the remaining two.

use serde::{Deserialize, Serialize};

/// A fully resolved curve.
///
/// The serde field names are the capitalised survey symbols the JS app already
/// reads off this object (`solved.R`, `solved.Delta`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CurveSolution {
    /// Radius.
    #[serde(rename = "R")]
    pub r: f64,
    /// Arc length.
    #[serde(rename = "L")]
    pub l: f64,
    /// Central angle, decimal degrees.
    #[serde(rename = "Delta")]
    pub delta: f64,
    /// Chord length.
    #[serde(rename = "C")]
    pub c: f64,
}

impl CurveSolution {
    /// Whether every parameter came out finite and positive — the check the
    /// importer applies before accepting a solved curve.
    pub fn is_valid(&self) -> bool {
        [self.r, self.l, self.delta, self.c]
            .iter()
            .all(|v| !v.is_nan() && *v > 0.0)
    }
}

/// JS truthiness for the delta input: zero and NaN both mean "not supplied".
fn truthy(v: Option<f64>) -> Option<f64> {
    match v {
        Some(x) if x != 0.0 && !x.is_nan() => Some(x),
        _ => None,
    }
}

/// Solve a circular curve from any two of radius, arc length, central angle
/// (degrees) and chord length.
///
/// Returns `None` when fewer than two are supplied or the geometry is
/// impossible (a chord longer than the diameter, a zero central angle, a
/// chord/arc ratio outside `0 < C/L < 1`).
pub fn solve_curve(
    radius: Option<f64>,
    arc_length: Option<f64>,
    delta_deg: Option<f64>,
    chord: Option<f64>,
) -> Option<CurveSolution> {
    let delta_rad = truthy(delta_deg).map(|d| d.to_radians());

    let (r, l, delta_rad, c) = match (radius, arc_length, delta_rad, chord) {
        // Radius + arc length
        (Some(r), Some(l), _, _) => {
            let dr = l / r;
            (r, l, dr, 2.0 * r * (dr / 2.0).sin())
        }
        // Radius + central angle
        (Some(r), None, Some(dr), _) => (r, r * dr, dr, 2.0 * r * (dr / 2.0).sin()),
        // Radius + chord
        (Some(r), None, None, Some(c)) => {
            if c > 2.0 * r {
                return None; // chord cannot exceed the diameter
            }
            let dr = 2.0 * (c / (2.0 * r)).asin();
            (r, r * dr, dr, c)
        }
        // Arc length + central angle
        (None, Some(l), Some(dr), _) => {
            if dr == 0.0 {
                return None;
            }
            let r = l / dr;
            (r, l, dr, 2.0 * r * (dr / 2.0).sin())
        }
        // Chord + central angle
        (None, None, Some(dr), Some(c)) => {
            if dr == 0.0 {
                return None;
            }
            let r = c / (2.0 * (dr / 2.0).sin());
            (r, r * dr, dr, c)
        }
        // Arc length + chord: no closed form, solve sin(x) = k*x for x = Δ/2.
        (None, Some(l), None, Some(c)) => {
            let k = c / l;
            if !(0.0 < k && k < 1.0) {
                return None;
            }
            let dr = solve_half_delta(k) * 2.0;
            (l / dr, l, dr, c)
        }
        _ => return None, // fewer than two knowns
    };

    Some(CurveSolution {
        r,
        l,
        delta: dr_to_deg(delta_rad),
        c,
    })
}

fn dr_to_deg(rad: f64) -> f64 {
    rad.to_degrees()
}

/// Newton-Raphson on `sin(x) - k*x = 0`, where `k` is the chord/arc ratio and
/// `x` is half the central angle. The series expansion of that equation gives
/// `sqrt(6(1-k))` as a starting guess that converges in a handful of steps.
fn solve_half_delta(k: f64) -> f64 {
    let mut x = (6.0 * (1.0 - k)).sqrt();
    for _ in 0..100 {
        let fx = x.sin() - k * x;
        let dfx = x.cos() - k;
        let next = x - fx / dfx;
        if (next - x).abs() < 1e-7 {
            return next;
        }
        x = next;
    }
    x
}
