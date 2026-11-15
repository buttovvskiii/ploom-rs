/** A shared wall clock. */
class Clock {
  now = $state(Date.now());
  private preciseUsers = 0;
  private frame = 0;

  constructor() {
    setInterval(() => {
      if (this.preciseUsers === 0) this.now = Date.now();
    }, 1000);
  }

  /** Returns a release function. */
  precise(): () => void {
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) return () => {};
    this.preciseUsers++;
    if (this.preciseUsers === 1) this.loop();
    return () => {
      this.preciseUsers--;
      if (this.preciseUsers === 0) cancelAnimationFrame(this.frame);
    };
  }

  private loop = (): void => {
    this.now = Date.now();
    this.frame = requestAnimationFrame(this.loop);
  };
}

export const clock = new Clock();
