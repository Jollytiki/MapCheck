//! Survey traverse math and plat file handling for MapCheck.
//!
//! This is a port of the calculation core that used to live in `app.js`, which
//! was itself a port of the 1992 QuickBASIC `MAPCHECK.BAS`. The browser UI
//! still owns rendering, the canvas, and the plat library; everything
//! numeric — bearings, curves, closure, area, and both file formats — lives
//! here so it can be tested directly.
//!
//! ```
//! use mapcheck_core::prelude::*;
//!
//! let mut courses = vec![
//!     Course::Line(LineCourse {
//!         quad: Quad::N, bearing: 45.0, hemi: Hemi::E, distance: 100.0,
//!         raw_bearing: "145.0000".into(), delta_n: 0.0, delta_e: 0.0,
//!     }),
//! ];
//! let closure = recalculate(&mut courses);
//! assert!((closure.total_delta_n - 70.71).abs() < 0.01);
//! ```

pub mod bearing;
pub mod csvfile;
pub mod curve;
pub mod jsnum;
pub mod mapfile;
pub mod traverse;
pub mod types;

/// Everything a caller normally needs, in one import.
pub mod prelude {
    pub use crate::bearing::{
        azimuth_to_quad_bearing_hemi, bear_dec, course_azimuth, fmt_bear, fmt_bear_dms,
        parse_bearing_input, parse_csv_bearing, ParsedBearing,
    };
    pub use crate::csvfile::parse_csv_text;
    pub use crate::curve::{solve_curve, CurveSolution};
    pub use crate::mapfile::{
        detect_and_parse, generate_map_file, parse_map_file, ParseError, ParsedFile,
    };
    pub use crate::traverse::{
        absolute_point_coordinates, calculate_area, calculate_deltas, course_end_tangent,
        plat_coordinates, recalculate, Closure, NamedPoint, Point,
    };
    pub use crate::types::{
        Course, CurveCourse, Hemi, InvalidCourse, LineCourse, PartialData, PartialType, Quad, Turn,
    };
}
