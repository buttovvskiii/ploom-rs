/** The built-in device, modelled on the Ploom Aura from product photographs. */
import * as THREE from "three";
import type { Finish, Surface } from "./finishes";
import { DEVICE, type DeviceModel } from "./model";
import { flakeNoise, softSpot, verticalRamp } from "./textures";

const H = DEVICE.height;
const A = DEVICE.width / 2;
const B = DEVICE.depth / 2;
const SEG = 192;
const QUARTER = Math.PI / 2;

/** Superellipse exponents: near-circular at the back, flatter across the front. */
const EXP_BACK = 2.2;
const EXP_FRONT = 3.4;

// Base: nearly a half-round seen from the front.
const R_BASE = 10.4;
const USB_R = 1.55;
const USB_L = 2.6; // half-length of the straight part; the port is 2(L + R) long
const USB_DEPTH = 3.4;

// Cap.
const SEAM_DROP = 7.6; // parting line below the top, at the front
const SEAM_BACK_DROP = 1; // ...and at the back, where the cap thins out on the top edge
const SEAM_BOW = 1.25; // > 1: the arc stays high along the side and dips toward the front
const CAP_PROUD = 0.03; // the cap is flush with the body; this only avoids z-fighting
const VISOR = 0.42; // overhang of the cap's front edge: level with the panel's face
const CAP_LEDGE = 0.32; // how far the parting gap is cut back under the cap
/** Plan of the crest: a rounded rectangle just inside the body's outline. */
const RIM = { halfWidth: 9, front: 13.55, back: 10.4, exp: 3 };
const SHOULDER = 4.2; // height of the roll from the wall up to the crest, along the sides
const BACK_ROLL = 6.4; // ...and at the back, where the top corner is rounder
const LIP_ROLL = 0.75; // the same roll at the front lip: thin and sharp
const LIP_DROP = 2.6; // the pocket's side walls slope down by this much to the lip
const CREST_START_Z = -3; // the crest is level behind this and arcs down in front of it
const NOTCH = 1.5; // extra dip of the front lip's middle: the U-shaped notch
const NOTCH_HALF = 2.5; // the notch is full depth within this half-width...
const NOTCH_EASE = 6.5; // ...and eases out over this much more
const R_RIM_IN = 0.45; // tight inner roll from the crest into the pocket
const RIM_FLAT = 0.15;
const BOWL_GROW = -(RIM_FLAT + R_RIM_IN);
const RECESS = 4.4; // depth of the pocket below the crest
const Y_STRAIGHT = H - 9.5; // top of the plain extrusion; the cap's variable rim starts above

// Shutter: a lid whose back half matches the recess.
const LID_GAP = 0.2;
const LID_FRONT = 2; // how far the lid reaches in front of its centre
const LID_RISE = 0.12; // the faint crown of the lid above the crest
const LID_EDGE = 0.5; // radius of the lid's rounded edge along the hood
const LID_NOSE = 2; // ...and of its front, which rolls off into the pocket
const LID_SINK = 2.6; // the open lid rides this much lower, down in the pocket
const SHUTTER_CLOSED_Z = 0;
const SHUTTER_OPEN_Z = 8.4;
const SOCKET_R = 3.1;
const SOCKET_Z = -4.7;
const SOCKET_DEPTH = 15;

// Front panel (arc length across the face × height).
const PANEL = {
  halfWidth: 8.9, // measured across x
  bottom: 3.2,
  top: H - SEAM_DROP + 0.3, // the top end tucks under the cap's visor
  tuck: 0.38, // ...leaning back by this much
  tuckSpan: 3.4, // over this height
  cornerTop: 1.7,
  cornerBottom: 8.4,
  proud: 0.3,
  roll: 0.3,
  crown: 0.16,
  crownSpan: 7,
};
const GAP_WIDTH = 0.28;

// Status slit, centred on the panel.
const SLIT_TOP = H - 17.2;
const SLIT_LENGTH = 12.6;
const SLIT_WIDTH = 0.9;

function smooth(t: number): number {
  const c = Math.min(1, Math.max(0, t));
  return c * c * (3 - 2 * c);
}

/** A point on the cross-section, optionally inset by `inset` millimetres. */
function section(theta: number, inset = 0): [x: number, z: number] {
  const c = Math.cos(theta);
  const s = Math.sin(theta);
  const n = EXP_BACK + (EXP_FRONT - EXP_BACK) * smooth((s + 1) / 2);
  const e = 2 / n;
  return [
    (A - inset) * Math.sign(c) * Math.abs(c) ** e,
    (B - inset) * Math.sign(s) * Math.abs(s) ** e,
  ];
}

/** A point on the plan of the rim's crest, optionally grown outward by `grow` mm. */
function rimOutline(theta: number, grow = 0): [x: number, z: number] {
  const c = Math.cos(theta);
  const s = Math.sin(theta);
  const e = 2 / RIM.exp;
  const depth = s >= 0 ? RIM.front : RIM.back;
  return [
    (RIM.halfWidth + grow) * Math.sign(c) * Math.abs(c) ** e,
    (depth + grow) * Math.sign(s) * Math.abs(s) ** e,
  ];
}

/** 0 along the sides and back of the rim, 1 across the front lip. */
function lipness(theta: number): number {
  return smooth((rimOutline(theta)[1] - 6.5) / 6.5);
}

/**
 * Height of the crest at depth `z`: level at the back, then one convex arc down to the front lip,
 * so the side silhouette is a continuous curve.
 */
function crestAt(z: number): number {
  const t = Math.min(1, Math.max(0, (z - CREST_START_Z) / (RIM.front - CREST_START_Z)));
  return H - LIP_DROP * t * t;
}

/** Depth of the U-shaped notch in the front lip at this point of the rim. */
function notchAt(theta: number): number {
  const [x, z] = rimOutline(theta);
  return NOTCH * smooth((z - 8) / 4.5) * (1 - smooth((Math.abs(x) - NOTCH_HALF) / NOTCH_EASE));
}

function rimAt(theta: number): number {
  return crestAt(rimOutline(theta)[1]) - notchAt(theta);
}

/** Height of the recess floor at depth `z`: it follows the crest. */
function floorAt(z: number): number {
  return crestAt(z) - RECESS;
}

function rollUp(theta: number): number {
  const back = smooth((-rimOutline(theta)[1] - 4) / 5);
  const side = SHOULDER + (BACK_ROLL - SHOULDER) * back;
  return side + (LIP_ROLL - side) * lipness(theta);
}

/** Where the vertical wall ends and the rim starts to roll inward. */
function wallTop(theta: number): number {
  return rimAt(theta) - rollUp(theta);
}

/** The rim's outer roll: a quarter-ellipse from the wall (k = 0) to the crest (k = 1). */
function rollPoint(theta: number, k: number): [x: number, z: number] {
  const [bx, bz] = section(theta);
  const [rx, rz] = rimOutline(theta);
  return [bx + (rx - bx) * k, bz + (rz - bz) * k];
}

/** A point on the outer skin at height `y`: base round-over, wall, or rim roll. */
function surfaceAt(theta: number, y: number): THREE.Vector3 {
  if (y < R_BASE) {
    const k = 1 - Math.max(0, y) / R_BASE;
    const [x, z] = section(theta, R_BASE * (1 - Math.sqrt(Math.max(0, 1 - k * k))));
    return new THREE.Vector3(x, y, z);
  }
  const top = wallTop(theta);
  const u = Math.min(1, Math.max(0, (y - top) / rollUp(theta)));
  const [x, z] = rollPoint(theta, 1 - Math.sqrt(1 - u * u));
  return new THREE.Vector3(x, y, z);
}

/** Outward unit normal of the outer skin (central differences). */
function normalAt(theta: number, y: number): THREE.Vector3 {
  const dt = surfaceAt(theta + 1e-3, y).sub(surfaceAt(theta - 1e-3, y));
  const dy = surfaceAt(theta, y + 2e-2).sub(surfaceAt(theta, y - 2e-2));
  return dy.cross(dt).normalize();
}

/** Height of the parting line between cap and body around the outline. */
function seamAt(theta: number): number {
  const t = Math.min(1, Math.max(0, (section(theta)[1] + B) / (2 * B)));
  return H - SEAM_BACK_DROP - (SEAM_DROP - SEAM_BACK_DROP) * t ** SEAM_BOW;
}

/** How far the cap's lower edge overhangs the body: a visor across the front. */
function capOverhang(theta: number): number {
  const [x, z] = section(theta);
  const front = z > 0 ? 1 - smooth((Math.abs(x) - 9.4) / 2.2) : 0;
  return CAP_PROUD + VISOR * front;
}

/** A point on the cap's outer skin, `w` of the way (0..1) from the parting line to the crest. */
function capPoint(theta: number, w: number): [x: number, y: number, z: number] {
  const seam = seamAt(theta);
  const wall = wallTop(theta);
  const roll = rollUp(theta);
  const straight = Math.max(0, wall - seam);
  const psi0 = Math.asin(Math.min(1, Math.max(0, (seam - wall) / roll)));
  const s = w * (straight + roll * (QUARTER - psi0));
  const y = s < straight ? seam + s : wall + roll * Math.sin(psi0 + (s - straight) / roll);
  const point = surfaceAt(theta, y);
  const out = capOverhang(theta) * (1 - w) ** 1.4;
  if (out > 1e-3) point.addScaledVector(normalAt(theta, Math.min(y, rimAt(theta) - 0.05)), out);
  return [point.x, point.y, point.z];
}

/** The USB-C opening: a stadium, as a support mapping so its sides stay straight. */
function stadium(theta: number): [x: number, z: number] {
  const c = Math.cos(theta);
  const s = Math.sin(theta);
  return [USB_R * c, USB_L * Math.sign(Math.round(s * 1e6)) + USB_R * s];
}

/** Arc length along the front of the cross-section, so the panel is laid out in millimetres. */
class ArcLength {
  private readonly thetas: Float64Array;
  private readonly lengths: Float64Array;

  constructor(samples = 4096) {
    this.thetas = new Float64Array(samples + 1);
    this.lengths = new Float64Array(samples + 1);
    let [px, pz] = section(0);
    for (let i = 1; i <= samples; i++) {
      const theta = (Math.PI * i) / samples;
      const [x, z] = section(theta);
      this.thetas[i] = theta;
      this.lengths[i] = (this.lengths[i - 1] ?? 0) + Math.hypot(x - px, z - pz);
      px = x;
      pz = z;
    }
  }

  at(theta: number): number {
    return interpolate(this.thetas, this.lengths, theta);
  }

  theta(length: number): number {
    return interpolate(this.lengths, this.thetas, length);
  }
}

function interpolate(xs: Float64Array, ys: Float64Array, x: number): number {
  let lo = 0;
  let hi = xs.length - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if ((xs[mid] ?? 0) <= x) lo = mid;
    else hi = mid;
  }
  const x0 = xs[lo] ?? 0;
  const x1 = xs[hi] ?? 1;
  const t = x1 === x0 ? 0 : (x - x0) / (x1 - x0);
  return (ys[lo] ?? 0) + ((ys[hi] ?? 0) - (ys[lo] ?? 0)) * t;
}

/** Bisection for the angle in [lo, hi] at which `f(theta)` crosses zero (f falling). */
function solveTheta(lo: number, hi: number, f: (theta: number) => number): number {
  let a = lo;
  let b = hi;
  for (let i = 0; i < 48; i++) {
    const mid = (a + b) / 2;
    if (f(mid) > 0) a = mid;
    else b = mid;
  }
  return (a + b) / 2;
}

type Ring = (theta: number) => [x: number, y: number, z: number];

/** Skin a stack of closed rings. */
function loft(rings: Ring[], seg = SEG, shade?: number[]): THREE.BufferGeometry {
  const w = seg + 1;
  const count = rings.length * w;
  const positions = new Float32Array(count * 3);
  const uvs = new Float32Array(count * 2);
  const colors = new Float32Array(count * 3).fill(1);
  let p = 0;
  let q = 0;
  let along = 0;
  let previous: [number, number, number] | null = null;
  for (const [r, ring] of rings.entries()) {
    const first = ring(0);
    colors.fill(shade?.[r] ?? 1, r * w * 3, (r + 1) * w * 3);
    if (previous) {
      along += Math.hypot(first[0] - previous[0], first[1] - previous[1], first[2] - previous[2]);
    }
    previous = first;
    for (let j = 0; j <= seg; j++) {
      const [x, y, z] = j === seg ? first : ring((2 * Math.PI * j) / seg);
      positions[p++] = x;
      positions[p++] = y;
      positions[p++] = z;
      uvs[q++] = (j / seg) * 11;
      uvs[q++] = along / 8;
    }
  }
  const indices: number[] = [];
  for (let r = 0; r < rings.length - 1; r++) {
    for (let j = 0; j < seg; j++) {
      const a = r * w + j;
      const b = a + 1;
      const c = a + w;
      const d = c + 1;
      indices.push(a, c, b, b, c, d);
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute("uv", new THREE.BufferAttribute(uvs, 2));
  geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  // The first and last vertex of each ring coincide; share their normal.
  const normal = geometry.getAttribute("normal");
  for (let r = 0; r < rings.length; r++) {
    const a = r * w;
    const b = a + seg;
    const x = normal.getX(a) + normal.getX(b);
    const y = normal.getY(a) + normal.getY(b);
    const z = normal.getZ(a) + normal.getZ(b);
    const len = Math.hypot(x, y, z) || 1;
    normal.setXYZ(a, x / len, y / len, z / len);
    normal.setXYZ(b, x / len, y / len, z / len);
  }
  return geometry;
}

/** The satin body: base, walls, and up to the cap's parting line. */
function buildBody(): THREE.BufferGeometry {
  const rings: Ring[] = [];

  // Underside: from the USB-C opening out to the flat of the base.
  for (let i = 0, n = 5; i <= n; i++) {
    const w = i / n;
    rings.push((t) => {
      const [sx, sz] = stadium(t);
      const [fx, fz] = section(t, R_BASE);
      return [sx + (fx - sx) * w, 0, sz + (fz - sz) * w];
    });
  }
  // Base round-over up to the full section.
  for (let i = 1, n = 20; i <= n; i++) {
    const phi = (QUARTER * i) / n;
    const inset = R_BASE * (1 - Math.sin(phi));
    const y = R_BASE * (1 - Math.cos(phi));
    rings.push((t) => {
      const [x, z] = section(t, inset);
      return [x, y, z];
    });
  }
  // Plain extrusion.
  for (let i = 1, n = 6; i <= n; i++) {
    const y = R_BASE + ((Y_STRAIGHT - R_BASE) * i) / n;
    rings.push((t) => {
      const [x, z] = section(t);
      return [x, y, z];
    });
  }
  // Up to the parting line: low across the front, high at the back, where it runs onto the
  // shoulder's roll.
  for (let i = 1, n = 28; i <= n; i++) {
    const w = i / n;
    rings.push((t) => {
      const p = surfaceAt(t, Y_STRAIGHT + (seamAt(t) - Y_STRAIGHT) * w);
      return [p.x, p.y, p.z];
    });
  }
  // The ledge under the cap: a hard edge, then cut back into shadow.
  const shade = rings.map(() => 1);
  const edge: Ring = (t) => {
    const p = surfaceAt(t, seamAt(t));
    return [p.x, p.y, p.z];
  };
  rings.push(edge, (t) => {
    const y = seamAt(t);
    const p = surfaceAt(t, y).addScaledVector(normalAt(t, y), -CAP_LEDGE);
    return [p.x, p.y, p.z];
  });
  shade.push(0.3, 0.12);
  return loft(rings, SEG, shade);
}

/**
 * The glossy cap: from its lower edge (the visor across the front) over the shoulders to the crest,
 * and down the inner roll into the recess.
 */
function buildCap(): THREE.BufferGeometry {
  const rings: Ring[] = [];
  const shade: number[] = [];
  // Underside of the lower edge, starting inside the parting gap.
  rings.push((t) => {
    const y = seamAt(t);
    const p = surfaceAt(t, y).addScaledVector(normalAt(t, y), -CAP_LEDGE);
    return [p.x, p.y, p.z];
  });
  shade.push(0.12);
  // The lower edge twice, so the underside and the face shade separately.
  rings.push((t) => capPoint(t, 0));
  shade.push(0.3);
  // Outer skin: front wall and visor, the broad shoulders, the back edge.
  for (let i = 0, n = 34; i <= n; i++) {
    const w = i / n;
    rings.push((t) => capPoint(t, w));
    shade.push(1);
  }
  // Inner roll, down into the recess.
  for (let i = 0, n = 6; i <= n; i++) {
    const phi = (QUARTER * i) / n;
    const drop = R_RIM_IN * (1 - Math.cos(phi));
    rings.push((t) => {
      const [x, z] = rimOutline(t, -(RIM_FLAT + R_RIM_IN * Math.sin(phi)));
      return [x, rimAt(t) - drop, z];
    });
    shade.push(1);
  }
  // Short wall down to the recess floor.
  for (let i = 1, n = 2; i <= n; i++) {
    const w = i / n;
    rings.push((t) => {
      const [x, z] = rimOutline(t, BOWL_GROW);
      const top = rimAt(t) - R_RIM_IN;
      return [x, top + (floorAt(z) - top) * w, z];
    });
    shade.push(1 - 0.3 * w);
  }
  return loft(rings, SEG, shade);
}

/** The recess floor: from its outline in to the mouth of the stick socket. */
function buildFloor(): THREE.BufferGeometry {
  const rings: Ring[] = [];
  for (let i = 0, n = 14; i <= n; i++) {
    const w = i / n;
    rings.push((t) => {
      const [ox, oz] = rimOutline(t, BOWL_GROW + 0.05);
      const x = ox + (SOCKET_R * Math.cos(t) - ox) * w;
      const z = oz + (SOCKET_Z + SOCKET_R * Math.sin(t) - oz) * w;
      return [x, floorAt(z), z];
    });
  }
  return loft(rings, 96);
}

/** The lid's plan, around its own centre: the recess's back, a rounder front. */
function lidOutline(theta: number, scale: number): [x: number, z: number] {
  const c = Math.cos(theta);
  const s = Math.sin(theta);
  const halfWidth = RIM.halfWidth + BOWL_GROW - LID_GAP;
  const back = RIM.back + BOWL_GROW - LID_GAP;
  const exp = s >= 0 ? 4 : RIM.exp;
  return [
    halfWidth * scale * Math.sign(c) * Math.abs(c) ** (2 / RIM.exp),
    (s >= 0 ? LID_FRONT : back) * scale * Math.sign(s) * Math.abs(s) ** (2 / exp),
  ];
}

/** The sliding shutter. */
function buildShutter(): THREE.BufferGeometry {
  // A slab as deep as the pocket, flush with the crest: a flat top with a faint crown, a rounded
  // edge, and a skirt down to the floor.
  const rings: Ring[] = [];
  rings.push((t) => {
    const [x, z] = lidOutline(t, 1);
    return [x, -0.6, z];
  });
  for (let i = 0, n = 44; i <= n; i++) {
    const u = 1 - i / n;
    const scale = Math.max(u, 0.002);
    rings.push((t) => {
      const [x, z] = lidOutline(t, scale);
      // The edge is tight where the lid meets the hood and a broad nose in front.
      const nose = smooth(Math.sin(t) / 0.6);
      const band = 0.16 + 0.44 * nose;
      const e = Math.min(1, Math.max(0, (u - (1 - band)) / band));
      const radius = LID_EDGE + (LID_NOSE - LID_EDGE) * nose;
      return [x, RECESS + LID_RISE * (1 - u * u) - radius * (1 - Math.sqrt(1 - e * e)), z];
    });
  }
  return loft(rings, 128);
}

/** The inner wall of the USB-C port. */
function buildUsbWall(): THREE.BufferGeometry {
  return loft(
    [
      (t) => {
        const [x, z] = stadium(t);
        return [x, 0, z];
      },
      (t) => {
        const [x, z] = stadium(t);
        return [x, USB_DEPTH, z];
      },
    ],
    64,
  );
}

function stadiumShape(): THREE.Shape {
  const shape = new THREE.Shape();
  shape.absarc(0, USB_L, USB_R, 0, Math.PI, false);
  shape.absarc(0, -USB_L, USB_R, Math.PI, Math.PI * 2, false);
  shape.closePath();
  return shape;
}

function roundRectDistance(
  px: number,
  py: number,
  hx: number,
  hy: number,
  rTop: number,
  rBottom: number,
): number {
  const r = py >= 0 ? rTop : rBottom;
  const qx = Math.abs(px) - hx + r;
  const qy = Math.abs(py) - hy + r;
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
}

/** Sample positions, fine near `edges` and coarse elsewhere. */
function samples(
  lo: number,
  hi: number,
  edges: number[],
  band: number,
  fine: number,
  coarse: number,
): number[] {
  const out = [lo];
  let x = lo;
  while (x < hi) {
    const near = edges.some((e) => Math.abs(x - e) < band);
    x = Math.min(hi, x + (near ? fine : coarse));
    out.push(x);
  }
  return out;
}

/**
 * A patch of the front face lifted by `offset(d)` along its normal, where d is the signed distance
 * (mm) to the panel outline.
 */
function buildPatch(arc: ArcLength, offset: (d: number) => number): THREE.BufferGeometry {
  const sMid = arc.at(QUARTER);
  const edge = solveTheta(0, QUARTER, (t) => section(t)[0] - PANEL.halfWidth);
  const halfS = sMid - arc.at(edge);
  const yRef = (PANEL.bottom + PANEL.top) / 2;
  const halfY = (PANEL.top - PANEL.bottom) / 2;
  const margin = 1.1;

  const cols = samples(sMid - halfS - margin, sMid + halfS + margin, [], 0, 0.22, 0.22);
  const rows = samples(
    PANEL.bottom - margin,
    PANEL.top + margin,
    [PANEL.bottom, PANEL.top],
    PANEL.cornerBottom + margin + 1,
    0.22,
    1.6,
  );

  const positions = new Float32Array(cols.length * rows.length * 3);
  let p = 0;
  for (const y of rows) {
    for (const s of cols) {
      const theta = arc.theta(s);
      const d = roundRectDistance(
        s - sMid,
        y - yRef,
        halfS,
        halfY,
        PANEL.cornerTop,
        PANEL.cornerBottom,
      );
      // Toward its top end the panel leans back, under the cap's visor.
      const tuck = PANEL.tuck * smooth((y - (PANEL.top - PANEL.tuckSpan)) / PANEL.tuckSpan);
      const point = surfaceAt(theta, y).addScaledVector(normalAt(theta, y), offset(d) - tuck);
      positions[p++] = point.x;
      positions[p++] = point.y;
      positions[p++] = point.z;
    }
  }
  const indices: number[] = [];
  const w = cols.length;
  for (let i = 0; i < rows.length - 1; i++) {
    for (let j = 0; j < w - 1; j++) {
      const a = i * w + j;
      const b = a + 1;
      const c = a + w;
      const d = c + 1;
      indices.push(a, c, b, b, c, d);
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  return geometry;
}

function panelOffset(d: number): number {
  const { proud, roll, crown, crownSpan } = PANEL;
  // Inside the rolled edge the panel crowns gently toward its middle, so highlights sweep across
  // it.
  if (d <= -roll) {
    const t = Math.min(1, (-d - roll) / crownSpan);
    return proud + crown * (t * t * (3 - 2 * t));
  }
  if (d <= 0) return proud - roll + Math.sqrt(roll * roll - (d + roll) ** 2);
  return Math.max(-0.8, proud - roll - 4 * d);
}

function gapOffset(d: number): number {
  const half = GAP_WIDTH / 2;
  return Math.max(-0.6, Math.min(0.05, half - Math.abs(d - half)));
}

/** The cap's parting line: a hair-thin dark ribbon on the body, right under the cap's edge. */
function buildSeam(): THREE.BufferGeometry {
  const n = 480;
  const half = 0.11;
  const positions: number[] = [];
  for (let i = 0; i <= n; i++) {
    const theta = (2 * Math.PI * i) / n;
    const y = seamAt(theta) - half - 0.02;
    const normal = normalAt(theta, y);
    for (const side of [-half, half]) {
      const point = surfaceAt(theta, y + side).addScaledVector(normal, 0.03);
      positions.push(point.x, point.y, point.z);
    }
  }
  const indices: number[] = [];
  for (let i = 0; i < n; i++) {
    const a = i * 2;
    indices.push(a, a + 1, a + 2, a + 2, a + 1, a + 3);
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  return geometry;
}

/** An upright capsule in the XY plane, facing +Z. */
function capsule(length: number, width: number): THREE.ShapeGeometry {
  const r = width / 2;
  const half = length / 2 - r;
  const shape = new THREE.Shape();
  shape.absarc(0, half, r, 0, Math.PI, false);
  shape.absarc(0, -half, r, Math.PI, Math.PI * 2, false);
  return new THREE.ShapeGeometry(shape, 12);
}

/** A surface whose parameters ease toward a target finish. */
class EasedSurface {
  readonly color = new THREE.Color();
  metalness = 0;
  roughness = 0.5;
  clearcoat = 0;
  clearcoatRoughness = 0;
  specularIntensity = 1;
  private readonly target = new THREE.Color();
  private targetSurface: Surface | null = null;

  set(surface: Surface, instant: boolean): void {
    this.target.set(surface.color);
    this.targetSurface = surface;
    if (instant) this.step(1);
  }

  /** Blend a fraction `k` of the way; returns whether it is still moving. */
  step(k: number): boolean {
    const t = this.targetSurface;
    if (!t) return false;
    this.color.lerp(this.target, k);
    this.metalness += (t.metalness - this.metalness) * k;
    this.roughness += (t.roughness - this.roughness) * k;
    this.clearcoat += (t.clearcoat - this.clearcoat) * k;
    this.clearcoatRoughness += (t.clearcoatRoughness - this.clearcoatRoughness) * k;
    this.specularIntensity += (t.specularIntensity - this.specularIntensity) * k;
    const c = this.color;
    const g = this.target;
    return (
      Math.abs(c.r - g.r) + Math.abs(c.g - g.g) + Math.abs(c.b - g.b) > 1e-4 ||
      Math.abs(t.metalness - this.metalness) + Math.abs(t.roughness - this.roughness) > 1e-4
    );
  }

  apply(material: THREE.MeshPhysicalMaterial, dim: number): void {
    const c = this.color;
    const luma = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    material.color.setRGB(luma, luma, luma).lerp(c, 1 - 0.55 * dim);
    material.metalness = this.metalness;
    material.roughness = this.roughness;
    material.clearcoat = this.clearcoat;
    material.clearcoatRoughness = this.clearcoatRoughness;
    material.specularIntensity = this.specularIntensity;
  }
}

export function createProceduralModel(): DeviceModel {
  const root = new THREE.Group();
  root.name = "PloomAura";

  const flake = flakeNoise();
  const bodyMaterial = new THREE.MeshPhysicalMaterial({ roughnessMap: flake, vertexColors: true });
  // Clearcoat stays above zero on the panel so swapping finishes never recompiles shaders.
  const panelMaterial = new THREE.MeshPhysicalMaterial({ clearcoat: 1 });
  // The cap is lacquered like the panel; vertex colors shade its underside and recess.
  const capMaterial = new THREE.MeshPhysicalMaterial({ clearcoat: 1, vertexColors: true });
  const lineMaterial = new THREE.MeshStandardMaterial({
    roughness: 0.7,
    metalness: 0,
    side: THREE.DoubleSide,
  });
  // The bowl floor: the body's own color in shadow.
  const floorMaterial = new THREE.MeshStandardMaterial({
    roughness: 0.9,
    metalness: 0,
    envMapIntensity: 0.3,
    side: THREE.DoubleSide,
  });

  const arc = new ArcLength();
  const body = new THREE.Mesh(buildBody(), bodyMaterial);
  body.name = "Body";
  const cap = new THREE.Mesh(buildCap(), capMaterial);
  cap.name = "Cap";
  const panel = new THREE.Mesh(buildPatch(arc, panelOffset), panelMaterial);
  panel.name = "FrontPanel";
  const gap = new THREE.Mesh(buildPatch(arc, gapOffset), lineMaterial);
  const seam = new THREE.Mesh(buildSeam(), lineMaterial);
  const bowlFloor = new THREE.Mesh(buildFloor(), floorMaterial);
  root.add(body, cap, panel, gap, seam, bowlFloor);

  // Sliding shutter over the stick socket.
  const shutter = new THREE.Mesh(buildShutter(), capMaterial);
  shutter.name = "Shutter";
  /** Seat the lid on the floor at depth `z`, leaning with the ramp. */
  const seatShutter = (z: number): void => {
    const slope = (floorAt(z + 0.5) - floorAt(z - 0.5)) / 1;
    const open = (z - SHUTTER_CLOSED_Z) / (SHUTTER_OPEN_Z - SHUTTER_CLOSED_Z);
    shutter.position.set(0, floorAt(z) - LID_SINK * smooth(open), z);
    shutter.rotation.x = -Math.atan(slope);
  };
  seatShutter(SHUTTER_CLOSED_Z);
  root.add(shutter);

  // Stick socket: a dark well under the bowl, with the heater's glow at the bottom.
  const socketTop = floorAt(SOCKET_Z) + 0.05;
  const socketBottom = socketTop - SOCKET_DEPTH;
  const ramp = verticalRamp();
  const wellMaterial = new THREE.MeshStandardMaterial({
    color: 0x0a0a0b,
    roughness: 0.85,
    metalness: 0,
    emissive: 0x000000,
    emissiveMap: ramp,
    side: THREE.BackSide,
  });
  const wellGeometry = new THREE.CylinderGeometry(SOCKET_R, SOCKET_R, SOCKET_DEPTH, 64, 1, true);
  wellGeometry.translate(0, (socketTop + socketBottom) / 2, SOCKET_Z);
  const well = new THREE.Mesh(wellGeometry, wellMaterial);
  const wellFloorMaterial = new THREE.MeshStandardMaterial({
    color: 0x060606,
    roughness: 0.9,
    emissive: 0x000000,
  });
  const wellFloorGeometry = new THREE.CircleGeometry(SOCKET_R, 48);
  wellFloorGeometry.rotateX(-QUARTER);
  wellFloorGeometry.translate(0, socketBottom, SOCKET_Z);
  const wellFloor = new THREE.Mesh(wellFloorGeometry, wellFloorMaterial);
  const heatLight = new THREE.PointLight(0xffffff, 0, 16, 2);
  heatLight.position.set(0, socketTop + 1.5, SOCKET_Z);
  const spot = softSpot();
  const heatHalo = new THREE.Sprite(
    new THREE.SpriteMaterial({
      map: spot,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      transparent: true,
      opacity: 0,
    }),
  );
  heatHalo.scale.setScalar(13);
  heatHalo.position.set(0, socketTop + 1, SOCKET_Z);
  root.add(well, wellFloor, heatLight, heatHalo);

  // USB-C port in the base: a metal-lined slot with a dark tongue.
  const portMetal = new THREE.MeshStandardMaterial({
    color: 0xb9bbc0,
    metalness: 1,
    roughness: 0.32,
    side: THREE.BackSide,
  });
  const portDark = new THREE.MeshStandardMaterial({ color: 0x08080a, roughness: 0.8 });
  const portWall = new THREE.Mesh(buildUsbWall(), portMetal);
  const portFloorGeometry = new THREE.ShapeGeometry(stadiumShape(), 16);
  portFloorGeometry.rotateX(QUARTER);
  portFloorGeometry.translate(0, USB_DEPTH, 0);
  const portFloor = new THREE.Mesh(portFloorGeometry, portDark);
  const tongueGeometry = new THREE.BoxGeometry(0.7, USB_DEPTH - 0.7, 6.3);
  tongueGeometry.translate(0, 0.7 + (USB_DEPTH - 0.7) / 2, 0);
  const tongue = new THREE.Mesh(tongueGeometry, portDark);
  root.add(portWall, portFloor, tongue);

  // Status slit: hidden until lit, like the real one.
  const slitY = SLIT_TOP - SLIT_LENGTH / 2;
  const slitZ = B + PANEL.proud + PANEL.crown;
  const lampMaterial = new THREE.MeshBasicMaterial({
    color: 0x000000,
    transparent: true,
    opacity: 0,
    depthWrite: false,
  });
  const lamp = new THREE.Mesh(capsule(SLIT_LENGTH, SLIT_WIDTH), lampMaterial);
  lamp.name = "StatusLight";
  lamp.position.set(0, slitY, slitZ + 0.04);
  // A hairline surround that appears with the light, so a white lamp still reads on a white panel.
  const surroundMaterial = new THREE.MeshBasicMaterial({
    color: 0x1c1d20,
    transparent: true,
    opacity: 0,
    depthWrite: false,
  });
  const surround = new THREE.Mesh(capsule(SLIT_LENGTH + 0.5, SLIT_WIDTH + 0.5), surroundMaterial);
  surround.position.set(0, slitY, slitZ + 0.02);
  const lampHalo = new THREE.Sprite(
    new THREE.SpriteMaterial({
      map: spot,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      transparent: true,
      opacity: 0,
    }),
  );
  lampHalo.scale.set(5, SLIT_LENGTH + 6, 1);
  lampHalo.position.set(0, slitY, slitZ + 0.5);
  root.add(surround, lamp, lampHalo);

  const bodySurface = new EasedSurface();
  const panelSurface = new EasedSurface();
  let dim = 0;
  let surfacesMoving = false;
  let shutterOpen = 0;
  let shutterTarget = 0;
  let statusIntensity = 0;
  const shadowTone = new THREE.Color(0x0c0c0e);

  const applySurfaces = (): void => {
    bodySurface.apply(bodyMaterial, dim);
    panelSurface.apply(panelMaterial, dim);
    panelSurface.apply(capMaterial, dim);
    floorMaterial.color.copy(bodyMaterial.color).lerp(shadowTone, 0.975);
    // Parting lines are shadow, not paint: a darker tone of the body itself.
    lineMaterial.color.copy(bodyMaterial.color).multiplyScalar(0.22);
  };

  const facing = new THREE.Vector3();
  const toCamera = new THREE.Vector3();
  const lampWorld = new THREE.Vector3();

  return {
    root,
    procedural: true,

    setFinish(finish: Finish, instant: boolean): void {
      bodySurface.set(finish.body, instant);
      panelSurface.set(finish.cover, instant);
      surfacesMoving = !instant;
      applySurfaces();
    },

    setStatus(color: THREE.Color, intensity: number): void {
      statusIntensity = intensity;
      lampMaterial.color.copy(color).multiplyScalar(1.6 + 1.2 * intensity);
      lampMaterial.opacity = Math.min(1, intensity * 1.6);
      surroundMaterial.opacity = 0.4 * Math.min(1, intensity * 1.6);
      (lampHalo.material as THREE.SpriteMaterial).color.copy(color);
    },

    setGlow(color: THREE.Color, intensity: number): void {
      wellMaterial.emissive.copy(color);
      wellMaterial.emissiveIntensity = 2.2 * intensity;
      wellFloorMaterial.emissive.copy(color);
      wellFloorMaterial.emissiveIntensity = 3 * intensity;
      heatLight.color.copy(color);
      heatLight.intensity = 45 * intensity * shutterOpen;
      const halo = heatHalo.material as THREE.SpriteMaterial;
      halo.color.copy(color);
      halo.opacity = 0.3 * intensity * shutterOpen;
    },

    setShutter(open: boolean, instant = false): void {
      shutterTarget = open ? 1 : 0;
      if (instant) shutterOpen = shutterTarget;
    },

    setDim(next: number): void {
      dim = next;
      applySurfaces();
    },

    update(dt: number, camera: THREE.Camera): boolean {
      if (surfacesMoving) {
        const k = 1 - Math.exp(-dt / 0.16);
        const a = bodySurface.step(k);
        const b = panelSurface.step(k);
        surfacesMoving = a || b;
        applySurfaces();
      }
      // The shutter slides at a steady pace and eases at both ends.
      if (shutterOpen !== shutterTarget) {
        const step = dt / 0.7;
        shutterOpen =
          shutterTarget > shutterOpen
            ? Math.min(shutterTarget, shutterOpen + step)
            : Math.max(shutterTarget, shutterOpen - step);
      }
      seatShutter(SHUTTER_CLOSED_Z + (SHUTTER_OPEN_Z - SHUTTER_CLOSED_Z) * smooth(shutterOpen));

      // The lamp's halo fades as it turns away from the camera.
      lamp.getWorldPosition(lampWorld);
      facing.set(0, 0, 1).transformDirection(lamp.matrixWorld);
      toCamera.copy(camera.position).sub(lampWorld).normalize();
      const f = THREE.MathUtils.smoothstep(facing.dot(toCamera), 0.05, 0.6);
      (lampHalo.material as THREE.SpriteMaterial).opacity = 0.34 * statusIntensity * f;
      return surfacesMoving || shutterOpen !== shutterTarget;
    },

    dispose(): void {
      root.traverse((object) => {
        if (object instanceof THREE.Mesh || object instanceof THREE.Sprite) {
          // Sprites share one geometry inside three.js; only meshes own theirs.
          if (object instanceof THREE.Mesh) object.geometry.dispose();
          const materials = Array.isArray(object.material) ? object.material : [object.material];
          for (const m of materials) m.dispose();
        }
      });
      ramp.dispose();
      spot.dispose();
      flake.dispose();
    },
  };
}
