// Generates tests/fixtures/parity.json by running the ORIGINAL app.js
// functions in Node, so the Rust port is checked against real behaviour
// rather than a reading of the source.
//
// app.js touches the DOM at load time, so the pure calculation functions are
// sliced out of it by brace matching and evaluated on their own.
//
// Usage: node scripts/gen-parity-fixtures.mjs

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(join(root, "app.js"), "utf8");

/** Extract `function NAME(...) { ... }` by matching braces. */
function extractFunction(name) {
  const start = src.indexOf(`function ${name}(`);
  if (start === -1) throw new Error(`function ${name} not found in app.js`);
  let i = src.indexOf("{", start);
  let depth = 0;
  for (let j = i; j < src.length; j++) {
    const ch = src[j];
    if (ch === "{") depth++;
    else if (ch === "}") {
      depth--;
      if (depth === 0) return src.slice(start, j + 1);
    }
  }
  throw new Error(`unbalanced braces in ${name}`);
}

const PURE = [
  "bearDec",
  "fmtBear",
  "fmtBearDMS",
  "parseBearingInput",
  "calculateDeltas",
  "solveCurve",
  "getCourseAzimuth",
  "getCourseEndTangent",
  "azimuthToQuadBearingHemi",
  "parseCsvBearing",
  "parseCsvTextFile",
  "parseMapFile",
  "detectAndParseFile",
  "generateMapFile",
];

// `generateMapFile` reads the plat's lot designation from module scope.
const preamble = "let lotDesignation = '';\n";
const body = PURE.map(extractFunction).join("\n\n");
const exportList = `return { ${PURE.join(", ")} };`;
const api = new Function(preamble + body + "\n" + exportList)();

const BEARING_INPUTS = [
  "45", "45.3000", "0", "90", "90.0001", "-5", "35.0020", "12.5959",
  "45-30-00", "45 30 00", "45:30:00", "45°30'00\"", "135.0020", "245.1000",
  "312.0000", "400.0000", "500.0000", "1", "12", "123", "1234", "  45.3000  ",
  "", "abc", "91", "45-61-00", "45-30-61", "N45E", "89.5959", "1 45 30 00",
];

const CSV_BEARINGS = [
  "N 35.0020 E", "S 35.0020 W", "n35.0020e", "135.0020", "435.0020",
  "35.0020", "N 45-30-00 W", "S0E", "garbage", "",
];

const DMS_VALUES = [
  0, 45, 45.5, 35.0020, 89.5959, 0.0001, 12.3456, 90, 1e-9, 44.999999,
  30.596, 60.0000, 33.3333,
];

const CURVE_CASES = [
  [100, 50, null, null],
  [100, null, 30, null],
  [100, null, null, 50],
  [null, 50, 30, null],
  [null, null, 30, 50],
  [null, 50, null, 49],
  [10, null, null, 25],      // chord longer than the diameter
  [null, 50, null, 50],      // chord equal to arc
  [null, 50, null, 0.001],   // degenerate ratio
  [null, null, 0, 50],
  [500, 200, null, null],
  [null, 120, 45, null],
  [250.5, null, 22.5, null],
];

const COURSE_SAMPLES = [
  { type: "line", quad: "N", bearing: 45, hemi: "E", distance: 100 },
  { type: "line", quad: "S", bearing: 30.5, hemi: "W", distance: 250.25 },
  { type: "curve", quad: "N", bearing: 12.5, hemi: "E", distance: 78.5,
    chordLength: 77.9, radius: 200, deltaAngle: 22.5, turn: "R" },
  { type: "curve", quad: "S", bearing: 80, hemi: "W", distance: 78.5,
    chordLength: 77.9, radius: 200, deltaAngle: 22.5, turn: "L" },
];

const MAP_FILES = [
  // A clean four-sided lot.
  "Sample Plat | Lot 4\r\n4\r\nN\r\n45.0000000000\r\nE\r\n100.000\r\nS\r\n45.0000000000\r\nE\r\n100.000\r\nS\r\n45.0000000000\r\nW\r\n100.000\r\nN\r\n45.0000000000\r\nW\r\n100.000\r\n",
  // No lot designation.
  "Plain Name\r\n1\r\nN\r\n30.0000000000\r\nE\r\n50.000\r\n",
  // Curve smuggled into the quadrant field.
  "Curved | Lot 9\r\n2\r\nN | CURVE | R | 200.000 | 78.540 | 77.847 | 22.50000\r\n12.5000000000\r\nE\r\n77.847\r\nS\r\n10.0000000000\r\nW\r\n60.000\r\n",
  // Bad quadrant, bad hemisphere, bad number.
  "Broken\r\n3\r\nX\r\n45.0000000000\r\nE\r\n100.000\r\nN\r\nnotanumber\r\nE\r\n100.000\r\nN\r\n45.0000000000\r\nQ\r\n100.000\r\n",
  // Malformed curve field list.
  "ShortCurve\r\n1\r\nN | CURVE | R | 200.000\r\n12.5000000000\r\nE\r\n77.847\r\n",
];

const CSV_FILES = [
  "1, 135.0020, 100.00\n2, 225.0000, 50.5\n3, 315.0000, 100.00\n4, 45.0000, 50.5\n",
  "# comment line\n// another comment\n1, 145.0000, 100\n",
  "1, 145.0000, 100\n2, C, R200, LA78.54\n",
  "1, 145.0000, 100\n2, 130.0000, C, R200, RA78.54\n",
  "1, C, R200, RA78.54\n",
  "1, 145.0000, 100\n2, C, R200, RA78.54, CL77.85\n",
  "1, 145.0000, 100\n2, C, R200, RA78.54, CH77.85\n",
  "1, 145.0000, 100\n2, C, R200, RA78.54, CL77.85, CH130.0000\n",
  "1, 145.0000, 100\n2, C, R200, RA78.54, CB130.0000\n",
  "1, 145.0000, 100\n2, C, R200, RA78.54, D22.3000\n",
  "1, 145.0000, 100\n2, C, R200\n",
  "1, 145.0000, 100\n2, C, R200, CL50\n",
  "1, 145.0000\n",
  "1, badbearing, 100\n",
  "1, 145.0000, 0\n",
  "1, 145.0000, -50\n",
  "1, 145.0000, notanumber\n",
  "1, C, R200, LA78.54\n2, C, R200, LA78.54\n",
  "1, badbearing, 100\n2, C, R200, LA78.54\n",
  "1, 145.0000, 100\n2, C, R, LA78.54\n",
  "1, 145.0000, 100\n2, C, R200, L, D22.3000\n",
];

const DETECT_FILES = [
  ["Sample\r\n2\r\nN\r\n45.0\r\nE\r\n10.0\r\nS\r\n45.0\r\nW\r\n10.0\r\n", "sample.MAP"],
  ["1, 145.0000, 100\n2, 245.0000, 100\n", "courses.txt"],
  ["only one line\n", "one.txt"],
];

const fixtures = {
  note: "Generated by scripts/gen-parity-fixtures.mjs from app.js. Do not edit by hand.",
  bearDec: DMS_VALUES.map((dms) => ({ dms, expected: api.bearDec(dms) })),
  fmtBear: DMS_VALUES.map((dec) => ({ dec, expected: api.fmtBear(dec) })),
  fmtBearDMS: DMS_VALUES.map((dec) => ({ dec, expected: api.fmtBearDMS(dec) })),
  parseBearingInput: BEARING_INPUTS.map((input) => ({
    input,
    expected: api.parseBearingInput(input),
  })),
  parseCsvBearing: CSV_BEARINGS.map((input) => ({
    input,
    expected: api.parseCsvBearing(input),
  })),
  calculateDeltas: COURSE_SAMPLES.map((c) => ({
    quad: c.quad,
    bearing: c.bearing,
    hemi: c.hemi,
    distance: c.type === "curve" ? c.chordLength : c.distance,
    expected: api.calculateDeltas(
      c.quad,
      c.bearing,
      c.hemi,
      c.type === "curve" ? c.chordLength : c.distance,
    ),
  })),
  solveCurve: CURVE_CASES.map(([r, l, d, c]) => ({
    r, l, d, c,
    expected: api.solveCurve(r, l, d, c),
  })),
  courseAzimuth: COURSE_SAMPLES.map((c) => ({
    course: c,
    expected: api.getCourseAzimuth(c),
  })),
  courseEndTangent: COURSE_SAMPLES.map((c) => ({
    course: c,
    expected: api.getCourseEndTangent(c),
  })),
  azimuthToQuadBearingHemi: [0, 45, 90, 135, 180, 225, 270, 315, 359.9, 360, 420, -30]
    .map((az) => ({ az, expected: api.azimuthToQuadBearingHemi(az) })),
  parseMapFile: MAP_FILES.map((content) => {
    let expected = null;
    let error = null;
    try {
      expected = api.parseMapFile(content);
    } catch (e) {
      error = e.message;
    }
    return { content, expected, error };
  }),
  parseCsvTextFile: CSV_FILES.map((content) => {
    let expected = null;
    let error = null;
    try {
      expected = api.parseCsvTextFile(content, "courses.txt");
    } catch (e) {
      error = e.message;
    }
    return { content, expected, error };
  }),
  detectAndParseFile: DETECT_FILES.map(([content, fileName]) => {
    let expected = null;
    let error = null;
    try {
      expected = api.detectAndParseFile(content, fileName);
    } catch (e) {
      error = e.message;
    }
    return { content, fileName, expected, error };
  }),
  generateMapFile: MAP_FILES.slice(0, 3).map((content) => {
    const parsed = api.parseMapFile(content);
    return {
      name: parsed.name,
      // The JS `generateMapFile` reads the lot from module scope, which the
      // harness pins to "" — so the expected output carries no lot field.
      lot: "",
      courses: parsed.courses,
      expected: api.generateMapFile(parsed.name, parsed.courses),
    };
  }),
};

const out = join(root, "crates/mapcheck-core/tests/fixtures/parity.json");
writeFileSync(out, JSON.stringify(fixtures, null, 2) + "\n");
console.log(`wrote ${out}`);
