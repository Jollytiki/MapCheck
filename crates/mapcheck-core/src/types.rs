//! Course data model.
//!
//! The serde representation is deliberately identical to the JSON shape the
//! browser app has always used, so `app.js` and the library saved in
//! localStorage keep working unchanged.

use serde::{Deserialize, Serialize};

/// North/south half of a quadrant bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quad {
    N,
    S,
}

/// East/west half of a quadrant bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hemi {
    E,
    W,
}

/// Direction a curve turns, viewed along the direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Turn {
    L,
    R,
}

impl Quad {
    pub fn parse(s: &str) -> Option<Quad> {
        match s.trim().to_ascii_uppercase().as_str() {
            "N" => Some(Quad::N),
            "S" => Some(Quad::S),
            _ => None,
        }
    }
}

impl Hemi {
    pub fn parse(s: &str) -> Option<Hemi> {
        match s.trim().to_ascii_uppercase().as_str() {
            "E" => Some(Hemi::E),
            "W" => Some(Hemi::W),
            _ => None,
        }
    }
}

impl Turn {
    pub fn parse(s: &str) -> Option<Turn> {
        match s.trim().to_ascii_uppercase().as_str() {
            "L" => Some(Turn::L),
            "R" => Some(Turn::R),
            _ => None,
        }
    }
}

/// A straight course.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineCourse {
    pub quad: Quad,
    /// Bearing angle in decimal degrees, always 0..=90.
    pub bearing: f64,
    pub hemi: Hemi,
    pub distance: f64,
    #[serde(default)]
    pub raw_bearing: String,
    /// Latitude, filled in by [`crate::traverse::recalculate`].
    #[serde(default)]
    pub delta_n: f64,
    /// Departure, filled in by [`crate::traverse::recalculate`].
    #[serde(default)]
    pub delta_e: f64,
}

/// A circular curve course. `bearing`/`quad`/`hemi` describe the *chord*
/// bearing; `distance` holds the arc length, matching the JS app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveCourse {
    pub quad: Quad,
    pub bearing: f64,
    pub hemi: Hemi,
    /// Arc length.
    pub distance: f64,
    pub chord_length: f64,
    pub radius: f64,
    /// Central angle in decimal degrees.
    pub delta_angle: f64,
    pub turn: Turn,
    #[serde(default)]
    pub raw_bearing: String,
    #[serde(default)]
    pub delta_n: f64,
    #[serde(default)]
    pub delta_e: f64,
}

/// Which kind of course a failed line was trying to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartialType {
    Line,
    Curve,
}

/// Whatever could still be salvaged from a course that failed to parse, so the
/// UI can pre-fill the correction form.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
// Every field defaults, so the UI can hand back a bare `{}`.
#[serde(rename_all = "camelCase", default)]
pub struct PartialData {
    pub quad: Option<Quad>,
    pub hemi: Option<Hemi>,
    pub raw_bearing: Option<String>,
    pub radius: Option<f64>,
    pub chord_length: Option<f64>,
    pub arc_length: Option<f64>,
    pub delta_angle: Option<f64>,
    pub turn: Option<Turn>,
    pub distance: Option<f64>,
}

/// A course that failed to parse. Kept in place so course numbering survives
/// and the user can fix errors one at a time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidCourse {
    /// Always `true`; `app.js` branches on this flag.
    #[serde(default)]
    pub is_invalid: bool,
    pub error_msg: String,
    pub partial_type: PartialType,
    #[serde(default)]
    pub partial_data: PartialData,
}

impl InvalidCourse {
    pub fn new(error_msg: String, partial_type: PartialType, partial_data: PartialData) -> Self {
        InvalidCourse {
            is_invalid: true,
            error_msg,
            partial_type,
            partial_data,
        }
    }
}

/// One leg of a traverse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Course {
    Line(LineCourse),
    Curve(CurveCourse),
    Invalid(InvalidCourse),
}

impl Course {
    pub fn is_invalid(&self) -> bool {
        matches!(self, Course::Invalid(_))
    }

    /// Bearing/quadrant/hemisphere of the course (the chord, for curves).
    pub fn bearing_parts(&self) -> Option<(Quad, f64, Hemi)> {
        match self {
            Course::Line(c) => Some((c.quad, c.bearing, c.hemi)),
            Course::Curve(c) => Some((c.quad, c.bearing, c.hemi)),
            Course::Invalid(_) => None,
        }
    }

    /// Distance along the ground: arc length for curves.
    pub fn distance(&self) -> f64 {
        match self {
            Course::Line(c) => c.distance,
            Course::Curve(c) => c.distance,
            Course::Invalid(_) => 0.0,
        }
    }

    /// The distance the latitude/departure are computed from: the *chord* for
    /// curves, since the chord is what moves the point.
    pub fn delta_distance(&self) -> f64 {
        match self {
            Course::Line(c) => c.distance,
            Course::Curve(c) => c.chord_length,
            Course::Invalid(_) => 0.0,
        }
    }

    pub fn deltas(&self) -> (f64, f64) {
        match self {
            Course::Line(c) => (c.delta_n, c.delta_e),
            Course::Curve(c) => (c.delta_n, c.delta_e),
            Course::Invalid(_) => (0.0, 0.0),
        }
    }

    pub(crate) fn set_deltas(&mut self, dn: f64, de: f64) {
        match self {
            Course::Line(c) => {
                c.delta_n = dn;
                c.delta_e = de;
            }
            Course::Curve(c) => {
                c.delta_n = dn;
                c.delta_e = de;
            }
            Course::Invalid(_) => {}
        }
    }
}

impl std::fmt::Display for Quad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Quad::N => "N",
            Quad::S => "S",
        })
    }
}

impl std::fmt::Display for Hemi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Hemi::E => "E",
            Hemi::W => "W",
        })
    }
}

impl std::fmt::Display for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Turn::L => "L",
            Turn::R => "R",
        })
    }
}
