/** Drag-to-rotate with inertia and a spring back to the hero pose. */

const YAW_PER_PX = 0.0105;
const PITCH_PER_PX = 0.007;
/** Enough tilt to glimpse the top and the base, not enough to lose the pose. */
const PITCH_LIMIT = 0.85;
/** Free spin after release decays at this rate (1/s) before the spring takes over. */
const FRICTION = 2.6;
/** Spring toward the hero pose (stiffness 1/s², damping 1/s; slightly underdamped). */
const STIFFNESS = 11;
const DAMPING = 5.6;
/**
 * After release the device stays where it was left for a moment (so the back or the top can be
 * looked at), then the spring fades in.
 */
const SPRING_DELAY_S = 2.6;
const SPRING_RAMP_S = 0.9;

export class Orbit {
  yaw: number;
  pitch: number;
  private yawVel = 0;
  private pitchVel = 0;
  private dragging = false;
  private lastX = 0;
  private lastY = 0;
  private lastT = 0;
  private sinceRelease = Infinity;

  constructor(yaw: number, pitch: number) {
    this.yaw = yaw;
    this.pitch = pitch;
  }

  get active(): boolean {
    return this.dragging;
  }

  begin(x: number, y: number, timeMs: number): void {
    this.dragging = true;
    this.lastX = x;
    this.lastY = y;
    this.lastT = timeMs;
    this.yawVel = 0;
    this.pitchVel = 0;
  }

  move(x: number, y: number, timeMs: number): void {
    if (!this.dragging) return;
    const dx = x - this.lastX;
    const dy = y - this.lastY;
    const dt = Math.max(1, timeMs - this.lastT) / 1000;
    this.lastX = x;
    this.lastY = y;
    this.lastT = timeMs;
    this.yaw += dx * YAW_PER_PX;
    // Resist at the limits instead of stopping hard.
    const nextPitch = this.pitch + dy * PITCH_PER_PX * (1 - Math.abs(this.pitch) / PITCH_LIMIT);
    this.pitch = Math.max(-PITCH_LIMIT, Math.min(PITCH_LIMIT, nextPitch));
    // Smoothed release velocity, so a pause before letting go means no fling.
    const k = Math.min(1, dt / 0.05);
    this.yawVel += ((dx * YAW_PER_PX) / dt - this.yawVel) * k;
    this.pitchVel += ((dy * PITCH_PER_PX) / dt - this.pitchVel) * k;
  }

  end(timeMs: number): void {
    if (!this.dragging) return;
    this.dragging = false;
    this.sinceRelease = 0;
    // Held still before release: no fling.
    if (timeMs - this.lastT > 80) {
      this.yawVel = 0;
      this.pitchVel = 0;
    }
    const cap = 14;
    this.yawVel = Math.max(-cap, Math.min(cap, this.yawVel));
  }

  /** Advance by `dt` seconds toward the rest pose. */
  step(dt: number, restYaw: number, restPitch: number): boolean {
    if (this.dragging) return true;
    this.sinceRelease += dt;
    const ramp = Math.min(1, Math.max(0, (this.sinceRelease - SPRING_DELAY_S) / SPRING_RAMP_S));
    const spring = ramp * ramp * (3 - 2 * ramp);
    // Return to the hero angle nearest in whole turns, so a long spin doesn't unwind.
    const turn = Math.PI * 2;
    const target = restYaw + turn * Math.round((this.yaw - restYaw) / turn);

    // Semi-implicit Euler in small substeps for stability at low frame rates.
    const steps = Math.max(1, Math.ceil(dt / (1 / 120)));
    const h = dt / steps;
    for (let i = 0; i < steps; i++) {
      const yawAcc =
        spring * (STIFFNESS * (target - this.yaw) - DAMPING * this.yawVel) -
        (1 - spring) * FRICTION * this.yawVel;
      const pitchAcc =
        spring * (STIFFNESS * (restPitch - this.pitch) - DAMPING * this.pitchVel) -
        (1 - spring) * FRICTION * 2 * this.pitchVel;
      this.yawVel += yawAcc * h;
      this.pitchVel += pitchAcc * h;
      this.yaw += this.yawVel * h;
      this.pitch = Math.max(-PITCH_LIMIT, Math.min(PITCH_LIMIT, this.pitch + this.pitchVel * h));
    }

    const still =
      spring === 1 &&
      Math.abs(target - this.yaw) < 1e-4 &&
      Math.abs(restPitch - this.pitch) < 1e-4 &&
      Math.abs(this.yawVel) < 1e-3 &&
      Math.abs(this.pitchVel) < 1e-3;
    if (still) {
      this.yaw = target;
      this.pitch = restPitch;
      this.yawVel = 0;
      this.pitchVel = 0;
    }
    return !still;
  }
}
