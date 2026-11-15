import type * as THREE from "three";
import type { Finish } from "./finishes";

/** The device's published dimensions, in millimetres (scene units). */
/** Width is across the front panel (x); the body is deeper (z) than it is wide. */
export const DEVICE = { height: 109.4, width: 24.2, depth: 29.2 } as const;

/** What the scene drives, whichever model is loaded. */
export interface DeviceModel {
  readonly root: THREE.Object3D;
  /** True for the built-in model; a custom GLB brings its own materials. */
  readonly procedural: boolean;
  /** Ease toward a finish. */
  setFinish(finish: Finish, instant: boolean): void;
  /** Linear-space color; intensity 0..~1.2. */
  setStatus(color: THREE.Color, intensity: number): void;
  setGlow(color: THREE.Color, intensity: number): void;
  /** Slide the shutter over the stick socket open or closed (built-in model only). */
  setShutter?(open: boolean, instant?: boolean): void;
  /** 0..1, desaturation while the device is asleep. */
  setDim(dim: number): void;
  /** Per frame. */
  update(dt: number, camera: THREE.Camera): boolean;
  dispose(): void;
}
