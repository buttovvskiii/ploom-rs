import type { Diagnostics } from "$lib/api";
import { run } from "$lib/state/api.svelte";

class DiagnosticsStore {
  report = $state<Diagnostics | null>(null);
  loading = $state(false);

  async read(): Promise<void> {
    if (this.loading) return;
    this.loading = true;
    const result = await run((api) => api.readDiagnostics());
    if (result) this.report = result;
    this.loading = false;
  }
}

export const diagnostics = new DiagnosticsStore();
