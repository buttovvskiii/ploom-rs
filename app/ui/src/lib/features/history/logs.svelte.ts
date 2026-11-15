import type { LogsReport } from "$lib/api";
import { run } from "$lib/state/api.svelte";

/** Last read logs; kept across page switches so reading (slow, BLE) isn't repeated. */
class LogsStore {
  report = $state<LogsReport | null>(null);
  loading = $state(false);

  async read(): Promise<void> {
    if (this.loading) return;
    this.loading = true;
    const result = await run((api) => api.readLogs());
    if (result) this.report = result;
    this.loading = false;
  }
}

export const logs = new LogsStore();
