/**
 * The studio: renderer, camera, image-based lighting, the contact shadow, and the render loop that
 * eases the device toward its visual targets.
 */
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import type { Lamp, VisualTargets } from "./behavior";
import { FINISH_SPECS, type FinishId } from "./finishes";
import { CUSTOM_MODEL_URL, loadCustomModel } from "./gltf";
import { DEVICE, type DeviceModel } from "./model";
import { Orbit } from "./motion";
import { createProceduralModel } from "./procedural";
import { softSpot } from "./textures";

export interface DeviceSceneOptions {
  finish: FinishId;
  reducedMotion: boolean;
  /** Called once, after the first frame is on screen. */
  onFirstFrame: () => void;
  /** The GPU dropped the context; the viewer falls back to its static image. */
  onLost: () => void;
}

export interface DeviceScene {
  /** A custom GLB was loaded instead of the built-in model. */
  readonly custom: boolean;
  setTargets(targets: VisualTargets): void;
  setFinish(id: FinishId): void;
  setReducedMotion(on: boolean): void;
  /** Visible on screen and the page is in front; the loop only runs while active. */
  setActive(on: boolean): void;
  resize(width: number, height: number): void;
  dispose(): void;
}

/** Three-quarter view: the satin side toward the viewer, the front panel turned to the text. */
const HERO_YAW = 0.62;
const HERO_PITCH = 0;
const CAMERA_ELEVATION = THREE.MathUtils.degToRad(23);
const FOV = 17;
/** Resting height above the floor, and the float around it (mm). */
const LIFT = 2.4;
const FLOAT = 1.2;

/** Exponential approach with separate time constants for rising and falling. */
class Eased {
  constructor(
    public value: number,
    public target = value,
  ) {}

  step(dt: number, rise: number, fall = rise): boolean {
    const tau = this.target > this.value ? rise : fall;
    this.value += (this.target - this.value) * (1 - Math.exp(-dt / tau));
    if (Math.abs(this.target - this.value) < 1e-4) this.value = this.target;
    return this.value !== this.target;
  }
}

class LampState {
  readonly color = new THREE.Color(0, 0, 0);
  readonly targetColor = new THREE.Color(0, 0, 0);
  readonly intensity = new Eased(0);
  readonly depth = new Eased(0);
  private period = 4;
  private phase = 0;

  constructor(
    private readonly rise: number,
    private readonly fall: number,
  ) {}

  set(lamp: Lamp): void {
    // Keep the old color while fading out, so a light dies in its own color.
    if (lamp.intensity > 0) this.targetColor.set(lamp.color);
    this.intensity.target = lamp.intensity;
    this.depth.target = lamp.breathe?.depth ?? 0;
    if (lamp.breathe) this.period = lamp.breathe.periodS;
  }

  /** Returns [output intensity, still animating]. */
  step(dt: number): [number, boolean] {
    let moving = this.intensity.step(dt, this.rise, this.fall);
    moving = this.depth.step(dt, 0.6) || moving;
    const k = 1 - Math.exp(-dt / 0.3);
    this.color.lerp(this.targetColor, k);
    if (!this.color.equals(this.targetColor)) {
      const d =
        Math.abs(this.color.r - this.targetColor.r) +
        Math.abs(this.color.g - this.targetColor.g) +
        Math.abs(this.color.b - this.targetColor.b);
      if (d < 1e-4) this.color.copy(this.targetColor);
      else moving = true;
    }
    const breathing = this.depth.value > 0 && this.intensity.value > 0;
    if (breathing) {
      this.phase = (this.phase + dt / this.period) % 1;
      moving = true;
    }
    const swell = 0.5 - 0.5 * Math.cos(2 * Math.PI * this.phase);
    return [this.intensity.value * (1 - this.depth.value * (1 - swell)), moving];
  }
}

export async function createDeviceScene(
  canvas: HTMLCanvasElement,
  options: DeviceSceneOptions,
): Promise<DeviceScene> {
  const abort = new AbortController();
  // Look for a custom model while the GPU side is being set up.
  const customModel = loadCustomModel(CUSTOM_MODEL_URL, abort.signal);

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    powerPreference: "low-power",
    // Keep the last frame on the canvas, so re-attaching it (returning to the Overview tab) shows
    // the device straight away instead of a blank buffer.
    preserveDrawingBuffer: true,
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.setClearColor(0x000000, 0);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.outputColorSpace = THREE.SRGBColorSpace;

  const scene = new THREE.Scene();
  const pmrem = new THREE.PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const environment = pmrem.fromScene(room, 0.035);
  room.dispose();
  pmrem.dispose();
  scene.environment = environment.texture;
  scene.environmentIntensity = 0.72;
  // Turn the studio so its broad softboxes sweep across the cover at the hero angle.
  scene.environmentRotation.set(0, 0.9, 0);

  // A key from the front right (the side the text is on) and a cool rim from behind keep the
  // silhouette readable against the dark window.
  const key = new THREE.DirectionalLight(0xffffff, 2.1);
  key.position.set(0.8, 0.9, 0.75);
  const rim = new THREE.DirectionalLight(0xdfe8ff, 2.2);
  rim.position.set(-1, 0.55, -0.8);
  scene.add(key, rim);

  const camera = new THREE.PerspectiveCamera(FOV, 1, 10, 4000);

  // Pivot chain: tilt (pitch) > spin (yaw) > shake > lift > model.
  const tilt = new THREE.Group();
  tilt.position.y = DEVICE.height / 2;
  const spin = new THREE.Group();
  const shaker = new THREE.Group();
  const lift = new THREE.Group();
  lift.position.y = -DEVICE.height / 2;
  tilt.add(spin);
  spin.add(shaker);
  shaker.add(lift);
  scene.add(tilt);

  // Contact shadow: two soft ellipses on the floor that follow the footprint.
  const shadowTexture = softSpot();
  const shadowMaterial = (opacity: number) =>
    new THREE.MeshBasicMaterial({
      map: shadowTexture,
      color: 0x000000,
      transparent: true,
      depthWrite: false,
      opacity,
    });
  const floorPlane = new THREE.PlaneGeometry(1, 1);
  floorPlane.rotateX(-Math.PI / 2);
  const contact = new THREE.Mesh(floorPlane, shadowMaterial(0.9));
  const ambient = new THREE.Mesh(floorPlane, shadowMaterial(0.45));
  const shadow = new THREE.Group();
  shadow.add(ambient, contact);
  ambient.position.y = 0.01;
  contact.position.y = 0.02;
  scene.add(shadow);

  const model: DeviceModel = (await customModel) ?? createProceduralModel();
  lift.add(model.root);
  model.setFinish(FINISH_SPECS[options.finish], true);

  let active = false;
  let disposed = false;
  let raf = 0;
  let last = 0;
  let time = 0;
  let firstFrame = false;
  const orbit = new Orbit(HERO_YAW, HERO_PITCH);
  const motion = new Eased(options.reducedMotion ? 0 : 1);
  const dim = new Eased(0);
  const exposure = new Eased(model.procedural ? FINISH_SPECS[options.finish].exposure : 1);
  const shake = new Eased(0);
  const status = new LampState(0.35, 0.5);
  // Heaters warm up quickly and cool down slowly.
  const glow = new LampState(0.9, 2.2);
  const scratch = new THREE.Color();

  // Skip rendering while a page scrolls so the GPU belongs to the scroll.
  let holdUntil = 0;
  const onPageScroll = (): void => {
    holdUntil = performance.now() + 160;
  };
  window.addEventListener("ploom:scroll", onPageScroll);

  const frame = (now: number): void => {
    raf = 0;
    if (disposed) return;
    if (now < holdUntil) {
      last = 0;
      raf = requestAnimationFrame(frame);
      return;
    }
    const dt = last ? Math.min(0.05, Math.max(0, (now - last) / 1000)) : 1 / 60;
    last = now;
    time += dt;

    let moving = motion.step(dt, 1.2);
    const m = motion.value;
    const tau = Math.PI * 2;

    // Idle drift around the hero pose; drag and spring are layered on top.
    const restYaw = HERO_YAW + m * 0.3 * Math.sin((tau * time) / 26);
    const restPitch = HERO_PITCH + m * 0.03 * Math.sin((tau * time) / 19 + 1.3);
    moving = orbit.step(dt, restYaw, restPitch) || moving;
    spin.rotation.y = orbit.yaw;
    tilt.rotation.x = orbit.pitch;

    const height = LIFT + m * FLOAT * Math.sin((tau * time) / 7);
    lift.position.y = -DEVICE.height / 2 + height;
    // The shadow tightens and darkens as the device comes down.
    const near = 1 - (height - (LIFT - FLOAT)) / (FLOAT * 2 + 1e-6);
    contact.scale.set(DEVICE.width * (1.5 - 0.12 * near), 1, DEVICE.depth * (1.6 - 0.12 * near));
    ambient.scale.set(DEVICE.width * 3.4, 1, DEVICE.depth * 3.8);
    (contact.material as THREE.MeshBasicMaterial).opacity =
      (0.72 + 0.18 * near) * (1 - 0.3 * dim.value);
    shadow.rotation.y = spin.rotation.y;

    moving = shake.step(dt, 0.08, 0.2) || moving;
    if (shake.value > 0) {
      const a = shake.value;
      shaker.position.x =
        a * 0.32 * (Math.sin(tau * 43 * time) + 0.6 * Math.sin(tau * 71 * time + 1));
      shaker.position.z = a * 0.16 * Math.sin(tau * 59 * time + 2);
      shaker.rotation.z = a * 0.0045 * Math.sin(tau * 37 * time + 0.5);
      moving = true;
    } else {
      shaker.position.set(0, 0, 0);
      shaker.rotation.set(0, 0, 0);
    }

    moving = dim.step(dt, 0.7, 0.45) || moving;
    moving = exposure.step(dt, 0.3) || moving;
    renderer.toneMappingExposure = exposure.value * (1 - 0.42 * dim.value);
    model.setDim(dim.value);

    const [statusLevel, statusMoving] = status.step(dt);
    model.setStatus(scratch.copy(status.color), statusLevel);
    const [glowLevel, glowMoving] = glow.step(dt);
    model.setGlow(scratch.copy(glow.color), glowLevel);
    moving = statusMoving || glowMoving || moving;

    scene.updateMatrixWorld();
    moving = model.update(dt, camera) || moving;
    renderer.render(scene, camera);

    if (!firstFrame) {
      firstFrame = true;
      options.onFirstFrame();
    }
    if (active && (moving || m > 0)) raf = requestAnimationFrame(frame);
  };

  const wake = (): void => {
    if (!active || disposed || raf) return;
    last = 0;
    raf = requestAnimationFrame(frame);
  };

  const onPointerDown = (event: PointerEvent): void => {
    if (event.button !== 0) return;
    event.preventDefault();
    canvas.setPointerCapture(event.pointerId);
    canvas.style.cursor = "grabbing";
    orbit.begin(event.clientX, event.clientY, event.timeStamp);
    wake();
  };
  const onPointerMove = (event: PointerEvent): void => {
    if (!orbit.active) return;
    orbit.move(event.clientX, event.clientY, event.timeStamp);
    wake();
  };
  const onPointerUp = (event: PointerEvent): void => {
    if (!orbit.active) return;
    if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
    canvas.style.cursor = "";
    orbit.end(event.timeStamp);
    wake();
  };
  const onContextLost = (event: Event): void => {
    event.preventDefault();
    active = false;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
    options.onLost();
  };
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("webglcontextlost", onContextLost);

  const fit = (w: number, h: number): void => {
    renderer.setSize(w, h, false);
    const aspect = w / h;
    const halfFov = THREE.MathUtils.degToRad(FOV / 2);
    // Fit the device with room for the float, the shadow and a turn of the body.
    const contentHeight = DEVICE.height + 14;
    const contentWidth = 50;
    const distance = Math.max(
      contentHeight / 2 / Math.tan(halfFov),
      contentWidth / 2 / (Math.tan(halfFov) * aspect),
    );
    const target = new THREE.Vector3(0, DEVICE.height / 2 + 1.5, 0);
    camera.position.set(
      0,
      target.y + Math.sin(CAMERA_ELEVATION) * distance,
      Math.cos(CAMERA_ELEVATION) * distance,
    );
    camera.lookAt(target);
    camera.fov = FOV;
    camera.aspect = aspect;
    camera.near = distance / 4;
    camera.far = distance * 2;
    camera.updateProjectionMatrix();
  };

  return {
    custom: !model.procedural,

    setTargets(targets: VisualTargets): void {
      dim.target = targets.dim;
      shake.target = targets.shake;
      status.set(targets.status);
      glow.set(targets.glow);
      // The shutter is open exactly while a stick is being heated.
      model.setShutter?.(targets.glow.intensity > 0);
      wake();
    },

    setFinish(id: FinishId): void {
      model.setFinish(FINISH_SPECS[id], false);
      // A custom model keeps its own materials, so it keeps neutral exposure too.
      exposure.target = model.procedural ? FINISH_SPECS[id].exposure : 1;
      wake();
    },

    setReducedMotion(on: boolean): void {
      motion.target = on ? 0 : 1;
      wake();
    },

    setActive(on: boolean): void {
      if (active === on) return;
      active = on;
      if (on) wake();
      else if (raf) {
        cancelAnimationFrame(raf);
        raf = 0;
      }
    },

    resize(w: number, h: number): void {
      if (w <= 0 || h <= 0) return;
      fit(w, h);
      if (disposed) return;
      // Resizing clears the buffer: redraw in the same task, running or not.
      if (raf) cancelAnimationFrame(raf);
      last = performance.now();
      frame(last);
    },

    dispose(): void {
      if (disposed) return;
      disposed = true;
      abort.abort();
      if (raf) cancelAnimationFrame(raf);
      window.removeEventListener("ploom:scroll", onPageScroll);
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", onPointerUp);
      canvas.removeEventListener("webglcontextlost", onContextLost);
      model.dispose();
      floorPlane.dispose();
      contact.material.dispose();
      ambient.material.dispose();
      shadowTexture.dispose();
      environment.dispose();
      renderer.dispose();
      // Release the context now rather than at garbage collection (WebKit caps live contexts).
      renderer.forceContextLoss();
    },
  };
}
