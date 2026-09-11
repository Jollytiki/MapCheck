// Routes MapCheck's calculations through the Rust core (mapcheck-core,
// compiled to WebAssembly) when it has been built.
//
// The Rust functions are drop-in replacements: same arguments, same return
// shapes. So rather than editing every call site in app.js, this swaps the
// globals once the module has loaded. Where the .wasm has not been built,
// nothing is swapped and app.js keeps using its own JavaScript versions — the
// app works either way.
//
// Build the wasm with: ./scripts/build-wasm.sh

const MODULE_PATH = "./pkg/mapcheck_wasm.js";

// Functions the Rust core replaces one-for-one.
const DIRECT = [
    "bearDec",
    "fmtBear",
    "fmtBearDMS",
    "parseBearingInput",
    "parseCsvBearing",
    "calculateDeltas",
    "solveCurve",
    "getCourseAzimuth",
    "getCourseEndTangent",
    "azimuthToQuadBearingHemi",
    "parseMapFile",
    "parseCsvTextFile",
    "detectAndParseFile",
];

async function loadCore() {
    let module;
    try {
        module = await import(MODULE_PATH);
    } catch {
        return null; // not built yet — stay on the JavaScript implementation
    }
    await module.default();
    return module;
}

const core = await loadCore();

if (core) {
    for (const name of DIRECT) {
        if (typeof core[name] === "function" && typeof window[name] === "function") {
            window[name] = core[name];
        }
    }

    // The JS signature reads the lot designation from module scope; the Rust
    // one takes it as an argument.
    if (typeof core.generateMapFile === "function") {
        window.generateMapFile = (name, courses) =>
            core.generateMapFile(name, window.getLotDesignation?.() ?? "", courses);
    }

    // Available to app.js for the whole-traverse calls, which return the
    // courses, closure and area together rather than mutating in place.
    window.MapCheckCore = core;

    // init() may already have run against the JavaScript versions; redraw so
    // what is on screen came from the Rust core.
    window.recalculatePlat?.();

    console.info("MapCheck: calculations running on the Rust/WASM core");
}
