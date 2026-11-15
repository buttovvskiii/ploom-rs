/** A single, persistent 3D scene shared across the app's lifetime. */
import type { VisualTargets } from "./behavior";
import type { FinishId } from "./finishes";
import type { DeviceScene } from "./scene";

interface Handlers {
  onReady: () => void;
  onLost: () => void;
  onCustom: (custom: boolean) => void;
}

interface Holder {
  canvas: HTMLCanvasElement;
  /** A hidden, document-connected home for the canvas between mounts. */
  parking: HTMLElement;
  scene: DeviceScene | null;
  ready: boolean;
  /** The canvas has been shown once; later mounts skip the entrance. */
  revealed: boolean;
  active: boolean;
  width: number;
  height: number;
  handlers: Handlers | null;
  /** Cancels a move between two hosts that is still in the air. */
  landing: (() => void) | null;
}

let holder: Holder | null = null;

const FLIGHT_MS = 640;
/** Enough pixels to compile the shaders before the canvas has a place. */
const WARM_SIZE = { width: 240, height: 360 };

/** True once the device has been shown; a new viewer can display it at once. */
export function isViewerRevealed(): boolean {
  return holder?.revealed === true;
}

export function markViewerRevealed(): void {
  if (holder) holder.revealed = true;
}

/** WebGL 2 (three.js's minimum), probed without three.js and released right away. */
export function webglAvailable(): boolean {
  try {
    const gl = document.createElement("canvas").getContext("webgl2");
    gl?.getExtension("WEBGL_lose_context")?.loseContext();
    return gl !== null;
  } catch {
    return false;
  }
}

export interface ViewerHandle {
  setTargets(targets: VisualTargets): void;
  setFinish(finish: FinishId): void;
  setReducedMotion(on: boolean): void;
  setActive(on: boolean): void;
  resize(width: number, height: number): void;
  /** Detach the canvas and pause; the scene stays alive for the next mount. */
  release(): void;
}

function disposeHolder(): void {
  if (!holder) return;
  holder.landing?.();
  holder.scene?.dispose();
  holder.canvas.remove();
  holder.parking.remove();
  holder = null;
}

function ensureHolder(finish: FinishId, reducedMotion: boolean): Holder {
  if (holder) return holder;
  const canvas = document.createElement("canvas");
  canvas.setAttribute("aria-hidden", "true");
  // The canvas stays in the document at all times, or Chrome drops the WebGL context.
  const parking = document.createElement("div");
  parking.style.cssText = "position:fixed;width:0;height:0;overflow:hidden;visibility:hidden;";
  parking.appendChild(canvas);
  document.body.appendChild(parking);
  const h: Holder = {
    canvas,
    parking,
    scene: null,
    ready: false,
    revealed: false,
    active: false,
    width: 0,
    height: 0,
    handlers: null,
    landing: null,
  };
  holder = h;

  void import("./scene")
    .then(({ createDeviceScene }) =>
      createDeviceScene(canvas, {
        finish,
        reducedMotion,
        onFirstFrame: () => {
          h.ready = true;
          h.handlers?.onReady();
        },
        onLost: () => {
          const notify = h.handlers?.onLost;
          disposeHolder();
          notify?.();
        },
      }),
    )
    .then((scene) => {
      if (holder !== h) {
        scene.dispose();
        return;
      }
      h.scene = scene;
      // Drawing once here compiles the shaders, sized or not.
      if (h.width > 0 && h.height > 0) scene.resize(h.width, h.height);
      else scene.resize(WARM_SIZE.width, WARM_SIZE.height);
      scene.setActive(h.active);
      h.handlers?.onCustom(scene.custom);
    })
    .catch(() => {
      if (holder === h) holder = null;
      h.handlers?.onLost();
    });

  return h;
}

/** Start loading the scene before any viewer is on screen. */
export function prewarmViewer(finish: FinishId): void {
  if (holder || !webglAvailable()) return;
  ensureHolder(finish, matchMedia("(prefers-reduced-motion: reduce)").matches);
}

function easeInOut(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/** Carry the canvas from where it is to `host`, so the device never blinks out. */
function fly(h: Holder, from: DOMRect, host: HTMLElement): void {
  const carrier = document.createElement("div");
  carrier.className = "device-flight";
  carrier.style.cssText = `left:${from.left}px;top:${from.top}px;width:${from.width}px;height:${from.height}px;`;
  document.body.appendChild(carrier);
  carrier.appendChild(h.canvas);

  let raf = 0;
  const started = performance.now();
  const land = (attach: boolean): void => {
    cancelAnimationFrame(raf);
    h.landing = null;
    if (attach && host.isConnected) {
      host.appendChild(h.canvas);
      h.width = host.clientWidth;
      h.height = host.clientHeight;
      h.scene?.resize(h.width, h.height);
    } else {
      h.parking.appendChild(h.canvas);
    }
    carrier.remove();
  };
  const step = (): void => {
    const to = host.getBoundingClientRect();
    // Not the frame timestamp: after a busy frame it can predate `started`.
    const p = Math.min(1, Math.max(0, (performance.now() - started) / FLIGHT_MS));
    if (p >= 1 || !host.isConnected || to.width <= 0 || to.height <= 0) {
      land(true);
      return;
    }
    const e = easeInOut(p);
    const mix = (a: number, b: number) => a + (b - a) * e;
    const width = mix(from.width, to.width);
    const height = mix(from.height, to.height);
    carrier.style.left = `${mix(from.left, to.left)}px`;
    carrier.style.top = `${mix(from.top, to.top)}px`;
    carrier.style.width = `${width}px`;
    carrier.style.height = `${height}px`;
    h.scene?.resize(Math.round(width), Math.round(height));
    raf = requestAnimationFrame(step);
  };
  h.landing = () => land(false);
  raf = requestAnimationFrame(step);
}

/** Attach the shared scene to `host` and take over its callbacks. */
export function acquireViewer(options: {
  host: HTMLElement;
  finish: FinishId;
  reducedMotion: boolean;
  onReady: () => void;
  onLost: () => void;
  onCustom: (custom: boolean) => void;
}): ViewerHandle {
  const h = ensureHolder(options.finish, options.reducedMotion);
  const handlers: Handlers = {
    onReady: options.onReady,
    onLost: options.onLost,
    onCustom: options.onCustom,
  };
  // Another viewer still shows the device (connect screen → app): move it across.
  const from =
    h.handlers !== null &&
    h.revealed &&
    !options.reducedMotion &&
    h.canvas.parentElement !== h.parking
      ? h.canvas.getBoundingClientRect()
      : null;
  h.landing?.();
  h.handlers = handlers;
  h.width = options.host.clientWidth;
  h.height = options.host.clientHeight;

  if (from && from.width > 0 && from.height > 0) {
    fly(h, from, options.host);
  } else {
    options.host.appendChild(h.canvas);
  }

  if (h.scene) {
    h.scene.setFinish(options.finish);
    h.scene.setReducedMotion(options.reducedMotion);
    if (!h.landing) h.scene.resize(h.width, h.height);
    options.onCustom(h.scene.custom);
    if (h.ready) options.onReady();
  }

  const owns = () => holder === h && h.handlers === handlers;
  const live = () => (owns() ? h.scene : null);
  return {
    setTargets: (t) => live()?.setTargets(t),
    setFinish: (f) => live()?.setFinish(f),
    setReducedMotion: (on) => live()?.setReducedMotion(on),
    setActive: (on) => {
      if (!owns()) return;
      h.active = on;
      h.scene?.setActive(on);
    },
    resize: (w, height) => {
      if (!owns()) return;
      h.width = w;
      h.height = height;
      if (!h.landing) h.scene?.resize(w, height);
    },
    release: () => {
      // A newer viewer has taken over: leave the scene to it.
      if (!owns()) return;
      h.landing?.();
      h.handlers = null;
      h.active = false;
      h.scene?.setActive(false);
      if (h.canvas.parentElement !== h.parking) h.parking.appendChild(h.canvas);
    },
  };
}
