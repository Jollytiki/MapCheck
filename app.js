// Map Checker App Logic

// --- App State ---
let platName = "New Plat";
let lotDesignation = "Lot 4";
let isPrinting = false;
let activeTab = "line"; // line or curve
let solvedCurve = null; // Stores solved curve parameters: R, L, Delta, C
let courses = [];
let library = [];
let editingIndex = null;
let insertIndex = null; // Position a new course will be spliced into, or null to append
let lastErrorMsg = null;

// Viewport and Interaction State for Canvas
let zoom = 1.0;
let panX = 0;
let panY = 0;
let isDragging = false;
let startX = 0;
let startY = 0;
let hoveredCourseIndex = null;
let mouseCanvasX = 0;
let mouseCanvasY = 0;

// --- Elements ---
const canvas = document.getElementById("mapCanvas");
const ctx = canvas.getContext("2d");
const canvasContainer = document.getElementById("canvasContainer");

const platNameInput = document.getElementById("platNameInput");
const lotDesignationInput = document.getElementById("lotDesignationInput");
const platTitleDisplay = document.getElementById("platTitleDisplay");

// Starting Coordinates and Coordinate Table DOM
const startNorthingInput = document.getElementById("startNorthingInput");
const startEastingInput = document.getElementById("startEastingInput");
const coordinateTableSection = document.getElementById("coordinateTableSection");
const coordinateTableBody = document.getElementById("coordinateTableBody");
const printStartNorthing = document.getElementById("printStartNorthing");
const printStartEasting = document.getElementById("printStartEasting");

// Curve inputs DOM
const tabLineBtn = document.getElementById("tabLineBtn");
const tabCurveBtn = document.getElementById("tabCurveBtn");
const lineInputsPanel = document.getElementById("lineInputsPanel");
const curveInputsPanel = document.getElementById("curveInputsPanel");

const curveTurnInput = document.getElementById("curveTurnInput");
const curveRadiusInput = document.getElementById("curveRadiusInput");
const curveArcLengthInput = document.getElementById("curveArcLengthInput");
const curveDeltaInput = document.getElementById("curveDeltaInput");
const curveChordInput = document.getElementById("curveChordInput");
const curveSolvedInfo = document.getElementById("curveSolvedInfo");
const btnResetCurveParams = document.getElementById("btnResetCurveParams");

const bearingInputLabel = document.getElementById("bearingInputLabel");
const bearingInputHelper = document.getElementById("bearingInputHelper");
const platSubDisplay = document.getElementById("platSubDisplay");

const quadrantInput = document.getElementById("quadrantInput");
const hemisphereInput = document.getElementById("hemisphereInput");
const bearingInput = document.getElementById("bearingInput");
const distanceInput = document.getElementById("distanceInput");
const addCourseForm = document.getElementById("addCourseForm");
const btnAddCourse = document.getElementById("btnAddCourse");
const btnCancelEdit = document.getElementById("btnCancelEdit");
const editInsertActions = document.getElementById("editInsertActions");
const btnInsertBefore = document.getElementById("btnInsertBefore");
const btnInsertAfter = document.getElementById("btnInsertAfter");

const courseCountBadge = document.getElementById("courseCountBadge");
const courseTableBody = document.getElementById("courseTableBody");
const tableEmptyState = document.getElementById("tableEmptyState");

const precisionBox = document.getElementById("precisionBox");
const precisionVal = document.getElementById("precisionVal");
const closureErrorBox = document.getElementById("closureErrorBox");
const errorVal = document.getElementById("errorVal");
const totalDistVal = document.getElementById("totalDistVal");
const totalDeltaNVal = document.getElementById("totalDeltaNVal");
const totalDeltaEVal = document.getElementById("totalDeltaEVal");


const btnClearPlat = document.getElementById("btnClearPlat");
const btnSaveLibrary = document.getElementById("btnSaveLibrary");
const btnOpenLibrary = document.getElementById("btnOpenLibrary");
const libraryModal = document.getElementById("libraryModal");
const btnCloseLibrary = document.getElementById("btnCloseLibrary");
const btnCancelLibrary = document.getElementById("btnCancelLibrary");
const libraryList = document.getElementById("libraryList");

const btnZoomIn = document.getElementById("btnZoomIn");
const btnZoomOut = document.getElementById("btnZoomOut");
const btnZoomFit = document.getElementById("btnZoomFit");

const btnExportMap = document.getElementById("btnExportMap");
const btnImportMap = document.getElementById("btnImportMap");
const toastContainer = document.getElementById("toastContainer");



// --- Toast Notification ---
function escapeHTML(str) {
    if (!str) return "";
    return str
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

// --- Toast Notification ---
function showToast(message, type = "success") {
    const toast = document.createElement("div");
    toast.className = `toast ${type}`;
    
    const iconSpan = document.createElement("span");
    iconSpan.textContent = type === "success" ? "✓" : "✗";
    toast.appendChild(iconSpan);
    
    const msgSpan = document.createElement("span");
    msgSpan.textContent = message;
    toast.appendChild(msgSpan);
    
    toastContainer.appendChild(toast);
    
    if (type === "error") {
        lastErrorMsg = message;
        const btnShow = document.getElementById("btnShowLastError");
        if (btnShow) {
            btnShow.style.display = "inline-flex";
        }
    }
    
    const duration = type === "error" ? 10000 : 3000;
    setTimeout(() => {
        toast.style.opacity = '0';
        setTimeout(() => toast.remove(), 300);
    }, duration);
}

// --- Math & Bearing Calculations ---

// Convert DD.MMSS to Decimal Degrees
function bearDec(dms) {
    let sign = 1;
    if (dms < 0) {
        sign = -1;
        dms = Math.abs(dms);
    }
    const deg = Math.floor(dms);
    const frac = (dms - deg) * 100;
    const mm = Math.floor(frac + 0.0001);
    const ss = (frac - mm) * 100;
    return sign * (deg + mm / 60 + ss / 3600);
}

// Format decimal degrees as DD-MM-SS string
function fmtBear(dec) {
    if (dec === 0) return "00-00-00";
    let d = Math.floor(dec);
    let m = Math.floor((dec - d) * 60);
    let s = Math.round(((dec - d) * 60 - m) * 60);
    
    // Carry over rounding spills
    if (s >= 60) {
        s -= 60;
        m += 1;
    }
    if (m >= 60) {
        m -= 60;
        d += 1;
    }
    
    const dStr = String(d);
    const mStr = String(m).padStart(2, '0');
    const sStr = String(s).padStart(2, '0');
    return `${dStr}-${mStr}-${sStr}`;
}

// Format decimal degrees as DD.MMSS string
function fmtBearDMS(dec) {
    if (dec === 0) return "0.0000";
    let d = Math.floor(dec);
    let m = Math.floor((dec - d) * 60);
    let s = Math.round(((dec - d) * 60 - m) * 60);
    
    // Carry over rounding spills
    if (s >= 60) {
        s -= 60;
        m += 1;
    }
    if (m >= 60) {
        m -= 60;
        d += 1;
    }
    
    const dStr = String(d);
    const mStr = String(m).padStart(2, '0');
    const sStr = String(s).padStart(2, '0');
    return `${dStr}.${mStr}${sStr}`;
}

// Smart input parser for bearing angle (supports standard DD.MMSS, DD-MM-SS and shorthand QDD.MMSS)
function parseBearingInput(inputStr) {
    inputStr = inputStr.trim();
    if (!inputStr) return null;
    
    let qCode = null;
    let workingStr = inputStr;
    
    // Check if starts with a quadrant indicator (1-4) followed by 2 digits of degrees
    const qddMatch = inputStr.match(/^([1-4])(\d{2})(.*)$/);
    if (qddMatch) {
        qCode = parseInt(qddMatch[1], 10);
        const degStr = qddMatch[2];
        const restStr = qddMatch[3];
        workingStr = degStr + restStr; // Strip quadrant prefix for angle parsing
    }
    
    let bearingDec = null;
    // Check for separators: dash, space, colon, or degree symbol
    if (workingStr.includes('-') || workingStr.includes(' ') || workingStr.includes(':') || workingStr.includes('°') || workingStr.includes("'")) {
        const parts = workingStr.split(/[- :°'"]+/).filter(p => p.length > 0);
        const deg = parseFloat(parts[0]) || 0;
        const min = parseFloat(parts[1]) || 0;
        const sec = parseFloat(parts[2]) || 0;
        
        if (deg < 0 || deg > 90 || min < 0 || min >= 60 || sec < 0 || sec >= 60) {
            return null; // Invalid range
        }
        bearingDec = deg + min / 60 + sec / 3600;
    } else {
        const val = parseFloat(workingStr);
        if (isNaN(val) || val < 0 || val > 90) return null;
        bearingDec = bearDec(val);
    }
    
    if (qCode !== null) {
        let quad, hemi;
        if (qCode === 1) { quad = "N"; hemi = "E"; }
        else if (qCode === 2) { quad = "S"; hemi = "E"; }
        else if (qCode === 3) { quad = "S"; hemi = "W"; }
        else if (qCode === 4) { quad = "N"; hemi = "W"; }
        
        return {
            bearing: bearingDec,
            quad,
            hemi,
            hasPrefix: true
        };
    }
    
    return {
        bearing: bearingDec,
        hasPrefix: false
    };
}

// Calculate deltas for a single course
function calculateDeltas(quad, bearingDec, hemi, distance) {
    const rad = bearingDec * 0.017453292519943295; // PI/180
    const dn = distance * Math.cos(rad);
    const de = distance * Math.sin(rad);
    const deltaN = (quad === 'N') ? dn : -dn;
    const deltaE = (hemi === 'E') ? de : -de;
    return { deltaN, deltaE };
}

// Compute total closure calculations
function recalculatePlat() {
    let totalDist = 0;
    let totalN = 0;
    let totalE = 0;
    
    courses = courses.map(c => {
        if (c.isInvalid) {
            return {
                ...c,
                deltaN: 0,
                deltaE: 0
            };
        }
        const distForDelta = c.type === "curve" ? c.chordLength : c.distance;
        const { deltaN, deltaE } = calculateDeltas(c.quad, c.bearing, c.hemi, distForDelta);
        totalDist += c.distance; // distance contains arc length for curves
        totalN += deltaN;
        totalE += deltaE;
        return {
            ...c,
            deltaN,
            deltaE
        };
    });
    
    const error = Math.sqrt(totalN * totalN + totalE * totalE);
    const precision = error > 0.00001 ? Math.round(totalDist / error) : Infinity;
    
    // Update UI Stats
    totalDistVal.textContent = totalDist.toFixed(3);
    totalDeltaNVal.textContent = totalN.toFixed(4);
    totalDeltaEVal.textContent = totalE.toFixed(4);
    errorVal.textContent = error.toFixed(4);
    
    if (courses.length === 0) {
        precisionVal.textContent = "1 : 0";
        precisionBox.className = "stat-box highlight";
        closureErrorBox.className = "stat-box";
    } else if (error < 0.005) {
        precisionVal.textContent = "PERFECT CLOSURE";
        precisionBox.className = "stat-box highlight success";
        closureErrorBox.className = "stat-box success";
    } else {
        precisionVal.textContent = `1 : ${precision.toLocaleString()}`;
        precisionBox.className = "stat-box highlight";
        if (precision < 2000) {
            closureErrorBox.className = "stat-box warning";
        } else {
            closureErrorBox.className = "stat-box success";
        }
    }
    
    platSubDisplay.textContent = `${lotDesignation ? lotDesignation + ' • ' : ''}${courses.length} course${courses.length === 1 ? '' : 's'}`;
    courseCountBadge.textContent = `${courses.length} Course${courses.length === 1 ? '' : 's'}`;
    courseCountBadge.className = courses.length > 0 ? "badge badge-e" : "badge badge-n";
    
    const areaBox = document.getElementById("areaDisplayBox");
    if (areaBox) areaBox.style.display = "none";
    
    renderTable();
    renderCoordinateTable();
    draw();
}

// Compute the area inside the boundary (using Shoelace formula + curve segment adjustments)
function calculateTraverseArea() {
    if (courses.length < 3 || courses.some(c => c.isInvalid)) {
        return null;
    }
    
    const pts = getPlatCoordinates();
    
    // Calculate signed shoelace area of the chord polygon
    let signedArea = 0;
    const n = pts.length - 1; // pts has courses.length + 1 elements
    for (let i = 0; i < n; i++) {
        const p1 = pts[i];
        const p2 = pts[i + 1];
        signedArea += (p1.x * p2.y - p2.x * p1.y);
    }
    
    signedArea = signedArea / 2;
    
    // Adjust for circular curve segments
    courses.forEach(c => {
        if (c.type === "curve" && c.radius > 0 && c.deltaAngle > 0) {
            const rad = c.radius;
            const deltaRad = c.deltaAngle * Math.PI / 180;
            // Area of segment = 0.5 * R^2 * (theta - sin(theta))
            const segArea = 0.5 * rad * rad * (deltaRad - Math.sin(deltaRad));
            const turnSign = c.turn === "L" ? 1 : -1;
            signedArea += turnSign * segArea;
        }
    });
    
    return Math.abs(signedArea);
}

// --- Canvas Plotting Engine ---

function getPlatCoordinates() {
    let x = 0;
    let y = 0;
    const pts = [{ x: 0, y: 0 }];
    
    courses.forEach(c => {
        x += c.deltaE;
        y += c.deltaN;
        pts.push({ x, y });
    });
    
    return pts;
}

function getAbsolutePointCoordinates() {
    let startN = parseFloat(startNorthingInput.value);
    let startE = parseFloat(startEastingInput.value);
    
    if (isNaN(startN)) startN = 0.0;
    if (isNaN(startE)) startE = 0.0;
    
    let n = startN;
    let e = startE;
    
    const pts = [{ name: "POB", n, e }];
    
    courses.forEach((c, index) => {
        if (c.isInvalid) {
            pts.push({ name: `${index + 1}`, n: NaN, e: NaN });
        } else {
            n += c.deltaN;
            e += c.deltaE;
            const isClosure = (index === courses.length - 1);
            pts.push({ name: isClosure ? "Closure" : `${index + 1}`, n, e });
        }
    });
    
    return pts;
}

// Auto-scale to fit traverse on canvas
function zoomToFit() {
    if (courses.length === 0) {
        zoom = 1.0;
        panX = canvas.width / 2;
        panY = canvas.height / 2;
        return;
    }
    
    const pts = getPlatCoordinates();
    const xs = pts.map(p => p.x);
    const ys = pts.map(p => p.y);
    
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minY = Math.min(...ys);
    const maxY = Math.max(...ys);
    
    const w = maxX - minX;
    const h = maxY - minY;
    
    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    
    const padding = 60;
    const fitW = canvas.width - padding * 2;
    const fitH = canvas.height - padding * 2;
    
    let calcZoom = 1.0;
    if (w > 0 || h > 0) {
        const zoomX = w > 0 ? fitW / w : Infinity;
        const zoomY = h > 0 ? fitH / h : Infinity;
        calcZoom = Math.min(zoomX, zoomY);
    }
    
    // Clamp zoom
    zoom = Math.min(Math.max(calcZoom, 0.01), 100);
    
    panX = canvas.width / 2 - cx * zoom;
    panY = canvas.height / 2 + cy * zoom;
}

// Convert survey coordinates to canvas pixel space
function toCanvasCoords(sx, sy) {
    return {
        x: panX + sx * zoom,
        y: panY - sy * zoom // Invert Y
    };
}

// Convert canvas pixel space to survey coordinates
function toSurveyCoords(cx, cy) {
    return {
        x: (cx - panX) / zoom,
        y: (panY - cy) / zoom
    };
}

// Draw the survey traverse
function draw() {
    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    // Draw Subtle Grid relative to zoom and pan
    const gridSize = 50;
    const scaledGridSize = gridSize * zoom;
    
    // Adjust grid frequency based on zoom to prevent clutter or emptiness
    let activeGridSize = scaledGridSize;
    if (activeGridSize < 15) activeGridSize *= 10;
    if (activeGridSize > 150) activeGridSize /= 5;
    
    const startGridX = panX % activeGridSize;
    const startGridY = panY % activeGridSize;
    
    ctx.strokeStyle = isPrinting ? '#e2e8f0' : '#161f30';
    ctx.lineWidth = 1;
    
    // Vertical grid lines
    for (let x = startGridX; x < canvas.width; x += activeGridSize) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, canvas.height);
        ctx.stroke();
    }
    // Horizontal grid lines
    for (let y = startGridY; y < canvas.height; y += activeGridSize) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(canvas.width, y);
        ctx.stroke();
    }
    
    if (courses.length === 0) {
        if (!isPrinting) {
            // Draw central target when empty
            ctx.strokeStyle = '#22304d';
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            ctx.arc(canvas.width / 2, canvas.height / 2, 20, 0, Math.PI * 2);
            ctx.stroke();
            
            ctx.beginPath();
            ctx.moveTo(canvas.width / 2 - 30, canvas.height / 2);
            ctx.lineTo(canvas.width / 2 + 30, canvas.height / 2);
            ctx.moveTo(canvas.width / 2, canvas.height / 2 - 30);
            ctx.lineTo(canvas.width / 2, canvas.height / 2 + 30);
            ctx.stroke();
        }
        return;
    }
    
    const pts = getPlatCoordinates();
    const pixelPts = pts.map(p => toCanvasCoords(p.x, p.y));
    
    // 1. Draw Course Lines
    for (let i = 0; i < courses.length; i++) {
        const c = courses[i];
        if (c.isInvalid) continue;
        const s1 = pts[i];
        const s2 = pts[i + 1];
        const p1 = pixelPts[i];
        const p2 = pixelPts[i + 1];
        const isHovered = hoveredCourseIndex === i;
        
        ctx.beginPath();
        
        let mx, my, angle;
        
        if (c.type === "curve") {
            const dx = s2.x - s1.x;
            const dy = s2.y - s1.y;
            const chordLen = Math.sqrt(dx * dx + dy * dy);
            
            if (chordLen > 0.001) {
                const ux = dx / chordLen;
                const uy = dy / chordLen;
                
                // Perpendicular vector rotation
                const nx = c.turn === "L" ? -uy : uy;
                const ny = c.turn === "L" ? ux : -ux;
                
                const rad = c.radius;
                const deltaHalf = (c.deltaAngle * Math.PI / 180) / 2;
                const d = rad * Math.cos(deltaHalf);
                
                // Circle Center
                const ox = s1.x + (chordLen / 2) * ux + d * nx;
                const oy = s1.y + (chordLen / 2) * uy + d * ny;
                
                // Draw arc segments
                const startPx = toCanvasCoords(s1.x, s1.y);
                ctx.moveTo(startPx.x, startPx.y);
                
                const steps = 30;
                for (let step = 1; step <= steps; step++) {
                    const t = step / steps;
                    const theta = -deltaHalf + t * (2 * deltaHalf);
                    
                    const sx = ox - rad * Math.cos(theta) * nx + rad * Math.sin(theta) * ux;
                    const sy = oy - rad * Math.cos(theta) * ny + rad * Math.sin(theta) * uy;
                    
                    const px = toCanvasCoords(sx, sy);
                    ctx.lineTo(px.x, px.y);
                }
                
                // Midpoint of arc (at theta = 0)
                const arcMidX = ox - rad * nx;
                const arcMidY = oy - rad * ny;
                const arcMidPx = toCanvasCoords(arcMidX, arcMidY);
                mx = arcMidPx.x;
                my = arcMidPx.y;
                
                angle = Math.atan2(p2.y - p1.y, p2.x - p1.x);
            } else {
                ctx.moveTo(p1.x, p1.y);
                ctx.lineTo(p2.x, p2.y);
                mx = (p1.x + p2.x) / 2;
                my = (p1.y + p2.y) / 2;
                angle = Math.atan2(p2.y - p1.y, p2.x - p1.x);
            }
        } else {
            ctx.moveTo(p1.x, p1.y);
            ctx.lineTo(p2.x, p2.y);
            mx = (p1.x + p2.x) / 2;
            my = (p1.y + p2.y) / 2;
            angle = Math.atan2(p2.y - p1.y, p2.x - p1.x);
        }
        
        if (isPrinting) {
            ctx.strokeStyle = '#0f172a'; // Deep slate line for crisp printing
            ctx.lineWidth = 2.0;
        } else if (isHovered) {
            ctx.strokeStyle = '#f59e0b'; // Gold accent for hover
            ctx.lineWidth = 4;
            ctx.shadowColor = '#f59e0b';
            ctx.shadowBlur = 6;
        } else {
            ctx.strokeStyle = '#06b6d4'; // Cyan neon for normal lines
            ctx.lineWidth = 2.5;
            ctx.shadowColor = '#06b6d4';
            ctx.shadowBlur = 3;
        }
        ctx.stroke();
        ctx.shadowBlur = 0; // Reset
        
        // Draw direction arrow at midpoint
        ctx.beginPath();
        ctx.strokeStyle = isPrinting ? '#0f172a' : (isHovered ? '#f59e0b' : '#06b6d4');
        ctx.lineWidth = 1.5;
        ctx.moveTo(mx, my);
        ctx.lineTo(mx - 8 * Math.cos(angle - Math.PI / 6), my - 8 * Math.sin(angle - Math.PI / 6));
        ctx.moveTo(mx, my);
        ctx.lineTo(mx - 8 * Math.cos(angle + Math.PI / 6), my - 8 * Math.sin(angle + Math.PI / 6));
        ctx.stroke();
        
        // Calculate length of segment in screen pixels
        const pxDx = p2.x - p1.x;
        const pxDy = p2.y - p1.y;
        const pxLen = Math.sqrt(pxDx * pxDx + pxDy * pxDy);
        
        // Find rotation angle for text (parallel to line)
        let textAngle = angle;
        // Keep text right-side up for cartographic readability
        if (textAngle > Math.PI / 2 || textAngle < -Math.PI / 2) {
            textAngle += Math.PI;
        }
        
        if (pxLen > 65 || isHovered) {
            ctx.save();
            ctx.translate(mx, my);
            ctx.rotate(textAngle);
            
            ctx.font = '8px "JetBrains Mono", monospace';
            ctx.fillStyle = isPrinting ? '#0f172a' : (isHovered ? '#fde047' : '#94a3b8');
            ctx.textAlign = 'center';
            
            if (c.type === "curve") {
                // Bearing line (Chord)
                ctx.textBaseline = 'bottom';
                ctx.fillText(`#${i + 1} Ch:${c.quad} ${fmtBear(c.bearing)} ${c.hemi}`, 0, -3);
                
                // Distance line (Arc + Radius)
                ctx.textBaseline = 'top';
                ctx.fillText(`Arc:${c.distance.toFixed(2)} R:${c.radius.toFixed(2)}`, 0, 3);
            } else {
                // Bearing line
                ctx.textBaseline = 'bottom';
                ctx.fillText(`#${i + 1} ${c.quad} ${fmtBear(c.bearing)} ${c.hemi}`, 0, -3);
                
                // Distance line
                ctx.textBaseline = 'top';
                ctx.fillText(`${c.distance.toFixed(2)}`, 0, 3);
            }
            
            ctx.restore();
        } else {
            // Draw just the course index if line is too short on screen
            const perpAngle = angle + Math.PI / 2;
            const offset = 10;
            const lx = mx + offset * Math.cos(perpAngle);
            const ly = my + offset * Math.sin(perpAngle);
            
            ctx.fillStyle = isPrinting ? '#0f172a' : (isHovered ? '#fde047' : '#94a3b8');
            ctx.font = 'bold 9px "JetBrains Mono", monospace';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(`${i + 1}`, lx, ly);
        }
    }
    
    // 2. Draw Closure Error Dotted Line (if not closed)
    const startPt = pixelPts[0];
    const endPt = pixelPts[pixelPts.length - 1];
    const dx = endPt.x - startPt.x;
    const dy = endPt.y - startPt.y;
    const distPx = Math.sqrt(dx * dx + dy * dy);
    
    if (distPx > 2) {
        ctx.beginPath();
        ctx.setLineDash([4, 4]);
        ctx.moveTo(endPt.x, endPt.y);
        ctx.lineTo(startPt.x, startPt.y);
        ctx.strokeStyle = isPrinting ? '#b91c1c' : '#f43f5e'; // Dark red for print
        ctx.lineWidth = 1.5;
        ctx.stroke();
        ctx.setLineDash([]); // Reset
        
        // Error label
        const errValReal = Math.sqrt(
            Math.pow(pts[pts.length - 1].x - pts[0].x, 2) + 
            Math.pow(pts[pts.length - 1].y - pts[0].y, 2)
        );
        ctx.fillStyle = isPrinting ? '#b91c1c' : '#fda4af';
        ctx.font = '9px "JetBrains Mono", monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'bottom';
        ctx.fillText(`Error: ${errValReal.toFixed(2)}`, (startPt.x + endPt.x)/2, (startPt.y + endPt.y)/2 - 5);
    }
    
    // 3. Draw Vertex Markers
    pixelPts.forEach((p, index) => {
        ctx.beginPath();
        if (index === 0) {
            // Start node (emerald green)
            ctx.arc(p.x, p.y, 6, 0, Math.PI * 2);
            ctx.fillStyle = isPrinting ? '#047857' : '#10b981';
            ctx.fill();
            ctx.strokeStyle = isPrinting ? '#0f172a' : '#ffffff';
            ctx.lineWidth = 1.5;
            ctx.stroke();
        } else if (index === pixelPts.length - 1) {
            // End node (rose red)
            ctx.arc(p.x, p.y, 5, 0, Math.PI * 2);
            ctx.fillStyle = isPrinting ? '#b91c1c' : '#f43f5e';
            ctx.fill();
            ctx.strokeStyle = isPrinting ? '#0f172a' : '#ffffff';
            ctx.lineWidth = 1.5;
            ctx.stroke();
        } else {
            // Intermediate node (grey)
            ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
            ctx.fillStyle = isPrinting ? '#475569' : '#475569';
            ctx.fill();
            ctx.strokeStyle = isPrinting ? '#0f172a' : '#94a3b8';
            ctx.lineWidth = 1;
            ctx.stroke();
        }
    });
    
    // 4. Draw Hover Tooltip on Canvas (Skip on print)
    if (hoveredCourseIndex !== null && hoveredCourseIndex < courses.length && !isPrinting) {
        const c = courses[hoveredCourseIndex];
        const formattedTxt = `#${hoveredCourseIndex + 1}: ${c.quad} ${fmtBear(c.bearing)} ${c.hemi} - ${c.distance.toFixed(2)}`;
        
        ctx.font = '11px "Inter", sans-serif';
        const textWidth = ctx.measureText(formattedTxt).width;
        
        const tooltipX = Math.min(mouseCanvasX + 15, canvas.width - textWidth - 25);
        const tooltipY = Math.min(mouseCanvasY + 15, canvas.height - 30);
        
        // Draw tooltip background
        ctx.fillStyle = 'rgba(22, 30, 49, 0.95)';
        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.roundRect(tooltipX, tooltipY, textWidth + 16, 22, 4);
        ctx.fill();
        ctx.stroke();
        
        // Draw tooltip text
        ctx.fillStyle = '#f8fafc';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'middle';
        ctx.fillText(formattedTxt, tooltipX + 8, tooltipY + 11);
    }
}

// Find course line segment closest to mouse cursor
function checkHoveredCourse(mx, my) {
    if (courses.length === 0) return null;
    
    const pts = getPlatCoordinates();
    const pixelPts = pts.map(p => toCanvasCoords(p.x, p.y));
    const hoverThreshold = 8; // Max distance in pixels
    
    for (let i = 0; i < courses.length; i++) {
        const p1 = pixelPts[i];
        const p2 = pixelPts[i + 1];
        
        // Compute distance from cursor (mx, my) to line segment p1-p2
        const dx = p2.x - p1.x;
        const dy = p2.y - p1.y;
        const segmentLenSq = dx * dx + dy * dy;
        
        if (segmentLenSq === 0) continue;
        
        // Projection factor clamped between 0 and 1
        let t = ((mx - p1.x) * dx + (my - p1.y) * dy) / segmentLenSq;
        t = Math.max(0, Math.min(1, t));
        
        const projX = p1.x + t * dx;
        const projY = p1.y + t * dy;
        
        const distSq = (mx - projX) * (mx - projX) + (my - projY) * (my - projY);
        
        if (distSq < hoverThreshold * hoverThreshold) {
            return i;
        }
    }
    return null;
}

// --- Dynamic Table Renderer ---

function renderTable() {
    courseTableBody.innerHTML = "";
    
    if (courses.length === 0) {
        tableEmptyState.style.display = "flex";
        return;
    } else {
        tableEmptyState.style.display = "none";
    }
    
    const insertMarkerRow = () => {
        const tr = document.createElement("tr");
        tr.className = "insert-marker-row";
        tr.innerHTML = `<td colspan="8">New course will be inserted here as #${insertIndex + 1}</td>`;
        courseTableBody.appendChild(tr);
    };
    
    courses.forEach((c, index) => {
        if (insertIndex === index) insertMarkerRow();
        
        const tr = document.createElement("tr");
        tr.dataset.index = index;
        if (c.isInvalid) {
            tr.style.backgroundColor = "rgba(239, 68, 68, 0.15)";
            tr.style.borderLeft = "4px solid #ef4444";
        } else if (editingIndex === index) {
            tr.style.backgroundColor = "rgba(16, 185, 129, 0.15)";
            tr.style.borderLeft = "4px solid #10b981";
        } else if (hoveredCourseIndex === index) {
            tr.style.backgroundColor = "rgba(245, 158, 11, 0.15)";
        }
        
        let quadrantCell, bearingCell, hemiCell, distanceCell;
        let dnCell, deCell;
        
        if (c.isInvalid) {
            quadrantCell = `<span class="badge badge-danger" style="background-color:#ef4444; color:#ffffff; border-color:#ef4444;">Invalid</span>`;
            bearingCell = `<div style="color:#f87171; font-weight:bold; font-size:0.75rem; white-space:normal; word-break:break-word;">${escapeHTML(c.errorMsg)}</div>`;
            hemiCell = `<span class="badge badge-n">-</span>`;
            distanceCell = `-`;
            dnCell = `-`;
            deCell = `-`;
        } else {
            const escapedTurn = escapeHTML(c.turn);
            const escapedQuad = escapeHTML(c.quad);
            const escapedHemi = escapeHTML(c.hemi);
            
            quadrantCell = c.type === "curve" 
                ? `<span class="badge badge-s" style="border-color:#a78bfa !important; color:#c084fc !important;">Curve ${escapedTurn === 'L' ? 'Left' : 'Right'}</span>`
                : `<span class="badge badge-${escapedQuad.toLowerCase()}">${escapedQuad === 'N' ? 'North' : 'South'}</span>`;
                
            bearingCell = c.type === "curve"
                ? `<div>Ch: ${escapedQuad} ${fmtBear(c.bearing)} ${escapedHemi}</div>
                   <div style="font-size:0.7rem; color:var(--text-muted)">Rad: ${c.radius.toFixed(1)} • Δ: ${fmtBear(c.deltaAngle)}</div>`
                : `${fmtBear(c.bearing)}`;
                
            hemiCell = `<span class="badge badge-${escapedHemi.toLowerCase()}">${escapedHemi === 'E' ? 'East' : 'West'}</span>`;
                
            distanceCell = c.type === "curve"
                ? `<div>Arc: ${c.distance.toFixed(2)}</div>
                   <div style="font-size:0.7rem; color:var(--text-muted)">Chord L: ${c.chordLength.toFixed(1)}</div>`
                : `${c.distance.toFixed(2)}`;
                
            dnCell = c.deltaN.toFixed(3);
            deCell = c.deltaE.toFixed(3);
        }

        tr.innerHTML = `
            <td><div class="table-print-cell">${index + 1}</div></td>
            <td><div class="table-print-cell">${quadrantCell}</div></td>
            <td><div class="table-print-cell">${bearingCell}</div></td>
            <td><div class="table-print-cell">${hemiCell}</div></td>
            <td><div class="table-print-cell">${distanceCell}</div></td>
            <td style="${!c.isInvalid && c.deltaN >= 0 ? 'color:#a7f3d0' : (!c.isInvalid ? 'color:#fecdd3' : '')}"><div class="table-print-cell">${dnCell}</div></td>
            <td style="${!c.isInvalid && c.deltaE >= 0 ? 'color:#a7f3d0' : (!c.isInvalid ? 'color:#fecdd3' : '')}"><div class="table-print-cell">${deCell}</div></td>
            <td style="text-align: center;">
                <div class="table-row-actions">
                    <button class="action-icon-btn" onclick="editCourse(${index})" title="Edit course">✏️</button>
                    <button class="action-icon-btn delete" onclick="deleteCourse(${index})" title="Delete course">🗑️</button>
                </div>
            </td>
        `;
        
        // Hover effects synched between table rows and canvas
        tr.addEventListener("mouseenter", () => {
            hoveredCourseIndex = index;
            if (editingIndex !== index) {
                tr.style.backgroundColor = "rgba(245, 158, 11, 0.15)";
            }
            draw();
        });
        
        tr.addEventListener("mouseleave", () => {
            hoveredCourseIndex = null;
            tr.style.backgroundColor = (editingIndex === index) ? "rgba(16, 185, 129, 0.15)" : "";
            draw();
        });
        
        courseTableBody.appendChild(tr);
    });
    
    if (insertIndex !== null && insertIndex >= courses.length) insertMarkerRow();
}

function renderCoordinateTable() {
    if (courses.length === 0) {
        coordinateTableSection.classList.remove("print-visible");
        return;
    }
    
    const startNVal = startNorthingInput.value.trim();
    const startEVal = startEastingInput.value.trim();
    const hasStartCoords = (startNVal !== "" || startEVal !== "");
    
    if (!hasStartCoords) {
        coordinateTableSection.classList.remove("print-visible");
        return;
    }
    
    coordinateTableSection.classList.add("print-visible");
    coordinateTableBody.innerHTML = "";
    
    const pts = getAbsolutePointCoordinates();
    pts.forEach(p => {
        const tr = document.createElement("tr");
        const nText = isNaN(p.n) ? "-" : p.n.toFixed(3);
        const eText = isNaN(p.e) ? "-" : p.e.toFixed(3);
        tr.innerHTML = `
            <td><div class="table-print-cell">${escapeHTML(p.name)}</div></td>
            <td><div class="table-print-cell">${nText}</div></td>
            <td><div class="table-print-cell">${eText}</div></td>
        `;
        coordinateTableBody.appendChild(tr);
    });
}

function cancelEdit() {
    editingIndex = null;
    insertIndex = null;
    btnAddCourse.textContent = "Add Course";
    btnCancelEdit.style.display = "none";
    editInsertActions.style.display = "none";
    
    // Clear form fields
    bearingInput.value = "";
    distanceInput.value = "";
    btnResetCurveParams.click();
    bearingInput.focus();
    recalculatePlat(); // Redraws table to clear highlights
}

// Switch the form from editing a course to inserting a new one at `position`
function startInsert(position) {
    editingIndex = null;
    insertIndex = Math.max(0, Math.min(position, courses.length));
    
    btnAddCourse.textContent = `Insert Course as #${insertIndex + 1}`;
    btnCancelEdit.style.display = "block";
    editInsertActions.style.display = "none";
    
    // Clear form fields so the user starts fresh
    bearingInput.value = "";
    distanceInput.value = "";
    btnResetCurveParams.click();
    bearingInput.focus();
    recalculatePlat(); // Redraws table with the insert marker
    showToast(`Inserting new course as #${insertIndex + 1}`, "info");
}

// Global actions for inline items (exposed to window)
window.deleteCourse = function(index) {
    courses.splice(index, 1);
    
    // Adjust editingIndex if in edit mode
    if (editingIndex === index) {
        cancelEdit();
    } else if (editingIndex !== null && index < editingIndex) {
        editingIndex--;
    }
    
    // Keep the insert position pointing at the same neighbours
    if (insertIndex !== null && index < insertIndex) {
        insertIndex--;
        btnAddCourse.textContent = `Insert Course as #${insertIndex + 1}`;
    }
    
    recalculatePlat();
    showToast("Course deleted");
};

window.editCourse = function(index) {
    const oldEditingIndex = editingIndex;
    editingIndex = index;
    insertIndex = null;
    
    // Redraw table to shift highlights
    if (oldEditingIndex !== null) {
        recalculatePlat();
    } else {
        renderTable();
    }
    
    const c = courses[index];
    
    // Update button states
    btnAddCourse.textContent = "Update Course";
    btnCancelEdit.style.display = "block";
    editInsertActions.style.display = "flex";
    
    if (c.isInvalid) {
        const pd = c.partialData || {};
        const isCurve = c.partialType === "curve";
        
        quadrantInput.value = pd.quad && (pd.quad === "N" || pd.quad === "S") ? pd.quad : "N";
        hemisphereInput.value = pd.hemi && (pd.hemi === "E" || pd.hemi === "W") ? pd.hemi : "E";
        bearingInput.value = pd.rawBearing || "";
        
        if (isCurve) {
            switchTab("curve");
            curveTurnInput.value = pd.turn && (pd.turn === "L" || pd.turn === "R") ? pd.turn : "L";
            curveRadiusInput.value = pd.radius !== null && !isNaN(pd.radius) ? pd.radius : "";
            curveArcLengthInput.value = pd.arcLength !== null && !isNaN(pd.arcLength) ? pd.arcLength : "";
            curveDeltaInput.value = pd.deltaAngle !== null && !isNaN(pd.deltaAngle) ? fmtBearDMS(pd.deltaAngle) : "";
            curveChordInput.value = pd.chordLength !== null && !isNaN(pd.chordLength) ? pd.chordLength : "";
            updateCurveSolver();
        } else {
            switchTab("line");
            distanceInput.value = pd.distance !== null && !isNaN(pd.distance) ? pd.distance : "";
        }
        
        bearingInput.focus();
        showToast(`Loaded partial course #${index + 1} data for correction: ${c.errorMsg}`, "info");
        return;
    }
    
    quadrantInput.value = c.quad;
    hemisphereInput.value = c.hemi;
    bearingInput.value = c.rawBearing || fmtBearDMS(c.bearing);
    
    if (c.type === "curve") {
        switchTab("curve");
        curveTurnInput.value = c.turn;
        curveRadiusInput.value = c.radius;
        curveArcLengthInput.value = c.distance; // distance contains arc length
        curveDeltaInput.value = fmtBearDMS(c.deltaAngle);
        curveChordInput.value = c.chordLength;
        updateCurveSolver(); // Trigger locking display
    } else {
        switchTab("line");
        distanceInput.value = c.distance;
    }
    
    bearingInput.focus();
    showToast("Loaded course into editor for modification", "info");
};

// --- Library Management (LocalStorage) ---

function initLibrary() {
    try {
        const data = localStorage.getItem("mapcheck_library");
        library = data ? JSON.parse(data) : [];
    } catch (e) {
        console.error("Failed to load local storage library", e);
        library = [];
    }
}

function saveToLibrary() {
    if (courses.length === 0) {
        showToast("No courses to save", "error");
        return;
    }
    
    const name = platNameInput.value.trim() || "Untitled Plat";
    const lot = lotDesignationInput.value.trim();
    
    const newItem = {
        id: 'plat_' + Date.now(),
        name: name,
        lotDesignation: lot,
        startNorthing: startNorthingInput.value.trim(),
        startEasting: startEastingInput.value.trim(),
        courses: JSON.parse(JSON.stringify(courses)),
        updatedAt: new Date().toLocaleDateString()
    };
    
    // Replace if same name, or add new
    const existingIndex = library.findIndex(l => l.name.toLowerCase() === name.toLowerCase());
    if (existingIndex > -1) {
        library[existingIndex] = newItem;
        showToast(`Updated plat: "${name}"`);
    } else {
        library.push(newItem);
        showToast(`Saved new plat: "${name}"`);
    }
    
    localStorage.setItem("mapcheck_library", JSON.stringify(library));
}

function loadLibraryItem(id) {
    const item = library.find(l => l.id === id);
    if (!item) return;
    
    if (courses.length > 0) {
        if (!confirm(`Are you sure you want to load "${item.name}"? Your current traverse will be overwritten.`)) {
            return;
        }
    }
    
    platName = item.name;
    platNameInput.value = item.name;
    platTitleDisplay.textContent = item.name;
    
    lotDesignation = item.lotDesignation || "";
    lotDesignationInput.value = lotDesignation;
    
    startNorthingInput.value = item.startNorthing || "";
    startEastingInput.value = item.startEasting || "";
    
    courses = JSON.parse(JSON.stringify(item.courses));
    recalculatePlat();
    zoomToFit();
    
    libraryModal.classList.remove("active");
    showToast(`Loaded plat: "${item.name}"`);
}

function deleteLibraryItem(id, event) {
    event.stopPropagation(); // Stop trigger load
    const item = library.find(l => l.id === id);
    if (!item) return;
    
    if (confirm(`Are you sure you want to delete "${item.name}" from your library?`)) {
        library = library.filter(l => l.id !== id);
        localStorage.setItem("mapcheck_library", JSON.stringify(library));
        renderLibraryList();
        showToast("Plat deleted from library");
    }
}

function renderLibraryList() {
    libraryList.innerHTML = "";
    if (library.length === 0) {
        libraryList.innerHTML = `
            <div style="color: var(--text-muted); text-align: center; padding: 2rem;">
                No saved plats in library yet.
            </div>
        `;
        return;
    }
    
    library.forEach(item => {
        const div = document.createElement("div");
        div.className = "library-item";
        div.onclick = () => loadLibraryItem(item.id);
        
        const escapedName = escapeHTML(item.name);
        const escapedLot = escapeHTML(item.lotDesignation);
        const subtext = escapedLot ? `${escapedLot} • ` : "";
        div.innerHTML = `
            <div class="library-item-info">
                <h4>${escapedName}</h4>
                <p>${subtext}${item.courses.length} courses • Saved: ${item.updatedAt}</p>
            </div>
            <button class="action-icon-btn delete" title="Delete from library">🗑️</button>
        `;
        
        const deleteBtn = div.querySelector(".action-icon-btn.delete");
        if (deleteBtn) {
            deleteBtn.onclick = (event) => deleteLibraryItem(item.id, event);
        }
        
        libraryList.appendChild(div);
    });
}

// --- Import & Export Features ---

// Converts course properties to a 0-360 degree azimuth
function getCourseAzimuth(course) {
    const b = course.bearing;
    const q = course.quad;
    const h = course.hemi;
    if (q === 'N' && h === 'E') return b;
    if (q === 'S' && h === 'E') return 180 - b;
    if (q === 'S' && h === 'W') return 180 + b;
    if (q === 'N' && h === 'W') return 360 - b;
    return 0;
}

// Calculates tangent direction at exit of course
function getCourseEndTangent(course) {
    const chordAz = getCourseAzimuth(course);
    if (course.type === 'line') {
        return chordAz;
    } else {
        const halfDelta = course.deltaAngle / 2;
        if (course.turn === 'R') {
            return (chordAz + halfDelta) % 360;
        } else {
            return (chordAz - halfDelta + 360) % 360;
        }
    }
}

// Converts a 0-360 degree azimuth back to quad, bearing, and hemi
function azimuthToQuadBearingHemi(az) {
    az = (az % 360 + 360) % 360;
    let quad, hemi, bearing;
    if (az >= 0 && az < 90) {
        quad = 'N'; hemi = 'E'; bearing = az;
    } else if (az >= 90 && az < 180) {
        quad = 'S'; hemi = 'E'; bearing = 180 - az;
    } else if (az >= 180 && az < 270) {
        quad = 'S'; hemi = 'W'; bearing = az - 180;
    } else {
        quad = 'N'; hemi = 'W'; bearing = 360 - az;
    }
    return { quad, bearing, hemi };
}

// Parses explicit (N 35.0020 E) or shorthand (135.0020) CSV bearings
function parseCsvBearing(bearingStr) {
    bearingStr = bearingStr.trim().toUpperCase();
    
    const explicitMatch = bearingStr.match(/^([NS])\s*(.*?)\s*([EW])$/);
    if (explicitMatch) {
        const quad = explicitMatch[1];
        const hemi = explicitMatch[3];
        const angleStr = explicitMatch[2];
        const parsed = parseBearingInput(angleStr);
        if (parsed !== null) {
            return {
                bearing: parsed.bearing,
                quad,
                hemi
            };
        }
    }
    
    const parsed = parseBearingInput(bearingStr);
    if (parsed !== null && parsed.hasPrefix) {
        return {
            bearing: parsed.bearing,
            quad: parsed.quad,
            hemi: parsed.hemi
        };
    }
    
    return null;
}

// Parser for the custom comma-separated text format
function parseCsvTextFile(content, fileName) {
    const lines = content.split(/\r?\n/).map(l => l.trim()).filter(l => l.length > 0);
    const loadedCourses = [];
    let errorCount = 0;
    
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.startsWith('#') || line.startsWith('//')) {
            continue;
        }
        
        const parts = line.split(',').map(p => p.trim());
        if (parts.length < 3) {
            loadedCourses.push({
                type: "invalid",
                isInvalid: true,
                errorMsg: `Line ${i + 1}: Invalid format. Expected at least 3 values.`,
                partialType: "line",
                partialData: {}
            });
            errorCount++;
            continue;
        }
        
        const courseNum = parts[0];
        
        let isCurve = false;
        let bearingStr = null;
        let paramStartIndex = -1;
        
        if (parts[1].toUpperCase() === 'C') {
            isCurve = true;
            paramStartIndex = 2;
        } else if (parts[2].toUpperCase() === 'C') {
            isCurve = true;
            // Field 2 may carry the documented `CB` prefix ("CBN 45-00-00 E").
            bearingStr = parts[1].trim().replace(/^CB\s*/i, '');
            paramStartIndex = 3;
        }
        
        try {
            if (isCurve) {
                let radius = null;
                let chordLength = null;
                let arcLength = null;
                let deltaAngle = null;
                let turn = null;
                
                // Pre-scan to check if explicit chord length prefixes (CL or CD) are present
                const hasChordLengthParam = parts.slice(paramStartIndex).some(p => {
                    const u = p.trim().toUpperCase();
                    return u.startsWith("CL") || u.startsWith("CD");
                });
                
                for (let j = paramStartIndex; j < parts.length; j++) {
                    const param = parts[j].trim().toUpperCase();
                    if (!param) continue;
                    
                    if (param.startsWith("CB")) {
                        bearingStr = parts[j].trim().substring(2);
                    } else if (param.startsWith("CL")) {
                        chordLength = parseFloat(param.substring(2));
                    } else if (param.startsWith("CD")) {
                        chordLength = parseFloat(param.substring(2));
                    } else if (param.startsWith("CH")) {
                        if (hasChordLengthParam) {
                            bearingStr = parts[j].trim().substring(2); // Treat CH as Chord Bearing shorthand
                        } else {
                            chordLength = parseFloat(param.substring(2)); // Treat CH as Chord Length
                        }
                    } else if (param.startsWith("LA")) {
                        const valStr = param.substring(2);
                        if (valStr) {
                            arcLength = parseFloat(valStr);
                        }
                        turn = 'L';
                    } else if (param.startsWith("RA")) {
                        const valStr = param.substring(2);
                        if (valStr) {
                            arcLength = parseFloat(valStr);
                        }
                        turn = 'R';
                    } else if (param === 'L' || param === 'R') {
                        turn = param; // bare turn direction, not an empty radius
                    } else if (param.startsWith("R")) {
                        radius = parseFloat(param.substring(1));
                    } else if (param.startsWith("D")) {
                        const rawD = parseFloat(param.substring(1));
                        deltaAngle = bearDec(rawD);
                    } else if (param === 'L' || param === 'R') {
                        turn = param;
                    }
                }
                
                if (!turn) {
                    throw new Error(`Course #${courseNum} (on line ${i + 1}) (Curve): Turn direction is missing. Please add 'LA' (Left Arc) or 'RA' (Right Arc) for arc length, or specify turn direction as 'L' or 'R'.`);
                }
                
                const provided = [];
                if (radius !== null && !isNaN(radius)) provided.push("Radius (R)");
                if (arcLength !== null && !isNaN(arcLength)) provided.push("Arc Length (LA/RA)");
                if (chordLength !== null && !isNaN(chordLength)) provided.push("Chord Distance (CH/CD/CL)");
                if (deltaAngle !== null && !isNaN(deltaAngle)) provided.push("Delta Angle (D)");
                
                if (provided.length < 2) {
                    const missing = [];
                    if (radius === null) missing.push("Radius (R)");
                    if (arcLength === null) missing.push("Arc Length (LA/RA)");
                    if (chordLength === null) missing.push("Chord Distance (CH/CD/CL)");
                    if (deltaAngle === null) missing.push("Delta Angle (D)");
                    
                    throw new Error(`Course #${courseNum} (on line ${i + 1}) (Curve): Not enough parameters to compute curve. Provided: ${provided.join(', ') || 'None'}. Please provide at least one of the following to resolve the curve geometry: ${missing.join(', ')}.`);
                }
                
                const solved = solveCurve(radius, arcLength, deltaAngle, chordLength);
                if (!solved || isNaN(solved.R) || isNaN(solved.L) || isNaN(solved.C) || isNaN(solved.Delta) || solved.R <= 0 || solved.L <= 0 || solved.C <= 0 || solved.Delta <= 0) {
                    throw new Error(`Course #${courseNum} (on line ${i + 1}) (Curve): Invalid curve geometry combination.`);
                }
                
                let quad, bearing, hemi;
                if (bearingStr) {
                    const parsedBear = parseCsvBearing(bearingStr);
                    if (parsedBear === null) {
                        throw new Error(`Course #${courseNum} (on line ${i + 1}) (Curve): Invalid bearing value "${bearingStr}". Formats: QDD.MMSS (e.g. 135.0020) or explicit (e.g. N 35.0020 E).`);
                    }
                    quad = parsedBear.quad;
                    bearing = parsedBear.bearing;
                    hemi = parsedBear.hemi;
                } else {
                    if (loadedCourses.length === 0) {
                        throw new Error(`Course #${courseNum} (on line ${i + 1}) (Curve): No chord bearing specified, and there is no previous course to calculate a tangent curve. Please specify a bearing (e.g. "1, 135.0020, C, ...").`);
                    }
                    const prev = loadedCourses[loadedCourses.length - 1];
                    if (prev.isInvalid) {
                        throw new Error(`Course #${courseNum} (on line ${i + 1}) (Curve): Cannot compute tangent chord bearing because the previous course is invalid.`);
                    }
                    const startTangent = getCourseEndTangent(prev);
                    const halfDelta = solved.Delta / 2;
                    let chordAz;
                    if (turn === 'R') {
                        chordAz = (startTangent + halfDelta) % 360;
                    } else {
                        chordAz = (startTangent - halfDelta + 360) % 360;
                    }
                    const converted = azimuthToQuadBearingHemi(chordAz);
                    quad = converted.quad;
                    bearing = converted.bearing;
                    hemi = converted.hemi;
                }
                
                loadedCourses.push({
                    type: "curve",
                    quad,
                    bearing,
                    hemi,
                    distance: solved.L,
                    chordLength: solved.C,
                    radius: solved.R,
                    deltaAngle: solved.Delta,
                    turn,
                    rawBearing: bearingStr || fmtBearDMS(bearing)
                });
                
            } else {
                bearingStr = parts[1];
                const distance = parseFloat(parts[2]);
                
                const parsedBear = parseCsvBearing(bearingStr);
                if (parsedBear === null) {
                    throw new Error(`Course #${courseNum} (on line ${i + 1}): Invalid bearing value "${bearingStr}". Formats: QDD.MMSS (e.g. 135.0020) or explicit (e.g. N 35.0020 E).`);
                }
                if (isNaN(distance) || distance <= 0) {
                    throw new Error(`Course #${courseNum} (on line ${i + 1}): Invalid distance value "${parts[2]}". Must be greater than zero.`);
                }
                
                loadedCourses.push({
                    type: "line",
                    quad: parsedBear.quad,
                    bearing: parsedBear.bearing,
                    hemi: parsedBear.hemi,
                    distance,
                    rawBearing: bearingStr
                });
            }
        } catch (err) {
            let quad = null, hemi = null, rawBearing = null;
            let radius = null, chordLength = null, arcLength = null, deltaAngle = null, turn = null;
            
            if (isCurve) {
                const hasChordLengthParam = parts.slice(paramStartIndex).some(p => {
                    const u = p.trim().toUpperCase();
                    return u.startsWith("CL") || u.startsWith("CD");
                });
                
                let cbParam = parts.slice(paramStartIndex).find(p => {
                    const u = p.trim().toUpperCase();
                    if (u.startsWith("CB")) return true;
                    if (hasChordLengthParam && u.startsWith("CH")) return true;
                    return false;
                });
                if (cbParam) {
                    bearingStr = cbParam.trim().substring(2);
                }
                
                let rParam = parts.slice(paramStartIndex).find(p => {
                    const u = p.trim().toUpperCase();
                    return u.startsWith("R") && !u.startsWith("RA") && u !== "R";
                });
                if (rParam) radius = parseFloat(rParam.trim().substring(1));
                
                let clParam = parts.slice(paramStartIndex).find(p => {
                    const u = p.trim().toUpperCase();
                    if (u.startsWith("CL") || u.startsWith("CD")) return true;
                    if (!hasChordLengthParam && u.startsWith("CH")) return true;
                    return false;
                });
                if (clParam) {
                    const prefixLen = clParam.trim().toUpperCase().startsWith("CH") || clParam.trim().toUpperCase().startsWith("CL") || clParam.trim().toUpperCase().startsWith("CD") ? 2 : 1;
                    chordLength = parseFloat(clParam.trim().substring(prefixLen));
                }
                
                let laParam = parts.slice(paramStartIndex).find(p => {
                    const u = p.trim().toUpperCase();
                    return u.startsWith("LA") || u.startsWith("RA");
                });
                if (laParam) {
                    const u = laParam.trim().toUpperCase();
                    const valStr = laParam.trim().substring(2);
                    if (valStr) arcLength = parseFloat(valStr);
                    turn = u.startsWith("LA") ? "L" : "R";
                } else {
                    let tParam = parts.slice(paramStartIndex).find(p => {
                        const u = p.trim().toUpperCase();
                        return u === "L" || u === "R";
                    });
                    if (tParam) turn = tParam.trim().toUpperCase();
                }
                
                let dParam = parts.slice(paramStartIndex).find(p => {
                    const u = p.trim().toUpperCase();
                    return u.startsWith("D");
                });
                if (dParam) {
                    const rawD = parseFloat(dParam.trim().substring(1));
                    if (!isNaN(rawD)) deltaAngle = bearDec(rawD);
                }
            }
            
            if (bearingStr) {
                const parsedBear = parseCsvBearing(bearingStr);
                if (parsedBear !== null) {
                    quad = parsedBear.quad;
                    hemi = parsedBear.hemi;
                }
                rawBearing = bearingStr;
            }
            
            loadedCourses.push({
                type: "invalid",
                isInvalid: true,
                errorMsg: err.message,
                partialType: isCurve ? "curve" : "line",
                partialData: {
                    quad,
                    hemi,
                    rawBearing,
                    radius,
                    chordLength,
                    arcLength,
                    deltaAngle,
                    turn,
                    distance: !isCurve ? parseFloat(parts[2]) : null
                }
            });
            errorCount++;
        }
    }
    
    const name = fileName.replace(/\.[^/.]+$/, "");
    return { name, lot: "", courses: loadedCourses, errorCount };
}

// Auto-detect format and route to correct parser
function detectAndParseFile(content, fileName) {
    const lines = content.split(/\r?\n/).map(l => l.trim()).filter(l => l.length > 0);
    if (lines.length === 0) {
        throw new Error("Empty file");
    }
    
    const secondLine = lines[1];
    const looksLikeRetro = lines.length >= 2 && /^\d+$/.test(secondLine);
    
    if (looksLikeRetro) {
        return parseMapFile(content);
    } else {
        return parseCsvTextFile(content, fileName);
    }
}

// Parse retro .MAP file format
function parseMapFile(content) {
    const lines = content.split(/\r?\n/).map(l => l.trim()).filter(l => l.length > 0);
    if (lines.length < 2) {
        throw new Error("Invalid file format: too few lines");
    }
    
    let name = lines[0];
    let lot = "";
    if (lines[0].includes(" | ")) {
        const parts = lines[0].split(" | ");
        name = parts[0];
        lot = parts[1];
    }
    const numCourses = parseInt(lines[1], 10);
    if (isNaN(numCourses)) {
        throw new Error("Invalid file format: line 2 should contain course count");
    }
    
    const loadedCourses = [];
    let lineIdx = 2;
    let errorCount = 0;
    for (let i = 0; i < numCourses; i++) {
        if (lineIdx + 3 >= lines.length) {
            throw new Error(`Invalid file format: missing details for course #${i + 1}`);
        }
        
        const rawQuad = lines[lineIdx];
        const bearingDec = parseFloat(lines[lineIdx + 1]);
        const hemi = lines[lineIdx + 2].toUpperCase();
        const distance = parseFloat(lines[lineIdx + 3]);
        
        try {
            if (isNaN(bearingDec) || (hemi !== 'E' && hemi !== 'W') || isNaN(distance)) {
                throw new Error(`Invalid data in course #${i + 1} fields`);
            }
            
            if (rawQuad.includes(" | CURVE | ")) {
                const parts = rawQuad.split(" | ");
                if (parts.length < 7) {
                    throw new Error(`Invalid curve format in course #${i + 1}`);
                }
                const quad = parts[0].toUpperCase();
                const turn = parts[2].toUpperCase();
                const radius = parseFloat(parts[3]);
                const arcLength = parseFloat(parts[4]);
                const chordLength = parseFloat(parts[5]);
                const deltaAngle = parseFloat(parts[6]);
                
                if (quad !== 'N' && quad !== 'S') {
                    throw new Error(`Invalid curve quadrant value in course #${i + 1}: ${quad}`);
                }
                if (turn !== 'L' && turn !== 'R') {
                    throw new Error(`Invalid curve turn direction in course #${i + 1}: ${turn}`);
                }
                if (isNaN(radius) || isNaN(arcLength) || isNaN(chordLength) || isNaN(deltaAngle)) {
                    throw new Error(`Invalid curve measurements in course #${i + 1}`);
                }
                
                loadedCourses.push({
                    type: "curve",
                    quad,
                    bearing: bearingDec,
                    hemi,
                    distance: arcLength, // Arc Length
                    chordLength,
                    radius,
                    deltaAngle,
                    turn,
                    rawBearing: fmtBearDMS(bearingDec)
                });
            } else {
                const quad = rawQuad.toUpperCase();
                if (quad !== 'N' && quad !== 'S') {
                    throw new Error(`Invalid quadrant value in course #${i + 1}: ${quad}`);
                }
                loadedCourses.push({
                    type: "line",
                    quad,
                    bearing: bearingDec,
                    hemi,
                    distance,
                    rawBearing: fmtBearDMS(bearingDec)
                });
            }
        } catch (err) {
            loadedCourses.push({
                type: "invalid",
                isInvalid: true,
                errorMsg: err.message,
                partialType: rawQuad.includes(" | CURVE | ") ? "curve" : "line",
                partialData: {
                    quad: rawQuad.includes(" | CURVE | ") ? (rawQuad.split(" | ")[0].toUpperCase() || null) : (rawQuad.toUpperCase() || null),
                    hemi: (hemi === 'E' || hemi === 'W') ? hemi : null,
                    rawBearing: !isNaN(bearingDec) ? fmtBearDMS(bearingDec) : null,
                    radius: rawQuad.includes(" | CURVE | ") ? (parseFloat(rawQuad.split(" | ")[3]) || null) : null,
                    arcLength: rawQuad.includes(" | CURVE | ") ? (parseFloat(rawQuad.split(" | ")[4]) || null) : null,
                    chordLength: rawQuad.includes(" | CURVE | ") ? (parseFloat(rawQuad.split(" | ")[5]) || null) : null,
                    deltaAngle: rawQuad.includes(" | CURVE | ") ? (parseFloat(rawQuad.split(" | ")[6]) || null) : null,
                    turn: rawQuad.includes(" | CURVE | ") ? (rawQuad.split(" | ")[2].toUpperCase() || null) : null,
                    distance: !isNaN(distance) ? distance : null
                }
            });
            errorCount++;
        }
        
        lineIdx += 4;
    }
    
    return { name, lot, courses: loadedCourses, errorCount };
}

// Generate retro .MAP output
function generateMapFile(name, platCourses) {
    const firstLine = lotDesignation ? `${name} | ${lotDesignation}` : name;
    let out = `${firstLine}\r\n`;
    out += `${platCourses.length}\r\n`;
    platCourses.forEach(c => {
        if (c.type === "curve") {
            // Save curve details inside legacy Quad field with pipe delimiters
            out += `${c.quad} | CURVE | ${c.turn} | ${c.radius.toFixed(3)} | ${c.distance.toFixed(3)} | ${c.chordLength.toFixed(3)} | ${c.deltaAngle.toFixed(5)}\r\n`;
            out += `${c.bearing.toFixed(10)}\r\n`;
            out += `${c.hemi}\r\n`;
            out += `${c.chordLength.toFixed(3)}\r\n`; // Legacy app reads chord length as straight-line distance
        } else {
            out += `${c.quad}\r\n`;
            out += `${c.bearing.toFixed(10)}\r\n`;
            out += `${c.hemi}\r\n`;
            out += `${c.distance.toFixed(3)}\r\n`;
        }
    });
    return out;
}

// --- Event Handlers & Core Init ---

// Circular curve solver
function solveCurve(R, L, Delta, C) {
    let deltaRad = Delta ? Delta * Math.PI / 180 : null;
    
    // 1. R and L
    if (R !== null && L !== null) {
        deltaRad = L / R;
        C = 2 * R * Math.sin(deltaRad / 2);
        Delta = deltaRad * 180 / Math.PI;
    }
    // 2. R and Delta
    else if (R !== null && deltaRad !== null) {
        L = R * deltaRad;
        C = 2 * R * Math.sin(deltaRad / 2);
    }
    // 3. R and C
    else if (R !== null && C !== null) {
        if (C > 2 * R) return null; // Geometry violation
        deltaRad = 2 * Math.asin(C / (2 * R));
        L = R * deltaRad;
        Delta = deltaRad * 180 / Math.PI;
    }
    // 4. L and Delta
    else if (L !== null && deltaRad !== null) {
        if (deltaRad === 0) return null;
        R = L / deltaRad;
        C = 2 * R * Math.sin(deltaRad / 2);
    }
    // 5. C and Delta
    else if (C !== null && deltaRad !== null) {
        if (deltaRad === 0) return null;
        R = C / (2 * Math.sin(deltaRad / 2));
        L = R * deltaRad;
    }
    // 6. L and C
    else if (L !== null && C !== null) {
        const k = C / L;
        if (k >= 1.0 || k <= 0.0) return null;
        
        let x = Math.sqrt(6 * (1 - k)); // Newton-Raphson starting guess
        for (let iter = 0; iter < 100; iter++) {
            const fx = Math.sin(x) - k * x;
            const dfx = Math.cos(x) - k;
            const nextX = x - fx / dfx;
            if (Math.abs(nextX - x) < 1e-7) {
                x = nextX;
                break;
            }
            x = nextX;
        }
        deltaRad = x * 2;
        R = L / deltaRad;
        Delta = deltaRad * 180 / Math.PI;
    }
    
    return { R, L, Delta, C };
}

function updateCurveSolver() {
    const rVal = parseFloat(curveRadiusInput.value) || null;
    const lVal = parseFloat(curveArcLengthInput.value) || null;
    const cVal = parseFloat(curveChordInput.value) || null;
    
    let dVal = null;
    const deltaStr = curveDeltaInput.value.trim();
    if (deltaStr) {
        const parsedD = parseBearingInput(deltaStr);
        if (parsedD !== null) {
            dVal = parsedD.bearing;
        }
    }
    
    const fields = [
        { el: curveRadiusInput, val: rVal },
        { el: curveArcLengthInput, val: lVal },
        { el: curveDeltaInput, val: dVal },
        { el: curveChordInput, val: cVal }
    ];
    
    const filled = fields.filter(f => f.val !== null);
    
    if (filled.length >= 2) {
        fields.forEach(f => {
            if (f.val === null) {
                f.el.disabled = true;
            }
        });
        
        const solved = solveCurve(rVal, lVal, dVal, cVal);
        if (solved && solved.R > 0 && solved.L > 0 && solved.C > 0 && solved.Delta > 0) {
            solvedCurve = solved;
            curveSolvedInfo.style.display = "block";
            curveSolvedInfo.innerHTML = `
                <strong>Solved Curve Specs:</strong><br>
                Radius: ${solved.R.toFixed(2)}<br>
                Arc Length: ${solved.L.toFixed(2)}<br>
                Chord Length: ${solved.C.toFixed(2)}<br>
                Delta Angle: ${fmtBear(solved.Delta)}
            `;
            curveSolvedInfo.style.color = "var(--accent-emerald)";
        } else {
            solvedCurve = null;
            curveSolvedInfo.style.display = "block";
            curveSolvedInfo.innerHTML = "Invalid curve geometry combination.";
            curveSolvedInfo.style.color = "var(--accent-rose)";
        }
    } else {
        fields.forEach(f => f.el.disabled = false);
        solvedCurve = null;
        curveSolvedInfo.style.display = "none";
    }
}

window.switchTab = function(tab) {
    activeTab = tab;
    if (tab === "line") {
        tabLineBtn.classList.add("active");
        tabCurveBtn.classList.remove("active");
        lineInputsPanel.style.display = "block";
        curveInputsPanel.style.display = "none";
        
        distanceInput.required = true;
        bearingInputLabel.textContent = "Bearing (DD.MMSS or Degrees)";
        bearingInputHelper.textContent = "Formats: 45.3015 (45°30'15\") or 45-30-15";
    } else {
        tabLineBtn.classList.remove("active");
        tabCurveBtn.classList.add("active");
        lineInputsPanel.style.display = "none";
        curveInputsPanel.style.display = "block";
        
        distanceInput.required = false;
        bearingInputLabel.textContent = "Chord Bearing (DD.MMSS)";
        bearingInputHelper.textContent = "e.g. 45.3015 or prefix 445.2714 (NW)";
    }
}

function initEvents() {
    // Resize Handler
    window.addEventListener("resize", () => {
        const rect = canvasContainer.getBoundingClientRect();
        canvas.width = rect.width;
        canvas.height = rect.height;
        draw();
    });
    
    // Show Last Error recall handler
    const btnShowLastError = document.getElementById("btnShowLastError");
    if (btnShowLastError) {
        btnShowLastError.addEventListener("click", () => {
            if (lastErrorMsg) {
                showToast(lastErrorMsg, "error");
            }
        });
    }
    
    function updateTitleOverlay() {
        platTitleDisplay.textContent = platName;
        platSubDisplay.textContent = `${lotDesignation ? lotDesignation + ' • ' : ''}${courses.length} course${courses.length === 1 ? '' : 's'}`;
    }

    // Plat Title Sync
    platNameInput.addEventListener("input", () => {
        platName = platNameInput.value.trim() || "Untitled Plat";
        updateTitleOverlay();
    });

    // Lot Designation Sync
    lotDesignationInput.addEventListener("input", () => {
        lotDesignation = lotDesignationInput.value.trim();
        updateTitleOverlay();
    });

    // Starting Coordinates Sync
    startNorthingInput.addEventListener("input", () => {
        recalculatePlat();
    });
    startEastingInput.addEventListener("input", () => {
        recalculatePlat();
    });

    // Dynamic Quadrant code sync while typing
    bearingInput.addEventListener("input", () => {
        const val = bearingInput.value.trim();
        const match = val.match(/^([1-4])(\d{2})/);
        if (match) {
            const qCode = parseInt(match[1], 10);
            let quad, hemi;
            if (qCode === 1) { quad = "N"; hemi = "E"; }
            else if (qCode === 2) { quad = "S"; hemi = "E"; }
            else if (qCode === 3) { quad = "S"; hemi = "W"; }
            else if (qCode === 4) { quad = "N"; hemi = "W"; }
            
            quadrantInput.value = quad;
            hemisphereInput.value = hemi;
        }
    });

    // Curve parameter listeners
    [curveRadiusInput, curveArcLengthInput, curveDeltaInput, curveChordInput].forEach(el => {
        el.addEventListener("input", updateCurveSolver);
    });

    btnResetCurveParams.addEventListener("click", () => {
        [curveRadiusInput, curveArcLengthInput, curveDeltaInput, curveChordInput].forEach(el => {
            el.value = "";
            el.disabled = false;
        });
        solvedCurve = null;
        curveSolvedInfo.style.display = "none";
    });

    // Tab buttons
    tabLineBtn.addEventListener("click", () => switchTab("line"));
    tabCurveBtn.addEventListener("click", () => switchTab("curve"));
    
    // Add Course submission
    addCourseForm.addEventListener("submit", (e) => {
        e.preventDefault();
        
        let quad = quadrantInput.value;
        let hemi = hemisphereInput.value;
        const rawBear = bearingInput.value.trim();
        
        const parsed = parseBearingInput(rawBear);
        if (parsed === null) {
            showToast("Invalid bearing. Enter QDD.MMSS or DD.MMSS/DD-MM-SS", "error");
            return;
        }

        if (parsed.hasPrefix) {
            quad = parsed.quad;
            hemi = parsed.hemi;
            quadrantInput.value = quad;
            hemisphereInput.value = hemi;
        }
        
        let newCourse;
        if (activeTab === "line") {
            const dist = parseFloat(distanceInput.value);
            if (isNaN(dist) || dist <= 0) {
                showToast("Distance must be greater than zero", "error");
                return;
            }
            newCourse = {
                type: "line",
                quad,
                bearing: parsed.bearing,
                hemi,
                distance: dist,
                rawBearing: rawBear
            };
        } else {
            // Curve mode
            if (!solvedCurve) {
                showToast("Please enter exactly two curve parameters to solve curve geometry", "error");
                return;
            }
            newCourse = {
                type: "curve",
                quad,
                bearing: parsed.bearing,
                hemi,
                distance: solvedCurve.L, // Arc Length
                chordLength: solvedCurve.C,
                radius: solvedCurve.R,
                deltaAngle: solvedCurve.Delta,
                turn: curveTurnInput.value,
                rawBearing: rawBear
            };
            btnResetCurveParams.click();
        }

        if (editingIndex !== null) {
            const wasEditingInvalid = !!courses[editingIndex].isInvalid;
            courses[editingIndex] = newCourse;
            recalculatePlat();
            
            if (wasEditingInvalid) {
                const nextInvalidIndex = courses.findIndex(c => c.isInvalid);
                if (nextInvalidIndex !== -1) {
                    editCourse(nextInvalidIndex);
                    zoomToFit();
                    draw();
                    return;
                } else {
                    showToast("All course errors corrected!");
                }
            } else {
                showToast("Course updated successfully");
            }
            
            editingIndex = null;
            btnAddCourse.textContent = "Add Course";
            btnCancelEdit.style.display = "none";
            editInsertActions.style.display = "none";
        } else if (insertIndex !== null) {
            const position = insertIndex;
            courses.splice(position, 0, newCourse);
            insertIndex = null;
            btnAddCourse.textContent = "Add Course";
            btnCancelEdit.style.display = "none";
            recalculatePlat();
            showToast(`Course inserted as #${position + 1}`);
        } else {
            courses.push(newCourse);
            showToast("Course added successfully");
            recalculatePlat();
        }
        
        // Auto zoom fit to show newly added segments
        zoomToFit();
        draw();
        
        // Reset form bearing/distance
        bearingInput.value = "";
        distanceInput.value = "";
        bearingInput.focus();
    });
    
    // Cancel Edit Listener
    if (btnCancelEdit) {
        btnCancelEdit.addEventListener("click", cancelEdit);
    }
    
    // Insert a new course before/after the course being edited
    btnInsertBefore.addEventListener("click", () => {
        if (editingIndex !== null) startInsert(editingIndex);
    });
    btnInsertAfter.addEventListener("click", () => {
        if (editingIndex !== null) startInsert(editingIndex + 1);
    });

    // Clear plat
    btnClearPlat.addEventListener("click", () => {
        if (courses.length === 0) return;
        if (confirm("Are you sure you want to clear the entire plat?")) {
            courses = [];
            startNorthingInput.value = "";
            startEastingInput.value = "";
            if (editingIndex !== null || insertIndex !== null) {
                cancelEdit();
            }
            recalculatePlat();
            draw();
            showToast("Plat cleared", "info");
        }
    });
    
    // Library triggers
    btnSaveLibrary.addEventListener("click", saveToLibrary);
    btnOpenLibrary.addEventListener("click", () => {
        renderLibraryList();
        libraryModal.classList.add("active");
    });
    btnCloseLibrary.addEventListener("click", () => libraryModal.classList.remove("active"));
    btnCancelLibrary.addEventListener("click", () => libraryModal.classList.remove("active"));
    
    libraryModal.addEventListener("click", (e) => {
        if (e.target === libraryModal) libraryModal.classList.remove("active");
    });
    
    // Help Modal triggers
    const helpModal = document.getElementById("helpModal");
    const btnHelpImport = document.getElementById("btnHelpImport");
    const btnCloseHelp = document.getElementById("btnCloseHelp");
    const btnCancelHelp = document.getElementById("btnCancelHelp");
    
    if (btnHelpImport && helpModal) {
        btnHelpImport.addEventListener("click", () => {
            helpModal.classList.add("active");
        });
    }
    if (btnCloseHelp && helpModal) {
        btnCloseHelp.addEventListener("click", () => helpModal.classList.remove("active"));
    }
    if (btnCancelHelp && helpModal) {
        btnCancelHelp.addEventListener("click", () => helpModal.classList.remove("active"));
    }
    if (helpModal) {
        helpModal.addEventListener("click", (e) => {
            if (e.target === helpModal) helpModal.classList.remove("active");
        });
    }
    
    // Global ESC key to close active modals
    window.addEventListener("keydown", (e) => {
        if (e.key === "Escape") {
            const activeModals = document.querySelectorAll(".modal-overlay.active");
            activeModals.forEach(modal => modal.classList.remove("active"));
        }
    });
    
    // Zoom control buttons
    btnZoomIn.addEventListener("click", () => {
        zoom *= 1.25;
        draw();
    });
    btnZoomOut.addEventListener("click", () => {
        zoom /= 1.25;
        draw();
    });
    btnZoomFit.addEventListener("click", () => {
        zoomToFit();
        draw();
    });
    

    
    // Drag-to-pan implementation
    canvas.addEventListener("mousedown", (e) => {
        isDragging = true;
        canvas.style.cursor = "grabbing";
        startX = e.clientX - panX;
        startY = e.clientY - panY;
    });
    
    window.addEventListener("mouseup", () => {
        if (isDragging) {
            isDragging = false;
            canvas.style.cursor = "grab";
        }
    });
    
    canvas.addEventListener("mousemove", (e) => {
        const rect = canvas.getBoundingClientRect();
        mouseCanvasX = e.clientX - rect.left;
        mouseCanvasY = e.clientY - rect.top;
        
        if (isDragging) {
            panX = e.clientX - startX;
            panY = e.clientY - startY;
            draw();
        } else {
            // Hover course checking
            const hoverIndex = checkHoveredCourse(mouseCanvasX, mouseCanvasY);
            if (hoverIndex !== hoveredCourseIndex) {
                hoveredCourseIndex = hoverIndex;
                
                // Highlights correspond in table row too
                const rows = courseTableBody.querySelectorAll("tr");
                rows.forEach(r => {
                    const idx = parseInt(r.dataset.index);
                    if (idx === hoveredCourseIndex) {
                        if (editingIndex !== idx) {
                            r.style.backgroundColor = "rgba(245, 158, 11, 0.15)";
                        }
                    } else {
                        r.style.backgroundColor = (editingIndex === idx) ? "rgba(16, 185, 129, 0.15)" : "";
                    }
                });
                
                draw();
            }
        }
    });
    
    // Mouse wheel Zoom
    canvas.addEventListener("wheel", (e) => {
        e.preventDefault();
        
        // Find survey coordinate of mouse cursor prior to zoom
        const mouseSurvey = toSurveyCoords(mouseCanvasX, mouseCanvasY);
        
        // Adjust zoom
        const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
        zoom = Math.min(Math.max(zoom * factor, 0.01), 100);
        
        // Adjust pan to zoom into cursor point
        panX = mouseCanvasX - mouseSurvey.x * zoom;
        panY = mouseCanvasY + mouseSurvey.y * zoom; // Inverted Y coordinate math
        
        draw();
    }, { passive: false });
    
    let oldWidth, oldHeight, oldZoom, oldPanX, oldPanY;

    window.addEventListener("beforeprint", () => {
        if (courses.length === 0) return;

        // Update printing metadata grid fields
        document.getElementById("printPlatName").textContent = platName;
        document.getElementById("printLotDesignation").textContent = lotDesignation || "N/A";
        document.getElementById("printDate").textContent = new Date().toLocaleDateString() + ' ' + new Date().toLocaleTimeString();
        document.getElementById("printPrecision").textContent = precisionVal.textContent;

        const startNVal = startNorthingInput.value.trim();
        const startEVal = startEastingInput.value.trim();
        printStartNorthing.textContent = startNVal !== "" ? parseFloat(startNVal).toFixed(3) : "N/A";
        printStartEasting.textContent = startEVal !== "" ? parseFloat(startEVal).toFixed(3) : "N/A";

        // Calculate area for printing
        const areaSqFt = calculateTraverseArea();
        if (areaSqFt !== null) {
            const acres = areaSqFt / 43560;
            document.getElementById("printAreaSqFt").textContent = areaSqFt.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + " sq ft";
            document.getElementById("printAreaAcres").textContent = acres.toFixed(4) + " ac";
        } else {
            document.getElementById("printAreaSqFt").textContent = "N/A (Open)";
            document.getElementById("printAreaAcres").textContent = "N/A (Open)";
        }

        // Save current canvas state to restore after print
        oldWidth = canvas.width;
        oldHeight = canvas.height;
        oldZoom = zoom;
        oldPanX = panX;
        oldPanY = panY;

        // Set standard print dimensions
        canvas.width = 720;
        canvas.height = 480;

        isPrinting = true;
        zoomToFit();
        draw();
    });

    window.addEventListener("afterprint", () => {
        if (courses.length === 0) return;

        // Restore screen dimensions and state
        isPrinting = false;
        canvas.width = oldWidth;
        canvas.height = oldHeight;
        zoom = oldZoom;
        panX = oldPanX;
        panY = oldPanY;
        draw();
    });

    // Print Report Button
    const btnPrintReport = document.getElementById("btnPrintReport");
    btnPrintReport.addEventListener("click", () => {
        if (courses.length === 0) {
            showToast("No traverse data to print", "error");
            return;
        }
        window.print();
    });

    // Compute Area Button
    const btnComputeArea = document.getElementById("btnComputeArea");
    if (btnComputeArea) {
        btnComputeArea.addEventListener("click", () => {
            const areaSqFt = calculateTraverseArea();
            if (areaSqFt === null) {
                showToast("At least 3 courses are required to calculate area", "error");
                return;
            }
            const acres = areaSqFt / 43560;
            document.getElementById("areaSqFtVal").textContent = areaSqFt.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
            document.getElementById("areaAcresVal").textContent = acres.toFixed(4) + " ac";
            document.getElementById("areaDisplayBox").style.display = "block";
            showToast("Area computed successfully");
        });
    }

    // --- File Operations Event listeners ---
    
    function exportPlatFile() {
        if (courses.length === 0) {
            showToast("No courses to export", "error");
            return;
        }
        
        try {
            const outText = generateMapFile(platName, courses);
            const blob = new Blob([outText], { type: 'text/plain;charset=utf-8' });
            const url = URL.createObjectURL(blob);
            const link = document.createElement("a");
            link.href = url;
            link.download = `${platName.trim().replace(/[^a-z0-9]/gi, '_') || 'plat'}.MAP`;
            link.click();
            URL.revokeObjectURL(url);
            showToast("Exported .MAP file successfully");
        } catch (err) {
            console.error(err);
            showToast("Export failed", "error");
        }
    }
    
    // Export .MAP
    btnExportMap.addEventListener("click", exportPlatFile);
    
    const btnSaveFileSidebar = document.getElementById("btnSaveFileSidebar");
    if (btnSaveFileSidebar) {
        btnSaveFileSidebar.addEventListener("click", exportPlatFile);
    }
    
    // Import .MAP
    btnImportMap.addEventListener("change", (e) => {
        const file = e.target.files[0];
        if (!file) return;
        
        if (courses.length > 0) {
            if (!confirm(`Are you sure you want to import "${file.name}"? Your current traverse will be overwritten.`)) {
                e.target.value = "";
                return;
            }
        }
        
        const reader = new FileReader();
        reader.onload = function(evt) {
            try {
                const parsed = detectAndParseFile(evt.target.result, file.name);
                platName = parsed.name || file.name.replace(/\.[^/.]+$/, "");
                platNameInput.value = platName;
                platTitleDisplay.textContent = platName;
                
                lotDesignation = parsed.lot || "";
                lotDesignationInput.value = lotDesignation;
                
                startNorthingInput.value = "";
                startEastingInput.value = "";
                
                courses = parsed.courses;
                recalculatePlat();
                zoomToFit();
                draw();
                
                if (parsed.errorCount > 0) {
                    const firstInvalid = courses.findIndex(c => c.isInvalid);
                    if (firstInvalid !== -1) {
                        editCourse(firstInvalid);
                    }
                    showToast(`Imported "${platName}" with ${parsed.errorCount} invalid course(s). Loaded Course #${firstInvalid + 1} into editor for correction.`, "error");
                } else {
                    showToast(`Successfully imported: "${platName}"`);
                }
            } catch (err) {
                console.error(err);
                showToast(`Import error: ${err.message}`, "error");
            }
            e.target.value = ""; // Clear input file trigger
        };
        reader.onerror = () => showToast("Failed to read file", "error");
        reader.readAsText(file);
    });
}

function init() {
    // Set initial size of canvas to fit container
    const rect = canvasContainer.getBoundingClientRect();
    canvas.width = rect.width;
    canvas.height = rect.height;
    
    initLibrary();
    initEvents();
    
    // Start centering viewport
    panX = canvas.width / 2;
    panY = canvas.height / 2;
    
    recalculatePlat();
}

// Read by wasm-bridge.js, which needs the lot designation to call the Rust
// generateMapFile (a `let` at script scope is not reachable from a module).
window.getLotDesignation = () => lotDesignation;

window.onload = init;
