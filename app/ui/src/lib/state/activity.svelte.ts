import type { ActivityEvent } from "$lib/api";

const LIMIT = 100;

/** Activity feed, newest first, de-duplicated by id (history and live events can overlap). */
class ActivityStore {
  events = $state<ActivityEvent[]>([]);

  replace(history: ActivityEvent[]): void {
    const known = new Set(this.events.map((e) => e.id));
    const merged = [...this.events, ...history.filter((e) => !known.has(e.id))];
    this.events = merged.sort((a, b) => b.id - a.id).slice(0, LIMIT);
  }

  push(event: ActivityEvent): void {
    if (this.events.some((e) => e.id === event.id)) return;
    this.events = [event, ...this.events].slice(0, LIMIT);
  }
}

export const activity = new ActivityStore();
